//! Run of the TargetPath from the identified successful build.
//!
//! Run restores never and guesses no path: it launches the evaluated `TargetPath` recorded by
//! the current build (through the project SDK muxer for a managed `.dll`) and streams the real
//! stdout/stderr into a bounded log. When there is no current build, the host starts the same
//! `build.start` pipeline and shows `building` until it finishes (decision 2026-11-01). Exit and
//! termination are the ones the registry observed, so a missing exit code is never success.

use crate::error::{HostError, HostResult};
use crate::logbuf::LogBuffer;
use crate::process::{signal_name, ProcessStatus, Termination};
use serde::Serialize;
use serde_json::json;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Start {
    pub revision: u64,
    pub run_id: String,
    pub project_name: String,
    pub project_path: String,
    pub target_path: String,
    pub build_id: String,
    pub configuration: String,
    pub target_framework: Option<String>,
    pub command: Vec<String>,
    pub cwd: String,
    pub workspace_id: u64,
    pub context_id: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunProject {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunView {
    pub state: &'static str,
    pub project: Option<RunProject>,
    pub target_path: Option<String>,
    pub build_id: Option<String>,
    pub configuration: String,
    pub target_framework: Option<String>,
    pub command: Vec<String>,
    pub cwd: String,
    pub log: String,
    pub log_truncated: bool,
    pub exit: Option<i32>,
    pub signal: Option<String>,
    pub termination: Option<Termination>,
    pub process_id: Option<u64>,
    pub awaiting_decision: bool,
    pub revision: u64,
    pub run_id: Option<String>,
    pub workspace_id: Option<u64>,
    pub context_id: Option<u64>,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub sealed: bool,
    pub stale: bool,
    pub stale_reason: Option<String>,
}

struct Data {
    revision: u64,
    state: &'static str,
    run_id: String,
    project: Option<RunProject>,
    target_path: Option<String>,
    build_id: String,
    configuration: String,
    target_framework: Option<String>,
    command: Vec<String>,
    cwd: String,
    workspace_id: u64,
    context_id: u64,
    process_id: Option<u64>,
    exit: Option<i32>,
    signal: Option<i32>,
    termination: Option<Termination>,
    awaiting_decision: bool,
    cancel_requested: bool,
    sealed: bool,
    stale: bool,
    stale_reason: Option<String>,
    started_at: Option<u64>,
    finished_at: Option<u64>,
    log: LogBuffer,
}

pub struct Run {
    data: Mutex<Data>,
}

impl Run {
    pub fn idle() -> Self {
        Self {
            data: Mutex::new(Data {
                revision: 0,
                state: "idle",
                run_id: String::new(),
                project: None,
                target_path: None,
                build_id: String::new(),
                configuration: String::new(),
                target_framework: None,
                command: Vec::new(),
                cwd: String::new(),
                workspace_id: 0,
                context_id: 0,
                process_id: None,
                exit: None,
                signal: None,
                termination: None,
                awaiting_decision: false,
                cancel_requested: false,
                sealed: false,
                stale: false,
                stale_reason: None,
                started_at: None,
                finished_at: None,
                log: LogBuffer::default(),
            }),
        }
    }

    pub fn begin(start: Start) -> Self {
        Self::from_start(start, "running", Some(now_ms()))
    }

    /// No process yet: the host is building the target this run needs. The record carries the
    /// same identity, so the snapshot can show `building` with the project and build id.
    pub fn building(start: Start) -> Self {
        Self::from_start(start, "building", None)
    }

    fn from_start(start: Start, state: &'static str, started_at: Option<u64>) -> Self {
        Self {
            data: Mutex::new(Data {
                revision: start.revision,
                state,
                run_id: start.run_id,
                project: Some(RunProject {
                    name: start.project_name,
                    path: start.project_path,
                }),
                target_path: Some(start.target_path),
                build_id: start.build_id,
                configuration: start.configuration,
                target_framework: start.target_framework,
                command: start.command,
                cwd: start.cwd,
                workspace_id: start.workspace_id,
                context_id: start.context_id,
                process_id: None,
                exit: None,
                signal: None,
                termination: None,
                awaiting_decision: false,
                cancel_requested: false,
                sealed: false,
                stale: false,
                stale_reason: None,
                started_at,
                finished_at: None,
                log: LogBuffer::default(),
            }),
        }
    }

    pub fn view(&self) -> RunView {
        let data = self.data.lock().unwrap();
        let (log, log_truncated) = data.log.text();
        RunView {
            state: data.state,
            project: data.project.clone(),
            target_path: data.target_path.clone(),
            build_id: (data.revision > 0).then(|| data.build_id.clone()),
            configuration: data.configuration.clone(),
            target_framework: data.target_framework.clone(),
            command: data.command.clone(),
            cwd: data.cwd.clone(),
            log,
            log_truncated,
            exit: data.exit,
            signal: data.signal.map(signal_name),
            termination: data.termination.clone(),
            process_id: data.process_id,
            awaiting_decision: data.awaiting_decision,
            revision: data.revision,
            run_id: (data.revision > 0).then(|| data.run_id.clone()),
            workspace_id: (data.revision > 0).then_some(data.workspace_id),
            context_id: (data.revision > 0).then_some(data.context_id),
            started_at: data.started_at,
            finished_at: data.finished_at,
            sealed: data.sealed,
            stale: data.stale,
            stale_reason: data.stale_reason.clone(),
        }
    }

    pub fn note_process(&self, id: u64) {
        let mut data = self.data.lock().unwrap();
        if data.sealed || data.state != "running" {
            return;
        }
        data.process_id = Some(id);
    }

    pub fn running_process_id(&self) -> Option<u64> {
        let data = self.data.lock().unwrap();
        if data.state == "running" && !data.sealed {
            data.process_id
        } else {
            None
        }
    }

    pub fn request_cancel(&self) {
        let mut data = self.data.lock().unwrap();
        if data.state == "running" && !data.sealed {
            data.cancel_requested = true;
        }
    }

    pub fn observe(&self, status: &ProcessStatus) {
        let mut data = self.data.lock().unwrap();
        if data.state != "running" || data.sealed {
            return;
        }
        data.exit = status.exit;
        data.signal = status.signal;
        data.termination = status.termination.clone();
        data.awaiting_decision = status.awaiting_decision;
    }

    /// The observed exit is recorded as-is. `exited` is neutral: a non-zero exit stays a
    /// real exit code and `null` stays unknown, never success. Only a termination the host
    /// requested changes the state to `cancelled`.
    pub fn complete(&self, status: &ProcessStatus) -> &'static str {
        let mut data = self.data.lock().unwrap();
        if data.state != "running" || data.sealed {
            return data.state;
        }
        data.exit = status.exit;
        data.signal = status.signal;
        if let Some(termination) = &status.termination {
            data.termination = Some(termination.clone());
        }
        let cancelled = data.cancel_requested || data.termination.is_some();
        data.state = if cancelled { "cancelled" } else { "exited" };
        data.awaiting_decision = false;
        data.finished_at = Some(now_ms());
        data.state
    }

    pub fn fail(&self, message: impl Into<String>) {
        let mut data = self.data.lock().unwrap();
        if data.sealed || data.state != "running" {
            return;
        }
        data.log.push(message.into());
        data.state = "failed";
        data.finished_at = Some(now_ms());
    }

    /// The implicit build did not succeed: the run fails with the reason and never spawns.
    pub fn fail_pending(&self, message: impl Into<String>) {
        let mut data = self.data.lock().unwrap();
        if data.sealed || data.state != "building" {
            return;
        }
        data.log.push(message.into());
        data.state = "failed";
        data.finished_at = Some(now_ms());
    }

    /// The implicit build was cancelled or replaced: no program ran.
    pub fn cancel_pending(&self) {
        let mut data = self.data.lock().unwrap();
        if data.sealed || data.state != "building" {
            return;
        }
        data.state = "cancelled";
        data.finished_at = Some(now_ms());
    }

    /// A workspace boundary seals the record: a late line or exit from the old process
    /// cannot be reported as the new context's run.
    pub fn seal(&self, reason: impl Into<String>, status: Option<&ProcessStatus>) {
        let mut data = self.data.lock().unwrap();
        if data.revision == 0 {
            return;
        }
        data.sealed = true;
        data.stale = true;
        data.stale_reason = Some(reason.into());
        if data.state != "running" {
            return;
        }
        data.cancel_requested = true;
        if let Some(status) = status {
            data.exit = status.exit;
            data.signal = status.signal;
            data.termination = status.termination.clone();
            data.awaiting_decision = status.state == "running";
            if status.state == "exited" {
                data.state = if data.termination.is_some() {
                    "cancelled"
                } else {
                    "exited"
                };
                data.finished_at = Some(now_ms());
            }
        }
    }

    pub fn append_line(&self, line: &str, stderr: bool) {
        let mut data = self.data.lock().unwrap();
        if data.sealed {
            return;
        }
        let line = if stderr {
            format!("[stderr] {line}")
        } else {
            line.to_string()
        };
        data.log.push(line);
    }
}

/// Resolves the launch from the evaluated TargetPath, never from a guessed apphost path.
/// A managed `.dll` runs through the project SDK muxer; any other target must be a real
/// executable file. A missing target or a non-executable one is refused with its reason.
pub fn launch_command(target: &Path, muxer: &Path) -> HostResult<Vec<String>> {
    let target_text = target.to_string_lossy().to_string();
    let meta = fs::metadata(target).map_err(|error| {
        HostError::with_details(
            "invalid_state",
            format!(
                "Run was not started: the built TargetPath {target_text} is missing ({error}). Rebuild before running."
            ),
            json!({
                "reason": "target_missing",
                "targetPath": target_text,
                "action": { "method": "build.start", "params": {} },
            }),
        )
    })?;
    if !meta.is_file() {
        return Err(HostError::with_details(
            "invalid_state",
            format!("Run was not started: TargetPath {target_text} is not a regular file."),
            json!({
                "reason": "target_not_file",
                "targetPath": target_text,
                "action": { "method": "build.start", "params": {} },
            }),
        ));
    }
    let managed = target
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("dll"));
    let command = if managed {
        if !muxer.is_file() {
            return Err(HostError::with_details(
                "sdk_missing",
                format!(
                    "Run was not started: {} does not contain a .NET muxer.",
                    muxer.to_string_lossy()
                ),
                json!({ "reason": "muxer_missing", "targetPath": target_text }),
            ));
        }
        vec![
            muxer.to_string_lossy().to_string(),
            target.to_string_lossy().to_string(),
        ]
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if meta.permissions().mode() & 0o111 == 0 {
                return Err(HostError::with_details(
                    "invalid_state",
                    format!(
                        "Run was not started: TargetPath {target_text} is not executable; rebuild or fix its permissions."
                    ),
                    json!({
                        "reason": "target_not_executable",
                        "targetPath": target_text,
                        "action": { "method": "build.start", "params": {} },
                    }),
                ));
            }
        }
        vec![target.to_string_lossy().to_string()]
    };
    Ok(command)
}

/// Streams one pipe of the run into the record. Drains even when sealed so the child cannot
/// block on a full pipe.
pub fn pump(reader: impl Read, run: std::sync::Arc<Run>, stderr: bool) {
    let mut reader = std::io::BufReader::new(reader);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match std::io::BufRead::read_until(&mut reader, b'\n', &mut bytes) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let text = String::from_utf8_lossy(&bytes);
        let text = text.trim_end_matches(['\r', '\n']);
        if !text.is_empty() {
            run.append_line(text, stderr);
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start() -> Start {
        Start {
            revision: 1,
            run_id: "run:1:1".to_string(),
            project_name: "App".to_string(),
            project_path: "App.csproj".to_string(),
            target_path: "/work/bin/App.dll".to_string(),
            build_id: "build:1:1".to_string(),
            configuration: "Debug".to_string(),
            target_framework: None,
            command: vec!["/sdk/dotnet".to_string(), "/work/bin/App.dll".to_string()],
            cwd: "/work".to_string(),
            workspace_id: 1,
            context_id: 1,
        }
    }

    fn started() -> Run {
        Run::begin(start())
    }

    #[test]
    fn a_pending_build_is_visible_and_never_becomes_a_running_program() {
        let failed = Run::building(start());
        assert_eq!(failed.view().state, "building");
        assert_eq!(failed.running_process_id(), None);
        failed.fail_pending("Build failed; see the Build tab for diagnostics.");
        assert_eq!(failed.view().state, "failed");
        assert!(failed
            .view()
            .log
            .contains("Build failed; see the Build tab"));

        let cancelled = Run::building(start());
        cancelled.cancel_pending();
        let view = cancelled.view();
        assert_eq!(view.state, "cancelled");
        assert_eq!(view.exit, None);
        assert_eq!(view.build_id.as_deref(), Some("build:1:1"));
        assert_eq!(view.run_id.as_deref(), Some("run:1:1"));
    }

    fn status(
        state: &'static str,
        exit: Option<i32>,
        termination: Option<Termination>,
    ) -> ProcessStatus {
        ProcessStatus {
            state,
            exit,
            signal: None,
            termination,
            awaiting_decision: false,
        }
    }

    #[test]
    fn an_observed_exit_is_reported_and_a_missing_code_stays_unknown() {
        let run = started();
        run.append_line("hello", false);
        run.append_line("oops", true);
        assert_eq!(run.view().state, "running");
        assert_eq!(run.complete(&status("exited", Some(0), None)), "exited");
        let view = run.view();
        assert_eq!(view.exit, Some(0));
        assert!(view.log.contains("hello"));
        assert!(view.log.contains("[stderr] oops"));
        assert_eq!(view.run_id.as_deref(), Some("run:1:1"));
        assert_eq!(view.build_id.as_deref(), Some("build:1:1"));

        let unknown = started();
        assert_eq!(unknown.complete(&status("exited", None, None)), "exited");
        assert_eq!(unknown.view().exit, None);
    }

    #[test]
    fn cancel_and_seal_keep_the_log_and_drop_late_output() {
        let run = started();
        run.append_line("before cancel", false);
        run.request_cancel();
        assert_eq!(run.view().state, "running");
        assert_eq!(run.view().exit, None);
        let termination = Termination {
            requested: "SIGTERM".to_string(),
            observed: "exited".to_string(),
            exit: Some(143),
            killed_after_grace: false,
        };
        assert_eq!(
            run.complete(&status("exited", Some(143), Some(termination))),
            "cancelled"
        );
        let view = run.view();
        assert_eq!(view.exit, Some(143));
        assert_eq!(view.termination.as_ref().unwrap().observed, "exited");
        assert!(view.log.contains("before cancel"));

        run.seal("Trust was revoked.", None);
        let finished_at = run.view().finished_at;
        run.append_line("late line", false);
        let sealed = run.view();
        assert!(sealed.sealed && sealed.stale);
        assert_eq!(sealed.finished_at, finished_at);
        assert!(!sealed.log.contains("late line"));
        assert_eq!(run.complete(&status("exited", Some(0), None)), "cancelled");
        assert_eq!(run.view().exit, Some(143));
    }

    #[cfg(unix)]
    #[test]
    fn a_missing_or_non_executable_target_is_refused_with_its_reason() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("Missing.dll");
        let error = launch_command(&missing, &temp.path().join("dotnet")).expect_err("missing");
        assert_eq!(error.details.unwrap()["reason"], "target_missing");

        let plain = temp.path().join("app");
        fs::write(&plain, "#!/bin/sh\necho hi\n").unwrap();
        let error = launch_command(&plain, &temp.path().join("dotnet")).expect_err("no x bit");
        assert_eq!(error.details.unwrap()["reason"], "target_not_executable");

        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&plain, fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            launch_command(&plain, &temp.path().join("dotnet")).unwrap(),
            vec![plain.to_string_lossy().to_string()]
        );
    }

    #[test]
    fn a_managed_target_runs_through_the_muxer_and_never_a_guessed_path() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("App.dll");
        fs::write(&target, "not really a dll").unwrap();
        let muxer = temp.path().join("dotnet");
        fs::write(&muxer, "").unwrap();
        let command = launch_command(&target, &muxer).unwrap();
        assert_eq!(command.len(), 2);
        assert_eq!(command[0], muxer.to_string_lossy());
        assert_eq!(command[1], target.to_string_lossy());
    }
}
