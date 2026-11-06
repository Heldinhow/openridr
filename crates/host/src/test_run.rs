//! Run all or exactly the selected discovered tests through VSTest, then read the real TRX.
//!
//! Run never restores and never builds: it requires the current discovery (whose
//! `--list-tests` already compiled the project), a current restore and the identified
//! discovery identities. The TRX logger is VSTest's built-in one, written under the app home,
//! never into the repository. Outcomes come from the TRX; an absent result stays unknown and
//! a non-zero exit with no test result is infrastructure, not a test failure.

use crate::logbuf::LogBuffer;
use crate::process::{ProcessStatus, Termination};
use crate::test_discovery::TestItem;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// The one built-in VSTest logger used; no package is added or installed.
pub const TRX_FILE: &str = "openridr.trx";
pub const MAX_RESULTS: usize = 2_000;
const MAX_TEXT_CHARS: usize = 16_000;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestPosition {
    /// Workspace-relative path, exactly as `file.read` returns it.
    pub path: String,
    /// 0-based, like every other editor position in the host.
    pub line: u64,
    pub character: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestResult {
    /// Discovery identity when the TestDefinitions FQN matches one; never inferred from a label.
    pub test_id: Option<String>,
    pub fully_qualified_name: String,
    pub display_name: String,
    pub outcome: &'static str,
    /// Exactly what the TRX said (`Passed`, `Failed`, `NotExecuted`, …).
    pub outcome_raw: String,
    pub duration_ms: Option<u64>,
    pub message: Option<String>,
    pub stack_trace: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub matched: bool,
    pub position: Option<TestPosition>,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub not_executed: usize,
    pub unknown: usize,
    pub matched: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunView {
    pub state: &'static str,
    pub project: Option<ProjectView>,
    pub configuration: String,
    pub target_framework: Option<String>,
    pub selection: &'static str,
    pub selected_test_ids: Vec<String>,
    pub requested_count: usize,
    pub command: Vec<String>,
    pub cwd: String,
    pub results_directory: Option<String>,
    pub trx_path: Option<String>,
    pub log_path: Option<String>,
    pub trx_state: &'static str,
    pub log: String,
    pub log_truncated: bool,
    pub exit: Option<i32>,
    pub termination: Option<Termination>,
    pub process_id: Option<u64>,
    pub awaiting_decision: bool,
    pub revision: u64,
    pub run_id: Option<String>,
    pub discovery_id: Option<String>,
    pub discovery_revision: u64,
    pub workspace_id: Option<u64>,
    pub context_id: Option<u64>,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub results: Vec<TestResult>,
    pub results_truncated: bool,
    pub counts: Counts,
    pub infrastructure_failure: bool,
    pub error: Option<String>,
    pub sealed: bool,
    pub stale: bool,
    pub stale_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub name: String,
    pub path: String,
}

pub struct Start {
    pub revision: u64,
    pub run_id: String,
    pub project_name: String,
    pub project_path: String,
    pub discovery_id: String,
    pub discovery_revision: u64,
    pub configuration: String,
    pub target_framework: Option<String>,
    pub selection: &'static str,
    pub selected_test_ids: Vec<String>,
    pub command: Vec<String>,
    pub cwd: String,
    pub workspace_root: PathBuf,
    pub results_directory: PathBuf,
    pub workspace_id: u64,
    pub context_id: u64,
    /// The identities of the discovery this run was validated against, used only to match TRX
    /// results back; a result whose FQN is absent stays unmatched.
    pub cases: Vec<TestItem>,
}

struct Data {
    revision: u64,
    state: &'static str,
    run_id: String,
    project: ProjectView,
    discovery_id: String,
    discovery_revision: u64,
    configuration: String,
    target_framework: Option<String>,
    selection: &'static str,
    selected_test_ids: Vec<String>,
    requested_count: usize,
    command: Vec<String>,
    cwd: String,
    workspace_root: PathBuf,
    results_directory: PathBuf,
    workspace_id: u64,
    context_id: u64,
    trx_state: &'static str,
    log: LogBuffer,
    exit: Option<i32>,
    termination: Option<Termination>,
    process_id: Option<u64>,
    awaiting_decision: bool,
    cancel_requested: bool,
    sealed: bool,
    stale: bool,
    stale_reason: Option<String>,
    started_at: Option<u64>,
    finished_at: Option<u64>,
    results: Vec<TestResult>,
    results_truncated: bool,
    infrastructure_failure: bool,
    error: Option<String>,
    cases: Vec<TestItem>,
}

pub struct TestRun {
    data: Mutex<Data>,
}

impl TestRun {
    pub fn idle() -> Self {
        Self {
            data: Mutex::new(Data {
                revision: 0,
                state: "idle",
                run_id: String::new(),
                project: ProjectView {
                    name: String::new(),
                    path: String::new(),
                },
                discovery_id: String::new(),
                discovery_revision: 0,
                configuration: String::new(),
                target_framework: None,
                selection: "all",
                selected_test_ids: Vec::new(),
                requested_count: 0,
                command: Vec::new(),
                cwd: String::new(),
                workspace_root: PathBuf::new(),
                results_directory: PathBuf::new(),
                workspace_id: 0,
                context_id: 0,
                trx_state: "none",
                log: LogBuffer::default(),
                exit: None,
                termination: None,
                process_id: None,
                awaiting_decision: false,
                cancel_requested: false,
                sealed: false,
                stale: false,
                stale_reason: None,
                started_at: None,
                finished_at: None,
                results: Vec::new(),
                results_truncated: false,
                infrastructure_failure: false,
                error: None,
                cases: Vec::new(),
            }),
        }
    }

    /// The idle view used when no test run has happened yet in this context.
    pub fn idle_view() -> RunView {
        Self::idle().view()
    }

    pub fn begin(start: Start) -> Self {
        let requested_count = if start.selection == "selected" {
            start.selected_test_ids.len()
        } else {
            start.cases.len()
        };
        Self {
            data: Mutex::new(Data {
                revision: start.revision,
                state: "running",
                run_id: start.run_id,
                project: ProjectView {
                    name: start.project_name,
                    path: start.project_path,
                },
                discovery_id: start.discovery_id,
                discovery_revision: start.discovery_revision,
                configuration: start.configuration,
                target_framework: start.target_framework,
                selection: start.selection,
                selected_test_ids: start.selected_test_ids,
                requested_count,
                command: start.command,
                cwd: start.cwd,
                workspace_root: start.workspace_root,
                results_directory: start.results_directory,
                workspace_id: start.workspace_id,
                context_id: start.context_id,
                trx_state: "none",
                log: LogBuffer::default(),
                exit: None,
                termination: None,
                process_id: None,
                awaiting_decision: false,
                cancel_requested: false,
                sealed: false,
                stale: false,
                stale_reason: None,
                started_at: Some(now_ms()),
                finished_at: None,
                results: Vec::new(),
                results_truncated: false,
                infrastructure_failure: false,
                error: None,
                cases: start.cases,
            }),
        }
    }

    pub fn view(&self) -> RunView {
        let data = self.data.lock().unwrap();
        let (log, log_truncated) = data.log.text();
        let counts = counts(&data.results);
        let trx_path = if data.revision > 0 {
            Some(
                data.results_directory
                    .join(TRX_FILE)
                    .to_string_lossy()
                    .to_string(),
            )
        } else {
            None
        };
        RunView {
            state: data.state,
            project: (data.revision > 0).then(|| data.project.clone()),
            configuration: data.configuration.clone(),
            target_framework: data.target_framework.clone(),
            selection: data.selection,
            selected_test_ids: data.selected_test_ids.clone(),
            requested_count: data.requested_count,
            command: data.command.clone(),
            cwd: data.cwd.clone(),
            results_directory: (data.revision > 0)
                .then(|| data.results_directory.to_string_lossy().to_string()),
            trx_path,
            log_path: (data.revision > 0).then(|| {
                data.results_directory
                    .join("run.log")
                    .to_string_lossy()
                    .to_string()
            }),
            trx_state: data.trx_state,
            log,
            log_truncated,
            exit: data.exit,
            termination: data.termination.clone(),
            process_id: data.process_id,
            awaiting_decision: data.awaiting_decision,
            revision: data.revision,
            run_id: (data.revision > 0).then(|| data.run_id.clone()),
            discovery_id: (data.revision > 0).then(|| data.discovery_id.clone()),
            discovery_revision: data.discovery_revision,
            workspace_id: (data.revision > 0).then_some(data.workspace_id),
            context_id: (data.revision > 0).then_some(data.context_id),
            started_at: data.started_at,
            finished_at: data.finished_at,
            results: data.results.clone(),
            results_truncated: data.results_truncated,
            counts,
            infrastructure_failure: data.infrastructure_failure,
            error: data.error.clone(),
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
        data.termination = status.termination.clone();
        data.awaiting_decision = status.awaiting_decision;
    }

    pub fn append_line(&self, line: &str, stderr: bool) {
        let mut data = self.data.lock().unwrap();
        if data.sealed || data.revision == 0 {
            return;
        }
        data.log.push(if stderr {
            format!("[stderr] {line}")
        } else {
            line.to_string()
        });
    }

    /// Finalizes from the exit the registry observed and the TRX that actually exists.
    /// Absence is stated, never converted into success.
    pub fn complete(&self, status: &ProcessStatus) -> &'static str {
        let mut data = self.data.lock().unwrap();
        if data.state != "running" || data.sealed {
            return data.state;
        }
        data.exit = status.exit;
        if let Some(termination) = &status.termination {
            data.termination = Some(termination.clone());
        }
        data.awaiting_decision = false;
        data.finished_at = Some(now_ms());
        let cancelled = data.cancel_requested || data.termination.is_some();
        finalize(&mut data, cancelled);
        data.state
    }

    pub fn fail(&self, message: impl Into<String>) {
        let mut data = self.data.lock().unwrap();
        if data.sealed || data.state != "running" {
            return;
        }
        let message = message.into();
        data.log.push(message.clone());
        data.error = Some(message);
        data.infrastructure_failure = true;
        data.state = "failed";
        data.finished_at = Some(now_ms());
    }

    /// Results stay visible; they are simply not current for the saved code.
    pub fn mark_stale(&self, reason: impl Into<String>) {
        let mut data = self.data.lock().unwrap();
        if data.revision == 0 || data.stale {
            return;
        }
        data.stale = true;
        data.stale_reason = Some(reason.into());
    }

    /// A trust/workspace boundary: stop believing this run is current. A late exit or output
    /// from the old process is dropped because the record is sealed.
    pub fn seal(&self, reason: impl Into<String>, status: Option<&ProcessStatus>) {
        let mut data = self.data.lock().unwrap();
        if data.revision == 0 {
            return;
        }
        data.stale = true;
        data.stale_reason = Some(reason.into());
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
                data.finished_at = Some(now_ms());
                finalize(&mut data, true);
            }
        }
    }
}

fn finalize(data: &mut Data, cancelled: bool) {
    let xml = fs::read_to_string(data.results_directory.join(TRX_FILE)).ok();
    let parsed = match &xml {
        Some(xml) => parse_trx(xml, &data.cases, &data.workspace_root),
        None => ParsedTrx::missing(),
    };
    data.trx_state = parsed.state;
    data.results = parsed.results;
    data.results_truncated = parsed.truncated;
    let _ = fs::write(data.results_directory.join("run.log"), data.log.text().0);
    let (state, infrastructure, error) =
        classify(cancelled, data.exit, &data.results, data.trx_state);
    data.state = state;
    data.infrastructure_failure = infrastructure;
    data.error = error;
}

/// Only a non-zero exit explained by a failed/not-executed test is a test failure. Everything
/// else with no usable result is infrastructure, never success. Exit `null` stays unknown.
fn classify(
    cancelled: bool,
    exit: Option<i32>,
    results: &[TestResult],
    trx_state: &'static str,
) -> (&'static str, bool, Option<String>) {
    if cancelled {
        let error = if results.is_empty() {
            format!(
                "Cancellation was requested; the run was terminated with exit {}. No test result was reported, so this run is not a success and no missing result is passed or skipped.",
                exit.map(|code| code.to_string()).unwrap_or_else(|| "unknown".to_string())
            )
        } else {
            format!(
                "Cancellation was requested; {} partial test result(s) are preserved and this run is not a success.",
                results.len()
            )
        };
        return ("cancelled", false, Some(error));
    }
    match trx_state {
        "missing" => {
            return (
                "failed",
                true,
                Some(format!(
                    "The test host finished with exit {} without writing a TRX file; no test result is known, so this is an infrastructure failure. The raw output is preserved.",
                    exit.map(|code| code.to_string()).unwrap_or_else(|| "unknown".to_string())
                )),
            )
        }
        "corrupt" => {
            return (
                "failed",
                true,
                Some(
                    "The TRX file is not a readable TRX document; no test result is known, so this is an infrastructure failure. The raw file is preserved."
                        .to_string(),
                ),
            )
        }
        "partial" => {
            return (
                "failed",
                true,
                Some(format!(
                    "The TRX file was written partially (exit {}); {} complete test result(s) are preserved, but this is an infrastructure failure, not a success.",
                    exit.map(|code| code.to_string()).unwrap_or_else(|| "unknown".to_string()),
                    results.len()
                )),
            )
        }
        _ => {}
    }
    if results.is_empty() {
        return (
            "failed",
            true,
            Some(format!(
                "The test host finished with exit {} and reported no test result; this is an infrastructure failure, not a test pass or skip.",
                exit.map(|code| code.to_string()).unwrap_or_else(|| "unknown".to_string())
            )),
        );
    }
    let explained = results
        .iter()
        .any(|result| matches!(result.outcome, "failed" | "notExecuted"));
    if exit != Some(0) && !explained {
        return (
            "failed",
            true,
            Some(format!(
                "The test host exited {} but every reported test passed or was skipped; the exit is not explained by a test result, so this is an infrastructure failure.",
                exit.map(|code| code.to_string()).unwrap_or_else(|| "unknown".to_string())
            )),
        );
    }
    let mut error = None;
    if exit == Some(0) && results.iter().any(|result| result.outcome == "failed") {
        error = Some(
            "The TRX reports failed tests while the test host exited 0; the outcomes shown are the TRX's, not a success claim."
                .to_string(),
        );
    }
    ("completed", false, error)
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectionError {
    Empty,
    Duplicate(String),
    Unknown(String),
    Ambiguous(String),
}

impl SelectionError {
    pub fn reason(&self) -> &'static str {
        match self {
            Self::Empty => "empty_selection",
            Self::Duplicate(_) => "duplicate_selection",
            Self::Unknown(_) => "invalid_selection",
            Self::Ambiguous(_) => "ambiguous_selection",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Empty => "The selected run has no test id. An empty selection never runs all tests."
                .to_string(),
            Self::Duplicate(id) => format!(
                "The selected run contains {id} more than once; no run was started."
            ),
            Self::Unknown(id) => format!(
                "{id} is not an identity of the current discovery; select from the discovered list or discover again."
            ),
            Self::Ambiguous(id) => format!(
                "{id} was listed more than once by VSTest with the same name (for example two tests sharing a DisplayName); it is not a unique identity, so it cannot be run individually."
            ),
        }
    }
}

/// Selected ids are validated against the current discovery. An empty, duplicated or unknown
/// id blocks the run instead of falling back to `all`.
pub fn validate_selection(
    items: &[TestItem],
    ids: &[String],
) -> Result<Vec<TestItem>, SelectionError> {
    if ids.is_empty() {
        return Err(SelectionError::Empty);
    }
    let mut seen = HashSet::new();
    let mut chosen = Vec::with_capacity(ids.len());
    for id in ids {
        if !seen.insert(id.as_str()) {
            return Err(SelectionError::Duplicate(id.clone()));
        }
        let Some(item) = items.iter().find(|item| &item.id == id) else {
            return Err(SelectionError::Unknown(id.clone()));
        };
        if item.occurrences > 1 {
            return Err(SelectionError::Ambiguous(id.clone()));
        }
        chosen.push(item.clone());
    }
    Ok(chosen)
}

/// VSTest filter values are not shell-quoted here. A name that contains filter grammar is
/// refused instead of escaped, because guessing could run a different set of tests.
pub fn filter_expression(items: &[TestItem]) -> Result<String, String> {
    if items.is_empty() {
        return Err("The filter would select no test; no run was started.".to_string());
    }
    let mut parts = Vec::with_capacity(items.len());
    for item in items {
        let fqn = item.fully_qualified_name.as_str();
        if item.occurrences > 1 {
            return Err(format!(
                "The test identity {fqn:?} was listed more than once; VSTest cannot target one copy by name, so the run was refused."
            ));
        }
        if fqn.is_empty()
            || fqn.chars().any(|c| {
                matches!(
                    c,
                    '|' | '&' | '(' | ')' | '=' | '!' | '~' | '"' | '\'' | '\n' | '\r'
                )
            })
        {
            return Err(format!(
                "The test identity {fqn:?} contains VSTest filter grammar; the run was refused instead of escaping and risking another set of tests."
            ));
        }
        parts.push(format!("FullyQualifiedName={fqn}"));
    }
    Ok(parts.join("|"))
}

/// Streams one pipe of `dotnet test` into the record; drains even when sealed so the child
/// cannot block on a full pipe.
pub fn pump(reader: impl std::io::Read, run: std::sync::Arc<TestRun>, stderr: bool) {
    crate::logbuf::pump_lines(reader, stderr, |line, stderr| {
        run.append_line(line, stderr);
    });
}

pub fn counts(results: &[TestResult]) -> Counts {
    let mut counts = Counts::default();
    for result in results {
        counts.total += 1;
        if result.matched {
            counts.matched += 1;
        }
        match result.outcome {
            "passed" => counts.passed += 1,
            "failed" => counts.failed += 1,
            "skipped" => counts.skipped += 1,
            "notExecuted" => counts.not_executed += 1,
            _ => counts.unknown += 1,
        }
    }
    counts
}

#[derive(Debug, Clone)]
pub struct ParsedTrx {
    pub results: Vec<TestResult>,
    pub state: &'static str,
    pub truncated: bool,
}

impl ParsedTrx {
    fn missing() -> Self {
        Self {
            results: Vec::new(),
            state: "missing",
            truncated: false,
        }
    }
}

/// A deliberately narrow TRX reader: attributes and the Output child elements, no XML
/// library. An incomplete document yields the results that were fully written plus
/// `state:"partial"`; a document without a TestRun root is `corrupt`. It never invents a
/// passed result.
pub fn parse_trx(xml: &str, items: &[TestItem], workspace_root: &Path) -> ParsedTrx {
    if !xml.contains("<TestRun") {
        return ParsedTrx {
            results: Vec::new(),
            state: "corrupt",
            truncated: false,
        };
    }
    let partial = !xml.contains("</TestRun>");
    let definitions = unit_test_definitions(xml);
    let skipped = skipped_names(xml);
    let mut results = Vec::new();
    let mut search = 0usize;
    let mut truncated = false;
    while let Some(offset) = xml[search..].find("<UnitTestResult") {
        let start = search + offset;
        let after = &xml[start + "<UnitTestResult".len()..];
        if !after.starts_with([' ', '\t', '\r', '\n', '>']) {
            search = start + 1;
            continue;
        }
        let Some(tag_end) = tag_end(xml, start) else {
            break;
        };
        let tag = &xml[start..=tag_end];
        let self_closing = tag.trim_end().ends_with("/>");
        let body_start = tag_end + 1;
        let (body, next) = if self_closing {
            ("", body_start)
        } else {
            match xml[body_start..].find("</UnitTestResult>") {
                Some(relative) => (
                    &xml[body_start..body_start + relative],
                    body_start + relative + "</UnitTestResult>".len(),
                ),
                None => (&xml[body_start..], xml.len()),
            }
        };
        if results.len() >= MAX_RESULTS {
            truncated = true;
            break;
        }
        results.push(parse_result(
            tag,
            body,
            &definitions,
            &skipped,
            items,
            workspace_root,
        ));
        search = next.max(start + 1);
    }
    ParsedTrx {
        results,
        state: if partial { "partial" } else { "ok" },
        truncated,
    }
}

#[derive(Debug)]
struct Definition {
    class_name: String,
    name: String,
    unit_name: String,
}

fn unit_test_definitions(xml: &str) -> HashMap<String, Definition> {
    let mut definitions = HashMap::new();
    let mut search = 0usize;
    while let Some(offset) = xml[search..].find("<UnitTest ") {
        let start = search + offset;
        let Some(end) = xml[start..].find("</UnitTest>") else {
            break;
        };
        let block = &xml[start..start + end];
        if let (Some(id), Some(unit_name)) = (attribute(block, "id"), attribute(block, "name")) {
            let method = block
                .find("<TestMethod")
                .and_then(|index| {
                    tag_end(&block[index..], 0).map(|close| &block[index..index + close + 1])
                })
                .unwrap_or("");
            definitions.insert(
                id,
                Definition {
                    class_name: attribute(method, "className").unwrap_or_default(),
                    name: attribute(method, "name").unwrap_or_default(),
                    unit_name,
                },
            );
        }
        search = start + end + "</UnitTest>".len();
    }
    definitions
}

fn skipped_names(xml: &str) -> HashSet<String> {
    let mut names = HashSet::new();
    let Some(start) = xml.find("<ResultSummary") else {
        return names;
    };
    let Some(end) = xml[start..].find("</ResultSummary>") else {
        return names;
    };
    let summary = &xml[start..start + end];
    for line in summary.lines() {
        let line = line.trim();
        // xUnit's adapter reports an explicit per-test skip line in the summary output.
        let Some(rest) = line
            .find("Test '")
            .map(|index| &line[index + "Test '".len()..])
        else {
            continue;
        };
        if let Some((name, _)) = rest.split_once("' was skipped in the test run.") {
            names.insert(name.to_string());
        }
    }
    names
}

fn parse_result(
    tag: &str,
    body: &str,
    definitions: &HashMap<String, Definition>,
    skipped: &HashSet<String>,
    items: &[TestItem],
    workspace_root: &Path,
) -> TestResult {
    let outcome_raw = attribute(tag, "outcome").unwrap_or_default();
    let test_id = attribute(tag, "testId");
    let display_name = attribute(tag, "testName").unwrap_or_default();
    // Identity comes only from TestDefinitions. A missing testId (or a definition without a
    // usable method name) leaves the result unattributed: the display name is never promoted
    // to an identity, because two different tests can print the same label.
    let definition = test_id.as_ref().and_then(|id| definitions.get(id));
    let fully_qualified_name = definition
        .map(|definition| {
            if definition.class_name.is_empty() || definition.name.is_empty() {
                definition.unit_name.clone()
            } else {
                format!("{}.{}", definition.class_name, definition.name)
            }
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_default();
    let matched = (!fully_qualified_name.is_empty())
        .then(|| {
            items
                .iter()
                .find(|item| item.fully_qualified_name == fully_qualified_name)
        })
        .flatten();
    let outcome = match outcome_raw.as_str() {
        "Passed" => "passed",
        "Failed" | "Error" | "Timeout" => "failed",
        "NotExecuted"
            if !fully_qualified_name.is_empty() && skipped.contains(&fully_qualified_name) =>
        {
            "skipped"
        }
        "NotExecuted" => "notExecuted",
        _ => "unknown",
    };
    let stack_trace = element_text(body, "StackTrace");
    TestResult {
        test_id: matched.map(|item| item.id.clone()),
        fully_qualified_name: fully_qualified_name.clone(),
        display_name,
        outcome,
        outcome_raw,
        duration_ms: attribute(tag, "duration").as_deref().and_then(duration_ms),
        message: element_text(body, "Message").map(truncate),
        stack_trace: stack_trace.clone().map(truncate),
        stdout: element_text(body, "StdOut").map(truncate),
        stderr: element_text(body, "StdErr").map(truncate),
        matched: matched.is_some(),
        position: stack_trace
            .as_deref()
            .and_then(|stack| position_from_stack(stack, workspace_root)),
    }
}

fn duration_ms(raw: &str) -> Option<u64> {
    let mut parts = raw.split(':');
    let hours: u64 = parts.next()?.parse().ok()?;
    let minutes: u64 = parts.next()?.parse().ok()?;
    let seconds = parts.next()?;
    let mut seconds = seconds.split('.');
    let whole: u64 = seconds.next()?.parse().ok()?;
    let fraction = seconds.next().unwrap_or("0");
    let millis: u64 = fraction
        .chars()
        .chain(std::iter::repeat('0'))
        .take(3)
        .collect::<String>()
        .parse()
        .unwrap_or(0);
    Some(((hours * 60 + minutes) * 60 + whole) * 1000 + millis)
}

/// A position exists only when the stack says `in <file>:line <n>` and that file resolves
/// inside the workspace. Names, types and messages are never used to guess a position.
fn position_from_stack(stack: &str, workspace_root: &Path) -> Option<TestPosition> {
    let root = fs::canonicalize(workspace_root).ok()?;
    for line in stack.lines() {
        let Some(marker) = line.find(" in ") else {
            continue;
        };
        let rest = &line[marker + 4..];
        let Some((path, number)) = rest.rsplit_once(":line ") else {
            continue;
        };
        let Ok(line_number) = number.trim().parse::<u64>() else {
            continue;
        };
        if line_number == 0 {
            continue;
        }
        let candidate = Path::new(path.trim());
        let candidate = if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            root.join(candidate)
        };
        let Ok(canonical) = fs::canonicalize(&candidate) else {
            continue;
        };
        if !canonical.starts_with(&root) || !canonical.is_file() {
            continue;
        }
        let relative = canonical.strip_prefix(&root).ok()?;
        let path = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("/");
        return Some(TestPosition {
            path,
            line: line_number - 1,
            character: 0,
        });
    }
    None
}

fn truncate(text: String) -> String {
    let mut chars = text.chars();
    let truncated: String = chars.by_ref().take(MAX_TEXT_CHARS).collect();
    if chars.next().is_some() {
        format!("{truncated}… [truncated]")
    } else {
        truncated
    }
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let mut search = 0usize;
    while let Some(offset) = tag[search..].find(name) {
        let start = search + offset;
        let before_ok = start == 0
            || tag[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_whitespace() || c == '<');
        let after = &tag[start + name.len()..];
        if before_ok {
            if let Some(rest) = after.strip_prefix("=\"") {
                if let Some(end) = rest.find('"') {
                    return Some(xml_unescape(&rest[..end]));
                }
            }
        }
        search = start + name.len();
    }
    None
}

fn tag_end(xml: &str, start: usize) -> Option<usize> {
    let mut quote = false;
    for (index, c) in xml[start..].char_indices() {
        match c {
            '"' => quote = !quote,
            '>' if !quote => return Some(start + index),
            _ => {}
        }
    }
    None
}

/// The text of the first `<tag>` element, including CDATA and escaped entities.
fn element_text(body: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let start = body.find(&open)?;
    let after = &body[start + open.len()..];
    if !after.starts_with([' ', '\t', '\r', '\n', '>']) {
        return None;
    }
    let open_end = tag_end(body, start)?;
    if body[start..=open_end].trim_end().ends_with("/>") {
        return Some(String::new());
    }
    let content = &body[open_end + 1..];
    let close = format!("</{tag}>");
    let end = content.find(&close)?;
    Some(xml_text(&content[..end]))
}

fn xml_text(raw: &str) -> String {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find("<![CDATA[") {
        out.push_str(&xml_unescape(&rest[..start]));
        let after = &rest[start + "<![CDATA[".len()..];
        let Some(end) = after.find("]]>") else {
            out.push_str(after);
            return out;
        };
        out.push_str(&after[..end]);
        rest = &after[end + 3..];
    }
    out.push_str(&xml_unescape(rest));
    out
}

fn xml_unescape(raw: &str) -> String {
    raw.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
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
    use crate::process::Termination;

    fn item(id: &str, fqn: &str) -> TestItem {
        TestItem {
            id: format!("test:{id}"),
            fully_qualified_name: fqn.to_string(),
            display_name: fqn.to_string(),
            source: "vstest",
            project: "tests/Tests.csproj".to_string(),
            tfm: Some("net8.0".to_string()),
            discovery_id: "discovery:1:1".to_string(),
            revision: 1,
            occurrences: 1,
        }
    }

    fn items() -> Vec<TestItem> {
        vec![
            item("a", "N.C.Pass"),
            item("b", "N.C.Fail"),
            item("c", "N.C.Skip"),
            item("d", "N.C.Skip2"),
        ]
    }

    const TRX: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<TestRun xmlns="http://microsoft.com/schemas/VisualStudio/TeamTest/2010">
  <Results>
    <UnitTestResult testId="1" testName="N.C.Pass" duration="00:00:00.1200000" outcome="Passed">
      <Output><StdOut>hello &amp; bye</StdOut><StdErr>warn</StdErr></Output>
    </UnitTestResult>
    <UnitTestResult testId="2" testName="N.C.Fail" duration="00:00:01.0050000" outcome="Failed">
      <Output><ErrorInfo><Message>Expected 43 but was &lt;42&gt;</Message><StackTrace>   at N.C.Fail() in {ROOT}/tests/F.cs:line 29
   at System.RuntimeMethodHandle.InvokeMethod()</StackTrace></ErrorInfo></Output>
    </UnitTestResult>
    <UnitTestResult testId="3" testName="N.C.Skip" duration="00:00:00.0010000" outcome="NotExecuted">
      <Output><ErrorInfo><Message>skipped on purpose</Message></ErrorInfo></Output>
    </UnitTestResult>
    <UnitTestResult testId="4" testName="N.C.Skip2" duration="00:00:00.0010000" outcome="NotExecuted" />
  </Results>
  <TestDefinitions>
    <UnitTest name="N.C.Pass" id="1"><TestMethod className="N.C" name="Pass" /></UnitTest>
    <UnitTest name="N.C.Fail" id="2"><TestMethod className="N.C" name="Fail" /></UnitTest>
    <UnitTest name="N.C.Skip" id="3"><TestMethod className="N.C" name="Skip" /></UnitTest>
    <UnitTest name="N.C.Skip2" id="4"><TestMethod className="N.C" name="Skip2" /></UnitTest>
  </TestDefinitions>
  <ResultSummary outcome="Failed">
    <Output><StdOut>[xUnit.net] Test 'N.C.Skip' was skipped in the test run.</StdOut></Output>
  </ResultSummary>
</TestRun>"#;

    #[test]
    fn selection_rejects_empty_unknown_and_duplicate_ids() {
        let items = items();
        assert_eq!(
            validate_selection(&items, &[]).unwrap_err().reason(),
            "empty_selection"
        );
        assert_eq!(
            validate_selection(&items, &["test:zzz".to_string()])
                .unwrap_err()
                .reason(),
            "invalid_selection"
        );
        assert_eq!(
            validate_selection(&items, &["test:a".to_string(), "test:a".to_string()])
                .unwrap_err()
                .reason(),
            "duplicate_selection"
        );
        let chosen = validate_selection(&items, &["test:b".to_string()]).unwrap();
        assert_eq!(chosen.len(), 1);
        assert_eq!(chosen[0].fully_qualified_name, "N.C.Fail");

        let mut ambiguous = items.clone();
        ambiguous[0].occurrences = 2;
        assert_eq!(
            validate_selection(&ambiguous, &["test:a".to_string()])
                .unwrap_err()
                .reason(),
            "ambiguous_selection"
        );
        assert!(filter_expression(&ambiguous[..1]).is_err());
    }

    #[test]
    fn the_filter_uses_exact_identities_and_refuses_grammar() {
        let items = items();
        assert_eq!(
            filter_expression(&items[..2]).unwrap(),
            "FullyQualifiedName=N.C.Pass|FullyQualifiedName=N.C.Fail"
        );
        let spaces = vec![item("s", "N.C.Method name with spaces")];
        assert_eq!(
            filter_expression(&spaces).unwrap(),
            "FullyQualifiedName=N.C.Method name with spaces"
        );
        for name in [
            "N.C.Pass|N.C.Fail",
            "N.C.(a)",
            "N.C.a&b",
            "N.C.a=b",
            "N.C.a!b",
            "N.C.a~b",
            "N.C.a\nb",
            "N.C.a'b",
        ] {
            let unsafe_item = vec![item("u", name)];
            assert!(filter_expression(&unsafe_item).is_err(), "{name}");
        }
        assert!(filter_expression(&[]).is_err());
    }

    #[test]
    fn a_real_trx_maps_definitions_outcomes_details_and_positions() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(root.join("tests/F.cs"), "// fixture\n").unwrap();
        let xml = TRX.replace("{ROOT}", &root.to_string_lossy());
        let parsed = parse_trx(&xml, &items(), root);
        assert_eq!(parsed.state, "ok");
        assert_eq!(parsed.results.len(), 4);
        assert_eq!(parsed.results[0].outcome, "passed");
        assert_eq!(parsed.results[0].duration_ms, Some(120));
        assert_eq!(parsed.results[0].stdout.as_deref(), Some("hello & bye"));
        assert_eq!(parsed.results[0].stderr.as_deref(), Some("warn"));
        assert_eq!(parsed.results[1].outcome, "failed");
        assert_eq!(
            parsed.results[1].message.as_deref(),
            Some("Expected 43 but was <42>")
        );
        assert_eq!(
            parsed.results[1].position,
            Some(TestPosition {
                path: "tests/F.cs".to_string(),
                line: 28,
                character: 0
            })
        );
        // The summary has an explicit skip line; the other NotExecuted has none.
        assert_eq!(parsed.results[2].outcome, "skipped");
        assert_eq!(parsed.results[3].outcome, "notExecuted");
        assert_eq!(
            parsed.results[1].test_id.as_deref(),
            Some("test:b"),
            "identity comes from TestDefinitions, not the label"
        );
        let counts = counts(&parsed.results);
        assert_eq!((counts.total, counts.passed, counts.failed), (4, 1, 1));
        assert_eq!((counts.skipped, counts.not_executed), (1, 1));
    }

    #[test]
    fn a_trx_result_without_a_definition_keeps_its_label_but_no_identity() {
        let temp = tempfile::tempdir().unwrap();
        let xml = r#"<TestRun><Results><UnitTestResult testName="N.C.Pass" outcome="Passed" /></Results></TestRun>"#;
        let parsed = parse_trx(xml, &items(), temp.path());
        assert_eq!(parsed.results.len(), 1);
        assert_eq!(parsed.results[0].test_id, None);
        assert!(!parsed.results[0].matched);
        assert_eq!(parsed.results[0].fully_qualified_name, "");
        assert_eq!(parsed.results[0].display_name, "N.C.Pass");
    }

    #[test]
    fn a_trx_result_with_an_unknown_test_id_is_never_matched_by_display_name() {
        let temp = tempfile::tempdir().unwrap();
        let xml = r#"<TestRun><Results><UnitTestResult testId="999" testName="N.C.Pass" outcome="Passed" /></Results><TestDefinitions><UnitTest name="N.C.Pass" id="1"><TestMethod className="N.C" name="Pass" /></UnitTest></TestDefinitions></TestRun>"#;
        let parsed = parse_trx(xml, &items(), temp.path());
        assert_eq!(parsed.results.len(), 1);
        assert_eq!(parsed.results[0].test_id, None);
        assert!(!parsed.results[0].matched);
        assert_eq!(parsed.results[0].fully_qualified_name, "");
    }

    #[test]
    fn a_stack_position_outside_the_workspace_or_without_line_is_not_navigable() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("Other.cs"), "// other\n").unwrap();
        let outside_xml = format!(
            r#"<TestRun><Results><UnitTestResult testId="1" testName="N.C.Fail" outcome="Failed"><Output><ErrorInfo><StackTrace>   at N.C.Fail() in {}/Other.cs:line 4</StackTrace></ErrorInfo></Output></UnitTestResult></Results><TestDefinitions><UnitTest name="N.C.Fail" id="1"><TestMethod className="N.C" name="Fail" /></UnitTest></TestDefinitions></TestRun>"#,
            outside.path().to_string_lossy()
        );
        let parsed = parse_trx(&outside_xml, &items(), root);
        assert_eq!(parsed.results[0].position, None);

        let no_line = r#"<TestRun><Results><UnitTestResult testId="1" testName="N.C.Fail" outcome="Failed"><Output><ErrorInfo><StackTrace>at N.C.Fail() in /work/F.cs</StackTrace></ErrorInfo></Output></UnitTestResult></Results><TestDefinitions><UnitTest name="N.C.Fail" id="1"><TestMethod className="N.C" name="Fail" /></UnitTest></TestDefinitions></TestRun>"#;
        assert_eq!(parse_trx(no_line, &items(), root).results[0].position, None);

        let missing_file = format!(
            r#"<TestRun><Results><UnitTestResult testId="1" testName="N.C.Fail" outcome="Failed"><Output><ErrorInfo><StackTrace>at N.C.Fail() in {}/Gone.cs:line 2</StackTrace></ErrorInfo></Output></UnitTestResult></Results></TestRun>"#,
            root.to_string_lossy()
        );
        assert_eq!(
            parse_trx(&missing_file, &items(), root).results[0].position,
            None
        );
    }

    #[test]
    fn corrupt_missing_partial_and_empty_trx_are_not_success() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let corrupt = parse_trx("not xml at all", &items(), root);
        assert_eq!(corrupt.state, "corrupt");
        assert!(corrupt.results.is_empty());

        let (state, infrastructure, _) = classify(false, Some(2), &[], "missing");
        assert_eq!((state, infrastructure), ("failed", true));
        let (state, infrastructure, _) = classify(false, Some(1), &[], "ok");
        assert_eq!((state, infrastructure), ("failed", true));
        let (state, infrastructure, _) = classify(false, None, &[], "ok");
        assert_eq!((state, infrastructure), ("failed", true));

        let partial = parse_trx(
            r#"<TestRun><Results><UnitTestResult testId="1" testName="N.C.Pass" outcome="Passed" />"#,
            &items(),
            root,
        );
        assert_eq!(partial.state, "partial");
        assert_eq!(partial.results.len(), 1);
        let (state, infrastructure, error) = classify(false, Some(1), &partial.results, "partial");
        assert_eq!((state, infrastructure), ("failed", true));
        assert!(error.unwrap().contains("partially"));
        assert_eq!(partial.results[0].outcome, "passed");
    }

    #[test]
    fn infra_and_assertion_failures_are_distinct_and_exit_unknown_stays_unknown() {
        let passed = TestResult {
            test_id: Some("test:a".to_string()),
            fully_qualified_name: "N.C.Pass".to_string(),
            display_name: "N.C.Pass".to_string(),
            outcome: "passed",
            outcome_raw: "Passed".to_string(),
            duration_ms: Some(1),
            message: None,
            stack_trace: None,
            stdout: None,
            stderr: None,
            matched: true,
            position: None,
        };
        let failed = TestResult {
            outcome: "failed",
            outcome_raw: "Failed".to_string(),
            ..passed.clone()
        };
        let skipped = TestResult {
            outcome: "skipped",
            outcome_raw: "NotExecuted".to_string(),
            ..passed.clone()
        };
        assert_eq!(
            classify(false, Some(1), &[passed.clone(), failed], "ok"),
            ("completed", false, None)
        );
        assert_eq!(
            classify(false, Some(1), &[passed.clone(), skipped.clone()], "ok").0,
            "failed"
        );
        let (state, infra, error) = classify(false, Some(143), &[], "ok");
        assert_eq!((state, infra), ("failed", true));
        assert!(error.unwrap().contains("infrastructure"));
        assert_eq!(classify(false, None, &[passed.clone()], "ok").0, "failed");
        let (state, infra, error) = classify(false, None, &[passed, skipped], "ok");
        assert_eq!((state, infra), ("failed", true));
        assert!(error.unwrap().contains("not explained"));
    }

    fn status(exit: Option<i32>, termination: Option<Termination>) -> ProcessStatus {
        ProcessStatus {
            state: "exited",
            exit,
            signal: None,
            termination,
            awaiting_decision: false,
        }
    }

    fn started(directory: &Path) -> TestRun {
        TestRun::begin(Start {
            revision: 1,
            run_id: "testrun:1:1".to_string(),
            project_name: "Tests".to_string(),
            project_path: "tests/Tests.csproj".to_string(),
            discovery_id: "discovery:1:1".to_string(),
            discovery_revision: 1,
            configuration: "Debug".to_string(),
            target_framework: Some("net8.0".to_string()),
            selection: "all",
            selected_test_ids: Vec::new(),
            command: vec!["dotnet".to_string(), "test".to_string()],
            cwd: "/work".to_string(),
            workspace_root: directory.to_path_buf(),
            results_directory: directory.to_path_buf(),
            workspace_id: 1,
            context_id: 2,
            cases: items(),
        })
    }

    #[test]
    fn cancel_keeps_partial_results_and_never_marks_a_test_cancelled() {
        let temp = tempfile::tempdir().unwrap();
        let run = started(temp.path());
        fs::write(
            temp.path().join(TRX_FILE),
            r#"<TestRun><Results><UnitTestResult testId="1" testName="N.C.Pass" outcome="Passed" /></Results>"#,
        )
        .unwrap();
        run.request_cancel();
        let terminated = status(
            Some(143),
            Some(Termination {
                requested: "SIGTERM".to_string(),
                observed: "exited".to_string(),
                exit: Some(143),
                killed_after_grace: false,
            }),
        );
        assert_eq!(run.complete(&terminated), "cancelled");
        let view = run.view();
        assert_eq!(view.state, "cancelled");
        assert_eq!(view.results.len(), 1);
        assert_eq!(view.results[0].outcome, "passed");
        assert!(view
            .results
            .iter()
            .all(|result| result.outcome != "cancelled"));
        assert_eq!(view.trx_state, "partial");
        assert!(view.error.unwrap().contains("not a success"));
        assert!(view.log_path.as_ref().unwrap().ends_with("run.log"));
        assert!(std::path::Path::new(view.trx_path.as_ref().unwrap()).exists());
        assert!(std::path::Path::new(view.log_path.as_ref().unwrap()).exists());
    }

    #[test]
    fn a_stale_run_keeps_its_results_and_is_not_a_new_result() {
        let temp = tempfile::tempdir().unwrap();
        let run = started(temp.path());
        fs::write(
            temp.path().join(TRX_FILE),
            r#"<TestRun><Results><UnitTestResult testId="1" testName="N.C.Pass" outcome="Passed" /></Results><TestDefinitions><UnitTest name="N.C.Pass" id="1"><TestMethod className="N.C" name="Pass" /></UnitTest></TestDefinitions></TestRun>"#,
        )
        .unwrap();
        assert_eq!(run.complete(&status(Some(0), None)), "completed");
        run.mark_stale("A buffer changed after this run.");
        let stale = run.view();
        assert!(stale.stale);
        assert_eq!(stale.state, "completed");
        assert_eq!(stale.results.len(), 1);
        assert_eq!(stale.counts.passed, 1);
        assert_eq!(
            stale.stale_reason.as_deref(),
            Some("A buffer changed after this run.")
        );
    }

    #[test]
    fn sealing_a_run_drops_late_output_and_keeps_the_observed_exit() {
        let temp = tempfile::tempdir().unwrap();
        let run = started(temp.path());
        run.append_line("before revoke", false);
        run.seal("Trust was revoked.", Some(&status(Some(143), None)));
        let sealed = run.view();
        assert!(sealed.sealed && sealed.stale);
        assert_eq!(sealed.state, "cancelled");
        assert_eq!(sealed.exit, Some(143));
        let finished_at = sealed.finished_at;
        run.append_line("late line", false);
        assert_eq!(run.view().finished_at, finished_at);
        assert!(!run.view().log.contains("late line"));
        assert_eq!(run.complete(&status(Some(0), None)), "cancelled");
    }

    #[test]
    fn a_new_run_and_an_old_record_do_not_share_results() {
        let temp = tempfile::tempdir().unwrap();
        let old = started(temp.path());
        old.append_line("old line", false);
        let current = TestRun::idle();
        assert_eq!(current.view().revision, 0);
        assert!(current.view().results.is_empty());
        old.complete(&status(Some(1), None));
        assert!(!current.view().log.contains("old line"));
        assert_eq!(current.view().run_id, None);
    }
}
