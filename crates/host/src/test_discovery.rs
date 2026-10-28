//! Real VSTest discovery for one explicitly selected test project.
//!
//! Discovery is one `dotnet test --list-tests` process of the project SDK muxer, with no
//! implicit restore and no package changes. The only identity this command provides is the
//! fully-qualified name printed under `The following Tests are available:`; display names are
//! kept as what the runner printed, never used alone as identity. Microsoft.Testing.Platform
//! projects are refused before any VSTest command because the console runner does not own them.

use crate::evaluation::PackageRef;
use crate::logbuf::LogBuffer;
use crate::process::{ProcessStatus, Termination};
use serde::Serialize;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// The exact message shown for an MTP project. No VSTest process is started.
pub const MTP_MESSAGE: &str =
    "Microsoft.Testing.Platform is not supported in this version. Use the terminal.";
pub const LIST_MARKER: &str = "The following Tests are available:";
pub const EMPTY_GUIDANCE: &str =
    "The VSTest adapter ran and listed no tests. This is not a discovery failure: add a test or check the adapter output, then discover again.";

const ADAPTERS: &[&str] = &[
    "xunit.runner.visualstudio",
    "nunit3testadapter",
    "mstest.testadapter",
];

/// Everything one discovery needs to identify itself, including the synthetic preflight
/// states that never run a command (`command` stays empty).
pub struct Start {
    pub revision: u64,
    pub discovery_id: String,
    pub project_name: String,
    pub project_path: String,
    pub target_framework: Option<String>,
    pub command: Vec<String>,
    pub cwd: PathBuf,
    pub sdk_root: PathBuf,
    pub sdk_version: String,
    pub workspace_id: u64,
    pub context_id: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryProject {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestItem {
    /// Stable FNV-1a of project, TFM and fully-qualified name; not the discovery index.
    pub id: String,
    pub fully_qualified_name: String,
    /// What the runner printed; with `--list-tests` this is the same string, never the identity.
    pub display_name: String,
    pub source: &'static str,
    pub project: String,
    pub tfm: Option<String>,
    pub discovery_id: String,
    pub revision: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveryView {
    pub state: &'static str,
    pub project: Option<DiscoveryProject>,
    pub target_framework: Option<String>,
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
    pub revision: u64,
    pub discovery_id: Option<String>,
    pub workspace_id: Option<u64>,
    pub context_id: Option<u64>,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub items: Vec<TestItem>,
    pub partial: bool,
    pub error: Option<String>,
    pub mtp_message: Option<String>,
    pub stale: bool,
    pub stale_reason: Option<String>,
    pub sealed: bool,
}

/// The snapshot node. Ticket 17 adds execution fields next to `discovery`; this stays additive.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TestsView {
    pub discovery: DiscoveryView,
}

struct Data {
    revision: u64,
    state: &'static str,
    discovery_id: String,
    project: DiscoveryProject,
    target_framework: Option<String>,
    command: Vec<String>,
    cwd: String,
    sdk_root: String,
    sdk_version: String,
    workspace_id: u64,
    context_id: u64,
    process_id: Option<u64>,
    exit: Option<i32>,
    termination: Option<Termination>,
    awaiting_decision: bool,
    cancel_requested: bool,
    sealed: bool,
    stale: bool,
    stale_reason: Option<String>,
    started_at: Option<u64>,
    finished_at: Option<u64>,
    log: LogBuffer,
    items: Vec<TestItem>,
    seen: HashSet<String>,
    in_list: bool,
    list_ended: bool,
    partial: bool,
    error: Option<String>,
    mtp_message: Option<String>,
}

pub struct Tests {
    data: Mutex<Data>,
}

impl Tests {
    pub fn idle() -> Self {
        Self {
            data: Mutex::new(Data {
                revision: 0,
                state: "idle",
                discovery_id: String::new(),
                project: DiscoveryProject {
                    name: String::new(),
                    path: String::new(),
                },
                target_framework: None,
                command: Vec::new(),
                cwd: String::new(),
                sdk_root: String::new(),
                sdk_version: String::new(),
                workspace_id: 0,
                context_id: 0,
                process_id: None,
                exit: None,
                termination: None,
                awaiting_decision: false,
                cancel_requested: false,
                sealed: false,
                stale: false,
                stale_reason: None,
                started_at: None,
                finished_at: None,
                log: LogBuffer::default(),
                items: Vec::new(),
                seen: HashSet::new(),
                in_list: false,
                list_ended: false,
                partial: false,
                error: None,
                mtp_message: None,
            }),
        }
    }

    pub fn begin(start: Start) -> Self {
        Self {
            data: Mutex::new(data_for(start, "discovering", None, None)),
        }
    }

    /// A preflight that found nothing to run (`adapter_missing`, `mtp_unsupported`). The
    /// command stays empty because no VSTest process was started.
    pub fn notice(
        start: Start,
        state: &'static str,
        error: Option<String>,
        mtp_message: Option<String>,
    ) -> Self {
        let mut data = data_for(start, state, error, mtp_message);
        let message = data
            .mtp_message
            .clone()
            .or_else(|| data.error.clone())
            .unwrap_or_default();
        if !message.is_empty() {
            data.log.push(message);
        }
        Self {
            data: Mutex::new(data),
        }
    }

    pub fn view(&self) -> TestsView {
        let data = self.data.lock().unwrap();
        let (log, log_truncated) = data.log.text();
        TestsView {
            discovery: DiscoveryView {
                state: data.state,
                project: (data.revision > 0).then(|| data.project.clone()),
                target_framework: data.target_framework.clone(),
                command: data.command.clone(),
                cwd: data.cwd.clone(),
                sdk_root: (data.revision > 0).then(|| data.sdk_root.clone()),
                sdk_version: (data.revision > 0).then(|| data.sdk_version.clone()),
                log,
                log_truncated,
                exit: data.exit,
                termination: data.termination.clone(),
                process_id: data.process_id,
                awaiting_decision: data.awaiting_decision,
                revision: data.revision,
                discovery_id: (data.revision > 0).then(|| data.discovery_id.clone()),
                workspace_id: (data.revision > 0).then_some(data.workspace_id),
                context_id: (data.revision > 0).then_some(data.context_id),
                started_at: data.started_at,
                finished_at: data.finished_at,
                items: data.items.clone(),
                partial: data.partial,
                error: data.error.clone(),
                mtp_message: data.mtp_message.clone(),
                stale: data.stale,
                stale_reason: data.stale_reason.clone(),
                sealed: data.sealed,
            },
        }
    }

    pub fn note_process(&self, id: u64) {
        let mut data = self.data.lock().unwrap();
        if data.sealed {
            return;
        }
        data.process_id = Some(id);
    }

    pub fn running_process_id(&self) -> Option<u64> {
        let data = self.data.lock().unwrap();
        if data.state == "discovering" && !data.sealed {
            data.process_id
        } else {
            None
        }
    }

    pub fn request_cancel(&self) {
        let mut data = self.data.lock().unwrap();
        if data.state == "discovering" && !data.sealed {
            data.cancel_requested = true;
        }
    }

    /// Records what the registry observed; only a discovering, unsealed record changes.
    pub fn observe(&self, status: &ProcessStatus) {
        let mut data = self.data.lock().unwrap();
        if data.state != "discovering" || data.sealed {
            return;
        }
        data.exit = status.exit;
        data.termination = status.termination.clone();
        data.awaiting_decision = status.awaiting_decision;
    }

    /// Finalizes from the observed exit. `exit: null` is never a successful discovery.
    pub fn complete(&self, status: &ProcessStatus) -> &'static str {
        let mut data = self.data.lock().unwrap();
        if data.state != "discovering" || data.sealed {
            return data.state;
        }
        data.exit = status.exit;
        if let Some(termination) = &status.termination {
            data.termination = Some(termination.clone());
        }
        let cancelled = data.cancel_requested || data.termination.is_some();
        if cancelled {
            data.state = "cancelled";
            // The list stopped where termination arrived: whatever was printed is partial.
            data.partial = true;
        } else if status.exit == Some(0) {
            if !data.in_list {
                data.state = "failed";
                data.error = Some(
                    "dotnet test exited 0 but printed no `The following Tests are available:` section; discovery is unverified, not empty."
                        .to_string(),
                );
            } else if data.items.is_empty() {
                data.state = "empty";
                data.error = Some(EMPTY_GUIDANCE.to_string());
            } else {
                data.state = "completed";
            }
        } else {
            let exit = status
                .exit
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            data.state = "failed";
            data.partial = data.in_list;
            data.error = Some(format!(
                "dotnet test exited {exit} before discovery completed; the raw log is preserved."
            ));
        }
        data.awaiting_decision = false;
        data.finished_at = Some(now_ms());
        data.state
    }

    pub fn fail(&self, message: impl Into<String>) {
        let mut data = self.data.lock().unwrap();
        if data.sealed || data.state != "discovering" {
            return;
        }
        let message = message.into();
        data.log.push(message.clone());
        data.error = Some(message);
        data.state = "failed";
        data.finished_at = Some(now_ms());
    }

    /// Discovery output stays available after success, failure and cancellation. Sealed
    /// records drop later lines, so an old process cannot rewrite another context.
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
        if stderr || data.list_ended {
            return;
        }
        if !data.in_list {
            if line.trim() == LIST_MARKER {
                data.in_list = true;
            }
            return;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return;
        }
        // VSTest indents listed names; a non-indented line is the summary that follows an
        // empty list (`No test is available …`) or other output, not a test identity.
        if !line.starts_with(' ') && !line.starts_with('\t') {
            data.list_ended = true;
            return;
        }
        push_item(&mut data, trimmed);
    }

    /// Keeps the completed/failed observation and its items; it is simply not current.
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

    /// A trust/workspace boundary: stop believing this record is current. Partial items and
    /// the log stay; an exit that never arrived stays `null`.
    pub fn seal(&self, reason: impl Into<String>, status: Option<&ProcessStatus>) {
        let mut data = self.data.lock().unwrap();
        if data.revision == 0 {
            return;
        }
        data.stale = true;
        data.stale_reason = Some(reason.into());
        data.sealed = true;
        if data.state == "discovering" {
            data.cancel_requested = true;
            data.state = "cancelled";
            data.partial = true;
        }
        if let Some(status) = status {
            data.exit = status.exit;
            data.termination = status.termination.clone();
            data.awaiting_decision = status.state == "running";
            if status.state == "exited" {
                data.finished_at = Some(now_ms());
            }
        }
    }
}

fn data_for(
    start: Start,
    state: &'static str,
    error: Option<String>,
    mtp_message: Option<String>,
) -> Data {
    Data {
        revision: start.revision,
        state,
        discovery_id: start.discovery_id,
        project: DiscoveryProject {
            name: start.project_name,
            path: start.project_path,
        },
        target_framework: start.target_framework,
        command: start.command,
        cwd: start.cwd.to_string_lossy().to_string(),
        sdk_root: start.sdk_root.to_string_lossy().to_string(),
        sdk_version: start.sdk_version,
        workspace_id: start.workspace_id,
        context_id: start.context_id,
        process_id: None,
        exit: None,
        termination: None,
        awaiting_decision: false,
        cancel_requested: false,
        sealed: false,
        stale: false,
        stale_reason: None,
        started_at: Some(now_ms()),
        finished_at: Some(now_ms()),
        log: LogBuffer::default(),
        items: Vec::new(),
        seen: HashSet::new(),
        in_list: false,
        list_ended: false,
        partial: false,
        error,
        mtp_message,
    }
}

fn push_item(data: &mut Data, listed: &str) {
    if !data.seen.insert(listed.to_string()) {
        return;
    }
    let project = data.project.path.clone();
    let tfm = data.target_framework.clone();
    data.items.push(TestItem {
        id: item_id(&project, tfm.as_deref(), listed),
        fully_qualified_name: listed.to_string(),
        display_name: listed.to_string(),
        source: "vstest",
        project,
        tfm,
        discovery_id: data.discovery_id.clone(),
        revision: data.revision,
    });
}

/// Stable across discoveries of the same project/TFM/test; independent of list position.
pub fn item_id(project: &str, tfm: Option<&str>, fully_qualified_name: &str) -> String {
    let input = format!("{project}|{}|{fully_qualified_name}", tfm.unwrap_or(""));
    format!("test:{:016x}", fnv1a64(input.as_bytes()))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// MTP is detected before any VSTest argv exists. The project text and the evaluated
/// package references are both checked; the property is treated as a gate even when set to
/// false, because guessing here would run a runner that does not own the project.
pub fn mtp_detected(project_text: &str, packages: &[PackageRef]) -> bool {
    let text = project_text.to_ascii_lowercase();
    text.contains("microsoft.testing.platform")
        || text.contains("usemicrosofttestingplatformrunner")
        || packages.iter().any(|package| {
            package
                .name
                .to_ascii_lowercase()
                .contains("microsoft.testing.platform")
        })
}

/// `None` only when the evaluated references prove both Microsoft.NET.Test.Sdk and one
/// known VSTest adapter. The host never adds or edits a package.
pub fn adapter_missing(packages: &[PackageRef]) -> Option<String> {
    let names: HashSet<String> = packages
        .iter()
        .map(|package| package.name.to_ascii_lowercase())
        .collect();
    let has_sdk = names.contains("microsoft.net.test.sdk");
    let has_adapter = ADAPTERS.iter().any(|adapter| names.contains(*adapter));
    if has_sdk && has_adapter {
        return None;
    }
    let mut missing = Vec::new();
    if !has_sdk {
        missing.push("Microsoft.NET.Test.Sdk");
    }
    if !has_adapter {
        missing.push(
            "a VSTest adapter (xunit.runner.visualstudio, NUnit3TestAdapter or MSTest.TestAdapter)",
        );
    }
    Some(format!(
        "No VSTest adapter is configured for this project: {}. OpenRidr does not add or install packages; configure the adapter in the project, restore explicitly and discover again.",
        missing.join(" and ")
    ))
}

/// Streams one pipe of the discovery process into the record, line by line. Drains even when
/// the record is sealed so the child cannot block on a full pipe.
pub fn pump(reader: impl std::io::Read, tests: std::sync::Arc<Tests>, stderr: bool) {
    crate::logbuf::pump_lines(reader, stderr, |line, stderr| {
        tests.append_line(line, stderr);
    });
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

    fn start(revision: u64, discovery: &str) -> Start {
        Start {
            revision,
            discovery_id: discovery.to_string(),
            project_name: "OrderDesk.Tests".to_string(),
            project_path: "tests/OrderDesk.Tests/OrderDesk.Tests.csproj".to_string(),
            target_framework: None,
            command: vec!["dotnet".to_string(), "test".to_string()],
            cwd: PathBuf::from("/work/Order Desk"),
            sdk_root: PathBuf::from("/sdk/8.0.408"),
            sdk_version: "8.0.408".to_string(),
            workspace_id: 1,
            context_id: 2,
        }
    }

    fn exited(exit: Option<i32>) -> ProcessStatus {
        ProcessStatus {
            state: "exited",
            exit,
            termination: None,
            awaiting_decision: false,
        }
    }

    fn completed(items: &[&str]) -> Tests {
        let tests = Tests::begin(start(1, "discovery:1:1"));
        tests.append_line(
            "Test run for /work/OrderDesk.Tests.dll (.NETCoreApp,Version=v8.0)",
            false,
        );
        tests.append_line(LIST_MARKER, false);
        for item in items {
            tests.append_line(&format!("    {item}"), false);
        }
        assert_eq!(tests.complete(&exited(Some(0))), "completed");
        tests
    }

    #[test]
    fn parses_identities_from_the_real_section_with_nested_names_and_spaces() {
        let tests = completed(&[
            "OrderDesk.Tests.OrderTests.Id_is_preserved",
            "OrderDesk.Tests.OrderTests.Deliberate_failure_for_runner_evidence",
            "OrderDesk.Tests.Outer+Inner.Nested_test",
            "OrderDesk.Tests.Names.Method name with spaces",
        ]);
        let view = tests.view().discovery;
        assert_eq!(view.state, "completed");
        assert_eq!(view.items.len(), 4);
        assert!(!view.partial);
        assert_eq!(
            view.items[2].fully_qualified_name,
            "OrderDesk.Tests.Outer+Inner.Nested_test"
        );
        assert_eq!(
            view.items[3].fully_qualified_name,
            "OrderDesk.Tests.Names.Method name with spaces"
        );
        assert_eq!(view.items[0].source, "vstest");
        assert_eq!(
            view.items[0].project,
            "tests/OrderDesk.Tests/OrderDesk.Tests.csproj"
        );
        assert_eq!(view.items[0].discovery_id, "discovery:1:1");
        assert_eq!(view.items[0].revision, 1);
        // Identity is stable and independent of list position, not derived from a label.
        let moved = item_id(
            "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
            None,
            "OrderDesk.Tests.OrderTests.Id_is_preserved",
        );
        assert_eq!(view.items[0].id, moved);
        assert_ne!(view.items[0].id, view.items[1].id);
    }

    #[test]
    fn duplicate_listed_names_count_once_and_an_empty_section_is_empty_not_failure() {
        let duplicate = completed(&["A.B.Same", "A.B.Same"]);
        assert_eq!(duplicate.view().discovery.items.len(), 1);

        let empty = Tests::begin(start(2, "discovery:1:2"));
        empty.append_line(LIST_MARKER, false);
        empty.append_line(
            "No test is available in /work/OrderDesk.Tests.dll. Make sure that test discoverer & executors are registered.",
            false,
        );
        assert_eq!(empty.complete(&exited(Some(0))), "empty");
        let view = empty.view().discovery;
        assert!(view.items.is_empty());
        assert!(view
            .error
            .as_deref()
            .unwrap()
            .contains("not a discovery failure"));

        let unparseable = Tests::begin(start(3, "discovery:1:3"));
        unparseable.append_line("Build succeeded.", false);
        assert_eq!(unparseable.complete(&exited(Some(0))), "failed");
        assert!(unparseable
            .view()
            .discovery
            .error
            .as_deref()
            .unwrap()
            .contains("unverified"));
    }

    #[test]
    fn a_non_zero_exit_fails_with_the_log_and_never_becomes_empty() {
        let tests = Tests::begin(start(1, "discovery:1:1"));
        tests.append_line(
            "/work/OrderDesk.Tests/Broken.cs(1,38): error CS0246: missing type",
            false,
        );
        assert_eq!(tests.complete(&exited(Some(1))), "failed");
        let view = tests.view().discovery;
        assert_eq!(view.exit, Some(1));
        assert!(view.log.contains("CS0246"));
        assert!(view.error.as_deref().unwrap().contains("exited 1"));
        assert!(!view.partial);
    }

    #[test]
    fn cancel_preserves_partial_items_and_an_unknown_exit_stays_unknown() {
        let tests = Tests::begin(start(1, "discovery:1:1"));
        tests.append_line(LIST_MARKER, false);
        tests.append_line("    A.B.First", false);
        tests.append_line("    A.B.Second", false);
        tests.request_cancel();
        let status = ProcessStatus {
            state: "exited",
            exit: None,
            termination: Some(Termination {
                requested: "SIGTERM".to_string(),
                observed: "exited".to_string(),
                exit: None,
                killed_after_grace: false,
            }),
            awaiting_decision: false,
        };
        assert_eq!(tests.complete(&status), "cancelled");
        let view = tests.view().discovery;
        assert_eq!(view.state, "cancelled");
        assert!(view.partial);
        assert_eq!(view.items.len(), 2);
        assert_eq!(view.exit, None);
        assert!(view.log.contains("A.B.First"));
    }

    #[test]
    fn sealed_and_stale_records_keep_items_and_drop_late_lines() {
        let tests = completed(&["A.B.First"]);
        tests.mark_stale("The project model was re-evaluated.");
        let stale = tests.view().discovery;
        assert_eq!(stale.state, "completed");
        assert!(stale.stale);
        assert_eq!(stale.items.len(), 1);

        tests.seal("Trust was revoked.", None);
        let finished_at = tests.view().discovery.finished_at;
        tests.append_line("    A.B.Late", false);
        let sealed = tests.view().discovery;
        assert!(sealed.sealed);
        assert_eq!(sealed.state, "completed");
        assert_eq!(sealed.items.len(), 1);
        assert_eq!(sealed.finished_at, finished_at);
        assert!(!sealed.log.contains("A.B.Late"));
    }

    #[test]
    fn a_late_old_record_never_touches_the_current_discovery() {
        let old = Tests::begin(start(1, "discovery:1:1"));
        old.append_line(LIST_MARKER, false);
        let current = Tests::begin(start(2, "discovery:1:2"));
        old.append_line("    A.B.Old", false);
        old.complete(&exited(Some(0)));
        let view = current.view().discovery;
        assert_eq!(view.revision, 2);
        assert!(view.items.is_empty());
        assert_eq!(view.discovery_id.as_deref(), Some("discovery:1:2"));
    }

    #[test]
    fn mtp_is_detected_from_project_text_and_evaluated_packages() {
        let packages = vec![PackageRef {
            name: "Microsoft.Testing.Platform.MSBuild".to_string(),
            version: Some("1.4.0".to_string()),
        }];
        assert!(mtp_detected("", &packages));
        assert!(mtp_detected(
            "<UseMicrosoftTestingPlatformRunner>true</UseMicrosoftTestingPlatformRunner>",
            &[]
        ));
        assert!(mtp_detected(
            "<PackageReference Include=\"Microsoft.Testing.Platform\" />",
            &[]
        ));
        assert!(!mtp_detected(
            "<PackageReference Include=\"Microsoft.NET.Test.Sdk\" />",
            &[]
        ));
    }

    #[test]
    fn adapter_detection_requires_test_sdk_and_a_known_adapter() {
        let sdk = PackageRef {
            name: "Microsoft.NET.Test.Sdk".to_string(),
            version: None,
        };
        let adapter = PackageRef {
            name: "xunit.runner.visualstudio".to_string(),
            version: None,
        };
        assert!(adapter_missing(&[]).is_some());
        assert!(adapter_missing(&[sdk.clone()]).is_some());
        assert!(adapter_missing(&[sdk.clone()])
            .unwrap()
            .contains("xunit.runner.visualstudio"));
        let guidance = adapter_missing(&[adapter.clone()]).unwrap();
        assert!(guidance.contains("Microsoft.NET.Test.Sdk"));
        assert!(adapter_missing(&[sdk, adapter]).is_none());
    }
}
