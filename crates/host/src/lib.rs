mod build;
mod dap;
mod debugger;
mod diagnostics;
mod error;
mod evaluation;
mod fsutil;
mod git;
pub mod hostroot;
mod logbuf;
mod lsp;
mod nuget;
mod outline;
pub mod process;
mod restore;
pub mod sdk;
mod run;
mod run_configs;
mod search;
mod settings;
mod terminal;
#[path = "test_discovery.rs"]
mod test_discovery;
#[path = "test_run.rs"]
mod test_run;
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
use std::collections::{HashMap, HashSet};
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
    "lsp.completion",
    "lsp.definition",
    "lsp.rename",
    "lsp.codeActions",
    "lsp.references",
    "lsp.documentSymbol",
    "tools.install",
    "tools.retry",
    "debug.install",
    "debug.retry",
    "terminal.start",
    "terminal.write",
    "terminal.resize",
    "build.start",
    "run.start",
    "debug.start",
    "debug.continue",
    "debug.stepOver",
    "debug.stepInto",
    "debug.setBreakpoints",
    "debug.variables",
    "tests.discover",
    "tests.run",
    "processes.probe",
    "diagnostics.probe",
    "git.status",
    "git.diff",
    "git.branches",
    "git.branchCreate",
    "git.checkout",
    "git.log",
    "git.commit",
    "git.pull",
    "git.push",
    "git.info",
    "nuget.list",
    "nuget.planUpdate",
    "nuget.applyUpdate",
    "nuget.probe",
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
    processes: Arc<Mutex<Registry>>,
    dap: Arc<dap::Dap>,
    dap_seq: u64,
    evaluation: Evaluation,
    restore: Arc<restore::Restore>,
    restore_seq: u64,
    build: Arc<build::Build>,
    build_seq: u64,
    run: Arc<run::Run>,
    run_seq: u64,
    terminal: Arc<terminal::Terminal>,
    terminal_seq: u64,
    tests: Arc<test_discovery::Tests>,
    tests_seq: u64,
    test_run: Arc<test_run::TestRun>,
    test_run_seq: u64,
    tools: Arc<tools::Tools>,
    debugger: Arc<debugger::Debugger>,
    debugger_experimental: bool,
    lsp: Option<Arc<lsp::Lsp>>,
    lsp_seq: u64,
    lsp_verbose: bool,
    rename_seq: u64,
    action_seq: u64,
    nuget_seq: u64,
    nuget_plan: Option<nuget::Plan>,
    diagnostics: diagnostics::Store,
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

/// Everything a completion/definition answer must state, including when nothing was requested
/// (analysis absent, capability absent, document not open). Answers are data, not errors, so
/// they never block editing.
struct AnalysisBase {
    method: &'static str,
    token: ContextToken,
    session: u64,
    path: String,
    line: u64,
    character: u64,
    revision: Option<i64>,
    version: Option<i64>,
    coverage: &'static str,
    coverage_note: Option<String>,
}

struct AnalysisRequest {
    base: AnalysisBase,
    lsp: Arc<lsp::Lsp>,
    uri: String,
}

impl AnalysisBase {
    fn answer(&self, state: &'static str, message: Option<String>) -> Value {
        json!({
            "method": self.method,
            "state": state,
            "message": message,
            "session": self.session,
            "workspaceId": self.token.workspace_id,
            "contextId": self.token.context_id,
            "path": self.path,
            "line": self.line,
            "character": self.character,
            "revision": self.revision,
            "clientRevision": self.version,
            "documentVersion": self.version,
            "documentKnown": self.version.is_some(),
            "coverage": self.coverage,
            "coverageNote": self.coverage_note,
            "items": [],
            "itemCount": 0,
            "totalItems": 0,
            "truncated": false,
            "isIncomplete": false,
            "replaceRange": Value::Null,
            "locations": [],
            "rejectedTargets": 0,
            "actions": [],
            "actionId": Value::Null,
            "actionCount": 0,
            "totalActions": 0,
            "rejectedActions": 0,
            "unsupportedActions": 0,
            "diagnosticsUsed": 0,
            "references": [],
            "referenceCount": 0,
            "totalReferences": 0,
            "rejectedReferences": 0,
            "symbols": [],
            "symbolCount": 0,
            "totalSymbols": 0,
            "rejectedSymbols": 0,
        })
    }
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
struct Snapshot {
    context_id: u64,
    app: AppInfo,
    workspace: Option<WorkspaceView>,
    outline: Option<Outline>,
    trust: TrustView,
    sdk: SdkView,
    debug: debugger::View,
    evaluation: Evaluation,
    processes: Vec<Value>,
    restore: RestoreView,
    build: build::BuildView,
    run: run::RunView,
    terminal: terminal::TerminalView,
    tests: test_discovery::TestsView,
    dap: dap::DapView,
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
                processes: Arc::new(Mutex::new(Registry::new())),
                dap: Arc::new(dap::Dap::idle()),
                dap_seq: 0,
                evaluation: Evaluation::idle(),
                restore: Arc::new(restore::Restore::idle()),
                restore_seq: 0,
                build: Arc::new(build::Build::idle()),
                build_seq: 0,
                run: Arc::new(run::Run::idle()),
                run_seq: 0,
                terminal: Arc::new(terminal::Terminal::idle()),
                terminal_seq: 0,
                tests: Arc::new(test_discovery::Tests::idle()),
                tests_seq: 0,
                test_run: Arc::new(test_run::TestRun::idle()),
                test_run_seq: 0,
                tools: Arc::new(tools::Tools::new()),
                debugger: Arc::new(debugger::Debugger::new()),
                debugger_experimental: false,
                lsp: None,
                lsp_seq: 0,
                lsp_verbose: false,
                rename_seq: 0,
                action_seq: 0,
                nuget_seq: 0,
                nuget_plan: None,
                diagnostics: diagnostics::Store::new(),
            }),
        }
    }

    pub fn dispatch(&self, method: &str, params: Value) -> HostResult<Value> {
        if method == "processes.probe" {
            return self.probe(params);
        }
        if method == "diagnostics.probe" {
            return self.diagnostics_probe(params);
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
        if method == "build.start" {
            return self.start_build(params);
        }
        if method == "build.cancel" {
            return self.cancel_build();
        }
        if method == "run.start" {
            return self.start_run(params);
        }
        if method == "run.cancel" {
            return self.cancel_run();
        }
        if method == "tests.discover" {
            return self.start_tests_discovery(params);
        }
        if method == "tests.discover_cancel" {
            return self.cancel_tests_discovery();
        }
        if method == "tests.run" {
            return self.start_test_run(params);
        }
        if method == "tests.run_cancel" {
            return self.cancel_test_run();
        }
        if method == "terminal.start" {
            return self.start_terminal(params);
        }
        if method == "terminal.write" {
            return self.write_terminal(params);
        }
        if method == "terminal.resize" {
            return self.resize_terminal(params);
        }
        if method == "terminal.stop" {
            return self.stop_terminal_session(params);
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
        if method == "debug.plan" {
            return self.plan_debugger();
        }
        if method == "debug.install" {
            return self.install_debugger(params);
        }
        if method == "debug.retry" {
            return self.retry_debugger();
        }
        if method == "debug.cancel" {
            return self.cancel_debugger();
        }
        if method == "debug.flag" {
            return self.flag_debugger(params);
        }
        if method == "debug.start" {
            return self.start_debug(params);
        }
        if method == "debug.stop" {
            return self.stop_debug_session();
        }
        if method == "debug.continue" {
            return self.control_debug("continue");
        }
        if method == "debug.stepOver" {
            return self.control_debug("stepOver");
        }
        if method == "debug.stepInto" {
            return self.control_debug("stepInto");
        }
        if method == "debug.setBreakpoints" {
            return self.set_debug_breakpoints(params);
        }
        if method == "debug.variables" {
            return self.debug_variables(params);
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
        if method == "lsp.completion" {
            return self.lsp_completion(params);
        }
        if method == "lsp.definition" {
            return self.lsp_definition(params);
        }
        if method == "lsp.rename" {
            return self.lsp_rename(params);
        }
        if method == "lsp.codeActions" {
            return self.lsp_code_actions(params);
        }
        if method == "lsp.references" {
            return self.lsp_references(params);
        }
        if method == "lsp.documentSymbol" {
            return self.lsp_document_symbol(params);
        }
        if method == "lsp.workspaceSymbol" {
            return self.lsp_workspace_symbol(params);
        }
        if method == "workspace.files" {
            return self.workspace_files();
        }
        if method == "search.text" {
            return self.search_text(params);
        }
        if method == "git.status" {
            return self.git_status();
        }
        if method == "git.diff" {
            return self.git_diff(params);
        }
        if method == "git.branches" {
            return self.git_branches();
        }
        if method == "git.branchCreate" {
            return self.git_branch_create(params);
        }
        if method == "git.checkout" {
            return self.git_checkout(params);
        }
        if method == "git.log" {
            return self.git_log();
        }
        if method == "git.commit" {
            return self.git_commit(params);
        }
        if method == "git.pull" {
            return self.git_pull();
        }
        if method == "git.push" {
            return self.git_push(params);
        }
        if method == "git.info" {
            return self.git_info();
        }
        if method == "nuget.list" {
            return self.nuget_list(params);
        }
        if method == "nuget.planUpdate" {
            return self.nuget_plan_update(params);
        }
        if method == "nuget.applyUpdate" {
            return self.nuget_apply_update(params);
        }
        if method == "nuget.probe" {
            return self.nuget_probe(params);
        }
        if EXECUTABLE_METHODS.contains(&method) {
            return self.gate_unhandled_executable(method);
        }

        match method {
            "app.state" | "workspace.state" | "tools.state" | "debug.state" | "lsp.state"
            | "terminal.state" | "tests.state" => {
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
            "settings.read" => settings::read(),
            "settings.write" => settings::write(&params),
            "runConfigs.read" => run_configs::read(&params),
            "runConfigs.write" => run_configs::write(&params),
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

    /// Trust/restore gate for executable methods this build does not implement, so an
    /// unhandled method still cannot bypass the policy.
    fn gate_unhandled_executable(&self, method: &str) -> HostResult<Value> {
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
        inner.processes.lock().unwrap().refresh();
        inner.sync_restore();
        inner.sync_build();
        inner.sync_run();
        inner.sync_terminal();
        inner.sync_tests();
        inner.sync_test_run();
        inner.sync_dap();
        inner.sync_tools();
        inner.sync_lsp();
        let root = inner.workspace.as_ref().map(|workspace| workspace.root.clone());
        if let Some(root) = root {
            inner.evaluation.check_external_changes(&root);
        }
        if inner.evaluation.stale {
            inner.build.mark_stale(
                "The evaluated project model is stale; the previous build is not current.",
            );
            inner.tests.mark_stale(
                "The evaluated project model is stale; discover tests again before using the selection.",
            );
            inner.test_run.mark_stale(
                "The evaluated project model is stale; these test results are not current.",
            );
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
        let documents = inner
            .lsp
            .as_ref()
            .map(|lsp| lsp.document_revisions())
            .unwrap_or_default();
        if let Some(lsp) = inner.lsp.clone() {
            match lsp_view.state {
                "failed" => inner.diagnostics.mark_unavailable(
                    diagnostics::LSP_ORIGIN,
                    format!(
                        "Analysis failed; diagnostics from session {} are not current. {}",
                        lsp_view.session,
                        lsp_view
                            .error
                            .clone()
                            .unwrap_or_else(|| "See the analysis log.".to_string())
                    ),
                ),
                "stopped" => inner.diagnostics.mark_unavailable(
                    diagnostics::LSP_ORIGIN,
                    format!(
                        "Analysis is stopped; diagnostics from session {} are not current. {}",
                        lsp_view.session,
                        lsp_view
                            .stopped_reason
                            .clone()
                            .unwrap_or_else(|| "Start analysis again to refresh them.".to_string())
                    ),
                ),
                _ => {
                    for publication in lsp.publications() {
                        inner
                            .diagnostics
                            .ingest(&diagnostics::Origin::lsp(), &publication);
                    }
                }
            }
        }
        let diagnostics_view = inner.diagnostics.view(
            &documents,
            lsp_view.coverage,
            lsp_view.coverage_note.clone(),
        );
        let mut tests_view = inner.tests.view();
        tests_view.run = inner.test_run.view();
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
            debug: inner.debugger.view(inner.debugger_experimental),
            evaluation: inner.evaluation.clone(),
            processes: inner.processes.lock().unwrap().views(),
            restore: inner.restore.view(),
            build: inner.build.view(),
            run: inner.run.view(),
            terminal: inner.terminal.view(),
            tests: tests_view,
            dap: inner.dap.view(),
            tools: json!({ tools::TOOL: inner.tools.view() }),
            lsp: lsp_view,
            diagnostics: serde_json::to_value(diagnostics_view).expect("diagnostics serialize"),
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
        stop_build(
            &mut inner,
            "Workspace changed while a build was running; it was stopped and its result will not be applied to the new workspace.",
        );
        stop_run(
            &mut inner,
            "Workspace changed while a program was running; it was stopped and its result will not be applied to the new workspace.",
        );
        stop_terminal(
            &mut inner,
            "Workspace changed while the shell was running; it was stopped and its output will not be attached to the new workspace.",
        );
        stop_tests(
            &mut inner,
            "Workspace changed while tests were being discovered; the discovery was stopped and its result will not be applied to the new workspace.",
        );
        stop_test_run(
            &mut inner,
            "Workspace changed while tests were running; the run was stopped and its results will not be applied to the new workspace.",
        );
        inner.lsp = None;
        inner.diagnostics.clear();
        inner.processes.lock().unwrap().mark_awaiting();
        inner.workspace_seq += 1;
        inner.context_seq += 1;
        inner.nuget_plan = None;
        workspace.id = inner.workspace_seq;
        let request = global_json_request(&workspace.outline);
        inner.project_sdk = ProjectSdk::not_inspected(request.as_ref());
        inner.workspace = Some(workspace);
        inner.trusted = false;
        inner.debugger_experimental = false;
        inner.trust_decision = None;
        inner.evaluation = evaluation;
        inner.restore = Arc::new(restore::Restore::idle());
        inner.build = Arc::new(build::Build::idle());
        inner.run = Arc::new(run::Run::idle());
        inner.terminal = Arc::new(terminal::Terminal::idle());
        inner.tests = Arc::new(test_discovery::Tests::idle());
        inner.test_run = Arc::new(test_run::TestRun::idle());
        inner.dap = Arc::new(dap::Dap::idle());
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
        stop_build(
            &mut inner,
            "Workspace closed while a build was running; it was stopped and its result is not current.",
        );
        stop_run(
            &mut inner,
            "Workspace closed while a program was running; it was stopped and its result is not current.",
        );
        stop_terminal(
            &mut inner,
            "Workspace closed while the shell was running; it was stopped and its output is not current.",
        );
        stop_tests(
            &mut inner,
            "Workspace closed while tests were being discovered; the discovery was stopped and its result is not current.",
        );
        stop_test_run(
            &mut inner,
            "Workspace closed while tests were running; the run was stopped and its results are not current.",
        );
        inner.lsp = None;
        inner.diagnostics.clear();
        inner.processes.lock().unwrap().mark_awaiting();
        inner.workspace = None;
        inner.trusted = false;
        inner.debugger_experimental = false;
        inner.trust_decision = None;
        inner.workspace_seq += 1;
        inner.context_seq += 1;
        inner.nuget_plan = None;
        inner.project_sdk = ProjectSdk::no_workspace();
        inner.evaluation = Evaluation::idle();
        inner.build = Arc::new(build::Build::idle());
        inner.run = Arc::new(run::Run::idle());
        inner.terminal = Arc::new(terminal::Terminal::idle());
        inner.tests = Arc::new(test_discovery::Tests::idle());
        inner.test_run = Arc::new(test_run::TestRun::idle());
        inner.dap = Arc::new(dap::Dap::idle());
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
        inner.nuget_plan = None;
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
        inner.nuget_plan = None;
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
        stop_build(
            &mut inner,
            "Trust was revoked while this build was running; it was stopped and its result is not current.",
        );
        stop_run(
            &mut inner,
            "Trust was revoked while this program was running; it was stopped and its result is not current.",
        );
        stop_terminal(
            &mut inner,
            "Trust was revoked while the shell was running; it was stopped and its output is not current.",
        );
        stop_tests(
            &mut inner,
            "Trust was revoked while tests were being discovered; the discovery was stopped and its result is not current.",
        );
        stop_test_run(
            &mut inner,
            "Trust was revoked while tests were running; the run was stopped and its results are not current.",
        );
        inner.diagnostics.mark_unavailable(
            diagnostics::LSP_ORIGIN,
            "Trust was revoked; analysis is unavailable and these diagnostics are not current. Editing stays available.",
        );
        inner.project_sdk = ProjectSdk::not_inspected(request.as_ref());
        inner.evaluation = Evaluation::idle();
        inner.processes.lock().unwrap().mark_awaiting();
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
        inner.nuget_plan = None;
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
            inner.build.mark_stale(
                "SDK inventory refreshed; the previous build used another toolchain.",
            );
            inner.tests.mark_stale(
                "SDK inventory refreshed; discover tests again with the new toolchain.",
            );
            inner.test_run.mark_stale(
                "SDK inventory refreshed; these test results used another toolchain.",
            );
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
            .lock()
            .unwrap()
            .spawn(&format!("probe:{kind}"), token.workspace_id, command)
            .map_err(|error| {
                HostError::new(
                    "operation_failed",
                    format!("Could not start probe process: {error}"),
                )
            })?;
        Ok(self.snapshot_value(&mut inner))
    }

    /// Development-only boundary: publishes a diagnostic without a range for the current
    /// session so the UI contract "no position, no navigation" can be exercised end to end.
    fn diagnostics_probe(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("diagnostics.probe")?;
        if std::env::var("OPENRIDR_ENABLE_PROBE").as_deref() != Ok("1") {
            return Err(HostError::new(
                "invalid_state",
                "diagnostics.probe is a development-only probe; set OPENRIDR_ENABLE_PROBE=1 to enable it.",
            ));
        }
        let rel = optional_str(&params, "path")
            .unwrap_or_else(|| "diagnostics-probe/NoPosition.cs".to_string());
        let mut inner = self.inner.lock().unwrap();
        accept_locked(&inner, token)?;
        let lsp = inner
            .lsp
            .clone()
            .ok_or_else(|| HostError::new("invalid_state", "Analysis is not running."))?;
        if !lsp.accepts_documents() {
            return Err(HostError::new(
                "invalid_state",
                "Analysis has not finished initializing; the probe was not stored.",
            ));
        }
        // A synthetic document keeps the probe from replacing a real publication for an
        // existing file; the UI contract under test is only "no position, no navigation".
        let publication = lsp::Publication {
            uri: "file:///openridr-probe/NoPosition.cs".to_string(),
            path: Some(rel),
            received_at: now_ms(),
            client_revision: None,
            server_version: Value::Null,
            session: lsp.session(),
            workspace_id: token.workspace_id,
            context_id: token.context_id,
            items: vec![lsp::DiagnosticItem {
                range: Value::Null,
                severity: Some(1),
                code: json!("PROBE0001"),
                source: Some("lsp".to_string()),
                message: "Development-only probe: this diagnostic has no published position."
                    .to_string(),
                code_description: None,
            }],
        };
        if !inner
            .diagnostics
            .ingest(&diagnostics::Origin::lsp(), &publication)
        {
            return Err(HostError::new(
                "stale_context",
                "The probe publication did not match the current analysis session.",
            ));
        }
        Ok(self.snapshot_value(&mut inner))
    }

    fn terminate_process(&self, params: Value) -> HostResult<Value> {
        let id = required_u64(&params, "id")?;
        let mut inner = self.inner.lock().unwrap();
        if inner.processes.lock().unwrap().terminate(id).is_none() {
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
        if inner.processes.lock().unwrap().keep(id).is_none() {
            return Err(HostError::new(
                "not_found",
                format!("No tracked process with id {id}."),
            ));
        }
        inner.terminal.keep(id);
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
        inner.build.mark_stale(
            "The project model was re-evaluated; build again to match the current evaluation.",
        );
        inner.tests.mark_stale(
            "The project model was re-evaluated; discover tests again to match the current evaluation.",
        );
        inner.test_run.mark_stale(
            "The project model was re-evaluated; these test results are not current.",
        );
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

        // Separate arguments and cwd via the shared dotnet environment helper.
        let process = dotnet_command(&command, &root, &sdk_root);
        let spawned = inner
            .processes
            .lock()
            .unwrap()
            .spawn_piped(&format!("restore:{scope}"), token.workspace_id, process);
        let piped = match spawned {
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
        let status = inner.processes.lock().unwrap().terminate_status(process_id);
        if let Some(status) = &status {
            inner.restore.observe(status);
        }
        inner.sync_restore();
        Ok(self.snapshot_value(&mut inner))
    }

    /// Explicit `dotnet build` of one console startup project from the current evaluation.
    /// Never restores, never saves a buffer, and never reuses a stale TargetPath.
    fn start_build(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("build.start")?;
        let mut inner = self.inner.lock().unwrap();

        if let Some(process_id) = inner.build.running_process_id() {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "A build is already running (process {process_id}). Cancel it before starting another."
                ),
            ));
        }
        if inner.restore.running_process_id().is_some() {
            return Err(HostError::new(
                "invalid_state",
                "A restore is running; wait for it to finish before building.",
            ));
        }
        let Some(workspace) = inner.workspace.as_ref() else {
            return Err(HostError::new("invalid_state", "No workspace is open."));
        };
        let is_solution = matches!(workspace.kind, WorkspaceKind::Solution);
        let workspace_file = workspace.path.clone();
        let root = workspace.root.clone();

        // The same policy as every executable consumer: no implicit restore, ever.
        restore::require_restored(&workspace_file, &root, is_solution, "build.start")?;

        let sdk = inner.project_sdk.clone();
        if sdk.state != "resolved" {
            return Err(HostError::new(
                if sdk.state == "sdk_missing" {
                    "sdk_missing"
                } else {
                    "invalid_state"
                },
                format!(
                    "Build was not started: the project SDK is not resolved ({}). {}",
                    sdk.state,
                    sdk.message
                        .clone()
                        .unwrap_or_else(|| "Resolve the project SDK before building.".to_string())
                ),
            ));
        }
        let Some(sdk_root) = sdk.root.clone().map(PathBuf::from) else {
            return Err(HostError::new(
                "invalid_state",
                "Build was not started: the resolved project SDK has no root path.",
            ));
        };
        let muxer = sdk_root.join(dotnet_exe());
        if !muxer.is_file() {
            return Err(HostError::new(
                "sdk_missing",
                format!(
                    "Build was not started: {} does not contain a .NET muxer.",
                    sdk_root.to_string_lossy()
                ),
            ));
        }

        let evaluation = inner.evaluation.clone();
        if evaluation.stale {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "Build was not started: the evaluated project model is stale ({}). Re-evaluate before building.",
                    evaluation
                        .stale_reason
                        .clone()
                        .unwrap_or_else(|| "no current evaluation".to_string())
                ),
            ));
        }
        if !matches!(evaluation.state, "ok" | "partial")
            || evaluation.workspace_id != Some(token.workspace_id)
            || evaluation.context_id != Some(token.context_id)
        {
            return Err(HostError::new(
                "invalid_state",
                "Build was not started: the current evaluation is not valid for this workspace/context. Evaluate the project model again.",
            ));
        }

        let configuration = optional_str(&params, "configuration")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| evaluation.context.configuration.clone());
        if !valid_selection(&configuration) {
            return Err(HostError::new(
                "invalid_state",
                "Configuration must not contain `;`, `=`, quotes or line breaks; MSBuild was not run.",
            ));
        }
        if configuration != evaluation.context.configuration {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "Build was not started: configuration {configuration} has not been evaluated. Re-evaluate with that configuration first."
                ),
            ));
        }
        let target_framework = match params.get("targetFramework") {
            Some(Value::String(value)) if !value.trim().is_empty() => {
                Some(value.trim().to_string())
            }
            Some(_) => None,
            None => evaluation.context.target_framework.clone(),
        };
        if target_framework.as_deref().is_some_and(|tfm| !valid_selection(tfm)) {
            return Err(HostError::new(
                "invalid_state",
                "TFM must not contain `;`, `=`, quotes or line breaks; MSBuild was not run.",
            ));
        }
        if target_framework != evaluation.context.target_framework {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "Build was not started: TFM {} has not been evaluated. Re-evaluate with that TFM first.",
                    target_framework.unwrap_or_else(|| "not selected".to_string())
                ),
            ));
        }

        let requested = optional_str(&params, "project").filter(|value| !value.trim().is_empty());
        let selected = build::select_startup(&evaluation.projects, requested.as_deref())?;
        let Some(target_path) = selected.target_path.clone() else {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "Build was not started: {} has no evaluated TargetPath.",
                    selected.name
                ),
            ));
        };
        let saved_note = optional_str(&params, "savedNote")
            .map(|note| {
                let note = note.trim().to_string();
                note.chars().take(200).collect::<String>()
            })
            .filter(|note| !note.is_empty());

        inner.build_seq += 1;
        let build_id = format!("build:{}:{}", token.workspace_id, inner.build_seq);
        let project_name = selected.name.clone();
        let sources = build::capture_sources(&root, &selected.path, &evaluation.projects);
        let mut command = vec![
            muxer.to_string_lossy().to_string(),
            "build".to_string(),
            root.join(&selected.path).to_string_lossy().to_string(),
            "-c".to_string(),
            configuration.clone(),
            "-nologo".to_string(),
            "-v:minimal".to_string(),
            "--no-restore".to_string(),
        ];
        if let Some(tfm) = &target_framework {
            command.push(format!("-p:TargetFramework={tfm}"));
        }

        inner.build = Arc::new(build::Build::begin(build::Start {
            revision: inner.build_seq,
            build_id,
            evaluation_revision: evaluation.revision,
            sources,
            project_name: selected.name.clone(),
            project_path: selected.path.clone(),
            project_kind: selected.kind,
            target_path: Some(target_path),
            configuration,
            target_framework,
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
            saved_note,
        }));

        // Separate arguments and cwd via the shared dotnet environment helper.
        let process = dotnet_command(&command, &root, &sdk_root);
        let spawned = inner.processes.lock().unwrap().spawn_piped(
            &format!("build:{project_name}"),
            token.workspace_id,
            process,
        );
        let piped = match spawned {
            Ok(piped) => piped,
            Err(error) => {
                inner.build.fail(format!("Could not start dotnet build: {error}"));
                return Ok(self.snapshot_value(&mut inner));
            }
        };
        inner.build.note_process(piped.id);
        let shared = inner.build.clone();
        if let Some(stdout) = piped.stdout {
            let sink = shared.clone();
            std::thread::spawn(move || build::pump(stdout, sink, false));
        }
        if let Some(stderr) = piped.stderr {
            let sink = shared.clone();
            std::thread::spawn(move || build::pump(stderr, sink, true));
        }
        Ok(self.snapshot_value(&mut inner))
    }

    /// Safety action: allowed without trust, like `processes.terminate`. It requests
    /// termination and reports what the registry observed; the raw log is kept.
    fn cancel_build(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        let Some(process_id) = inner.build.running_process_id() else {
            return Err(HostError::new("invalid_state", "No build is running."));
        };
        inner.build.request_cancel();
        let status = inner.processes.lock().unwrap().terminate_status(process_id);
        if let Some(status) = &status {
            inner.build.observe(status);
        }
        inner.sync_build();
        Ok(self.snapshot_value(&mut inner))
    }

    /// Real VSTest discovery of one explicitly selected test project. Never restores, never
    /// edits a package, and refuses MTP projects before any VSTest argv exists.
    fn start_tests_discovery(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("tests.discover")?;
        let mut inner = self.inner.lock().unwrap();

        if let Some(process_id) = inner.tests.running_process_id() {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "A test discovery is already running (process {process_id}). Cancel it before starting another."
                ),
            ));
        }
        if let Some(process_id) = inner.test_run.running_process_id() {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "A test run is running (process {process_id}); wait for it to finish before discovering again."
                ),
            ));
        }
        let scope = resolve_test_scope(&inner, token, &params, "tests.discover", "Test discovery")?;
        let TestScope {
            evaluation,
            project,
            target_framework,
            sdk_root,
            sdk_version,
            root,
        } = scope;

        inner.tests_seq += 1;
        let start = test_discovery::Start {
            revision: inner.tests_seq,
            discovery_id: format!("discovery:{}:{}", token.workspace_id, inner.tests_seq),
            project_name: project.name.clone(),
            project_path: project.path.clone(),
            target_framework: target_framework.clone(),
            command: Vec::new(),
            cwd: root.clone(),
            sdk_root: sdk_root.clone(),
            sdk_version,
            workspace_id: token.workspace_id,
            context_id: token.context_id,
        };

        // MTP gate: read the project text and the evaluated references before any VSTest
        // command exists. Nothing is converted or installed.
        let project_text = fs::read_to_string(root.join(&project.path)).unwrap_or_default();
        if test_discovery::mtp_detected(&project_text, &project.package_refs) {
            let notice = test_discovery::Start {
                command: Vec::new(),
                ..start
            };
            inner.tests = Arc::new(test_discovery::Tests::notice(
                notice,
                "mtp_unsupported",
                None,
                Some(test_discovery::MTP_MESSAGE.to_string()),
            ));
            return Ok(self.snapshot_value(&mut inner));
        }
        if let Some(guidance) = test_discovery::adapter_missing(&project.package_refs) {
            let notice = test_discovery::Start {
                command: Vec::new(),
                ..start
            };
            inner.tests = Arc::new(test_discovery::Tests::notice(
                notice,
                "adapter_missing",
                Some(guidance),
                None,
            ));
            return Ok(self.snapshot_value(&mut inner));
        }

        validate_test_tfm(&project, &target_framework, "Test discovery")?;

        let mut command = vec![
            sdk_root.join(dotnet_exe()).to_string_lossy().to_string(),
            "test".to_string(),
            root.join(&project.path).to_string_lossy().to_string(),
            "-c".to_string(),
            evaluation.context.configuration.clone(),
            "--no-restore".to_string(),
            "--list-tests".to_string(),
            "--nologo".to_string(),
            "-v:minimal".to_string(),
        ];
        if let Some(tfm) = &target_framework {
            command.push("-f".to_string());
            command.push(tfm.clone());
        }
        let start = test_discovery::Start {
            command: command.clone(),
            ..start
        };
        inner.tests = Arc::new(test_discovery::Tests::begin(start));

        // Separate arguments and cwd via the shared dotnet environment helper.
        let process = dotnet_command(&command, &root, &sdk_root);
        let project_name = project.name.clone();
        let spawned = inner.processes.lock().unwrap().spawn_piped(
            &format!("tests:{project_name}"),
            token.workspace_id,
            process,
        );
        let piped = match spawned {
            Ok(piped) => piped,
            Err(error) => {
                inner
                    .tests
                    .fail(format!("Could not start dotnet test --list-tests: {error}"));
                return Ok(self.snapshot_value(&mut inner));
            }
        };
        inner.tests.note_process(piped.id);
        let shared = inner.tests.clone();
        if let Some(stdout) = piped.stdout {
            let sink = shared.clone();
            std::thread::spawn(move || test_discovery::pump(stdout, sink, false));
        }
        if let Some(stderr) = piped.stderr {
            let sink = shared.clone();
            std::thread::spawn(move || test_discovery::pump(stderr, sink, true));
        }
        Ok(self.snapshot_value(&mut inner))
    }

    /// Safety action, allowed without trust: request termination and keep whatever the list
    /// already printed. The exit shown is only what the registry observed.
    fn cancel_tests_discovery(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        let Some(process_id) = inner.tests.running_process_id() else {
            return Err(HostError::new(
                "invalid_state",
                "No test discovery is running.",
            ));
        };
        inner.tests.request_cancel();
        let status = inner.processes.lock().unwrap().terminate_status(process_id);
        if let Some(status) = &status {
            inner.tests.observe(status);
        }
        inner.sync_tests();
        Ok(self.snapshot_value(&mut inner))
    }

    /// Run all or exactly the validated selected identities of the current discovery. Never
    /// restores, builds or installs a package; the TRX logger is VSTest's built-in one.
    fn start_test_run(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("tests.run")?;
        let mut inner = self.inner.lock().unwrap();

        if let Some(process_id) = inner.test_run.running_process_id() {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "A test run is already running (process {process_id}). Cancel it before starting another."
                ),
            ));
        }
        if let Some(process_id) = inner.tests.running_process_id() {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "A test discovery is running (process {process_id}); wait for it to finish before running tests."
                ),
            ));
        }

        let scope = resolve_test_scope(&inner, token, &params, "tests.run", "Test run")?;
        let TestScope {
            project,
            target_framework,
            sdk_root,
            root,
            ..
        } = scope;
        let configuration = inner.evaluation.context.configuration.clone();

        // MTP gate stays before every VSTest argv, including direct host access.
        let project_text = fs::read_to_string(root.join(&project.path)).unwrap_or_default();
        if test_discovery::mtp_detected(&project_text, &project.package_refs) {
            return Err(HostError::with_details(
                "invalid_state",
                test_discovery::MTP_MESSAGE,
                json!({ "reason": "mtp_unsupported" }),
            ));
        }

        validate_test_tfm(&project, &target_framework, "Test run")?;

        let discovery = inner.tests.view().discovery;
        if discovery.revision == 0 || discovery.state != "completed" {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Test run was not started: there is no completed current discovery ({}). Discover the tests first.",
                    discovery.state
                ),
                json!({
                    "reason": if discovery.revision == 0 { "no_discovery" } else { "discovery_not_completed" },
                    "action": { "method": "tests.discover", "params": {} },
                }),
            ));
        }
        if discovery.stale || discovery.sealed {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Test run was not started: the discovery is stale ({}). Discover the tests again.",
                    discovery
                        .stale_reason
                        .clone()
                        .unwrap_or_else(|| "not current".to_string())
                ),
                json!({
                    "reason": "discovery_stale",
                    "action": { "method": "tests.discover", "params": {} },
                }),
            ));
        }
        if discovery.workspace_id != Some(token.workspace_id)
            || discovery.context_id != Some(token.context_id)
        {
            return Err(HostError::with_details(
                "invalid_state",
                "Test run was not started: the discovery belongs to another workspace/context. Discover the tests again.",
                json!({
                    "reason": "discovery_context_mismatch",
                    "action": { "method": "tests.discover", "params": {} },
                }),
            ));
        }
        if discovery.project.as_ref().map(|project| project.path.as_str())
            != Some(project.path.as_str())
            || discovery.target_framework != target_framework
        {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Test run was not started: the current discovery is for {} · {}. Discover this project/TFM first.",
                    discovery
                        .project
                        .as_ref()
                        .map(|project| project.name.clone())
                        .unwrap_or_else(|| "unknown project".to_string()),
                    discovery
                        .target_framework
                        .clone()
                        .unwrap_or_else(|| "TFM not selected".to_string())
                ),
                json!({
                    "reason": "discovery_scope_mismatch",
                    "action": { "method": "tests.discover", "params": {} },
                }),
            ));
        }

        let selection = match required_str(&params, "selection")?.as_str() {
            "all" => "all",
            "selected" => "selected",
            other => {
                return Err(HostError::with_details(
                    "invalid_state",
                    format!("Unknown test run selection {other:?}; use `all` or `selected`."),
                    json!({ "reason": "invalid_selection" }),
                ))
            }
        };
        let items = discovery.items.clone();
        let mut test_ids: Vec<String> = Vec::new();
        let mut filter: Option<String> = None;
        if selection == "selected" {
            let raw_ids = match params.get("testIds") {
                Some(value) => match value.as_array() {
                    Some(array) => array,
                    None => {
                        return Err(HostError::with_details(
                            "invalid_state",
                            "The selected run must provide testIds as an array of strings from the current discovery.",
                            json!({ "reason": "invalid_selection" }),
                        ))
                    }
                },
                None => {
                    return Err(HostError::with_details(
                        "invalid_state",
                        "The selected run has no test id list. An empty selection never runs all tests.",
                        json!({ "reason": "empty_selection" }),
                    ))
                }
            };
            let ids: Vec<String> = raw_ids
                .iter()
                .map(|value| value.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    HostError::with_details(
                        "invalid_state",
                        "Every selected test id must be a string from the current discovery.",
                        json!({ "reason": "invalid_selection" }),
                    )
                })?;
            let chosen = test_run::validate_selection(&items, &ids).map_err(|error| {
                HostError::with_details(
                    "invalid_state",
                    error.message(),
                    json!({ "reason": error.reason() }),
                )
            })?;
            filter = Some(test_run::filter_expression(&chosen).map_err(|message| {
                HostError::with_details(
                    "invalid_state",
                    message,
                    json!({ "reason": "filter_refused" }),
                )
            })?);
            test_ids = ids;
        }

        inner.test_run_seq += 1;
        let results_directory = tools::app_home()
            .join("test-results")
            .join(format!(
                "{}-{}-{}-{}",
                token.workspace_id,
                inner.test_run_seq,
                now_ms(),
                std::process::id()
            ));
        if let Err(error) = fs::create_dir_all(&results_directory) {
            return Err(HostError::new(
                "operation_failed",
                format!(
                    "Test run was not started: could not create the TRX results directory {}: {error}",
                    results_directory.to_string_lossy()
                ),
            ));
        }

        let mut command = vec![
            sdk_root.join(dotnet_exe()).to_string_lossy().to_string(),
            "test".to_string(),
            root.join(&project.path).to_string_lossy().to_string(),
            "-c".to_string(),
            configuration.clone(),
            "--no-build".to_string(),
            "--no-restore".to_string(),
            "--nologo".to_string(),
            "-v:minimal".to_string(),
        ];
        if let Some(tfm) = &target_framework {
            command.push("-f".to_string());
            command.push(tfm.clone());
        }
        if let Some(filter) = &filter {
            command.push("--filter".to_string());
            command.push(filter.clone());
        }
        command.push("--logger".to_string());
        command.push(format!("trx;LogFileName={}", test_run::TRX_FILE));
        command.push("--results-directory".to_string());
        command.push(results_directory.to_string_lossy().to_string());

        let start = test_run::Start {
            revision: inner.test_run_seq,
            run_id: format!("testrun:{}:{}", token.workspace_id, inner.test_run_seq),
            project_name: project.name.clone(),
            project_path: project.path.clone(),
            discovery_id: discovery.discovery_id.clone().unwrap_or_default(),
            discovery_revision: discovery.revision,
            configuration,
            target_framework: target_framework.clone(),
            selection,
            selected_test_ids: test_ids,
            command: command.clone(),
            cwd: root.to_string_lossy().to_string(),
            workspace_root: root.clone(),
            results_directory,
            workspace_id: token.workspace_id,
            context_id: token.context_id,
            cases: items,
        };
        inner.test_run = Arc::new(test_run::TestRun::begin(start));

        let process = dotnet_command(&command, &root, &sdk_root);
        let project_name = project.name.clone();
        let spawned = inner.processes.lock().unwrap().spawn_piped(
            &format!("tests:run:{project_name}"),
            token.workspace_id,
            process,
        );
        let piped = match spawned {
            Ok(piped) => piped,
            Err(error) => {
                inner
                    .test_run
                    .fail(format!("Could not start dotnet test: {error}"));
                return Ok(self.snapshot_value(&mut inner));
            }
        };
        inner.test_run.note_process(piped.id);
        let shared = inner.test_run.clone();
        if let Some(stdout) = piped.stdout {
            let sink = shared.clone();
            std::thread::spawn(move || test_run::pump(stdout, sink, false));
        }
        if let Some(stderr) = piped.stderr {
            let sink = shared.clone();
            std::thread::spawn(move || test_run::pump(stderr, sink, true));
        }
        Ok(self.snapshot_value(&mut inner))
    }

    /// Safety action, allowed without trust: request termination of the test run group and
    /// keep whatever TRX/log exists. No missing result becomes passed.
    fn cancel_test_run(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        let Some(process_id) = inner.test_run.running_process_id() else {
            return Err(HostError::new("invalid_state", "No test run is running."));
        };
        inner.test_run.request_cancel();
        let status = inner.processes.lock().unwrap().terminate_status(process_id);
        if let Some(status) = &status {
            inner.test_run.observe(status);
        }
        inner.sync_test_run();
        Ok(self.snapshot_value(&mut inner))
    }

    fn start_run(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("run.start")?;
        let mut inner = self.inner.lock().unwrap();

        if let Some(process_id) = inner.run.running_process_id() {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "A program is already running (process {process_id}). Stop it before starting another."
                ),
            ));
        }
        if let Some(process_id) = inner.build.running_process_id() {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "A build is running (process {process_id}); wait for it to finish before running."
                ),
            ));
        }
        let Some(workspace) = inner.workspace.as_ref() else {
            return Err(HostError::new("invalid_state", "No workspace is open."));
        };
        let is_solution = matches!(workspace.kind, WorkspaceKind::Solution);
        let workspace_file = workspace.path.clone();
        let root = workspace.root.clone();

        // The active configuration's argv, cwd and environment are validated before any
        // restore/build gate so an invalid launch explains its own field first.
        let options = run_configs::launch_options(&params, &root)?;

        // Same policy as build/tests/debug: Run never restores implicitly, and a build whose
        // restore assets were invalidated afterwards is not a licence to launch.
        restore::require_restored(&workspace_file, &root, is_solution, "run.start")?;

        let evaluation = inner.evaluation.clone();
        if evaluation.stale {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Run was not started: the evaluated project model is stale ({}). Re-evaluate before running.",
                    evaluation
                        .stale_reason
                        .clone()
                        .unwrap_or_else(|| "no current evaluation".to_string())
                ),
                json!({ "reason": "evaluation_stale", "action": { "method": "evaluation.run", "params": {} } }),
            ));
        }
        if !matches!(evaluation.state, "ok" | "partial")
            || evaluation.workspace_id != Some(token.workspace_id)
            || evaluation.context_id != Some(token.context_id)
        {
            return Err(HostError::with_details(
                "invalid_state",
                "Run was not started: the current evaluation is not valid for this workspace/context. Evaluate the project model again.",
                json!({ "reason": "evaluation_required", "action": { "method": "evaluation.run", "params": {} } }),
            ));
        }

        let configuration = optional_str(&params, "configuration")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| evaluation.context.configuration.clone());
        if !valid_selection(&configuration) {
            return Err(HostError::new(
                "invalid_state",
                "Configuration must not contain `;`, `=`, quotes or line breaks; no program was started.",
            ));
        }
        if configuration != evaluation.context.configuration {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Run was not started: configuration {configuration} has not been evaluated. Re-evaluate with that configuration first."
                ),
                json!({ "reason": "configuration_not_evaluated", "action": { "method": "evaluation.run", "params": {} } }),
            ));
        }
        let target_framework = match params.get("targetFramework") {
            Some(Value::String(value)) if !value.trim().is_empty() => {
                Some(value.trim().to_string())
            }
            Some(_) => None,
            None => evaluation.context.target_framework.clone(),
        };
        if target_framework
            .as_deref()
            .is_some_and(|tfm| !valid_selection(tfm))
        {
            return Err(HostError::new(
                "invalid_state",
                "TFM must not contain `;`, `=`, quotes or line breaks; no program was started.",
            ));
        }
        if target_framework != evaluation.context.target_framework {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Run was not started: TFM {} has not been evaluated. Re-evaluate with that TFM first.",
                    target_framework.unwrap_or_else(|| "not selected".to_string())
                ),
                json!({ "reason": "tfm_not_evaluated", "action": { "method": "evaluation.run", "params": {} } }),
            ));
        }

        let requested = optional_str(&params, "project").filter(|value| !value.trim().is_empty());
        let selected =
            build::select_startup_for(&evaluation.projects, requested.as_deref(), "run")?;
        let Some(evaluated_target) = selected.target_path.clone() else {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Run was not started: {} has no evaluated TargetPath. Re-evaluate the project model.",
                    selected.name
                ),
                json!({ "reason": "target_missing", "action": { "method": "evaluation.run", "params": {} } }),
            ));
        };

        let build_action = json!({
            "method": "build.start",
            "params": {
                "project": selected.path,
                "configuration": configuration,
                "targetFramework": target_framework,
            },
        });
        let build = inner.build.view();
        if build.revision == 0 {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Run was not started: there is no build of {} yet. Build it first; Run never builds or restores implicitly.",
                    selected.name
                ),
                json!({ "reason": "no_build", "action": build_action }),
            ));
        }
        if build.sealed || build.stale {
            return Err(HostError::with_details(
                "build_stale",
                format!(
                    "Run was not started: the saved build is stale ({}). Rebuild before running.",
                    build
                        .stale_reason
                        .clone()
                        .unwrap_or_else(|| "stale build".to_string())
                ),
                json!({
                    "reason": "build_stale",
                    "buildId": build.build_id,
                    "staleReason": build.stale_reason,
                    "buildLog": log_tail(&build.log, 30),
                    "action": build_action,
                }),
            ));
        }
        if build.state != "ok" || build.exit != Some(0) {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Run was not started: the last build did not succeed ({}). Fix the build before running.",
                    if build.state == "idle" {
                        "no build".to_string()
                    } else {
                        let exit = build
                            .exit
                            .map(|exit| format!("exit {exit}"))
                            .unwrap_or_else(|| "exit unknown".to_string());
                        format!("{} · {exit}", build.state)
                    }
                ),
                json!({
                    "reason": "build_failed",
                    "buildId": build.build_id,
                    "exit": build.exit,
                    "buildLog": log_tail(&build.log, 30),
                    "action": build_action,
                }),
            ));
        }
        if build.workspace_id != Some(token.workspace_id)
            || build.context_id != Some(token.context_id)
        {
            return Err(HostError::with_details(
                "build_stale",
                "Run was not started: the saved build belongs to an older workspace or context. Rebuild before running.",
                json!({
                    "reason": "context_changed",
                    "buildId": build.build_id,
                    "action": build_action,
                }),
            ));
        }
        if inner.build.evaluation_revision() != evaluation.revision {
            return Err(HostError::with_details(
                "build_stale",
                "Run was not started: the project model was re-evaluated after this build. Rebuild before running.",
                json!({
                    "reason": "evaluation_changed",
                    "buildId": build.build_id,
                    "evaluationRevision": evaluation.revision,
                    "action": build_action,
                }),
            ));
        }
        let Some(build_project) = build.project.as_ref() else {
            return Err(HostError::with_details(
                "build_stale",
                "Run was not started: the saved build has no startup project identity. Rebuild before running.",
                json!({ "reason": "build_project_missing", "action": build_action }),
            ));
        };
        if build_project.path != selected.path {
            return Err(HostError::with_details(
                "build_stale",
                format!(
                    "Run was not started: the saved build is for {}, not {}. Build the selected startup project before running.",
                    build_project.name, selected.name
                ),
                json!({
                    "reason": "project_changed",
                    "buildId": build.build_id,
                    "buildProject": build_project.path,
                    "action": build_action,
                }),
            ));
        }
        if build.configuration != configuration || build.target_framework != target_framework {
            return Err(HostError::with_details(
                "build_stale",
                format!(
                    "Run was not started: the saved build used {} / {}; rebuild with the current configuration and TFM.",
                    build.configuration,
                    build
                        .target_framework
                        .clone()
                        .unwrap_or_else(|| "not selected".to_string())
                ),
                json!({
                    "reason": "build_context_mismatch",
                    "buildId": build.build_id,
                    "action": build_action,
                }),
            ));
        }
        let Some(built_target) = build_project.target_path.clone() else {
            return Err(HostError::with_details(
                "build_stale",
                "Run was not started: the saved build has no TargetPath. Rebuild before running.",
                json!({ "reason": "target_missing", "buildId": build.build_id, "action": build_action }),
            ));
        };
        if built_target != evaluated_target {
            return Err(HostError::with_details(
                "build_stale",
                "Run was not started: the evaluated TargetPath changed after the build. Rebuild before running.",
                json!({
                    "reason": "target_changed",
                    "buildId": build.build_id,
                    "targetPath": built_target,
                    "evaluatedTargetPath": evaluated_target,
                    "action": build_action,
                }),
            ));
        }
        let changed = inner.build.changed_sources(&root);
        if !changed.is_empty() {
            return Err(HostError::with_details(
                "build_stale",
                format!(
                    "Run was not started: {} saved source{} changed after this build ({}). Rebuild before running.",
                    changed.len(),
                    if changed.len() == 1 { "" } else { "s" },
                    changed.join(", ")
                ),
                json!({
                    "reason": "source_changed",
                    "buildId": build.build_id,
                    "changedSources": changed,
                    "buildLog": log_tail(&build.log, 30),
                    "action": build_action,
                }),
            ));
        }

        let sdk = inner.project_sdk.clone();
        if sdk.state != "resolved" {
            return Err(HostError::with_details(
                if sdk.state == "sdk_missing" {
                    "sdk_missing"
                } else {
                    "invalid_state"
                },
                format!(
                    "Run was not started: the project SDK is not resolved ({}). {}",
                    sdk.state,
                    sdk.message.clone().unwrap_or_else(|| {
                        "Resolve the project SDK before running.".to_string()
                    })
                ),
                json!({ "reason": "sdk_unresolved", "action": { "method": "build.start", "params": {} } }),
            ));
        }
        let Some(sdk_root) = sdk.root.clone().map(PathBuf::from) else {
            return Err(HostError::new(
                "invalid_state",
                "Run was not started: the resolved project SDK has no root path.",
            ));
        };
        let muxer = sdk_root.join(dotnet_exe());
        let Some(project_directory) = root.join(&selected.path).parent().map(Path::to_path_buf)
        else {
            return Err(HostError::new(
                "invalid_state",
                "Run was not started: the startup project has no directory.",
            ));
        };
        let project_directory = options
            .working_directory
            .clone()
            .unwrap_or(project_directory);
        let mut command = run::launch_command(Path::new(&built_target), &muxer)?;
        command.extend(options.args.iter().cloned());

        inner.run_seq += 1;
        let run_id = format!("run:{}:{}", token.workspace_id, inner.run_seq);
        inner.run = Arc::new(run::Run::begin(run::Start {
            revision: inner.run_seq,
            run_id,
            project_name: selected.name.clone(),
            project_path: selected.path.clone(),
            target_path: built_target,
            build_id: build.build_id.clone().unwrap_or_default(),
            configuration: configuration.clone(),
            target_framework: target_framework.clone(),
            command: command.clone(),
            cwd: project_directory.to_string_lossy().to_string(),
            workspace_id: token.workspace_id,
            context_id: token.context_id,
        }));

        // Separate arguments and cwd via the shared dotnet environment helper.
        let mut process = dotnet_command(&command, &project_directory, &sdk_root);
        for (key, value) in &options.env {
            process.env(key, value);
        }
        let spawned = inner.processes.lock().unwrap().spawn_piped(
            &format!("run:{}", selected.name),
            token.workspace_id,
            process,
        );
        let piped = match spawned {
            Ok(piped) => piped,
            Err(error) => {
                inner
                    .run
                    .fail(format!("Could not start the program: {error}"));
                return Ok(self.snapshot_value(&mut inner));
            }
        };
        inner.run.note_process(piped.id);
        let shared = inner.run.clone();
        if let Some(stdout) = piped.stdout {
            let sink = shared.clone();
            std::thread::spawn(move || run::pump(stdout, sink, false));
        }
        if let Some(stderr) = piped.stderr {
            let sink = shared.clone();
            std::thread::spawn(move || run::pump(stderr, sink, true));
        }
        Ok(self.snapshot_value(&mut inner))
    }

    /// Safety action: allowed without trust. It requests group termination and reports what
    /// the registry observed, keeping the log and the exit code (or its absence).
    fn cancel_run(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        let Some(process_id) = inner.run.running_process_id() else {
            return Err(HostError::new("invalid_state", "No program is running."));
        };
        inner.run.request_cancel();
        let status = inner.processes.lock().unwrap().terminate_status(process_id);
        if let Some(status) = &status {
            inner.run.observe(status);
        }
        inner.sync_run();
        Ok(self.snapshot_value(&mut inner))
    }

    /// One real PTY shell after trust. The gate runs before anything observable: a direct
    /// call in Restricted mode spawns no shell. Shell and cwd are explicit in the record.
    fn start_terminal(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("terminal.start")?;
        let (cols, rows) = terminal::clamp_size(
            params.get("cols").and_then(Value::as_u64),
            params.get("rows").and_then(Value::as_u64),
        );
        let mut inner = self.inner.lock().unwrap();
        accept_locked(&inner, token)?;
        if let Some(process_id) = inner.terminal.running_process_id() {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "A shell is already running (process {process_id}). Close it before starting another."
                ),
            ));
        }
        let root = inner
            .workspace
            .as_ref()
            .map(|workspace| workspace.root.clone())
            .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
        let shell = terminal::discover_shell()?;
        let revision = inner.terminal_seq + 1;
        let start = terminal::Start {
            revision,
            shell,
            cwd: root,
            cols,
            rows,
            workspace_id: token.workspace_id,
            context_id: token.context_id,
        };
        let name = start
            .shell
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "shell".to_string());
        let session = match terminal::spawn(start.clone()) {
            Ok(session) => session,
            Err(error) => {
                inner.terminal = Arc::new(terminal::Terminal::failed(start, error.message));
                return Ok(self.snapshot_value(&mut inner));
            }
        };
        let terminal = Arc::new(session.terminal);
        let (process_id, exit_slot) = inner.processes.lock().unwrap().register_pty(
            &format!("terminal:{name}"),
            token.workspace_id,
            session.pid,
            session.killer,
        );
        terminal.note_process(process_id);
        let sink = terminal.clone();
        std::thread::spawn(move || terminal::pump(session.reader, sink));
        let mut child = session.child;
        std::thread::spawn(move || {
            // The waiter is the only observer of the shell exit; the registry reads the slot.
            // A shell killed by a signal has no exit code, so `exit` stays unknown (null).
            let exit = match child.wait() {
                Ok(status) if status.signal().is_none() => Some(status.exit_code() as i32),
                Ok(_) => None,
                Err(_) => None,
            };
            *exit_slot.lock().unwrap() = Some(exit);
        });
        inner.terminal_seq = revision;
        inner.terminal = terminal;
        Ok(self.snapshot_value(&mut inner))
    }

    /// Raw bytes to the PTY. The UI sends the typed line plus `\r`; Ctrl+C is byte 0x03.
    fn write_terminal(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("terminal.write")?;
        let data = required_str(&params, "data")?;
        if data.is_empty() {
            return Err(HostError::new(
                "invalid_state",
                "Nothing was sent: the data is empty.",
            ));
        }
        if data.len() > 8_192 {
            return Err(HostError::new(
                "invalid_state",
                "Nothing was sent: one write is limited to 8192 bytes.",
            ));
        }
        let mut inner = self.inner.lock().unwrap();
        accept_locked(&inner, token)?;
        inner.terminal.write(&data)?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn resize_terminal(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("terminal.resize")?;
        let cols = required_u64(&params, "cols")?;
        let rows = required_u64(&params, "rows")?;
        let (cols, rows) = terminal::clamp_size(Some(cols), Some(rows));
        let mut inner = self.inner.lock().unwrap();
        accept_locked(&inner, token)?;
        inner.terminal.resize(cols, rows)?;
        Ok(self.snapshot_value(&mut inner))
    }

    /// Safety action: allowed without trust, exactly like `run.cancel`. It terminates the
    /// shell group, reports what the registry observed and keeps the log and exit.
    fn stop_terminal_session(&self, params: Value) -> HostResult<Value> {
        if params.get("confirm") != Some(&Value::Bool(true)) {
            return Err(HostError::new(
                "confirmation_required",
                "Closing a running shell needs an explicit confirmation; keep the terminal or terminate the shell.",
            ));
        }
        let mut inner = self.inner.lock().unwrap();
        let Some(process_id) = inner.terminal.running_process_id() else {
            return Err(HostError::new(
                "invalid_state",
                "The shell is not running; there is nothing to close.",
            ));
        };
        inner.terminal.signal_foreground();
        let status = inner.processes.lock().unwrap().terminate_status(process_id);
        if let Some(status) = &status {
            inner.terminal.complete(status);
        }
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
            &mut inner.processes.lock().unwrap(),
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
            &mut inner.processes.lock().unwrap(),
        )?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn cancel_tool(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        let tools = inner.tools.clone();
        tools.cancel(&mut inner.processes.lock().unwrap())?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn plan_debugger(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        let plan = debugger::plan(
            std::env::consts::OS,
            std::env::consts::ARCH,
            inner.debugger_experimental,
        );
        inner.debugger.set_plan(plan);
        Ok(self.snapshot_value(&mut inner))
    }

    fn install_debugger(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("debug.install")?;
        if params.get("confirm") != Some(&Value::Bool(true)) {
            return Err(HostError::new(
                "confirmation_required",
                "Installing netcoredbg needs an explicit confirmation of the current plan.",
            ));
        }
        let plan_id = required_str(&params, "planId")?;
        let mut inner = self.inner.lock().unwrap();
        let plan = debugger::plan(
            std::env::consts::OS,
            std::env::consts::ARCH,
            inner.debugger_experimental,
        );
        if plan.plan_id != plan_id {
            inner.debugger.set_plan(plan.clone());
            return Err(HostError::with_details(
                "confirmation_required",
                "The netcoredbg install plan changed since it was shown; review and confirm the new plan. Nothing was downloaded or executed.",
                json!({ "plan": plan }),
            ));
        }
        let debugger = inner.debugger.clone();
        debugger.begin_install(plan, token.workspace_id, token.context_id)?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn retry_debugger(&self) -> HostResult<Value> {
        let token = self.require_trust("debug.retry")?;
        let mut inner = self.inner.lock().unwrap();
        let plan = debugger::plan(
            std::env::consts::OS,
            std::env::consts::ARCH,
            inner.debugger_experimental,
        );
        let Some(confirmed) = inner.debugger.confirmed_plan_id() else {
            return Err(HostError::new(
                "invalid_state",
                "Review and confirm the netcoredbg install plan before retrying; no plan was confirmed.",
            ));
        };
        if plan.plan_id != confirmed {
            inner.debugger.set_plan(plan.clone());
            return Err(HostError::with_details(
                "confirmation_required",
                "The netcoredbg install plan changed since it was confirmed; review and confirm the new plan. Nothing was downloaded or executed.",
                json!({ "plan": plan }),
            ));
        }
        let debugger = inner.debugger.clone();
        debugger.begin_install(plan, token.workspace_id, token.context_id)?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn cancel_debugger(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        let debugger = inner.debugger.clone();
        debugger.cancel()?;
        Ok(self.snapshot_value(&mut inner))
    }

    /// Launches the evaluated managed assembly under the verified adapter. Every gate is
    /// evaluated here: trust, adapter/platform/flag, a current successful Debug build with the
    /// same identity as the current evaluation, unchanged saved sources, and a managed
    /// assembly path from the evaluation. A failure names the reason and spawns nothing.
    fn start_debug(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("debug.start")?;
        let mut inner = self.inner.lock().unwrap();
        if inner.dap.is_active() {
            return Err(HostError::new(
                "invalid_state",
                "A debug session is already active; stop it before starting another.",
            ));
        }
        if let Some(process_id) = inner.build.running_process_id() {
            return Err(HostError::new(
                "invalid_state",
                format!("A build is running (process {process_id}); wait for it to finish."),
            ));
        }
        if inner.restore.running_process_id().is_some() {
            return Err(HostError::new(
                "invalid_state",
                "A restore is running; wait for it to finish before debugging.",
            ));
        }

        let Some(workspace) = inner.workspace.as_ref() else {
            return Err(HostError::new("invalid_state", "No workspace is open."));
        };
        let root = workspace.root.clone();
        // The active configuration is validated before the adapter/build gates so a bad cwd,
        // argument or .env line is reported as that field instead of an unrelated failure.
        let options = run_configs::launch_options(&params, &root)?;
        // Same restore policy as every executable consumer: Debug never restores implicitly.
        restore::require_restored(
            &workspace.path,
            &root,
            matches!(workspace.kind, WorkspaceKind::Solution),
            "debug.start",
        )?;

        let adapter = debugger::ready_adapter(
            std::env::consts::OS,
            std::env::consts::ARCH,
            inner.debugger_experimental,
        )?;
        let evaluation = inner.evaluation.clone();
        let build_action = |project: &str, configuration: &str, tfm: Option<&str>| {
            json!({
                "method": "build.start",
                "params": {
                    "project": project,
                    "configuration": configuration,
                    "targetFramework": tfm,
                },
            })
        };
        if evaluation.stale {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Debug was not started: the evaluated project model is stale ({}). Re-evaluate before debugging.",
                    evaluation
                        .stale_reason
                        .clone()
                        .unwrap_or_else(|| "no current evaluation".to_string())
                ),
                json!({ "reason": "evaluation_stale", "action": { "method": "evaluation.run", "params": {} } }),
            ));
        }
        if !matches!(evaluation.state, "ok" | "partial")
            || evaluation.workspace_id != Some(token.workspace_id)
            || evaluation.context_id != Some(token.context_id)
        {
            return Err(HostError::with_details(
                "invalid_state",
                "Debug was not started: the current evaluation is not valid for this workspace/context. Evaluate the project model again.",
                json!({ "reason": "evaluation_required", "action": { "method": "evaluation.run", "params": {} } }),
            ));
        }
        let configuration = optional_str(&params, "configuration")
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| evaluation.context.configuration.clone());
        if !valid_selection(&configuration) || configuration != evaluation.context.configuration {
            return Err(HostError::with_details(
                "invalid_state",
                "Debug was not started: the configuration has not been evaluated. Re-evaluate with that configuration first.",
                json!({ "reason": "configuration_not_evaluated", "action": { "method": "evaluation.run", "params": {} } }),
            ));
        }
        if !configuration.eq_ignore_ascii_case("Debug") {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Debug was not started: the evaluated configuration is {configuration}. Debug requires a Debug build; select Debug and evaluate again."
                ),
                json!({ "reason": "debug_configuration_required", "action": { "method": "evaluation.run", "params": {} } }),
            ));
        }
        let target_framework = match params.get("targetFramework") {
            Some(Value::String(value)) if !value.trim().is_empty() => Some(value.trim().to_string()),
            Some(_) => None,
            None => evaluation.context.target_framework.clone(),
        };
        if target_framework
            .as_deref()
            .is_some_and(|tfm| !valid_selection(tfm))
            || target_framework != evaluation.context.target_framework
        {
            return Err(HostError::with_details(
                "invalid_state",
                "Debug was not started: the TFM has not been evaluated. Re-evaluate with that TFM first.",
                json!({ "reason": "tfm_not_evaluated", "action": { "method": "evaluation.run", "params": {} } }),
            ));
        }
        let requested = optional_str(&params, "project").filter(|value| !value.trim().is_empty());
        let selected = build::select_startup_for(&evaluation.projects, requested.as_deref(), "debug")?;
        let Some(assembly) = selected.assembly_path.clone() else {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Debug was not started: {} has no evaluated managed assembly. Re-evaluate the project model.",
                    selected.name
                ),
                json!({ "reason": "assembly_missing", "action": { "method": "evaluation.run", "params": {} } }),
            ));
        };
        let build = inner.build.view();
        let action = build_action(
            &selected.path,
            &configuration,
            target_framework.as_deref(),
        );
        if build.revision == 0 {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Debug was not started: there is no build of {} yet. Build it first; Debug never builds or restores implicitly.",
                    selected.name
                ),
                json!({ "reason": "no_build", "action": action }),
            ));
        }
        if build.sealed || build.stale {
            return Err(HostError::with_details(
                "build_stale",
                format!(
                    "Debug was not started: the saved build is stale ({}). Rebuild before debugging.",
                    build
                        .stale_reason
                        .clone()
                        .unwrap_or_else(|| "stale build".to_string())
                ),
                json!({
                    "reason": "build_stale",
                    "buildId": build.build_id,
                    "staleReason": build.stale_reason,
                    "buildLog": log_tail(&build.log, 30),
                    "action": action,
                }),
            ));
        }
        if build.state != "ok" || build.exit != Some(0) {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Debug was not started: the last build did not succeed ({}). Fix the build before debugging.",
                    if build.state == "idle" {
                        "no build".to_string()
                    } else {
                        let exit = build
                            .exit
                            .map(|exit| format!("exit {exit}"))
                            .unwrap_or_else(|| "exit unknown".to_string());
                        format!("{} · {exit}", build.state)
                    }
                ),
                json!({
                    "reason": "build_failed",
                    "buildId": build.build_id,
                    "exit": build.exit,
                    "buildLog": log_tail(&build.log, 30),
                    "action": action,
                }),
            ));
        }
        if build.workspace_id != Some(token.workspace_id)
            || build.context_id != Some(token.context_id)
        {
            return Err(HostError::with_details(
                "build_stale",
                "Debug was not started: the saved build belongs to an older workspace or context. Rebuild before debugging.",
                json!({ "reason": "context_changed", "buildId": build.build_id, "action": action }),
            ));
        }
        if !build.configuration.eq_ignore_ascii_case("Debug") {
            return Err(HostError::with_details(
                "build_stale",
                format!(
                    "Debug was not started: the saved build used {}. Build with Debug before debugging.",
                    build.configuration
                ),
                json!({ "reason": "build_not_debug", "buildId": build.build_id, "action": action }),
            ));
        }
        if inner.build.evaluation_revision() != evaluation.revision {
            return Err(HostError::with_details(
                "build_stale",
                "Debug was not started: the project model was re-evaluated after this build. Rebuild before debugging.",
                json!({ "reason": "evaluation_changed", "buildId": build.build_id, "action": action }),
            ));
        }
        let Some(build_project) = build.project.as_ref() else {
            return Err(HostError::with_details(
                "build_stale",
                "Debug was not started: the saved build has no startup project identity. Rebuild before debugging.",
                json!({ "reason": "build_project_missing", "action": action }),
            ));
        };
        if build_project.path != selected.path
            || build.configuration != configuration
            || build.target_framework != target_framework
        {
            return Err(HostError::with_details(
                "build_stale",
                format!(
                    "Debug was not started: the saved build is not the current {} / {}{}. Build the selected startup project before debugging.",
                    selected.name,
                    configuration,
                    target_framework
                        .clone()
                        .map(|tfm| format!(" / {tfm}"))
                        .unwrap_or_default()
                ),
                json!({ "reason": "build_context_mismatch", "buildId": build.build_id, "action": action }),
            ));
        }
        if build_project.target_path.as_deref() != selected.target_path.as_deref() {
            return Err(HostError::with_details(
                "build_stale",
                "Debug was not started: the evaluated TargetPath changed after the build. Rebuild before debugging.",
                json!({ "reason": "target_changed", "buildId": build.build_id, "action": action }),
            ));
        }
        let changed = inner.build.changed_sources(&root);
        if !changed.is_empty() {
            return Err(HostError::with_details(
                "build_stale",
                format!(
                    "Debug was not started: {} saved source{} changed after this build ({}). Rebuild before debugging.",
                    changed.len(),
                    if changed.len() == 1 { "" } else { "s" },
                    changed.join(", ")
                ),
                json!({
                    "reason": "source_changed",
                    "buildId": build.build_id,
                    "changedSources": changed,
                    "action": action,
                }),
            ));
        }
        let assembly_path = PathBuf::from(&assembly);
        if !assembly_path.is_file() {
            return Err(HostError::with_details(
                "invalid_state",
                format!(
                    "Debug was not started: the evaluated assembly {} is not on disk. Rebuild before debugging.",
                    assembly
                ),
                json!({ "reason": "assembly_missing", "action": action }),
            ));
        }
        let sdk = inner.project_sdk.clone();
        if sdk.state != "resolved" {
            return Err(HostError::new(
                if sdk.state == "sdk_missing" {
                    "sdk_missing"
                } else {
                    "invalid_state"
                },
                format!(
                    "Debug was not started: the project SDK is not resolved ({}). {}",
                    sdk.state,
                    sdk.message.clone().unwrap_or_else(|| {
                        "Resolve the project SDK before debugging.".to_string()
                    })
                ),
            ));
        }
        let Some(sdk_root) = sdk.root.clone().map(PathBuf::from) else {
            return Err(HostError::new(
                "invalid_state",
                "Debug was not started: the resolved project SDK has no root path.",
            ));
        };
        let Some(project_directory) = root.join(&selected.path).parent().map(Path::to_path_buf) else {
            return Err(HostError::new(
                "invalid_state",
                "Debug was not started: the startup project has no directory.",
            ));
        };
        let project_directory = options
            .working_directory
            .unwrap_or(project_directory);

        inner.dap_seq += 1;
        let session = inner.dap_seq;
        let launch = dap::Launch {
            session,
            project_name: selected.name.clone(),
            project_path: selected.path.clone(),
            build_id: build.build_id.clone().unwrap_or_default(),
            assembly: assembly.clone(),
            configuration: configuration.clone(),
            target_framework: target_framework.clone(),
            cwd: project_directory,
            args: options.args,
            env: options.env,
            root: root.clone(),
            sdk_root: sdk_root.clone(),
            adapter_binary: adapter.binary.to_string_lossy().to_string(),
            adapter_version: adapter.version.clone(),
            adapter_platform: adapter.platform.clone(),
            workspace_id: token.workspace_id,
            context_id: token.context_id,
            breakpoints: inner.dap.breakpoints(),
        };
        let mut command = Command::new(&adapter.binary);
        command.arg("--interpreter=vscode");
        let dap = dap::Dap::begin(launch, command, inner.processes.clone())?;
        inner.dap = dap;
        Ok(self.snapshot_value(&mut inner))
    }

    /// Safety action: stops the adapter group and the debuggee, then reports requested vs
    /// observed termination and keeps the logs. Allowed without trust.
    fn stop_debug_session(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        let dap = inner.dap.clone();
        if !dap.is_active() && dap.state() != "preparing" {
            return Err(HostError::new("invalid_state", "No debug session is active."));
        }
        let processes = inner.processes.clone();
        if let Err(error) = dap.shutdown(&processes, "Stopped by the user.", false) {
            return Err(error);
        }
        inner.sync_dap();
        Ok(self.snapshot_value(&mut inner))
    }

    fn control_debug(&self, action: &str) -> HostResult<Value> {
        let _token = self.require_trust("debug.continue")?;
        let mut inner = self.inner.lock().unwrap();
        let dap = inner.dap.clone();
        match action {
            "continue" => dap.continue_()?,
            "stepOver" => dap.step_over()?,
            "stepInto" => dap.step_into()?,
            _ => {
                return Err(HostError::new("invalid_state", "Unknown debug action."));
            }
        }
        Ok(self.snapshot_value(&mut inner))
    }

    fn set_debug_breakpoints(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("debug.setBreakpoints")?;
        let raw = required_str(&params, "path")?;
        let mut lines = Vec::new();
        if let Some(values) = params.get("lines").and_then(Value::as_array) {
            for value in values {
                let Some(line) = value.as_i64() else {
                    return Err(HostError::new(
                        "invalid_state",
                        "Breakpoint lines must be positive integers.",
                    ));
                };
                if line < 1 {
                    return Err(HostError::new(
                        "invalid_state",
                        "Breakpoint lines must be positive integers.",
                    ));
                }
                lines.push(line);
            }
        }
        if lines.len() > 500 {
            return Err(HostError::new(
                "invalid_state",
                "At most 500 breakpoints are accepted per source.",
            ));
        }
        let mut inner = self.inner.lock().unwrap();
        let (_, relative) = resolve_in_workspace(&inner, &raw)?;
        inner.dap.set_breakpoints(&relative, &lines);
        Ok(self.snapshot_value(&mut inner))
    }

    fn debug_variables(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("debug.variables")?;
        let frame_id = params
            .get("frameId")
            .and_then(Value::as_i64)
            .ok_or_else(|| HostError::new("invalid_state", "debug.variables needs frameId."))?;
        let mut inner = self.inner.lock().unwrap();
        let dap = inner.dap.clone();
        dap.load_locals(frame_id)?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn flag_debugger(&self, params: Value) -> HostResult<Value> {
        let enabled = params
            .get("enabled")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                HostError::new(
                    "invalid_state",
                    "debug.flag needs enabled:true or enabled:false.",
                )
            })?;
        if enabled && params.get("confirm") != Some(&Value::Bool(true)) {
            return Err(HostError::new(
                "confirmation_required",
                "Enabling the Apple Silicon experimental flag needs an explicit confirmation. It does not install the adapter and does not prove compatibility.",
            ));
        }
        let mut inner = self.inner.lock().unwrap();
        let had_plan = inner.debugger.has_plan();
        inner.debugger_experimental = enabled;
        inner.debugger.log_flag(enabled);
        if had_plan {
            // The plan is flag-dependent: the consent shown after enabling is the one that
            // authorizes the experimental install.
            let plan = debugger::plan(
                std::env::consts::OS,
                std::env::consts::ARCH,
                inner.debugger_experimental,
            );
            inner.debugger.set_plan(plan);
        }
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
        inner.diagnostics.begin_session(
            &diagnostics::Origin::lsp(),
            session,
            token.workspace_id,
            token.context_id,
        );

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
        let spawned = inner
            .processes
            .lock()
            .unwrap()
            .spawn_piped("lsp:csharp-ls", token.workspace_id, command);
        let piped = match spawned {
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

    fn lsp_completion(&self, params: Value) -> HostResult<Value> {
        let request = match self.prepare_analysis_request(
            "lsp.completion",
            "textDocument/completion",
            &params,
        )? {
            Ok(request) => request,
            Err(answer) => return Ok(answer),
        };
        let response = request.lsp.request(
            "textDocument/completion",
            json!({
                "textDocument": { "uri": request.uri },
                "position": { "line": request.base.line, "character": request.base.character },
                "context": { "triggerKind": 1 },
            }),
            lsp::REQUEST_TIMEOUT,
        );
        if let Some(answer) = self.analysis_invalidated(&request) {
            return Ok(answer);
        }
        match response {
            Ok(value) => match lsp::parse_completion(&value, lsp::COMPLETION_LIMIT) {
                Some(list) => {
                    let mut answer = request.base.answer("ok", None);
                    answer["items"] =
                        serde_json::to_value(&list.items).expect("completion items serialize");
                    answer["itemCount"] = json!(list.items.len());
                    answer["totalItems"] = json!(list.total);
                    answer["truncated"] = json!(list.truncated);
                    answer["isIncomplete"] = json!(list.is_incomplete);
                    answer["replaceRange"] = lsp::completion_replace_range(&list.items);
                    if list.truncated {
                        answer["message"] = json!(format!(
                            "csharp-ls returned {} items; only the first {} are shown.",
                            list.total,
                            list.items.len()
                        ));
                    }
                    Ok(answer)
                }
                None => Ok(request.base.answer(
                    "failed",
                    Some(
                        "csharp-ls returned a completion result this client does not understand."
                            .to_string(),
                    ),
                )),
            },
            Err(lsp::RequestError::Timeout(timeout)) => Ok(request.base.answer(
                "timeout",
                Some(format!(
                    "csharp-ls did not answer within {} seconds; nothing was applied.",
                    timeout.as_secs()
                )),
            )),
            Err(lsp::RequestError::Unavailable(message)) => {
                Ok(request.base.answer("unavailable", Some(message)))
            }
            Err(lsp::RequestError::Server(message)) => Ok(request.base.answer(
                "failed",
                Some(format!("csharp-ls rejected the completion request: {message}")),
            )),
        }
    }

    fn lsp_definition(&self, params: Value) -> HostResult<Value> {
        let request = match self.prepare_analysis_request(
            "lsp.definition",
            "textDocument/definition",
            &params,
        )? {
            Ok(request) => request,
            Err(answer) => return Ok(answer),
        };
        let response = request.lsp.request(
            "textDocument/definition",
            json!({
                "textDocument": { "uri": request.uri },
                "position": { "line": request.base.line, "character": request.base.character },
            }),
            lsp::REQUEST_TIMEOUT,
        );
        if let Some(answer) = self.analysis_invalidated(&request) {
            return Ok(answer);
        }
        match response {
            Ok(value) => match lsp::parse_definition(&value) {
                Some(view) => {
                    let root = self
                        .inner
                        .lock()
                        .unwrap()
                        .workspace
                        .as_ref()
                        .map(|workspace| workspace.root.clone());
                    let Some(root) = root else {
                        return Ok(request.base.answer(
                            "unavailable",
                            Some(
                                "The workspace closed while csharp-ls was answering; nothing was opened."
                                    .to_string(),
                            ),
                        ));
                    };
                    let mut locations = Vec::new();
                    let mut rejected = view.rejected;
                    for target in view.targets {
                        match definition_location(&root, &target) {
                            Some(location) => locations.push(location),
                            None => rejected += 1,
                        }
                    }
                    let mut answer = request.base.answer("ok", None);
                    answer["rejectedTargets"] = json!(rejected);
                    if locations.is_empty() {
                        answer["message"] = json!(if rejected > 0 {
                            format!(
                                "csharp-ls returned {rejected} location(s) outside the workspace or not on disk; nothing was opened."
                            )
                        } else {
                            "csharp-ls returned no definition location.".to_string()
                        });
                        answer["state"] = json!(if rejected > 0 { "unavailable" } else { "ok" });
                    } else {
                        if rejected > 0 {
                            answer["message"] = json!(format!(
                                "{rejected} location(s) outside the workspace or not on disk were rejected."
                            ));
                        }
                        answer["locations"] =
                            serde_json::to_value(&locations).expect("locations serialize");
                    }
                    Ok(answer)
                }
                None => Ok(request.base.answer(
                    "failed",
                    Some(
                        "csharp-ls returned a definition result this client does not understand."
                            .to_string(),
                    ),
                )),
            },
            Err(lsp::RequestError::Timeout(timeout)) => Ok(request.base.answer(
                "timeout",
                Some(format!(
                    "csharp-ls did not answer within {} seconds; no file was opened.",
                    timeout.as_secs()
                )),
            )),
            Err(lsp::RequestError::Unavailable(message)) => {
                Ok(request.base.answer("unavailable", Some(message)))
            }
            Err(lsp::RequestError::Server(message)) => Ok(request.base.answer(
                "failed",
                Some(format!("csharp-ls rejected the definition request: {message}")),
            )),
        }
    }

    fn lsp_rename(&self, params: Value) -> HostResult<Value> {
        let request =
            match self.prepare_analysis_request("lsp.rename", "textDocument/rename", &params)? {
                Ok(request) => request,
                Err(answer) => return Ok(answer),
            };
        let new_name = required_str(&params, "newName")?.trim().to_string();
        if new_name.is_empty()
            || new_name.chars().count() > 200
            || new_name
                .chars()
                .any(|character| matches!(character, '\n' | '\r' | '\0'))
        {
            return Err(HostError::new(
                "invalid_state",
                "A rename needs a non-empty name of at most 200 characters without line breaks.",
            ));
        }
        let source_text = request
            .lsp
            .open_documents()
            .into_iter()
            .find(|document| document.path == request.base.path)
            .map(|document| document.text);
        let Some(source_text) = source_text else {
            return Ok(request.base.answer(
                "unavailable",
                Some("The source document is not open in the analysis session.".to_string()),
            ));
        };
        let position = lsp::Position {
            line: request.base.line,
            character: request.base.character,
        };
        if lsp::utf16_offset(&source_text, position).is_none() {
            return Ok(request.base.answer(
                "failed",
                Some(format!(
                    "The rename position {}:{} is outside the open document; no request was sent.",
                    request.base.line, request.base.character
                )),
            ));
        }

        let mut prepared = false;
        if request.lsp.supports_prepare_rename() {
            let response = request.lsp.request(
                "textDocument/prepareRename",
                json!({
                    "textDocument": { "uri": request.uri },
                    "position": { "line": request.base.line, "character": request.base.character },
                }),
                lsp::REQUEST_TIMEOUT,
            );
            if let Some(answer) = self.analysis_invalidated(&request) {
                return Ok(answer);
            }
            match response {
                Ok(Value::Null) => {
                    return Ok(request.base.answer(
                        "unavailable",
                        Some("csharp-ls says the symbol at this position cannot be renamed; nothing was requested.".to_string()),
                    ));
                }
                Ok(_) => prepared = true,
                Err(lsp::RequestError::Timeout(timeout)) => {
                    return Ok(request.base.answer(
                        "timeout",
                        Some(format!(
                            "csharp-ls did not answer the rename preparation within {} seconds; nothing was applied.",
                            timeout.as_secs()
                        )),
                    ));
                }
                Err(lsp::RequestError::Unavailable(message)) => {
                    return Ok(request.base.answer("unavailable", Some(message)));
                }
                Err(lsp::RequestError::Server(message)) => {
                    return Ok(request.base.answer(
                        "failed",
                        Some(format!(
                            "csharp-ls rejected the rename preparation: {message}"
                        )),
                    ));
                }
            }
        }

        let response = request.lsp.request(
            "textDocument/rename",
            json!({
                "textDocument": { "uri": request.uri },
                "position": { "line": request.base.line, "character": request.base.character },
                "newName": new_name,
            }),
            lsp::REQUEST_TIMEOUT,
        );
        if let Some(answer) = self.analysis_invalidated(&request) {
            return Ok(answer);
        }
        let value = match response {
            Ok(value) => value,
            Err(lsp::RequestError::Timeout(timeout)) => {
                return Ok(request.base.answer(
                    "timeout",
                    Some(format!(
                        "csharp-ls did not answer the rename within {} seconds; nothing was applied.",
                        timeout.as_secs()
                    )),
                ));
            }
            Err(lsp::RequestError::Unavailable(message)) => {
                return Ok(request.base.answer("unavailable", Some(message)));
            }
            Err(lsp::RequestError::Server(message)) => {
                return Ok(request.base.answer(
                    "failed",
                    Some(format!("csharp-ls rejected the rename request: {message}")),
                ));
            }
        };
        let edits = match lsp::parse_workspace_edit(&value) {
            Ok(edits) => edits,
            Err(reason) => {
                request
                    .lsp
                    .log_line(format!("[client] rename rejected: {reason}"));
                return Ok(request.base.answer(
                    "failed",
                    Some(format!(
                        "csharp-ls returned a WorkspaceEdit this client does not apply ({reason}); nothing was applied."
                    )),
                ));
            }
        };
        let (root, open) = {
            let inner = self.inner.lock().unwrap();
            let Some(workspace) = inner.workspace.as_ref() else {
                return Ok(request.base.answer(
                    "stale",
                    Some(
                        "The workspace closed while csharp-ls was answering; nothing was applied."
                            .to_string(),
                    ),
                ));
            };
            (workspace.root.clone(), request.lsp.open_documents())
        };
        let open: HashMap<String, (String, i64)> = open
            .into_iter()
            .map(|document| (document.path, (document.text, document.version)))
            .collect();
        let (documents, total_edits) = match workspace_edit_preview(&root, &edits, &open) {
            Ok(preview) => preview,
            Err(reason) => {
                request
                    .lsp
                    .log_line(format!("[client] rename rejected: {reason}"));
                return Ok(request.base.answer(
                    "failed",
                    Some(format!(
                        "The rename preview was rejected ({reason}); nothing was applied."
                    )),
                ));
            }
        };

        let sequence = {
            let mut inner = self.inner.lock().unwrap();
            inner.rename_seq += 1;
            inner.rename_seq
        };
        let fingerprint = documents
            .iter()
            .map(|document| {
                format!(
                    "{}:{}:{}",
                    document["path"].as_str().unwrap_or_default(),
                    document["baseHash"].as_str().unwrap_or_default(),
                    document["newContent"]
                        .as_str()
                        .map(content_hash)
                        .unwrap_or_default()
                )
            })
            .collect::<Vec<_>>()
            .join("|");
        let fingerprint = content_hash(&format!(
            "{}:{}:{}:{}:{}:{new_name}:{prepared}:{fingerprint}",
            request.base.token.workspace_id,
            request.base.token.context_id,
            request.base.path,
            request.base.line,
            request.base.character
        ));
        let rename_id = format!(
            "rename-{}-{sequence}-{}",
            request.base.session,
            fingerprint.trim_start_matches("fnv1a64:")
        );
        request.lsp.log_line(format!(
            "[client] rename {new_name} at {}:{} · session {} · {} document(s), {total_edits} edit(s) · {rename_id}",
            request.base.line, request.base.character, request.base.session, documents.len()
        ));
        let document_count = documents.len();
        let mut answer = request.base.answer(
            "ok",
            (document_count == 0).then(|| {
                "csharp-ls returned no edits for this rename; nothing to preview.".to_string()
            }),
        );
        answer["newName"] = json!(new_name);
        answer["renameId"] = json!(rename_id);
        answer["prepared"] = json!(prepared);
        answer["documents"] = json!(documents);
        answer["documentCount"] = json!(document_count);
        answer["editCount"] = json!(total_edits);
        Ok(answer)
    }

    fn lsp_code_actions(&self, params: Value) -> HostResult<Value> {
        let request = match self.prepare_analysis_request(
            "lsp.codeActions",
            "textDocument/codeAction",
            &params,
        )? {
            Ok(request) => request,
            Err(answer) => return Ok(answer),
        };
        let diagnostics = self.code_action_diagnostics(&request);
        let diagnostics_used = diagnostics.len();
        let position = json!({
            "line": request.base.line,
            "character": request.base.character,
        });
        let response = request.lsp.request(
            "textDocument/codeAction",
            json!({
                "textDocument": { "uri": request.uri },
                "range": { "start": position, "end": position },
                "context": { "diagnostics": diagnostics },
            }),
            lsp::REQUEST_TIMEOUT,
        );
        if let Some(answer) = self.analysis_invalidated(&request) {
            return Ok(answer);
        }
        let value = match response {
            Ok(value) => value,
            Err(lsp::RequestError::Timeout(timeout)) => {
                return Ok(request.base.answer(
                    "timeout",
                    Some(format!(
                        "csharp-ls did not answer the code action request within {} seconds; nothing was applied.",
                        timeout.as_secs()
                    )),
                ));
            }
            Err(lsp::RequestError::Unavailable(message)) => {
                return Ok(request.base.answer("unavailable", Some(message)));
            }
            Err(lsp::RequestError::Server(message)) => {
                return Ok(request.base.answer(
                    "failed",
                    Some(format!(
                        "csharp-ls rejected the code action request: {message}"
                    )),
                ));
            }
        };
        let Some(parsed) = lsp::parse_code_actions(&value, lsp::CODE_ACTION_LIMIT) else {
            request
                .lsp
                .log_line("[client] code actions rejected: the result is not an array");
            return Ok(request.base.answer(
                "failed",
                Some(
                    "csharp-ls returned a code action payload this client does not understand; nothing was applied."
                        .to_string(),
                ),
            ));
        };
        let (root, open) = {
            let inner = self.inner.lock().unwrap();
            let Some(workspace) = inner.workspace.as_ref() else {
                return Ok(request.base.answer(
                    "stale",
                    Some(
                        "The workspace closed while csharp-ls was answering; nothing was applied."
                            .to_string(),
                    ),
                ));
            };
            (workspace.root.clone(), request.lsp.open_documents())
        };
        let open: HashMap<String, (String, i64)> = open
            .into_iter()
            .map(|document| (document.path, (document.text, document.version)))
            .collect();
        let sequence = {
            let mut inner = self.inner.lock().unwrap();
            inner.action_seq += 1;
            inner.action_seq
        };
        let fingerprint = content_hash(&format!(
            "{}:{}:{}:{}:{}:{:?}",
            request.base.token.workspace_id,
            request.base.token.context_id,
            request.base.path,
            request.base.line,
            request.base.character,
            request.base.version
        ));
        let action_id = format!(
            "actions-{}-{sequence}-{}",
            request.base.session,
            fingerprint.trim_start_matches("fnv1a64:")
        );
        let mut actions = Vec::with_capacity(parsed.actions.len());
        let mut unsupported = 0;
        for (index, action) in parsed.actions.iter().enumerate() {
            let mut view = code_action_document_preview(&root, action, &open);
            if !view["supported"].as_bool().unwrap_or(false) {
                unsupported += 1;
            }
            view["actionId"] = json!(format!("{action_id}-{index}"));
            view["title"] = json!(action.title);
            view["kind"] = json!(action.kind);
            view["isPreferred"] = json!(action.is_preferred);
            view["disabled"] = json!(action.disabled);
            view["command"] = json!(action.command);
            view["hasData"] = json!(action.has_data);
            actions.push(view);
        }
        request.lsp.log_line(format!(
            "[client] code actions at {}:{} · session {} · {} returned, {} supported, {} unsupported, {} diagnostic(s) · {action_id}",
            request.base.line,
            request.base.character,
            request.base.session,
            parsed.total,
            actions.len() - unsupported,
            unsupported,
            diagnostics_used
        ));
        let message = if parsed.total == 0 {
            Some("csharp-ls returned no code actions for this position.".to_string())
        } else if parsed.truncated {
            Some(format!(
                "Showing {} of {} code actions returned by csharp-ls.",
                actions.len(),
                parsed.total
            ))
        } else if actions.is_empty() {
            Some(format!(
                "csharp-ls returned {} code action(s), none of which this client could read.",
                parsed.total
            ))
        } else {
            None
        };
        let mut answer = request.base.answer("ok", message);
        answer["actionId"] = json!(action_id);
        answer["actions"] = json!(actions);
        answer["actionCount"] = json!(actions.len());
        answer["totalActions"] = json!(parsed.total);
        answer["truncated"] = json!(parsed.truncated);
        answer["rejectedActions"] = json!(parsed.rejected);
        answer["unsupportedActions"] = json!(unsupported);
        answer["diagnosticsUsed"] = json!(diagnostics_used);
        Ok(answer)
    }

    fn lsp_references(&self, params: Value) -> HostResult<Value> {
        let request = match self.prepare_analysis_request(
            "lsp.references",
            "textDocument/references",
            &params,
        )? {
            Ok(request) => request,
            Err(answer) => return Ok(answer),
        };
        let response = request.lsp.request(
            "textDocument/references",
            json!({
                "textDocument": { "uri": request.uri },
                "position": { "line": request.base.line, "character": request.base.character },
                "context": { "includeDeclaration": false },
            }),
            lsp::REQUEST_TIMEOUT,
        );
        if let Some(answer) = self.analysis_invalidated(&request) {
            return Ok(answer);
        }
        let value = match response {
            Ok(value) => value,
            Err(lsp::RequestError::Timeout(timeout)) => {
                return Ok(request.base.answer(
                    "timeout",
                    Some(format!(
                        "csharp-ls did not answer the references request within {} seconds; nothing was changed.",
                        timeout.as_secs()
                    )),
                ));
            }
            Err(lsp::RequestError::Unavailable(message)) => {
                return Ok(request.base.answer("unavailable", Some(message)));
            }
            Err(lsp::RequestError::Server(message)) => {
                return Ok(request.base.answer(
                    "failed",
                    Some(format!(
                        "csharp-ls rejected the references request: {message}"
                    )),
                ));
            }
        };
        let Some(parsed) = lsp::parse_references(&value, lsp::REFERENCE_LIMIT) else {
            request
                .lsp
                .log_line("[client] references rejected: the result is not an array");
            return Ok(request.base.answer(
                "failed",
                Some(
                    "csharp-ls returned a references payload this client does not understand."
                        .to_string(),
                ),
            ));
        };
        let root = {
            let inner = self.inner.lock().unwrap();
            let Some(workspace) = inner.workspace.as_ref() else {
                return Ok(request.base.answer(
                    "stale",
                    Some(
                        "The workspace closed while csharp-ls was answering; no references were shown."
                            .to_string(),
                    ),
                ));
            };
            workspace.root.clone()
        };
        let mut locations = Vec::new();
        let mut rejected = parsed.rejected;
        for target in &parsed.targets {
            match definition_location(&root, target) {
                Some(location) => locations.push(location),
                None => rejected += 1,
            }
        }
        let truncated = parsed.total > parsed.targets.len();
        let mut notes = Vec::new();
        if truncated {
            notes.push(format!(
                "Showing {} of {} references returned by csharp-ls.",
                locations.len(),
                parsed.total
            ));
        }
        if rejected > 0 {
            notes.push(format!(
                "{rejected} reference(s) outside the workspace or not on disk were rejected."
            ));
        }
        let state = if locations.is_empty() && rejected > 0 {
            "unavailable"
        } else {
            "ok"
        };
        let message = if locations.is_empty() && notes.is_empty() {
            Some("csharp-ls returned no references for this position.".to_string())
        } else if notes.is_empty() {
            None
        } else {
            Some(notes.join(" "))
        };
        let mut answer = request.base.answer(state, message);
        answer["references"] = json!(locations);
        answer["referenceCount"] = json!(locations.len());
        answer["totalReferences"] = json!(parsed.total);
        answer["rejectedReferences"] = json!(rejected);
        answer["truncated"] = json!(truncated);
        request.lsp.log_line(format!(
            "[client] references at {}:{} · session {} · {} shown, {} total, {} rejected",
            request.base.line,
            request.base.character,
            request.base.session,
            locations.len(),
            parsed.total,
            rejected
        ));
        Ok(answer)
    }

    fn lsp_document_symbol(&self, params: Value) -> HostResult<Value> {
        // The contract takes only a path; line/character are inert fields of the shared
        // analysis answer and never part of the document symbol request.
        let mut gated = params.clone();
        if gated.get("line").is_none() {
            gated["line"] = json!(0);
        }
        if gated.get("character").is_none() {
            gated["character"] = json!(0);
        }
        let request = match self.prepare_analysis_request(
            "lsp.documentSymbol",
            "textDocument/documentSymbol",
            &gated,
        )? {
            Ok(request) => request,
            Err(answer) => return Ok(answer),
        };
        let response = request.lsp.request(
            "textDocument/documentSymbol",
            json!({ "textDocument": { "uri": request.uri } }),
            lsp::REQUEST_TIMEOUT,
        );
        if let Some(answer) = self.analysis_invalidated(&request) {
            return Ok(answer);
        }
        let value = match response {
            Ok(value) => value,
            Err(lsp::RequestError::Timeout(timeout)) => {
                return Ok(request.base.answer(
                    "timeout",
                    Some(format!(
                        "csharp-ls did not answer the document symbol request within {} seconds.",
                        timeout.as_secs()
                    )),
                ));
            }
            Err(lsp::RequestError::Unavailable(message)) => {
                return Ok(request.base.answer("unavailable", Some(message)));
            }
            Err(lsp::RequestError::Server(message)) => {
                return Ok(request.base.answer(
                    "failed",
                    Some(format!(
                        "csharp-ls rejected the document symbol request: {message}"
                    )),
                ));
            }
        };
        let Some(parsed) = lsp::parse_document_symbols(&value, lsp::DOCUMENT_SYMBOL_LIMIT) else {
            request
                .lsp
                .log_line("[client] document symbols rejected: the result is not an array");
            return Ok(request.base.answer(
                "failed",
                Some(
                    "csharp-ls returned a document symbol payload this client does not understand."
                        .to_string(),
                ),
            ));
        };
        let count = count_document_symbols(&parsed.symbols);
        let truncated = parsed.total > count;
        let mut notes = Vec::new();
        if truncated {
            notes.push(format!(
                "Showing {count} of {} document symbols returned by csharp-ls.",
                parsed.total
            ));
        }
        if parsed.rejected > 0 {
            notes.push(format!(
                "{} malformed document symbol(s) were ignored.",
                parsed.rejected
            ));
        }
        let message = if count == 0 && parsed.rejected == 0 {
            Some("csharp-ls returned no document symbols for this file.".to_string())
        } else if notes.is_empty() {
            None
        } else {
            Some(notes.join(" "))
        };
        let mut answer = request.base.answer("ok", message);
        answer["symbols"] = json!(parsed.symbols);
        answer["symbolCount"] = json!(count);
        answer["totalSymbols"] = json!(parsed.total);
        answer["rejectedSymbols"] = json!(parsed.rejected);
        answer["truncated"] = json!(truncated);
        Ok(answer)
    }

    /// Only current LSP publications for this document are sent as context; an item from a
    /// previous session, an awaiting publication or another origin is never forwarded.
    fn code_action_diagnostics(&self, request: &AnalysisRequest) -> Vec<Value> {
        let documents = request.lsp.document_revisions();
        let inner = self.inner.lock().unwrap();
        let view = inner.diagnostics.view(
            &documents,
            request.base.coverage,
            request.base.coverage_note.clone(),
        );
        let Some(snapshot) = view
            .snapshots
            .iter()
            .find(|snapshot| snapshot.uri == request.uri && snapshot.state == "current")
        else {
            return Vec::new();
        };
        snapshot
            .items
            .iter()
            .filter(|item| {
                range_covers_position(
                    &item.range,
                    request.base.line,
                    request.base.character,
                )
            })
            .map(|item| {
                let mut diagnostic = json!({
                    "range": item.range,
                    "message": item.message,
                });
                if let Some(severity) = item.severity {
                    diagnostic["severity"] = json!(severity);
                }
                if !item.code.is_null() {
                    diagnostic["code"] = item.code.clone();
                }
                if let Some(source) = &item.source {
                    diagnostic["source"] = json!(source);
                }
                if let Some(href) = &item.code_description {
                    diagnostic["codeDescription"] = json!({ "href": href });
                }
                diagnostic
            })
            .collect()
    }

    fn prepare_analysis_request(
        &self,
        method: &'static str,
        lsp_method: &str,
        params: &Value,
    ) -> HostResult<Result<AnalysisRequest, Value>> {
        let token = self.require_trust(method)?;
        let raw = required_str(params, "path")?;
        let line = required_u64(params, "line")?;
        let character = required_u64(params, "character")?;
        let revision = params.get("revision").and_then(Value::as_i64);
        let inner = self.inner.lock().unwrap();
        let (file, rel) = resolve_in_workspace(&inner, &raw)?;
        let mut base = AnalysisBase {
            method,
            token,
            session: 0,
            path: rel,
            line,
            character,
            revision,
            version: None,
            coverage: "partial",
            coverage_note: Some(
                "Analysis has not started; completion, definition, rename and code actions need a live csharp-ls session."
                    .to_string(),
            ),
        };
        let Some(lsp) = inner.lsp.clone() else {
            return Ok(Err(base.answer(
                "unavailable",
                Some(
                    "Analysis is not running. Start analysis to request editor features."
                        .to_string(),
                ),
            )));
        };
        let view = lsp.view();
        base.session = view.session;
        base.coverage = view.coverage;
        base.coverage_note = view.coverage_note.clone();
        if !lsp.accepts_documents() {
            return Ok(Err(base.answer(
                "unavailable",
                Some(
                    "Analysis is not ready to answer requests; csharp-ls has not finished initializing."
                        .to_string(),
                ),
            )));
        }
        let Some(capabilities) = view.capabilities.as_ref() else {
            return Ok(Err(base.answer(
                "unavailable",
                Some("csharp-ls did not report capabilities for this session.".to_string()),
            )));
        };
        if !capabilities.supports(lsp_method) {
            return Ok(Err(base.answer(
                "unsupported",
                Some(format!(
                    "csharp-ls did not advertise {lsp_method} for this session; the request was not sent."
                )),
            )));
        }
        let uri = lsp::file_uri(&file);
        let Some(version) = lsp.document_version(&uri) else {
            return Ok(Err(base.answer(
                "unavailable",
                Some(
                    "This document is not open in the analysis session; no request was sent."
                        .to_string(),
                ),
            )));
        };
        base.version = Some(version);
        Ok(Ok(AnalysisRequest { base, lsp, uri }))
    }

    /// Rejects an answer whose session, workspace context or document revision changed while
    /// the request was in flight. Returning data (not an error) keeps the editor usable.
    fn analysis_invalidated(&self, request: &AnalysisRequest) -> Option<Value> {
        let same_session = {
            let inner = self.inner.lock().unwrap();
            accept_locked(&inner, request.base.token).is_ok()
                && inner
                    .lsp
                    .as_ref()
                    .is_some_and(|live| Arc::ptr_eq(live, &request.lsp))
        };
        if !same_session {
            return Some(request.base.answer(
                "stale",
                Some(
                    "The workspace or analysis session changed while csharp-ls was answering; the result was discarded."
                        .to_string(),
                ),
            ));
        }
        if !request
            .lsp
            .response_is_current(request.base.session, &request.uri, request.base.version)
        {
            if request.lsp.document_version(&request.uri) != request.base.version {
                return Some(request.base.answer(
                    "stale",
                    Some(
                        "The document changed while csharp-ls was answering; the result was discarded."
                            .to_string(),
                    ),
                ));
            }
            return Some(request.base.answer(
                "unavailable",
                Some(
                    "Analysis stopped while the result was pending; nothing was applied."
                        .to_string(),
                ),
            ));
        }
        None
    }

    fn workspace_root(&self) -> HostResult<PathBuf> {
        let inner = self.inner.lock().unwrap();
        inner
            .workspace
            .as_ref()
            .map(|workspace| workspace.root.clone())
            .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))
    }

    fn workspace_files(&self) -> HostResult<Value> {
        let root = self.workspace_root()?;
        let list = search::list_files(&root, search::FILE_LIMIT);
        Ok(json!({
            "files": list.files,
            "returned": list.files.len(),
            "total": list.total,
            "truncated": list.truncated,
        }))
    }

    fn search_text(&self, params: Value) -> HostResult<Value> {
        let query = required_str(&params, "query")?;
        if query.is_empty() {
            return Err(HostError::new(
                "invalid_state",
                "search.text requires a non-empty query.",
            ));
        }
        let case_sensitive = params
            .get("caseSensitive")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let root = self.workspace_root()?;
        let found = search::search_text(
            &root,
            &query,
            case_sensitive,
            search::TEXT_FILE_LIMIT,
            search::TEXT_MATCH_LIMIT,
        );
        Ok(json!({
            "query": query,
            "caseSensitive": case_sensitive,
            "files": found.files,
            "fileCount": found.file_count,
            "matchCount": found.match_count,
            "truncated": found.truncated,
        }))
    }

    /// Every git method passes the same trust gate before spawning anything. Unavailability
    /// (no repository, no git on PATH) is data for the surfaces; refusals and failed commands
    /// stay observable.
    fn git_status(&self) -> HostResult<Value> {
        let _token = self.require_trust("git.status")?;
        let root = self.workspace_root()?;
        Ok(serde_json::to_value(git::status(&root)).expect("git status serializes"))
    }

    fn git_diff(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("git.diff")?;
        let root = self.workspace_root()?;
        let path = required_str(&params, "path")?;
        let staged = params
            .get("staged")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let answer = git::diff(&root, &path, staged)?;
        Ok(serde_json::to_value(answer).expect("git diff serializes"))
    }

    fn git_branches(&self) -> HostResult<Value> {
        let _token = self.require_trust("git.branches")?;
        let root = self.workspace_root()?;
        Ok(serde_json::to_value(git::branches(&root)).expect("git branches serialize"))
    }

    fn git_branch_create(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("git.branchCreate")?;
        let root = self.workspace_root()?;
        let name = required_str(&params, "name")?;
        let checkout = params
            .get("checkout")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        let answer = git::branch_create(&root, &name, checkout)?;
        Ok(serde_json::to_value(answer).expect("git branchCreate serializes"))
    }

    fn git_checkout(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("git.checkout")?;
        let root = self.workspace_root()?;
        let name = required_str(&params, "name")?;
        let answer = git::checkout(&root, &name)?;
        Ok(serde_json::to_value(answer).expect("git checkout serializes"))
    }

    fn git_log(&self) -> HostResult<Value> {
        let _token = self.require_trust("git.log")?;
        let root = self.workspace_root()?;
        Ok(serde_json::to_value(git::log(&root)).expect("git log serializes"))
    }

    fn git_commit(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("git.commit")?;
        let root = self.workspace_root()?;
        let paths_param = params
            .get("paths")
            .and_then(Value::as_array)
            .ok_or_else(|| HostError::new("invalid_state", "Missing `paths` array."))?;
        let mut paths = Vec::with_capacity(paths_param.len());
        for value in paths_param {
            let path = value.as_str().ok_or_else(|| {
                HostError::new("invalid_state", "Every commit path must be a string.")
            })?;
            paths.push(path.to_string());
        }
        let message = required_str(&params, "message")?;
        let amend = params
            .get("amend")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let answer = git::commit(&root, &paths, &message, amend)?;
        Ok(serde_json::to_value(answer).expect("git commit serializes"))
    }

    fn git_pull(&self) -> HostResult<Value> {
        let _token = self.require_trust("git.pull")?;
        let root = self.workspace_root()?;
        let answer = git::pull(&root)?;
        Ok(serde_json::to_value(answer).expect("git pull serializes"))
    }

    fn git_push(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("git.push")?;
        let root = self.workspace_root()?;
        let answer = git::push(&root, &params)?;
        Ok(serde_json::to_value(answer).expect("git push serializes"))
    }

    /// Reports the `git` PATH would run; no repository is opened and nothing is changed.
    fn git_info(&self) -> HostResult<Value> {
        let _token = self.require_trust("git.info")?;
        Ok(serde_json::to_value(git::info()).expect("git info serializes"))
    }

    /// Installed and outdated packages for one project, read through the project SDK.
    fn nuget_list(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("nuget.list")?;
        let raw = required_str(&params, "project")?;
        let target_framework = optional_str(&params, "targetFramework").filter(|tfm| !tfm.is_empty());
        if let Some(tfm) = target_framework.as_deref() {
            nuget::validate_tfm(tfm)?;
        }
        let (root, project, display, sdk) = {
            let inner = self.inner.lock().unwrap();
            accept_locked(&inner, token)?;
            let workspace = inner
                .workspace
                .as_ref()
                .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
            let (project, display) = nuget::resolve_project(&workspace.root, &raw)?;
            (
                workspace.root.clone(),
                project,
                display,
                inner.project_sdk.clone(),
            )
        };
        let (muxer, sdk_root, sdk_version) = project_muxer(&sdk)?;
        let listing = nuget::list(
            &root,
            &project,
            &display,
            target_framework.as_deref(),
            &muxer,
            &sdk_root,
            &sdk_version,
        );
        Ok(serde_json::to_value(listing).expect("nuget listing serializes"))
    }

    /// A plan is data: it names the exact command and a planId, and starts nothing.
    fn nuget_plan_update(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("nuget.planUpdate")?;
        let raw = required_str(&params, "project")?;
        let package = required_str(&params, "id")?;
        let version = required_str(&params, "version")?;
        let from_version = optional_str(&params, "fromVersion");
        let target_framework = optional_str(&params, "targetFramework").filter(|tfm| !tfm.is_empty());
        let (root, project, display, sdk, seq) = {
            let mut inner = self.inner.lock().unwrap();
            accept_locked(&inner, token)?;
            let workspace = inner
                .workspace
                .as_ref()
                .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
            let root = workspace.root.clone();
            let (project, display) = nuget::resolve_project(&root, &raw)?;
            let sdk = inner.project_sdk.clone();
            inner.nuget_seq += 1;
            (
                root,
                project,
                display,
                sdk,
                inner.nuget_seq,
            )
        };
        let (muxer, sdk_root, _) = project_muxer(&sdk)?;
        let id = nuget::plan_id(seq, token.context_id, &display, &package, &version);
        let plan = nuget::make_plan(nuget::PlanRequest {
            id,
            workspace_id: token.workspace_id,
            context_id: token.context_id,
            root,
            project,
            project_display: display,
            package,
            from_version,
            version,
            target_framework,
            muxer,
            sdk_root,
        })?;
        let mut inner = self.inner.lock().unwrap();
        accept_locked(&inner, token)?;
        let view = nuget::plan_view(&plan);
        inner.nuget_plan = Some(plan);
        Ok(view)
    }

    /// Confirmed single-package update. The plan is revalidated against the current project
    /// before `dotnet add`; any refusal leaves the project untouched and no restore follows.
    fn nuget_apply_update(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("nuget.applyUpdate")?;
        if params.get("confirm").and_then(Value::as_bool) != Some(true) {
            return Err(HostError::new(
                "confirmation_required",
                "nuget.applyUpdate requires confirm:true; no command was run.",
            ));
        }
        let plan_id = required_str(&params, "planId")?;
        let plan = {
            let mut inner = self.inner.lock().unwrap();
            accept_locked(&inner, token)?;
            let plan = inner
                .nuget_plan
                .as_ref()
                .filter(|plan| plan.id == plan_id)
                .cloned()
                .ok_or_else(|| {
                    HostError::new(
                        "nuget_stale_plan",
                        "The update plan is not current; create a new plan. No command was run.",
                    )
                })?;
            if plan.workspace_id != token.workspace_id || plan.context_id != token.context_id {
                inner.nuget_plan = None;
                return Err(HostError::new(
                    "nuget_stale_plan",
                    "The workspace or context changed after the plan; no command was run.",
                ));
            }
            inner.nuget_plan = None;
            plan
        };
        let result = nuget::apply(&plan)?;
        Ok(serde_json::to_value(result).expect("nuget update result serializes"))
    }

    /// Development-only deterministic listing so the NuGet surface is debuggable where no
    /// .NET SDK exists. It never runs dotnet and is gated by OPENRIDR_ENABLE_PROBE=1.
    fn nuget_probe(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("nuget.probe")?;
        if std::env::var("OPENRIDR_ENABLE_PROBE").as_deref() != Ok("1") {
            return Err(HostError::new(
                "invalid_state",
                "nuget.probe is a development-only probe; set OPENRIDR_ENABLE_PROBE=1 to enable it.",
            ));
        }
        let inner = self.inner.lock().unwrap();
        accept_locked(&inner, token)?;
        let workspace = inner
            .workspace
            .as_ref()
            .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
        let requested = optional_str(&params, "project")
            .or_else(|| workspace.outline.projects.first().map(|project| project.path.clone()))
            .unwrap_or_else(|| workspace.name.clone());
        let (project, display) = nuget::resolve_project(&workspace.root, &requested)
            .unwrap_or_else(|_| (workspace.root.join("Probe.csproj"), requested.clone()));
        let target_framework = optional_str(&params, "targetFramework").filter(|tfm| !tfm.is_empty());
        if let Some(tfm) = target_framework.as_deref() {
            nuget::validate_tfm(tfm)?;
        }
        let cpm = params.get("cpm").and_then(Value::as_bool).unwrap_or(false);
        let listing = nuget::probe(
            &display,
            &nuget::project_name(&project),
            target_framework.as_deref(),
            cpm,
        );
        Ok(serde_json::to_value(listing).expect("nuget probe serializes"))
    }

    /// Workspace symbols come from the live analysis session only. Unavailability and trust
    /// are data (`state`), so the palette can say what is missing instead of showing zero.
    fn lsp_workspace_symbol(&self, params: Value) -> HostResult<Value> {
        let query = required_str(&params, "query")?;
        let (root, workspace_id, context_id, trusted, lsp) = {
            let inner = self.inner.lock().unwrap();
            let workspace = inner.workspace.as_ref().ok_or_else(|| {
                HostError::new("invalid_state", "No workspace is open.")
            })?;
            (
                workspace.root.clone(),
                workspace.id,
                inner.context_seq,
                inner.trusted,
                inner.lsp.clone(),
            )
        };
        let answer = |state: &'static str,
                      message: Option<String>,
                      missing: Option<&'static str>,
                      session: u64,
                      items: Vec<Value>,
                      total: usize,
                      truncated: bool| {
            json!({
                "method": "lsp.workspaceSymbol",
                "state": state,
                "message": message,
                "missingAction": missing,
                "session": session,
                "workspaceId": workspace_id,
                "contextId": context_id,
                "query": query,
                "items": items,
                "total": total,
                "truncated": truncated,
            })
        };
        if !trusted {
            return Ok(answer(
                "analysis_not_ready",
                Some("Trust the workspace to search workspace symbols.".to_string()),
                Some("Trust the workspace"),
                lsp.as_ref().map(|lsp| lsp.session()).unwrap_or(0),
                Vec::new(),
                0,
                false,
            ));
        }
        let Some(lsp) = lsp else {
            return Ok(answer(
                "analysis_not_ready",
                Some("Analysis is not running. Start analysis to search workspace symbols.".to_string()),
                Some("Start analysis"),
                0,
                Vec::new(),
                0,
                false,
            ));
        };
        if !lsp.accepts_documents() {
            return Ok(answer(
                "analysis_not_ready",
                Some("Analysis is initializing; workspace symbols are not available yet.".to_string()),
                Some("Wait for analysis to initialize"),
                lsp.session(),
                Vec::new(),
                0,
                false,
            ));
        }
        let view = lsp.view();
        let Some(capabilities) = view.capabilities.as_ref() else {
            return Ok(answer(
                "analysis_not_ready",
                Some("Analysis has not reported its capabilities yet.".to_string()),
                Some("Wait for analysis to initialize"),
                lsp.session(),
                Vec::new(),
                0,
                false,
            ));
        };
        if !capabilities.supports("workspace/symbol") {
            return Ok(answer(
                "unsupported",
                Some(
                    "The analysis session did not advertise workspace/symbol; no request was sent."
                        .to_string(),
                ),
                None,
                lsp.session(),
                Vec::new(),
                0,
                false,
            ));
        }
        let response = lsp.request(
            "workspace/symbol",
            json!({ "query": query }),
            lsp::REQUEST_TIMEOUT,
        );
        let current = {
            let inner = self.inner.lock().unwrap();
            accept_locked(
                &inner,
                ContextToken {
                    workspace_id,
                    context_id,
                },
            )
            .is_ok()
                && inner
                    .lsp
                    .as_ref()
                    .is_some_and(|live| Arc::ptr_eq(live, &lsp))
        };
        if !current {
            return Ok(answer(
                "stale",
                Some(
                    "The workspace or analysis session changed while csharp-ls was answering; the result was discarded."
                        .to_string(),
                ),
                None,
                lsp.session(),
                Vec::new(),
                0,
                false,
            ));
        }
        match response {
            Ok(value) => {
                let Some(list) = lsp::parse_workspace_symbols(&value, lsp::WORKSPACE_SYMBOL_LIMIT)
                else {
                    return Ok(answer(
                        "failed",
                        Some(
                            "csharp-ls returned a workspace symbol result this client does not understand."
                                .to_string(),
                        ),
                        None,
                        lsp.session(),
                        Vec::new(),
                        0,
                        false,
                    ));
                };
                let items: Vec<Value> = list
                    .items
                    .iter()
                    .filter_map(|item| {
                        let path = lsp::uri_to_path(&item.uri)?;
                        let canonical = fs::canonicalize(&path).ok()?;
                        let rel = canonical
                            .strip_prefix(&root)
                            .ok()?
                            .to_string_lossy()
                            .replace('\\', "/");
                        Some(json!({
                            "name": item.name,
                            "kind": item.kind,
                            "containerName": item.container_name,
                            "uri": item.uri,
                            "path": rel,
                            "range": item.range,
                        }))
                    })
                    .collect();
                let truncated = list.truncated || items.len() < list.items.len();
                let message = truncated.then(|| {
                    format!(
                        "csharp-ls returned {} symbols; only the first {} inside the workspace are shown.",
                        list.total,
                        items.len()
                    )
                });
                Ok(answer(
                    "ok",
                    message,
                    None,
                    lsp.session(),
                    items,
                    list.total,
                    truncated,
                ))
            }
            Err(lsp::RequestError::Timeout(timeout)) => Ok(answer(
                "failed",
                Some(format!(
                    "csharp-ls did not answer within {} seconds; no symbols were returned.",
                    timeout.as_secs()
                )),
                None,
                lsp.session(),
                Vec::new(),
                0,
                false,
            )),
            Err(lsp::RequestError::Unavailable(message)) => Ok(answer(
                "analysis_not_ready",
                Some(message),
                Some("Start analysis"),
                lsp.session(),
                Vec::new(),
                0,
                false,
            )),
            Err(lsp::RequestError::Server(message)) => Ok(answer(
                "failed",
                Some(format!("csharp-ls rejected the workspace symbol request: {message}")),
                None,
                lsp.session(),
                Vec::new(),
                0,
                false,
            )),
        }
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
            inner.nuget_plan = None;
            stop_lsp_inner(
                &mut inner,
                "A project input changed; analysis stopped so it cannot report the previous context.",
            );
        }
        // A source save is shown as an obsolete build snapshot by the UI (buffer edit
        // time vs. build time); the host marks stale only when the evaluated context changes.
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
        self.tools.sync(&mut self.processes.lock().unwrap());
    }

    fn sync_lsp(&mut self) {
        let Some(lsp) = self.lsp.clone() else {
            return;
        };
        let Some(process_id) = lsp.process_id() else {
            return;
        };
        let mut processes = self.processes.lock().unwrap();
        processes.refresh();
        let Some(status) = processes.status(process_id) else {
            return;
        };
        if status.state == "exited" {
            lsp.observe_exit(status.exit, status.termination.as_ref());
        } else if lsp.state() == "failed" {
            // Fallback for platforms where the reader cannot signal the child (Windows): a
            // failed session is never left running.
            let status = processes.terminate_status(process_id);
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
        let mut processes = self.processes.lock().unwrap();
        processes.refresh();
        let Some(status) = processes.status(process_id) else {
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
        self.build
            .mark_stale("Dependencies changed after the build; rebuild before using its result.");
        self.tests.mark_stale(
            "Dependencies changed after discovery; discover tests again before using the selection.",
        );
        self.test_run.mark_stale(
            "Dependencies changed after this test run; its results are not current.",
        );
    }

    /// Observes the exit of a running build. The state is the observed one: no exit code is
    /// ever treated as zero, and a cancelled build stays cancelled.
    fn sync_build(&mut self) {
        let Some(process_id) = self.build.running_process_id() else {
            return;
        };
        let mut processes = self.processes.lock().unwrap();
        processes.refresh();
        let Some(status) = processes.status(process_id) else {
            return;
        };
        if status.state != "exited" {
            return;
        }
        self.build.complete(&status);
    }

    /// Observes the debug adapter exit. The session is finished only by what the registry and
    /// the adapter events reported; a missing exit code stays unknown.
    fn sync_dap(&mut self) {
        let dap = self.dap.clone();
        let Some(process_id) = dap.process_id() else {
            return;
        };
        let status = {
            let mut processes = self.processes.lock().unwrap();
            processes.refresh();
            processes.status(process_id)
        };
        if status.as_ref().is_some_and(|status| status.state == "exited") {
            dap.observe_adapter_exit(
                status.as_ref().and_then(|status| status.exit),
                status
                    .as_ref()
                    .and_then(|status| status.termination.as_ref()),
            );
        }
        dap.refresh_terminals(&self.processes);
    }

    /// Observes the exit of a running program. `exited` is not `ok`: the exit code, when
    /// the registry got one, is exactly what is published.
    fn sync_run(&mut self) {
        let Some(process_id) = self.run.running_process_id() else {
            return;
        };
        let mut processes = self.processes.lock().unwrap();
        processes.refresh();
        let Some(status) = processes.status(process_id) else {
            return;
        };
        if status.state != "exited" {
            return;
        }
        self.run.complete(&status);
    }

    /// Observes the shell: the waiter publishes the exit into the registry, and only an
    /// exit the registry observed can finish the session. A missing code stays unknown.
    fn sync_terminal(&mut self) {
        let Some(process_id) = self.terminal.running_process_id() else {
            return;
        };
        let mut processes = self.processes.lock().unwrap();
        processes.refresh();
        let Some(status) = processes.status(process_id) else {
            return;
        };
        if status.state == "exited" {
            self.terminal.complete(&status);
        } else {
            self.terminal.observe_decision(status.awaiting_decision);
        }
    }

    /// Observes the discovery process. Only an exit the registry observed can finish the
    /// discovery; cancellation keeps the items printed so far.
    fn sync_tests(&mut self) {
        let Some(process_id) = self.tests.running_process_id() else {
            return;
        };
        let mut processes = self.processes.lock().unwrap();
        processes.refresh();
        let Some(status) = processes.status(process_id) else {
            return;
        };
        if status.state == "exited" {
            self.tests.complete(&status);
        }
    }

    /// Observes the test run process. The TRX and log are read only after the exit the registry
    /// actually observed; a missing exit stays unknown instead of becoming success.
    fn sync_test_run(&mut self) {
        let Some(process_id) = self.test_run.running_process_id() else {
            return;
        };
        let mut processes = self.processes.lock().unwrap();
        processes.refresh();
        let Some(status) = processes.status(process_id) else {
            return;
        };
        if status.state == "exited" {
            self.test_run.complete(&status);
        }
    }
}

/// A workspace boundary (revoke, close, switch) terminates a real build and keeps only what
/// was observed. The record is sealed so its late exit cannot become another context's build.
fn stop_build(inner: &mut Inner, reason: &str) {
    if let Some(process_id) = inner.build.running_process_id() {
        let status = inner.processes.lock().unwrap().terminate_status(process_id);
        inner.build.seal(reason, status.as_ref());
    } else {
        inner.build.mark_stale(reason);
    }
}

/// A workspace boundary (revoke, close, switch) terminates a real program and keeps only
/// what was observed. The record is sealed so its late exit cannot become another context's.
fn stop_run(inner: &mut Inner, reason: &str) {
    if let Some(process_id) = inner.run.running_process_id() {
        let status = inner.processes.lock().unwrap().terminate_status(process_id);
        inner.run.seal(reason, status.as_ref());
    } else {
        inner.run.seal(reason, None);
    }
}

/// A workspace boundary (revoke, close, switch) terminates the shell group. The record is
/// sealed so output or an exit arriving later cannot be attached to the new workspace; the
/// termination shown is the observed one (a failure stays visible, never a claimed cleanup).
fn stop_terminal(inner: &mut Inner, reason: &str) {
    if let Some(process_id) = inner.terminal.running_process_id() {
        inner.terminal.signal_foreground();
        let status = inner.processes.lock().unwrap().terminate_status(process_id);
        inner.terminal.seal(reason, status.as_ref());
    } else {
        inner.terminal.seal(reason, None);
    }
}

/// A workspace boundary (revoke, close, switch) terminates a real discovery and keeps only
/// what was observed. The record is sealed so its late output cannot become another context's.
fn stop_tests(inner: &mut Inner, reason: &str) {
    if let Some(process_id) = inner.tests.running_process_id() {
        let status = inner.processes.lock().unwrap().terminate_status(process_id);
        inner.tests.seal(reason, status.as_ref());
    } else {
        inner.tests.seal(reason, None);
    }
}

/// A workspace boundary terminates a real test run and keeps the partial results/TRX/log that
/// were observed. The record is sealed so a late exit cannot become another context's result.
fn stop_test_run(inner: &mut Inner, reason: &str) {
    if let Some(process_id) = inner.test_run.running_process_id() {
        let status = inner.processes.lock().unwrap().terminate_status(process_id);
        inner.test_run.seal(reason, status.as_ref());
    } else {
        inner.test_run.seal(reason, None);
    }
}

/// A workspace boundary (revoke, close, switch) terminates a real restore and keeps only
/// what was observed. The record is sealed so its late exit cannot become another context's.
fn stop_restore(inner: &mut Inner, reason: &str) {
    let Some(process_id) = inner.restore.running_process_id() else {
        return;
    };
    let status = inner.processes.lock().unwrap().terminate_status(process_id);
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
        let mut status = inner.processes.lock().unwrap().status(process_id);
        while std::time::Instant::now() < deadline
            && status.as_ref().is_some_and(|status| status.state != "exited")
        {
            std::thread::sleep(std::time::Duration::from_millis(50));
            status = inner.processes.lock().unwrap().status(process_id);
        }
        if status.as_ref().is_some_and(|status| status.state != "exited") {
            status = inner.processes.lock().unwrap().terminate_status(process_id);
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
    tools.seal(reason, None, &mut inner.processes.lock().unwrap());
    inner.debugger.seal(reason);
    stop_debug(inner, reason, true);
}

/// A workspace boundary stops the adapter group and its debuggee, then seals the record so
/// later DAP events cannot reach the new context. What the registry observed stays visible.
fn stop_debug(inner: &mut Inner, reason: &str, seal: bool) {
    let dap = inner.dap.clone();
    if dap.view().session == 0 {
        return;
    }
    if !dap.is_active() && !seal {
        return;
    }
    match dap.shutdown(&inner.processes, reason, seal) {
        Ok(()) => {}
        Err(_) => {
            if let Some(process_id) = dap.process_id() {
                let _ = inner.processes.lock().unwrap().terminate_status(process_id);
            }
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

/// Shared gate for VSTest discovery and execution: current restore, resolved project SDK,
/// current evaluation, one explicitly selected test project and an evaluated TFM. Neither
/// operation restores, builds a solution or edits a package.
struct TestScope {
    evaluation: Evaluation,
    project: evaluation::EvaluatedProject,
    target_framework: Option<String>,
    sdk_root: PathBuf,
    sdk_version: String,
    root: PathBuf,
}

fn resolve_test_scope(
    inner: &Inner,
    token: ContextToken,
    params: &Value,
    method: &str,
    label: &str,
) -> HostResult<TestScope> {
    let Some(workspace) = inner.workspace.as_ref() else {
        return Err(HostError::new("invalid_state", "No workspace is open."));
    };
    let is_solution = matches!(workspace.kind, WorkspaceKind::Solution);
    let workspace_file = workspace.path.clone();
    let root = workspace.root.clone();

    restore::require_restored(&workspace_file, &root, is_solution, method)?;

    let sdk = inner.project_sdk.clone();
    if sdk.state != "resolved" {
        return Err(HostError::new(
            if sdk.state == "sdk_missing" {
                "sdk_missing"
            } else {
                "invalid_state"
            },
            format!(
                "{label} was not started: the project SDK is not resolved ({}). {}",
                sdk.state,
                sdk.message.clone().unwrap_or_else(|| {
                    "Resolve the project SDK first.".to_string()
                })
            ),
        ));
    }
    let Some(sdk_root) = sdk.root.clone().map(PathBuf::from) else {
        return Err(HostError::new(
            "invalid_state",
            format!("{label} was not started: the resolved project SDK has no root path."),
        ));
    };
    if !sdk_root.join(dotnet_exe()).is_file() {
        return Err(HostError::new(
            "sdk_missing",
            format!(
                "{label} was not started: {} does not contain a .NET muxer.",
                sdk_root.to_string_lossy()
            ),
        ));
    }

    let evaluation = inner.evaluation.clone();
    if evaluation.stale {
        return Err(HostError::with_details(
            "invalid_state",
            format!(
                "{label} was not started: the evaluated project model is stale ({}). Re-evaluate first.",
                evaluation
                    .stale_reason
                    .clone()
                    .unwrap_or_else(|| "no current evaluation".to_string())
            ),
            json!({ "reason": "evaluation_stale", "action": { "method": "evaluation.run", "params": {} } }),
        ));
    }
    if !matches!(evaluation.state, "ok" | "partial")
        || evaluation.workspace_id != Some(token.workspace_id)
        || evaluation.context_id != Some(token.context_id)
    {
        return Err(HostError::with_details(
            "invalid_state",
            format!(
                "{label} was not started: the current evaluation is not valid for this workspace/context. Evaluate the project model again."
            ),
            json!({ "reason": "evaluation_required", "action": { "method": "evaluation.run", "params": {} } }),
        ));
    }

    let requested = optional_str(params, "project")
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            HostError::new(
                "invalid_state",
                "Select a test project explicitly; a test operation never guesses which project to use.",
            )
        })?;
    let normalized = requested
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string();
    let Some(project) = evaluation
        .projects
        .iter()
        .find(|project| project.path == normalized)
    else {
        return Err(HostError::new(
            "invalid_state",
            format!("{requested} is not a project of the current evaluation."),
        ));
    };
    if project.kind != "test" {
        return Err(HostError::new(
            "invalid_state",
            format!(
                "{} is a {} project; only an evaluated test project can be used.",
                project.name, project.kind
            ),
        ));
    }
    if project.state != "ok" {
        return Err(HostError::new(
            "invalid_state",
            format!(
                "{} was not evaluated successfully; re-evaluate the project model first.",
                project.name
            ),
        ));
    }

    let target_framework = match params.get("targetFramework") {
        Some(Value::String(value)) if !value.trim().is_empty() => Some(value.trim().to_string()),
        Some(_) => None,
        None => evaluation.context.target_framework.clone(),
    };
    if target_framework
        .as_deref()
        .is_some_and(|tfm| !valid_selection(tfm))
    {
        return Err(HostError::new(
            "invalid_state",
            "TFM must not contain `;`, `=`, quotes or line breaks; no test operation was started.",
        ));
    }
    if target_framework != evaluation.context.target_framework {
        return Err(HostError::with_details(
            "invalid_state",
            format!(
                "{label} was not started: TFM {} has not been evaluated. Re-evaluate with that TFM first.",
                target_framework.unwrap_or_else(|| "not selected".to_string())
            ),
            json!({ "reason": "tfm_not_evaluated", "action": { "method": "evaluation.run", "params": {} } }),
        ));
    }

    let project = project.clone();
    Ok(TestScope {
        evaluation,
        project,
        target_framework,
        sdk_version: sdk
            .resolved_version
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
        sdk_root,
        root,
    })
}

/// One VSTest scope: never let the runner pick a TFM for a multi-target project and never
/// claim a TFM the evaluated project does not declare. Called after the MTP gate so an MTP
/// project keeps its exact preflight message.
fn validate_test_tfm(
    project: &evaluation::EvaluatedProject,
    target_framework: &Option<String>,
    label: &str,
) -> HostResult<()> {
    if target_framework.is_none() && project.tfms.len() > 1 {
        return Err(HostError::with_details(
            "invalid_state",
            format!(
                "{label} was not started: {} targets {}; select a TFM and re-evaluate first.",
                project.name,
                project.tfms.join(", ")
            ),
            json!({ "reason": "tfm_required", "action": { "method": "evaluation.run", "params": {} } }),
        ));
    }
    if let Some(tfm) = target_framework {
        if !project.tfms.is_empty() && !project.tfms.contains(tfm) {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "{label} was not started: {} does not declare TFM {tfm} ({}).",
                    project.name,
                    project.tfms.join(", ")
                ),
            ));
        }
    }
    Ok(())
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

/// The muxer of the project's already-resolved SDK, never PATH's dotnet.
fn project_muxer(sdk: &ProjectSdk) -> HostResult<(PathBuf, PathBuf, String)> {
    if sdk.state != "resolved" {
        return Err(HostError::new(
            if sdk.state == "sdk_missing" {
                "sdk_missing"
            } else {
                "invalid_state"
            },
            format!(
                "NuGet was not run: the project SDK is not resolved ({}). {}",
                sdk.state,
                sdk.message
                    .clone()
                    .unwrap_or_else(|| "Resolve the project SDK first.".to_string())
            ),
        ));
    }
    let root = sdk
        .root
        .clone()
        .map(PathBuf::from)
        .ok_or_else(|| {
            HostError::new(
                "invalid_state",
                "NuGet was not run: the resolved project SDK has no root path.",
            )
        })?;
    let muxer = root.join(dotnet_exe());
    if !muxer.is_file() {
        return Err(HostError::new(
            "sdk_missing",
            format!(
                "NuGet was not run: {} does not contain a .NET muxer.",
                root.to_string_lossy()
            ),
        ));
    }
    Ok((
        muxer,
        root,
        sdk.resolved_version
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
    ))
}

/// The one environment every `dotnet` child gets: explicit SDK root, no multi-level lookup,
/// English output and no telemetry/nologo noise. Values are never written to the log.
fn dotnet_command(command: &[String], cwd: &Path, sdk_root: &Path) -> Command {
    let mut process = Command::new(&command[0]);
    process
        .args(&command[1..])
        .current_dir(cwd)
        .env("DOTNET_ROOT", sdk_root)
        .env("DOTNET_MULTILEVEL_LOOKUP", "0")
        .env("DOTNET_CLI_UI_LANGUAGE", "en")
        .env("DOTNET_NOLOGO", "1")
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1");
    process
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

/// A rename or code action preview is validated as a whole: every document must be readable
/// inside the workspace, every range must be valid and non-overlapping, and the caps reject
/// the complete set instead of truncating it.
fn workspace_edit_preview(
    root: &Path,
    edits: &[lsp::RenameDocumentEdits],
    open: &HashMap<String, (String, i64)>,
) -> Result<(Vec<Value>, usize), String> {
    let total_edits: usize = edits.iter().map(|document| document.edits.len()).sum();
    if edits.len() > lsp::RENAME_DOCUMENT_LIMIT || total_edits > lsp::RENAME_EDIT_LIMIT {
        return Err(format!(
            "the edit would touch {} documents and {total_edits} edits; this build previews at most {} documents and {} edits",
            edits.len(),
            lsp::RENAME_DOCUMENT_LIMIT,
            lsp::RENAME_EDIT_LIMIT
        ));
    }
    let mut documents = Vec::with_capacity(edits.len());
    let mut seen = HashSet::new();
    for document in edits {
        let preview = rename_document_preview(root, document, open)?;
        let path = preview["path"].as_str().unwrap_or_default().to_string();
        if !seen.insert(path.clone()) {
            return Err(format!("csharp-ls returned two edits for {path}"));
        }
        documents.push(preview);
    }
    let bytes: usize = documents
        .iter()
        .map(|document| {
            document["baseContent"].as_str().map(str::len).unwrap_or(0)
                + document["newContent"].as_str().map(str::len).unwrap_or(0)
        })
        .sum();
    if bytes > lsp::RENAME_CONTENT_LIMIT {
        return Err(format!(
            "the preview would hold {bytes} bytes of document text; this build previews at most {} bytes",
            lsp::RENAME_CONTENT_LIMIT
        ));
    }
    Ok((documents, total_edits))
}

fn range_covers_position(range: &Value, line: u64, character: u64) -> bool {
    let Some(start) = range.get("start") else {
        return false;
    };
    let Some(end) = range.get("end") else {
        return false;
    };
    let (Some(start_line), Some(start_character), Some(end_line), Some(end_character)) = (
        start.get("line").and_then(Value::as_u64),
        start.get("character").and_then(Value::as_u64),
        end.get("line").and_then(Value::as_u64),
        end.get("character").and_then(Value::as_u64),
    ) else {
        return false;
    };
    (start_line, start_character) <= (line, character)
        && (line, character) <= (end_line, end_character)
}

/// A server `Command` is listed with its reason, never executed; an edit is validated by the
/// same whole-set preview used by rename.
fn code_action_document_preview(
    root: &Path,
    action: &lsp::CodeActionView,
    open: &HashMap<String, (String, i64)>,
) -> Value {
    let unsupported = |reason: String| {
        json!({
            "supported": false,
            "reason": reason,
            "documents": [],
            "documentCount": 0,
            "editCount": 0,
        })
    };
    if let Some(reason) = &action.disabled {
        return unsupported(format!("the server disabled this action: {reason}"));
    }
    if action.command_present {
        return unsupported(match &action.command {
            Some(command) => format!(
                "the action requires the command `{}`, which this build never executes",
                command.command
            ),
            None => "the action requires a command this build cannot read; commands are never executed"
                .to_string(),
        });
    }
    let Some(edit) = action.edit.as_ref() else {
        return unsupported(if action.has_data {
            "csharp-ls returned a deferred action without an edit; this build does not resolve code actions"
                .to_string()
        } else {
            "csharp-ls returned no edit for this action".to_string()
        });
    };
    let edits = match lsp::parse_workspace_edit(edit) {
        Ok(edits) => edits,
        Err(reason) => return unsupported(format!("the edit is not supported ({reason})")),
    };
    match workspace_edit_preview(root, &edits, open) {
        Ok((documents, edit_count)) => {
            let document_count = documents.len();
            json!({
                "supported": true,
                "reason": Value::Null,
                "documents": documents,
                "documentCount": document_count,
                "editCount": edit_count,
            })
        }
        Err(reason) => unsupported(reason),
    }
}

fn count_document_symbols(symbols: &[lsp::DocumentSymbolView]) -> usize {
    symbols
        .iter()
        .map(|symbol| 1 + count_document_symbols(&symbol.children))
        .sum()
}

/// Builds one document of a rename/code-action preview without writing anything: the base
/// text comes from the analysis session for open documents and from disk for unopened ones,
/// and all UTF-16 positions are resolved here so the UI never does offset math.
fn rename_document_preview(
    root: &Path,
    document: &lsp::RenameDocumentEdits,
    open: &HashMap<String, (String, i64)>,
) -> Result<Value, String> {
    let raw = lsp::uri_to_path(&document.uri)
        .ok_or_else(|| format!("{} is not a file URI", document.uri))?;
    let canonical =
        fs::canonicalize(&raw).map_err(|error| format!("{raw} is not readable: {error}"))?;
    if !canonical.starts_with(root) {
        return Err(format!("{raw} is outside the workspace"));
    }
    if !canonical.is_file() {
        return Err(format!("{raw} is not a file"));
    }
    let rel = canonical
        .strip_prefix(root)
        .unwrap_or(&canonical)
        .to_string_lossy()
        .replace('\\', "/");
    let opened = open.get(&rel);
    if let Some((_, version)) = opened {
        if let Some(expected) = document.version {
            if expected != *version {
                return Err(format!(
                    "{rel} was at version {version} in analysis but the edit was computed for version {expected}; the preview is stale"
                ));
            }
        }
    }
    let (base, document_version) = match opened {
        Some((text, version)) => (text.clone(), Some(*version)),
        None => {
            let text = fs::read_to_string(&canonical)
                .map_err(|error| format!("{rel} could not be read as UTF-8 text: {error}"))?;
            (text, None)
        }
    };
    let applied = lsp::apply_rename_edits(&base, &document.edits)
        .map_err(|error| format!("{rel}: {error}"))?;
    let changes = applied
        .changes
        .iter()
        .map(|change| {
            json!({
                "line": change.line,
                "before": change.before,
                "after": change.after,
                "displayTruncated": change.display_truncated,
            })
        })
        .collect::<Vec<_>>();
    let edits = document
        .edits
        .iter()
        .map(|edit| {
            json!({
                "range": {
                    "start": { "line": edit.start.line, "character": edit.start.character },
                    "end": { "line": edit.end.line, "character": edit.end.character },
                },
                "newText": edit.new_text,
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "path": rel,
        "uri": document.uri,
        "opened": opened.is_some(),
        "documentVersion": document_version,
        "editCount": document.edits.len(),
        "edits": edits,
        "changes": changes,
        "displayTruncated": applied.changes.iter().any(|change| change.display_truncated),
        "baseContent": base,
        "newContent": applied.content,
        "baseHash": content_hash(&base),
    }))
}

fn definition_location(root: &Path, target: &lsp::DefinitionTarget) -> Option<Value> {
    let path = PathBuf::from(lsp::uri_to_path(&target.uri)?);
    let canonical = fs::canonicalize(&path).ok()?;
    if !canonical.starts_with(root) || !canonical.is_file() {
        return None;
    }
    let rel = canonical
        .strip_prefix(root)
        .ok()?
        .to_string_lossy()
        .replace('\\', "/");
    let chosen = target.selection_range.as_ref().unwrap_or(&target.range);
    let start = chosen.get("start")?;
    Some(json!({
        "path": rel,
        "uri": target.uri,
        "line": start.get("line")?.as_u64()?,
        "character": start.get("character")?.as_u64()?,
        "range": target.range,
        "selectionRange": target.selection_range,
    }))
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

/// A bounded excerpt kept in refusal details, so a failed/stale build still shows why.
fn log_tail(log: &str, lines: usize) -> String {
    let all: Vec<&str> = log.lines().collect();
    let start = all.len().saturating_sub(lines);
    all[start..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target(path: &Path) -> lsp::DefinitionTarget {
        lsp::DefinitionTarget {
            uri: lsp::file_uri(path),
            range: json!({
                "start": { "line": 2, "character": 20 },
                "end": { "line": 2, "character": 25 }
            }),
            selection_range: Some(json!({
                "start": { "line": 2, "character": 20 },
                "end": { "line": 2, "character": 25 }
            })),
        }
    }

    #[test]
    fn workspace_files_and_text_search_need_no_trust() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let project = root.join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>").unwrap();
        fs::write(root.join("Program.cs"), "class Program\n{\n    string s = \"needle\";\n}\n").unwrap();
        let host = Host::default();
        host.dispatch("workspace.open", json!({ "path": project }))
            .unwrap();

        let files = host.dispatch("workspace.files", json!({})).unwrap();
        let names: Vec<&str> = files["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert!(names.contains(&"Program.cs"), "{names:?}");
        assert!(names.contains(&"App.csproj"), "{names:?}");
        assert_eq!(files["truncated"], false);

        let found = host
            .dispatch("search.text", json!({ "query": "needle" }))
            .unwrap();
        assert_eq!(found["fileCount"], 1);
        assert_eq!(found["matchCount"], 1);
        assert_eq!(found["files"][0]["path"], "Program.cs");
        assert_eq!(found["files"][0]["matches"][0]["line"], 2);
        assert_eq!(found["files"][0]["matches"][0]["character"], 16);
        assert_eq!(found["truncated"], false);
    }

    #[test]
    fn workspace_symbol_reports_the_missing_gate_without_trust_or_analysis() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let project = root.join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>").unwrap();
        let host = Host::default();
        host.dispatch("workspace.open", json!({ "path": project }))
            .unwrap();

        let untrusted = host
            .dispatch("lsp.workspaceSymbol", json!({ "query": "Order" }))
            .unwrap();
        assert_eq!(untrusted["state"], "analysis_not_ready");
        assert_eq!(untrusted["missingAction"], "Trust the workspace");
        assert_eq!(untrusted["items"].as_array().unwrap().len(), 0);

        host.dispatch("trust.grant", json!({})).unwrap();
        let no_analysis = host
            .dispatch("lsp.workspaceSymbol", json!({ "query": "Order" }))
            .unwrap();
        assert_eq!(no_analysis["state"], "analysis_not_ready");
        assert_eq!(no_analysis["missingAction"], "Start analysis");
        assert_eq!(no_analysis["truncated"], false);
        assert_eq!(no_analysis["items"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn references_and_document_symbols_report_the_missing_gate_without_trust_or_analysis() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let project = root.join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>").unwrap();
        fs::write(root.join("Program.cs"), "class Program { static void Main() {} }\n").unwrap();
        let host = Host::default();
        host.dispatch("workspace.open", json!({ "path": project }))
            .unwrap();

        let references = json!({ "path": "Program.cs", "line": 0, "character": 16 });
        let symbols = json!({ "path": "Program.cs" });
        assert_eq!(
            host.dispatch("lsp.references", references.clone())
                .unwrap_err()
                .code,
            "trust_required"
        );
        assert_eq!(
            host.dispatch("lsp.documentSymbol", symbols.clone())
                .unwrap_err()
                .code,
            "trust_required"
        );

        host.dispatch("trust.grant", json!({})).unwrap();
        let no_analysis = host.dispatch("lsp.references", references).unwrap();
        assert_eq!(no_analysis["state"], "unavailable");
        assert!(no_analysis["message"]
            .as_str()
            .unwrap()
            .contains("Analysis is not running"));
        assert_eq!(no_analysis["references"].as_array().unwrap().len(), 0);
        assert_eq!(no_analysis["truncated"], false);

        let no_analysis = host.dispatch("lsp.documentSymbol", symbols).unwrap();
        assert_eq!(no_analysis["state"], "unavailable");
        assert!(no_analysis["message"]
            .as_str()
            .unwrap()
            .contains("Analysis is not running"));
        assert_eq!(no_analysis["symbols"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn references_answer_is_discarded_when_the_analysis_session_changes() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let project = root.join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>").unwrap();
        fs::write(root.join("Program.cs"), "class Program {}\n").unwrap();
        let host = Host::default();
        host.dispatch("workspace.open", json!({ "path": project }))
            .unwrap();

        let analysis = lsp::Lsp::begin(
            1,
            1,
            1,
            "",
            false,
            false,
            "",
            None,
            Vec::new(),
            "/tmp".to_string(),
            false,
            "full",
            None,
        );
        let (workspace_id, context_id) = {
            let mut inner = host.inner.lock().unwrap();
            inner.lsp = Some(analysis.clone());
            let workspace = inner.workspace.as_ref().unwrap();
            (workspace.id, inner.context_seq)
        };
        let request = AnalysisRequest {
            base: AnalysisBase {
                method: "lsp.references",
                token: ContextToken {
                    workspace_id,
                    context_id,
                },
                session: 1,
                path: "Program.cs".to_string(),
                line: 0,
                character: 0,
                revision: None,
                version: None,
                coverage: "full",
                coverage_note: None,
            },
            lsp: analysis,
            uri: lsp::file_uri(&root.join("Program.cs")),
        };
        host.inner.lock().unwrap().lsp = Some(lsp::Lsp::begin(
            2,
            1,
            1,
            "",
            false,
            false,
            "",
            None,
            Vec::new(),
            "/tmp".to_string(),
            false,
            "full",
            None,
        ));
        let answer = host
            .analysis_invalidated(&request)
            .expect("a replaced session must discard the answer");
        assert_eq!(answer["state"], "stale");
        assert!(answer["message"]
            .as_str()
            .unwrap()
            .contains("discarded"));
    }

    #[test]
    fn definition_locations_must_exist_inside_the_workspace() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        fs::create_dir_all(root.join("src")).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let inside = root.join("src/Order.cs");
        fs::write(&inside, "class Order {}\n").unwrap();
        let outside = temp.path().join("Outside.cs");
        fs::write(&outside, "class Outside {}\n").unwrap();

        let location = definition_location(&root, &target(&inside)).expect("inside location");
        assert_eq!(location["path"], "src/Order.cs");
        assert_eq!(location["line"], 2);
        assert_eq!(location["character"], 20);
        assert_eq!(location["uri"], lsp::file_uri(&inside));

        assert!(definition_location(&root, &target(&outside)).is_none());
        assert!(definition_location(&root, &target(&root.join("src/Missing.cs"))).is_none());
        let remote = lsp::DefinitionTarget {
            uri: "https://example.com/Order.cs".to_string(),
            ..target(&inside)
        };
        assert!(definition_location(&root, &remote).is_none());
    }
    fn rename_edit(line: u64, start: u64, end: u64, text: &str) -> lsp::RenameTextEdit {
        lsp::RenameTextEdit {
            start: lsp::Position {
                line,
                character: start,
            },
            end: lsp::Position {
                line,
                character: end,
            },
            new_text: text.to_string(),
        }
    }

    #[test]
    fn rename_preview_reads_and_rewrites_without_touching_disk() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        fs::create_dir_all(root.join("src")).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let file = root.join("src/Order.cs");
        let base = "class Order\n{\n    Order() {}\n}\n";
        fs::write(&file, base).unwrap();
        let document = lsp::RenameDocumentEdits {
            uri: lsp::file_uri(&file),
            version: None,
            edits: vec![
                rename_edit(0, 6, 11, "Purchase"),
                rename_edit(2, 4, 9, "Purchase"),
            ],
        };
        let preview = rename_document_preview(&root, &document, &HashMap::new()).expect("preview");
        assert_eq!(preview["path"], "src/Order.cs");
        assert_eq!(preview["opened"], false);
        assert_eq!(preview["documentVersion"], Value::Null);
        assert_eq!(preview["editCount"], 2);
        assert_eq!(
            preview["newContent"],
            "class Purchase\n{\n    Purchase() {}\n}\n"
        );
        assert_eq!(preview["baseHash"], content_hash(base));
        assert_eq!(preview["changes"].as_array().unwrap().len(), 2);
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            base,
            "building a preview wrote the file"
        );
    }

    #[test]
    fn rename_preview_uses_the_open_buffer_and_rejects_its_version_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let file = root.join("Program.cs");
        fs::write(&file, "new Order();\n").unwrap();
        let mut open = HashMap::new();
        open.insert("Program.cs".to_string(), ("new Order(1);\n".to_string(), 4));
        let document = lsp::RenameDocumentEdits {
            uri: lsp::file_uri(&file),
            version: Some(4),
            edits: vec![rename_edit(0, 4, 9, "Purchase")],
        };
        let preview = rename_document_preview(&root, &document, &open).expect("open preview");
        assert_eq!(preview["opened"], true);
        assert_eq!(preview["documentVersion"], 4);
        assert_eq!(preview["baseContent"], "new Order(1);\n");
        assert_eq!(preview["newContent"], "new Purchase(1);\n");
        assert_eq!(fs::read_to_string(&file).unwrap(), "new Order();\n");

        let stale = lsp::RenameDocumentEdits {
            version: Some(3),
            ..document
        };
        let error = rename_document_preview(&root, &stale, &open).expect_err("stale version");
        assert!(error.contains("version"), "{error}");
    }

    #[test]
    fn rename_preview_rejects_outside_workspace_missing_and_invalid_ranges() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        fs::create_dir_all(&root).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let inside = root.join("Order.cs");
        fs::write(&inside, "class Order {}\n").unwrap();
        let outside = temp.path().join("Outside.cs");
        fs::write(&outside, "class Order {}\n").unwrap();

        let outside_document = lsp::RenameDocumentEdits {
            uri: lsp::file_uri(&outside),
            version: None,
            edits: vec![rename_edit(0, 6, 11, "Purchase")],
        };
        let error = rename_document_preview(&root, &outside_document, &HashMap::new())
            .expect_err("outside workspace");
        assert!(error.contains("outside the workspace"), "{error}");

        let missing = lsp::RenameDocumentEdits {
            uri: lsp::file_uri(&root.join("Missing.cs")),
            version: None,
            edits: vec![rename_edit(0, 0, 1, "Purchase")],
        };
        assert!(rename_document_preview(&root, &missing, &HashMap::new())
            .expect_err("missing file")
            .contains("not readable"));

        let invalid = lsp::RenameDocumentEdits {
            uri: lsp::file_uri(&inside),
            version: None,
            edits: vec![rename_edit(9, 0, 1, "Purchase")],
        };
        assert!(rename_document_preview(&root, &invalid, &HashMap::new())
            .expect_err("invalid range")
            .contains("outside the document"));
        assert_eq!(fs::read_to_string(&inside).unwrap(), "class Order {}\n");
    }

    fn action_with_edit(uri: String, line: u64, start: u64, end: u64, text: &str) -> lsp::CodeActionView {
        let mut changes = serde_json::Map::new();
        changes.insert(
            uri,
            json!([{
                "range": {
                    "start": { "line": line, "character": start },
                    "end": { "line": line, "character": end }
                },
                "newText": text
            }]),
        );
        action_with_value(json!({ "changes": changes }))
    }

    fn action_with_value(edit: Value) -> lsp::CodeActionView {
        lsp::CodeActionView {
            title: "Test action".to_string(),
            kind: None,
            is_preferred: false,
            disabled: None,
            edit: Some(edit),
            command: None,
            command_present: false,
            has_data: false,
        }
    }

    #[test]
    fn code_action_preview_reuses_the_rename_validation_and_never_writes() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        fs::create_dir_all(root.join("src")).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let file = root.join("src/Order.cs");
        let base = "class Order {}\n";
        fs::write(&file, base).unwrap();

        let action = action_with_edit(lsp::file_uri(&file), 0, 6, 11, "Purchase");
        let preview = code_action_document_preview(&root, &action, &HashMap::new());
        assert_eq!(preview["supported"], true);
        assert_eq!(preview["documentCount"], 1);
        assert_eq!(preview["editCount"], 1);
        assert_eq!(preview["documents"][0]["path"], "src/Order.cs");
        assert_eq!(preview["documents"][0]["baseContent"], base);
        assert_eq!(preview["documents"][0]["newContent"], "class Purchase {}\n");
        assert_eq!(preview["documents"][0]["baseHash"], content_hash(base));
        assert_eq!(fs::read_to_string(&file).unwrap(), base, "a preview wrote the file");
    }

    #[test]
    fn unsupported_actions_are_listed_with_reasons_and_never_applied() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        fs::create_dir_all(&root).unwrap();
        let root = fs::canonicalize(root).unwrap();
        let file = root.join("Order.cs");
        fs::write(&file, "class Order {}\n").unwrap();
        let outside = temp.path().join("Outside.cs");
        fs::write(&outside, "class Outside {}\n").unwrap();

        let mut command = action_with_value(Value::Null);
        command.edit = None;
        command.command_present = true;
        command.command = Some(lsp::CodeActionCommand {
            title: "Apply fix".to_string(),
            command: "csharp.applyFix".to_string(),
        });
        let preview = code_action_document_preview(&root, &command, &HashMap::new());
        assert_eq!(preview["supported"], false);
        assert!(preview["reason"]
            .as_str()
            .unwrap()
            .contains("never executes"));
        assert_eq!(preview["documents"].as_array().unwrap().len(), 0);

        let mut deferred = action_with_value(Value::Null);
        deferred.edit = None;
        deferred.has_data = true;
        let preview = code_action_document_preview(&root, &deferred, &HashMap::new());
        assert_eq!(preview["supported"], false);
        assert!(preview["reason"]
            .as_str()
            .unwrap()
            .contains("deferred action"));

        let mut disabled = action_with_edit(lsp::file_uri(&file), 0, 6, 11, "Purchase");
        disabled.disabled = Some("not available here".to_string());
        let preview = code_action_document_preview(&root, &disabled, &HashMap::new());
        assert_eq!(preview["supported"], false);
        assert!(preview["reason"].as_str().unwrap().contains("disabled"));

        let outside_action = action_with_edit(lsp::file_uri(&outside), 0, 6, 11, "Purchase");
        let preview = code_action_document_preview(&root, &outside_action, &HashMap::new());
        assert_eq!(preview["supported"], false);
        assert!(preview["reason"]
            .as_str()
            .unwrap()
            .contains("outside the workspace"));

        let missing = action_with_edit(
            lsp::file_uri(&root.join("Missing.cs")),
            0,
            0,
            0,
            "class Missing {}",
        );
        let preview = code_action_document_preview(&root, &missing, &HashMap::new());
        assert_eq!(preview["supported"], false);
        assert!(preview["reason"].as_str().unwrap().contains("not readable"));

        let file_operation = action_with_value(json!({
            "documentChanges": [{ "kind": "create", "uri": lsp::file_uri(&file) }]
        }));
        let preview = code_action_document_preview(&root, &file_operation, &HashMap::new());
        assert_eq!(preview["supported"], false);
        assert!(preview["reason"].as_str().unwrap().contains("file operation"));
        assert_eq!(fs::read_to_string(&file).unwrap(), "class Order {}\n");
    }

    #[test]
    fn rename_and_action_previews_reject_caps_before_reading_documents() {
        let root = tempfile::tempdir().unwrap();
        let documents: Vec<lsp::RenameDocumentEdits> = (0..51)
            .map(|index| lsp::RenameDocumentEdits {
                uri: format!("file:///missing/{index}.cs"),
                version: None,
                edits: vec![rename_edit(0, 0, 1, "X")],
            })
            .collect();
        let error = workspace_edit_preview(root.path(), &documents, &HashMap::new())
            .expect_err("document cap");
        assert!(error.contains("at most"), "{error}");

        let many_edits = lsp::RenameDocumentEdits {
            uri: "file:///missing/many.cs".to_string(),
            version: None,
            edits: (0..2001)
                .map(|_| rename_edit(0, 0, 1, "X"))
                .collect(),
        };
        let error = workspace_edit_preview(root.path(), &[many_edits], &HashMap::new())
            .expect_err("edit cap");
        assert!(error.contains("2000"), "{error}");
    }

    #[test]
    fn diagnostic_context_requires_a_current_range_covering_the_position() {
        let range = json!({
            "start": { "line": 2, "character": 6 },
            "end": { "line": 2, "character": 11 }
        });
        assert!(!range_covers_position(&range, 2, 5));
        assert!(range_covers_position(&range, 2, 6));
        assert!(range_covers_position(&range, 2, 9));
        assert!(range_covers_position(&range, 2, 11));
        assert!(!range_covers_position(&range, 2, 12));
        assert!(!range_covers_position(&Value::Null, 2, 6));
    }
}
