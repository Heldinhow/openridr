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
use crate::scope::Scope;
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
    pub evaluation_revision: u64,
    pub sources: Vec<SourceHash>,
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
    /// Directories this workspace may touch: the root plus the projects the solution names.
    pub scope: Scope,
    pub saved_note: Option<String>,
    /// `true` when the command built the workspace .sln instead of one project.
    pub solution: bool,
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

/// One navigable `path(line,column)` span in the displayed build log. `line` is the zero-based
/// index of the line in `BuildView.log`; `start`/`end` are zero-based character columns of the
/// span, `end` exclusive. Only paths that passed the workspace containment check become links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildLogLink {
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub path: String,
    pub position: BuildPosition,
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
    pub log_links: Vec<BuildLogLink>,
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
    pub solution: bool,
}

struct Data {
    revision: u64,
    state: &'static str,
    build_id: String,
    evaluation_revision: u64,
    /// Saved source files and their content hashes when this build started. Run uses them
    /// to refuse launching a build whose sources changed afterwards.
    sources: Vec<SourceHash>,
    project: Option<BuildProject>,
    configuration: String,
    target_framework: Option<String>,
    command: Vec<String>,
    cwd: String,
    sdk_root: Option<String>,
    sdk_version: Option<String>,
    workspace_id: u64,
    context_id: u64,
    scope: Scope,
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
    solution: bool,
    /// Positions of accepted problems, used only to avoid printing the same MSBuild
    /// diagnostic twice (inline and again in the failure summary).
    seen: HashMap<String, ()>,
    /// Location spans with `line` as the absolute push index; `view()` rebases them onto the
    /// retained log, which can drop earlier lines and prepends a truncation notice.
    links: Vec<BuildLogLink>,
    lines_pushed: usize,
}

pub struct Build {
    data: Mutex<Data>,
}

/// One saved source as it was read when the build started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceHash {
    pub path: String,
    pub hash: Option<String>,
}

/// Hashes the saved sources of the startup project and its referenced projects at build
/// start. A source that cannot be read keeps `None`; the gate compares it with the current
/// state, so a file absent at both moments counts as unchanged.
pub fn capture_sources(
    root: &Path,
    startup: &str,
    projects: &[EvaluatedProject],
) -> Vec<SourceHash> {
    let by_path: HashMap<&str, &EvaluatedProject> = projects
        .iter()
        .map(|project| (project.path.as_str(), project))
        .collect();
    let mut seen = std::collections::HashSet::new();
    let mut queue = vec![startup.to_string()];
    let mut sources: Vec<SourceHash> = Vec::new();
    while let Some(path) = queue.pop() {
        if !seen.insert(path.clone()) {
            continue;
        }
        let Some(project) = by_path.get(path.as_str()) else {
            continue;
        };
        for source in &project.sources {
            sources.push(SourceHash {
                path: source.clone(),
                hash: hash_file(&root.join(source)),
            });
        }
        for reference in &project.project_refs {
            queue.push(reference.path.clone());
        }
    }
    sources.sort_by(|left, right| left.path.cmp(&right.path));
    sources.dedup_by(|left, right| left.path == right.path);
    sources
}

/// Hashes every evaluated project's sources for a solution build, so Run/Debug can refuse a
/// solution build whose saved content changed afterwards.
pub fn capture_all_sources(root: &Path, projects: &[EvaluatedProject]) -> Vec<SourceHash> {
    let mut sources: Vec<SourceHash> = Vec::new();
    for project in projects {
        for source in &project.sources {
            sources.push(SourceHash {
                path: source.clone(),
                hash: hash_file(&root.join(source)),
            });
        }
    }
    sources.sort_by(|left, right| left.path.cmp(&right.path));
    sources.dedup_by(|left, right| left.path == right.path);
    sources
}

fn hash_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    Some(format!("fnv1a64:{hash:016x}"))
}

impl Build {
    pub fn idle() -> Self {
        Self {
            data: Mutex::new(Data {
                revision: 0,
                state: "idle",
                build_id: String::new(),
                evaluation_revision: 0,
                sources: Vec::new(),
                project: None,
                configuration: String::new(),
                target_framework: None,
                command: Vec::new(),
                cwd: String::new(),
                sdk_root: None,
                sdk_version: None,
                workspace_id: 0,
                context_id: 0,
                scope: Scope::default(),
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
                solution: false,
                seen: HashMap::new(),
                links: Vec::new(),
                lines_pushed: 0,
            }),
        }
    }

    pub fn begin(start: Start) -> Self {
        Self {
            data: Mutex::new(Data {
                revision: start.revision,
                state: "running",
                build_id: start.build_id,
                evaluation_revision: start.evaluation_revision,
                sources: start.sources,
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
                scope: start.scope,
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
                solution: start.solution,
                seen: HashMap::new(),
                links: Vec::new(),
                lines_pushed: 0,
            }),
        }
    }

    pub fn view(&self) -> BuildView {
        let data = self.data.lock().unwrap();
        let (log, log_truncated) = data.log.text();
        // `text()` prepends one truncation notice line, so retained lines shift by one.
        let dropped = data.log.dropped();
        let notice = usize::from(dropped > 0);
        let log_links = data
            .links
            .iter()
            .filter(|link| link.line >= dropped)
            .map(|link| BuildLogLink {
                line: link.line - dropped + notice,
                ..link.clone()
            })
            .collect();
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
            log_links,
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
            solution: data.solution,
        }
    }

    pub fn note_process(&self, id: u64) {
        let mut data = self.data.lock().unwrap();
        if data.sealed {
            return;
        }
        data.process_id = Some(id);
    }

    pub fn evaluation_revision(&self) -> u64 {
        self.data.lock().unwrap().evaluation_revision
    }

    /// Saved sources whose content no longer matches the build start, by workspace path.
    /// A source missing then and now is unchanged; one deleted after the build still counts.
    pub fn changed_sources(&self, root: &Path) -> Vec<String> {
        let data = self.data.lock().unwrap();
        data.sources
            .iter()
            .filter(|source| source.hash != hash_file(&root.join(&source.path)))
            .map(|source| source.path.clone())
            .collect()
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
        data.lines_pushed += 1;
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
        let parsed = parse_line(&data.scope, line);
        let link = match &parsed {
            Line::Problem(problem) => problem.position.clone().and_then(|position| {
                location_columns(line, stderr).map(|(start, end)| BuildLogLink {
                    // Absolute push index; `view()` rebases it onto the displayed log.
                    line: data.lines_pushed,
                    start,
                    end,
                    path: problem.path.clone(),
                    position,
                })
            }),
            _ => None,
        };
        let shown = if stderr {
            format!("[stderr] {line}")
        } else {
            line.to_string()
        };
        data.log.push(shown);
        data.lines_pushed += 1;
        if let Some(link) = link {
            // The log is bounded; spans whose line was already dropped can never be shown.
            let dropped = data.log.dropped();
            data.links.retain(|link| link.line >= dropped);
            data.links.push(link);
        }
        let Line::Problem(problem) = parsed else {
            return;
        };
        let build_id = data.build_id.clone();
        let revision = data.revision;
        let Data {
            problems,
            seen,
            problems_truncated,
            ..
        } = &mut *data;
        record_problem(
            problem,
            &build_id,
            revision,
            problems,
            seen,
            problems_truncated,
        );
    }
}

/// Records one parsed diagnostic in a build or discovery list: dedupe by
/// path/position/code/message, assign the `id`/`build_id`/`revision`, and stop listing at
/// `MAX_PROBLEMS`. The raw log is never affected by the cap.
pub fn record_problem(
    problem: BuildProblem,
    build_id: &str,
    revision: u64,
    problems: &mut Vec<BuildProblem>,
    seen: &mut HashMap<String, ()>,
    problems_truncated: &mut bool,
) {
    let key = format!(
        "{}:{:?}:{:?}:{}",
        problem.path, problem.position, problem.code, problem.message
    );
    if seen.contains_key(&key) {
        return;
    }
    if problems.len() >= MAX_PROBLEMS {
        *problems_truncated = true;
        return;
    }
    seen.insert(key, ());
    let index = problems.len();
    problems.push(BuildProblem {
        id: format!("{build_id}:{index}"),
        build_id: build_id.to_string(),
        revision,
        ..problem
    });
}

/// One line of MSBuild text. Unrecognized lines stay only in the raw log.
// A parsed line is short-lived (one per log line) and matching stays plain; boxing `Problem` would
// buy nothing here.
#[allow(clippy::large_enum_variant)]
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
pub fn parse_line(scope: &Scope, line: &str) -> Line {
    let trimmed = line.trim_start();
    let Some((path_text, numbers, suffix, _)) = split_location(trimmed) else {
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
    let Some(path) = resolve_inside(scope, path_text.trim()) else {
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
        relative_path(scope.root().unwrap_or(Path::new("")), &path),
        position,
        range,
        trimmed,
    ))
}

/// Zero-based character columns of the whole `path(line,column)` text on the displayed line,
/// `end` exclusive. The stderr prefix is part of what the user sees, so it is counted here.
fn location_columns(line: &str, stderr: bool) -> Option<(usize, usize)> {
    let trimmed = line.trim_start();
    let (_, _, _, end) = split_location(trimmed)?;
    let prefix = if stderr {
        "[stderr] ".chars().count()
    } else {
        0
    };
    let start = prefix + line[..line.len() - trimmed.len()].chars().count();
    Some((start, start + trimmed[..end].chars().count()))
}

/// Splits `path(line,column): error …` without assuming the path has no spaces or
/// parentheses: the last `(…)` that parses as 1-based positions wins. The fourth value is the
/// byte index just past the closing `)`.
fn split_location(line: &str) -> Option<(&str, Vec<u64>, &str, usize)> {
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
            candidate = Some((index, numbers, suffix, close + 1));
            break;
        }
    }
    let (index, numbers, suffix, end) = candidate?;
    let path = &line[..index];
    if path.trim().is_empty() {
        return None;
    }
    Some((path, numbers, suffix, end))
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
    if bracket.ends_with(".csproj") || bracket.ends_with(".sln") || bracket.ends_with(".slnx") {
        trimmed[..start].trim_end().to_string()
    } else {
        trimmed.to_string()
    }
}

/// Workspace-relative or absolute path, canonicalized when it exists. A path that resolves outside
/// the workspace scope is rejected instead of being presented as navigable. A relative path is read
/// against the root and refused by shape when it escapes; an absolute path (how MSBuild reports a
/// project the solution names elsewhere) is decided by scope containment.
fn resolve_inside(scope: &Scope, raw: &str) -> Option<PathBuf> {
    if raw.is_empty() || raw.contains("://") {
        return None;
    }
    let candidate = Path::new(raw);
    let absolute = candidate.is_absolute();
    let candidate = if absolute {
        candidate.to_path_buf()
    } else {
        scope.root()?.join(candidate)
    };
    let normalized = normalize(&candidate);
    if !absolute && !normalized.starts_with(scope.root()?) {
        return None;
    }
    match std::fs::canonicalize(&normalized) {
        Ok(path) if scope.contains(&path) => Some(path),
        Ok(_) => None,
        Err(_) => scope.contains(&normalized).then_some(normalized),
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

/// One console project per build; an explicit request may also name an evaluated library or
/// test project, matching the tree menu's build gate.
pub fn select_startup<'a>(
    projects: &'a [EvaluatedProject],
    requested: Option<&str>,
) -> HostResult<&'a EvaluatedProject> {
    select_kinds(
        projects,
        requested,
        "build",
        &["console", "library", "test"],
        "console executable, library or test",
    )
}

/// Same selection for another consumer (run, debug), so its refusal names what was refused.
pub fn select_startup_for<'a>(
    projects: &'a [EvaluatedProject],
    requested: Option<&str>,
    consumer: &str,
) -> HostResult<&'a EvaluatedProject> {
    select_kinds(
        projects,
        requested,
        consumer,
        &["console"],
        "console executable",
    )
}

fn select_kinds<'a>(
    projects: &'a [EvaluatedProject],
    requested: Option<&str>,
    consumer: &str,
    kinds: &[&str],
    allowed: &str,
) -> HostResult<&'a EvaluatedProject> {
    if let Some(requested) = requested {
        // The model names every project by its portable path; a caller may send the canonical
        // absolute one (`\\?\C:\...` on Windows), so both forms must match.
        let normalized =
            crate::fsutil::portable_path(Path::new(requested.trim_start_matches("./")));
        let Some(project) = projects.iter().find(|project| project.path == normalized) else {
            return Err(HostError::new(
                "invalid_state",
                format!("{requested} is not a project of the current evaluation."),
            ));
        };
        if !kinds.contains(&project.kind) {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "{} is a {} project; only a {allowed} project can be the {consumer} startup project.",
                    project.name, project.kind
                ),
            ));
        }
        if project.state != "ok" || project.target_path.is_none() {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "{} has no evaluated TargetPath; re-evaluate the project model before using it.",
                    project.name
                ),
            ));
        }
        return Ok(project);
    }

    let candidates: Vec<&EvaluatedProject> = projects
        .iter()
        .filter(|project| project.kind == "console" && project.state == "ok")
        .filter(|project| project.assembly_path.is_some())
        .collect();
    match candidates.as_slice() {
        [only] => Ok(only),
        [] => Err(HostError::new(
            "invalid_state",
            format!(
                "No console executable project is available in the current evaluation; select a solution with an executable project and re-evaluate before {consumer}."
            ),
        )),
        many => Err(HostError::with_details(
            "invalid_state",
            format!(
                "This workspace has {} console projects ({}); choose the {consumer} startup project explicitly.",
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
    use crate::logbuf::MAX_LOG_LINES;
    use std::fs;

    fn root() -> PathBuf {
        PathBuf::from("/work/Order Desk")
    }

    fn accepted(line: &str) -> BuildProblem {
        match parse_line(&Scope::new(&root()), line) {
            Line::Problem(problem) => problem,
            Line::Rejected => panic!("line was rejected: {line}"),
            Line::Ignored => panic!("line was not recognized: {line}"),
        }
    }

    #[test]
    fn msbuild_project_suffix_is_stripped_for_sln_slnx_and_csproj() {
        assert_eq!(strip_project_suffix("bad [/w/App.csproj]"), "bad");
        assert_eq!(strip_project_suffix("bad [/w/App.sln]"), "bad");
        assert_eq!(strip_project_suffix("bad [/w/Modern.slnx]"), "bad");
        assert_eq!(
            strip_project_suffix("kept [/w/notes.txt]"),
            "kept [/w/notes.txt]"
        );
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
                matches!(parse_line(&Scope::new(&root()), line), Line::Ignored),
                "unexpected parse: {line}"
            );
        }
    }

    #[test]
    fn a_diagnostic_in_a_project_outside_the_root_is_navigable_inside_the_scope() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("app");
        fs::create_dir_all(&root).unwrap();
        let lib = temp.path().join("lib");
        fs::create_dir_all(&lib).unwrap();
        fs::write(lib.join("Lib.csproj"), "<Project />\n").unwrap();
        let root = fs::canonicalize(&root).unwrap();
        let lib = fs::canonicalize(&lib).unwrap();
        let scope = crate::scope::resolve(&root, ["../lib/Lib.csproj".to_string()]).scope;
        let source = lib.join("Greeting.cs");
        let line = format!(
            "{}(3,5): error CS1002: ; expected [lib/Lib.csproj]",
            source.display()
        );

        match parse_line(&scope, &line) {
            Line::Problem(problem) => {
                assert_eq!(problem.path, crate::fsutil::portable_path(&source));
                assert_eq!(problem.code.as_deref(), Some("CS1002"));
                let position = problem.position.expect("a position");
                assert_eq!(position.line, 2);
                assert_eq!(position.character, 4);
            }
            _ => panic!("outside project diagnostic was not accepted: {line}"),
        }

        // The same line stays text when the scope does not include that directory, and a relative
        // path that escapes the root is still refused by shape.
        assert!(matches!(
            parse_line(&Scope::new(&root), &line),
            Line::Rejected
        ));
        assert!(matches!(
            parse_line(&scope, "../lib/Greeting.cs(1,1): error CS1002: nope"),
            Line::Rejected
        ));
    }

    #[test]
    fn outside_workspace_diagnostics_are_rejected_and_keep_the_raw_line_only() {
        for line in [
            "/usr/local/share/dotnet/sdk/8.0.408/Sdks/Microsoft.NET.Sdk/targets/File.cs(1,1): warning MSB3277: outside",
            "/work/Other/Program.cs(1,1): error CS0246: outside",
            "../Outside.cs(1,1): error CS0246: outside",
            "https://example.test/Program.cs(1,1): error CS0246: not a file",
        ] {
            assert!(matches!(parse_line(&Scope::new(&root()), line), Line::Rejected), "{line}");
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
            assembly_path: target.map(str::to_string),
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
    fn startup_selection_defaults_to_console_but_build_accepts_any_evaluated_project() {
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
            evaluated(
                "Tool",
                "tools/Tool/Tool.csproj",
                "unknown",
                Some("/work/Tool.dll"),
            ),
        ];
        let selected = select_startup(&projects, None).expect("one console project");
        assert_eq!(selected.name, "App");
        assert_eq!(
            select_startup(&projects, Some("src/Core/Core.csproj"))
                .expect("an explicit library build")
                .name,
            "Core"
        );
        assert_eq!(
            select_startup(&projects, Some("tests/Tests.csproj"))
                .expect("an explicit test-project build")
                .name,
            "Tests"
        );
        let error = select_startup(&projects, Some("tools/Tool/Tool.csproj"))
            .expect_err("an unknown project kind is not built through build.start");
        assert_eq!(error.code, "invalid_state");
        assert!(
            error
                .message
                .contains("only a console executable, library or test project"),
            "{}",
            error.message
        );
        let error = select_startup_for(&projects, Some("src/Core/Core.csproj"), "run")
            .expect_err("run still refuses a library");
        assert_eq!(error.code, "invalid_state");
        assert!(
            error.message.contains("only a console executable project"),
            "{}",
            error.message
        );

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
            evaluation_revision: 2,
            sources: Vec::new(),
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
            scope: Scope::new(&root),
            saved_note: Some("Built saved content.".to_string()),
            solution: false,
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
            signal: None,
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
            evaluation_revision: 2,
            sources: Vec::new(),
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
            scope: Scope::new(&root),
            saved_note: None,
            solution: false,
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

    fn build_at(root: &Path) -> Build {
        Build::begin(Start {
            revision: 1,
            build_id: "build:1:links".to_string(),
            evaluation_revision: 1,
            sources: Vec::new(),
            project_name: "App".to_string(),
            project_path: "App.csproj".to_string(),
            project_kind: "console",
            target_path: None,
            configuration: "Debug".to_string(),
            target_framework: None,
            command: Vec::new(),
            cwd: root.to_path_buf(),
            sdk_root: root.to_path_buf(),
            sdk_version: "8.0.408".to_string(),
            workspace_id: 1,
            context_id: 1,
            scope: Scope::new(root),
            saved_note: None,
            solution: false,
        })
    }

    fn span(view: &BuildView, link: &BuildLogLink) -> String {
        let line = view
            .log
            .split('\n')
            .nth(link.line)
            .expect("link line is in the displayed log");
        line.chars()
            .skip(link.start)
            .take(link.end - link.start)
            .collect()
    }

    #[test]
    fn log_links_cover_the_location_on_the_displayed_line_including_stderr() {
        let temp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(temp.path()).unwrap();
        let build = build_at(&root);
        build.append_line("src/App/Program.cs(5,17): error CS0246: missing", false);
        build.append_line("src/App/Program.cs(7,3): warning CS0168: stderr", true);
        build.append_line("Build succeeded.", false);
        build.append_line(
            "/usr/local/share/dotnet/File.cs(1,1): error MSB3277: outside",
            false,
        );
        let view = build.view();
        assert_eq!(
            view.log_links.len(),
            2,
            "outside and unknown lines stay text"
        );

        let stdout = &view.log_links[0];
        assert_eq!(stdout.line, 0);
        assert_eq!(stdout.start, 0);
        assert_eq!(stdout.path, "src/App/Program.cs");
        assert_eq!(
            stdout.position,
            BuildPosition {
                line: 4,
                character: 16
            }
        );
        assert_eq!(span(&view, stdout), "src/App/Program.cs(5,17)");

        let stderr = &view.log_links[1];
        assert_eq!(stderr.line, 1);
        assert_eq!(stderr.start, "[stderr] ".chars().count());
        assert_eq!(
            stderr.position,
            BuildPosition {
                line: 6,
                character: 2
            }
        );
        assert_eq!(span(&view, stderr), "src/App/Program.cs(7,3)");
    }

    #[test]
    fn a_truncated_log_rebases_links_and_drops_links_for_discarded_lines() {
        let temp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(temp.path()).unwrap();
        let build = build_at(&root);
        for index in 0..(MAX_LOG_LINES + 3) {
            build.append_line(&format!("padding line {index}"), false);
        }
        build.append_line("src/App/Program.cs(2,3): error CS0103: lost", false);
        let view = build.view();
        assert!(view.log_truncated);
        assert_eq!(view.log_links.len(), 1);
        let link = &view.log_links[0];
        assert!(link.line > 0, "the truncation notice shifts the line");
        assert_eq!(
            view.log.split('\n').nth(link.line).unwrap(),
            "src/App/Program.cs(2,3): error CS0103: lost"
        );
        assert_eq!(span(&view, link), "src/App/Program.cs(2,3)");

        // The link's own line is gone once the buffer has dropped it.
        for index in 0..MAX_LOG_LINES {
            build.append_line(&format!("more {index}"), false);
        }
        assert!(build.view().log_links.is_empty());
    }

    #[test]
    fn captured_sources_detect_edits_and_deletes_but_not_a_file_that_never_existed() {
        let temp = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(temp.path()).unwrap();
        fs::create_dir_all(root.join("src/Cli")).unwrap();
        fs::create_dir_all(root.join("src/Core")).unwrap();
        fs::write(root.join("src/Cli/Program.cs"), "class Program {}\n").unwrap();
        fs::write(root.join("src/Core/Order.cs"), "class Order {}\n").unwrap();
        let mut cli = evaluated(
            "Cli",
            "src/Cli/Cli.csproj",
            "console",
            Some("/work/Cli.dll"),
        );
        cli.sources = vec!["src/Cli/Program.cs".to_string()];
        cli.project_refs = vec![crate::evaluation::ProjectRef {
            name: "Core".to_string(),
            path: "src/Core/Core.csproj".to_string(),
        }];
        let mut core = evaluated("Core", "src/Core/Core.csproj", "library", None);
        core.sources = vec![
            "src/Core/Order.cs".to_string(),
            "src/Core/Removed.cs".to_string(),
        ];
        let projects = [cli, core];
        let sources = capture_sources(&root, "src/Cli/Cli.csproj", &projects);
        assert_eq!(
            sources
                .iter()
                .map(|source| source.path.as_str())
                .collect::<Vec<_>>(),
            vec![
                "src/Cli/Program.cs",
                "src/Core/Order.cs",
                "src/Core/Removed.cs"
            ]
        );

        let build = Build::begin(Start {
            revision: 1,
            build_id: "build:1:1".to_string(),
            evaluation_revision: 1,
            sources,
            project_name: "Cli".to_string(),
            project_path: "src/Cli/Cli.csproj".to_string(),
            project_kind: "console",
            target_path: Some("/work/Cli.dll".to_string()),
            configuration: "Debug".to_string(),
            target_framework: None,
            command: vec!["dotnet".to_string(), "build".to_string()],
            cwd: root.clone(),
            sdk_root: root.clone(),
            sdk_version: "8.0.408".to_string(),
            workspace_id: 1,
            context_id: 1,
            scope: Scope::new(&root),
            saved_note: None,
            solution: false,
        });
        // Removed.cs was already missing when the build started and is still missing.
        assert!(build.changed_sources(&root).is_empty());

        fs::write(root.join("src/Core/Order.cs"), "class Order { int Id; }\n").unwrap();
        assert_eq!(build.changed_sources(&root), vec!["src/Core/Order.cs"]);

        // A source present at build start and deleted afterwards still needs the rebuild.
        fs::remove_file(root.join("src/Core/Order.cs")).unwrap();
        assert_eq!(build.changed_sources(&root), vec!["src/Core/Order.cs"]);

        // The implicit rebuild captures the deletion as absent, so the gate releases.
        let rebuilt = Build::begin(Start {
            revision: 2,
            build_id: "build:1:2".to_string(),
            evaluation_revision: 1,
            sources: capture_sources(&root, "src/Cli/Cli.csproj", &projects),
            project_name: "Cli".to_string(),
            project_path: "src/Cli/Cli.csproj".to_string(),
            project_kind: "console",
            target_path: Some("/work/Cli.dll".to_string()),
            configuration: "Debug".to_string(),
            target_framework: None,
            command: vec!["dotnet".to_string(), "build".to_string()],
            cwd: root.clone(),
            sdk_root: root.clone(),
            sdk_version: "8.0.408".to_string(),
            workspace_id: 1,
            context_id: 1,
            scope: Scope::new(&root),
            saved_note: None,
            solution: false,
        });
        assert!(rebuilt.changed_sources(&root).is_empty());
    }

    #[test]
    fn cancel_is_requested_not_observed_and_a_sealed_record_drops_late_output() {
        let build = Build::begin(Start {
            revision: 2,
            build_id: "build:1:2".to_string(),
            evaluation_revision: 2,
            sources: Vec::new(),
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
            scope: Scope::new(Path::new("/work")),
            saved_note: None,
            solution: false,
        });
        build.append_line("App.cs(1,1): error CS0001: before cancel", false);
        build.request_cancel();
        assert_eq!(build.view().state, "running");
        assert_eq!(build.view().exit, None);

        let status = ProcessStatus {
            state: "exited",
            exit: Some(143),
            signal: None,
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
