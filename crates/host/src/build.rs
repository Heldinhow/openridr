//! Explicit `dotnet build` of one console startup project.
//!
//! The build consumes the current evaluated model (never a stale TargetPath), only runs
//! after `restore::require_restored` passed, and never saves a buffer. Output is streamed
//! line by line into a bounded log; MSBuild diagnostics are parsed conservatively from that
//! text and kept under their own origin (`Build · dotnet`), separate from the LSP store.

use crate::error::{HostError, HostResult};
use crate::evaluation::{relative_path, EvaluatedProject};
use crate::logbuf::LogBuffer;
use crate::process::ProcessStatus;
use crate::process::Termination;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// A build is a snapshot of one compilation; more problems than this are not presented as
/// individually navigable, but the raw log keeps every line.
pub const MAX_PROBLEMS: usize = 200;

pub struct Start {
    pub revision: u64,
    pub build_id: String,
    pub project_name: String,
    pub project_path: String,
    pub project_kind: &'static str,
    pub target_path: Option<String>,
    pub configuration: String,
    pub target_framework: Option<String>,
    pub command: Vec<String>,
    pub cwd: PathBuf,
    pub sdk_root: PathBuf,
    pub sdk_version: String,
    pub workspace_id: u64,
    pub context_id: u64,
    pub root: PathBuf,
    pub saved_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildProject {
    pub name: String,
    pub path: String,
    pub kind: &'static str,
    pub target_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildPosition {
    pub line: u64,
    pub character: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildRange {
    pub start: BuildPosition,
    pub end: BuildPosition,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildProblem {
    pub id: String,
    pub severity: &'static str,
    pub severity_label: &'static str,
    /// LSP-compatible numbering (1 error, 2 warning); never inferred for other severities.
    pub severity_raw: u32,
    pub code: Option<String>,
    pub source: &'static str,
    pub category: &'static str,
    pub message: String,
    pub path: String,
    /// Zero-based, only when MSBuild printed a `(line,column)` pair.
    pub position: Option<BuildPosition>,
    pub range: Option<BuildRange>,
    pub build_id: String,
    pub revision: u64,
    /// The exact log line this problem came from.
    pub raw: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildView {
    pub state: &'static str,
    pub project: Option<BuildProject>,
    pub configuration: String,
    pub target_framework: Option<String>,
    pub command: Vec<String>,
    pub cwd: String,
    pub sdk_root: Option<String>,
    pub sdk_version: Option<String>,
    pub log: String,
    pub log_truncated: bool,
    pub exit: Option<i32>,
    pub termination: Option<Termination>,
    pub revision: u64,
    pub build_id: Option<String>,
    pub workspace_id: Option<u64>,
    pub context_id: Option<u64>,
    pub process_id: Option<u64>,
    pub awaiting_decision: bool,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub stale: bool,
    pub stale_reason: Option<String>,
    pub sealed: bool,
    pub saved_note: Option<String>,
    pub problems: Vec<BuildProblem>,
    pub problems_truncated: bool,
}

struct Data {
    revision: u64,
    state: &'static str,
    build_id: String,
    project: Option<BuildProject>,
    configuration: String,
    target_framework: Option<String>,
    command: Vec<String>,
    cwd: String,
    sdk_root: Option<String>,
    sdk_version: Option<String>,
    workspace_id: u64,
    context_id: u64,
    root: PathBuf,
    process_id: Option<u64>,
    exit: Option<i32>,
    termination: Option<Termination>,
    awaiting_decision: bool,
    cancel_requested: bool,
    sealed: bool,
    stale: bool,
    stale_reason: Option<String>,
    saved_note: Option<String>,
    started_at: Option<u64>,
    finished_at: Option<u64>,
    log: LogBuffer,
    problems: Vec<BuildProblem>,
    problems_truncated: bool,
    /// Positions of accepted problems, used only to avoid printing the same MSBuild
    /// diagnostic twice (inline and again in the failure summary).
    seen: HashMap<String, ()>,
}

pub struct Build {
    data: Mutex<Data>,
}

impl Build {
    pub fn idle() -> Self {
        Self {
            data: Mutex::new(Data {
                revision: 0,
                state: "idle",
                build_id: String::new(),
                project: None,
                configuration: String::new(),
                target_framework: None,
                command: Vec::new(),
                cwd: String::new(),
                sdk_root: None,
                sdk_version: None,
                workspace_id: 0,
                context_id: 0,
                root: PathBuf::new(),
                process_id: None,
                exit: None,
                termination: None,
                awaiting_decision: false,
                cancel_requested: false,
                sealed: false,
                stale: false,
                stale_reason: None,
                saved_note: None,
                started_at: None,
                finished_at: None,
                log: LogBuffer::default(),
                problems: Vec::new(),
                problems_truncated: false,
                seen: HashMap::new(),
            }),
        }
    }

    pub fn begin(start: Start) -> Self {
        Self {
            data: Mutex::new(Data {
                revision: start.revision,
                state: "running",
                build_id: start.build_id,
                project: Some(BuildProject {
                    name: start.project_name,
                    path: start.project_path,
                    kind: start.project_kind,
                    target_path: start.target_path,
                }),
                configuration: start.configuration,
                target_framework: start.target_framework,
                command: start.command,
                cwd: start.cwd.to_string_lossy().to_string(),
                sdk_root: Some(start.sdk_root.to_string_lossy().to_string()),
                sdk_version: Some(start.sdk_version),
                workspace_id: start.workspace_id,
                context_id: start.context_id,
                root: start.root,
                process_id: None,
                exit: None,
                termination: None,
                awaiting_decision: false,
                cancel_requested: false,
                sealed: false,
                stale: false,
                stale_reason: None,
                saved_note: start.saved_note,
                started_at: Some(now_ms()),
                finished_at: None,
                log: LogBuffer::default(),
                problems: Vec::new(),
                problems_truncated: false,
                seen: HashMap::new(),
            }),
        }
    }

    pub fn view(&self) -> BuildView {
        let data = self.data.lock().unwrap();
        let (log, log_truncated) = data.log.text();
        BuildView {
            state: data.state,
            project: data.project.clone(),
            configuration: data.configuration.clone(),
            target_framework: data.target_framework.clone(),
            command: data.command.clone(),
            cwd: data.cwd.clone(),
            sdk_root: data.sdk_root.clone(),
            sdk_version: data.sdk_version.clone(),
            log,
            log_truncated,
            exit: data.exit,
            termination: data.termination.clone(),
            revision: data.revision,
            build_id: (data.revision > 0).then(|| data.build_id.clone()),
            workspace_id: (data.revision > 0).then_some(data.workspace_id),
            context_id: (data.revision > 0).then_some(data.context_id),
            process_id: data.process_id,
            awaiting_decision: data.awaiting_decision,
            started_at: data.started_at,
            finished_at: data.finished_at,
            stale: data.stale,
            stale_reason: data.stale_reason.clone(),
            sealed: data.sealed,
            saved_note: data.saved_note.clone(),
            problems: data.problems.clone(),
            problems_truncated: data.problems_truncated,
        }
    }

    pub fn note_process(&self, id: u64) {
        let mut data = self.data.lock().unwrap();
        if data.sealed {
            return;
        }
        data.process_id = Some(id);
    }

    /// Process id of the build that may still be cancelled or observed, if any.
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
    /// Only a real exit observed by the registry can produce `ok`: `exit: null` is failure.
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

    pub fn mark_stale(&self, reason: impl Into<String>) {
        let mut data = self.data.lock().unwrap();
        if data.revision == 0 {
            return;
        }
        data.stale = true;
        if data.stale_reason.is_none() {
            data.stale_reason = Some(reason.into());
        }
    }

    /// A workspace boundary: the user leaves trust/workspace while a build may still run.
    /// The outcome is sealed so a later exit cannot be reported as the new context's build.
    pub fn seal(&self, reason: impl Into<String>, status: Option<&ProcessStatus>) {
        let mut data = self.data.lock().unwrap();
        if data.revision == 0 {
            return;
        }
        data.stale = true;
        data.stale_reason = Some(reason.into());
        // Even a finished record is sealed at the boundary: its log must not grow lines from
        // a process that belonged to the previous context.
        data.sealed = true;
        if data.state != "running" {
            return;
        }
        data.cancel_requested = true;
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
        let parsed = parse_line(&data.root, line);
        let shown = if stderr {
            format!("[stderr] {line}")
        } else {
            line.to_string()
        };
        data.log.push(shown);
        let Line::Problem(problem) = parsed else {
            return;
        };
        let key = format!(
            "{}:{:?}:{:?}:{}",
            problem.path, problem.position, problem.code, problem.message
        );
        if data.seen.contains_key(&key) {
            return;
        }
        if data.problems.len() >= MAX_PROBLEMS {
            data.problems_truncated = true;
            return;
        }
        data.seen.insert(key, ());
        let index = data.problems.len();
        let build_id = data.build_id.clone();
        let revision = data.revision;
        data.problems.push(BuildProblem {
            id: format!("{build_id}:{index}"),
            build_id,
            revision,
            ..problem
        });
    }
}

/// One line of MSBuild text. Unrecognized lines stay only in the raw log.
pub enum Line {
    Ignored,
    /// Looks like a diagnostic but its file cannot be resolved inside the workspace.
    Rejected,
    Problem(BuildProblem),
}

fn problem(
    severity: &'static str,
    code: Option<String>,
    message: String,
    path: String,
    position: Option<(u64, u64)>,
    range: Option<((u64, u64), (u64, u64))>,
    raw: &str,
) -> BuildProblem {
    let (severity_raw, severity_label) = match severity {
        "error" => (1, "error"),
        _ => (2, "warning"),
    };
    BuildProblem {
        id: String::new(),
        severity,
        severity_label,
        severity_raw,
        code,
        source: "dotnet",
        category: "unknown",
        message,
        path,
        position: position.map(|(line, character)| BuildPosition { line, character }),
        range: range.map(|(start, end)| BuildRange {
            start: BuildPosition {
                line: start.0,
                character: start.1,
            },
            end: BuildPosition {
                line: end.0,
                character: end.1,
            },
        }),
        build_id: String::new(),
        revision: 0,
        raw: raw.to_string(),
    }
}

/// Conservative parse of `path(line,column): error CS0246: message [project]`. Anything
/// that does not match exactly keeps no position and does not become a problem.
pub fn parse_line(root: &Path, line: &str) -> Line {
    let trimmed = line.trim_start();
    let Some((path_text, numbers, suffix)) = split_location(trimmed) else {
        return Line::Ignored;
    };
    let Some(severity) = severity_of(suffix) else {
        return Line::Ignored;
    };
    let rest = suffix[1..].trim_start();
    let rest = rest[severity.len()..].trim_start();
    let (code, message) = match rest.find(':') {
        Some(index) if plausible_code(&rest[..index]) => (
            Some(rest[..index].trim().to_string()),
            rest[index + 1..].trim().to_string(),
        ),
        _ => (None, rest.trim().to_string()),
    };
    let message = strip_project_suffix(&message);
    if message.is_empty() {
        return Line::Ignored;
    }
    let Some(path) = resolve_inside(root, path_text.trim()) else {
        return Line::Rejected;
    };
    let (position, range) = match numbers.as_slice() {
        [line, column] => (Some((line - 1, column - 1)), None),
        [line, column, end_line, end_column] => (
            Some((line - 1, column - 1)),
            Some(((line - 1, column - 1), (end_line - 1, end_column - 1))),
        ),
        _ => return Line::Ignored,
    };
    Line::Problem(problem(
        severity,
        code,
        message,
        relative_path(root, &path),
        position,
        range,
        trimmed,
    ))
}

/// Splits `path(line,column): error …` without assuming the path has no spaces or
/// parentheses: the last `(…)` that parses as 1-based positions wins.
fn split_location(line: &str) -> Option<(&str, Vec<u64>, &str)> {
    let mut candidate = None;
    for (index, _) in line.match_indices('(') {
        let Some(close) = line[index..].find(')') else {
            continue;
        };
        let close = index + close;
        let inside = &line[index + 1..close];
        let mut numbers = Vec::new();
        let mut ok = !inside.trim().is_empty();
        for part in inside.split(',') {
            match part.trim().parse::<u64>() {
                Ok(value) if value >= 1 => numbers.push(value),
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok || !matches!(numbers.len(), 2 | 4) {
            continue;
        }
        let suffix = &line[close + 1..];
        if severity_of(suffix).is_some() {
            candidate = Some((index, numbers, suffix));
            break;
        }
    }
    let (index, numbers, suffix) = candidate?;
    let path = &line[..index];
    if path.trim().is_empty() {
        return None;
    }
    Some((path, numbers, suffix))
}

fn severity_of(suffix: &str) -> Option<&'static str> {
    let rest = suffix.strip_prefix(":")?.trim_start();
    let lower = rest.to_ascii_lowercase();
    for severity in ["error", "warning"] {
        if let Some(after) = lower.strip_prefix(severity) {
            if after.is_empty() || after.starts_with(char::is_whitespace) || after.starts_with(':')
            {
                return Some(severity);
            }
        }
    }
    None
}

fn plausible_code(text: &str) -> bool {
    let code = text.trim();
    !code.is_empty()
        && code.len() <= 40
        && code
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'))
}

/// MSBuild appends `[project.csproj]` to diagnostics; that suffix is not part of the text.
fn strip_project_suffix(message: &str) -> String {
    let trimmed = message.trim_end();
    let Some(start) = trimmed.rfind(" [") else {
        return trimmed.to_string();
    };
    if !trimmed.ends_with(']') {
        return trimmed.to_string();
    }
    let bracket = &trimmed[start + 2..trimmed.len() - 1];
    if bracket.ends_with(".csproj") || bracket.ends_with(".sln") {
        trimmed[..start].trim_end().to_string()
    } else {
        trimmed.to_string()
    }
}

/// Workspace-relative or absolute path, canonicalized when it exists. A path that resolves
/// outside the workspace is rejected instead of being presented as navigable.
fn resolve_inside(root: &Path, raw: &str) -> Option<PathBuf> {
    if raw.is_empty() || raw.contains("://") {
        return None;
    }
    let candidate = Path::new(raw);
    let candidate = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };
    let normalized = normalize(&candidate);
    if !normalized.starts_with(root) {
        return None;
    }
    match std::fs::canonicalize(&normalized) {
        Ok(path) if path.starts_with(root) => Some(path),
        Ok(_) => None,
        Err(_) => Some(normalized),
    }
}

fn normalize(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                result.pop();
            }
            Component::CurDir => {}
            other => result.push(other),
        }
    }
    result
}

/// One console project per build; anything else needs an explicit reason, never a guess.
pub fn select_startup<'a>(
    projects: &'a [EvaluatedProject],
    requested: Option<&str>,
) -> HostResult<&'a EvaluatedProject> {
    if let Some(requested) = requested {
        let normalized = requested
            .replace('\\', "/")
            .trim_start_matches("./")
            .to_string();
        let Some(project) = projects.iter().find(|project| project.path == normalized) else {
            return Err(HostError::new(
                "invalid_state",
                format!("{requested} is not a project of the current evaluation."),
            ));
        };
        if project.kind != "console" {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "{} is a {} project; only a console executable project can be the build startup project.",
                    project.name, project.kind
                ),
            ));
        }
        if project.state != "ok" || project.target_path.is_none() {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "{} has no evaluated TargetPath; re-evaluate the project model before building.",
                    project.name
                ),
            ));
        }
        return Ok(project);
    }

    let candidates: Vec<&EvaluatedProject> = projects
        .iter()
        .filter(|project| project.kind == "console" && project.state == "ok")
        .filter(|project| project.target_path.is_some())
        .collect();
    match candidates.as_slice() {
        [only] => Ok(only),
        [] => Err(HostError::new(
            "invalid_state",
            "No console executable project is available in the current evaluation; select a solution with an executable project and re-evaluate.",
        )),
        many => Err(HostError::with_details(
            "invalid_state",
            format!(
                "This workspace has {} console projects ({}); choose the startup project explicitly.",
                many.len(),
                many.iter()
                    .map(|project| project.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            json!({
                "reason": "ambiguous",
                "candidates": many
                    .iter()
                    .map(|project| json!({ "name": project.name, "path": project.path }))
                    .collect::<Vec<_>>(),
            }),
        )),
    }
}

/// Streams one pipe of the build process into the record, line by line. Drains even when
/// the record is sealed so the child cannot block on a full pipe.
pub fn pump(reader: impl std::io::Read, build: std::sync::Arc<Build>, stderr: bool) {
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
            build.append_line(text, stderr);
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

    fn root() -> PathBuf {
        PathBuf::from("/work/Order Desk")
    }

    fn accepted(line: &str) -> BuildProblem {
        match parse_line(&root(), line) {
            Line::Problem(problem) => problem,
            Line::Rejected => panic!("line was rejected: {line}"),
            Line::Ignored => panic!("line was not recognized: {line}"),
        }
    }

    #[test]
    fn parses_error_warning_and_ranges_without_inventing_positions() {
        let error = accepted(
            "/work/Order Desk/src/OrderDesk.Cli/Program.cs(5,17): error CS0246: The type or namespace name 'OrderReport' could not be found [/work/Order Desk/src/OrderDesk.Cli/OrderDesk.Cli.csproj]",
        );
        assert_eq!(error.severity, "error");
        assert_eq!(error.severity_label, "error");
        assert_eq!(error.severity_raw, 1);
        assert_eq!(error.code.as_deref(), Some("CS0246"));
        assert_eq!(error.source, "dotnet");
        assert_eq!(error.category, "unknown");
        assert_eq!(error.path, "src/OrderDesk.Cli/Program.cs");
        assert_eq!(
            error.position,
            Some(BuildPosition {
                line: 4,
                character: 16
            })
        );
        assert!(error.range.is_none());
        assert!(!error.message.contains("["));
        assert!(error.raw.contains("CS0246"));

        let warning =
            accepted("src/OrderDesk.Cli/Program.cs(3,7): warning CS0168: unused variable");
        assert_eq!(warning.severity_label, "warning");
        assert_eq!(warning.severity_raw, 2);
        assert_eq!(
            warning.position,
            Some(BuildPosition {
                line: 2,
                character: 6
            })
        );
        assert_eq!(warning.path, "src/OrderDesk.Cli/Program.cs");

        let ranged = accepted(
            "/work/Order Desk/Program.cs(1,2,3,4): error CS1002: ; expected [/work/Order Desk/App.csproj]",
        );
        assert_eq!(
            ranged.position,
            Some(BuildPosition {
                line: 0,
                character: 1
            })
        );
        assert_eq!(
            ranged.range,
            Some(BuildRange {
                start: BuildPosition {
                    line: 0,
                    character: 1
                },
                end: BuildPosition {
                    line: 2,
                    character: 3
                },
            })
        );
    }

    #[test]
    fn unrecognized_or_positionless_lines_never_become_problems() {
        for line in [
            "  OrderDesk.Cli -> /work/Order Desk/src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll",
            "",
            "Build succeeded.",
            "error CS0246: no path and no position",
            "/work/Order Desk/Program.cs(abc,1): error CS0246: nope",
            "/work/Order Desk/Program.cs(0,1): error CS0246: zero is not a MSBuild position",
            "/work/Order Desk/Program.cs(5): error CS0246: one number is not a position",
            "/work/Order Desk/Program.cs(5,1): note XY0001: not a build severity",
        ] {
            assert!(
                matches!(parse_line(&root(), line), Line::Ignored),
                "unexpected parse: {line}"
            );
        }
    }

    #[test]
    fn outside_workspace_diagnostics_are_rejected_and_keep_the_raw_line_only() {
        for line in [
            "/usr/local/share/dotnet/sdk/8.0.408/Sdks/Microsoft.NET.Sdk/targets/File.cs(1,1): warning MSB3277: outside",
            "/work/Other/Program.cs(1,1): error CS0246: outside",
            "../Outside.cs(1,1): error CS0246: outside",
            "https://example.test/Program.cs(1,1): error CS0246: not a file",
        ] {
            assert!(matches!(parse_line(&root(), line), Line::Rejected), "{line}");
        }
    }

    fn evaluated(
        name: &str,
        path: &str,
        kind: &'static str,
        target: Option<&str>,
    ) -> EvaluatedProject {
        EvaluatedProject {
            name: name.to_string(),
            path: path.to_string(),
            folder: None,
            state: "ok",
            kind,
            tfms: vec!["net8.0".to_string()],
            evaluated_tfm: Some("net8.0".to_string()),
            target_path: target.map(str::to_string),
            output_type: None,
            project_refs: Vec::new(),
            package_refs: Vec::new(),
            sources: Vec::new(),
            error: None,
            log: String::new(),
            exit: Some(0),
            restored: "restored",
            restored_reason: None,
            note: None,
        }
    }

    #[test]
    fn startup_selection_requires_one_console_project_and_rejects_libraries() {
        let projects = vec![
            evaluated(
                "App",
                "src/App/App.csproj",
                "console",
                Some("/work/App.dll"),
            ),
            evaluated(
                "Core",
                "src/Core/Core.csproj",
                "library",
                Some("/work/Core.dll"),
            ),
            evaluated(
                "Tests",
                "tests/Tests.csproj",
                "test",
                Some("/work/Tests.dll"),
            ),
        ];
        let selected = select_startup(&projects, None).expect("one console project");
        assert_eq!(selected.name, "App");
        for (name, path) in [
            ("Core", "src/Core/Core.csproj"),
            ("Tests", "tests/Tests.csproj"),
        ] {
            let error = select_startup(&projects, Some(path))
                .expect_err("only a console project may be the startup project");
            assert_eq!(error.code, "invalid_state");
            assert!(
                error.message.contains("only a console executable project"),
                "{name}: {}",
                error.message
            );
        }

        let with_two = vec![
            evaluated(
                "App",
                "src/App/App.csproj",
                "console",
                Some("/work/App.dll"),
            ),
            evaluated(
                "Tool",
                "src/Tool/Tool.csproj",
                "console",
                Some("/work/Tool.dll"),
            ),
        ];
        let error = select_startup(&with_two, None).expect_err("ambiguity needs a choice");
        assert_eq!(error.code, "invalid_state");
        let details = error.details.unwrap();
        assert_eq!(details["reason"], "ambiguous");
        assert_eq!(details["candidates"].as_array().unwrap().len(), 2);
        assert_eq!(
            select_startup(&with_two, Some("src/Tool/Tool.csproj"))
                .unwrap()
                .name,
            "Tool"
        );
    }

    #[test]
    fn a_capped_build_says_so_and_unknown_exit_is_never_success() {
        let temp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(temp.path()).unwrap();
        let build = Build::begin(Start {
            revision: 1,
            build_id: "build:1:1".to_string(),
            project_name: "App".to_string(),
            project_path: "App.csproj".to_string(),
            project_kind: "console",
            target_path: Some("/work/App.dll".to_string()),
            configuration: "Debug".to_string(),
            target_framework: None,
            command: vec!["dotnet".to_string(), "build".to_string()],
            cwd: root.clone(),
            sdk_root: root.clone(),
            sdk_version: "8.0.408".to_string(),
            workspace_id: 1,
            context_id: 1,
            root: root.clone(),
            saved_note: Some("Built saved content.".to_string()),
        });
        for index in 0..(MAX_PROBLEMS + 5) {
            build.append_line(
                &format!(
                    "App.cs({},2): error CS{index:04}: failure {index}",
                    index + 1
                ),
                false,
            );
        }
        build.append_line("warning CS0168: stderr without a position", true);
        let view = build.view();
        assert_eq!(view.problems.len(), MAX_PROBLEMS);
        assert!(view.problems_truncated);
        assert_eq!(view.problems[0].id, "build:1:1:0");
        assert_eq!(view.problems[0].build_id, "build:1:1");
        assert!(view.log.contains("[stderr] warning CS0168"));
        assert!(view.saved_note.as_deref().unwrap().contains("saved"));

        let unknown = ProcessStatus {
            state: "exited",
            exit: None,
            termination: None,
            awaiting_decision: false,
        };
        assert_eq!(build.complete(&unknown), "failed");
        assert_eq!(build.view().exit, None);
        assert!(!build.view().log.is_empty());
    }

    #[test]
    fn crlf_lines_are_pumped_without_a_carriage_return_in_the_log() {
        let temp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(temp.path()).unwrap();
        let build = std::sync::Arc::new(Build::begin(Start {
            revision: 1,
            build_id: "build:1:crlf".to_string(),
            project_name: "App".to_string(),
            project_path: "App.csproj".to_string(),
            project_kind: "console",
            target_path: None,
            configuration: "Debug".to_string(),
            target_framework: None,
            command: Vec::new(),
            cwd: root.clone(),
            sdk_root: root.clone(),
            sdk_version: "8.0.408".to_string(),
            workspace_id: 1,
            context_id: 1,
            root: root.clone(),
            saved_note: None,
        }));
        pump(
            std::io::Cursor::new(
                b"App.cs(2,3): error CS0002: crlf line\r\nplain line\r\n".to_vec(),
            ),
            build.clone(),
            false,
        );
        let view = build.view();
        assert_eq!(view.problems.len(), 1);
        assert_eq!(
            view.problems[0].position,
            Some(BuildPosition {
                line: 1,
                character: 2
            })
        );
        assert!(!view.log.contains('\r'));
        assert!(view.log.contains("plain line"));
    }

    #[test]
    fn cancel_is_requested_not_observed_and_a_sealed_record_drops_late_output() {
        let build = Build::begin(Start {
            revision: 2,
            build_id: "build:1:2".to_string(),
            project_name: "App".to_string(),
            project_path: "App.csproj".to_string(),
            project_kind: "console",
            target_path: None,
            configuration: "Debug".to_string(),
            target_framework: None,
            command: Vec::new(),
            cwd: PathBuf::from("/work"),
            sdk_root: PathBuf::from("/sdk"),
            sdk_version: "8.0.408".to_string(),
            workspace_id: 1,
            context_id: 1,
            root: PathBuf::from("/work"),
            saved_note: None,
        });
        build.append_line("App.cs(1,1): error CS0001: before cancel", false);
        build.request_cancel();
        assert_eq!(build.view().state, "running");
        assert_eq!(build.view().exit, None);

        let status = ProcessStatus {
            state: "exited",
            exit: Some(143),
            termination: Some(Termination {
                requested: "SIGTERM".to_string(),
                observed: "exited".to_string(),
                exit: Some(143),
                killed_after_grace: false,
            }),
            awaiting_decision: false,
        };
        assert_eq!(build.complete(&status), "cancelled");
        let view = build.view();
        assert_eq!(view.termination.as_ref().unwrap().observed, "exited");
        assert_eq!(view.problems.len(), 1);

        build.seal(
            "Trust was revoked; this build is not current.",
            Some(&status),
        );
        let finished_at = build.view().finished_at;
        build.append_line("App.cs(2,1): error CS0002: late line", false);
        let sealed = build.view();
        assert!(sealed.sealed);
        assert!(sealed.stale);
        assert_eq!(sealed.finished_at, finished_at);
        assert_eq!(sealed.problems.len(), 1);
        assert!(!sealed.log.contains("late line"));
    }
}
