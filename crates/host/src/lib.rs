mod build;
pub mod bundled;
mod dap;
mod debugger;
mod diagnostics;
mod error;
mod evaluation;
mod fsutil;
mod git;
pub mod hostroot;
mod http;
mod logbuf;
mod lsp;
mod nuget;
mod outline;
pub mod process;
mod recent;
mod restore;
mod run;
mod run_configs;
pub mod sdk;
mod search;
mod settings;
mod terminal;
#[path = "test_discovery.rs"]
mod test_discovery;
#[cfg(test)]
#[path = "test_harness.rs"]
mod test_harness;
#[path = "test_run.rs"]
mod test_run;
pub mod tools;
mod trusted;

pub use error::{HostError, HostResult};
pub use evaluation::{Evaluation, EvaluationContext};
pub use outline::{GlobalJson, Outline, OutlineProject};
pub use restore::{RestoreView, GATED_METHODS as RESTORE_GATED_METHODS};

use outline::build_outline;
use process::Registry;
use sdk::{AnalysisRuntime, GlobalJsonRequest, Inventory, ProjectSdk, SdkRoot};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
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
    "debug.stepOut",
    "debug.setBreakpoints",
    "debug.setBreakpointCondition",
    "debug.setBreakpointsMuted",
    "debug.setExceptionBreakpoints",
    "debug.runToCursor",
    "debug.variables",
    "debug.children",
    "debug.selectThread",
    "debug.evaluate",
    "debug.hover",
    "debug.addWatch",
    "debug.removeWatch",
    "debug.pause",
    "tests.discover",
    "tests.run",
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
];

pub struct Host {
    inner: Mutex<Inner>,
}

impl Drop for Host {
    fn drop(&mut self) {
        if let Ok(inner) = self.inner.get_mut() {
            stop_services(inner, "The host is shutting down.");
            let tools = inner.tools.clone();
            tools.seal(
                "The host is shutting down.",
                None,
                &mut inner.processes.lock().unwrap(),
            );
        }
    }
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
    /// A run waiting for the build it started; `None` when nothing is pending.
    pending_run: Option<PendingRun>,
    /// A debug session waiting for the build it started; `None` when nothing is pending.
    pending_debug: Option<PendingDebug>,
    terminal: Arc<terminal::Terminal>,
    terminal_seq: u64,
    tests: Arc<test_discovery::Tests>,
    tests_seq: u64,
    test_run: Arc<test_run::TestRun>,
    test_run_seq: u64,
    tools: Arc<tools::Tools>,
    debugger: Arc<debugger::Debugger>,
    debugger_experimental: bool,
    setup_error: Option<String>,
    setup_attempt: u64,
    setup_started_at: Option<u64>,
    setup_finished_at: Option<u64>,
    /// Test-only seam for host tests that need trust without installing real tools.
    setup_override: bool,
    lsp: Option<Arc<lsp::Lsp>>,
    lsp_seq: u64,
    lsp_verbose: bool,
    rename_seq: u64,
    action_seq: u64,
    nuget_seq: u64,
    nuget_plan: Option<nuget::Plan>,
    nuget_batch: Option<Vec<nuget::Plan>>,
    nuget_batch_id: Option<String>,
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

/// A launch that started an implicit build and waits for it. The original params are kept so
/// the launch runs exactly what was asked for, through the same gates.
#[derive(Clone)]
struct PendingRun {
    build_id: String,
    token: ContextToken,
    params: Value,
}

#[derive(Clone)]
struct PendingDebug {
    build_id: String,
    token: ContextToken,
    params: Value,
    /// The session shown in the `building` record; the real launch reuses it.
    session: u64,
}

/// What the snapshot must do with a launch whose implicit build has been observed.
enum PendingOutcome {
    Wait,
    Launch,
    Cancelled,
    Failed,
    Replaced,
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
    /// Stored Trust decision of the open workspace: trusted | restricted | null (never decided).
    decision: Option<&'static str>,
    granted: bool,
    workspace_id: Option<u64>,
    decided_at: Option<u64>,
    revoked_at: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SdkView {
    host: AnalysisRuntime,
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
    setup: SetupView,
    lsp: lsp::LspView,
    diagnostics: Value,
}

/// Setup runs before trust and before the workspace becomes usable: both fixed tools must be
/// installed and verified. Only `ready` unlocks the workspace; `failed` (or `blocked` by a
/// missing runtime) keeps the setup screen with the error and a retry.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SetupView {
    state: &'static str,
    error: Option<String>,
    started_at: Option<u64>,
    finished_at: Option<u64>,
    attempt: u64,
    reinstall_available: bool,
    runtime: sdk::AnalysisRuntime,
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
                pending_run: None,
                pending_debug: None,
                terminal: Arc::new(terminal::Terminal::idle()),
                terminal_seq: 0,
                tests: Arc::new(test_discovery::Tests::idle()),
                tests_seq: 0,
                test_run: Arc::new(test_run::TestRun::idle()),
                test_run_seq: 0,
                tools: Arc::new(tools::Tools::new()),
                debugger: Arc::new(debugger::Debugger::new()),
                debugger_experimental: false,
                setup_error: None,
                setup_attempt: 0,
                setup_started_at: None,
                setup_finished_at: None,
                setup_override: false,
                lsp: None,
                lsp_seq: 0,
                lsp_verbose: false,
                rename_seq: 0,
                action_seq: 0,
                nuget_seq: 0,
                nuget_plan: None,
                nuget_batch: None,
                nuget_batch_id: None,
                diagnostics: diagnostics::Store::new(),
            }),
        }
    }

    pub fn dispatch(&self, method: &str, params: Value) -> HostResult<Value> {
        #[cfg(debug_assertions)]
        if method == "processes.probe" {
            return self.probe(params);
        }
        #[cfg(debug_assertions)]
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
        if method == "build.solution" {
            return self.start_solution_build(params);
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
        if method == "tests.export" {
            return self.export_tests();
        }
        if method == "terminal.read" {
            return self.read_terminal(params);
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
        if method == "setup.start" {
            return self.start_setup();
        }
        if method == "setup.reinstall" {
            return self.reinstall_setup();
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
        if method == "debug.stepOut" {
            return self.control_debug("stepOut");
        }
        if method == "debug.setBreakpoints" {
            return self.set_debug_breakpoints(params);
        }
        if method == "debug.setBreakpointCondition" {
            return self.set_debug_breakpoint_condition(params);
        }
        if method == "debug.setBreakpointsMuted" {
            return self.set_debug_breakpoints_muted(params);
        }
        if method == "debug.setExceptionBreakpoints" {
            return self.set_debug_exception_breakpoints(params);
        }
        if method == "debug.runToCursor" {
            return self.debug_run_to_cursor(params);
        }
        if method == "debug.variables" {
            return self.debug_variables(params);
        }
        if method == "debug.children" {
            return self.debug_children(params);
        }
        if method == "debug.selectThread" {
            return self.debug_select_thread(params);
        }
        if method == "debug.pause" {
            return self.control_debug("pause");
        }
        if method == "debug.evaluate" {
            return self.debug_evaluate(params);
        }
        if method == "debug.hover" {
            return self.debug_hover(params);
        }
        if method == "debug.addWatch" {
            return self.debug_watch(params, true);
        }
        if method == "debug.removeWatch" {
            return self.debug_watch(params, false);
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
        if method == "workspace.tree" {
            return self.workspace_tree(&params);
        }
        if method == "workspace.symbolKinds" {
            return self.workspace_symbol_kinds(&params);
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
        if method == "recent.list" {
            return Ok(recent::list());
        }
        if method == "recent.remove" {
            recent::remove(&trust_key(&params, false)?);
            return Ok(recent::list());
        }
        if method == "workspace.pick" {
            return pick_workspace();
        }
        if method == "git.path" {
            return Ok(json!({ "path": git::path() }));
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
        if method == "nuget.search" {
            return self.nuget_search(params);
        }
        if method == "nuget.listSolution" {
            return self.nuget_list_solution(params);
        }
        if method == "nuget.planBatch" {
            return self.nuget_plan_batch(params);
        }
        if method == "nuget.applyBatch" {
            return self.nuget_apply_batch(params);
        }
        #[cfg(debug_assertions)]
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
            "workspace.open" => {
                let opened = self.open_workspace(params)?;
                // Cancel leaves no trace: an undecided project enters Recents only once decided.
                if let Some(path) = opened["workspace"]["path"].as_str() {
                    if trusted::get(path).is_some() {
                        recent::record(path);
                    }
                }
                Ok(opened)
            }
            "workspace.close" => self.close_workspace(),
            "trust.grant" => self.grant_trust(&params),
            "trust.revoke" => self.revoke_trust(),
            "trust.get" => self.get_trust(&params),
            "trust.set" => self.set_trust(&params),
            "sdk.redetect" => self.redetect_sdks(),
            "processes.terminate" => self.terminate_process(params),
            "processes.keep" => self.keep_process(params),
            "file.read" => self.read_file(params),
            "file.write" => self.write_file(params),
            "file.create" => self.create_file(params),
            "file.rename" => self.rename_path(params),
            "file.delete" => self.delete_path(params),
            "workspace.reveal" => self.reveal_path(params),
            "settings.read" => settings::read(),
            "settings.write" => settings::write(&params),
            "runConfigs.read" => run_configs::read(&params),
            "runConfigs.write" => run_configs::write(&params),
            "runConfigs.setStartup" => run_configs::set_startup(&params),
            "runConfigs.launchProfiles" => self.launch_profile_request(&params, false),
            "runConfigs.setLaunchProfile" => self.launch_profile_request(&params, true),
            "session.read" => run_configs::read_session(&params),
            "session.write" => run_configs::write_session(&params),
            _ => Err(HostError::new(
                "not_found",
                format!("Unknown host method: {method}"),
            )),
        }
    }

    fn launch_profile_request(&self, params: &Value, select: bool) -> HostResult<Value> {
        let inner = self.inner.lock().unwrap();
        let workspace = inner
            .workspace
            .as_ref()
            .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
        let solution = params
            .get("solution")
            .and_then(Value::as_str)
            .and_then(|path| fs::canonicalize(path).ok());
        if solution.as_deref() != Some(workspace.path.as_path()) {
            return Err(HostError::new(
                "invalid_state",
                "The solution is not the open workspace.",
            ));
        }
        run_configs::validate_evaluated_project(
            params,
            &workspace.root,
            inner
                .evaluation
                .projects
                .iter()
                .filter(|project| project.state == "ok")
                .map(|project| project.path.as_str()),
        )?;
        if select {
            run_configs::set_launch_profile(params)
        } else {
            run_configs::launch_profiles(params)
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
        if !is_setup_ready(&inner) {
            return Err(HostError::new(
                "setup_required",
                format!("{method} requires the verified tool setup; no process was started."),
            ));
        }
        let workspace = inner
            .workspace
            .as_ref()
            .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
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

    /// After `sync_build`, finishes launches whose implicit build has been observed. A build
    /// still running leaves them waiting; a different build id means the build was replaced.
    /// The launch itself reruns every gate, so a stale build cannot slip through.
    fn finish_pending_launches(&self, inner: &mut Inner) {
        if let Some(pending) = inner.pending_run.take() {
            match pending_outcome(&inner.build.view(), &pending.build_id) {
                PendingOutcome::Wait => inner.pending_run = Some(pending),
                PendingOutcome::Launch => {
                    if let Err(error) =
                        self.start_run_locked(inner, pending.token, pending.params, false)
                    {
                        inner.run.fail_pending(format!(
                            "The build finished, but the program was not started: {}",
                            error.message
                        ));
                    }
                }
                PendingOutcome::Cancelled => inner.run.cancel_pending(),
                PendingOutcome::Failed => inner.run.fail_pending(
                    "Build failed; see the Build tab for diagnostics. The program was not started.",
                ),
                PendingOutcome::Replaced => inner.run.fail_pending(
                    "The build this run started was replaced before it finished; the program was not started.",
                ),
            }
        }
        if let Some(pending) = inner.pending_debug.take() {
            match pending_outcome(&inner.build.view(), &pending.build_id) {
                PendingOutcome::Wait => inner.pending_debug = Some(pending),
                PendingOutcome::Launch => {
                    // start_debug_locked takes `dap_seq + 1`; moving the counter back makes the
                    // real launch reuse the session the building record already showed.
                    inner.dap_seq = pending.session.saturating_sub(1);
                    let result =
                        self.start_debug_locked(inner, pending.token, pending.params, false);
                    // Burned even on failure: a retry must never repeat the session number.
                    inner.dap_seq = pending.session;
                    if let Err(error) = result {
                        inner.dap.fail_pending(format!(
                            "The build finished, but the debug session was not started: {}",
                            error.message
                        ));
                    }
                }
                PendingOutcome::Cancelled => inner.dap.cancel_pending(),
                PendingOutcome::Failed => inner.dap.fail_pending(
                    "Build failed; see the Build tab for diagnostics. No debug session was started.",
                ),
                PendingOutcome::Replaced => inner.dap.fail_pending(
                    "The build this session started was replaced before it finished; no debug session was started.",
                ),
            }
        }
    }

    fn snapshot(&self, inner: &mut Inner) -> Snapshot {
        inner.ensure_inventory();
        inner.processes.lock().unwrap().refresh();
        inner.sync_restore();
        inner.sync_build();
        // External .csproj/import edits are checked before a pending launch is completed, so
        // the launch gates see `evaluation_stale` instead of running the previous build.
        let root = inner
            .workspace
            .as_ref()
            .map(|workspace| workspace.root.clone());
        if let Some(root) = root {
            inner.evaluation.check_external_changes(&root);
        }
        self.finish_pending_launches(inner);
        inner.sync_run();
        inner.sync_terminal();
        inner.sync_tests();
        inner.sync_test_run();
        inner.sync_dap();
        inner.sync_tools();
        inner.sync_lsp();
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
            host: AnalysisRuntime::for_analysis(&inventory.runtimes),
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
        let tests_view = inner.tests_view();
        let tools_view = inner.tools.view();
        let debug_view = inner.debugger.view(inner.debugger_experimental);
        let setup_active = matches!(tools_view.state, "downloading" | "installing" | "verifying");
        let runtime = sdk.host.clone();
        let setup_ready = is_setup_ready(inner)
            || (inner.workspace.is_some()
                && runtime.state == "found"
                && tools_view.state == "ready"
                && debug_view.adapter_state == "ready"
                && inner.setup_error.is_none());
        let setup_failed = inner.workspace.is_some()
            && !setup_active
            && !setup_ready
            && (matches!(tools_view.state, "failed" | "cancelled") || inner.setup_error.is_some());
        let setup_state = if inner.workspace.is_none() {
            "idle"
        } else if setup_ready {
            "ready"
        } else if setup_failed {
            "failed"
        } else if setup_active {
            "running"
        } else if runtime.state != "found" {
            "blocked"
        } else {
            "idle"
        };
        if setup_state == "ready" && inner.setup_finished_at.is_none() {
            inner.setup_finished_at = Some(now_ms());
        }
        let setup_error = if setup_state == "failed" {
            inner
                .setup_error
                .clone()
                .or_else(|| tools_view.error.clone())
                .or_else(|| {
                    (debug_view.adapter_state != "ready")
                        .then(|| debug_view.adapter_message.clone())
                })
        } else if setup_state == "blocked" {
            runtime.message.clone()
        } else {
            None
        };
        let setup = SetupView {
            state: setup_state,
            error: setup_error,
            started_at: inner.setup_started_at,
            finished_at: inner.setup_finished_at,
            attempt: inner.setup_attempt,
            reinstall_available: tools_view.state == "ready" && !setup_active,
            runtime,
        };
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
                mode: if inner.trusted {
                    "trusted"
                } else {
                    "restricted"
                },
                candidates: Vec::new(),
            }),
            outline: inner
                .workspace
                .as_ref()
                .map(|workspace| workspace.outline.clone()),
            trust: TrustView {
                decision: inner
                    .workspace
                    .as_ref()
                    .and_then(|workspace| trusted::get(&workspace.path.to_string_lossy())),
                granted: inner.trusted,
                workspace_id: inner
                    .trust_decision
                    .as_ref()
                    .map(|decision| decision.workspace_id),
                decided_at: inner
                    .trust_decision
                    .as_ref()
                    .map(|decision| decision.granted_at),
                revoked_at: inner
                    .trust_decision
                    .as_ref()
                    .and_then(|decision| decision.revoked_at),
            },
            sdk,
            debug: debug_view,
            evaluation: inner.evaluation.clone(),
            processes: inner.processes.lock().unwrap().views(),
            restore: inner.restore.view(),
            build: inner.build.view(),
            run: inner.run.view(),
            terminal: inner.terminal.view(),
            tests: tests_view,
            dap: inner.dap.view(),
            tools: json!({ tools::TOOL: tools_view }),
            setup,
            lsp: lsp_view,
            diagnostics: serde_json::to_value(diagnostics_view).expect("diagnostics serialize"),
        }
    }

    /// Test-only: host tests exercise trust/execution gates without downloading real tools.
    /// It is not reachable from `dispatch`, so the UI and HTTP bridge cannot bypass setup.
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn allow_setup_for_tests(&self) {
        self.inner.lock().unwrap().setup_override = true;
    }

    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn deny_setup_for_tests(&self) {
        self.inner.lock().unwrap().setup_override = false;
    }

    fn open_workspace(&self, params: Value) -> HostResult<Value> {
        let raw = required_str(&params, "path")?;
        if raw.trim().is_empty() {
            return Err(HostError::new(
                "path_not_allowed",
                "Workspace path is empty.",
            ));
        }
        let base = match params.get("baseDir") {
            None | Some(Value::Null) => std::env::current_dir().ok(),
            Some(value) => value
                .as_str()
                .map(PathBuf::from)
                .filter(|dir| dir.is_absolute()),
        }
        .ok_or_else(|| HostError::new("path_not_allowed", "Base directory must be absolute."))?;
        let canonical = fs::canonicalize(base.join(&raw))
            .map_err(|_| HostError::new("not_found", format!("Workspace path not found: {raw}")))?;
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
        inner.nuget_batch = None;
        inner.nuget_batch_id = None;
        workspace.id = inner.workspace_seq;
        let request = global_json_request(&workspace.outline);
        inner.project_sdk = ProjectSdk::not_inspected(request.as_ref());
        inner.workspace = Some(workspace);
        inner.trusted = false;
        inner.debugger_experimental = false;
        inner.trust_decision = None;
        inner.setup_error = None;
        inner.setup_attempt = 0;
        inner.setup_started_at = None;
        inner.setup_finished_at = None;
        inner.setup_override = false;
        inner.evaluation = evaluation;
        inner.restore = Arc::new(restore::Restore::idle());
        inner.build = Arc::new(build::Build::idle());
        inner.run = Arc::new(run::Run::idle());
        inner.terminal = Arc::new(terminal::Terminal::idle());
        inner.tests = Arc::new(test_discovery::Tests::idle());
        inner.test_run = Arc::new(test_run::TestRun::idle());
        inner.dap = Arc::new(dap::Dap::idle());
        // Only a stored `trusted` decision opens trusted; restricted or undecided stay restricted.
        let remembered = inner
            .workspace
            .as_ref()
            .filter(|workspace| {
                trusted::get(&workspace.path.to_string_lossy()) == Some(trusted::TRUSTED)
            })
            .map(|workspace| workspace.id);
        if let Some(workspace_id) = remembered {
            apply_trust(&mut inner, workspace_id);
        }
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
        inner.setup_error = None;
        inner.setup_attempt = 0;
        inner.setup_started_at = None;
        inner.setup_finished_at = None;
        inner.setup_override = false;
        inner.workspace_seq += 1;
        inner.context_seq += 1;
        inner.nuget_plan = None;
        inner.nuget_batch = None;
        inner.nuget_batch_id = None;
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

    /// `remember: true` (sent by the UI) also persists the decision for this workspace path.
    fn grant_trust(&self, params: &Value) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        self.grant_locked(&mut inner, params["remember"].as_bool() == Some(true))
    }

    fn grant_locked(&self, inner: &mut Inner, remember: bool) -> HostResult<Value> {
        let Some((workspace_id, path)) = inner
            .workspace
            .as_ref()
            .map(|workspace| (workspace.id, workspace.path.to_string_lossy().to_string()))
        else {
            return Err(HostError::new(
                "invalid_state",
                "Open a workspace before trusting it.",
            ));
        };
        if !is_setup_ready(inner) {
            return Err(HostError::new(
                "setup_required",
                "Trust requires the verified tool setup; finish or retry the project setup first.",
            ));
        }
        if remember {
            trusted::set(&path, trusted::TRUSTED)?;
        }
        recent::record(&path);
        if !inner.trusted {
            apply_trust(inner, workspace_id);
        }
        Ok(self.snapshot_value(inner))
    }

    /// The stored decision for any path (canonical when it exists), or null.
    fn get_trust(&self, params: &Value) -> HostResult<Value> {
        let path = trust_key(params, false)?;
        Ok(json!({ "path": path, "decision": trusted::get(&path) }))
    }

    /// Persists the decision for any path; for the open workspace it also applies or revokes
    /// trust through the same paths as `trust.grant` / `trust.revoke`.
    fn set_trust(&self, params: &Value) -> HostResult<Value> {
        let path = trust_key(params, true)?;
        let decision = match required_str(params, "decision")?.as_str() {
            trusted::TRUSTED => trusted::TRUSTED,
            trusted::RESTRICTED => trusted::RESTRICTED,
            _ => {
                return Err(HostError::new(
                    "invalid_params",
                    "decision must be trusted or restricted.",
                ))
            }
        };
        // Decide and apply under one lock so a concurrent workspace.open cannot be trusted by mistake.
        let mut inner = self.inner.lock().unwrap();
        if let Some(workspace) = inner.workspace.as_ref() {
            let open_path = workspace.path.to_string_lossy();
            if open_path != path {
                return Err(HostError::new(
                    "invalid_state",
                    "While a workspace is open, trust can only be changed for that workspace. Close it before deciding another path.",
                ));
            }
        }
        let is_open = inner
            .workspace
            .as_ref()
            .is_some_and(|workspace| workspace.path.to_string_lossy() == path.as_str());
        if is_open {
            return if decision == trusted::TRUSTED {
                self.grant_locked(&mut inner, true)
            } else {
                self.revoke_locked(&mut inner)
            };
        }
        // No workspace is open: storing a decision for a welcome-screen project starts no
        // process, so it is intentionally not gated by the per-workspace tool setup.
        trusted::set(&path, decision)?;
        Ok(json!({ "path": path, "decision": trusted::get(&path) }))
    }

    fn revoke_trust(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        self.revoke_locked(&mut inner)
    }

    fn revoke_locked(&self, inner: &mut Inner) -> HostResult<Value> {
        let Some((request, path)) = inner.workspace.as_ref().map(|workspace| {
            (
                global_json_request(&workspace.outline),
                workspace.path.to_string_lossy().to_string(),
            )
        }) else {
            return Err(HostError::new("invalid_state", "No workspace is open."));
        };
        trusted::set(&path, trusted::RESTRICTED)?;
        recent::record(&path);
        inner.trusted = false;
        inner.context_seq += 1;
        inner.nuget_plan = None;
        inner.nuget_batch = None;
        inner.nuget_batch_id = None;
        if let Some(decision) = inner.trust_decision.as_mut() {
            decision.revoked_at = Some(now_ms());
        }
        stop_services(
            inner,
            "Trust was revoked while analysis or an installation was running; it was stopped and its result is not current.",
        );
        stop_restore(
            inner,
            "Trust was revoked while this restore was running; it was stopped and its result is not current.",
        );
        stop_build(
            inner,
            "Trust was revoked while this build was running; it was stopped and its result is not current.",
        );
        stop_run(
            inner,
            "Trust was revoked while this program was running; it was stopped and its result is not current.",
        );
        stop_terminal(
            inner,
            "Trust was revoked while the shell was running; it was stopped and its output is not current.",
        );
        stop_tests(
            inner,
            "Trust was revoked while tests were being discovered; the discovery was stopped and its result is not current.",
        );
        stop_test_run(
            inner,
            "Trust was revoked while tests were running; the run was stopped and its results are not current.",
        );
        inner.diagnostics.mark_unavailable(
            diagnostics::LSP_ORIGIN,
            "Trust was revoked; analysis is unavailable and these diagnostics are not current. Editing stays available.",
        );
        inner.project_sdk = ProjectSdk::not_inspected(request.as_ref());
        inner.evaluation = Evaluation::idle();
        inner.processes.lock().unwrap().mark_awaiting();
        Ok(self.snapshot_value(inner))
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
        inner.nuget_batch = None;
        inner.nuget_batch_id = None;
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
            inner
                .build
                .mark_stale("SDK inventory refreshed; the previous build used another toolchain.");
            inner.tests.mark_stale(
                "SDK inventory refreshed; discover tests again with the new toolchain.",
            );
            inner
                .test_run
                .mark_stale("SDK inventory refreshed; these test results used another toolchain.");
        }
        Ok(self.snapshot_value(&mut inner))
    }

    #[cfg(debug_assertions)]
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
    #[cfg(debug_assertions)]
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
        // Fingerprints are taken before MSBuild: an external change made while it runs must
        // still look like a change after the new model is stored, not be masked by a late stat.
        let previous_sources: Vec<String> = inner
            .evaluation
            .projects
            .iter()
            .flat_map(|project| project.sources.iter().cloned())
            .collect();
        let earlier_watch =
            evaluation::watch_paths(&workspace_file, &root, &plan.projects, &previous_sources);

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
        inner
            .test_run
            .mark_stale("The project model was re-evaluated; these test results are not current.");
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
                sdk.resolved_version
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
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
        let sources: Vec<String> = projects
            .iter()
            .flat_map(|project| project.sources.iter().cloned())
            .collect();
        let watched = evaluation::merge_watched(
            earlier_watch,
            evaluation::watch_paths(&workspace_file, &root, &plan.projects, &sources),
        );

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
        let method = if retry {
            "restore.retry"
        } else {
            "restore.start"
        };
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
        let spawned = inner.processes.lock().unwrap().spawn_piped(
            &format!("restore:{scope}"),
            token.workspace_id,
            process,
        );
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

    fn start_build(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("build.start")?;
        let mut inner = self.inner.lock().unwrap();
        self.start_build_locked(&mut inner, token, params, false)?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn start_solution_build(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("build.solution")?;
        let mut inner = self.inner.lock().unwrap();
        self.start_build_locked(&mut inner, token, params, true)?;
        Ok(self.snapshot_value(&mut inner))
    }

    /// Explicit `dotnet build` of one console startup project from the current evaluation.
    /// Never restores, never saves a buffer, and never reuses a stale TargetPath. Shared by
    /// `build.start`, `build.solution` and the implicit build Run/Debug start; the lock stays
    /// with the caller.
    fn start_build_locked(
        &self,
        inner: &mut Inner,
        token: ContextToken,
        params: Value,
        solution: bool,
    ) -> HostResult<String> {
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
        let workspace_name = workspace.name.clone();
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
        if target_framework
            .as_deref()
            .is_some_and(|tfm| !valid_selection(tfm))
        {
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

        let saved_note = optional_str(&params, "savedNote")
            .map(|note| {
                let note = note.trim().to_string();
                note.chars().take(200).collect::<String>()
            })
            .filter(|note| !note.is_empty());

        let (
            project_name,
            project_path,
            project_kind,
            project_target,
            sources,
            build_file,
            process_name,
        ) = if solution {
            if !is_solution {
                return Err(HostError::new(
                    "invalid_state",
                    "Build Solution needs an open solution; build the project with Build.",
                ));
            }
            (
                workspace_name,
                workspace_file.to_string_lossy().to_string(),
                "unknown",
                None,
                build::capture_all_sources(&root, &evaluation.projects),
                workspace_file.clone(),
                "solution".to_string(),
            )
        } else {
            let requested =
                optional_str(&params, "project").filter(|value| !value.trim().is_empty());
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
            (
                selected.name.clone(),
                selected.path.clone(),
                selected.kind,
                Some(target_path),
                build::capture_sources(&root, &selected.path, &evaluation.projects),
                root.join(&selected.path),
                selected.name.clone(),
            )
        };

        inner.build_seq += 1;
        let build_id = format!("build:{}:{}", token.workspace_id, inner.build_seq);
        let mut command = vec![
            muxer.to_string_lossy().to_string(),
            "build".to_string(),
            build_file.to_string_lossy().to_string(),
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
            build_id: build_id.clone(),
            evaluation_revision: evaluation.revision,
            sources,
            project_name,
            project_path,
            project_kind,
            target_path: project_target,
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
            solution,
        }));

        // Separate arguments and cwd via the shared dotnet environment helper.
        let process = dotnet_command(&command, &root, &sdk_root);
        let spawned = inner.processes.lock().unwrap().spawn_piped(
            &format!("build:{process_name}"),
            token.workspace_id,
            process,
        );
        let piped = match spawned {
            Ok(piped) => piped,
            Err(error) => {
                inner
                    .build
                    .fail(format!("Could not start dotnet build: {error}"));
                return Ok(build_id);
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
        Ok(build_id)
    }

    /// Safety action: allowed without trust, like `processes.terminate`. It requests
    /// termination and reports what the registry observed; the raw log is kept. A Run/Debug
    /// waiting for this build is cancelled at once, so it never launches after the stop.
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
        self.cancel_pending_launches(&mut inner);
        Ok(self.snapshot_value(&mut inner))
    }

    fn cancel_pending_launches(&self, inner: &mut Inner) {
        if inner.pending_run.take().is_some() {
            inner.run.cancel_pending();
        }
        if inner.pending_debug.take().is_some() {
            inner.dap.cancel_pending();
        }
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
            root: root.clone(),
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
        if discovery
            .project
            .as_ref()
            .map(|project| project.path.as_str())
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
        let results_directory = tools::app_home().join("test-results").join(format!(
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

    /// Writes the current discovery and the last run as JSON in the app home. It reports the
    /// real path and never touches the workspace; no process is started.
    fn export_tests(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        // An exit the registry already observed must reach the file: exporting what the last
        // snapshot saw would write a finished run as still running.
        inner.sync_test_run();
        // The same composed view the snapshot serves: discovery plus the real last run.
        let tests = inner.tests_view();
        let discovery = tests.discovery;
        let run = tests.run;
        if discovery.revision == 0 && run.revision == 0 {
            return Err(HostError::new(
                "invalid_state",
                "No test discovery or run result exists to export.",
            ));
        }
        let discovery_revision = discovery.revision;
        let run_revision = run.revision;
        let identity = run
            .run_id
            .clone()
            .or_else(|| discovery.discovery_id.clone())
            .unwrap_or_else(|| format!("revision-{discovery_revision}-{run_revision}"));
        let safe: String = identity
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect();
        let directory = tools::app_home().join("exports");
        fs::create_dir_all(&directory).map_err(|error| {
            HostError::new(
                "operation_failed",
                format!("Could not create the export directory: {error}"),
            )
        })?;
        let path = directory.join(format!("tests-{safe}.json"));
        let payload = json!({
            "exportedAt": now_ms(),
            "project": discovery.project,
            "targetFramework": discovery.target_framework,
            "discovery": discovery,
            "run": run,
        });
        let bytes = serde_json::to_vec_pretty(&payload).expect("test export serializes");
        fs::write(&path, &bytes).map_err(|error| {
            HostError::new(
                "operation_failed",
                format!("Could not write the test export: {error}"),
            )
        })?;
        Ok(json!({
            "path": path.to_string_lossy(),
            "bytes": bytes.len(),
            "identity": identity,
            "discoveryRevision": discovery_revision,
            "runRevision": run_revision,
        }))
    }

    /// Launches the evaluated TargetPath. When no saved build can launch it, the same
    /// `build.start` pipeline runs first, the record shows `building`, and the snapshot finishes
    /// the launch. Restore stays explicit. `allow_build = false` is the completion path: the
    /// same failure is then a precise error that fails the pending record.
    fn start_run(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("run.start")?;
        let mut inner = self.inner.lock().unwrap();
        self.start_run_locked(&mut inner, token, params, true)?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn start_run_locked(
        &self,
        inner: &mut Inner,
        token: ContextToken,
        params: Value,
        allow_build: bool,
    ) -> HostResult<()> {
        if allow_build && inner.pending_run.is_some() {
            return Err(HostError::new(
                "invalid_state",
                "A run is already waiting for its build to finish; stop it before starting another.",
            ));
        }
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
        if params
            .get("launchProfile")
            .and_then(Value::as_str)
            .is_some()
        {
            run_configs::validate_evaluated_project(
                &params,
                &root,
                inner
                    .evaluation
                    .projects
                    .iter()
                    .filter(|project| project.state == "ok")
                    .map(|project| project.path.as_str()),
            )?;
        }
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
        // Every reason a saved build cannot launch this run: the host starts the same
        // `build.start` pipeline instead of refusing. At completion (`allow_build = false`)
        // the same reason fails the pending run with its precise message.
        let (build, built_target) = match (|| -> HostResult<(build::BuildView, String)> {
            let build = inner.build.view();
            if build.revision == 0 {
                return Err(HostError::with_details(
                    "invalid_state",
                    format!(
                        "Run was not started: there is no current build of {}; the host builds it first.",
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
            // A solution build already compiled every evaluated project, including the chosen
            // startup; only a project build must prove it built that exact project.
            let built_target = if build.solution {
                evaluated_target.clone()
            } else {
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
                built_target
            };
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
            Ok((build, built_target))
        })() {
            Ok((build, built_target)) => (build, built_target),
            Err(_) if allow_build => {
                return self.start_pending_run(
                    inner,
                    token,
                    params,
                    selected,
                    &configuration,
                    target_framework.clone(),
                    &root,
                );
            }
            Err(error) => return Err(error),
        };

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
        let mut command = match run::launch_command(Path::new(&built_target), &muxer) {
            Ok(command) => command,
            Err(_) if allow_build => {
                return self.start_pending_run(
                    inner,
                    token,
                    params,
                    selected,
                    &configuration,
                    target_framework.clone(),
                    &root,
                );
            }
            Err(error) => return Err(error),
        };
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
                return Ok(());
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
        Ok(())
    }

    /// Starts the implicit build and leaves a `building` run record; the snapshot launches it
    /// when that build succeeds. The build params mirror the evaluated selection exactly.
    // Arguments are the evaluated selection verbatim; a params struct would duplicate `EvaluatedProject`.
    #[allow(clippy::too_many_arguments)]
    fn start_pending_run(
        &self,
        inner: &mut Inner,
        token: ContextToken,
        params: Value,
        selected: &evaluation::EvaluatedProject,
        configuration: &str,
        target_framework: Option<String>,
        root: &Path,
    ) -> HostResult<()> {
        let build_params = json!({
            "project": selected.path,
            "configuration": configuration,
            "targetFramework": target_framework.clone(),
        });
        let build_id = self.start_build_locked(inner, token, build_params, false)?;
        let Some(project_directory) = root.join(&selected.path).parent().map(Path::to_path_buf)
        else {
            return Err(HostError::new(
                "invalid_state",
                "Run was not started: the startup project has no directory.",
            ));
        };
        inner.run_seq += 1;
        inner.run = Arc::new(run::Run::building(run::Start {
            revision: inner.run_seq,
            run_id: format!("run:{}:{}", token.workspace_id, inner.run_seq),
            project_name: selected.name.clone(),
            project_path: selected.path.clone(),
            target_path: selected.target_path.clone().unwrap_or_default(),
            build_id: build_id.clone(),
            configuration: configuration.to_string(),
            target_framework,
            command: Vec::new(),
            cwd: project_directory.to_string_lossy().to_string(),
            workspace_id: token.workspace_id,
            context_id: token.context_id,
        }));
        inner.pending_run = Some(PendingRun {
            build_id,
            token,
            params,
        });
        Ok(())
    }

    /// Safety action: allowed without trust. It requests group termination and reports what
    /// the registry observed, keeping the log and the exit code (or its absence). A run still
    /// waiting for its build is cancelled at once and the build it started is stopped.
    fn cancel_run(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        if inner.pending_run.take().is_some() {
            inner.run.cancel_pending();
            self.cancel_pending_build(&mut inner);
            inner.sync_run();
            return Ok(self.snapshot_value(&mut inner));
        }
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

    /// Stops the build a pending launch started. The observed exit is the registry's; nothing
    /// is invented when it has not exited yet.
    fn cancel_pending_build(&self, inner: &mut Inner) {
        let Some(process_id) = inner.build.running_process_id() else {
            return;
        };
        inner.build.request_cancel();
        let status = inner.processes.lock().unwrap().terminate_status(process_id);
        if let Some(status) = &status {
            inner.build.observe(status);
        }
        inner.sync_build();
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

    /// Read-only, like `terminal.state`: raw PTY bytes after `since`, so the UI can emulate the
    /// screen without the filtered log. Offsets belong to one `revision`; a new shell restarts at 0.
    fn read_terminal(&self, params: Value) -> HostResult<Value> {
        let since = params.get("since").and_then(Value::as_u64).unwrap_or(0);
        let inner = self.inner.lock().unwrap();
        let (revision, chunk) = inner.terminal.read(since);
        Ok(json!({ "revision": revision, "chunk": chunk }))
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

    fn start_setup(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        inner.begin_setup()?;
        Ok(self.snapshot_value(&mut inner))
    }

    /// Removes only the app-home installation and runs the same verified installation again.
    /// It never touches the workspace and is refused while analysis is using the launcher.
    fn reinstall_setup(&self) -> HostResult<Value> {
        if let Some(error) = bundled_damaged() {
            return Err(error);
        }
        let mut inner = self.inner.lock().unwrap();
        let workspace_id = inner
            .workspace
            .as_ref()
            .ok_or_else(|| HostError::new("invalid_state", "Open a workspace before setup."))?
            .id;
        if let Some(lsp) = inner.lsp.as_ref() {
            if matches!(
                lsp.state(),
                "starting" | "loading" | "ready" | "partial" | "awaiting_diagnostics"
            ) {
                return Err(HostError::new(
                    "invalid_state",
                    "Stop analysis before reinstalling csharp-ls; the running language server uses that launcher.",
                ));
            }
        }
        if matches!(
            inner.tools.view().state,
            "downloading" | "installing" | "verifying"
        ) {
            return Err(HostError::new(
                "invalid_state",
                "An installation is already running.",
            ));
        }
        if bundled::analysis().is_some() {
            return Err(HostError::new(
                "invalid_state",
                "csharp-ls and its .NET runtime are bundled with OpenRidr; there is nothing to reinstall. Reinstall the app to replace them.",
            ));
        }
        inner.ensure_inventory();
        let runtime = sdk::AnalysisRuntime::for_analysis(
            &inner.inventory.as_ref().expect("inventory").runtimes,
        );
        if runtime.state != "found" {
            return Err(HostError::with_details(
                "sdk_missing",
                runtime
                    .message
                    .unwrap_or_else(|| "The .NET 10 runtime is required.".to_string()),
                json!({ "action": { "method": "sdk.redetect", "params": {} } }),
            ));
        }
        // Keep the existing destination until the new staging copy verifies. A failed
        // download leaves the previous verified tool usable instead of locking every project.
        let _ = fs::remove_dir_all(tools::staging());
        inner.setup_error = None;
        inner.setup_finished_at = None;
        inner.setup_started_at = Some(now_ms());
        inner.setup_attempt += 1;
        let plan = tools::plan(inner.inventory.as_ref().expect("inventory"))?;
        let tools = inner.tools.clone();
        tools.begin_install(
            plan,
            workspace_id,
            inner.context_seq,
            inner.processes.clone(),
            true,
        )?;
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
        let plan = tools::plan(inner.inventory.as_ref().expect("inventory"))?;
        inner.tools.set_plan(plan);
        Ok(self.snapshot_value(&mut inner))
    }

    fn install_tool(&self, params: Value) -> HostResult<Value> {
        if let Some(error) = bundled_damaged() {
            return Err(error);
        }
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
        let current = tools::plan(inner.inventory.as_ref().expect("inventory"))?;
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
            inner.processes.clone(),
            false,
        )?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn retry_tool(&self) -> HostResult<Value> {
        if let Some(error) = bundled_damaged() {
            return Err(error);
        }
        let token = self.require_trust("tools.retry")?;
        let mut inner = self.inner.lock().unwrap();
        inner.ensure_inventory();
        let current = tools::plan(inner.inventory.as_ref().expect("inventory"))?;
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
            inner.processes.clone(),
            false,
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
        if debugger::bundled_binary().is_some() {
            return Err(debugger::bundled_refusal());
        }
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
        if debugger::bundled_binary().is_some() {
            return Err(debugger::bundled_refusal());
        }
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

    /// Launches the evaluated managed assembly under the verified adapter. Every non-build gate
    /// is evaluated here: trust, adapter/platform/flag, evaluation and Debug configuration. A
    /// saved build that cannot launch starts the same `build.start` pipeline and the session
    /// shows `building`; a failure names the reason and spawns nothing. Restore stays explicit.
    fn start_debug(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("debug.start")?;
        let mut inner = self.inner.lock().unwrap();
        self.start_debug_locked(&mut inner, token, params, true)?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn start_debug_locked(
        &self,
        inner: &mut Inner,
        token: ContextToken,
        params: Value,
        allow_build: bool,
    ) -> HostResult<()> {
        if allow_build && inner.pending_debug.is_some() {
            return Err(HostError::new(
                "invalid_state",
                "Debug is already waiting for its build to finish; stop it before starting another.",
            ));
        }
        // At completion the record is the `building` one this launch created; it is replaced.
        if allow_build && inner.dap.is_active() {
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
        if params
            .get("launchProfile")
            .and_then(Value::as_str)
            .is_some()
        {
            run_configs::validate_evaluated_project(
                &params,
                &root,
                inner
                    .evaluation
                    .projects
                    .iter()
                    .filter(|project| project.state == "ok")
                    .map(|project| project.path.as_str()),
            )?;
        }
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
            Some(Value::String(value)) if !value.trim().is_empty() => {
                Some(value.trim().to_string())
            }
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
        let selected =
            build::select_startup_for(&evaluation.projects, requested.as_deref(), "debug")?;
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
        let Some(project_directory) = root.join(&selected.path).parent().map(Path::to_path_buf)
        else {
            return Err(HostError::new(
                "invalid_state",
                "Debug was not started: the startup project has no directory.",
            ));
        };
        let project_directory = options
            .working_directory
            .clone()
            .unwrap_or(project_directory);

        let action = build_action(&selected.path, &configuration, target_framework.as_deref());
        let breakpoints = inner.dap.breakpoints();
        let breakpoints_muted = inner.dap.breakpoints_muted();
        let exception_filters = inner.dap.exception_filters();
        let make_launch = |build_id: String, session: u64| dap::Launch {
            session,
            project_name: selected.name.clone(),
            project_path: selected.path.clone(),
            build_id,
            assembly: assembly.clone(),
            configuration: configuration.clone(),
            target_framework: target_framework.clone(),
            cwd: project_directory.clone(),
            args: options.args.clone(),
            env: options.env.clone(),
            root: root.clone(),
            sdk_root: sdk_root.clone(),
            adapter_binary: adapter.binary.to_string_lossy().to_string(),
            adapter_version: adapter.version.clone(),
            adapter_platform: adapter.platform.clone(),
            workspace_id: token.workspace_id,
            context_id: token.context_id,
            breakpoints: breakpoints.clone(),
            breakpoints_muted,
            exception_filters: exception_filters.clone(),
        };

        // Every reason a saved build cannot launch this session starts the same
        // `build.start` pipeline; at completion (`allow_build = false`) the precise message
        // fails the pending record instead.
        let build = match (|| -> HostResult<build::BuildView> {
            let build = inner.build.view();
            if build.revision == 0 {
                return Err(HostError::with_details(
                    "invalid_state",
                    format!(
                        "Debug was not started: there is no current build of {}; the host builds it first.",
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
            if build.target_framework != target_framework {
                return Err(HostError::with_details(
                    "build_stale",
                    "Debug was not started: the saved build used another TFM. Rebuild before debugging.",
                    json!({ "reason": "build_context_mismatch", "buildId": build.build_id, "action": action }),
                ));
            }
            // A solution build covers every evaluated project, including the chosen startup.
            if !build.solution {
                let Some(build_project) = build.project.as_ref() else {
                    return Err(HostError::with_details(
                        "build_stale",
                        "Debug was not started: the saved build has no startup project identity. Rebuild before debugging.",
                        json!({ "reason": "build_project_missing", "action": action }),
                    ));
                };
                if build_project.path != selected.path || build.configuration != configuration {
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
            Ok(build)
        })() {
            Ok(build) => build,
            Err(_) if allow_build => {
                return self.start_pending_debug(
                    inner,
                    token,
                    params,
                    selected,
                    &configuration,
                    target_framework.clone(),
                    make_launch,
                );
            }
            Err(error) => return Err(error),
        };

        inner.dap_seq += 1;
        let launch = make_launch(build.build_id.clone().unwrap_or_default(), inner.dap_seq);
        let command = debug_adapter_command(&adapter.binary, &sdk_root)?;
        let dap = dap::Dap::begin(launch, command, inner.processes.clone())?;
        inner.dap = dap;
        Ok(())
    }

    /// Starts the implicit build and leaves a `building` session record; the snapshot starts the
    /// adapter once that build succeeds. `make_launch` builds the exact DAP launch identity.
    // Mirrors `start_pending_run` plus the launch builder; splitting it would split one transaction.
    #[allow(clippy::too_many_arguments)]
    fn start_pending_debug<F>(
        &self,
        inner: &mut Inner,
        token: ContextToken,
        params: Value,
        selected: &evaluation::EvaluatedProject,
        configuration: &str,
        target_framework: Option<String>,
        make_launch: F,
    ) -> HostResult<()>
    where
        F: FnOnce(String, u64) -> dap::Launch,
    {
        let build_params = json!({
            "project": selected.path,
            "configuration": configuration,
            "targetFramework": target_framework,
        });
        let build_id = self.start_build_locked(inner, token, build_params, false)?;
        inner.dap_seq += 1;
        let session = inner.dap_seq;
        let launch = make_launch(build_id.clone(), session);
        inner.dap = inner.dap.building(launch);
        inner.pending_debug = Some(PendingDebug {
            build_id,
            token,
            params,
            session,
        });
        Ok(())
    }

    /// Safety action: stops the adapter group and the debuggee, then reports requested vs
    /// observed termination and keeps the logs. Allowed without trust. A session still waiting
    /// for its build is cancelled at once and the build it started is stopped.
    fn stop_debug_session(&self) -> HostResult<Value> {
        let mut inner = self.inner.lock().unwrap();
        if inner.pending_debug.take().is_some() {
            inner.dap.cancel_pending();
            self.cancel_pending_build(&mut inner);
            inner.sync_dap();
            return Ok(self.snapshot_value(&mut inner));
        }
        let dap = inner.dap.clone();
        if !dap.is_active() && dap.state() != "preparing" {
            return Err(HostError::new(
                "invalid_state",
                "No debug session is active.",
            ));
        }
        let processes = inner.processes.clone();
        dap.shutdown(&processes, "Stopped by the user.", false)?;
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
            "stepOut" => dap.step_out()?,
            "pause" => dap.pause()?,
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

    fn set_debug_breakpoint_condition(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("debug.setBreakpointCondition")?;
        let raw = required_str(&params, "path")?;
        let line = params
            .get("line")
            .and_then(Value::as_i64)
            .filter(|line| *line >= 1)
            .ok_or_else(|| {
                HostError::new(
                    "invalid_state",
                    "Breakpoint lines must be positive integers.",
                )
            })?;
        let condition = params
            .get("condition")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let mut inner = self.inner.lock().unwrap();
        let (_, relative) = resolve_in_workspace(&inner, &raw)?;
        inner.dap.set_condition(&relative, line, condition)?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn set_debug_breakpoints_muted(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("debug.setBreakpointsMuted")?;
        let muted = params
            .get("muted")
            .and_then(Value::as_bool)
            .ok_or_else(|| HostError::new("invalid_state", "muted must be a boolean."))?;
        let mut inner = self.inner.lock().unwrap();
        inner.dap.set_muted(muted);
        Ok(self.snapshot_value(&mut inner))
    }

    fn set_debug_exception_breakpoints(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("debug.setExceptionBreakpoints")?;
        let thrown = params
            .get("thrown")
            .and_then(Value::as_bool)
            .ok_or_else(|| HostError::new("invalid_state", "thrown must be a boolean."))?;
        let mut inner = self.inner.lock().unwrap();
        inner.dap.set_thrown_exceptions(thrown)?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn debug_run_to_cursor(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("debug.runToCursor")?;
        let raw = required_str(&params, "path")?;
        let line = params
            .get("line")
            .and_then(Value::as_i64)
            .filter(|line| *line >= 1)
            .ok_or_else(|| {
                HostError::new(
                    "invalid_state",
                    "Breakpoint lines must be positive integers.",
                )
            })?;
        let mut inner = self.inner.lock().unwrap();
        let (_, relative) = resolve_in_workspace(&inner, &raw)?;
        inner.dap.run_to(&relative, line)?;
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

    fn debug_evaluate(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("debug.evaluate")?;
        let expression = required_str(&params, "expression")?;
        let mut inner = self.inner.lock().unwrap();
        let dap = inner.dap.clone();
        dap.evaluate(&expression)?;
        Ok(self.snapshot_value(&mut inner))
    }

    /// Hover evaluation: returns the value without touching the panel's evaluation.
    fn debug_hover(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("debug.hover")?;
        let expression = required_str(&params, "expression")?;
        let inner = self.inner.lock().unwrap();
        let dap = inner.dap.clone();
        let evaluation = dap.hover(&expression)?;
        Ok(serde_json::to_value(evaluation).expect("evaluation serializes"))
    }

    fn debug_watch(&self, params: Value, add: bool) -> HostResult<Value> {
        let method = if add {
            "debug.addWatch"
        } else {
            "debug.removeWatch"
        };
        let _token = self.require_trust(method)?;
        let expression = required_str(&params, "expression")?;
        let mut inner = self.inner.lock().unwrap();
        let dap = inner.dap.clone();
        if add {
            dap.add_watch(&expression)?;
        } else {
            dap.remove_watch(&expression);
        }
        Ok(self.snapshot_value(&mut inner))
    }

    fn debug_select_thread(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("debug.selectThread")?;
        let thread_id = params
            .get("threadId")
            .and_then(Value::as_i64)
            .ok_or_else(|| HostError::new("invalid_state", "debug.selectThread needs threadId."))?;
        let mut inner = self.inner.lock().unwrap();
        let dap = inner.dap.clone();
        dap.select_thread(thread_id)?;
        Ok(self.snapshot_value(&mut inner))
    }

    fn debug_children(&self, params: Value) -> HostResult<Value> {
        let _token = self.require_trust("debug.children")?;
        let variables_reference = params
            .get("variablesReference")
            .and_then(Value::as_i64)
            .filter(|reference| *reference > 0)
            .ok_or_else(|| {
                HostError::new(
                    "invalid_state",
                    "debug.children needs a positive variablesReference.",
                )
            })?;
        let mut inner = self.inner.lock().unwrap();
        let dap = inner.dap.clone();
        dap.load_children(variables_reference)?;
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
        if let Some(error) = bundled_damaged() {
            return Err(error);
        }
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
        let runtime =
            AnalysisRuntime::for_analysis(&inner.inventory.as_ref().expect("inventory").runtimes);
        if runtime.state != "found" {
            return Err(HostError::with_details(
                "sdk_missing",
                "Analysis was not started: the .NET 10 runtime (Microsoft.NETCore.App) is missing. C# analysis needs that runtime; the project SDK is separate.",
                json!({ "action": { "method": "sdk.redetect", "params": {} } }),
            ));
        }
        let runtime_root = PathBuf::from(runtime.root.clone().unwrap_or_default());
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

        let hostroot = match hostroot::ensure(&runtime_root, &project_install) {
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

        // The bundled pair runs on the composed root's muxer (project SDK + private runtime 10).
        // Without a bundle, the launcher installed into the app home runs against the same root.
        let bundled_analysis = match bundled::availability() {
            bundled::Availability::Bundled(analysis) => Some(analysis),
            _ => None,
        };
        let (program, mut argv) = match &bundled_analysis {
            Some(analysis) => (
                hostroot::muxer(Path::new(&hostroot.path))
                    .to_string_lossy()
                    .to_string(),
                vec![analysis.entry.to_string_lossy().to_string()],
            ),
            None => {
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
                (binary.to_string_lossy().to_string(), Vec::new())
            }
        };
        if is_solution {
            argv.push("--solution".to_string());
            argv.push(workspace_path.to_string_lossy().to_string());
        }
        argv.push("--loglevel".to_string());
        argv.push(log_level.to_string());

        let mut command = Command::new(&program);
        command
            .args(&argv)
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
        // The view logs the whole command line, including the muxer that runs the bundled entry.
        let mut displayed_argv = vec![program.clone()];
        displayed_argv.extend(argv.iter().cloned());
        let spawned = inner.processes.lock().unwrap().spawn_piped(
            "lsp:csharp-ls",
            token.workspace_id,
            command,
        );
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
                    displayed_argv,
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
            displayed_argv,
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
                Some(format!(
                    "csharp-ls rejected the completion request: {message}"
                )),
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
                Some(format!(
                    "csharp-ls rejected the definition request: {message}"
                )),
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
                range_covers_position(&item.range, request.base.line, request.base.character)
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
        if !request.lsp.response_is_current(
            request.base.session,
            &request.uri,
            request.base.version,
        ) {
            if request.lsp.document_version(&request.uri) != request.base.version {
                return Some(request.base.answer(
                    "stale",
                    Some(
                        "The document changed while csharp-ls was answering; the result was discarded."
                            .to_string(),
                    ),
                ));
            }
            return Some(
                request.base.answer(
                    "unavailable",
                    Some(
                        "Analysis stopped while the result was pending; nothing was applied."
                            .to_string(),
                    ),
                ),
            );
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

    /// Read-only listing of one workspace directory; like `workspace.files` it needs no trust.
    fn workspace_tree(&self, params: &Value) -> HostResult<Value> {
        let root = self.workspace_root()?;
        let raw = params.get("dir").and_then(Value::as_str).unwrap_or("");
        let candidate = root.join(raw);
        let dir = confine(&root, &candidate, shape_of(raw)).map_err(|error| match error {
            fsutil::ConfineError::Outside => HostError::new(
                "outside_workspace",
                format!("Not a folder inside the workspace: {raw}"),
            ),
            fsutil::ConfineError::Missing => {
                HostError::new("not_found", format!("Folder not found: {raw}"))
            }
        })?;
        if !dir.is_dir() {
            return Err(HostError::new(
                "outside_workspace",
                format!("Not a folder inside the workspace: {raw}"),
            ));
        }
        let entries = search::list_dir(&root, &dir).map_err(|error| {
            HostError::new("operation_failed", format!("Could not list {raw}: {error}"))
        })?;
        Ok(json!({ "dir": raw, "entries": entries }))
    }

    /// Read-only C# type-kind scan for Solution Explorer badges. No trust, no compile, no LSP:
    /// it reads at most 256 `.cs` files of at most 256 KB inside the workspace and ignores the rest.
    /// Read-only C# type-kind scan for Solution Explorer badges. No trust, no compile, no LSP.
    /// The workspace root is cloned under the lock; file I/O happens after the lock is released.
    /// `known` carries the mtime the UI cached, so unchanged files are stat'ed but never read.
    fn workspace_symbol_kinds(&self, params: &Value) -> HostResult<Value> {
        self.workspace_symbol_kinds_with(params, |path| fs::read_to_string(path).ok())
    }

    fn workspace_symbol_kinds_with(
        &self,
        params: &Value,
        read: impl Fn(&Path) -> Option<String>,
    ) -> HostResult<Value> {
        let paths = params
            .get("paths")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                HostError::new(
                    "invalid_state",
                    "workspace.symbolKinds requires a paths array.",
                )
            })?;
        let known = params.get("known").and_then(Value::as_object);
        let root = {
            let inner = self.inner.lock().unwrap();
            inner
                .workspace
                .as_ref()
                .map(|workspace| workspace.root.clone())
                .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?
        };
        let mut entries = Vec::new();
        let mut scanned = 0;
        let mut skipped = 0;
        for raw in paths.iter().filter_map(Value::as_str).take(256) {
            if !raw.to_ascii_lowercase().ends_with(".cs") {
                continue;
            }
            let Ok((path, rel)) = resolve_in_root(&root, raw) else {
                continue;
            };
            let Ok(metadata) = fs::metadata(&path) else {
                continue;
            };
            let modified = modified_ms(&metadata);
            if known.and_then(|map| map.get(&rel)).and_then(Value::as_u64) == Some(modified) {
                skipped += 1;
                continue;
            }
            // A file too large to scan is still an answer (kind null) so the UI caches the miss.
            let kind = if metadata.len() <= 256 * 1024 {
                read(&path).and_then(|text| {
                    let stem = path
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .unwrap_or("");
                    outline::symbol_kind(&text, stem).map(|kind| Value::String(kind.to_string()))
                })
            } else {
                None
            };
            entries.push(json!({ "path": rel, "kind": kind, "modified": modified }));
            scanned += 1;
        }
        Ok(json!({
            "entries": entries,
            "scanned": scanned,
            "skipped": skipped,
            "truncated": paths.len() > 256,
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
        let target_framework =
            optional_str(&params, "targetFramework").filter(|tfm| !tfm.is_empty());
        if let Some(tfm) = target_framework.as_deref() {
            nuget::validate_tfm(tfm)?;
        }
        let prerelease = params
            .get("prerelease")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let source = optional_str(&params, "source").filter(|source| !source.trim().is_empty());
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
            prerelease,
            source.as_deref(),
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
        let target_framework =
            optional_str(&params, "targetFramework").filter(|tfm| !tfm.is_empty());
        let install = params
            .get("install")
            .and_then(Value::as_bool)
            .unwrap_or(false);
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
            (root, project, display, sdk, inner.nuget_seq)
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
            install,
            defer_cpm: false,
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
                inner.nuget_batch = None;
                inner.nuget_batch_id = None;
                return Err(HostError::new(
                    "nuget_stale_plan",
                    "The workspace or context changed after the plan; no command was run.",
                ));
            }
            inner.nuget_plan = None;
            inner.nuget_batch = None;
            inner.nuget_batch_id = None;
            plan
        };
        let result = nuget::apply(&plan)?;
        Ok(serde_json::to_value(result).expect("nuget update result serializes"))
    }

    /// Development-only deterministic listing so the NuGet surface is debuggable where no
    /// .NET SDK exists. It never runs dotnet and is gated by OPENRIDR_ENABLE_PROBE=1.
    #[cfg(debug_assertions)]
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
            .or_else(|| {
                workspace
                    .outline
                    .projects
                    .first()
                    .map(|project| project.path.clone())
            })
            .unwrap_or_else(|| workspace.name.clone());
        let (project, display) = nuget::resolve_project(&workspace.root, &requested)
            .unwrap_or_else(|_| (workspace.root.join("Probe.csproj"), requested.clone()));
        let target_framework =
            optional_str(&params, "targetFramework").filter(|tfm| !tfm.is_empty());
        if let Some(tfm) = target_framework.as_deref() {
            nuget::validate_tfm(tfm)?;
        }
        let cpm = params.get("cpm").and_then(Value::as_bool).unwrap_or(false);
        let empty = params
            .get("empty")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let sources_failed = params
            .get("sourcesFailed")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let listing = nuget::probe(
            &display,
            &nuget::project_name(&project),
            target_framework.as_deref(),
            cpm,
            empty,
            sources_failed,
            params
                .get("prerelease")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            optional_str(&params, "source").as_deref(),
        );
        Ok(serde_json::to_value(listing).expect("nuget probe serializes"))
    }

    /// Discover reads only the selected HTTP v3 source; installation still uses the project's
    /// configured sources through the same confirmed plan gates.
    fn nuget_search(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("nuget.search")?;
        let source = required_str(&params, "source")?;
        let raw_project = required_str(&params, "project")?;
        let query = optional_str(&params, "query").unwrap_or_default();
        let prerelease = params
            .get("prerelease")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let take = params.get("take").and_then(Value::as_u64).unwrap_or(30) as usize;
        let (root, sdk) = {
            let inner = self.inner.lock().unwrap();
            accept_locked(&inner, token)?;
            let workspace = inner
                .workspace
                .as_ref()
                .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
            nuget::resolve_project(&workspace.root, &raw_project)?;
            (workspace.root.clone(), inner.project_sdk.clone())
        };
        let (muxer, sdk_root, _) = project_muxer(&sdk)?;
        let configured = nuget::configured_sources(&root, &muxer, &sdk_root);
        if configured.state != "ok" {
            return Err(HostError::new(
                "source_unavailable",
                configured
                    .message
                    .unwrap_or_else(|| "The configured NuGet sources are unknown.".to_string()),
            ));
        }
        if !nuget::source_matches(&source, &configured.items) {
            return Err(HostError::new(
                "source_not_allowed",
                "Discover only accepts a source configured for this project; no request was made.",
            ));
        }
        if let Err(error) = nuget::validate_source_arg(&source) {
            return Err(HostError::new("source_not_allowed", error));
        }
        let result = nuget::search(&source, &query, prerelease, take);
        Ok(serde_json::to_value(result).expect("nuget search serializes"))
    }

    /// Solution scope aggregates one real listing per project. A failed project is reported as
    /// incomplete; the others are not hidden and no package data is invented.
    fn nuget_list_solution(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("nuget.listSolution")?;
        let raw_projects = params
            .get("projects")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                HostError::new("invalid_state", "projects must be an array of paths.")
            })?;
        if raw_projects.is_empty() {
            return Err(HostError::new(
                "invalid_state",
                "No project was provided for the solution listing.",
            ));
        }
        let target_framework =
            optional_str(&params, "targetFramework").filter(|tfm| !tfm.is_empty());
        if let Some(tfm) = target_framework.as_deref() {
            nuget::validate_tfm(tfm)?;
        }
        let prerelease = params
            .get("prerelease")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let source = optional_str(&params, "source").filter(|source| !source.trim().is_empty());
        let (root, sdk, projects) = {
            let inner = self.inner.lock().unwrap();
            accept_locked(&inner, token)?;
            let workspace = inner
                .workspace
                .as_ref()
                .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
            let mut projects = Vec::new();
            for raw in raw_projects {
                let raw = raw.as_str().ok_or_else(|| {
                    HostError::new("invalid_state", "A project path was not a string.")
                })?;
                projects.push(nuget::resolve_project(&workspace.root, raw)?);
            }
            (workspace.root.clone(), inner.project_sdk.clone(), projects)
        };
        let (muxer, sdk_root, sdk_version) = project_muxer(&sdk)?;
        let mut listings = Vec::new();
        for (project, display) in &projects {
            listings.push(nuget::list(
                &root,
                project,
                display,
                target_framework.as_deref(),
                &muxer,
                &sdk_root,
                &sdk_version,
                prerelease,
                source.as_deref(),
            ));
        }
        let total_packages: usize = listings.iter().map(|listing| listing.total_packages).sum();
        let outdated_count: usize = listings.iter().map(|listing| listing.outdated_count).sum();
        let failed = listings
            .iter()
            .filter(|listing| listing.state == "failed")
            .count();
        let partial = listings
            .iter()
            .filter(|listing| listing.state == "partial")
            .count();
        let state = if failed == listings.len() {
            "failed"
        } else if failed > 0 || partial > 0 {
            "partial"
        } else {
            "ok"
        };
        let message = if state == "ok" {
            None
        } else {
            Some(format!(
                "{failed} of {} project listings failed and {partial} were partial; the aggregate is incomplete.",
                listings.len()
            ))
        };
        Ok(json!({
            "state": state,
            "message": message,
            "targetFramework": target_framework,
            "prerelease": prerelease,
            "selectedSource": source,
            "projects": listings,
            "totalPackages": total_packages,
            "outdatedCount": outdated_count,
        }))
    }

    /// A batch is still data: every item is revalidated through `make_plan`, and nothing runs
    /// until `nuget.applyBatch` receives confirm:true and the current batch id.
    fn nuget_plan_batch(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("nuget.planBatch")?;
        let items = params
            .get("items")
            .and_then(Value::as_array)
            .ok_or_else(|| HostError::new("invalid_state", "items must be an array of updates."))?;
        if items.is_empty() {
            return Err(HostError::new(
                "invalid_state",
                "No package update was selected.",
            ));
        }
        if items.len() > 50 {
            return Err(HostError::new(
                "invalid_state",
                "A batch update is limited to 50 package updates.",
            ));
        }
        let (root, sdk) = {
            let inner = self.inner.lock().unwrap();
            accept_locked(&inner, token)?;
            let workspace = inner
                .workspace
                .as_ref()
                .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
            (workspace.root.clone(), inner.project_sdk.clone())
        };
        let (muxer, sdk_root, _) = project_muxer(&sdk)?;
        let mut batch = Vec::new();
        let mut seq = {
            let inner = self.inner.lock().unwrap();
            inner.nuget_seq
        };
        for item in items {
            let raw_project = item
                .get("project")
                .and_then(Value::as_str)
                .ok_or_else(|| HostError::new("invalid_state", "An update has no project."))?;
            let package = item
                .get("id")
                .and_then(Value::as_str)
                .ok_or_else(|| HostError::new("invalid_state", "An update has no package id."))?;
            let version = item
                .get("version")
                .and_then(Value::as_str)
                .ok_or_else(|| HostError::new("invalid_state", "An update has no version."))?;
            let from_version = item.get("fromVersion").and_then(Value::as_str);
            let target_framework = item
                .get("targetFramework")
                .and_then(Value::as_str)
                .filter(|tfm| !tfm.is_empty());
            let (project, display) = nuget::resolve_project(&root, raw_project)?;
            seq += 1;
            let id = nuget::plan_id(seq, token.context_id, &display, package, version);
            batch.push(nuget::make_plan(nuget::PlanRequest {
                id,
                workspace_id: token.workspace_id,
                context_id: token.context_id,
                root: root.clone(),
                project,
                project_display: display,
                package: package.to_string(),
                from_version: from_version.map(str::to_string),
                version: version.to_string(),
                target_framework: target_framework.map(str::to_string),
                muxer: muxer.clone(),
                sdk_root: sdk_root.clone(),
                install: false,
                defer_cpm: true,
            })?);
        }
        let plan_id = format!("nuget-batch-{}-{}", token.context_id, seq);
        let views: Vec<Value> = batch.iter().map(nuget::plan_view).collect();
        let mut inner = self.inner.lock().unwrap();
        accept_locked(&inner, token)?;
        inner.nuget_seq = seq;
        inner.nuget_plan = None;
        inner.nuget_batch = Some(batch);
        inner.nuget_batch_id = Some(plan_id.clone());
        Ok(json!({ "planId": plan_id, "count": views.len(), "items": views }))
    }

    /// The trust check and the synchronous restore are performed under the same host lock:
    /// a revoke can land before the check or after the restore, never between them.
    fn batch_restores(
        &self,
        batch: &[nuget::Plan],
        updated_projects: &[String],
        token: ContextToken,
        restore: impl Fn(&nuget::Plan) -> nuget::RestoreRun,
    ) -> Vec<Value> {
        let mut restores = Vec::new();
        for project in updated_projects {
            let guard = self.inner.lock().unwrap();
            if !restore_allowed(&guard, token) {
                restores.push(json!({
                    "state": "failed",
                    "message": "Trust was revoked or the workspace changed before the batch restore; no restore was run.",
                    "command": "",
                    "exit": null,
                    "log": "",
                }));
                drop(guard);
                break;
            }
            let Some(plan) = batch.iter().find(|plan| &plan.project_display == project) else {
                drop(guard);
                continue;
            };
            let result = restore(plan);
            drop(guard);
            restores.push(serde_json::to_value(result).expect("restore serializes"));
        }
        restores
    }

    /// Confirmed batch: each package gets its own `dotnet add`, failures are reported per
    /// package, and one explicit restore per project follows. Failures are never masked.
    fn nuget_apply_batch(&self, params: Value) -> HostResult<Value> {
        let token = self.require_trust("nuget.applyBatch")?;
        if params.get("confirm").and_then(Value::as_bool) != Some(true) {
            return Err(HostError::new(
                "confirmation_required",
                "nuget.applyBatch requires confirm:true; no command was run.",
            ));
        }
        let plan_id = required_str(&params, "planId")?;
        let batch = {
            let mut inner = self.inner.lock().unwrap();
            accept_locked(&inner, token)?;
            if inner.nuget_batch_id.as_deref() != Some(plan_id.as_str()) {
                return Err(HostError::new(
                    "nuget_stale_plan",
                    "The batch plan is not current; create a new plan. No command was run.",
                ));
            }
            inner.nuget_batch_id = None;
            inner.nuget_batch.take().ok_or_else(|| {
                HostError::new(
                    "nuget_stale_plan",
                    "The batch plan is not current; create a new plan. No command was run.",
                )
            })?
        };
        let mut items = Vec::new();
        let mut updated = 0;
        let mut failed = 0;
        let mut updated_projects: Vec<String> = Vec::new();
        for (index, plan) in batch.iter().enumerate() {
            // Trust is rechecked before every executable item: a concurrent revoke or a
            // workspace/context switch must not run the rest of the batch.
            let allowed = {
                let inner = self.inner.lock().unwrap();
                inner.trusted && accept_locked(&inner, token).is_ok()
            };
            if !allowed {
                for remaining in &batch[index..] {
                    failed += 1;
                    items.push(json!({
                        "state": "failed",
                        "project": remaining.project_display,
                        "package": remaining.package,
                        "fromVersion": remaining.from_version,
                        "version": remaining.version,
                        "exit": null,
                        "command": nuget::plan_view(remaining)["command"],
                        "message": "Trust was revoked or the workspace changed before this item; no command was run.",
                        "log": "",
                    }));
                }
                break;
            }
            match nuget::apply(plan) {
                Ok(result) if result.state == "ok" => {
                    updated += 1;
                    if !updated_projects
                        .iter()
                        .any(|project| project == &result.project)
                    {
                        updated_projects.push(result.project.clone());
                    }
                    items.push(json!({
                        "state": "ok",
                        "project": result.project,
                        "package": result.package,
                        "fromVersion": result.from_version,
                        "version": result.version,
                        "exit": result.exit,
                        "command": result.command,
                        "message": null,
                        "log": result.log,
                    }));
                }
                Ok(result) => {
                    failed += 1;
                    items.push(json!({
                        "state": "failed",
                        "project": result.project,
                        "package": result.package,
                        "fromVersion": result.from_version,
                        "version": result.version,
                        "exit": result.exit,
                        "command": result.command,
                        "message": result.message.unwrap_or_else(|| "dotnet add failed.".to_string()),
                        "log": result.log,
                    }));
                }
                Err(error) => {
                    failed += 1;
                    items.push(json!({
                        "state": "failed",
                        "project": plan.project_display,
                        "package": plan.package,
                        "fromVersion": plan.from_version,
                        "version": plan.version,
                        "exit": null,
                        "command": nuget::plan_view(plan)["command"],
                        "message": error.message,
                        "log": "",
                    }));
                }
            }
        }
        let restores = self.batch_restores(&batch, &updated_projects, token, nuget::restore);
        let restore_failed = restores
            .iter()
            .any(|restore| restore.get("state").and_then(Value::as_str) != Some("ok"));
        let state = if updated > 0 && failed == 0 && !restore_failed {
            "ok"
        } else if updated > 0 {
            "partial"
        } else {
            "failed"
        };
        Ok(json!({
            "state": state,
            "updated": updated,
            "failed": failed,
            "items": items,
            "restores": restores,
        }))
    }

    /// Workspace symbols come from the live analysis session only. Unavailability and trust
    /// are data (`state`), so the palette can say what is missing instead of showing zero.
    fn lsp_workspace_symbol(&self, params: Value) -> HostResult<Value> {
        let query = required_str(&params, "query")?;
        let (root, workspace_id, context_id, trusted, lsp) = {
            let inner = self.inner.lock().unwrap();
            let workspace = inner
                .workspace
                .as_ref()
                .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
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
                Some(
                    "Analysis is not running. Start analysis to search workspace symbols."
                        .to_string(),
                ),
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
                Some(
                    "Analysis is initializing; workspace symbols are not available yet."
                        .to_string(),
                ),
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
                let (items, truncated) = workspace_symbol_items(&list, &root);
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
                Some(format!(
                    "csharp-ls rejected the workspace symbol request: {message}"
                )),
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
        let bytes = fs::read(&path).map_err(|error| {
            HostError::new("operation_failed", format!("Could not read {rel}: {error}"))
        })?;
        let (content, _) = fsutil::decode_text(&bytes).map_err(|error| {
            HostError::new("operation_failed", format!("Could not read {rel}: {error}"))
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

        let (current, format) = read_for_save(&path, &rel)?;
        if content_hash(&current) != expected_hash {
            return Err(save_conflict(&rel, &current));
        }

        // The hash covers LF text, so it stays stable between read and save.
        let content = fsutil::normalize_newlines(&content);
        let temp = path.with_file_name(format!(
            ".openridr-save-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        if let Err(error) = fs::write(&temp, format.encode(&content)) {
            return Err(HostError::new(
                "operation_failed",
                format!("Could not write {rel}: {error}"),
            ));
        }
        #[cfg(test)]
        take_before_rename_hook();
        // Look again right before replacing: narrows the window for another writer's save.
        // shortcut: check-then-rename still has a tiny window; upgrade to an exclusive write if it matters.
        let latest = read_for_save(&path, &rel);
        let latest = match latest {
            Ok((latest, _)) => latest,
            Err(error) => {
                let _ = fs::remove_file(&temp);
                return Err(error);
            }
        };
        if content_hash(&latest) != expected_hash {
            let _ = fs::remove_file(&temp);
            return Err(save_conflict(&rel, &latest));
        }
        if let Err(error) = fs::rename(&temp, &path) {
            let _ = fs::remove_file(&temp);
            return Err(HostError::new(
                "operation_failed",
                format!("Could not replace {rel}: {error}"),
            ));
        }
        mark_inputs_stale(
            &mut inner,
            &[rel.as_str()],
            format!("{rel} was saved; the evaluated project model is stale."),
            false,
        );
        // A source save is shown as an obsolete build snapshot by the UI (buffer edit
        // time vs. build time); the host marks stale only when the evaluated context changes.
        Ok(json!({ "path": rel, "hash": content_hash(&content) }))
    }

    fn create_file(&self, params: Value) -> HostResult<Value> {
        let raw = required_str(&params, "path")?;
        let content = optional_str(&params, "content").unwrap_or_default();
        let mut inner = self.inner.lock().unwrap();
        let (path, rel) = resolve_new_in_workspace(&inner, &raw)?;
        let mut file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                return Err(HostError::new(
                    "already_exists",
                    format!("{rel} already exists; nothing was overwritten."),
                ));
            }
            Err(error) => {
                return Err(HostError::new(
                    "operation_failed",
                    format!("Could not create {rel}: {error}"),
                ));
            }
        };
        if let Err(error) = file.write_all(content.as_bytes()) {
            drop(file);
            let _ = fs::remove_file(&path);
            return Err(HostError::new(
                "operation_failed",
                format!("Could not write {rel}: {error}"),
            ));
        }
        drop(file);
        mark_inputs_stale(
            &mut inner,
            &[rel.as_str()],
            format!("{rel} was created and changed on disk; re-evaluate the project model."),
            false,
        );
        Ok(json!({ "path": rel }))
    }

    fn rename_path(&self, params: Value) -> HostResult<Value> {
        let from_raw = required_str(&params, "from")?;
        let to_raw = required_str(&params, "to")?;
        let mut inner = self.inner.lock().unwrap();
        let (from, from_rel) = resolve_existing_in_workspace(&inner, &from_raw)?;
        let (to, to_rel) = resolve_new_in_workspace(&inner, &to_raw)?;
        if fs::symlink_metadata(&to).is_ok() {
            return Err(HostError::new(
                "already_exists",
                format!("{to_rel} already exists; nothing was overwritten."),
            ));
        }
        if to.starts_with(&from) {
            return Err(HostError::new(
                "path_not_allowed",
                format!("A folder cannot be moved inside itself: {from_rel} -> {to_rel}"),
            ));
        }
        let directory = fs::symlink_metadata(&from)
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false);
        fs::rename(&from, &to).map_err(|error| {
            HostError::new(
                "operation_failed",
                format!("Could not rename {from_rel} to {to_rel}: {error}"),
            )
        })?;
        mark_inputs_stale(
            &mut inner,
            &[from_rel.as_str(), to_rel.as_str()],
            format!(
                "{from_rel} was renamed to {to_rel}; the workspace changed on disk, so re-evaluate the project model."
            ),
            directory,
        );
        Ok(json!({ "from": from_rel, "to": to_rel }))
    }

    fn delete_path(&self, params: Value) -> HostResult<Value> {
        let raw = required_str(&params, "path")?;
        let mut inner = self.inner.lock().unwrap();
        let (path, rel) = resolve_existing_in_workspace(&inner, &raw)?;
        let directory = fs::symlink_metadata(&path)
            .map(|metadata| metadata.is_dir())
            .unwrap_or(false);
        let result = if directory {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        result.map_err(|error| {
            HostError::new(
                "operation_failed",
                format!("Could not delete {rel}: {error}"),
            )
        })?;
        mark_inputs_stale(
            &mut inner,
            &[rel.as_str()],
            format!("{rel} was deleted; the workspace changed on disk, so re-evaluate the project model."),
            directory,
        );
        Ok(json!({ "path": rel, "directory": directory }))
    }

    /// Selects a file or folder in Finder. It never runs project code, so it takes no trust
    /// gate; the containment check is the only thing between the UI and the filesystem.
    fn reveal_path(&self, params: Value) -> HostResult<Value> {
        let raw = required_str(&params, "path")?;
        let root = {
            let inner = self.inner.lock().unwrap();
            inner
                .workspace
                .as_ref()
                .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?
                .root
                .clone()
        };
        let (path, rel) = resolve_reveal_in_workspace(&root, &raw)?;
        let (program, args) = fsutil::reveal_command(std::env::consts::OS, &path)?;
        let status = Command::new(program)
            .args(&args)
            .status()
            .map_err(|error| {
                HostError::new(
                    "operation_failed",
                    format!("Could not open the Finder for {rel}: {error}"),
                )
            })?;
        if !status.success() {
            return Err(HostError::new(
                "operation_failed",
                format!("The Finder refused {rel} with {status}."),
            ));
        }
        Ok(json!({ "path": rel }))
    }
}

impl Inner {
    fn ensure_inventory(&mut self) {
        if self.inventory.is_none() {
            self.inventory = Some(sdk::inventory());
        }
    }

    /// Fixed tools are project setup, not a user decision: verify/install both without trust.
    /// A failure is recorded and surfaced by the snapshot instead of making `workspace.open`
    /// fail; the setup screen owns retry.
    fn begin_setup(&mut self) -> HostResult<()> {
        let workspace_id = self
            .workspace
            .as_ref()
            .ok_or_else(|| HostError::new("invalid_state", "Open a workspace before setup."))?
            .id;
        self.setup_error = None;
        self.setup_attempt += 1;
        if self.setup_started_at.is_none() {
            self.setup_started_at = Some(now_ms());
        }
        self.setup_finished_at = None;
        self.ensure_inventory();
        let runtime =
            AnalysisRuntime::for_analysis(&self.inventory.as_ref().expect("inventory").runtimes);
        if runtime.state != "found" {
            self.setup_error = runtime.message.clone();
            return Ok(());
        }
        if debugger::bundled_binary().is_some() {
            // The adapter ships with the app; it is enabled by default, with no experimental
            // consent. The flag still exists as the platform gate for non-bundled installs.
            self.debugger_experimental = true;
        }
        if let Err(error) = debugger::ready_adapter(
            std::env::consts::OS,
            std::env::consts::ARCH,
            self.debugger_experimental,
        ) {
            self.setup_error = Some(format!(
                "The debug adapter is not available: {}",
                error.message
            ));
        }
        // The analysis pair ships with the app on macOS ARM64: there is no plan and nothing to
        // download. A damaged bundle fails the setup here instead of falling back to the plan.
        match bundled::availability() {
            bundled::Availability::Damaged(reason) => {
                self.tools.sync_bundled();
                if self.setup_error.is_none() {
                    self.setup_error = Some(reason);
                }
                return Ok(());
            }
            bundled::Availability::Bundled(_) => {
                self.tools.sync_bundled();
                let tool = self.tools.view();
                if tool.state == "failed" && self.setup_error.is_none() {
                    self.setup_error = tool
                        .error
                        .clone()
                        .or_else(|| Some("The bundled analysis tool did not verify.".to_string()));
                }
                return Ok(());
            }
            bundled::Availability::Absent => {}
        }
        let plan = match tools::plan(self.inventory.as_ref().expect("inventory")) {
            Ok(plan) => plan,
            Err(error) => {
                if self.setup_error.is_none() {
                    self.setup_error = Some(error.message);
                }
                return Ok(());
            }
        };
        let tools = self.tools.clone();
        if matches!(
            tools.view().state,
            "downloading" | "installing" | "verifying"
        ) {
            return Ok(());
        }
        if let Err(error) = tools.begin_install(
            plan,
            workspace_id,
            self.context_seq,
            self.processes.clone(),
            false,
        ) {
            if error.code != "invalid_state" && self.setup_error.is_none() {
                self.setup_error = Some(error.message);
            }
        }
        Ok(())
    }

    fn sync_tools(&mut self) {
        self.tools.sync_bundled();
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
            "ok" => {
                "Restore completed; reload the evaluated model before using TargetPath or packages."
            }
            "cancelled" => {
                "Restore was cancelled; package dependencies and analysis may be incomplete."
            }
            "failed" => "Restore failed; package dependencies and analysis may be incomplete.",
            _ => return,
        };
        self.evaluation.mark_stale(reason);
        self.build
            .mark_stale("Dependencies changed after the build; rebuild before using its result.");
        self.tests.mark_stale(
            "Dependencies changed after discovery; discover tests again before using the selection.",
        );
        self.test_run
            .mark_stale("Dependencies changed after this test run; its results are not current.");
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
        if status
            .as_ref()
            .is_some_and(|status| status.state == "exited")
        {
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

    /// The one shape every reader of the Tests state gets: the discovery plus the real last run.
    /// `Tests::view()` always carries an idle run, so a caller that wants the run must not read
    /// the discovery view directly (that is how the export used to write an empty run).
    fn tests_view(&self) -> test_discovery::TestsView {
        let mut view = self.tests.view();
        view.run = self.test_run.view();
        view
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

/// The build a pending launch waits for: still running, finished, failed, cancelled or gone.
fn pending_outcome(build: &build::BuildView, pending_build_id: &str) -> PendingOutcome {
    if build.revision == 0 || build.build_id.as_deref() != Some(pending_build_id) {
        return PendingOutcome::Replaced;
    }
    if build.state == "running" {
        return PendingOutcome::Wait;
    }
    if build.state == "cancelled" {
        return PendingOutcome::Cancelled;
    }
    if build.state == "ok" && build.exit == Some(0) {
        return PendingOutcome::Launch;
    }
    PendingOutcome::Failed
}

/// A workspace boundary (revoke, close, switch) terminates a real build and keeps only what
/// was observed. The record is sealed so its late exit cannot become another context's build.
/// A Run/Debug waiting for that build is cancelled: it must never launch afterwards.
fn stop_build(inner: &mut Inner, reason: &str) {
    if let Some(process_id) = inner.build.running_process_id() {
        let status = inner.processes.lock().unwrap().terminate_status(process_id);
        inner.build.seal(reason, status.as_ref());
    } else {
        inner.build.mark_stale(reason);
    }
    if inner.pending_run.take().is_some() {
        inner.run.cancel_pending();
    }
    if inner.pending_debug.take().is_some() {
        inner.dap.cancel_pending();
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
            && status
                .as_ref()
                .is_some_and(|status| status.state != "exited")
        {
            std::thread::sleep(std::time::Duration::from_millis(50));
            status = inner.processes.lock().unwrap().status(process_id);
        }
        if status
            .as_ref()
            .is_some_and(|status| status.state != "exited")
        {
            status = inner.processes.lock().unwrap().terminate_status(process_id);
        }
        inner
            .processes
            .lock()
            .unwrap()
            .stop_remaining_group(process_id);
        if let Some(status) = status {
            lsp.observe_termination(&status);
        }
    }
}

/// Every workspace boundary stops analysis. Tools are global to the app home and deliberately
/// continue across a workspace switch; sealing here used to cancel a Reinstall after its swap
/// and leave the old installation only in staging. The host shutdown path seals them.
fn stop_services(inner: &mut Inner, reason: &str) {
    stop_lsp_inner(inner, reason);
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
                sdk.message
                    .clone()
                    .unwrap_or_else(|| { "Resolve the project SDK first.".to_string() })
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

    let requested_framework = match params.get("targetFramework") {
        Some(Value::String(value)) if !value.trim().is_empty() => Some(value.trim().to_string()),
        Some(_) => None,
        None => evaluation.context.target_framework.clone(),
    };
    if requested_framework
        .as_deref()
        .is_some_and(|tfm| !valid_selection(tfm))
    {
        return Err(HostError::new(
            "invalid_state",
            "TFM must not contain `;`, `=`, quotes or line breaks; no test operation was started.",
        ));
    }
    if requested_framework != evaluation.context.target_framework {
        return Err(HostError::with_details(
            "invalid_state",
            format!(
                "{label} was not started: TFM {} has not been evaluated. Re-evaluate with that TFM first.",
                requested_framework.unwrap_or_else(|| "not selected".to_string())
            ),
            json!({ "reason": "tfm_not_evaluated", "action": { "method": "evaluation.run", "params": {} } }),
        ));
    }
    // A project with one evaluated TFM has exactly one framework the runner could use, so the
    // operation names it instead of letting every row report "TFM not selected". A multi-TFM
    // project keeps the explicit selection and is refused above when none was made.
    let target_framework = test_framework(project, requested_framework);

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

/// The TFM a test operation runs: the explicit/evaluated selection when there is one, otherwise
/// the only framework the evaluated project declares.
fn test_framework(
    project: &evaluation::EvaluatedProject,
    selected: Option<String>,
) -> Option<String> {
    if selected.is_some() {
        return selected;
    }
    let mut tfms = project.tfms.iter().filter(|tfm| !tfm.trim().is_empty());
    match (tfms.next(), tfms.next()) {
        (Some(only), None) => Some(only.clone()),
        _ => None,
    }
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

/// Store key for the trust/recent methods: the canonical path when it exists. `must_exist` (trust.set,
/// which writes) rejects a missing path; reads and `recent.remove` accept it as given.
fn trust_key(params: &Value, must_exist: bool) -> HostResult<String> {
    let raw = required_str(params, "path")?;
    if !Path::new(&raw).is_absolute() {
        return Err(HostError::new("path_not_allowed", "Path must be absolute."));
    }
    match fs::canonicalize(&raw) {
        Ok(path) => Ok(path.to_string_lossy().to_string()),
        Err(_) if must_exist => Err(HostError::new(
            "not_found",
            format!("Path not found: {raw}"),
        )),
        Err(_) => Ok(raw),
    }
}

/// Shared by an explicit grant and by opening an already-trusted path.
/// Host-side setup answer, not just a UI state: every executable gate and trust grant asks
/// this before releasing the workspace. The test seam is the only bypass.
fn restore_allowed(inner: &Inner, token: ContextToken) -> bool {
    inner.trusted && accept_locked(inner, token).is_ok()
}

/// Fail-closed answer for a distributed app whose bundle lost the analyzer: no plan, no download
/// and no external runtime. `None` for a development binary, where the download flow still applies.
fn bundled_damaged() -> Option<HostError> {
    match bundled::availability() {
        bundled::Availability::Damaged(reason) => Some(HostError::with_details(
            "bundled_incomplete",
            reason,
            json!({}),
        )),
        _ => None,
    }
}

fn is_setup_ready(inner: &Inner) -> bool {
    if inner.setup_override {
        return true;
    }
    if inner.workspace.is_none() || inner.setup_error.is_some() {
        return false;
    }
    let Some(inventory) = inner.inventory.as_ref() else {
        return false;
    };
    if AnalysisRuntime::for_analysis(&inventory.runtimes).state != "found" {
        return false;
    }
    inner.tools.still_ready()
        && inner
            .debugger
            .view(inner.debugger_experimental)
            .adapter_state
            == "ready"
}

fn apply_trust(inner: &mut Inner, workspace_id: u64) {
    inner.ensure_inventory();
    inner.trusted = true;
    inner.context_seq += 1;
    inner.nuget_plan = None;
    inner.trust_decision = Some(TrustDecision {
        granted_at: now_ms(),
        workspace_id,
        revoked_at: None,
    });
    let project = inspect_project_sdk(inner);
    inner.project_sdk = project;
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
            return ProjectSdk::invalid(
                "global.json",
                format!("global.json is not valid JSON: {error}"),
            )
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
    match sdk::resolve(version, roll_forward, allow_prerelease, &inventory.installs) {
        Ok(install) => ProjectSdk::resolved(&request, &install),
        Err(message) => ProjectSdk::missing(&request, message),
    }
}

fn global_json_request(outline: &Outline) -> Option<GlobalJsonRequest> {
    outline
        .global_json
        .as_ref()
        .map(|global| GlobalJsonRequest {
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
    params.get(key).and_then(Value::as_str).map(str::to_string)
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
        || lower.ends_with(".slnx")
        || lower.ends_with("/global.json")
        || lower == "global.json"
}

/// Marks the evaluated model stale when a file operation touched an evaluation input (or a
/// whole folder). The operation itself never evaluates or builds; the freshness path does.
fn mark_inputs_stale(inner: &mut Inner, inputs: &[&str], reason: String, force: bool) {
    if !force && !inputs.iter().any(|rel| is_evaluation_input(rel)) {
        return;
    }
    if inner.evaluation.mark_stale(reason) {
        inner.context_seq += 1;
        inner.nuget_plan = None;
        inner.nuget_batch = None;
        inner.nuget_batch_id = None;
        stop_lsp_inner(
            inner,
            "A project input changed; analysis stopped so it cannot report the previous context.",
        );
    }
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
    let root = sdk.root.clone().map(PathBuf::from).ok_or_else(|| {
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

fn debug_adapter_command(adapter: &Path, sdk_root: &Path) -> HostResult<Command> {
    let muxer = sdk_root.join(dotnet_exe());
    if !muxer.is_file() {
        return Err(HostError::new(
            "sdk_missing",
            format!(
                "Debug was not started: the project .NET executable {} is missing.",
                muxer.display()
            ),
        ));
    }
    let inherited_path = std::env::var_os("PATH").unwrap_or_default();
    let paths =
        std::iter::once(sdk_root.to_path_buf()).chain(std::env::split_paths(&inherited_path));
    let path = std::env::join_paths(paths).map_err(|error| {
        HostError::new(
            "invalid_state",
            format!(
                "Debug was not started: cannot configure PATH for {}: {error}",
                muxer.display()
            ),
        )
    })?;
    let mut command = Command::new(adapter);
    command.arg("--interpreter=vscode").env("PATH", path);
    Ok(command)
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

/// Workspace extensions, compared without case: `.SLN` opens like `.sln`.
fn workspace_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(str::to_ascii_lowercase)
}

fn workspace_file(path: &Path) -> HostResult<PathBuf> {
    match workspace_extension(path).as_deref() {
        Some("sln" | "slnx" | "csproj") => Ok(path.to_path_buf()),
        _ => Err(HostError::new(
            "path_not_allowed",
            "Open a .sln, a .slnx, a .csproj or a directory that contains one.",
        )),
    }
}

fn resolve_directory(dir: &Path) -> HostResult<PathBuf> {
    let mut solutions = files_with_extension(dir, "sln");
    solutions.extend(files_with_extension(dir, "slnx"));
    solutions.sort();
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
            format!(
                "No .sln, .slnx or .csproj found in {}.",
                dir.to_string_lossy()
            ),
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
    let kind = match workspace_extension(path).as_deref() {
        Some("sln" | "slnx") => WorkspaceKind::Solution,
        Some("csproj") => WorkspaceKind::Project,
        _ => {
            return Err(HostError::new(
                "path_not_allowed",
                "Workspace must be a .sln, a .slnx or a .csproj file.",
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
            None => {
                "the action requires a command this build cannot read; commands are never executed"
                    .to_string()
            }
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
    // The same confinement contract as the editor frontiers, including the dangling-symlink rule:
    // an unprovable target is refused, and only the wording differs because this path answers to
    // the analysis session instead of the UI.
    let canonical =
        confine(root, Path::new(&raw), shape_of(&raw)).map_err(|error| match error {
            fsutil::ConfineError::Outside => format!("{raw} is outside the workspace"),
            fsutil::ConfineError::Missing => format!("{raw} is not readable"),
        })?;
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
    let raw = lsp::uri_to_path(&target.uri)?;
    // Same contract as every other frontier: a target outside the workspace — including one behind a
    // dangling symlink — is dropped, never offered as a location.
    let canonical = confine(root, Path::new(&raw), shape_of(&raw)).ok()?;
    if !canonical.is_file() {
        return None;
    }
    let rel = workspace_relative(root, &canonical);
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

/// Resolves a create/rename target or a rename/delete source. Only the parent is canonicalized,
/// so the last component may still be a symlink (the operation renames or deletes the link, never
/// its target). `must_exist` distinguishes an existing source from a destination that may be new.
fn resolve_path_in_workspace(
    inner: &Inner,
    raw: &str,
    must_exist: bool,
) -> HostResult<(PathBuf, String)> {
    let workspace = inner
        .workspace
        .as_ref()
        .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
    if raw.trim().is_empty() {
        return Err(HostError::new("path_not_allowed", "File path is empty."));
    }
    let requested = Path::new(raw);
    if requested
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(HostError::new(
            "outside_workspace",
            format!("Path is outside the workspace: {raw}"),
        ));
    }
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        workspace.root.join(requested)
    };
    let name = candidate.file_name().ok_or_else(|| {
        HostError::new(
            "path_not_allowed",
            format!("A file or folder name is required: {raw}"),
        )
    })?;
    let parent = candidate.parent().ok_or_else(|| {
        HostError::new(
            "path_not_allowed",
            format!("A file or folder name is required: {raw}"),
        )
    })?;
    // The shared contract: a joined path that normalizes outside is refused by shape, then the
    // canonical step decides containment (and sees through a symlinked folder).
    let parent = confine(&workspace.root, parent, shape_of(raw))
        .map_err(|error| confine_error(error, raw, "Folder not found"))?;
    if !parent.is_dir() {
        return Err(HostError::new(
            "not_found",
            format!("Folder not found: {raw}"),
        ));
    }
    let target = parent.join(name);
    if target == workspace.root {
        return Err(HostError::new(
            "path_not_allowed",
            "The workspace root cannot be renamed or deleted.",
        ));
    }
    if must_exist && fs::symlink_metadata(&target).is_err() {
        return Err(HostError::new(
            "not_found",
            format!("File or folder not found: {raw}"),
        ));
    }
    let rel = workspace_relative(&workspace.root, &target);
    Ok((target, rel))
}

fn resolve_new_in_workspace(inner: &Inner, raw: &str) -> HostResult<(PathBuf, String)> {
    resolve_path_in_workspace(inner, raw, false)
}

fn resolve_existing_in_workspace(inner: &Inner, raw: &str) -> HostResult<(PathBuf, String)> {
    resolve_path_in_workspace(inner, raw, true)
}

/// Reveal accepts files and folders, including the workspace root itself (empty path); every
/// other resolver refuses the root because they rename or delete their target.
fn resolve_reveal_in_workspace(root: &Path, raw: &str) -> HostResult<(PathBuf, String)> {
    let requested = Path::new(raw);
    let candidate = if raw.trim().is_empty() {
        root.to_path_buf()
    } else if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root.join(requested)
    };
    let canonical = confine(root, &candidate, shape_of(raw))
        .map_err(|error| confine_error(error, raw, "File or folder not found"))?;
    Ok((canonical.clone(), workspace_relative(root, &canonical)))
}

fn resolve_in_workspace(inner: &Inner, raw: &str) -> HostResult<(PathBuf, String)> {
    let workspace = inner
        .workspace
        .as_ref()
        .ok_or_else(|| HostError::new("invalid_state", "No workspace is open."))?;
    resolve_in_root(&workspace.root, raw)
}

/// Same confinement rules as `resolve_in_workspace`, but takes the root directly so callers can
/// drop the host lock before touching the disk.
fn resolve_in_root(root: &Path, raw: &str) -> HostResult<(PathBuf, String)> {
    if raw.trim().is_empty() {
        return Err(HostError::new("path_not_allowed", "File path is empty."));
    }
    let canonical = confine_raw(root, raw, "File not found")?;
    if !canonical.is_file() {
        return Err(HostError::new(
            "path_not_allowed",
            format!("Not a file: {raw}"),
        ));
    }
    Ok((canonical.clone(), workspace_relative(root, &canonical)))
}

/// Which textual comparison a frontier can make before touching the disk.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shape {
    /// The candidate was built by joining a relative path onto the root, so a path that normalizes
    /// outside is refused with no I/O at all.
    Joined,
    /// The caller passed an absolute path. Its prefix may be a symlink or differ in case
    /// (`/tmp/ws` and `/private/tmp/ws` are the same folder), so containment is decided by
    /// canonicalization only — which never reveals whether something exists outside.
    Absolute,
}

pub(crate) fn shape_of(raw: &str) -> Shape {
    if Path::new(raw).is_absolute() {
        Shape::Absolute
    } else {
        Shape::Joined
    }
}

/// The confinement contract, implemented once for every frontier (`file.*`, analysis,
/// `workspace.tree`, `file.reveal`, NuGet):
/// 1. a joined relative path that normalizes outside the root is refused by shape, before any I/O;
/// 2. the nearest **existing** ancestor must canonicalize inside the root, so a symlink that carries
///    the path outside is refused the same way whether or not the target exists (no existence
///    oracle: `LinkDir/etc/hosts` and `LinkDir/missing` answer alike);
/// 3. the target itself is canonicalized and checked again.
pub(crate) fn confine(
    root: &Path,
    candidate: &Path,
    shape: Shape,
) -> Result<PathBuf, fsutil::ConfineError> {
    if shape == Shape::Joined && !fsutil::stays_inside(root, candidate) {
        return Err(fsutil::ConfineError::Outside);
    }
    fsutil::canonical_confined(root, candidate)
}

fn confine_error(error: fsutil::ConfineError, raw: &str, missing: &str) -> HostError {
    match error {
        fsutil::ConfineError::Outside => HostError::new(
            "outside_workspace",
            format!("Path is outside the workspace: {raw}"),
        ),
        fsutil::ConfineError::Missing => HostError::new("not_found", format!("{missing}: {raw}")),
    }
}

/// Resolves a caller path (absolute or relative) against the workspace root; used by the NuGet
/// plan too, so every frontier shares one implementation of the contract above.
pub(crate) fn confine_raw(root: &Path, raw: &str, missing: &str) -> HostResult<PathBuf> {
    let requested = Path::new(raw);
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        root.join(requested)
    };
    confine(root, &candidate, shape_of(raw)).map_err(|error| confine_error(error, raw, missing))
}

/// The workspace-relative display path (`/`-separated) every snapshot and UI answer uses.
fn workspace_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn modified_ms(metadata: &fs::Metadata) -> u64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn content_hash(content: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in content.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64:{hash:016x}")
}

/// Reads a save destination and its encoding; both pre-save checks share this wording.
fn read_for_save(path: &Path, rel: &str) -> HostResult<(String, fsutil::TextFormat)> {
    let bytes = fs::read(path).map_err(|error| {
        HostError::new(
            "operation_failed",
            format!("Could not read {rel} before saving: {error}"),
        )
    })?;
    fsutil::decode_text(&bytes).map_err(|error| {
        HostError::new(
            "operation_failed",
            format!("Could not read {rel} before saving: {error}"),
        )
    })
}

/// The conflict both pre-save checks report, carrying the text currently on disk.
fn save_conflict(rel: &str, current: &str) -> HostError {
    HostError::with_details(
        "conflict",
        format!("{rel} changed on disk since it was read; the buffer was not saved."),
        json!({ "currentContent": current, "currentHash": content_hash(current) }),
    )
}

#[cfg(test)]
thread_local! {
    /// Lets a save test change the destination between the temp write and the pre-rename recheck.
    static BEFORE_RENAME: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        std::cell::RefCell::new(None);
}

#[cfg(test)]
fn take_before_rename_hook() {
    BEFORE_RENAME.with(|hook| {
        if let Some(hook) = hook.borrow_mut().take() {
            hook();
        }
    });
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

/// Native file chooser for the welcome screen (macOS only, like the verified target).
/// Returns `{ "path": null }` when the user cancels. No lock is held while it is open.
fn pick_workspace() -> HostResult<Value> {
    if !cfg!(target_os = "macos") {
        return Err(HostError::new(
            "unsupported",
            "The file chooser is only implemented on macOS; type the path instead.",
        ));
    }
    let output = Command::new("osascript")
        .args([
            "-e",
            r#"POSIX path of (choose file with prompt "Open solution or project" default location (path to home folder) of type {"sln","slnx","csproj"})"#,
        ])
        .output()
        .map_err(|error| HostError::new("pick_failed", format!("Cannot start the file chooser: {error}")))?;
    if !output.status.success() {
        // osascript exits 1 with "User canceled." when the dialog is dismissed.
        return Ok(json!({ "path": null }));
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(json!({ "path": if path.is_empty() { None } else { Some(path) } }))
}

/// Keeps only symbols whose file is inside the workspace. Dropping an outside symbol is
/// filtering, not truncation: `truncated` mirrors the cap applied while parsing the LSP answer.
fn workspace_symbol_items(list: &lsp::WorkspaceSymbolList, root: &Path) -> (Vec<Value>, bool) {
    let items = list
        .items
        .iter()
        .filter_map(|item| {
            let path = lsp::uri_to_path(&item.uri)?;
            let canonical = fs::canonicalize(&path).ok()?;
            let rel = canonical
                .strip_prefix(root)
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
    (items, list.truncated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_adapter_uses_project_muxer_before_inherited_path() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(dotnet_exe()), "").unwrap();
        let command = debug_adapter_command(Path::new("adapter"), root.path()).unwrap();
        let path = command
            .get_envs()
            .find(|(key, _)| *key == "PATH")
            .unwrap()
            .1
            .unwrap();
        assert_eq!(std::env::split_paths(path).next().unwrap(), root.path());
    }

    #[test]
    fn debug_adapter_reports_missing_project_muxer() {
        let root = tempfile::tempdir().unwrap();
        let error = debug_adapter_command(Path::new("adapter"), root.path()).unwrap_err();
        let muxer = root.path().join(dotnet_exe());
        assert!(error.message.contains(muxer.to_string_lossy().as_ref()));
    }

    #[cfg(unix)]
    #[test]
    fn dropping_host_stops_its_language_server() {
        let host = Host::new();
        let pid;
        {
            let mut inner = host.inner.lock().unwrap();
            let mut command = Command::new("/bin/sleep");
            command.arg("60");
            let child = inner
                .processes
                .lock()
                .unwrap()
                .spawn_piped("lsp:csharp-ls", 1, command)
                .unwrap();
            pid = child.pid;
            let server = lsp::Lsp::begin(
                1,
                1,
                1,
                "",
                false,
                false,
                "",
                None,
                vec![],
                "".into(),
                false,
                "partial",
                None,
            );
            server.attach(
                child.id,
                child.pid,
                child.stdin,
                lsp::spawn_killer(child.pid),
            );
            inner.lsp = Some(server);
        }
        drop(host);
        let still_alive = process::process_alive(pid);
        if still_alive {
            process::kill_group(pid, true);
        }
        assert!(!still_alive, "language server survived host shutdown");
    }

    #[test]
    fn reveal_resolves_inside_the_workspace_only() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/App.cs"), "").unwrap();

        let (path, rel) = resolve_reveal_in_workspace(&root, "src/App.cs").unwrap();
        assert_eq!(rel, "src/App.cs");
        assert_eq!(path, root.join("src/App.cs"));
        assert_eq!(resolve_reveal_in_workspace(&root, "src").unwrap().1, "src");
        // The solution row reveals the workspace root, which the rename/delete resolver refuses.
        assert_eq!(resolve_reveal_in_workspace(&root, "").unwrap().1, "");

        let outside = tempfile::tempdir().unwrap();
        let outside_file = fs::canonicalize(outside.path()).unwrap().join("App.cs");
        fs::write(&outside_file, "").unwrap();
        let error =
            resolve_reveal_in_workspace(&root, &outside_file.to_string_lossy()).unwrap_err();
        assert_eq!(error.code, "outside_workspace");
        let error = resolve_reveal_in_workspace(&root, "../").unwrap_err();
        assert_eq!(error.code, "outside_workspace");
        let error = resolve_reveal_in_workspace(&root, "missing.cs").unwrap_err();
        assert_eq!(error.code, "not_found");
    }

    #[test]
    fn symbols_outside_the_workspace_are_filtered_not_truncated() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let outside = tempfile::tempdir().unwrap();
        let inside_file = root.join("Program.cs");
        let outside_file = fs::canonicalize(outside.path()).unwrap().join("Program.cs");
        fs::write(&inside_file, "").unwrap();
        fs::write(&outside_file, "").unwrap();
        let item = |path: &Path| lsp::WorkspaceSymbolItem {
            name: "Program".into(),
            kind: 5,
            container_name: String::new(),
            uri: format!("file://{}", path.display()),
            range: json!({}),
        };
        let list = lsp::WorkspaceSymbolList {
            items: vec![item(&inside_file), item(&outside_file)],
            total: 2,
            truncated: false,
        };
        let (items, truncated) = workspace_symbol_items(&list, &root);
        assert_eq!(items.len(), 1);
        assert!(
            !truncated,
            "a filtered outside symbol is not a host truncation"
        );
        let capped = lsp::WorkspaceSymbolList {
            truncated: true,
            ..list
        };
        assert!(
            workspace_symbol_items(&capped, &root).1,
            "real truncation is kept"
        );
    }

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
    fn workspace_tree_lists_one_level_inside_the_workspace() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let project = root.join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>").unwrap();
        fs::create_dir_all(root.join("bin")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".gitignore"), "bin/\n").unwrap();
        let host = Host::default();
        host.dispatch("workspace.open", json!({ "path": project }))
            .unwrap();
        let tree = host.dispatch("workspace.tree", json!({})).unwrap();
        let names: Vec<(&str, bool)> = tree["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| (e["name"].as_str().unwrap(), e["hidden"].as_bool().unwrap()))
            .collect();
        assert_eq!(
            names,
            vec![("bin", true), (".gitignore", true), ("App.csproj", false)]
        );
        let outside = host
            .dispatch("workspace.tree", json!({ "dir": ".." }))
            .unwrap_err();
        assert_eq!(outside.code, "outside_workspace");
    }

    #[test]
    fn workspace_symbol_kinds_reads_files_after_releasing_the_host_lock() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let project = root.join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>").unwrap();
        fs::write(root.join("Widget.cs"), "public sealed class Widget {}\n").unwrap();
        let host = Host::default();
        host.dispatch("workspace.open", json!({ "path": project }))
            .unwrap();
        let result = host
            .workspace_symbol_kinds_with(&json!({ "paths": ["Widget.cs"] }), |path| {
                // If the method still held `inner.lock()`, the same-thread try_lock would fail.
                assert!(
                    host.inner.try_lock().is_ok(),
                    "host lock must be free during file I/O"
                );
                fs::read_to_string(path).ok()
            })
            .unwrap();
        assert_eq!(result["entries"][0]["kind"], "class");
    }

    #[test]
    fn workspace_symbol_kinds_flags_more_than_256_paths() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let project = root.join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>").unwrap();
        fs::write(root.join("Widget.cs"), "public sealed class Widget {}\n").unwrap();
        let host = Host::default();
        host.dispatch("workspace.open", json!({ "path": project }))
            .unwrap();
        let paths: Vec<&str> = (0..257).map(|_| "Widget.cs").collect();
        let result = host
            .dispatch("workspace.symbolKinds", json!({ "paths": paths }))
            .unwrap();
        assert_eq!(result["truncated"], true);
        assert_eq!(result["entries"].as_array().unwrap().len(), 256);
    }

    #[test]
    fn workspace_symbol_kinds_scans_only_workspace_sources() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let project = root.join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>").unwrap();
        fs::write(
            root.join("Widget.cs"),
            "namespace App;\npublic sealed class Widget {}\n",
        )
        .unwrap();
        fs::write(root.join("IRepo.cs"), "public interface IRepo {}\n").unwrap();
        fs::write(root.join("notes.txt"), "class NotACsharpFile {}\n").unwrap();
        let outside = root.parent().unwrap().join("Outside.cs");
        fs::write(&outside, "public class Outside {}\n").unwrap();

        let host = Host::default();
        host.dispatch("workspace.open", json!({ "path": project }))
            .unwrap();
        let result = host.dispatch(
            "workspace.symbolKinds",
            json!({ "paths": ["Widget.cs", "IRepo.cs", "notes.txt", "../Outside.cs", "Missing.cs"] }),
        )
        .unwrap();
        let entries = result["entries"].as_array().unwrap();
        let entry = |path: &str| entries.iter().find(|entry| entry["path"] == path);
        assert_eq!(entry("Widget.cs").unwrap()["kind"], "class");
        assert_eq!(entry("IRepo.cs").unwrap()["kind"], "interface");
        assert!(entry("notes.txt").is_none());
        assert!(entry("../Outside.cs").is_none());
        assert!(entry("Missing.cs").is_none());
        assert_eq!(result["scanned"], 2);
        assert_eq!(result["skipped"], 0);
        assert_eq!(result["truncated"], false);
    }

    #[test]
    fn workspace_symbol_kinds_uses_known_mtimes_and_caches_misses() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let project = root.join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>").unwrap();
        fs::write(root.join("Widget.cs"), "public sealed class Widget {}\n").unwrap();
        fs::write(
            root.join("Large.cs"),
            format!(
                "// {}\npublic sealed class Large {{}}\n",
                "x".repeat(300 * 1024)
            ),
        )
        .unwrap();

        let host = Host::default();
        host.dispatch("workspace.open", json!({ "path": project }))
            .unwrap();
        let first = host
            .dispatch(
                "workspace.symbolKinds",
                json!({ "paths": ["Widget.cs", "Large.cs"] }),
            )
            .unwrap();
        let entries = first["entries"].as_array().unwrap();
        let find = |path: &str| entries.iter().find(|entry| entry["path"] == path).unwrap();
        assert_eq!(find("Widget.cs")["kind"], "class");
        assert!(find("Large.cs")["kind"].is_null()); // too large to read, still a cacheable answer
        let modified = find("Widget.cs")["modified"].as_u64().unwrap();
        let large_modified = find("Large.cs")["modified"].as_u64().unwrap();
        assert!(modified > 0);

        let second = host
            .dispatch(
                "workspace.symbolKinds",
                json!({
                    "paths": ["Widget.cs", "Large.cs"],
                    "known": { "Widget.cs": modified, "Large.cs": large_modified },
                }),
            )
            .unwrap();
        assert_eq!(second["entries"].as_array().unwrap().len(), 0);
        assert_eq!(second["skipped"], 2);
        assert_eq!(second["scanned"], 0);

        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(root.join("Widget.cs"), "public interface Widget {}\n").unwrap();
        let third = host
            .dispatch(
                "workspace.symbolKinds",
                json!({ "paths": ["Widget.cs"], "known": { "Widget.cs": modified } }),
            )
            .unwrap();
        assert_eq!(third["entries"][0]["kind"], "interface");
        assert_ne!(third["entries"][0]["modified"].as_u64().unwrap(), modified);
    }

    #[test]
    fn workspace_files_and_text_search_need_no_trust() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let project = root.join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>").unwrap();
        fs::write(
            root.join("Program.cs"),
            "class Program\n{\n    string s = \"needle\";\n}\n",
        )
        .unwrap();
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
        host.allow_setup_for_tests();

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
        fs::write(
            root.join("Program.cs"),
            "class Program { static void Main() {} }\n",
        )
        .unwrap();
        let host = Host::default();
        host.dispatch("workspace.open", json!({ "path": project }))
            .unwrap();
        host.allow_setup_for_tests();

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
        assert!(answer["message"].as_str().unwrap().contains("discarded"));
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

    fn action_with_edit(
        uri: String,
        line: u64,
        start: u64,
        end: u64,
        text: &str,
    ) -> lsp::CodeActionView {
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
        assert_eq!(
            fs::read_to_string(&file).unwrap(),
            base,
            "a preview wrote the file"
        );
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
        assert!(preview["reason"]
            .as_str()
            .unwrap()
            .contains("file operation"));
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
            edits: (0..2001).map(|_| rename_edit(0, 0, 1, "X")).collect(),
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

    fn solution_file(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(
            &path,
            "Microsoft Visual Studio Solution File, Format Version 12.00\n",
        )
        .unwrap();
        path
    }

    #[test]
    fn workspace_open_resolves_relative_paths_against_base_dir() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let solution = solution_file(&root, "App.SLN");
        let host = Host::default();

        let relative = host
            .dispatch(
                "workspace.open",
                json!({ "path": "App.SLN", "baseDir": root }),
            )
            .unwrap();
        assert_eq!(relative["workspace"]["kind"], "solution");
        assert_eq!(relative["workspace"]["path"], solution.to_str().unwrap());

        let absolute = host
            .dispatch("workspace.open", json!({ "path": solution }))
            .unwrap();
        assert_eq!(absolute["workspace"]["path"], solution.to_str().unwrap());

        let empty = host
            .dispatch("workspace.open", json!({ "path": "" }))
            .unwrap_err();
        assert_eq!(empty.code, "path_not_allowed");
        let relative_base = host
            .dispatch(
                "workspace.open",
                json!({ "path": "App.SLN", "baseDir": "relative" }),
            )
            .unwrap_err();
        assert_eq!(relative_base.code, "path_not_allowed");
    }

    #[test]
    fn workspace_open_refuses_other_extensions_and_keeps_the_open_workspace() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let solution = solution_file(&root, "App.sln");
        let notes = root.join("notes.txt");
        fs::write(&notes, "").unwrap();
        let host = Host::default();
        host.dispatch("workspace.open", json!({ "path": solution }))
            .unwrap();

        let refused = host
            .dispatch("workspace.open", json!({ "path": notes }))
            .unwrap_err();
        assert_eq!(refused.code, "path_not_allowed");
        let state = host.dispatch("workspace.state", json!({})).unwrap();
        assert_eq!(state["workspace"]["path"], solution.to_str().unwrap());

        let missing = host
            .dispatch(
                "workspace.open",
                json!({ "path": "Missing.sln", "baseDir": root }),
            )
            .unwrap_err();
        assert_eq!(missing.code, "not_found");
    }

    #[test]
    fn workspace_open_directory_takes_one_slnx_and_lists_ambiguous_solutions() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let slnx = root.join("Modern.slnx");
        fs::write(&slnx, "<Solution />").unwrap();
        let host = Host::default();
        let opened = host
            .dispatch("workspace.open", json!({ "path": root }))
            .unwrap();
        assert_eq!(opened["workspace"]["kind"], "solution");
        assert_eq!(opened["workspace"]["path"], slnx.to_str().unwrap());

        solution_file(&root, "Legacy.sln");
        let ambiguous = host
            .dispatch("workspace.open", json!({ "path": root }))
            .unwrap_err();
        assert_eq!(ambiguous.code, "ambiguous_workspace");
        let candidates = ambiguous.details.unwrap()["candidates"]
            .as_array()
            .unwrap()
            .len();
        assert_eq!(candidates, 2);
    }

    #[cfg(unix)]
    #[test]
    fn workspace_open_follows_a_solution_symlink_to_its_target() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let solution = solution_file(&root, "App.sln");
        let link = root.join("Link.sln");
        std::os::unix::fs::symlink(&solution, &link).unwrap();
        let host = Host::default();

        let opened = host
            .dispatch("workspace.open", json!({ "path": link }))
            .unwrap();
        assert_eq!(opened["workspace"]["kind"], "solution");
        assert_eq!(opened["workspace"]["path"], solution.to_str().unwrap());
    }

    #[test]
    fn workspace_open_directory_with_one_csproj_opens_the_project() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let project = root.join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\" />").unwrap();
        let host = Host::default();

        let opened = host
            .dispatch("workspace.open", json!({ "path": root }))
            .unwrap();
        assert_eq!(opened["workspace"]["kind"], "project");
        assert_eq!(opened["workspace"]["path"], project.to_str().unwrap());
    }

    #[test]
    fn save_rechecks_the_destination_right_before_replacing_it() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let project = root.join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\" />").unwrap();
        let file = root.join("App.cs");
        fs::write(&file, "original\n").unwrap();
        let host = Host::default();
        host.dispatch("workspace.open", json!({ "path": project }))
            .unwrap();
        let read = host
            .dispatch("file.read", json!({ "path": "App.cs" }))
            .unwrap();

        // Simulate another writer saving between the first check and the rename.
        let external = file.clone();
        BEFORE_RENAME.with(|hook| {
            *hook.borrow_mut() = Some(Box::new(move || {
                fs::write(&external, "external save\n").unwrap();
            }));
        });

        let conflict = host
            .dispatch(
                "file.write",
                json!({
                    "path": "App.cs",
                    "content": "my buffer\n",
                    "expected_hash": read["hash"]
                }),
            )
            .expect_err("the recheck sees the external save");
        assert_eq!(conflict.code, "conflict");
        let details = conflict.details.unwrap();
        assert_eq!(details["currentContent"], "external save\n");
        assert_eq!(details["currentHash"], content_hash("external save\n"));
        assert_eq!(fs::read_to_string(&file).unwrap(), "external save\n");
        assert!(
            fs::read_dir(&root).unwrap().all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".openridr-save-")),
            "the temp file is removed on conflict"
        );
    }

    #[test]
    fn a_batch_restore_is_refused_after_trust_is_revoked() {
        let host = Host::default();
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>").unwrap();
        host.dispatch("workspace.open", json!({ "path": project }))
            .unwrap();
        host.allow_setup_for_tests();
        host.dispatch("trust.grant", json!({})).unwrap();
        let token = host.context_token().unwrap();
        {
            let inner = host.inner.lock().unwrap();
            assert!(restore_allowed(&inner, token));
        }
        host.dispatch("trust.revoke", json!({})).unwrap();
        let inner = host.inner.lock().unwrap();
        assert!(!restore_allowed(&inner, token));
    }

    #[test]
    fn a_batch_restore_is_not_called_after_revocation() {
        let host = Host::default();
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>").unwrap();
        host.dispatch("workspace.open", json!({ "path": project.clone() }))
            .unwrap();
        host.allow_setup_for_tests();
        host.dispatch("trust.grant", json!({})).unwrap();
        let token = host.context_token().unwrap();
        host.dispatch("trust.revoke", json!({})).unwrap();
        let plan = nuget::Plan {
            id: "plan".to_string(),
            workspace_id: token.workspace_id,
            context_id: token.context_id,
            root: temp.path().to_path_buf(),
            project: project.clone(),
            project_display: "App.csproj".to_string(),
            package: "xunit".to_string(),
            from_version: Some("2.9.2".to_string()),
            version: "2.9.3".to_string(),
            target_framework: Some("net8.0".to_string()),
            muxer: std::path::PathBuf::from("/bin/false"),
            sdk_root: temp.path().to_path_buf(),
            install: false,
            probe: false,
        };
        let called = std::cell::Cell::new(false);
        let restores = host.batch_restores(&[plan], &["App.csproj".to_string()], token, |_| {
            called.set(true);
            nuget::RestoreRun {
                state: "ok",
                message: None,
                command: String::new(),
                exit: Some(0),
                log: String::new(),
            }
        });
        assert!(!called.get());
        assert_eq!(restores[0]["state"], "failed");
    }
}
