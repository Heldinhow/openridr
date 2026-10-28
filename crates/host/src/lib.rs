mod error;
mod evaluation;
pub mod hostroot;
mod logbuf;
mod lsp;
mod outline;
pub mod process;
mod restore;
pub mod sdk;
mod tools;

pub use error::{HostError, HostResult};
pub use evaluation::{Evaluation, EvaluationContext};
pub use outline::{GlobalJson, Outline, OutlineProject};
pub use restore::{RestoreView, GATED_METHODS as RESTORE_GATED_METHODS};

use outline::build_outline;
use process::Registry;
use sdk::{GlobalJsonRequest, HostSdk, Inventory, ProjectSdk, SdkRoot};
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Methods that may start a process or touch executable project state. Every one of them
/// validates the same host gate before doing anything observable.
pub const EXECUTABLE_METHODS: &[&str] = &[
    "evaluation.run",
    "restore.start",
    "restore.retry",
    "lsp.start",
    "lsp.restart",
    "lsp.didOpen",
    "lsp.didChange",
    "tools.install",
    "tools.retry",
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
    restore: Arc<restore::Restore>,
    restore_seq: u64,
    tools: Arc<tools::Tools>,
    lsp: Option<Arc<lsp::Lsp>>,
    lsp_seq: u64,
    lsp_verbose: bool,
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
    app_home: String,
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
    restore: RestoreView,
    tools: Value,
    lsp: lsp::LspView,
    diagnostics: Value,
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
                restore: Arc::new(restore::Restore::idle()),
                restore_seq: 0,
                tools: Arc::new(tools::Tools::new()),
                lsp: None,
                lsp_seq: 0,
                lsp_verbose: false,
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
        if method == "restore.start" {
            return self.start_restore(params, false);
        }
        if method == "restore.retry" {
            return self.start_restore(params, true);
        }
        if method == "restore.cancel" {
            return self.cancel_restore();
        }
        if method == "tools.plan" {
            return self.plan_tool(params);
        }
        if method == "tools.install" {
            return self.install_tool(params);
        }
        if method == "tools.retry" {
            return self.retry_tool();
        }
        if method == "tools.cancel" {
            return self.cancel_tool();
        }
        if method == "lsp.start" {
            return self.start_lsp(params);
        }
        if method == "lsp.restart" {
            return self.restart_lsp(params);
        }
        if method == "lsp.stop" {
            return self.stop_lsp("Stopped by the user.");
        }
        if method == "lsp.didOpen" {
            return self.lsp_open(params);
        }
        if method == "lsp.didChange" {
            return self.lsp_change(params);
        }
        if EXECUTABLE_METHODS.contains(&method) {
            return self.executable_stub(method);
        }

        match method {
            "app.state" | "workspace.state" | "tools.state" | "lsp.state" => {
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
        if restore::GATED_METHODS.contains(&method) {
            let inner = self.inner.lock().unwrap();
            let workspace = inner
                .workspace
                .as_ref()
                .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
            restore::require_restored(
                &workspace.path,
                &workspace.root,
                matches!(workspace.kind, WorkspaceKind::Solution),
                method,
            )?;
        }
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
        inner.sync_restore();
        inner.sync_tools();
        inner.sync_lsp();
        let root = inner.workspace.as_ref().map(|workspace| workspace.root.clone());
        if let Some(root) = root {
            inner.evaluation.check_external_changes(&root);
        }
        if inner.evaluation.stale {
            if let Some(lsp) = &inner.lsp {
                lsp.mark_partial(
                    "The evaluated project model is stale; reload it and restart analysis for full coverage.",
                );
            }
        }
        let inventory = inner.inventory.as_ref().expect("inventory was initialized");
        let sdk = SdkView {
            host: HostSdk::from_inventory(&inventory.installs),
            project: inner.project_sdk.clone(),
            roots: inventory.roots.clone(),
            inspected_at: Some(inventory.inspected_at),
        };
        let lsp_view = inner
            .lsp
            .as_ref()
            .map(|lsp| lsp.view())
            .unwrap_or_else(lsp::Lsp::unavailable_view);
        let publications = inner
            .lsp
            .as_ref()
            .map(|lsp| lsp.publications())
            .unwrap_or_default();
        Snapshot {
            context_id: inner.context_seq,
            app: AppInfo {
                home: std::env::var("HOME").unwrap_or_default(),
                app_home: tools::app_home().to_string_lossy().to_string(),
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
            restore: inner.restore.view(),
            tools: json!({ tools::TOOL: inner.tools.view() }),
            lsp: lsp_view,
            diagnostics: json!({
                "session": inner.lsp.as_ref().map(|lsp| lsp.session()).unwrap_or(0),
                "snapshots": publications,
            }),
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
        stop_services(
            &mut inner,
            "Workspace changed while analysis or an installation was running; it was stopped and its result will not be applied to the new workspace.",
        );
        stop_restore(
            &mut inner,
            "Workspace changed while a restore was running; its result will not be applied to the new workspace.",
        );
        inner.lsp = None;
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
        inner.restore = Arc::new(restore::Restore::idle());
        Ok(self.snapshot_value(&mut inner))
    }

    fn close_workspace(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        stop_services(
            &mut inner,
            "Workspace closed while analysis or an installation was running; it was stopped and its result is not current.",
        );
        stop_restore(
            &mut inner,
            "Workspace closed while a restore was running; its result will not be applied.",
        );
        inner.lsp = None;
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
        stop_services(
            &mut inner,
            "Trust was revoked while analysis or an installation was running; it was stopped and its result is not current.",
        );
        stop_restore(
            &mut inner,
            "Trust was revoked while this restore was running; it was stopped and its result is not current.",
        );
        inner.project_sdk = ProjectSdk::not_inspected(request.as_ref());
        inner.evaluation = Evaluation::idle();
        inner.processes.mark_awaiting();
        Ok(self.snapshot_value(&mut inner))
    }

    fn redetect_sdks(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        stop_lsp_inner(
            &mut inner,
            "SDK inventory refreshed; analysis was stopped. Start or restart analysis to use the new inventory.",
        );
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
        if let Some(lsp) = &inner.lsp {
            lsp.mark_partial(
                "The project model was re-evaluated; restart analysis to match the current context.",
            );
        }
        Ok(self.snapshot_value(&mut inner))
    }

    /// Explicit `dotnet restore` only. Opening a workspace or evaluating never calls it, and
    /// build/run/test consumers are stopped by `restore_required` instead of restoring.
    fn start_restore(&self, params: Value, retry: bool) -> HostResult<Value> {
        let method = if retry { "restore.retry" } else { "restore.start" };
        let token = self.require_trust(method)?;
        let mut inner = self.inner.lock().unwrap();

        if let Some(process_id) = inner.restore.running_process_id() {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "A restore is already running (process {process_id}). Cancel it before starting another."
                ),
            ));
        }
        let Some(workspace) = inner.workspace.as_ref() else {
            return Err(HostError::new("invalid_state", "No workspace is open."));
        };
        let sdk = inner.project_sdk.clone();
        if sdk.state != "resolved" {
            return Err(HostError::new(
                if sdk.state == "sdk_missing" {
                    "sdk_missing"
                } else {
                    "invalid_state"
                },
                format!(
                    "Restore was not started: the project SDK is not resolved ({}). {}",
                    sdk.state,
                    sdk.message.clone().unwrap_or_else(|| {
                        "Resolve the project SDK before restoring.".to_string()
                    })
                ),
            ));
        }
        let Some(sdk_root) = sdk.root.clone().map(PathBuf::from) else {
            return Err(HostError::new(
                "invalid_state",
                "Restore was not started: the resolved project SDK has no root path.",
            ));
        };
        let muxer = sdk_root.join(dotnet_exe());
        if !muxer.is_file() {
            return Err(HostError::new(
                "sdk_missing",
                format!(
                    "Restore was not started: {} does not contain a .NET muxer.",
                    sdk_root.to_string_lossy()
                ),
            ));
        }

        let is_solution = matches!(workspace.kind, WorkspaceKind::Solution);
        let workspace_file = workspace.path.clone();
        let root = workspace.root.clone();
        stop_lsp_inner(
            &mut inner,
            "Restore started; analysis stopped so it cannot report stale assets. Start analysis after the restore.",
        );
        let plan = evaluation::plan_workspace(&workspace_file, &root, is_solution);
        let planned = |project: &evaluation::PlanProject| {
            project
                .abs_path
                .clone()
                .map(|path| (project.name.clone(), path))
        };
        let (scope, target, projects) =
            match optional_str(&params, "project").filter(|value| !value.trim().is_empty()) {
                Some(requested) => {
                    let normalized = requested
                        .replace('\\', "/")
                        .trim_start_matches("./")
                        .to_string();
                    let Some(project) = plan.projects.iter().find(|p| p.path == normalized) else {
                        return Err(HostError::new(
                            "invalid_state",
                            format!("{requested} is not a project of this workspace."),
                        ));
                    };
                    let Some(path) = project.abs_path.clone() else {
                        return Err(HostError::new(
                            "not_found",
                            project
                                .error
                                .clone()
                                .unwrap_or_else(|| format!("{requested} is not on disk.")),
                        ));
                    };
                    ("project", path.clone(), vec![(project.name.clone(), path)])
                }
                None => (
                    if is_solution { "solution" } else { "project" },
                    workspace_file.clone(),
                    plan.projects.iter().filter_map(planned).collect(),
                ),
            };

        let revision = inner.restore_seq + 1;
        inner.restore_seq = revision;
        let command = vec![
            muxer.to_string_lossy().to_string(),
            "restore".to_string(),
            target.to_string_lossy().to_string(),
            "-nologo".to_string(),
            "-v:minimal".to_string(),
        ];
        inner.restore = Arc::new(restore::Restore::begin(restore::Start {
            revision,
            scope,
            target,
            command: command.clone(),
            cwd: root.clone(),
            sdk_root: sdk_root.clone(),
            sdk_version: sdk
                .resolved_version
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            workspace_id: token.workspace_id,
            context_id: token.context_id,
            root: root.clone(),
            projects,
        }));

        // Separate arguments, explicit cwd/environment and its own process group. Environment
        // values are never written to the log.
        let mut process = Command::new(&command[0]);
        process
            .args(&command[1..])
            .current_dir(&root)
            .env("DOTNET_ROOT", &sdk_root)
            .env("DOTNET_MULTILEVEL_LOOKUP", "0")
            .env("DOTNET_CLI_UI_LANGUAGE", "en")
            .env("DOTNET_NOLOGO", "1")
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1");
        let piped = match inner
            .processes
            .spawn_piped(&format!("restore:{scope}"), token.workspace_id, process)
        {
            Ok(piped) => piped,
            Err(error) => {
                inner
                    .restore
                    .fail(format!("Could not start dotnet restore: {error}"));
                return Ok(self.snapshot_value(&mut inner));
            }
        };
        inner.restore.note_process(piped.id, piped.pid);
        let shared = inner.restore.clone();
        if let Some(stdout) = piped.stdout {
            let sink = shared.clone();
            std::thread::spawn(move || restore::pump(stdout, sink, false));
        }
        if let Some(stderr) = piped.stderr {
            let sink = shared.clone();
            std::thread::spawn(move || restore::pump(stderr, sink, true));
        }
        Ok(self.snapshot_value(&mut inner))
    }

    fn cancel_restore(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        let Some(process_id) = inner.restore.running_process_id() else {
            return Err(HostError::new("invalid_state", "No restore is running."));
        };
        inner.restore.request_cancel();
        let status = inner.processes.terminate_status(process_id);
        if let Some(status) = &status {
            inner.restore.observe(status);
        }
        inner.sync_restore();
        Ok(self.snapshot_value(&mut inner))
    }

    fn plan_tool(&self, params: Value) -> HostResult<Value> {
        let requested = optional_str(&params, "tool").unwrap_or_else(|| tools::TOOL.to_string());
        if requested != tools::TOOL {
            return Err(HostError::new(
                "invalid_state",
                format!("Unknown tool: {requested}. This build installs csharp-ls 0.28.0 only."),
            ));
        }
        let mut inner = self.inner.lock().unwrap();
        inner.ensure_inventory();
        let installs = &inner.inventory.as_ref().expect("inventory").installs;
        let host = HostSdk::from_inventory(installs);
        let plan = tools::plan(&host)?;
        inner.tools.set_plan(plan);
        Ok(self.snapshot_value(&mut inner))
    }

    fn install_tool(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("tools.install")?;
        if params.get("confirm") != Some(&Value::Bool(true)) {
            return Err(HostError::new(
                "confirmation_required",
                "Installing csharp-ls needs an explicit confirmation of the current plan.",
            ));
        }
        let plan_id = required_str(&params, "planId")?;
        let mut inner = self.inner.lock().unwrap();
        inner.ensure_inventory();
        let host = HostSdk::from_inventory(&inner.inventory.as_ref().expect("inventory").installs);
        let current = tools::plan(&host)?;
        if current.plan_id != plan_id {
            inner.tools.set_plan(current.clone());
            return Err(HostError::with_details(
                "confirmation_required",
                "The install plan changed since it was shown; review and confirm the new plan. Nothing was downloaded or executed.",
                json!({ "plan": current.to_json() }),
            ));
        }
        let tools = inner.tools.clone();
        tools.begin_install(
            current,
            token.workspace_id,
            token.context_id,
            &mut inner.processes,
        )?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn retry_tool(&self) -> HostResult<Value> {
        let token = self.require_trust("tools.retry")?;
        let mut inner = self.inner.lock().unwrap();
        inner.ensure_inventory();
        let host = HostSdk::from_inventory(&inner.inventory.as_ref().expect("inventory").installs);
        let current = tools::plan(&host)?;
        let Some(confirmed) = inner.tools.confirmed_plan_id() else {
            return Err(HostError::new(
                "invalid_state",
                "Review and confirm the install plan before retrying; no plan was confirmed.",
            ));
        };
        if current.plan_id != confirmed {
            inner.tools.set_plan(current.clone());
            return Err(HostError::with_details(
                "confirmation_required",
                "The install plan changed since it was confirmed; review and confirm the new plan. Nothing was downloaded or executed.",
                json!({ "plan": current.to_json() }),
            ));
        }
        let tools = inner.tools.clone();
        tools.begin_install(
            current,
            token.workspace_id,
            token.context_id,
            &mut inner.processes,
        )?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn cancel_tool(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        let tools = inner.tools.clone();
        tools.cancel(&mut inner.processes)?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn start_lsp(&self, _params: Value) -> HostResult<Value> {
        let token = self.require_trust("lsp.start")?;
        let mut inner = self.inner.lock().unwrap();
        self.start_analysis(&mut inner, token)
    }

    fn restart_lsp(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("lsp.restart")?;
        let verbose = params.get("verbose").and_then(Value::as_bool);
        let mut inner = self.inner.lock().unwrap();
        if let Some(verbose) = verbose {
            inner.lsp_verbose = verbose;
        }
        stop_lsp_inner(
            &mut inner,
            "Analysis restarted by the user; the previous session was shut down.",
        );
        self.start_analysis(&mut inner, token)
    }

    fn start_analysis(&self, inner: &mut Inner, token: ContextToken) -> HostResult<Value> {
        if let Some(lsp) = &inner.lsp {
            if lsp.is_active() {
                return Err(HostError::with_details(
                    "invalid_state",
                    "Analysis is already running; use Restart analysis to load it again.",
                    json!({ "action": { "method": "lsp.restart", "params": {} } }),
                ));
            }
        }
        if !inner.tools.still_ready() {
            return Err(HostError::with_details(
                "tool_missing",
                format!(
                    "{} {} is not installed and verified (or its launcher changed); analysis was not started.",
                    tools::TOOL,
                    tools::VERSION
                ),
                json!({ "action": { "method": "tools.plan", "params": { "tool": tools::TOOL } } }),
            ));
        }
        let Some(workspace) = inner.workspace.as_ref() else {
            return Err(HostError::new("invalid_state", "No workspace is open."));
        };
        let workspace_path = workspace.path.clone();
        let root = workspace.root.clone();
        let is_solution = matches!(workspace.kind, WorkspaceKind::Solution);
        let workspace_name = workspace.name.clone();
        let project_sdk = inner.project_sdk.clone();
        if project_sdk.state != "resolved" {
            let code = if project_sdk.state == "sdk_missing" {
                "sdk_missing"
            } else {
                "invalid_state"
            };
            return Err(HostError::with_details(
                code,
                format!(
                    "Analysis was not started: the project SDK is not resolved ({}). {}",
                    project_sdk.state,
                    project_sdk
                        .message
                        .clone()
                        .unwrap_or_else(|| "Resolve the project SDK first.".to_string())
                ),
                json!({ "action": { "method": "sdk.redetect", "params": {} } }),
            ));
        }
        let installs = &inner.inventory.as_ref().expect("inventory").installs;
        let host_sdk = HostSdk::from_inventory(installs);
        if host_sdk.state != "found" {
            return Err(HostError::with_details(
                "sdk_missing",
                "Analysis was not started: the .NET 10 host SDK is missing. The project SDK is never substituted.",
                json!({ "action": { "method": "sdk.redetect", "params": {} } }),
            ));
        }
        let host_install = installs
            .iter()
            .find(|install| {
                Some(&install.version) == host_sdk.version.as_ref()
                    && Some(install.root.to_string_lossy().to_string())
                        == host_sdk.root.clone()
            })
            .cloned()
            .ok_or_else(|| {
                HostError::new("sdk_missing", "The .NET 10 host SDK install could not be resolved.")
            })?;
        let project_install = installs
            .iter()
            .find(|install| {
                Some(&install.path) == project_sdk.sdk_path.as_ref()
                    && Some(&install.version) == project_sdk.resolved_version.as_ref()
            })
            .cloned()
            .ok_or_else(|| {
                HostError::new(
                    "sdk_missing",
                    "The resolved project SDK install could not be found in the inventory; run Redetect SDKs.",
                )
            })?;

        // The LSP never restores. Current assets are part of the start contract.
        restore::require_restored(&workspace_path, &root, is_solution, "lsp.start")?;

        let evaluation = inner.evaluation.clone();
        let (coverage, coverage_note) = if evaluation.state == "ok" && !evaluation.stale {
            ("full", None)
        } else {
            (
                "partial",
                Some(format!(
                    "The evaluated project model is {}; analysis may cover only part of the solution. Reload the model and restart analysis for full coverage.",
                    if evaluation.stale {
                        "stale".to_string()
                    } else {
                        evaluation.state.to_string()
                    }
                )),
            )
        };
        let session = inner.lsp_seq + 1;
        let verbose = inner.lsp_verbose;
        let log_level = if verbose { "debug" } else { "info" };

        let hostroot = match hostroot::ensure(&host_install, &project_install) {
            Ok(hostroot) => hostroot,
            Err(message) => {
                let failed = lsp::Lsp::begin(
                    session,
                    token.workspace_id,
                    token.context_id,
                    "",
                    false,
                    false,
                    "",
                    None,
                    Vec::new(),
                    root.to_string_lossy().to_string(),
                    verbose,
                    coverage,
                    coverage_note,
                );
                failed.fail_now(message.clone());
                inner.lsp = Some(failed);
                let code = if message.contains("SDK") || message.contains("runtime") {
                    "sdk_missing"
                } else {
                    "operation_failed"
                };
                return Err(HostError::with_details(
                    code,
                    message,
                    json!({ "action": { "method": "sdk.redetect", "params": {} } }),
                ));
            }
        };

        let binary = tools::destination().join(if cfg!(windows) {
            "csharp-ls.exe"
        } else {
            "csharp-ls"
        });
        if !binary.is_file() {
            return Err(HostError::with_details(
                "tool_missing",
                "The verified csharp-ls launcher disappeared from the destination; review the install plan.",
                json!({ "action": { "method": "tools.plan", "params": { "tool": tools::TOOL } } }),
            ));
        }
        let mut argv = vec![binary.to_string_lossy().to_string()];
        if is_solution {
            argv.push("--solution".to_string());
            argv.push(workspace_path.to_string_lossy().to_string());
        }
        argv.push("--loglevel".to_string());
        argv.push(log_level.to_string());

        let mut command = Command::new(&argv[0]);
        command
            .args(&argv[1..])
            .current_dir(&root)
            .env("DOTNET_ROOT", &hostroot.path)
            .env("DOTNET_MULTILEVEL_LOOKUP", "0")
            .env("DOTNET_CLI_UI_LANGUAGE", "en")
            .env("DOTNET_NOLOGO", "1")
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
            .env_remove("NUGET_PACKAGES")
            .env_remove("NUGET_HTTP_CACHE_PATH")
            .env_remove("DOTNET_CLI_HOME");
        let solution = is_solution.then(|| workspace_path.to_string_lossy().to_string());
        let piped = match inner
            .processes
            .spawn_piped("lsp:csharp-ls", token.workspace_id, command)
        {
            Ok(piped) => piped,
            Err(error) => {
                let failed = lsp::Lsp::begin(
                    session,
                    token.workspace_id,
                    token.context_id,
                    &hostroot.path,
                    hostroot.rebuilt,
                    hostroot.forced,
                    &hostroot.check,
                    solution,
                    argv,
                    root.to_string_lossy().to_string(),
                    verbose,
                    coverage,
                    coverage_note,
                );
                failed.fail_now(format!("Could not start csharp-ls: {error}"));
                inner.lsp = Some(failed);
                return Err(HostError::new(
                    "operation_failed",
                    format!("Could not start csharp-ls: {error}"),
                ));
            }
        };
        let kill = lsp::spawn_killer(piped.pid);
        let lsp = lsp::Lsp::begin(
            session,
            token.workspace_id,
            token.context_id,
            &hostroot.path,
            hostroot.rebuilt,
            hostroot.forced,
            &hostroot.check,
            solution,
            argv,
            root.to_string_lossy().to_string(),
            verbose,
            coverage,
            coverage_note,
        );
        for line in &hostroot.log {
            lsp.log_line(line.clone());
        }
        lsp.attach(piped.id, piped.pid, piped.stdin, kill);
        inner.lsp = Some(lsp.clone());
        inner.lsp_seq = session;
        if let Some(stdout) = piped.stdout {
            lsp::spawn_reader(lsp.clone(), stdout);
        }
        if let Some(stderr) = piped.stderr {
            lsp::spawn_stderr(lsp.clone(), stderr);
        }
        lsp.send_initialize(&root, &workspace_name);
        Ok(self.snapshot_value(inner))
    }

    fn stop_lsp(&self, reason: &str) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        if inner.lsp.is_none() {
            return Err(HostError::new("invalid_state", "Analysis is not running."));
        }
        stop_lsp_inner(&mut inner, reason);
        Ok(self.snapshot_value(&mut inner))
    }

    fn lsp_open(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("lsp.didOpen")?;
        let raw = required_str(&params, "path")?;
        let text = optional_str(&params, "text");
        let inner = self.inner.lock().unwrap();
        let lsp = inner
            .lsp
            .clone()
            .ok_or_else(|| HostError::new("invalid_state", "Analysis is not running."))?;
        if !lsp.accepts_documents() {
            return Err(HostError::new(
                "invalid_state",
                "Analysis has not finished initializing; the document was not sent.",
            ));
        }
        let (path, rel) = resolve_in_workspace(&inner, &raw)?;
        let text = match text {
            Some(text) => text,
            None => fs::read_to_string(&path).map_err(|error| {
                HostError::new(
                    "operation_failed",
                    format!("Could not read {rel} for analysis: {error}"),
                )
            })?,
        };
        let uri = lsp::file_uri(&path);
        let version = lsp
            .open_document(&uri, &rel, &text)
            .map_err(|error| HostError::new("invalid_state", error))?;
        Ok(json!({ "uri": uri, "version": version, "session": lsp.session() }))
    }

    fn lsp_change(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("lsp.didChange")?;
        let raw = required_str(&params, "path")?;
        let text = required_str(&params, "text")?;
        let inner = self.inner.lock().unwrap();
        let lsp = inner
            .lsp
            .clone()
            .ok_or_else(|| HostError::new("invalid_state", "Analysis is not running."))?;
        if !lsp.accepts_documents() {
            return Err(HostError::new(
                "invalid_state",
                "Analysis has not finished initializing; the change was not sent.",
            ));
        }
        let (path, rel) = resolve_in_workspace(&inner, &raw)?;
        let uri = lsp::file_uri(&path);
        let version = lsp
            .change_document(&uri, &rel, &text)
            .map_err(|error| HostError::new("invalid_state", error))?;
        Ok(json!({ "uri": uri, "version": version, "session": lsp.session() }))
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
            stop_lsp_inner(
                &mut inner,
                "A project input changed; analysis stopped so it cannot report the previous context.",
            );
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

    fn sync_tools(&mut self) {
        self.tools.sync(&mut self.processes);
    }

    fn sync_lsp(&mut self) {
        let Some(lsp) = self.lsp.clone() else {
            return;
        };
        let Some(process_id) = lsp.process_id() else {
            return;
        };
        self.processes.refresh();
        let Some(status) = self.processes.status(process_id) else {
            return;
        };
        if status.state == "exited" {
            lsp.observe_exit(status.exit, status.termination.as_ref());
        } else if lsp.state() == "failed" {
            // Fallback for platforms where the reader cannot signal the child (Windows): a
            // failed session is never left running.
            let status = self.processes.terminate_status(process_id);
            if let Some(status) = status {
                lsp.observe_termination(&status);
            }
        }
    }

    /// Observes the exit of a running restore (the registry is the only owner of the child)
    /// and invalidates the evaluated model exactly once, only for the current context.
    fn sync_restore(&mut self) {
        let Some(process_id) = self.restore.running_process_id() else {
            return;
        };
        self.processes.refresh();
        let Some(status) = self.processes.status(process_id) else {
            return;
        };
        if status.state != "exited" {
            return;
        }
        let outcome = self.restore.complete(&status);
        let current = self.workspace.as_ref().map(|workspace| workspace.id)
            == self.restore.workspace_id()
            && self.restore.context_id() == Some(self.context_seq);
        if !current {
            return;
        }
        let reason = match outcome {
            "ok" => "Restore completed; reload the evaluated model before using TargetPath or packages.",
            "cancelled" => "Restore was cancelled; package dependencies and analysis may be incomplete.",
            "failed" => "Restore failed; package dependencies and analysis may be incomplete.",
            _ => return,
        };
        self.evaluation.mark_stale(reason);
    }
}

/// A workspace boundary (revoke, close, switch) terminates a real restore and keeps only
/// what was observed. The record is sealed so its late exit cannot become another context's.
fn stop_restore(inner: &mut Inner, reason: &str) {
    let Some(process_id) = inner.restore.running_process_id() else {
        return;
    };
    let status = inner.processes.terminate_status(process_id);
    inner.restore.seal(reason, status.as_ref());
}

/// Shuts the live language server down (shutdown → exit), then makes the registry observe
/// termination. The stopped/failed record and its log stay in the snapshot.
fn stop_lsp_inner(inner: &mut Inner, reason: &str) {
    let Some(lsp) = inner.lsp.clone() else {
        return;
    };
    if lsp.state() == "stopped" && lsp.process_id().is_none() {
        return;
    }
    lsp.request_stop(reason);
    if let Some(process_id) = lsp.process_id() {
        // Give the server the chance to honour `exit` and exit 0 before the host signals the
        // group. The registry is the only observer of the child.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
        let mut status = inner.processes.status(process_id);
        while std::time::Instant::now() < deadline
            && status.as_ref().is_some_and(|status| status.state != "exited")
        {
            std::thread::sleep(std::time::Duration::from_millis(50));
            status = inner.processes.status(process_id);
        }
        if status.as_ref().is_some_and(|status| status.state != "exited") {
            status = inner.processes.terminate_status(process_id);
        }
        if let Some(status) = status {
            lsp.observe_termination(&status);
        }
    }
}

/// Every workspace boundary stops analysis and a real installation. A tool that was already
/// ready stays ready: it lives in the app home, not in the workspace.
fn stop_services(inner: &mut Inner, reason: &str) {
    stop_lsp_inner(inner, reason);
    let tools = inner.tools.clone();
    tools.seal(reason, None, &mut inner.processes);
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
