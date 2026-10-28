//! Explicit `dotnet restore`.
//!
//! Nothing else in the host calls restore: evaluation, build, run and tests only inspect
//! `obj/project.assets.json` freshness, and the consumers that need restored assets return
//! `restore_required` with the explicit action instead of restoring behind the user's back.
//!
//! Process output is streamed line by line into a bounded log; the process itself lives in
//! the shared registry, so cancellation and workspace boundaries use the same observed
//! termination semantics as every other child.

use crate::evaluation::{plan_workspace, relative_path};
use crate::error::{HostError, HostResult};
use crate::process::{ProcessStatus, Termination};
use serde::Serialize;
use serde_json::json;
use std::collections::VecDeque;
use std::fs;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_LOG_LINES: usize = 400;
const MAX_LOG_CHARS: usize = 60_000;
const MAX_LINE_CHARS: usize = 4_000;

/// Consumers that must not restore implicitly. Each validates the policy before its tool.
pub const GATED_METHODS: &[&str] = &[
    "build.start",
    "run.start",
    "debug.start",
    "tests.discover",
    "tests.run",
];

pub struct Start {
    pub revision: u64,
    pub scope: &'static str,
    pub target: PathBuf,
    pub command: Vec<String>,
    pub cwd: PathBuf,
    pub sdk_root: PathBuf,
    pub sdk_version: String,
    pub workspace_id: u64,
    pub context_id: u64,
    pub root: PathBuf,
    pub projects: Vec<(String, PathBuf)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreProject {
    pub name: String,
    pub path: String,
    /// `restored` | `not_restored` | `unknown`; derived from the assets file, never guessed.
    pub state: &'static str,
    pub reason: Option<String>,
    pub assets_path: Option<String>,
    pub assets_modified_at: Option<u64>,
    pub project_modified_at: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreView {
    pub state: &'static str,
    pub scope: &'static str,
    pub target: String,
    pub command: Vec<String>,
    pub cwd: String,
    pub sdk_root: Option<String>,
    pub sdk_version: Option<String>,
    pub log: String,
    pub log_truncated: bool,
    pub exit: Option<i32>,
    pub termination: Option<Termination>,
    pub process_id: Option<u64>,
    pub awaiting_decision: bool,
    pub projects: Vec<RestoreProject>,
    pub revision: u64,
    pub workspace_id: Option<u64>,
    pub context_id: Option<u64>,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    /// A record from another workspace/context. Its later writes are not applied.
    pub sealed: bool,
    pub stale_reason: Option<String>,
}

struct Data {
    revision: u64,
    state: &'static str,
    scope: &'static str,
    target: String,
    command: Vec<String>,
    cwd: String,
    sdk_root: Option<String>,
    sdk_version: Option<String>,
    workspace_id: u64,
    context_id: u64,
    root: PathBuf,
    projects: Vec<(String, PathBuf)>,
    derived: Vec<RestoreProject>,
    process_id: Option<u64>,
    pid: Option<u32>,
    exit: Option<i32>,
    termination: Option<Termination>,
    awaiting_decision: bool,
    cancel_requested: bool,
    sealed: bool,
    stale_reason: Option<String>,
    started_at: Option<u64>,
    finished_at: Option<u64>,
    log: LogBuffer,
}

pub struct Restore {
    data: Mutex<Data>,
}

impl Restore {
    pub fn idle() -> Self {
        Self {
            data: Mutex::new(Data {
                revision: 0,
                state: "idle",
                scope: "solution",
                target: String::new(),
                command: Vec::new(),
                cwd: String::new(),
                sdk_root: None,
                sdk_version: None,
                workspace_id: 0,
                context_id: 0,
                root: PathBuf::new(),
                projects: Vec::new(),
                derived: Vec::new(),
                process_id: None,
                pid: None,
                exit: None,
                termination: None,
                awaiting_decision: false,
                cancel_requested: false,
                sealed: false,
                stale_reason: None,
                started_at: None,
                finished_at: None,
                log: LogBuffer::default(),
            }),
        }
    }

    pub fn begin(start: Start) -> Self {
        Self {
            data: Mutex::new(Data {
                revision: start.revision,
                state: "running",
                scope: start.scope,
                target: start.target.to_string_lossy().to_string(),
                command: start.command,
                cwd: start.cwd.to_string_lossy().to_string(),
                sdk_root: Some(start.sdk_root.to_string_lossy().to_string()),
                sdk_version: Some(start.sdk_version),
                workspace_id: start.workspace_id,
                context_id: start.context_id,
                root: start.root,
                projects: start.projects,
                derived: Vec::new(),
                process_id: None,
                pid: None,
                exit: None,
                termination: None,
                awaiting_decision: false,
                cancel_requested: false,
                sealed: false,
                stale_reason: None,
                started_at: Some(now_ms()),
                finished_at: None,
                log: LogBuffer::default(),
            }),
        }
    }

    pub fn view(&self) -> RestoreView {
        let data = self.data.lock().unwrap();
        let (log, log_truncated) = data.log.text();
        RestoreView {
            state: data.state,
            scope: data.scope,
            target: data.target.clone(),
            command: data.command.clone(),
            cwd: data.cwd.clone(),
            sdk_root: data.sdk_root.clone(),
            sdk_version: data.sdk_version.clone(),
            log,
            log_truncated,
            exit: data.exit,
            termination: data.termination.clone(),
            process_id: data.process_id,
            awaiting_decision: data.awaiting_decision,
            projects: data.derived.clone(),
            revision: data.revision,
            workspace_id: (data.revision > 0).then_some(data.workspace_id),
            context_id: (data.revision > 0).then_some(data.context_id),
            started_at: data.started_at,
            finished_at: data.finished_at,
            sealed: data.sealed,
            stale_reason: data.stale_reason.clone(),
        }
    }

    pub fn note_process(&self, id: u64, pid: u32) {
        let mut data = self.data.lock().unwrap();
        if data.sealed {
            return;
        }
        data.process_id = Some(id);
        data.pid = Some(pid);
    }

    /// Process id of the run that may still be cancelled or observed, if any.
    pub fn running_process_id(&self) -> Option<u64> {
        let data = self.data.lock().unwrap();
        if data.state == "running" && !data.sealed {
            data.process_id
        } else {
            None
        }
    }

    pub fn workspace_id(&self) -> Option<u64> {
        let data = self.data.lock().unwrap();
        (data.revision > 0).then_some(data.workspace_id)
    }

    pub fn context_id(&self) -> Option<u64> {
        let data = self.data.lock().unwrap();
        (data.revision > 0).then_some(data.context_id)
    }

    pub fn request_cancel(&self) {        let mut data = self.data.lock().unwrap();
        if data.state == "running" && !data.sealed {
            data.cancel_requested = true;
        }
    }

    /// Records what the registry observed; only a running, unsealed record changes.
    pub fn observe(&self, status: &ProcessStatus) {
        let mut data = self.data.lock().unwrap();
        if data.state != "running" || data.sealed {
            return;
        }
        data.exit = status.exit;
        data.termination = status.termination.clone();
        data.awaiting_decision = status.awaiting_decision;
    }

    /// Finalizes a running record from an observed exit. Returns the resulting state.
    pub fn complete(&self, status: &ProcessStatus) -> &'static str {
        let mut data = self.data.lock().unwrap();
        if data.state != "running" || data.sealed {
            return data.state;
        }
        data.exit = status.exit;
        if let Some(termination) = &status.termination {
            data.termination = Some(termination.clone());
        }
        let cancelled = data.cancel_requested || data.termination.is_some();
        data.state = if cancelled {
            "cancelled"
        } else if status.exit == Some(0) {
            "ok"
        } else {
            "failed"
        };
        data.awaiting_decision = false;
        data.derived = derive_projects(&data.root, &data.projects);
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
        data.derived = derive_projects(&data.root, &data.projects);
    }

    /// A workspace boundary: the user leaves trust/workspace while a restore may still run.
    /// The outcome is sealed so a later exit cannot be reported as the new context's restore.
    pub fn seal(&self, reason: impl Into<String>, status: Option<&ProcessStatus>) {
        let mut data = self.data.lock().unwrap();
        if data.state != "running" {
            return;
        }
        data.cancel_requested = true;
        data.sealed = true;
        data.stale_reason = Some(reason.into());
        if let Some(status) = status {
            data.exit = status.exit;
            data.termination = status.termination.clone();
            data.awaiting_decision = status.state == "running";
            if status.state == "exited" {
                data.state = if data.termination.is_some() {
                    "cancelled"
                } else if status.exit == Some(0) {
                    "ok"
                } else {
                    "failed"
                };
                data.derived = derive_projects(&data.root, &data.projects);
                data.finished_at = Some(now_ms());
            }
        }
    }

    /// Streamed output stays available after success, failure and cancellation. Sealed
    /// records drop later lines, so an old process cannot rewrite another context.
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

/// Streams one pipe of the restore process into the record, line by line. Drains even when
/// the record is sealed so the child cannot block on a full pipe.
pub fn pump(reader: impl Read, restore: Arc<Restore>, stderr: bool) {
    let mut reader = BufReader::new(reader);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        match reader.read_until(b'\n', &mut bytes) {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let text = String::from_utf8_lossy(&bytes);
        let text = text.trim_end_matches(['\r', '\n']);
        if text.is_empty() {
            continue;
        }
        restore.append_line(text, stderr);
    }
}

fn derive_projects(root: &Path, projects: &[(String, PathBuf)]) -> Vec<RestoreProject> {
    projects
        .iter()
        .map(|(name, path)| {
            let info = assets(path);
            RestoreProject {
                name: name.clone(),
                path: relative_path(root, path),
                state: info.state,
                reason: info.reason,
                assets_path: info.assets_path,
                assets_modified_at: info.assets_modified_at,
                project_modified_at: info.project_modified_at,
            }
        })
        .collect()
}

pub struct Assets {
    pub state: &'static str,
    pub reason: Option<String>,
    pub assets_path: Option<String>,
    pub assets_modified_at: Option<u64>,
    pub project_modified_at: Option<u64>,
}

/// Freshness of `obj/project.assets.json` against the project file. A missing or older
/// assets file is `not_restored`; a file whose timestamps cannot be compared is `unknown`.
pub fn assets(project_file: &Path) -> Assets {
    let project_modified_at = mtime_ms(project_file);
    let Some(dir) = project_file.parent() else {
        return Assets {
            state: "unknown",
            reason: Some("The project has no directory.".to_string()),
            assets_path: None,
            assets_modified_at: None,
            project_modified_at,
        };
    };
    let assets = dir.join("obj").join("project.assets.json");
    let assets_path = Some(assets.to_string_lossy().to_string());
    let meta = match fs::metadata(&assets) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Assets {
                state: "not_restored",
                reason: Some(
                    "obj/project.assets.json is missing; restore was not requested.".to_string(),
                ),
                assets_path,
                assets_modified_at: None,
                project_modified_at,
            }
        }
        Err(error) => {
            return Assets {
                state: "unknown",
                reason: Some(format!("Could not read obj/project.assets.json: {error}")),
                assets_path,
                assets_modified_at: None,
                project_modified_at,
            }
        }
    };
    let assets_modified_at = meta.modified().ok().and_then(system_ms);
    // NuGet rewrites project.assets.json even when restore fails; the cache file records
    // whether the last restore of this project succeeded, so freshness alone cannot claim it.
    let cache = dir.join("obj").join("project.nuget.cache");
    match last_restore_success(&cache) {
        Some(false) => {
            return Assets {
                state: "not_restored",
                reason: Some(
                    "The last restore of this project failed (obj/project.nuget.cache reports success: false); dependencies are incomplete."
                        .to_string(),
                ),
                assets_path,
                assets_modified_at,
                project_modified_at,
            }
        }
        None => {
            return Assets {
                state: "unknown",
                reason: Some(
                    "obj/project.nuget.cache is missing or unreadable; restore status cannot be verified."
                        .to_string(),
                ),
                assets_path,
                assets_modified_at,
                project_modified_at,
            }
        }
        Some(true) => {}
    }
    match (project_modified_at, assets_modified_at) {
        (Some(project_time), Some(assets_time)) if assets_time < project_time => Assets {
            state: "not_restored",
            reason: Some(
                "obj/project.assets.json is older than the project file; restore was not requested."
                    .to_string(),
            ),
            assets_path,
            assets_modified_at,
            project_modified_at,
        },
        (Some(_), Some(_)) => Assets {
            state: "restored",
            reason: None,
            assets_path,
            assets_modified_at,
            project_modified_at,
        },
        _ => Assets {
            state: "unknown",
            reason: Some("Could not compare the assets and project timestamps.".to_string()),
            assets_path,
            assets_modified_at,
            project_modified_at,
        },
    }
}

pub fn state(project_file: &Path) -> (&'static str, Option<String>) {
    let info = assets(project_file);
    (info.state, info.reason)
}

fn last_restore_success(cache: &Path) -> Option<bool> {
    let text = fs::read_to_string(cache).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("success").and_then(serde_json::Value::as_bool)
}

/// The policy every later consumer (build/run/debug/tests) must pass before running.
pub fn require_restored(
    workspace_file: &Path,
    root: &Path,
    is_solution: bool,
    consumer: &str,
) -> HostResult<()> {
    let plan = plan_workspace(workspace_file, root, is_solution);
    let mut incomplete = Vec::new();
    for project in &plan.projects {
        let Some(path) = project.abs_path.as_deref() else {
            incomplete.push(json!({
                "name": project.name,
                "path": project.path,
                "restored": "unknown",
                "reason": project.error,
            }));
            continue;
        };
        let (restored, reason) = state(path);
        if restored != "restored" {
            incomplete.push(json!({
                "name": project.name,
                "path": project.path,
                "restored": restored,
                "reason": reason,
            }));
        }
    }
    if incomplete.is_empty() {
        return Ok(());
    }
    Err(HostError::with_details(
        "restore_required",
        format!(
            "{consumer} needs current restore assets and OpenRidr never restores implicitly; run the explicit restore first."
        ),
        json!({
            "consumer": consumer,
            "projects": incomplete,
            "action": { "method": "restore.start", "params": {} },
        }),
    ))
}

#[derive(Default)]
struct LogBuffer {
    lines: VecDeque<String>,
    chars: usize,
    dropped: usize,
}

impl LogBuffer {
    fn push(&mut self, line: String) {
        let mut line = redact_url_credentials(&line);
        if line.chars().count() > MAX_LINE_CHARS {
            line = format!("{}…", line.chars().take(MAX_LINE_CHARS).collect::<String>());
        }
        self.chars += line.chars().count() + 1;
        self.lines.push_back(line);
        while self.lines.len() > MAX_LOG_LINES || self.chars > MAX_LOG_CHARS {
            let Some(front) = self.lines.pop_front() else {
                break;
            };
            self.chars = self.chars.saturating_sub(front.chars().count() + 1);
            self.dropped += 1;
        }
    }

    fn text(&self) -> (String, bool) {
        let mut text = String::new();
        if self.dropped > 0 {
            text.push_str(&format!(
                "… log truncated: {} earlier line(s) not kept; showing the last {} line(s) …\n",
                self.dropped,
                self.lines.len()
            ));
        }
        for line in &self.lines {
            text.push_str(line);
            text.push('\n');
        }
        (text.trim_end().to_string(), self.dropped > 0)
    }
}

/// Feed URLs can carry credentials; the visible log never keeps the userinfo part.
fn redact_url_credentials(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(index) = rest.find("://") {
        let (head, tail) = rest.split_at(index + 3);
        out.push_str(head);
        let end = tail
            .find(['/', ' ', '"', '\'', ')'])
            .unwrap_or(tail.len());
        match tail[..end].find('@') {
            Some(at) => {
                out.push_str("***@");
                rest = &tail[at + 1..];
            }
            None => rest = tail,
        }
    }
    out.push_str(rest);
    out
}

fn system_ms(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as u64)
}

fn mtime_ms(path: &Path) -> Option<u64> {
    fs::metadata(path).ok()?.modified().ok().and_then(system_ms)
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

    #[test]
    fn url_credentials_are_not_kept_in_the_log() {
        let line = "error: source https://user:secret@feed.example/v3/index.json failed";
        assert_eq!(
            redact_url_credentials(line),
            "error: source https://***@feed.example/v3/index.json failed"
        );
        assert_eq!(
            redact_url_credentials("https://feed.example/v3/index.json"),
            "https://feed.example/v3/index.json"
        );
    }

    #[test]
    fn the_log_keeps_the_tail_and_reports_truncation() {
        let mut log = LogBuffer::default();
        for index in 0..(MAX_LOG_LINES + 5) {
            log.push(format!("line {index}"));
        }
        let (text, truncated) = log.text();
        assert!(truncated);
        assert!(text.starts_with("… log truncated: 5 earlier line(s) not kept"));
        assert!(text.ends_with(&format!("line {}", MAX_LOG_LINES + 4)));
        assert!(!text.contains("line 0\n"));
    }

    #[test]
    fn assets_missing_is_not_restored_and_never_success() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("P.csproj");
        fs::write(&project, "<Project />").unwrap();
        assert_eq!(state(&project).0, "not_restored");
        fs::create_dir_all(temp.path().join("obj")).unwrap();
        fs::write(temp.path().join("obj/project.assets.json"), "{}").unwrap();
        assert_eq!(state(&project).0, "unknown");
        fs::write(
            temp.path().join("obj/project.nuget.cache"),
            "{\"success\":false}",
        )
        .unwrap();
        assert_eq!(state(&project).0, "not_restored");
        fs::write(
            temp.path().join("obj/project.nuget.cache"),
            "{\"success\":true}",
        )
        .unwrap();
        assert_eq!(state(&project).0, "restored");
    }

    #[test]
    fn policy_reports_every_project_without_assets() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        fs::write(
            root.join("P.csproj"),
            "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n",
        )
        .unwrap();
        let error = require_restored(&root.join("P.csproj"), &root, false, "build.start")
            .expect_err("missing assets need an explicit restore");
        assert_eq!(error.code, "restore_required");
        let details = error.details.unwrap();
        assert_eq!(details["action"]["method"], "restore.start");
        assert_eq!(details["projects"][0]["restored"], "not_restored");
    }
}
