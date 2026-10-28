//! Minimal Debug Adapter Protocol client for netcoredbg (`--interpreter=vscode`).
//!
//! One adapter per session, spoken over stdio with the same `Content-Length` framing as LSP
//! (lengths are UTF-8 bytes). Every message the host writes carries a monotonic `seq`; every
//! request id is answered through a pending channel, so a response can only reach the request
//! it belongs to. Events, reverse requests (`runInTerminal`, answered by spawning the exact
//! argv with cwd/env and returning the observed pid, never through a shell), output, and the
//! debuggee exit are kept as observed. A stop generation invalidates the previous pause, and a
//! sealed session drops every late message instead of resurrecting frames, locals or state.

use crate::error::{HostError, HostResult};
use crate::logbuf::{self, LogBuffer};
use crate::lsp::{read_frame, write_message, Frame};
use crate::process::{Registry, Termination};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{BufReader, Write};
use std::path::PathBuf;
use std::process::{ChildStderr, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const LAUNCH_TIMEOUT: Duration = Duration::from_secs(20);
const INITIALIZED_TIMEOUT: Duration = Duration::from_secs(20);
const DISCONNECT_WAIT: Duration = Duration::from_secs(3);

type Writer = Arc<Mutex<Box<dyn Write + Send>>>;

#[derive(Debug, Clone)]
pub struct Launch {
    pub session: u64,
    pub project_name: String,
    pub project_path: String,
    pub build_id: String,
    pub assembly: String,
    pub configuration: String,
    pub target_framework: Option<String>,
    pub cwd: PathBuf,
    pub root: PathBuf,
    pub sdk_root: PathBuf,
    pub adapter_binary: String,
    pub adapter_version: String,
    pub adapter_platform: String,
    pub workspace_id: u64,
    pub context_id: u64,
    /// Breakpoints set before the session started, kept by file.
    pub breakpoints: BTreeMap<String, Vec<i64>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakpointView {
    pub path: String,
    pub line: i64,
    pub verified: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadView {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameView {
    pub id: i64,
    pub name: String,
    pub line: i64,
    pub column: i64,
    pub path: Option<String>,
    pub source_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VariableView {
    pub name: String,
    pub value: String,
    #[serde(rename = "type")]
    pub type_name: Option<String>,
    pub variables_reference: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeView {
    pub name: String,
    pub expensive: bool,
    pub variables: Vec<VariableView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalsView {
    pub frame_id: i64,
    pub scopes: Vec<ScopeView>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalView {
    pub process_id: u64,
    pub pid: u32,
    pub command: Vec<String>,
    pub state: String,
    pub exit: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoppedView {
    pub reason: String,
    pub thread_id: Option<i64>,
    pub all_threads_stopped: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdapterView {
    pub binary: String,
    pub version: String,
    pub platform: String,
    pub argv: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DapView {
    pub state: &'static str,
    pub phase: String,
    pub session: u64,
    pub project: Option<ProjectView>,
    pub build_id: Option<String>,
    pub assembly: Option<String>,
    pub configuration: String,
    pub target_framework: Option<String>,
    pub adapter: AdapterView,
    pub command: Vec<String>,
    pub cwd: String,
    pub process_id: Option<u64>,
    pub pid: Option<u32>,
    pub debuggee_pid: Option<u32>,
    pub exit: Option<i32>,
    pub exit_known: bool,
    pub adapter_exit: Option<i32>,
    pub termination: Option<Termination>,
    pub threads: Vec<ThreadView>,
    pub frames: Vec<FrameView>,
    pub selected_frame_id: Option<i64>,
    pub locals: Option<LocalsView>,
    pub stopped: Option<StoppedView>,
    pub breakpoints: Vec<BreakpointView>,
    pub terminals: Vec<TerminalView>,
    pub log: String,
    pub log_truncated: bool,
    pub output: String,
    pub output_truncated: bool,
    pub revision: u64,
    pub workspace_id: Option<u64>,
    pub context_id: Option<u64>,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub sealed: bool,
    pub stale_reason: Option<String>,
}

struct Data {
    session: u64,
    state: &'static str,
    phase: String,
    project: Option<ProjectView>,
    build_id: String,
    assembly: String,
    configuration: String,
    target_framework: Option<String>,
    adapter_binary: String,
    adapter_version: String,
    adapter_platform: String,
    command: Vec<String>,
    cwd: String,
    root: PathBuf,
    workspace_id: u64,
    context_id: u64,
    process_id: Option<u64>,
    pid: Option<u32>,
    debuggee_pid: Option<u32>,
    exit: Option<i32>,
    adapter_exit: Option<i32>,
    termination: Option<Termination>,
    cancel_requested: bool,
    sealed: bool,
    stale_reason: Option<String>,
    started_at: Option<u64>,
    finished_at: Option<u64>,
    log: LogBuffer,
    output: LogBuffer,
    writer: Option<Writer>,
    pending: BTreeMap<i64, Sender<Value>>,
    next_id: i64,
    seq: i64,
    initialized_event: bool,
    configuration_done: bool,
    cleanup_started: bool,
    stop_seq: u64,
    stopped: Option<StoppedView>,
    threads: Vec<ThreadView>,
    frames: Vec<FrameView>,
    selected_frame_id: Option<i64>,
    locals: Option<LocalsView>,
    breakpoints: BTreeMap<String, Vec<BreakpointView>>,
    terminals: Vec<TerminalView>,
    terminal_ids: Vec<u64>,
}

impl Data {
    fn finish(&mut self, state: &'static str, phase: &str) {
        self.state = state;
        self.phase = phase.to_string();
        self.finished_at = Some(now_ms());
        self.stopped = None;
        self.frames.clear();
        self.threads.clear();
        self.locals = None;
        self.selected_frame_id = None;
        let line = format!("[dap] session {state} · {}", self.phase);
        self.log.push(line);
    }
}

pub struct Dap {
    data: Mutex<Data>,
    registry: Option<Arc<Mutex<Registry>>>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn adapter_view(binary: &str, version: &str, platform: &str) -> AdapterView {
    AdapterView {
        binary: binary.to_string(),
        version: version.to_string(),
        platform: platform.to_string(),
        argv: vec![binary.to_string(), "--interpreter=vscode".to_string()],
    }
}

impl Dap {
    #[cfg(test)]
    fn with_registry(registry: Arc<Mutex<Registry>>) -> Arc<Self> {
        let idle = Self::idle();
        Arc::new(Self {
            data: idle.data,
            registry: Some(registry),
        })
    }

    /// No session. Breakpoints can still be prepared; nothing is spawned.
    pub fn idle() -> Self {
        Self {
            data: Mutex::new(Data {
                session: 0,
                state: "idle",
                phase: "No debug session.".to_string(),
                project: None,
                build_id: String::new(),
                assembly: String::new(),
                configuration: String::new(),
                target_framework: None,
                adapter_binary: String::new(),
                adapter_version: String::new(),
                adapter_platform: String::new(),
                command: Vec::new(),
                cwd: String::new(),
                root: PathBuf::new(),
                workspace_id: 0,
                context_id: 0,
                process_id: None,
                pid: None,
                debuggee_pid: None,
                exit: None,
                adapter_exit: None,
                termination: None,
                cancel_requested: false,
                sealed: false,
                stale_reason: None,
                started_at: None,
                finished_at: None,
                log: LogBuffer::default(),
                output: LogBuffer::default(),
                writer: None,
                pending: BTreeMap::new(),
                next_id: 0,
                seq: 0,
                initialized_event: false,
                configuration_done: false,
                cleanup_started: false,
                stop_seq: 0,
                stopped: None,
                threads: Vec::new(),
                frames: Vec::new(),
                selected_frame_id: None,
                locals: None,
                breakpoints: BTreeMap::new(),
                terminals: Vec::new(),
                terminal_ids: Vec::new(),
            }),
            registry: None,
        }
    }

    /// Starts the adapter and the launch negotiation. The session is visible as `preparing`
    /// immediately; `running`/`paused`/`failed` come from the adapter's real messages.
    pub fn begin(
        launch: Launch,
        mut command: Command,
        registry: Arc<Mutex<Registry>>,
    ) -> HostResult<Arc<Self>> {
        command
            .current_dir(&launch.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut registry_lock = registry.lock().unwrap();
        let piped = registry_lock
            .spawn_piped(
                &format!("debug:{}", launch.project_name),
                launch.workspace_id,
                command,
            )
            .map_err(|error| {
                HostError::new(
                    "operation_failed",
                    format!("Could not start netcoredbg: {error}"),
                )
            })?;
        drop(registry_lock);

        let breakpoints = launch
            .breakpoints
            .iter()
            .map(|(path, lines)| {
                (
                    path.clone(),
                    lines
                        .iter()
                        .map(|line| BreakpointView {
                            path: path.clone(),
                            line: *line,
                            verified: false,
                            message: None,
                        })
                        .collect::<Vec<_>>(),
                )
            })
            .collect();

        let command_line = adapter_view(
            &launch.adapter_binary,
            &launch.adapter_version,
            &launch.adapter_platform,
        )
        .argv;
        let mut log = LogBuffer::default();
        log.push(format!("DAP session {} · {}", launch.session, command_line.join(" ")));
        log.push(format!("cwd {}", launch.cwd.to_string_lossy()));
        log.push(format!("program {}", launch.assembly));
        log.push(format!(
            "build {} · {} / {} · adapter {} · {}",
            launch.build_id,
            launch.configuration,
            launch.target_framework.clone().unwrap_or_else(|| "no TFM".to_string()),
            launch.adapter_version,
            launch.adapter_platform
        ));

        let dap = Arc::new(Self {
            data: Mutex::new(Data {
                session: launch.session,
                state: "preparing",
                phase: "Starting netcoredbg and negotiating DAP.".to_string(),
                project: Some(ProjectView {
                    name: launch.project_name.clone(),
                    path: launch.project_path.clone(),
                }),
                build_id: launch.build_id.clone(),
                assembly: launch.assembly.clone(),
                configuration: launch.configuration.clone(),
                target_framework: launch.target_framework.clone(),
                adapter_binary: launch.adapter_binary.clone(),
                adapter_version: launch.adapter_version.clone(),
                adapter_platform: launch.adapter_platform.clone(),
                command: command_line,
                cwd: launch.cwd.to_string_lossy().to_string(),
                root: launch.root.clone(),
                workspace_id: launch.workspace_id,
                context_id: launch.context_id,
                process_id: Some(piped.id),
                pid: Some(piped.pid),
                debuggee_pid: None,
                exit: None,
                adapter_exit: None,
                termination: None,
                cancel_requested: false,
                sealed: false,
                stale_reason: None,
                started_at: Some(now_ms()),
                finished_at: None,
                log,
                output: LogBuffer::default(),
                writer: piped
                    .stdin
                    .map(|stdin| Arc::new(Mutex::new(Box::new(stdin) as Box<dyn Write + Send>))),
                pending: BTreeMap::new(),
                next_id: 0,
                seq: 0,
                initialized_event: false,
                configuration_done: false,
                cleanup_started: false,
                stop_seq: 1,
                stopped: None,
                threads: Vec::new(),
                frames: Vec::new(),
                selected_frame_id: None,
                locals: None,
                breakpoints,
                terminals: Vec::new(),
                terminal_ids: Vec::new(),
            }),
            registry: Some(registry),
        });

        if let Some(stdout) = piped.stdout {
            spawn_reader(Arc::clone(&dap), stdout);
        }
        if let Some(stderr) = piped.stderr {
            spawn_stderr(Arc::clone(&dap), stderr);
        }
        let worker = Arc::clone(&dap);
        std::thread::spawn(move || worker.negotiate(launch));
        Ok(dap)
    }

    pub fn is_active(&self) -> bool {
        matches!(
            self.data.lock().unwrap().state,
            "preparing" | "running" | "paused"
        )
    }

    pub fn process_id(&self) -> Option<u64> {
        self.data.lock().unwrap().process_id
    }

    pub fn state(&self) -> &'static str {
        self.data.lock().unwrap().state
    }

    pub fn log_line(&self, line: impl Into<String>) {
        self.data.lock().unwrap().log.push(line.into());
    }

    pub fn breakpoints(&self) -> BTreeMap<String, Vec<i64>> {
        self.data
            .lock()
            .unwrap()
            .breakpoints
            .iter()
            .map(|(path, items)| {
                (
                    path.clone(),
                    items.iter().map(|item| item.line).collect::<Vec<_>>(),
                )
            })
            .collect()
    }

    pub fn view(&self) -> DapView {
        let data = self.data.lock().unwrap();
        let (log, log_truncated) = data.log.text();
        let (output, output_truncated) = data.output.text();
        DapView {
            state: data.state,
            phase: data.phase.clone(),
            session: data.session,
            project: data.project.clone(),
            build_id: (data.session > 0).then(|| data.build_id.clone()),
            assembly: (data.session > 0).then(|| data.assembly.clone()),
            configuration: data.configuration.clone(),
            target_framework: data.target_framework.clone(),
            adapter: adapter_view(
                &data.adapter_binary,
                &data.adapter_version,
                &data.adapter_platform,
            ),
            command: data.command.clone(),
            cwd: data.cwd.clone(),
            process_id: data.process_id,
            pid: data.pid,
            debuggee_pid: data.debuggee_pid,
            exit: data.exit,
            exit_known: data.exit.is_some(),
            adapter_exit: data.adapter_exit,
            termination: data.termination.clone(),
            threads: data.threads.clone(),
            frames: data.frames.clone(),
            selected_frame_id: data.selected_frame_id,
            locals: data.locals.clone(),
            stopped: data.stopped.clone(),
            breakpoints: data
                .breakpoints
                .values()
                .flat_map(|items| items.iter().cloned())
                .collect(),
            terminals: data.terminals.clone(),
            log,
            log_truncated,
            output,
            output_truncated,
            revision: data.session,
            workspace_id: (data.session > 0).then_some(data.workspace_id),
            context_id: (data.session > 0).then_some(data.context_id),
            started_at: data.started_at,
            finished_at: data.finished_at,
            sealed: data.sealed,
            stale_reason: data.stale_reason.clone(),
        }
    }

    fn note(&self, line: impl Into<String>) {
        let mut data = self.data.lock().unwrap();
        data.log.push(line.into());
    }

    /// Stores the requested lines for one source. If a session is live the adapter is asked
    /// now; its verification state is the only one shown. Without symbols the adapter keeps
    /// the breakpoint pending and so does the host.
    pub fn set_breakpoints(&self, path: &str, lines: &[i64]) {
        let mut unique: Vec<i64> = lines.to_vec();
        unique.sort_unstable();
        unique.dedup();
        let mut data = self.data.lock().unwrap();
        data.breakpoints.insert(
            path.to_string(),
            unique
                .iter()
                .map(|line| BreakpointView {
                    path: path.to_string(),
                    line: *line,
                    verified: false,
                    message: None,
                })
                .collect(),
        );
        let live = matches!(data.state, "preparing" | "running" | "paused");
        drop(data);
        if live {
            self.send_breakpoints(path, &unique);
        }
    }

    fn send_breakpoints(&self, path: &str, lines: &[i64]) {
        let source = {
            let data = self.data.lock().unwrap();
            let candidate = PathBuf::from(path);
            if candidate.is_absolute() {
                candidate
            } else {
                data.root.join(candidate)
            }
        };
        let params = json!({
            "source": { "path": source.to_string_lossy() },
            "breakpoints": lines.iter().map(|line| json!({ "line": line })).collect::<Vec<_>>(),
            "lines": lines,
            "sourceModified": false,
        });
        let Ok(response) = self.request("setBreakpoints", params, REQUEST_TIMEOUT) else {
            return;
        };
        let Some(items) = response.get("breakpoints").and_then(Value::as_array) else {
            return;
        };
        let mut data = self.data.lock().unwrap();
        let Some(entry) = data.breakpoints.get_mut(path) else {
            return;
        };
        for (index, item) in items.iter().enumerate() {
            let line = item.get("line").and_then(Value::as_i64);
            let verified = item
                .get("verified")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let message = item
                .get("message")
                .and_then(Value::as_str)
                .map(str::to_string);
            let position = entry
                .iter()
                .position(|entry| Some(entry.line) == line)
                .or(Some(index))
                .filter(|index| *index < entry.len());
            if let Some(position) = position {
                let entry = &mut entry[position];
                entry.verified = verified;
                entry.message = message;
                if let Some(line) = line {
                    entry.line = line;
                }
            }
        }
    }

    fn negotiate(self: Arc<Self>, launch: Launch) {
        let initialize = self.request(
            "initialize",
            json!({
                "clientID": "openridr",
                "clientName": "OpenRidr",
                "adapterID": "coreclr",
                "pathFormat": "path",
                "linesStartAt1": true,
                "columnsStartAt1": true,
                "supportsRunInTerminalRequest": true,
                "supportsVariableType": true,
                "supportsTerminateRequest": true,
            }),
            REQUEST_TIMEOUT,
        );
        if let Err(error) = initialize {
            self.fail(format!("DAP initialize failed: {error}"));
            return;
        }
        let launch_params = json!({
            "program": launch.assembly,
            "cwd": launch.cwd.to_string_lossy(),
            "args": [],
            "env": {
                "DOTNET_ROOT": launch.sdk_root.to_string_lossy(),
                "DOTNET_MULTILEVEL_LOOKUP": "0",
                "DOTNET_CLI_UI_LANGUAGE": "en",
            },
            "stopAtEntry": false,
            "console": "internalConsole",
        });
        if let Err(error) = self.request("launch", launch_params, LAUNCH_TIMEOUT) {
            self.fail(format!("DAP launch failed: {error}"));
            return;
        }
        if !self.wait_initialized() {
            self.fail(format!(
                "netcoredbg did not send the initialized event within {} s; no breakpoints were set.",
                INITIALIZED_TIMEOUT.as_secs()
            ));
            return;
        }
        let files = self.breakpoints();
        for (path, lines) in files {
            if !lines.is_empty() {
                self.send_breakpoints(&path, &lines);
            }
        }
        if let Err(error) = self.request("configurationDone", json!({}), REQUEST_TIMEOUT) {
            self.fail(format!("DAP configurationDone failed: {error}"));
            return;
        }
        let mut data = self.data.lock().unwrap();
        data.configuration_done = true;
        if !data.sealed && data.state == "preparing" {
            data.state = "running";
            data.phase = "Running; breakpoints are active.".to_string();
        }
    }

    fn wait_initialized(&self) -> bool {
        let deadline = Instant::now() + INITIALIZED_TIMEOUT;
        while Instant::now() < deadline {
            if self.data.lock().unwrap().initialized_event {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    fn current(&self, session: u64, stop_seq: u64) -> bool {
        let data = self.data.lock().unwrap();
        data.session == session
            && data.stop_seq == stop_seq
            && !data.sealed
            && matches!(data.state, "paused" | "preparing")
    }

    fn request(
        &self,
        command: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        let Some((id, receiver)) = self.post_request(command, params) else {
            return Err("the debug adapter is not accepting requests".to_string());
        };
        match receiver.recv_timeout(timeout) {
            Ok(response) => {
                if response.get("success").and_then(Value::as_bool) == Some(false) {
                    let message = response
                        .get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("the adapter reported an error");
                    self.note(format!("[dap] {command} id {id} failed: {message}"));
                    Err(message.to_string())
                } else {
                    self.note(format!("[dap] {command} id {id} answered"));
                    Ok(response.get("body").cloned().unwrap_or(Value::Null))
                }
            }
            Err(RecvTimeoutError::Timeout) => {
                self.data.lock().unwrap().pending.remove(&id);
                self.note(format!(
                    "[dap] {command} id {id} timed out after {} ms",
                    timeout.as_millis()
                ));
                Err(format!("{command} timed out after {} ms", timeout.as_millis()))
            }
            Err(RecvTimeoutError::Disconnected) => {
                Err("the debug adapter stopped before answering".to_string())
            }
        }
    }

    fn post_request(&self, command: &str, params: Value) -> Option<(i64, Receiver<Value>)> {
        let mut data = self.data.lock().unwrap();
        if data.sealed && command != "disconnect" {
            return None;
        }
        data.next_id += 1;
        let id = data.next_id;
        data.seq += 1;
        let seq = data.seq;
        let writer = data.writer.clone();
        let message = json!({
            "seq": seq,
            "type": "request",
            "command": command,
            "arguments": params,
        });
        let Some(writer) = writer else {
            data.log.push(format!("[dap] -> {command} id {id} skipped: no stdin"));
            return None;
        };
        data.log.push(format!("[dap] -> {command} id {id}"));
        let (sender, receiver) = mpsc::channel();
        data.pending.insert(id, sender);
        drop(data);
        let written = writer.lock().ok().and_then(|mut writer| {
            write_message(&mut *writer, &message).ok()
        });
        if written.is_none() {
            self.data.lock().unwrap().pending.remove(&id);
            self.note(format!("[dap] -> {command} id {id} write failed"));
            return None;
        }
        Some((id, receiver))
    }

    fn send_response(&self, id: Value, result: Value) {
        let mut data = self.data.lock().unwrap();
        data.seq += 1;
        let seq = data.seq;
        let message = json!({
            "seq": seq,
            "type": "response",
            "request_seq": id,
            "success": true,
            "body": result,
        });
        let writer = data.writer.clone();
        drop(data);
        if let Some(writer) = writer {
            if let Ok(mut writer) = writer.lock() {
                let _ = write_message(&mut *writer, &message);
            }
        }
    }

    fn send_failure(&self, id: Value, message: &str) {
        let mut data = self.data.lock().unwrap();
        data.seq += 1;
        let seq = data.seq;
        let request = json!({
            "seq": seq,
            "type": "response",
            "request_seq": id,
            "success": false,
            "message": message,
        });
        let writer = data.writer.clone();
        drop(data);
        if let Some(writer) = writer {
            if let Ok(mut writer) = writer.lock() {
                let _ = write_message(&mut *writer, &request);
            }
        }
    }

    fn handle_message(self: &Arc<Self>, value: Value) {
        match value.get("type").and_then(Value::as_str) {
            Some("event") => {
                let Some(event) = value.get("event").and_then(Value::as_str) else {
                    return;
                };
                let params = value.get("body").cloned().unwrap_or(Value::Null);
                self.handle_event(event, params);
                return;
            }
            Some("request") => {
                let Some(command) = value.get("command").and_then(Value::as_str) else {
                    return;
                };
                let params = value.get("arguments").cloned().unwrap_or(Value::Null);
                let Some(id) = value.get("seq").cloned() else {
                    return;
                };
                if command == "runInTerminal" {
                    match self.run_in_terminal(&params) {
                        Ok(body) => {
                            self.note("[dap] <- runInTerminal · spawned the exact argv");
                            self.send_response(id, body);
                        }
                        Err(error) => {
                            self.note(format!("[dap] <- runInTerminal refused: {error}"));
                            self.send_failure(id, &error);
                        }
                    }
                    return;
                }
                self.note(format!("[dap] <- reverse request {command} · answered null"));
                self.send_response(id, Value::Null);
                return;
            }
            _ => {}
        }
        let Some(id) = value.get("request_seq").and_then(Value::as_i64) else {
            return;
        };
        let sender = self.data.lock().unwrap().pending.remove(&id);
        if let Some(sender) = sender {
            let _ = sender.send(value);
        }
    }

    fn handle_event(self: &Arc<Self>, event: &str, params: Value) {
        match event {
            "initialized" => {
                let mut data = self.data.lock().unwrap();
                data.initialized_event = true;
                data.log.push("[dap] event initialized".to_string());
            }
            "stopped" => {
                let reason = params
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let thread_id = params.get("threadId").and_then(Value::as_i64);
                let all = params
                    .get("allThreadsStopped")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let (session, stop_seq) = {
                    let mut data = self.data.lock().unwrap();
                    if data.sealed {
                        data.log
                            .push("[dap] late stopped event ignored after seal".to_string());
                        return;
                    }
                    data.stop_seq += 1;
                    data.state = "paused";
                    data.phase = format!(
                        "Paused · {reason}{}",
                        thread_id
                            .map(|id| format!(" · thread {id}"))
                            .unwrap_or_default()
                    );
                    data.stopped = Some(StoppedView {
                        reason: reason.clone(),
                        thread_id,
                        all_threads_stopped: all,
                    });
                    data.threads.clear();
                    data.frames.clear();
                    data.locals = None;
                    data.selected_frame_id = None;
                    data.log.push(format!(
                        "[dap] event stopped · reason {reason} · thread {} · all threads {all}",
                        thread_id
                            .map(|id| id.to_string())
                            .unwrap_or_else(|| "unknown".to_string())
                    ));
                    (data.session, data.stop_seq)
                };
                let dap = Arc::clone(self);
                std::thread::spawn(move || dap.load_pause(session, stop_seq, thread_id));
            }
            "continued" => {
                let mut data = self.data.lock().unwrap();
                if data.sealed {
                    return;
                }
                data.stop_seq += 1;
                data.state = "running";
                data.phase = "Running; breakpoints are active.".to_string();
                data.stopped = None;
                data.frames.clear();
                data.locals = None;
                data.selected_frame_id = None;
                data.log.push("[dap] event continued".to_string());
            }
            "output" => {
                let category = params
                    .get("category")
                    .and_then(Value::as_str)
                    .unwrap_or("stdout");
                let text = params
                    .get("output")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let mut data = self.data.lock().unwrap();
                if data.sealed {
                    return;
                }
                for line in text.lines() {
                    let shown = match category {
                        "stdout" => line.to_string(),
                        "stderr" => format!("[stderr] {line}"),
                        other => format!("[{other}] {line}"),
                    };
                    data.output.push(shown);
                }
            }
            "exited" => {
                let code = params.get("exitCode").and_then(Value::as_i64);
                let mut data = self.data.lock().unwrap();
                if data.sealed {
                    return;
                }
                data.exit = code.map(|code| code as i32);
                data.log.push(format!(
                    "[dap] event exited · {}",
                    code.map(|code| format!("exit {code}"))
                        .unwrap_or_else(|| "exit unknown".to_string())
                ));
            }
            "terminated" => {
                let mut data = self.data.lock().unwrap();
                if data.sealed {
                    return;
                }
                data.finish("terminated", "The debug session terminated.");
                drop(data);
                self.cleanup_adapter();
            }
            "breakpoint" => {
                let Some(breakpoint) = params.get("breakpoint") else {
                    return;
                };
                let verified = breakpoint
                    .get("verified")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let line = breakpoint.get("line").and_then(Value::as_i64);
                let message = breakpoint
                    .get("message")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let mut data = self.data.lock().unwrap();
                let path = breakpoint
                    .get("source")
                    .and_then(|source| source.get("path"))
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let normalized = path.as_deref().map(|path| path.replace('\\', "/"));
                let mut changed = Vec::new();
                for (entry_path, entries) in data.breakpoints.iter_mut() {
                    let key = entry_path.replace('\\', "/");
                    if normalized
                        .as_deref()
                        .is_some_and(|path| path != key && !path.ends_with(&key))
                    {
                        continue;
                    }
                    if let Some(entry) = entries
                        .iter_mut()
                        .find(|entry| Some(entry.line) == line)
                    {
                        entry.verified = verified;
                        entry.message = message.clone();
                        changed.push(format!("{} line {}", entry_path, entry.line));
                    }
                }
                for line in changed {
                    data.log.push(format!(
                        "[dap] event breakpoint · {line} · verified {verified}"
                    ));
                }
            }
            "process" => {
                let system_pid = params
                    .get("systemProcessId")
                    .and_then(Value::as_u64)
                    .map(|pid| pid as u32);
                let mut data = self.data.lock().unwrap();
                if system_pid.is_some() {
                    data.debuggee_pid = system_pid;
                }
                let name = params
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("process");
                data.log.push(format!(
                    "[dap] event process · {name} · system pid {}",
                    system_pid
                        .map(|pid| pid.to_string())
                        .unwrap_or_else(|| "unknown".to_string())
                ));
            }
            "thread" | "module" | "capabilities" | "loadedSource" => {
                let reason = params
                    .get("reason")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                self.note(format!("[dap] event {event} {reason}"));
            }
            other => {
                self.note(format!("[dap] event {other} (kept as observed)"));
            }
        }
    }

    fn load_pause(&self, session: u64, stop_seq: u64, requested_thread: Option<i64>) {
        if !self.current(session, stop_seq) {
            return;
        }
        let threads = match self.request("threads", json!({}), REQUEST_TIMEOUT) {
            Ok(body) => parse_threads(&body),
            Err(error) => {
                self.note(format!("[dap] threads failed: {error}"));
                Vec::new()
            }
        };
        let thread_id = requested_thread
            .or_else(|| threads.first().map(|thread| thread.id))
            .unwrap_or(1);
        let stack = self.request(
            "stackTrace",
            json!({ "threadId": thread_id, "startFrame": 0, "levels": 20 }),
            REQUEST_TIMEOUT,
        );
        let frames = match stack {
            Ok(body) => parse_frames(&body),
            Err(error) => {
                self.note(format!("[dap] stackTrace failed: {error}"));
                Vec::new()
            }
        };
        {
            let mut data = self.data.lock().unwrap();
            if data.session != session || data.stop_seq != stop_seq || data.sealed {
                data.log
                    .push("[dap] late thread/frame response discarded (old stop generation)".to_string());
                return;
            }
            data.threads = threads;
            data.frames = frames;
            data.selected_frame_id = data.frames.first().map(|frame| frame.id);
        }
        let top_frame = self.data.lock().unwrap().frames.first().cloned();
        if let Some(frame) = top_frame {
            let _ = self.load_locals_for(frame.id, session, stop_seq);
        }
        let mut data = self.data.lock().unwrap();
        if data.session == session && data.stop_seq == stop_seq && !data.sealed {
            data.phase = format!(
                "Paused · {} · {} frame(s) · {} thread(s)",
                data.stopped
                    .as_ref()
                    .map(|stopped| stopped.reason.clone())
                    .unwrap_or_else(|| "unknown".to_string()),
                data.frames.len(),
                data.threads.len()
            );
        }
    }

    /// Fetches scopes/variables for one frame of the current pause. A frame from an older stop
    /// generation is refused instead of returning locals that no longer match the program.
    pub fn load_locals(&self, frame_id: i64) -> HostResult<()> {
        let (session, stop_seq) = {
            let data = self.data.lock().unwrap();
            if data.sealed {
                return Err(HostError::new(
                    "invalid_state",
                    "The debug session is sealed; frame variables are not current.",
                ));
            }
            if data.state != "paused" {
                return Err(HostError::new(
                    "invalid_state",
                    "Locals are available only while the debuggee is paused.",
                ));
            }
            if !data.frames.iter().any(|frame| frame.id == frame_id) {
                return Err(HostError::new(
                    "invalid_state",
                    "That frame does not belong to the current pause.",
                ));
            }
            (data.session, data.stop_seq)
        };
        {
            let mut data = self.data.lock().unwrap();
            if data.session == session && data.stop_seq == stop_seq {
                // The previous frame's locals stop being current before the request is sent.
                data.selected_frame_id = Some(frame_id);
                data.locals = None;
            }
        }
        self.load_locals_for(frame_id, session, stop_seq)?;
        Ok(())
    }

    fn load_locals_for(
        &self,
        frame_id: i64,
        session: u64,
        stop_seq: u64,
    ) -> HostResult<()> {
        let scopes = self
            .request("scopes", json!({ "frameId": frame_id }), REQUEST_TIMEOUT)
            .map_err(|error| HostError::new("operation_failed", format!("scopes failed: {error}")))?;
        let entries = scopes
            .get("scopes")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut views = Vec::new();
        for scope in entries {
            let name = scope
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("Scope")
                .to_string();
            let reference = scope
                .get("variablesReference")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let expensive = scope
                .get("expensive")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let mut variables = Vec::new();
            if reference > 0 {
                match self.request(
                    "variables",
                    json!({ "variablesReference": reference }),
                    REQUEST_TIMEOUT,
                ) {
                    Ok(body) => {
                        for variable in body
                            .get("variables")
                            .and_then(Value::as_array)
                            .cloned()
                            .unwrap_or_default()
                        {
                            variables.push(VariableView {
                                name: variable
                                    .get("name")
                                    .and_then(Value::as_str)
                                    .unwrap_or("?")
                                    .to_string(),
                                value: variable
                                    .get("value")
                                    .and_then(Value::as_str)
                                    .unwrap_or("")
                                    .to_string(),
                                type_name: variable
                                    .get("type")
                                    .and_then(Value::as_str)
                                    .map(str::to_string),
                                variables_reference: variable
                                    .get("variablesReference")
                                    .and_then(Value::as_i64)
                                    .unwrap_or(0),
                            });
                        }
                    }
                    Err(error) => {
                        self.note(format!("[dap] variables failed for {name}: {error}"));
                    }
                }
            }
            views.push(ScopeView {
                name,
                expensive,
                variables,
            });
        }
        let mut data = self.data.lock().unwrap();
        if data.session != session || data.stop_seq != stop_seq || data.sealed {
            data.log
                .push("[dap] late scopes/variables discarded (old stop generation)".to_string());
            return Err(HostError::new(
                "stale_context",
                "A scope response from an older pause was discarded.",
            ));
        }
        data.locals = Some(LocalsView {
            frame_id,
            scopes: views,
        });
        Ok(())
    }

    pub fn continue_(&self) -> HostResult<()> {
        self.resume("continue")
    }

    pub fn step_over(&self) -> HostResult<()> {
        self.resume("next")
    }

    pub fn step_into(&self) -> HostResult<()> {
        self.resume("stepIn")
    }

    fn resume(&self, command: &str) -> HostResult<()> {
        let thread_id = {
            let data = self.data.lock().unwrap();
            if data.state != "paused" || data.sealed {
                return Err(HostError::new(
                    "invalid_state",
                    "Continue/step is available only while the debuggee is paused.",
                ));
            }
            data.stopped
                .as_ref()
                .and_then(|stopped| stopped.thread_id)
                .or_else(|| data.threads.first().map(|thread| thread.id))
                .or_else(|| data.frames.first().map(|_| 1))
                .unwrap_or(1)
        };
        let sent_seq = {
            let mut data = self.data.lock().unwrap();
            if data.sealed || data.state != "paused" {
                return Err(HostError::new(
                    "invalid_state",
                    "The debuggee is no longer paused.",
                ));
            }
            // The previous pause stops being current before the request is sent, so even a
            // response that arrives late cannot be rendered as the new pause.
            data.stop_seq += 1;
            data.frames.clear();
            data.locals = None;
            data.selected_frame_id = None;
            data.stopped = None;
            data.phase = format!("Requested {command}; waiting for the adapter.");
            data.stop_seq
        };
        let result = self.request(command, json!({ "threadId": thread_id }), REQUEST_TIMEOUT);
        let mut data = self.data.lock().unwrap();
        if data.sealed {
            return Ok(());
        }
        if data.stop_seq != sent_seq {
            // A stopped event already started a newer pause; the resume answer belongs to the
            // pause that was left behind and must not overwrite it.
            return Ok(());
        }
        match result {
            Ok(_) => {
                data.state = "running";
                data.phase = "Running; breakpoints are active.".to_string();
                Ok(())
            }
            Err(error) => {
                data.state = "paused";
                data.phase = format!("{command} failed: {error}");
                data.log.push(format!("[dap] {command} failed: {error}"));
                Err(HostError::new(
                    "operation_failed",
                    format!("The adapter refused {command}: {error}"),
                ))
            }
        }
    }

    fn fail(&self, message: String) {
        let mut data = self.data.lock().unwrap();
        if data.sealed {
            return;
        }
        data.finish("failed", &message);
        data.log.push(format!("[error] {message}"));
    }

    /// A finished debuggee does not prove the adapter process is gone. Wait briefly for a
    /// natural exit, then disconnect and (if needed) terminate the group, recording what the
    /// registry observed. Runs on its own thread because the reader thread must keep reading.
    fn cleanup_adapter(self: &Arc<Self>) {
        {
            let mut data = self.data.lock().unwrap();
            if data.cleanup_started {
                return;
            }
            data.cleanup_started = true;
        }
        let Some(registry) = self.registry.clone() else {
            return;
        };
        let Some(id) = self.process_id() else {
            return;
        };
        let dap = Arc::clone(self);
        std::thread::spawn(move || {
            let exited = || {
                registry
                    .lock()
                    .unwrap()
                    .status(id)
                    .is_some_and(|status| status.state == "exited")
            };
            let deadline = Instant::now() + Duration::from_millis(800);
            while Instant::now() < deadline && !exited() {
                std::thread::sleep(Duration::from_millis(50));
            }
            if !exited() {
                let _ = dap.request(
                    "disconnect",
                    json!({ "terminateDebuggee": true, "restart": false }),
                    Duration::from_secs(2),
                );
            }
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline && !exited() {
                std::thread::sleep(Duration::from_millis(50));
            }
            if !exited() {
                dap.note("[host] adapter did not exit after disconnect; terminating its group");
                let _ = registry.lock().unwrap().terminate_status(id);
            }
            let terminal_ids: Vec<u64> = dap.data.lock().unwrap().terminal_ids.clone();
            for terminal in terminal_ids {
                let mut registry = registry.lock().unwrap();
                let running = registry
                    .status(terminal)
                    .is_some_and(|status| status.state != "exited");
                if running {
                    let _ = registry.terminate_status(terminal);
                }
            }
            let status = registry.lock().unwrap().status(id);
            if let Some(status) = status {
                dap.observe_adapter_exit(status.exit, status.termination.as_ref());
            }
        });
    }

    /// Called from the snapshot when the registry observed the adapter exit. Without an exit
    /// code the debuggee exit stays unknown; it is never reported as success.
    pub fn observe_adapter_exit(&self, exit: Option<i32>, termination: Option<&Termination>) {
        let mut data = self.data.lock().unwrap();
        if data.sealed {
            return;
        }
        if matches!(data.state, "terminated" | "cancelled" | "failed" | "idle") {
            data.adapter_exit = exit;
            return;
        }
        data.adapter_exit = exit;
        if let Some(termination) = termination {
            data.termination = Some(termination.clone());
        }
        let phase = format!(
            "netcoredbg exited ({}); debuggee exit {}",
            exit.map(|code| format!("exit {code}"))
                .unwrap_or_else(|| "unknown".to_string()),
            data.exit
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        );
        let state = if data.cancel_requested {
            "cancelled"
        } else {
            "terminated"
        };
        data.finish(state, &phase);
    }

    fn record_terminal(&self, id: u64, pid: u32, command: Vec<String>) {
        let mut data = self.data.lock().unwrap();
        data.terminal_ids.push(id);
        data.terminals.push(TerminalView {
            process_id: id,
            pid,
            command,
            state: "running".to_string(),
            exit: None,
        });
    }

    fn run_in_terminal(&self, params: &Value) -> Result<Value, String> {
        let args: Vec<String> = params
            .get("args")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        if args.is_empty() {
            return Err("runInTerminal sent an empty argv".to_string());
        }
        if params
            .get("argsCanBeInterpretedByShell")
            .and_then(Value::as_bool)
            == Some(true)
        {
            return Err(
                "runInTerminal asked for shell interpretation; the host never interpolates argv"
                    .to_string(),
            );
        }
        let cwd = params
            .get("cwd")
            .and_then(Value::as_str)
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(self.data.lock().unwrap().cwd.clone()));
        if !cwd.is_dir() {
            return Err(format!("runInTerminal cwd {} does not exist", cwd.to_string_lossy()));
        }
        let mut command = Command::new(&args[0]);
        command.args(&args[1..]).current_dir(&cwd);
        if let Some(env) = params.get("env").and_then(Value::as_object) {
            for (name, value) in env {
                match value {
                    Value::String(value) => {
                        command.env(name, value);
                    }
                    Value::Null => {
                        command.env_remove(name);
                    }
                    _ => {}
                }
            }
        }
        let registry = self
            .registry
            .as_ref()
            .ok_or_else(|| "the process registry is unavailable".to_string())?;
        let (workspace_id, project) = {
            let data = self.data.lock().unwrap();
            (
                data.workspace_id,
                data.project
                    .as_ref()
                    .map(|project| project.name.clone())
                    .unwrap_or_else(|| "debug".to_string()),
            )
        };
        let view = registry
            .lock()
            .unwrap()
            .spawn(&format!("debug-terminal:{project}"), workspace_id, command)
            .map_err(|error| format!("could not spawn runInTerminal argv: {error}"))?;
        let id = view.get("id").and_then(Value::as_u64).unwrap_or(0);
        let pid = view.get("pid").and_then(Value::as_u64).unwrap_or(0) as u32;
        self.record_terminal(id, pid, args);
        Ok(json!({ "processId": pid, "shellProcessId": pid }))
    }

    /// Real shutdown: disconnect/terminate, then observe the adapter and debuggee exit. If the
    /// adapter does not exit within the grace period the host signals the group and reports the
    /// observation; an absent exit stays `None`. With `seal`, late messages are dropped.
    pub fn shutdown(
        &self,
        registry: &Arc<Mutex<Registry>>,
        reason: &str,
        seal: bool,
    ) -> HostResult<()> {
        let (process_id, state, already_done) = {
            let data = self.data.lock().unwrap();
            (
                data.process_id,
                data.state,
                matches!(data.state, "terminated" | "cancelled" | "failed"),
            )
        };
        if state == "idle" {
            return Ok(());
        }
        if already_done && !seal {
            return Err(HostError::new("invalid_state", "No debug session is running."));
        }
        if !already_done {
            let mut data = self.data.lock().unwrap();
            data.cancel_requested = true;
            data.phase = format!("Stopping: {reason}");
            data.log.push(format!("[host] stop requested: {reason}"));
            drop(data);
            let _ = self.request(
                "disconnect",
                json!({ "terminateDebuggee": true, "restart": false }),
                Duration::from_secs(2),
            );
        }
        let mut status = None;
        if let Some(process_id) = process_id {
            let deadline = Instant::now() + DISCONNECT_WAIT;
            loop {
                status = registry.lock().unwrap().status(process_id);
                if status
                    .as_ref()
                    .is_none_or(|status| status.state == "exited")
                {
                    break;
                }
                if Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            if status
                .as_ref()
                .is_some_and(|status| status.state != "exited")
            {
                status = registry.lock().unwrap().terminate_status(process_id);
            }
        }
        let terminal_ids: Vec<u64> = self.data.lock().unwrap().terminal_ids.clone();
        for id in terminal_ids {
            let mut registry = registry.lock().unwrap();
            let current = registry
                .status(id)
                .map(|status| status.state)
                .unwrap_or("exited");
            if current != "exited" {
                let _ = registry.terminate_status(id);
            }
        }
        {
            let mut data = self.data.lock().unwrap();
            data.adapter_exit = status.as_ref().and_then(|status| status.exit);
            if let Some(status) = &status {
                if status.termination.is_some() {
                    data.termination = status.termination.clone();
                }
            }
            if data.termination.is_none() && !already_done {
                data.termination = Some(Termination {
                    requested: "DAP disconnect (terminateDebuggee)".to_string(),
                    observed: if status
                        .as_ref()
                        .is_some_and(|status| status.state == "exited")
                    {
                        "exited".to_string()
                    } else {
                        "still running".to_string()
                    },
                    exit: status.as_ref().and_then(|status| status.exit),
                    killed_after_grace: false,
                });
            }
            if !already_done {
                data.finish("cancelled", &format!("Stopped: {reason}"));
            }
            if seal {
                data.sealed = true;
                data.stale_reason = Some(reason.to_string());
                data.stop_seq += 1;
                data.log.push(format!(
                    "[host] session sealed at a workspace boundary: {reason}"
                ));
            }
        }
        Ok(())
    }

    pub fn refresh_terminals(&self, registry: &Arc<Mutex<Registry>>) {
        let ids: Vec<u64> = self.data.lock().unwrap().terminal_ids.clone();
        if ids.is_empty() {
            return;
        }
        let mut registry = registry.lock().unwrap();
        let mut data = self.data.lock().unwrap();
        for id in ids {
            let Some(status) = registry.status(id) else {
                continue;
            };
            if let Some(terminal) = data.terminals.iter_mut().find(|terminal| terminal.process_id == id)
            {
                terminal.state = status.state.to_string();
                terminal.exit = status.exit;
            }
        }
    }
}

fn parse_threads(body: &Value) -> Vec<ThreadView> {
    body.get("threads")
        .and_then(Value::as_array)
        .map(|threads| {
            threads
                .iter()
                .filter_map(|thread| {
                    Some(ThreadView {
                        id: thread.get("id").and_then(Value::as_i64)?,
                        name: thread
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("Thread")
                            .to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_frames(body: &Value) -> Vec<FrameView> {
    body.get("stackFrames")
        .and_then(Value::as_array)
        .map(|frames| {
            frames
                .iter()
                .filter_map(|frame| {
                    Some(FrameView {
                        id: frame.get("id").and_then(Value::as_i64)?,
                        name: frame
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("Frame")
                            .to_string(),
                        line: frame.get("line").and_then(Value::as_i64).unwrap_or(0),
                        column: frame.get("column").and_then(Value::as_i64).unwrap_or(0),
                        path: frame
                            .get("source")
                            .and_then(|source| source.get("path"))
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        source_name: frame
                            .get("source")
                            .and_then(|source| source.get("name"))
                            .and_then(Value::as_str)
                            .map(str::to_string),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn spawn_reader(dap: Arc<Dap>, stdout: ChildStdout) {
    std::thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            match read_frame(&mut reader) {
                Ok(Some(Frame::Message(value))) => dap.handle_message(value),
                Ok(Some(Frame::Invalid(error))) => dap.log_line(format!("[stdout] {error}")),
                Ok(None) => {
                    dap.log_line("[dap] adapter stdout closed");
                    break;
                }
                Err(error) => {
                    dap.log_line(format!("[stdout] frame read failed: {error}"));
                    break;
                }
            }
        }
    });
}

fn spawn_stderr(dap: Arc<Dap>, stderr: ChildStderr) {
    std::thread::spawn(move || {
        logbuf::pump_lines(stderr, true, |line, _| {
            dap.log_line(format!("[adapter stderr] {line}"))
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static TEMP_SEQ: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "openridr-dap-{name}-{}-{}",
            std::process::id(),
            TEMP_SEQ.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn with_sink() -> (Arc<Dap>, Arc<Mutex<Vec<u8>>>) {
        let sink = Arc::new(Mutex::new(Vec::new()));
        let dap = Dap::idle();
        let writer: Writer = Arc::new(Mutex::new(Box::new(Sink(sink.clone()))));
        dap.data.lock().unwrap().writer = Some(writer);
        (Arc::new(dap), sink)
    }

    struct Sink(Arc<Mutex<Vec<u8>>>);

    impl Write for Sink {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn written_frames(sink: &Arc<Mutex<Vec<u8>>>) -> Vec<Value> {
        let bytes = sink.lock().unwrap().clone();
        let mut values = Vec::new();
        let mut rest: &[u8] = &bytes;
        while !rest.is_empty() {
            let Some(split) = rest.windows(4).position(|window| window == b"\r\n\r\n") else {
                break;
            };
            let header = String::from_utf8_lossy(&rest[..split]).to_string();
            let Some(length) = header
                .lines()
                .find_map(|line| line.strip_prefix("Content-Length: "))
                .and_then(|value| value.trim().parse::<usize>().ok())
            else {
                break;
            };
            let start = split + 4;
            if rest.len() < start + length {
                break;
            }
            values.push(
                serde_json::from_slice(&rest[start..start + length]).expect("valid JSON frame"),
            );
            rest = &rest[start + length..];
        }
        values
    }

    #[test]
    fn requests_carry_ids_and_a_response_only_reaches_its_own_request() {
        let (dap, sink) = with_sink();
        for _ in 0..2 {
            dap.data.lock().unwrap().state = "running";
        }
        let first = dap.post_request("threads", json!({})).unwrap();
        let second = dap.post_request("stackTrace", json!({ "threadId": 1 })).unwrap();
        let frames = written_frames(&sink);
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0]["type"], "request");
        assert_eq!(frames[0]["command"], "threads");
        assert_ne!(frames[0]["seq"], frames[1]["seq"]);

        // A response for another request cannot complete this one.
        dap.handle_message(json!({
            "seq": 99, "type": "response", "request_seq": second.0 + 50,
            "success": true, "body": {}
        }));
        assert!(first.1.recv_timeout(Duration::from_millis(50)).is_err());
        dap.handle_message(json!({
            "seq": 100, "type": "response", "request_seq": first.0,
            "success": true, "body": { "threads": [] }
        }));
        let answer = first.1.recv_timeout(Duration::from_millis(500)).unwrap();
        assert_eq!(answer["request_seq"], first.0);
        assert!(dap.data.lock().unwrap().pending.contains_key(&second.0));
    }

    #[test]
    fn a_timed_out_request_is_not_completed_by_a_late_response() {
        let (dap, _) = with_sink();
        dap.data.lock().unwrap().state = "running";
        let error = dap
            .request("threads", json!({}), Duration::from_millis(40))
            .unwrap_err();
        assert!(error.contains("timed out"), "{error}");
        assert!(dap.data.lock().unwrap().pending.is_empty());
        dap.handle_message(json!({
            "seq": 2, "type": "response", "request_seq": 1, "success": true, "body": {}
        }));
        assert!(dap.data.lock().unwrap().pending.is_empty());
    }

    #[test]
    fn an_exited_event_without_a_code_stays_unknown() {
        let (dap, _) = with_sink();
        {
            let mut data = dap.data.lock().unwrap();
            data.state = "running";
        }
        dap.handle_message(json!({ "seq": 1, "type": "event", "event": "exited", "body": {} }));
        dap.handle_message(json!({ "seq": 2, "type": "event", "event": "terminated", "body": {} }));
        let view = dap.view();
        assert_eq!(view.state, "terminated");
        assert_eq!(view.exit, None);
        assert!(!view.exit_known);
        assert_eq!(view.exit, None);
    }

    #[test]
    fn late_frames_from_an_old_stop_generation_do_not_resurrect_the_pause() {
        let (dap, sink) = with_sink();
        {
            let mut data = dap.data.lock().unwrap();
            data.state = "preparing";
        }
        // Stopped starts a pause load that asks the adapter for threads.
        dap.handle_message(json!({
            "seq": 1, "type": "event", "event": "stopped",
            "body": { "reason": "breakpoint", "threadId": 7, "allThreadsStopped": true }
        }));
        assert_eq!(dap.state(), "paused");
        let deadline = Instant::now() + Duration::from_secs(2);
        let threads = loop {
            let frames = written_frames(&sink);
            if let Some(frame) = frames
                .iter()
                .find(|frame| frame["command"] == "threads")
            {
                break frame.clone();
            }
            assert!(Instant::now() < deadline, "threads request was never sent");
            std::thread::sleep(Duration::from_millis(10));
        };
        let threads = &threads;
        let request_seq = threads["seq"].as_i64().unwrap();
        // Continue before the frames response arrives: the old generation is no longer current.
        dap.handle_message(json!({ "seq": 2, "type": "event", "event": "continued", "body": {} }));
        dap.handle_message(json!({
            "seq": 3, "type": "response", "request_seq": request_seq,
            "success": true, "body": { "threads": [{ "id": 7, "name": "Main Thread" }] }
        }));
        let view = dap.view();
        assert_eq!(view.state, "running");
        assert!(view.frames.is_empty());
        assert!(view.locals.is_none());
        assert!(view.threads.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn run_in_terminal_spawns_the_exact_argv_with_cwd_and_env_without_a_shell() {
        let dir = temp_dir("run-terminal");
        let marker = dir.join("observed.txt");
        let script = dir.join("observe.sh");
        std::fs::write(
            &script,
            "#!/bin/sh\nprintf 'cwd=%s\\n' \"$PWD\" > \"$MARKER\"\nprintf 'arg1=<%s>\\n' \"$1\" >> \"$MARKER\"\nprintf 'arg2=<%s>\\n' \"$2\" >> \"$MARKER\"\nprintf 'env=<%s>\\n' \"$OPENRIDR_TEST\" >> \"$MARKER\"\n",
        )
        .unwrap();
        let registry = Arc::new(Mutex::new(Registry::new()));
        let dap = Dap::with_registry(registry.clone());
        {
            let mut data = dap.data.lock().unwrap();
            data.state = "running";
            data.workspace_id = 1;
            data.cwd = dir.to_string_lossy().to_string();
        }
        dap.handle_message(json!({
            "seq": 1, "type": "request", "command": "runInTerminal",
            "arguments": {
                "cwd": dir.to_string_lossy(),
                "args": ["/bin/sh", script.to_string_lossy(), "two words; not a shell"],
                "env": { "MARKER": marker.to_string_lossy(), "OPENRIDR_TEST": "exact" }
            }
        }));
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && !marker.is_file() {
            std::thread::sleep(Duration::from_millis(20));
        }
        let observed = std::fs::read_to_string(&marker).expect("the exact argv ran");
        assert!(observed.contains("cwd="), "{observed}");
        assert!(observed.contains("arg1=<two words; not a shell>"), "{observed}");
        assert!(observed.contains("arg2=<>"), "{observed}");
        assert!(observed.contains("env=<exact>"), "{observed}");
        let terminals = dap.view().terminals;
        assert_eq!(terminals.len(), 1);
        assert!(!terminals[0].command.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_script_adapter_over_stdio_drives_framing_breakpoints_pause_and_disconnect() {
        if std::process::Command::new("python3")
            .arg("--version")
            .output()
            .is_err()
        {
            // The controlled adapter is a local script over stdio; without an interpreter the
            // boundary cannot be exercised, and this test does not pretend it was.
            return;
        }
        let dir = temp_dir("script-adapter");
        let script = dir.join("adapter.py");
        let program = dir.join("Program.cs");
        std::fs::write(&program, "class Program { static void Main() {} }\n").unwrap();
        std::fs::write(
            &script,
            r#"import json, sys

def read_message():
    length = None
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            return None
        text = line.decode("utf-8").strip()
        if not text:
            break
        if text.lower().startswith("content-length:"):
            length = int(text.split(":", 1)[1].strip())
    if length is None:
        return None
    body = sys.stdin.buffer.read(length)
    return json.loads(body.decode("utf-8"))

def send(message):
    body = json.dumps(message).encode("utf-8")
    sys.stdout.buffer.write(b"Content-Length: " + str(len(body)).encode() + b"\r\n\r\n" + body)
    sys.stdout.buffer.flush()

seq = 0
def response(request, body):
    global seq
    seq += 1
    send({"seq": seq, "type": "response", "request_seq": request["seq"], "success": True, "command": request["command"], "body": body})

def event(name, body):
    global seq
    seq += 1
    send({"seq": seq, "type": "event", "event": name, "body": body})

while True:
    message = read_message()
    if message is None:
        break
    command = message.get("command", "")
    if command == "initialize":
        response(message, {})
        event("initialized", {})
    elif command == "launch":
        response(message, {})
    elif command == "setBreakpoints":
        response(message, {"breakpoints": [{"verified": False, "line": 4, "message": "pending until symbols load"}]})
    elif command == "configurationDone":
        response(message, {})
        event("stopped", {"reason": "breakpoint", "threadId": 7, "allThreadsStopped": True})
    elif command == "threads":
        response(message, {"threads": [{"id": 7, "name": "Main Thread"}]})
    elif command == "stackTrace":
        response(message, {"stackFrames": [{"id": 11, "name": "Program.Main", "line": 4, "column": 1, "source": {"path": "Program.cs", "name": "Program.cs"}}]})
    elif command == "scopes":
        response(message, {"scopes": [{"name": "Locals", "variablesReference": 3, "expensive": False}]})
    elif command == "variables":
        response(message, {"variables": [{"name": "order", "value": "{Order}", "type": "Order", "variablesReference": 0}]})
    elif command == "disconnect":
        response(message, {})
        event("terminated", {})
        break
    else:
        response(message, {})
"#,
        )
        .unwrap();

        let registry = Arc::new(Mutex::new(Registry::new()));
        let mut launch = Launch {
            session: 1,
            project_name: "Controlled".to_string(),
            project_path: "Controlled.csproj".to_string(),
            build_id: "build:1:1".to_string(),
            assembly: program.to_string_lossy().to_string(),
            configuration: "Debug".to_string(),
            target_framework: None,
            cwd: dir.clone(),
            root: dir.clone(),
            sdk_root: dir.clone(),
            adapter_binary: script.to_string_lossy().to_string(),
            adapter_version: "3.2.0".to_string(),
            adapter_platform: "osx-arm64".to_string(),
            workspace_id: 1,
            context_id: 1,
            breakpoints: BTreeMap::new(),
        };
        launch.breakpoints.insert("Program.cs".to_string(), vec![4]);
        let mut command = Command::new("python3");
        command.arg(&script);
        let dap = Dap::begin(launch, command, registry.clone()).unwrap();

        let deadline = Instant::now() + Duration::from_secs(8);
        while Instant::now() < deadline {
            let view = dap.view();
            if view.state == "paused" && !view.frames.is_empty() && view.locals.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let view = dap.view();
        assert_eq!(view.state, "paused", "{}", view.log);
        assert_eq!(view.frames.len(), 1);
        assert_eq!(view.frames[0].line, 4);
        assert_eq!(view.threads[0].id, 7);
        let locals = view.locals.as_ref().expect("locals were loaded");
        assert_eq!(locals.scopes[0].name, "Locals");
        assert_eq!(locals.scopes[0].variables[0].name, "order");
        // The breakpoint was pending: the controlled adapter has no symbols before the pause.
        assert!(
            view.breakpoints.iter().any(|breakpoint| !breakpoint.verified),
            "unverified breakpoints stay pending"
        );

        dap.shutdown(&registry, "test boundary", true).unwrap();
        let view = dap.view();
        assert_eq!(view.state, "cancelled");
        assert!(view.termination.is_some());
        assert!(view.sealed);
        assert_eq!(view.stale_reason.as_deref(), Some("test boundary"));
        assert_eq!(view.exit, None);
        assert!(!view.exit_known);
        // A message from the sealed session cannot resurrect frames or locals.
        dap.handle_message(json!({
            "seq": 900, "type": "event", "event": "stopped",
            "body": { "reason": "breakpoint", "threadId": 7 }
        }));
        let late = dap.view();
        assert_eq!(late.state, "cancelled");
        assert!(late.frames.is_empty());
        assert!(late.locals.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
