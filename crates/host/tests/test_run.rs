//! Boundary tests for real VSTest execution: trust/restore gates, MTP, all/selected scope,
//! TRX outcomes and positions, observed cancellation and sealing.

#[path = "support/mod.rs"]
mod support;

use openridr_host::Host;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// Only the macOS/unix leaves below use it, so it is dead code on the other targets.
#[cfg(unix)]
fn fixture() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/OrderDesk"))
        .expect("fixture OrderDesk exists")
}

// Only the macOS/unix leaves below use it, so it is dead code on the other targets.
#[cfg(unix)]
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        if name == "bin" || name == "obj" {
            continue;
        }
        let target = to.join(&name);
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

// Only the macOS/unix leaves below use it, so it is dead code on the other targets.
#[cfg(unix)]
fn fixture_copy(parent: &Path, name: &str) -> PathBuf {
    let copy = parent.join(name);
    copy_tree(&fixture(), &copy);
    copy
}

fn global_json_8(dir: &Path) {
    fs::write(
        dir.join("global.json"),
        "{\"sdk\":{\"version\":\"8.0.408\",\"rollForward\":\"disable\"}}\n",
    )
    .unwrap();
}

const ADAPTER_PACKAGES: &str = "<ItemGroup>\
<PackageReference Include=\"Microsoft.NET.Test.Sdk\" Version=\"17.11.1\" />\
<PackageReference Include=\"xunit\" Version=\"2.9.2\" />\
<PackageReference Include=\"xunit.runner.visualstudio\" Version=\"2.8.2\" />\
</ItemGroup>";

fn test_project(dir: &Path, name: &str, extra: &str, source: &str) {
    global_json_8(dir);
    fs::write(
        dir.join(format!("{name}.csproj")),
        format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework><IsTestProject>true</IsTestProject></PropertyGroup>{extra}{ADAPTER_PACKAGES}</Project>\n"
        ),
    )
    .unwrap();
    fs::write(dir.join(format!("{name}.cs")), source).unwrap();
}

fn open(host: &Host, path: &Path) -> Value {
    let opened = host
        .dispatch("workspace.open", json!({ "path": path }))
        .expect("workspace opens");
    host.allow_setup_for_tests();
    opened
}

fn grant(host: &Host) -> Value {
    host.dispatch("trust.grant", json!({}))
        .expect("trust is granted")
}

fn snapshot(host: &Host) -> Value {
    host.dispatch("workspace.state", json!({})).unwrap()
}

fn wait_for_restore(host: &Host, timeout: Duration) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let restore = snapshot(host)["restore"].clone();
        if restore["state"] != "running" {
            return restore;
        }
        assert!(
            Instant::now() < deadline,
            "restore did not finish: {}",
            restore["log"]
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn prepare(host: &Host, target_framework: Option<&str>) -> Value {
    let started = host.dispatch("restore.start", json!({})).unwrap();
    assert_eq!(started["restore"]["state"], "running");
    let restored = wait_for_restore(host, Duration::from_secs(90));
    assert_eq!(restored["state"], "ok", "{}", restored["log"]);
    host.dispatch(
        "evaluation.run",
        json!({ "configuration": "Debug", "targetFramework": target_framework }),
    )
    .unwrap();
    snapshot(host)
}

// Only the macOS/unix leaves below use it, so it is dead code on the other targets.
#[cfg(unix)]
const DISCOVER: &str = "tests/OrderDesk.Tests/OrderDesk.Tests.csproj";

// Only the macOS/unix leaves below use it, so it is dead code on the other targets.
#[cfg(unix)]
fn discover_fixture(host: &Host) -> Value {
    host.dispatch(
        "tests.discover",
        json!({ "project": DISCOVER, "targetFramework": "net8.0" }),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let discovery = snapshot(host)["tests"]["discovery"].clone();
        if discovery["state"] != "discovering" {
            assert_eq!(discovery["state"], "completed", "{}", discovery["log"]);
            return discovery;
        }
        assert!(
            Instant::now() < deadline,
            "discovery did not finish: {}",
            discovery["log"]
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

// Only the macOS/unix leaves below use it, so it is dead code on the other targets.
#[cfg(unix)]
fn wait_for_run(host: &Host, timeout: Duration) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let run = snapshot(host)["tests"]["run"].clone();
        if run["state"] != "running" {
            return run;
        }
        assert!(
            Instant::now() < deadline,
            "test run did not finish: {}",
            run["log"]
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn test_processes(state: &Value) -> Vec<Value> {
    state["processes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|process| {
            process["kind"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("tests:"))
        })
        .cloned()
        .collect()
}

#[test]
fn run_is_denied_before_trust_and_before_the_explicit_restore() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    test_project(&root, "Policy", "", "public sealed class C { }\n");
    let host = support::host();
    open(&host, &root.join("Policy.csproj"));

    let denied = host
        .dispatch("tests.run", json!({ "selection": "all" }))
        .expect_err("run is trust-gated");
    assert_eq!(denied.code, "trust_required");
    assert_eq!(snapshot(&host)["tests"]["run"]["revision"], 0);
    assert!(test_processes(&snapshot(&host)).is_empty());
    assert!(!root.join("obj").exists());

    grant(&host);
    let error = host
        .dispatch(
            "tests.run",
            json!({ "project": "Policy.csproj", "selection": "all" }),
        )
        .expect_err("a project without assets must not run");
    assert_eq!(error.code, "restore_required");
    let details = error.details.unwrap();
    assert_eq!(details["consumer"], "tests.run");
    assert_eq!(details["action"]["method"], "restore.start");
    assert_eq!(snapshot(&host)["tests"]["run"]["revision"], 0);
    assert!(test_processes(&snapshot(&host)).is_empty());
    assert!(!root.join("obj").exists(), "run must not restore");

    let cancel = host
        .dispatch("tests.run_cancel", json!({}))
        .expect_err("nothing is running");
    assert_eq!(cancel.code, "invalid_state");
}

#[test]
fn mtp_is_refused_before_any_vstest_process_even_with_a_direct_run_call() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    test_project(
        &root,
        "MtpRun",
        "<PropertyGroup><UseMicrosoftTestingPlatformRunner>true</UseMicrosoftTestingPlatformRunner></PropertyGroup>",
        "public sealed class C { }\n",
    );
    let host = support::host();
    open(&host, &root.join("MtpRun.csproj"));
    grant(&host);
    prepare(&host, None);

    let discovered = host
        .dispatch("tests.discover", json!({ "project": "MtpRun.csproj" }))
        .unwrap();
    assert_eq!(discovered["tests"]["discovery"]["state"], "mtp_unsupported");

    let error = host
        .dispatch(
            "tests.run",
            json!({ "project": "MtpRun.csproj", "selection": "all" }),
        )
        .expect_err("MTP is never run through VSTest");
    assert_eq!(error.code, "invalid_state");
    assert_eq!(
        error.message,
        "Microsoft.Testing.Platform is not supported in this version. Use the terminal."
    );
    assert_eq!(error.details.unwrap()["reason"], "mtp_unsupported");
    let state = snapshot(&host);
    assert_eq!(state["tests"]["run"]["revision"], 0);
    assert!(test_processes(&state).is_empty());
}

#[cfg(target_os = "macos")]
#[test]
fn real_run_reports_trx_outcomes_positions_and_only_the_selected_identity() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk run");
    let canonical = fs::canonicalize(&copy).unwrap();
    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    prepare(&host, Some("net8.0"));
    let discovery = discover_fixture(&host);
    let items = discovery["items"].as_array().unwrap().clone();

    let started = host
        .dispatch(
            "tests.run",
            json!({
                "project": DISCOVER,
                "targetFramework": "net8.0",
                "selection": "all",
            }),
        )
        .unwrap();
    assert_eq!(started["tests"]["run"]["state"], "running");
    let run = wait_for_run(&host, Duration::from_secs(120));
    assert_eq!(run["state"], "completed", "{}", run["log"]);
    assert_eq!(run["exit"], 1);
    assert_eq!(run["infrastructureFailure"], false);
    assert_eq!(run["results"].as_array().unwrap().len(), 4);
    let counts = run["counts"].clone();
    assert_eq!(counts["passed"], 2, "{run}");
    assert_eq!(counts["failed"], 1, "{run}");
    assert_eq!(counts["skipped"], 1, "{run}");
    assert_eq!(counts["notExecuted"], 0, "{run}");
    assert_eq!(counts["matched"], 4, "{run}");
    assert_eq!(run["selection"], "all");
    assert_eq!(run["runId"], "testrun:1:1");
    assert_eq!(run["discoveryId"], discovery["discoveryId"]);
    assert_eq!(run["discoveryRevision"], 1);
    assert_eq!(run["contextId"], 2);

    let failure = run["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|result| result["outcome"] == "failed")
        .expect("one real failed test");
    assert_eq!(
        failure["fullyQualifiedName"],
        "OrderDesk.Tests.OrderTests.Deliberate_failure_for_runner_evidence"
    );
    let message = failure["message"].as_str().unwrap();
    assert!(message.contains("Assert.Equal() Failure"), "{message}");
    assert!(message.contains("Expected: 43"), "{message}");
    let stack = failure["stackTrace"].as_str().unwrap();
    assert!(stack.contains("OrderTests.cs:line 29"), "{stack}");
    assert_eq!(
        failure["position"]["path"],
        "tests/OrderDesk.Tests/OrderTests.cs"
    );
    assert_eq!(failure["position"]["line"], 28);
    assert_eq!(failure["testId"], items[2]["id"]);

    let skipped = run["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|result| result["outcome"] == "skipped")
        .expect("one skipped test");
    assert_eq!(skipped["outcomeRaw"], "NotExecuted");
    assert_eq!(
        skipped["fullyQualifiedName"],
        "OrderDesk.Tests.OrderTests.Skipped_until_fixture_changes"
    );
    assert_eq!(
        skipped["message"],
        "Fixture evidence: skipped on purpose until the fixture changes."
    );

    let command: Vec<&str> = run["command"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert!(command[0].ends_with("/dotnet"), "{}", command[0]);
    assert!(command.contains(&"--no-build"));
    assert!(command.contains(&"--no-restore"));
    assert!(command.contains(&"--logger"));
    let trx_path = run["trxPath"].as_str().unwrap();
    assert!(trx_path.ends_with("openridr.trx"));
    assert!(!trx_path.contains("/fixtures/"));
    assert!(Path::new(trx_path).is_file(), "{trx_path}");
    assert!(Path::new(run["logPath"].as_str().unwrap()).is_file());
    assert!(run["log"].as_str().unwrap().contains("Test run for"));
    assert_eq!(run["cwd"], canonical.to_string_lossy().as_ref());

    // A selected run sends an exact filter and reports only the chosen identity.
    let selected_id = items[0]["id"].as_str().unwrap().to_string();
    host.dispatch(
        "tests.run",
        json!({
            "project": DISCOVER,
            "targetFramework": "net8.0",
            "selection": "selected",
            "testIds": [selected_id],
        }),
    )
    .unwrap();
    let selected = wait_for_run(&host, Duration::from_secs(120));
    assert_eq!(selected["state"], "completed", "{}", selected["log"]);
    assert_eq!(selected["exit"], 0);
    assert_eq!(selected["results"].as_array().unwrap().len(), 1);
    assert_eq!(selected["results"][0]["outcome"], "passed");
    assert_eq!(selected["results"][0]["testId"], items[0]["id"]);
    assert_eq!(selected["counts"]["passed"], 1);
    let filter = selected["command"]
        .as_array()
        .unwrap()
        .iter()
        .position(|value| value == "--filter")
        .map(|index| selected["command"][index + 1].as_str().unwrap().to_string())
        .expect("filter argument");
    assert_eq!(
        filter,
        format!(
            "FullyQualifiedName={}",
            items[0]["fullyQualifiedName"].as_str().unwrap()
        )
    );

    // Empty, unknown and duplicate selections block and never fall back to all.
    for (ids, reason) in [
        (json!([]), "empty_selection"),
        (json!(["test:not-current"]), "invalid_selection"),
        (
            json!([selected_id.clone(), selected_id]),
            "duplicate_selection",
        ),
    ] {
        let error = host
            .dispatch(
                "tests.run",
                json!({
                    "project": DISCOVER,
                    "targetFramework": "net8.0",
                    "selection": "selected",
                    "testIds": ids,
                }),
            )
            .expect_err("an invalid selection is refused");
        assert_eq!(error.code, "invalid_state");
        assert_eq!(error.details.unwrap()["reason"], reason);
    }
    assert_eq!(snapshot(&host)["tests"]["run"]["runId"], "testrun:1:2");
}

/// The export is the file equivalent of the Tests panel: it must carry the discovery and the
/// real last run. Reading the discovery view alone always yields `run.revision == 0`.
#[cfg(target_os = "macos")]
#[test]
fn export_carries_the_discovery_and_the_real_last_run() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk export");
    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    prepare(&host, Some("net8.0"));
    let discovery = discover_fixture(&host);

    // Before any run there is nothing to invent: the idle run is exported as idle.
    let before = host.dispatch("tests.export", json!({})).unwrap();
    assert_eq!(before["runRevision"], 0);
    let before_payload: Value =
        serde_json::from_slice(&fs::read(before["path"].as_str().unwrap()).unwrap()).unwrap();
    assert_eq!(before_payload["run"]["revision"], 0);

    host.dispatch(
        "tests.run",
        json!({
            "project": DISCOVER,
            "targetFramework": "net8.0",
            "selection": "all",
        }),
    )
    .unwrap();
    let run = wait_for_run(&host, Duration::from_secs(120));
    assert_eq!(run["state"], "completed", "{}", run["log"]);
    assert_eq!(run["counts"]["failed"], 1, "{run}");

    let exported = host.dispatch("tests.export", json!({})).unwrap();
    let path = exported["path"].as_str().unwrap();
    assert_eq!(exported["runRevision"], run["revision"]);
    assert_eq!(exported["discoveryRevision"], discovery["revision"]);
    assert_eq!(exported["identity"], run["runId"]);
    // The file name is the sanitized run identity; the discovery identity would mean a null run id.
    let run_id = run["runId"].as_str().unwrap();
    let safe: String = run_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    assert!(path.ends_with(&format!("tests-{safe}.json")), "{path}");
    let payload: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();

    // Discovery comes from the same export the panel shows.
    assert_eq!(
        payload["discovery"]["discoveryId"],
        discovery["discoveryId"]
    );
    assert_eq!(payload["discovery"]["items"].as_array().unwrap().len(), 4);
    assert_eq!(payload["discovery"]["targetFramework"], "net8.0");

    // The run is the real one, not the idle view.
    let file_run = &payload["run"];
    assert_eq!(file_run["revision"], run["revision"]);
    assert_eq!(file_run["runId"], run["runId"]);
    assert_eq!(file_run["state"], "completed");
    assert_eq!(file_run["counts"]["total"], 4, "{file_run}");
    assert_eq!(file_run["counts"]["passed"], 2);
    assert_eq!(file_run["counts"]["failed"], 1);
    assert_eq!(file_run["counts"]["skipped"], 1);
    assert_eq!(file_run["results"].as_array().unwrap().len(), 4);
    assert!(!file_run["command"].as_array().unwrap().is_empty());
}

/// The export must observe the process it reports: a run that exited after the last snapshot is
/// still `running` in the stored state, and writing that state to the file would be a lie.
#[cfg(target_os = "macos")]
#[test]
fn export_observes_a_finished_run_without_an_intervening_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk export sync");
    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    prepare(&host, Some("net8.0"));
    discover_fixture(&host);
    host.dispatch(
        "tests.run",
        json!({
            "project": DISCOVER,
            "targetFramework": "net8.0",
            "selection": "all",
        }),
    )
    .unwrap();
    // Wait for the run to be observed as running, then let the process finish without asking the
    // host for a snapshot: nothing but the export itself may notice the exit.
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let run = snapshot(&host)["tests"]["run"].clone();
        if run["state"] == "running" && run["processId"].is_u64() {
            break;
        }
        assert!(Instant::now() < deadline, "test run never started: {run}");
        std::thread::sleep(Duration::from_millis(50));
    }
    std::thread::sleep(Duration::from_secs(20));

    let exported = host.dispatch("tests.export", json!({})).unwrap();
    let payload: Value =
        serde_json::from_slice(&fs::read(exported["path"].as_str().unwrap()).unwrap()).unwrap();
    assert_eq!(payload["run"]["state"], "completed", "{}", payload["run"]);
    assert_eq!(payload["run"]["counts"]["total"], 4);
    assert_eq!(payload["run"]["results"].as_array().unwrap().len(), 4);
    assert_eq!(payload["run"]["exit"], 1);
}

#[cfg(target_os = "macos")]
#[test]
fn duplicate_display_names_are_one_ambiguous_identity_never_a_selected_run() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    test_project(
        &root,
        "Disp",
        "",
        "using Xunit;\nnamespace Disp;\npublic sealed class A { [Fact(DisplayName=\"Same display\")] public void One() {} }\npublic sealed class B { [Fact(DisplayName=\"Same display\")] public void Two() {} }\n",
    );
    let host = support::host();
    open(&host, &root.join("Disp.csproj"));
    grant(&host);
    prepare(&host, None);
    host.dispatch("tests.discover", json!({ "project": "Disp.csproj" }))
        .unwrap();
    let discovery = wait_for_run_discovery(&host);
    assert_eq!(discovery["state"], "completed", "{}", discovery["log"]);
    let items = discovery["items"].as_array().unwrap();
    assert_eq!(items.len(), 1, "VSTest printed the same display name twice");
    assert_eq!(items[0]["fullyQualifiedName"], "Same display");
    assert_eq!(items[0]["occurrences"], 2);
    assert_eq!(
        discovery["log"]
            .as_str()
            .unwrap()
            .matches("Same display")
            .count(),
        2
    );

    let error = host
        .dispatch(
            "tests.run",
            json!({
                "project": "Disp.csproj",
                "selection": "selected",
                "testIds": [items[0]["id"]],
            }),
        )
        .expect_err("an ambiguous name cannot target one test");
    assert_eq!(error.code, "invalid_state");
    assert_eq!(error.details.unwrap()["reason"], "ambiguous_selection");
    assert_eq!(snapshot(&host)["tests"]["run"]["revision"], 0);

    // Run all remains possible: no filter is sent, both real tests execute and are reported,
    // unmatched because the runner's list had no FQN for either.
    host.dispatch(
        "tests.run",
        json!({ "project": "Disp.csproj", "selection": "all" }),
    )
    .unwrap();
    let run = wait_for_run(&host, Duration::from_secs(120));
    assert_eq!(run["state"], "completed", "{}", run["log"]);
    assert_eq!(run["counts"]["total"], 2);
    assert_eq!(run["counts"]["passed"], 2);
    assert_eq!(run["counts"]["matched"], 0);
    let names: Vec<&str> = run["results"]
        .as_array()
        .unwrap()
        .iter()
        .map(|result| result["fullyQualifiedName"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"Disp.A.One"), "{names:?}");
    assert!(names.contains(&"Disp.B.Two"), "{names:?}");
}

// Only the macOS/unix leaves below use it, so it is dead code on the other targets.
#[cfg(unix)]
fn wait_for_run_discovery(host: &Host) -> Value {
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        let discovery = snapshot(host)["tests"]["discovery"].clone();
        if discovery["state"] != "discovering" {
            return discovery;
        }
        assert!(
            Instant::now() < deadline,
            "discovery did not finish: {}",
            discovery["log"]
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[cfg(unix)]
#[test]
fn cancel_keeps_partial_state_and_revocation_seals_a_running_test_run() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk cancel");
    fs::write(
        copy.join("tests/OrderDesk.Tests/LongTests.cs"),
        "using System.Threading.Tasks;\nusing Xunit;\n\nnamespace OrderDesk.Tests;\n\npublic sealed class LongTests\n{\n    [Fact]\n    public async Task Waits_for_cancellation()\n    {\n        await Task.Delay(60000);\n    }\n}\n",
    )
    .unwrap();
    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    prepare(&host, Some("net8.0"));
    let discovery = discover_fixture(&host);
    let long = discovery["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| {
            item["fullyQualifiedName"]
                .as_str()
                .unwrap()
                .ends_with("Waits_for_cancellation")
        })
        .expect("long test was discovered")
        .clone();

    host.dispatch(
        "tests.run",
        json!({
            "project": DISCOVER,
            "targetFramework": "net8.0",
            "selection": "selected",
            "testIds": [long["id"]],
        }),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let run = snapshot(&host)["tests"]["run"].clone();
        if run["processId"].is_u64() && !run["log"].as_str().unwrap_or_default().is_empty() {
            break;
        }
        assert!(Instant::now() < deadline, "test run never started: {run}");
        std::thread::sleep(Duration::from_millis(100));
    }
    let cancelled = host.dispatch("tests.run_cancel", json!({})).unwrap();
    let run = cancelled["tests"]["run"].clone();
    assert_eq!(run["state"], "cancelled", "{}", run["log"]);
    assert_eq!(run["termination"]["requested"], "SIGTERM");
    assert_eq!(run["termination"]["observed"], "exited");
    assert!(
        run["exit"].is_null() || run["exit"].is_i64(),
        "{}",
        run["exit"]
    );
    assert_eq!(run["counts"]["passed"], 0, "{run}");
    assert!(
        run["results"]
            .as_array()
            .unwrap()
            .iter()
            .all(|result| result["outcome"] != "passed"),
        "a cancelled run reported a passed test: {run}"
    );
    assert!(run["error"].as_str().unwrap().contains("not a success"));
    assert!(!run["log"].as_str().unwrap().is_empty());
    assert!(Path::new(run["logPath"].as_str().unwrap()).is_file());
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(snapshot(&host)["tests"]["run"]["state"], "cancelled");

    // Start it again and revoke trust while the long test is running.
    host.dispatch(
        "tests.run",
        json!({
            "project": DISCOVER,
            "targetFramework": "net8.0",
            "selection": "selected",
            "testIds": [long["id"]],
        }),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let run = snapshot(&host)["tests"]["run"].clone();
        if run["state"] == "running"
            && run["processId"].is_u64()
            && !run["log"].as_str().unwrap_or_default().is_empty()
        {
            break;
        }
        assert!(Instant::now() < deadline, "test run never started: {run}");
        std::thread::sleep(Duration::from_millis(100));
    }
    let revoked = host.dispatch("trust.revoke", json!({})).unwrap();
    let sealed = revoked["tests"]["run"].clone();
    assert_eq!(revoked["trust"]["granted"], false);
    assert_eq!(sealed["state"], "cancelled", "{}", sealed["log"]);
    assert_eq!(sealed["sealed"], true);
    assert_eq!(sealed["stale"], true);
    assert!(!sealed["log"].as_str().unwrap().is_empty());
    let finished_at = sealed["finishedAt"].clone();
    std::thread::sleep(Duration::from_millis(700));
    let later = snapshot(&host)["tests"]["run"].clone();
    assert_eq!(later["state"], "cancelled");
    assert_eq!(later["finishedAt"], finished_at);
    assert_eq!(later["log"], sealed["log"]);
}
