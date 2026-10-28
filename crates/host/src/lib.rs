mod error;
mod evaluation;
mod outline;
pub mod process;
pub mod sdk;

pub use error::{HostError, HostResult};
pub use evaluation::{Evaluation, EvaluationContext};
pub use outline::{GlobalJson, Outline, OutlineProject};

use outline::build_outline;
use process::Registry;
use sdk::{GlobalJsonRequest, HostSdk, Inventory, ProjectSdk, SdkRoot};
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Methods that may start a process or touch executable project state. Every one of them
/// validates the same host gate before doing anything observable.
pub const EXECUTABLE_METHODS: &[&str] = &[
    "evaluation.run",
    "restore.start",
    "lsp.start",
    "tools.install",
    "terminal.start",
    "build.start",
    "run.start",
    "debug.start",
    "tests.discover",
    "tests.run",
    "processes.probe",
];

pub struct Host {
    inner: Mutex<Inner>,
}

/// Identifies the executable context an operation started in. A result carrying an older
/// token is stale and must not be written into current state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextToken {
    pub workspace_id: u64,
    pub context_id: u64,
}

struct Inner {
    workspace: Option<Workspace>,
    trusted: bool,
    workspace_seq: u64,
    context_seq: u64,
    trust_decision: Option<TrustDecision>,
    inventory: Option<Inventory>,
    project_sdk: ProjectSdk,
    processes: Registry,
    evaluation: Evaluation,
}

struct Workspace {
    id: u64,
    path: PathBuf,
    root: PathBuf,
    name: String,
    kind: WorkspaceKind,
    outline: Outline,
}

struct TrustDecision {
    granted_at: u64,
    workspace_id: u64,
    revoked_at: Option<u64>,
}

#[derive(Clone, Copy)]
enum WorkspaceKind {
    Solution,
    Project,
}

impl WorkspaceKind {
    fn as_str(self) -> &'static str {
        match self {
            WorkspaceKind::Solution => "solution",
            WorkspaceKind::Project => "project",
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    path: String,
    name: String,
    kind: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    home: String,
    platform: &'static str,
    arch: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceView {
    id: u64,
    path: String,
    name: String,
    kind: &'static str,
    mode: &'static str,
    candidates: Vec<Candidate>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TrustView {
    granted: bool,
    workspace_id: Option<u64>,
    decided_at: Option<u64>,
    revoked_at: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SdkView {
    host: HostSdk,
    project: ProjectSdk,
    roots: Vec<SdkRoot>,
    inspected_at: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DebugView {
    adapter_state: &'static str,
    adapter_message: &'static str,
    apple_silicon_experimental: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    context_id: u64,
    app: AppInfo,
    workspace: Option<WorkspaceView>,
    outline: Option<Outline>,
    trust: TrustView,
    sdk: SdkView,
    debug: DebugView,
    evaluation: Evaluation,
    processes: Vec<Value>,
}

impl Default for Host {
    fn default() -> Self {
        Self::new()
    }
}

impl Host {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                workspace: None,
                trusted: false,
                workspace_seq: 0,
                context_seq: 0,
                trust_decision: None,
                inventory: None,
                project_sdk: ProjectSdk::no_workspace(),
                processes: Registry::new(),
                evaluation: Evaluation::idle(),
            }),
        }
    }

    pub fn dispatch(&self, method: &str, params: Value) -> HostResult<Value> {
        if method == "processes.probe" {
            return self.probe(params);
        }
        if method == "evaluation.run" {
            return self.run_evaluation(params);
        }
        if EXECUTABLE_METHODS.contains(&method) {
            return self.executable_stub(method);
        }

        match method {
            "app.state" | "workspace.state" => {
                let mut inner = self.inner.lock().unwrap();
                Ok(self.snapshot_value(&mut inner))
            }
            "workspace.open" => self.open_workspace(params),
            "workspace.close" => self.close_workspace(),
            "trust.grant" => self.grant_trust(),
            "trust.revoke" => self.revoke_trust(),
            "sdk.redetect" => self.redetect_sdks(),
            "processes.terminate" => self.terminate_process(params),
            "processes.keep" => self.keep_process(params),
            "file.read" => self.read_file(params),
            "file.write" => self.write_file(params),
            _ => Err(HostError::new(
                "not_found",
                format!("Unknown host method: {method}"),
            )),
        }
    }

    /// The one gate every executable path must pass. Direct host calls (HTTP bridge, tests,
    /// future shortcuts) get the same answer as the UI.
    fn require_trust(&self, method: &str) -> HostResult<ContextToken> {
        let inner = self.inner.lock().unwrap();
        if !inner.trusted {
            return Err(HostError::new(
                "trust_required",
                format!("{method} requires workspace trust; no process was started."),
            ));
        }
        let workspace = inner.workspace.as_ref().ok_or_else(|| {
            HostError::new("invalid_state", "No workspace is open.")
        })?;
        Ok(ContextToken {
            workspace_id: workspace.id,
            context_id: inner.context_seq,
        })
    }

    /// Captures the current context for an operation that may finish later.
    pub fn context_token(&self) -> HostResult<ContextToken> {
        let inner = self.inner.lock().unwrap();
        let workspace = inner
            .workspace
            .as_ref()
            .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
        Ok(ContextToken {
            workspace_id: workspace.id,
            context_id: inner.context_seq,
        })
    }

    /// Rejects a late result from another workspace/context. Future asynchronous consumers
    /// call this before writing anything into the snapshot.
    pub fn accept_result(&self, token: ContextToken) -> HostResult<()> {
        let inner = self.inner.lock().unwrap();
        accept_locked(&inner, token)
    }

    fn executable_stub(&self, method: &str) -> HostResult<Value> {
        let _token = self.require_trust(method)?;
        Err(HostError::new(
            "invalid_state",
            format!("{method} is not implemented in this build."),
        ))
    }

    fn snapshot_value(&self, inner: &mut Inner) -> Value {
        serde_json::to_value(self.snapshot(inner)).unwrap()
    }

    fn snapshot(&self, inner: &mut Inner) -> Snapshot {
        inner.ensure_inventory();
        inner.processes.refresh();
        let root = inner.workspace.as_ref().map(|workspace| workspace.root.clone());
        if let Some(root) = root {
            inner.evaluation.check_external_changes(&root);
        }
        let inventory = inner.inventory.as_ref().expect("inventory was initialized");
        let sdk = SdkView {
            host: HostSdk::from_inventory(&inventory.installs),
            project: inner.project_sdk.clone(),
            roots: inventory.roots.clone(),
            inspected_at: Some(inventory.inspected_at),
        };
        Snapshot {
            context_id: inner.context_seq,
            app: AppInfo {
                home: std::env::var("HOME").unwrap_or_default(),
                platform: std::env::consts::OS,
                arch: std::env::consts::ARCH,
                version: env!("CARGO_PKG_VERSION"),
            },
            workspace: inner.workspace.as_ref().map(|workspace| WorkspaceView {
                id: workspace.id,
                path: workspace.path.to_string_lossy().to_string(),
                name: workspace.name.clone(),
                kind: workspace.kind.as_str(),
                mode: if inner.trusted { "trusted" } else { "restricted" },
                candidates: Vec::new(),
            }),
            outline: inner.workspace.as_ref().map(|workspace| workspace.outline.clone()),
            trust: TrustView {
                granted: inner.trusted,
                workspace_id: inner.trust_decision.as_ref().map(|decision| decision.workspace_id),
                decided_at: inner.trust_decision.as_ref().map(|decision| decision.granted_at),
                revoked_at: inner
                    .trust_decision
                    .as_ref()
                    .and_then(|decision| decision.revoked_at),
            },
            sdk,
            debug: DebugView {
                adapter_state: "not_checked",
                adapter_message: "Installation and platform gates arrive with ticket 13.",
                apple_silicon_experimental: false,
            },
            evaluation: inner.evaluation.clone(),
            processes: inner.processes.views(),
        }
    }

    fn open_workspace(&self, params: Value) -> HostResult<Value> {
        let raw = required_str(&params, "path")?;
        let requested = Path::new(&raw);
        if !requested.is_absolute() {
            return Err(HostError::new(
                "path_not_allowed",
                "Workspace path must be absolute.",
            ));
        }
        let canonical = fs::canonicalize(requested).map_err(|_| {
            HostError::new("not_found", format!("Workspace path not found: {raw}"))
        })?;
        let workspace_path = if canonical.is_dir() {
            resolve_directory(&canonical)?
        } else {
            workspace_file(&canonical)?
        };

        let mut workspace = load_workspace(&workspace_path)?;
        let evaluation = Evaluation::for_workspace(
            &workspace.path,
            matches!(workspace.kind, WorkspaceKind::Solution),
            outline_tfms(&workspace.outline),
        );
        let mut inner = self.inner.lock().unwrap();
        inner.processes.mark_awaiting();
        inner.workspace_seq += 1;
        inner.context_seq += 1;
        workspace.id = inner.workspace_seq;
        let request = global_json_request(&workspace.outline);
        inner.project_sdk = ProjectSdk::not_inspected(request.as_ref());
        inner.workspace = Some(workspace);
        inner.trusted = false;
        inner.trust_decision = None;
        inner.evaluation = evaluation;
        Ok(self.snapshot_value(&mut inner))
    }

    fn close_workspace(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        inner.processes.mark_awaiting();
        inner.workspace = None;
        inner.trusted = false;
        inner.trust_decision = None;
        inner.workspace_seq += 1;
        inner.context_seq += 1;
        inner.project_sdk = ProjectSdk::no_workspace();
        inner.evaluation = Evaluation::idle();
        Ok(self.snapshot_value(&mut inner))
    }

    fn grant_trust(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        let Some(workspace_id) = inner.workspace.as_ref().map(|workspace| workspace.id) else {
            return Err(HostError::new(
                "invalid_state",
                "Open a workspace before trusting it.",
            ));
        };
        if inner.trusted {
            return Ok(self.snapshot_value(&mut inner));
        }
        inner.ensure_inventory();
        inner.trusted = true;
        inner.context_seq += 1;
        inner.trust_decision = Some(TrustDecision {
            granted_at: now_ms(),
            workspace_id,
            revoked_at: None,
        });
        let project = inspect_project_sdk(&inner);
        inner.project_sdk = project;
        Ok(self.snapshot_value(&mut inner))
    }

    fn revoke_trust(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        let Some(request) = inner
            .workspace
            .as_ref()
            .map(|workspace| global_json_request(&workspace.outline))
        else {
            return Err(HostError::new("invalid_state", "No workspace is open."));
        };
        inner.trusted = false;
        inner.context_seq += 1;
        if let Some(decision) = inner.trust_decision.as_mut() {
            decision.revoked_at = Some(now_ms());
        }
        inner.project_sdk = ProjectSdk::not_inspected(request.as_ref());
        inner.evaluation = Evaluation::idle();
        // Services started by later tickets stop here; the controlled child processes of this
        // slice become awaiting decisions instead of a cleanup claim.
        inner.processes.mark_awaiting();
        Ok(self.snapshot_value(&mut inner))
    }

    fn redetect_sdks(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        inner.inventory = Some(sdk::inventory());
        inner.context_seq += 1;
        if inner.workspace.is_some() {
            if inner.trusted {
                let project = inspect_project_sdk(&inner);
                inner.project_sdk = project;
            } else {
                let request = inner
                    .workspace
                    .as_ref()
                    .and_then(|workspace| global_json_request(&workspace.outline));
                inner.project_sdk = ProjectSdk::not_inspected(request.as_ref());
            }
            inner
                .evaluation
                .mark_stale("SDK inventory refreshed; re-evaluate the project model.");
        }
        Ok(self.snapshot_value(&mut inner))
    }

    fn probe(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("processes.probe")?;
        if std::env::var("OPENRIDR_ENABLE_PROBE").as_deref() != Ok("1") {
            return Err(HostError::new(
                "invalid_state",
                "processes.probe is a development-only probe; set OPENRIDR_ENABLE_PROBE=1 to enable it.",
            ));
        }
        let kind = params
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("sleep")
            .to_string();
        let mut command = process::probe_command(&kind).map_err(|error| {
            HostError::new(
                "operation_failed",
                format!("Could not prepare probe process: {error}"),
            )
        })?;
        let mut inner = self.inner.lock().unwrap();
        let root = inner
            .workspace
            .as_ref()
            .map(|workspace| workspace.root.clone())
            .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
        command.current_dir(root);
        accept_locked(&inner, token)?;
        inner
            .processes
            .spawn(&format!("probe:{kind}"), token.workspace_id, command)
            .map_err(|error| {
                HostError::new(
                    "operation_failed",
                    format!("Could not start probe process: {error}"),
                )
            })?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn terminate_process(&self, params: Value) -> HostResult<Value> {
        let id = required_u64(&params, "id")?;
        let mut inner = self.inner.lock().unwrap();
        if inner.processes.terminate(id).is_none() {
            return Err(HostError::new(
                "not_found",
                format!("No tracked process with id {id}."),
            ));
        }
        Ok(self.snapshot_value(&mut inner))
    }

    fn keep_process(&self, params: Value) -> HostResult<Value> {
        let id = required_u64(&params, "id")?;
        let mut inner = self.inner.lock().unwrap();
        if inner.processes.keep(id).is_none() {
            return Err(HostError::new(
                "not_found",
                format!("No tracked process with id {id}."),
            ));
        }
        Ok(self.snapshot_value(&mut inner))
    }

    /// Evaluates every project of the open solution (or the single project) with one MSBuild
    /// process per project. Never restores, builds or runs targets.
    fn run_evaluation(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("evaluation.run")?;

        let mut inner = self.inner.lock().unwrap();
        let Some(workspace) = inner.workspace.as_ref() else {
            return Err(HostError::new("invalid_state", "No workspace is open."));
        };
        let is_solution = matches!(workspace.kind, WorkspaceKind::Solution);
        let workspace_file = workspace.path.clone();
        let root = workspace.root.clone();
        let plan = evaluation::plan_workspace(&workspace_file, &root, is_solution);

        let context = EvaluationContext {
            configuration: optional_str(&params, "configuration")
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| inner.evaluation.context.configuration.clone()),
            target_framework: match params.get("targetFramework") {
                Some(Value::String(value)) if !value.trim().is_empty() => {
                    Some(value.trim().to_string())
                }
                Some(_) => None,
                None => inner.evaluation.context.target_framework.clone(),
            },
        };

        if !valid_selection(&context.configuration)
            || context
                .target_framework
                .as_deref()
                .is_some_and(|tfm| !valid_selection(tfm))
        {
            return Err(HostError::new(
                "invalid_state",
                "Configuration and TFM selections must not contain `;`, `=`, quotes or line breaks; MSBuild was not run.",
            ));
        }

        let sdk = inner.project_sdk.clone();
        if sdk.state != "resolved" {
            let message = format!(
                "MSBuild was not run: the project SDK is not resolved ({}). {}",
                sdk.state,
                sdk.message
                    .clone()
                    .unwrap_or_else(|| "Resolve the project SDK before evaluating.".to_string())
            );
            inner.evaluation = Evaluation::failed_context(context, message.clone());
            inner.evaluation.configurations = plan.configurations.clone();
            inner.evaluation.workspace_id = Some(token.workspace_id);
            inner.evaluation.context_id = Some(token.context_id);
            let code = if sdk.state == "sdk_missing" {
                "sdk_missing"
            } else {
                "invalid_state"
            };
            return Err(HostError::new(code, message));
        }
        let available_tfms = inner.evaluation.available_tfms.clone();
        let Some(sdk_root) = sdk.root.clone().map(PathBuf::from) else {
            let message = "MSBuild was not run: the resolved project SDK has no root path.";
            inner.evaluation = Evaluation::failed_context(context, message.to_string());
            inner.evaluation.configurations = plan.configurations.clone();
            inner.evaluation.available_tfms = available_tfms;
            inner.evaluation.workspace_id = Some(token.workspace_id);
            inner.evaluation.context_id = Some(token.context_id);
            return Err(HostError::new("invalid_state", message));
        };
        let muxer = sdk_root.join(dotnet_exe());
        if !muxer.is_file() {
            let message = format!(
                "MSBuild was not run: {} does not contain a .NET muxer.",
                sdk_root.to_string_lossy()
            );
            inner.evaluation = Evaluation::failed_context(context, message.clone());
            inner.evaluation.configurations = plan.configurations.clone();
            inner.evaluation.available_tfms = available_tfms;
            inner.evaluation.workspace_id = Some(token.workspace_id);
            inner.evaluation.context_id = Some(token.context_id);
            return Err(HostError::new("sdk_missing", message));
        }

        let available_tfms = inner.evaluation.available_tfms.clone();
        inner.evaluation.begin(
            context.clone(),
            plan.configurations.clone(),
            available_tfms,
            token.workspace_id,
            token.context_id,
        );
        let run_revision = inner.evaluation.revision;
        // Evaluation runs outside the lock so snapshots can observe `running` and revoke/close
        // can invalidate the context while MSBuild is working.
        drop(inner);

        let mut projects = Vec::new();
        let mut lines = vec![
            format!(
                "Project SDK {} · {}",
                sdk.resolved_version.clone().unwrap_or_else(|| "unknown".to_string()),
                sdk_root.to_string_lossy()
            ),
            format!(
                "Workspace {} · Configuration {} · TFM {}",
                workspace_file.to_string_lossy(),
                context.configuration,
                context
                    .target_framework
                    .clone()
                    .unwrap_or_else(|| "not selected".to_string())
            ),
            format!(
                "Evaluating {} project(s); no restore, no build, no tasks.",
                plan.projects.len()
            ),
        ];
        for plan_project in &plan.projects {
            let project = evaluation::evaluate_project(
                &muxer,
                &sdk_root,
                &root,
                plan_project,
                &context.configuration,
                context.target_framework.as_deref(),
            );
            lines.push(format!(
                "[{}] {} · exit {} · {} project reference(s) · {} package reference(s) · {} source(s)",
                project.name,
                project.state,
                project
                    .exit
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                project.project_refs.len(),
                project.package_refs.len(),
                project.sources.len()
            ));
            projects.push(project);
        }
        let mut available_tfms: Vec<String> = Vec::new();
        for project in &projects {
            for tfm in &project.tfms {
                if !available_tfms.contains(tfm) {
                    available_tfms.push(tfm.clone());
                }
            }
        }
        let unrestored = projects
            .iter()
            .filter(|project| project.restored != "restored")
            .count();
        if unrestored > 0 {
            lines.push(format!(
                "Restore not requested for {unrestored} project(s); package dependencies and analysis are incomplete."
            ));
        }
        let watched = evaluation::watch_paths(&workspace_file, &root, &plan.projects);

        let mut inner = self.inner.lock().unwrap();
        accept_locked(&inner, token)?;
        if inner.evaluation.revision != run_revision || inner.evaluation.state != "running" {
            return Err(HostError::new(
                "stale_context",
                "An older evaluation result was discarded; it did not replace the current evaluation.",
            ));
        }
        inner
            .evaluation
            .complete(projects, lines.join("\n"), watched, available_tfms);
        Ok(self.snapshot_value(&mut inner))
    }

    fn read_file(&self, params: Value) -> HostResult<Value> {
        let raw = required_str(&params, "path")?;
        let inner = self.inner.lock().unwrap();
        let (path, rel) = resolve_in_workspace(&inner, &raw)?;
        let content = fs::read_to_string(&path).map_err(|error| {
            HostError::new(
                "operation_failed",
                format!("Could not read {rel} as UTF-8 text: {error}"),
            )
        })?;
        let hash = content_hash(&content);
        Ok(json!({ "path": rel, "content": content, "hash": hash }))
    }

    fn write_file(&self, params: Value) -> HostResult<Value> {
        let raw = required_str(&params, "path")?;
        let content = required_str(&params, "content")?;
        let expected_hash = required_str(&params, "expected_hash")?;
        let mut inner = self.inner.lock().unwrap();
        let (path, rel) = resolve_in_workspace(&inner, &raw)?;

        let current = fs::read_to_string(&path).map_err(|error| {
            HostError::new(
                "operation_failed",
                format!("Could not read {rel} before saving: {error}"),
            )
        })?;
        let current_hash = content_hash(&current);
        if current_hash != expected_hash {
            return Err(HostError::with_details(
                "conflict",
                format!("{rel} changed on disk since it was read; the buffer was not saved."),
                json!({ "currentContent": current, "currentHash": current_hash }),
            ));
        }

        let temp = path.with_file_name(format!(
            ".openridr-save-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        if let Err(error) = fs::write(&temp, content.as_bytes()) {
            return Err(HostError::new(
                "operation_failed",
                format!("Could not write {rel}: {error}"),
            ));
        }
        if let Err(error) = fs::rename(&temp, &path) {
            let _ = fs::remove_file(&temp);
            return Err(HostError::new(
                "operation_failed",
                format!("Could not replace {rel}: {error}"),
            ));
        }
        if is_evaluation_input(&rel)
            && inner
                .evaluation
                .mark_stale(format!("{rel} was saved; the evaluated project model is stale."))
        {
            inner.context_seq += 1;
        }
        Ok(json!({ "path": rel, "hash": content_hash(&content) }))
    }
}

impl Inner {
    fn ensure_inventory(&mut self) {
        if self.inventory.is_none() {
            self.inventory = Some(sdk::inventory());
        }
    }
}

fn accept_locked(inner: &Inner, token: ContextToken) -> HostResult<()> {
    let current = inner
        .workspace
        .as_ref()
        .filter(|workspace| workspace.id == token.workspace_id)
        .is_some()
        && inner.context_seq == token.context_id
        && inner.trusted;
    if current {
        Ok(())
    } else {
        Err(HostError::new(
            "stale_context",
            "A result from an older workspace or context was discarded; it did not change current state.",
        ))
    }
}

fn inspect_project_sdk(inner: &Inner) -> ProjectSdk {
    let Some(workspace) = inner.workspace.as_ref() else {
        return ProjectSdk::no_workspace();
    };
    let Some(inventory) = inner.inventory.as_ref() else {
        return ProjectSdk::no_workspace();
    };
    let global_path = workspace.root.join("global.json");
    let Ok(canonical) = fs::canonicalize(&global_path) else {
        return ProjectSdk::no_global_json();
    };
    if !canonical.starts_with(&workspace.root) {
        return ProjectSdk::invalid("global.json", "global.json resolves outside the workspace.");
    }
    let text = match fs::read_to_string(&canonical) {
        Ok(text) => text,
        Err(error) => {
            return ProjectSdk::invalid(
                "global.json",
                format!("global.json could not be read as UTF-8 text: {error}"),
            )
        }
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            return ProjectSdk::invalid("global.json", format!("global.json is not valid JSON: {error}"))
        }
    };
    let sdk = value.get("sdk");
    let version = sdk
        .and_then(|sdk| sdk.get("version"))
        .and_then(Value::as_str);
    let roll_forward = sdk
        .and_then(|sdk| sdk.get("rollForward"))
        .and_then(Value::as_str);
    let allow_prerelease = sdk
        .and_then(|sdk| sdk.get("allowPrerelease"))
        .and_then(Value::as_bool);
    let request = GlobalJsonRequest {
        path: "global.json".to_string(),
        version: version.map(str::to_string),
        roll_forward: roll_forward.map(str::to_string),
        allow_prerelease,
    };
    let Some(version) = version else {
        return ProjectSdk::not_pinned("global.json");
    };
    match sdk::resolve(
        version,
        roll_forward,
        allow_prerelease,
        &inventory.installs,
    ) {
        Ok(install) => ProjectSdk::resolved(&request, &install),
        Err(message) => ProjectSdk::missing(&request, message),
    }
}

fn global_json_request(outline: &Outline) -> Option<GlobalJsonRequest> {
    outline.global_json.as_ref().map(|global| GlobalJsonRequest {
        path: global.path.clone(),
        version: global.version.clone(),
        roll_forward: global.roll_forward.clone(),
        allow_prerelease: None,
    })
}

fn required_str(params: &Value, key: &str) -> HostResult<String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            HostError::new(
                "invalid_state",
                format!("Missing string parameter `{key}`."),
            )
        })
}

fn required_u64(params: &Value, key: &str) -> HostResult<u64> {
    params.get(key).and_then(Value::as_u64).ok_or_else(|| {
        HostError::new(
            "invalid_state",
            format!("Missing numeric parameter `{key}`."),
        )
    })
}

fn optional_str(params: &Value, key: &str) -> Option<String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// MSBuild's `-p:` syntax also accepts `;name=value`, so a selection from the UI boundary
/// must never smuggle a second property. Spaces stay valid for existing solution configurations.
fn valid_selection(value: &str) -> bool {
    !value.trim().is_empty()
        && value
            .chars()
            .all(|c| !matches!(c, ';' | '=' | '"' | '\'' | '\n' | '\r'))
}

fn outline_tfms(outline: &Outline) -> Vec<String> {
    let mut tfms = Vec::new();
    for project in &outline.projects {
        for tfm in &project.tfms {
            if !tfms.contains(tfm) {
                tfms.push(tfm.clone());
            }
        }
    }
    tfms
}

/// Saving these files can change the MSBuild model; source files cannot.
fn is_evaluation_input(rel: &str) -> bool {
    let lower = rel.to_ascii_lowercase();
    lower.ends_with(".csproj")
        || lower.ends_with(".props")
        || lower.ends_with(".targets")
        || lower.ends_with(".sln")
        || lower.ends_with("/global.json")
        || lower == "global.json"
}

fn dotnet_exe() -> &'static str {
    if cfg!(windows) {
        "dotnet.exe"
    } else {
        "dotnet"
    }
}

fn workspace_file(path: &Path) -> HostResult<PathBuf> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("sln") | Some("csproj") => Ok(path.to_path_buf()),
        _ => Err(HostError::new(
            "path_not_allowed",
            "Open a .sln, a .csproj or a directory that contains one.",
        )),
    }
}

fn resolve_directory(dir: &Path) -> HostResult<PathBuf> {
    let solutions = files_with_extension(dir, "sln");
    if solutions.len() == 1 {
        return Ok(solutions[0].clone());
    }
    if solutions.len() > 1 {
        return Err(ambiguous_workspace(dir, &solutions, "solution"));
    }

    let projects = files_with_extension(dir, "csproj");
    match projects.len() {
        1 => Ok(projects[0].clone()),
        0 => Err(HostError::new(
            "not_found",
            format!("No .sln or .csproj found in {}.", dir.to_string_lossy()),
        )),
        _ => Err(ambiguous_workspace(dir, &projects, "project")),
    }
}

fn files_with_extension(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.symlink_metadata().is_ok_and(|meta| meta.is_file())
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
        })
        .collect();
    files.sort();
    files
}

fn ambiguous_workspace(dir: &Path, files: &[PathBuf], kind: &'static str) -> HostError {
    let candidates: Vec<Candidate> = files
        .iter()
        .map(|path| Candidate {
            path: path.to_string_lossy().to_string(),
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
            kind,
        })
        .collect();
    HostError::with_details(
        "ambiguous_workspace",
        format!(
            "Multiple solutions or projects found in {}; choose one explicitly.",
            dir.to_string_lossy()
        ),
        json!({ "candidates": candidates }),
    )
}

fn load_workspace(path: &Path) -> HostResult<Workspace> {
    let kind = match path.extension().and_then(|ext| ext.to_str()) {
        Some("sln") => WorkspaceKind::Solution,
        Some("csproj") => WorkspaceKind::Project,
        _ => {
            return Err(HostError::new(
                "path_not_allowed",
                "Workspace must be a .sln or .csproj file.",
            ))
        }
    };
    let root = path
        .parent()
        .ok_or_else(|| HostError::new("path_not_allowed", "Workspace has no parent directory."))?
        .to_path_buf();
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let outline = build_outline(&root, path, matches!(kind, WorkspaceKind::Solution));
    Ok(Workspace {
        id: 0,
        path: path.to_path_buf(),
        root,
        name,
        kind,
        outline,
    })
}

fn resolve_in_workspace(inner: &Inner, raw: &str) -> HostResult<(PathBuf, String)> {
    let workspace = inner
        .workspace
        .as_ref()
        .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
    if raw.trim().is_empty() {
        return Err(HostError::new("path_not_allowed", "File path is empty."));
    }
    let requested = Path::new(raw);
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        workspace.root.join(requested)
    };
    let canonical = fs::canonicalize(&candidate)
        .map_err(|_| HostError::new("not_found", format!("File not found: {raw}")))?;
    if !canonical.starts_with(&workspace.root) {
        return Err(HostError::new(
            "outside_workspace",
            format!("Path is outside the workspace: {raw}"),
        ));
    }
    if !canonical.is_file() {
        return Err(HostError::new(
            "path_not_allowed",
            format!("Not a file: {raw}"),
        ));
    }
    let rel = canonical
        .strip_prefix(&workspace.root)
        .unwrap_or(&canonical)
        .to_string_lossy()
        .replace('\\', "/");
    Ok((canonical, rel))
}

fn content_hash(content: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in content.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}
