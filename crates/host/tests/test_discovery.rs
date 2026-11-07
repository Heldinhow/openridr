//! Boundary tests for real VSTest discovery: explicit scope, restore/trust gates, MTP and
//! adapter preflights, empty/failure distinction and observed cancellation/sealing.

#[path = "support/mod.rs"]
mod support;

use openridr_host::{Host, HostError};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn fixture() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/OrderDesk"))
        .expect("fixture OrderDesk exists")
}

fn outside_fixture() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/OutsideSolution"))
        .expect("fixture OutsideSolution exists")
}

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

fn test_project(dir: &Path, name: &str, extra: &str, source: &str) {
    global_json_8(dir);
    fs::write(
        dir.join(format!("{name}.csproj")),
        format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework><IsTestProject>true</IsTestProject></PropertyGroup>{extra}</Project>\n"
        ),
    )
    .unwrap();
    fs::write(dir.join(format!("{name}.cs")), source).unwrap();
}

const ADAPTER_PACKAGES: &str = "<ItemGroup>\
<PackageReference Include=\"Microsoft.NET.Test.Sdk\" Version=\"17.11.1\" />\
<PackageReference Include=\"xunit\" Version=\"2.9.2\" />\
<PackageReference Include=\"xunit.runner.visualstudio\" Version=\"2.8.2\" />\
</ItemGroup>";

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

fn error_code(result: Result<Value, HostError>) -> String {
    result.expect_err("expected host error").code
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

/// Explicit restore + evaluation, exactly what a user does before discovery.
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
    let state = snapshot(host);
    assert_eq!(
        state["evaluation"]["state"], "ok",
        "{}",
        state["evaluation"]["log"]
    );
    state
}

fn wait_for_discovery(host: &Host, timeout: Duration) -> Value {
    let deadline = Instant::now() + timeout;
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

fn project_path(discovery: &Value) -> String {
    discovery["project"]["path"].as_str().unwrap().to_string()
}

fn discovery_processes(state: &Value) -> Vec<Value> {
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
fn discovery_is_denied_before_trust_and_before_the_explicit_restore() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    test_project(
        &root,
        "Policy",
        ADAPTER_PACKAGES,
        "public sealed class C { }\n",
    );
    let host = support::host();
    open(&host, &root.join("Policy.csproj"));

    let denied = host
        .dispatch("tests.discover", json!({ "project": "Policy.csproj" }))
        .expect_err("discovery is trust-gated");
    assert_eq!(denied.code, "trust_required");
    assert_eq!(snapshot(&host)["tests"]["discovery"]["revision"], 0);
    assert!(discovery_processes(&snapshot(&host)).is_empty());
    assert!(!root.join("obj").exists());

    grant(&host);
    let error = host
        .dispatch("tests.discover", json!({ "project": "Policy.csproj" }))
        .expect_err("a project without assets must not be discovered");
    assert_eq!(error.code, "restore_required");
    let details = error.details.unwrap();
    assert_eq!(details["consumer"], "tests.discover");
    assert_eq!(details["action"]["method"], "restore.start");
    assert_eq!(snapshot(&host)["tests"]["discovery"]["revision"], 0);
    assert!(discovery_processes(&snapshot(&host)).is_empty());
    assert!(!root.join("obj").exists(), "discovery must not restore");

    // Cancelling discovery is a safety action: no trust, only "nothing is running".
    assert_eq!(
        error_code(host.dispatch("tests.discover_cancel", json!({}))),
        "invalid_state"
    );
}

#[test]
fn discovery_requires_an_explicit_test_project_and_a_matching_evaluated_tfm() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk scope");
    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    prepare(&host, Some("net8.0"));

    let missing = host
        .dispatch("tests.discover", json!({}))
        .expect_err("discovery never guesses a project");
    assert_eq!(missing.code, "invalid_state");
    assert!(missing.message.contains("explicitly"));

    let console = host
        .dispatch(
            "tests.discover",
            json!({ "project": "src/OrderDesk.Cli/OrderDesk.Cli.csproj" }),
        )
        .expect_err("a console project is not a test project");
    assert_eq!(console.code, "invalid_state");
    assert!(console.message.contains("only an evaluated test project"));

    let tfm = host
        .dispatch(
            "tests.discover",
            json!({
                "project": "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
                "targetFramework": "net9.0",
            }),
        )
        .expect_err("an unevaluated TFM is not accepted");
    assert_eq!(tfm.code, "invalid_state");
    assert_eq!(tfm.details.unwrap()["reason"], "tfm_not_evaluated");
    assert!(discovery_processes(&snapshot(&host)).is_empty());
}

#[test]
fn a_test_project_outside_the_solution_is_discovered_and_runs() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = temp.path().join("OutsideSolution");
    copy_tree(&outside_fixture(), &fixture);
    let app = fs::canonicalize(fixture.join("app")).unwrap();
    let project = fs::canonicalize(fixture.join("tests"))
        .unwrap()
        .join("Lib.Tests.csproj");

    let host = support::host();
    open(&host, &app.join("App.sln"));
    grant(&host);
    prepare(&host, None);
    let project = project.to_string_lossy().to_string();

    host.dispatch("tests.discover", json!({ "project": project }))
        .unwrap();
    let discovery = wait_for_discovery(&host, Duration::from_secs(240));
    assert_eq!(discovery["state"], "completed", "{}", discovery["log"]);
    // The model names the project by its portable path; the caller sent the canonical one.
    assert_eq!(
        discovery["project"]["path"],
        support::portable(std::path::Path::new(&project))
    );
    let items = discovery["items"].as_array().unwrap();
    assert_eq!(items.len(), 1, "{discovery}");
    assert!(
        items[0]["displayName"]
            .as_str()
            .is_some_and(|name| name.contains("ForNamesThePerson")),
        "{discovery}"
    );

    host.dispatch(
        "tests.run",
        json!({ "project": project, "selection": "all" }),
    )
    .unwrap();
    let run = wait_for_run(&host, Duration::from_secs(240));
    assert_eq!(run["state"], "completed", "{}", run["log"]);
    assert_eq!(run["exit"], 0);
    assert_eq!(run["counts"]["passed"], 1, "{}", run["log"]);
    assert_eq!(run["counts"]["failed"], 0);
}

#[cfg(target_os = "macos")]
#[test]
fn a_single_target_project_is_discovered_on_its_evaluated_tfm() {
    // No TFM is selected in the evaluated context; the project declares exactly one, so discovery
    // names it instead of leaving every test row and the panel at "TFM not selected".
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk single tfm");
    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    let evaluated = prepare(&host, None);
    assert_eq!(
        evaluated["evaluation"]["context"]["targetFramework"],
        Value::Null
    );

    let started = host
        .dispatch(
            "tests.discover",
            json!({ "project": "tests/OrderDesk.Tests/OrderDesk.Tests.csproj" }),
        )
        .unwrap();
    assert_eq!(
        started["tests"]["discovery"]["targetFramework"], "net8.0",
        "the single declared TFM is the discovered scope"
    );
    let discovery = wait_for_discovery(&host, Duration::from_secs(120));
    assert_eq!(discovery["state"], "completed", "{}", discovery["log"]);
    assert_eq!(discovery["targetFramework"], "net8.0");
    let command: Vec<&str> = discovery["command"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert!(command.contains(&"-f"), "{command:?}");
    assert!(command.contains(&"net8.0"), "{command:?}");
    // Every row reports the framework, so the tree and the detail panel never fall back.
    let items = discovery["items"].as_array().unwrap();
    assert_eq!(items.len(), 4);
    assert!(
        items.iter().all(|item| item["tfm"] == "net8.0"),
        "{discovery}"
    );

    // A run without a TFM uses the same scope as the discovery it validates against.
    host.dispatch(
        "tests.run",
        json!({
            "project": "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
            "selection": "selected",
            "testIds": [items[0]["id"]],
        }),
    )
    .expect("a run without a TFM uses the discovery's own scope");
    let run = wait_for_run(&host, Duration::from_secs(120));
    assert_eq!(run["state"], "completed", "{}", run["log"]);
    assert_eq!(run["targetFramework"], "net8.0");
}

#[cfg(target_os = "macos")]
#[test]
fn a_multi_target_project_needs_an_explicit_tfm_before_any_command() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    global_json_8(&root);
    fs::write(
        root.join("Multi.csproj"),
        format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFrameworks>net8.0;netstandard2.0</TargetFrameworks><IsTestProject>true</IsTestProject></PropertyGroup>{ADAPTER_PACKAGES}</Project>\n"
        ),
    )
    .unwrap();
    fs::write(root.join("Multi.cs"), "public sealed class C { }\n").unwrap();
    let host = support::host();
    open(&host, &root.join("Multi.csproj"));
    grant(&host);
    prepare(&host, None);

    let error = host
        .dispatch("tests.discover", json!({ "project": "Multi.csproj" }))
        .expect_err("a multi-target project must not pick a TFM");
    assert_eq!(error.code, "invalid_state");
    assert_eq!(error.details.unwrap()["reason"], "tfm_required");
    assert!(error.message.contains("net8.0, netstandard2.0"));
    assert!(discovery_processes(&snapshot(&host)).is_empty());
}

#[cfg(target_os = "macos")]
#[test]
fn real_vstest_discovery_lists_the_fixture_tests_with_stable_identities() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk discovery");
    let canonical = fs::canonicalize(&copy).unwrap();
    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    prepare(&host, Some("net8.0"));

    let started = host
        .dispatch(
            "tests.discover",
            json!({
                "project": "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
                "targetFramework": "net8.0",
            }),
        )
        .unwrap();
    let running = started["tests"]["discovery"].clone();
    assert_eq!(running["state"], "discovering");
    let discovery = wait_for_discovery(&host, Duration::from_secs(120));
    assert_eq!(discovery["state"], "completed", "{}", discovery["log"]);
    assert_eq!(discovery["exit"], 0);
    assert_eq!(discovery["partial"], false);
    assert_eq!(discovery["project"]["name"], "OrderDesk.Tests");
    assert_eq!(
        project_path(&discovery),
        "tests/OrderDesk.Tests/OrderDesk.Tests.csproj"
    );
    assert_eq!(discovery["targetFramework"], "net8.0");
    assert_eq!(discovery["discoveryId"], "discovery:1:1");
    assert_eq!(discovery["contextId"], 2);
    assert_eq!(discovery["sdkVersion"], "8.0.408");
    assert_eq!(
        discovery["cwd"].as_str().unwrap(),
        canonical.to_string_lossy()
    );
    let command: Vec<String> = discovery["command"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_string())
        .collect();
    assert!(command[0].ends_with("/dotnet"), "{}", command[0]);
    assert_eq!(command[1], "test");
    assert_eq!(
        command[2],
        canonical
            .join("tests/OrderDesk.Tests/OrderDesk.Tests.csproj")
            .to_string_lossy()
    );
    assert_eq!(
        &command[3..],
        &[
            "-c",
            "Debug",
            "--no-restore",
            "--list-tests",
            "--nologo",
            "-v:minimal",
            "-f",
            "net8.0"
        ]
    );

    let items = discovery["items"].as_array().unwrap();
    assert_eq!(items.len(), 4, "{}", discovery["log"]);
    let names: Vec<&str> = items
        .iter()
        .map(|item| item["fullyQualifiedName"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        vec![
            "OrderDesk.Tests.OrderTests.Id_is_preserved",
            "OrderDesk.Tests.OrderTests.Total_is_preserved",
            "OrderDesk.Tests.OrderTests.Deliberate_failure_for_runner_evidence",
            "OrderDesk.Tests.OrderTests.Skipped_until_fixture_changes",
        ]
    );
    for item in items {
        assert_eq!(item["source"], "vstest");
        assert_eq!(
            item["project"],
            "tests/OrderDesk.Tests/OrderDesk.Tests.csproj"
        );
        assert_eq!(item["tfm"], "net8.0");
        assert_eq!(item["discoveryId"], "discovery:1:1");
        assert_eq!(item["revision"], 1);
        assert_eq!(item["displayName"], item["fullyQualifiedName"]);
    }
    assert!(discovery["log"]
        .as_str()
        .unwrap()
        .contains("The following Tests are available:"));
    assert!(discovery["log"]
        .as_str()
        .unwrap()
        .contains("Deliberate_failure_for_runner_evidence"));

    // A second discovery is a new operation with the same stable test identities.
    host.dispatch(
        "tests.discover",
        json!({
            "project": "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
            "targetFramework": "net8.0",
        }),
    )
    .unwrap();
    let second = wait_for_discovery(&host, Duration::from_secs(120));
    assert_eq!(second["state"], "completed");
    assert_eq!(second["discoveryId"], "discovery:1:2");
    for (first, again) in items.iter().zip(second["items"].as_array().unwrap()) {
        assert_eq!(first["id"], again["id"]);
        assert_eq!(again["revision"], 2);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn mtp_is_refused_without_spawning_any_vstest_process() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    test_project(
        &root,
        "Mtp",
        "<PropertyGroup><UseMicrosoftTestingPlatformRunner>true</UseMicrosoftTestingPlatformRunner></PropertyGroup>",
        "public sealed class C { }\n",
    );
    let host = support::host();
    open(&host, &root.join("Mtp.csproj"));
    grant(&host);
    prepare(&host, None);

    let state = host
        .dispatch("tests.discover", json!({ "project": "Mtp.csproj" }))
        .unwrap();
    let discovery = state["tests"]["discovery"].clone();
    assert_eq!(discovery["state"], "mtp_unsupported");
    assert_eq!(
        discovery["mtpMessage"],
        "Microsoft.Testing.Platform is not supported in this version. Use the terminal."
    );
    assert_eq!(discovery["command"], json!([]));
    assert_eq!(discovery["exit"], Value::Null);
    assert!(discovery["items"].as_array().unwrap().is_empty());
    assert!(discovery_processes(&state).is_empty());
    assert!(!discovery["log"].as_str().unwrap().contains("Test run for"));
    assert!(!root.join("tests").exists());
}

#[cfg(target_os = "macos")]
#[test]
fn a_project_without_a_vstest_adapter_is_adapter_missing_not_empty() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    test_project(&root, "NoAdapter", "", "public sealed class C { }\n");
    let host = support::host();
    open(&host, &root.join("NoAdapter.csproj"));
    grant(&host);
    prepare(&host, None);

    let state = host
        .dispatch("tests.discover", json!({ "project": "NoAdapter.csproj" }))
        .unwrap();
    let discovery = state["tests"]["discovery"].clone();
    assert_eq!(discovery["state"], "adapter_missing");
    assert_eq!(discovery["command"], json!([]));
    let error = discovery["error"].as_str().unwrap();
    assert!(error.contains("Microsoft.NET.Test.Sdk"), "{error}");
    assert!(error.contains("xunit.runner.visualstudio"), "{error}");
    assert!(
        error.contains("does not add or install packages"),
        "{error}"
    );
    assert_eq!(discovery["exit"], Value::Null);
    assert!(discovery_processes(&state).is_empty());
}

#[cfg(target_os = "macos")]
#[test]
fn an_adapter_with_no_tests_is_empty_distinct_from_failure() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    test_project(
        &root,
        "Empty",
        ADAPTER_PACKAGES,
        "public sealed class C { }\n",
    );
    let host = support::host();
    open(&host, &root.join("Empty.csproj"));
    grant(&host);
    prepare(&host, None);

    host.dispatch("tests.discover", json!({ "project": "Empty.csproj" }))
        .unwrap();
    let discovery = wait_for_discovery(&host, Duration::from_secs(120));
    assert_eq!(discovery["state"], "empty", "{}", discovery["log"]);
    assert_eq!(discovery["exit"], 0);
    assert!(discovery["items"].as_array().unwrap().is_empty());
    assert!(discovery["log"]
        .as_str()
        .unwrap()
        .contains("The following Tests are available:"));
    assert!(discovery["error"]
        .as_str()
        .unwrap()
        .contains("not a discovery failure"));

    // A real compile failure in the same project is `failed`, never an empty success.
    fs::write(
        root.join("Broken.cs"),
        "public sealed class Broken { private MissingType value; }\n",
    )
    .unwrap();
    host.dispatch("tests.discover", json!({ "project": "Empty.csproj" }))
        .unwrap();
    let failed = wait_for_discovery(&host, Duration::from_secs(120));
    assert_eq!(failed["state"], "failed", "{}", failed["log"]);
    assert_eq!(failed["exit"], 1);
    assert!(failed["log"].as_str().unwrap().contains("CS0246"));
    assert!(failed["error"].as_str().unwrap().contains("exited 1"));
}

#[cfg(unix)]
#[test]
fn cancelling_a_hanging_discovery_keeps_partial_state() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    test_project(
        &root,
        "Hang",
        &format!(
            "{ADAPTER_PACKAGES}<Target Name=\"Hang\" AfterTargets=\"Build\"><Exec Command=\"sleep 30\" /></Target>"
        ),
        "public sealed class C { }\n",
    );
    let host = support::host();
    open(&host, &root.join("Hang.csproj"));
    grant(&host);
    prepare(&host, None);

    host.dispatch("tests.discover", json!({ "project": "Hang.csproj" }))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let state = snapshot(&host);
        if state["tests"]["discovery"]["processId"].is_u64()
            && !state["tests"]["discovery"]["log"]
                .as_str()
                .unwrap_or_default()
                .is_empty()
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "discovery process never started: {}",
            state["tests"]["discovery"]
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    let cancelled = host.dispatch("tests.discover_cancel", json!({})).unwrap();
    let discovery = cancelled["tests"]["discovery"].clone();
    assert_eq!(discovery["state"], "cancelled", "{}", discovery["log"]);
    assert_eq!(discovery["partial"], true);
    assert_eq!(discovery["termination"]["requested"], "SIGTERM");
    assert_eq!(discovery["termination"]["observed"], "exited");
    // A killed process may have no code at all; `null` is exit unknown, never zero.
    assert!(
        discovery["exit"].is_null() || discovery["exit"].is_i64(),
        "{}",
        discovery["exit"]
    );
    assert!(!discovery["log"].as_str().unwrap().is_empty());
    assert!(cancelled["processes"]
        .as_array()
        .unwrap()
        .iter()
        .any(|process| process["kind"] == "tests:Hang" && process["state"] == "exited"));

    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(
        snapshot(&host)["tests"]["discovery"]["state"],
        "cancelled",
        "a late result replaced the cancelled discovery"
    );
}

#[cfg(unix)]
#[test]
fn revoking_trust_seals_a_running_discovery_and_keeps_its_log() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    test_project(
        &root,
        "Hang",
        &format!(
            "{ADAPTER_PACKAGES}<Target Name=\"Hang\" AfterTargets=\"Build\"><Exec Command=\"sleep 30\" /></Target>"
        ),
        "public sealed class C { }\n",
    );
    let host = support::host();
    open(&host, &root.join("Hang.csproj"));
    grant(&host);
    prepare(&host, None);

    host.dispatch("tests.discover", json!({ "project": "Hang.csproj" }))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        let state = snapshot(&host);
        if state["tests"]["discovery"]["processId"].is_u64()
            && !state["tests"]["discovery"]["log"]
                .as_str()
                .unwrap_or_default()
                .is_empty()
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "discovery process never started: {}",
            state["tests"]["discovery"]
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    let revoked = host.dispatch("trust.revoke", json!({})).unwrap();
    let discovery = revoked["tests"]["discovery"].clone();
    assert_eq!(revoked["trust"]["granted"], false);
    assert_eq!(discovery["state"], "cancelled", "{}", discovery["log"]);
    assert_eq!(discovery["sealed"], true);
    assert_eq!(discovery["stale"], true);
    assert_eq!(discovery["partial"], true);
    assert!(discovery["log"].as_str().unwrap().contains("Hang"));
    let finished_at = discovery["finishedAt"].clone();

    std::thread::sleep(Duration::from_millis(700));
    let later = snapshot(&host)["tests"]["discovery"].clone();
    assert_eq!(later["state"], "cancelled");
    assert_eq!(later["sealed"], true);
    assert_eq!(later["finishedAt"], finished_at);
    assert_eq!(
        later["log"], discovery["log"],
        "late output changed a sealed log"
    );
}
