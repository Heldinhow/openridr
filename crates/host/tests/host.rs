use openridr_host::{sdk, Host, EXECUTABLE_METHODS};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

fn fixture() -> PathBuf {
    fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/OrderDesk"),
    )
    .expect("fixture OrderDesk exists")
}

fn temp_workspace(projects: &[&str]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    for name in projects {
        fs::write(
            dir.path().join(name),
            "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n",
        )
        .unwrap();
        let stem = name.trim_end_matches(".csproj");
        fs::write(dir.path().join(format!("{stem}.cs")), "class C {}\n").unwrap();
    }
    dir
}

fn open(host: &Host, path: &Path) -> Value {
    host.dispatch("workspace.open", json!({ "path": path }))
        .expect("workspace opens")
}

fn error_code(result: Result<Value, openridr_host::HostError>) -> String {
    result.expect_err("expected host error").code
}

#[test]
fn text_outline_reads_solution_projects_without_evaluation() {
    let host = Host::new();
    let snapshot = open(&host, &fixture());

    assert_eq!(snapshot["outline"]["source"], "text");
    assert_eq!(snapshot["workspace"]["kind"], "solution");
    assert_eq!(snapshot["trust"]["granted"], false);
    assert_eq!(snapshot["workspace"]["mode"], "restricted");

    let projects = snapshot["outline"]["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 3);

    let cli = projects
        .iter()
        .find(|project| project["name"] == "OrderDesk.Cli")
        .unwrap();
    assert_eq!(cli["kind"], "console");
    assert_eq!(cli["tfms"], json!(["net8.0"]));
    assert_eq!(cli["references"], json!(["OrderDesk.Core"]));
    assert!(cli["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|file| file == "src/OrderDesk.Cli/Program.cs"));

    let core = projects
        .iter()
        .find(|project| project["name"] == "OrderDesk.Core")
        .unwrap();
    assert_eq!(core["kind"], "library");
    assert!(core["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|file| file == "src/OrderDesk.Core/Order.cs"));

    let tests = projects
        .iter()
        .find(|project| project["name"] == "OrderDesk.Tests")
        .unwrap();
    assert_eq!(tests["kind"], "test");
    assert!(tests["files"]
        .as_array()
        .unwrap()
        .iter()
        .any(|file| file == "tests/OrderDesk.Tests/OrderTests.cs"));

    assert_eq!(snapshot["outline"]["globalJson"]["version"], "8.0.408");
    assert_eq!(
        snapshot["outline"]["globalJson"]["rollForward"],
        "disable"
    );
}

#[test]
fn file_access_is_confined_to_the_workspace() {
    let host = Host::new();
    open(&host, &fixture());

    let read = host
        .dispatch("file.read", json!({ "path": "src/OrderDesk.Cli/Program.cs" }))
        .unwrap();
    assert!(read["content"].as_str().unwrap().contains("OrderDesk"));

    for escape in [
        "../../../../../../etc/hosts",
        "/etc/hosts",
        "src/OrderDesk.Cli/../../../../../../../../etc/hosts",
    ] {
        assert_eq!(
            error_code(host.dispatch("file.read", json!({ "path": escape }))),
            "outside_workspace",
            "expected {escape} to be rejected"
        );
    }

    let outside = tempfile::NamedTempFile::new().unwrap();
    fs::write(outside.path(), "outside\n").unwrap();
    assert_eq!(
        error_code(host.dispatch(
            "file.write",
            json!({
                "path": outside.path().to_string_lossy(),
                "content": "overwritten\n",
                "expected_hash": "fnv1a64:0000000000000000"
            })
        )),
        "outside_workspace"
    );

    // A symlink inside the workspace that points outside is rejected after canonicalization.
    #[cfg(unix)]
    {
        let workspace = temp_workspace(&["Mini.csproj"]);
        let target = tempfile::NamedTempFile::new().unwrap();
        fs::write(target.path(), "secret\n").unwrap();
        std::os::unix::fs::symlink(target.path(), workspace.path().join("Link.cs")).unwrap();
        open(&host, &workspace.path().join("Mini.csproj"));
        assert_eq!(
            error_code(host.dispatch("file.read", json!({ "path": "Link.cs" }))),
            "outside_workspace"
        );
    }
}

#[test]
fn directory_resolution_requires_an_explicit_choice() {
    let host = Host::new();
    let ambiguous = temp_workspace(&["First.csproj", "Second.csproj"]);
    let error = host
        .dispatch(
            "workspace.open",
            json!({ "path": ambiguous.path().to_string_lossy() }),
        )
        .expect_err("two projects are ambiguous");
    assert_eq!(error.code, "ambiguous_workspace");
    let candidates = error.details.unwrap()["candidates"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(candidates.len(), 2);
    assert!(Path::new(candidates[0]["path"].as_str().unwrap()).is_absolute());

    let empty = tempfile::tempdir().unwrap();
    assert_eq!(
        error_code(host.dispatch(
            "workspace.open",
            json!({ "path": empty.path().to_string_lossy() })
        )),
        "not_found"
    );

    let single = temp_workspace(&["Only.csproj"]);
    let snapshot = open(&host, single.path());
    assert_eq!(snapshot["workspace"]["name"], "Only.csproj");
    assert_eq!(snapshot["workspace"]["kind"], "project");
}

#[test]
fn save_reports_conflict_instead_of_overwriting_disk() {
    let host = Host::new();
    let workspace = temp_workspace(&["App.csproj"]);
    let file = workspace.path().join("App.cs");
    fs::write(&file, "original\n").unwrap();
    open(&host, &workspace.path().join("App.csproj"));

    let read = host
        .dispatch("file.read", json!({ "path": "App.cs" }))
        .unwrap();
    let stale_hash = read["hash"].as_str().unwrap().to_string();
    fs::write(&file, "changed on disk\n").unwrap();

    let conflict = host
        .dispatch(
            "file.write",
            json!({
                "path": "App.cs",
                "content": "my buffer\n",
                "expected_hash": stale_hash
            })
        )
        .expect_err("stale expected hash conflicts");
    assert_eq!(conflict.code, "conflict");
    let details = conflict.details.unwrap();
    assert_eq!(details["currentContent"], "changed on disk\n");
    assert_eq!(fs::read_to_string(&file).unwrap(), "changed on disk\n");

    let current_hash = details["currentHash"].as_str().unwrap().to_string();
    host.dispatch(
        "file.write",
        json!({
            "path": "App.cs",
            "content": "my buffer\n",
            "expected_hash": current_hash
        }),
    )
    .unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), "my buffer\n");
}

#[test]
fn executable_methods_fail_before_trust_and_spawn_nothing() {
    let host = Host::new();
    let workspace = temp_workspace(&["Probe.csproj"]);
    open(&host, &workspace.path().join("Probe.csproj"));

    for method in EXECUTABLE_METHODS {
        assert_eq!(
            error_code(host.dispatch(method, json!({}))),
            "trust_required",
            "{method} must not run before trust"
        );
    }

    std::thread::sleep(std::time::Duration::from_millis(300));
    let artifacts: Vec<String> = fs::read_dir(workspace.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| name == "bin" || name == "obj")
        .collect();
    assert!(
        artifacts.is_empty(),
        "a blocked executable method created build artifacts: {artifacts:?}"
    );
    assert_eq!(
        host.dispatch("workspace.state", json!({})).unwrap()["trust"]["granted"],
        false
    );
}

#[test]
fn one_workspace_at_a_time_and_awkward_names_work() {
    let host = Host::new();
    let first = temp_workspace(&["First.csproj"]);
    let second = temp_workspace(&["Second.csproj"]);
    open(&host, &first.path().join("First.csproj"));
    host.dispatch("file.read", json!({ "path": "First.cs" }))
        .unwrap();
    open(&host, &second.path().join("Second.csproj"));
    assert_eq!(
        error_code(host.dispatch(
            "file.read",
            json!({ "path": first.path().join("First.cs").to_string_lossy() })
        )),
        "outside_workspace"
    );

    let awkward = tempfile::tempdir().unwrap();
    let root = awkward.path().join("Order Desk ; & 01");
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("Odd Name ; &.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n",
    )
    .unwrap();
    let file = root.join("A file ; &.cs");
    fs::write(&file, "before\n").unwrap();
    open(&host, &root.join("Odd Name ; &.csproj"));
    let read = host
        .dispatch("file.read", json!({ "path": "A file ; &.cs" }))
        .unwrap();
    assert_eq!(read["content"], "before\n");
    let hash = read["hash"].as_str().unwrap();
    host.dispatch(
        "file.write",
        json!({ "path": "A file ; &.cs", "content": "after\n", "expected_hash": hash }),
    )
    .unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), "after\n");
}

fn snapshot(host: &Host) -> Value {
    host.dispatch("workspace.state", json!({})).unwrap()
}

fn grant(host: &Host) -> Value {
    host.dispatch("trust.grant", json!({})).expect("trust is granted")
}

fn process_id(snapshot: &Value) -> u64 {
    snapshot["processes"]
        .as_array()
        .and_then(|processes| processes.last())
        .and_then(|process| process["id"].as_u64())
        .expect("a tracked process exists")
}

#[test]
fn trust_grant_needs_an_open_workspace_and_gates_every_direct_call() {
    let without_workspace = Host::new();
    assert_eq!(
        error_code(without_workspace.dispatch("trust.grant", json!({}))),
        "invalid_state"
    );
    assert_eq!(
        error_code(without_workspace.dispatch("trust.revoke", json!({}))),
        "invalid_state"
    );

    let host = Host::new();
    let workspace = temp_workspace(&["Gate.csproj"]);
    let opened = open(&host, &workspace.path().join("Gate.csproj"));
    assert_eq!(opened["trust"]["granted"], false);
    assert_eq!(opened["trust"]["decidedAt"], Value::Null);
    assert_eq!(opened["sdk"]["project"]["state"], "not_inspected");

    for method in EXECUTABLE_METHODS {
        assert_eq!(
            error_code(host.dispatch(method, json!({}))),
            "trust_required",
            "{method} must be denied by the host gate"
        );
    }
    assert_eq!(snapshot(&host)["processes"], json!([]));

    let trusted = grant(&host);
    assert_eq!(trusted["trust"]["granted"], true);
    assert!(trusted["trust"]["decidedAt"].as_u64().is_some());
    assert_eq!(trusted["trust"]["workspaceId"], trusted["workspace"]["id"]);
    assert_eq!(trusted["workspace"]["mode"], "trusted");
    assert_eq!(trusted["sdk"]["project"]["state"], "no_global_json");

    for method in EXECUTABLE_METHODS.iter().filter(|method| **method != "processes.probe") {
        assert_eq!(
            error_code(host.dispatch(method, json!({}))),
            "invalid_state",
            "{method} passes the gate but is not implemented in this slice"
        );
    }

    let revoked = host.dispatch("trust.revoke", json!({})).unwrap();
    assert_eq!(revoked["trust"]["granted"], false);
    assert!(revoked["trust"]["revokedAt"].as_u64().is_some());
    assert_eq!(revoked["workspace"]["mode"], "restricted");
    assert_eq!(revoked["sdk"]["project"]["state"], "not_inspected");
    for method in EXECUTABLE_METHODS {
        assert_eq!(
            error_code(host.dispatch(method, json!({}))),
            "trust_required",
            "{method} must be denied again after revoke"
        );
    }
}

#[test]
fn closing_a_trusted_workspace_clears_trust_and_the_executable_context() {
    let host = Host::new();
    let workspace = temp_workspace(&["Close.csproj"]);
    open(&host, &workspace.path().join("Close.csproj"));
    grant(&host);

    let closed = host.dispatch("workspace.close", json!({})).unwrap();
    assert_eq!(closed["workspace"], Value::Null);
    assert_eq!(closed["outline"], Value::Null);
    assert_eq!(closed["trust"]["granted"], false);
    assert_eq!(closed["trust"]["decidedAt"], Value::Null);
    assert_eq!(closed["sdk"]["project"]["state"], "no_workspace");
    assert_eq!(
        error_code(host.dispatch("trust.grant", json!({}))),
        "invalid_state"
    );
    assert_eq!(
        error_code(host.dispatch("build.start", json!({}))),
        "trust_required"
    );
}

#[test]
fn a_late_result_from_an_older_workspace_or_context_is_discarded() {
    let host = Host::new();
    open(&host, &fixture());
    grant(&host);
    let old_token = host.context_token().unwrap();
    host.accept_result(old_token).expect("current token is accepted");

    let other = temp_workspace(&["Other.csproj"]);
    let switched = open(&host, &other.path().join("Other.csproj"));
    assert_eq!(switched["workspace"]["name"], "Other.csproj");
    assert_eq!(switched["trust"]["granted"], false);

    let stale = host.accept_result(old_token).expect_err("old workspace token is stale");
    assert_eq!(stale.code, "stale_context");
    assert_eq!(snapshot(&host)["workspace"]["name"], "Other.csproj");
    assert_eq!(snapshot(&host)["sdk"]["project"]["state"], "not_inspected");

    // Same workspace, revoked context: also stale.
    open(&host, &fixture());
    grant(&host);
    let revoked_token = host.context_token().unwrap();
    host.dispatch("trust.revoke", json!({})).unwrap();
    let stale = host
        .accept_result(revoked_token)
        .expect_err("revoked context token is stale");
    assert_eq!(stale.code, "stale_context");
    assert_eq!(snapshot(&host)["sdk"]["project"]["state"], "not_inspected");
}

#[cfg(target_os = "macos")]
#[test]
fn sdk_inventory_reads_the_real_roots_and_reports_separate_host_and_project_sdks() {
    // Expected versions were observed on the evidence machine (macOS arm64, 2026-10-27):
    // 8.0.416/10.0.100 under /usr/local/share/dotnet and 8.0.408 under the user SDK root.
    let inventory = sdk::inventory();
    let versions: Vec<&str> = inventory
        .installs
        .iter()
        .map(|install| install.version.as_str())
        .collect();
    for expected in ["10.0.100", "8.0.408", "8.0.416"] {
        assert!(
            versions.contains(&expected),
            "expected SDK {expected} in the real inventory, found {versions:?}"
        );
    }
    assert!(inventory
        .roots
        .iter()
        .all(|root| !root.root.is_empty() && root.source != ""));
    assert!(inventory
        .roots
        .iter()
        .all(|root| root.exit == Some(0) && root.error.is_none()));

    let host_sdk = sdk::HostSdk::from_inventory(&inventory.installs);
    assert_eq!(host_sdk.state, "found");
    assert_eq!(host_sdk.version.as_deref(), Some("10.0.100"));

    let host = Host::new();
    let opened = open(&host, &fixture());
    assert_eq!(opened["sdk"]["host"]["version"], "10.0.100");
    assert_eq!(opened["sdk"]["project"]["state"], "not_inspected");
    assert_eq!(opened["sdk"]["project"]["requestedVersion"], "8.0.408");
    assert_eq!(opened["sdk"]["project"]["rollForward"], "disable");
    assert_eq!(opened["outline"]["projects"][0]["tfms"], json!(["net8.0"]));

    let trusted = grant(&host);
    assert_eq!(trusted["sdk"]["project"]["state"], "resolved");
    assert_eq!(trusted["sdk"]["project"]["resolvedVersion"], "8.0.408");
    assert_eq!(trusted["sdk"]["project"]["requestedVersion"], "8.0.408");
    assert_eq!(trusted["sdk"]["project"]["rollForward"], "disable");
    let project_root = trusted["sdk"]["project"]["root"].as_str().unwrap();
    assert!(project_root.contains("OpenRidr/sdks/8.0.408"), "{project_root}");
    assert_ne!(project_root, trusted["sdk"]["host"]["root"].as_str().unwrap());
    assert_eq!(trusted["sdk"]["host"]["version"], "10.0.100");
}

#[cfg(target_os = "macos")]
#[test]
fn project_sdk_missing_keeps_a_distinct_state_and_does_not_rewrite_global_json() {
    let fixture = fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/MissingSdk"),
    )
    .expect("fixture MissingSdk exists");
    let global_json = fixture.join("global.json");
    let before = fs::read(&global_json).unwrap();

    let host = Host::new();
    open(&host, &fixture.join("MissingSdk.csproj"));
    let trusted = grant(&host);
    assert_eq!(trusted["sdk"]["host"]["state"], "found");
    assert_eq!(trusted["sdk"]["project"]["state"], "sdk_missing");
    assert_eq!(trusted["sdk"]["project"]["requestedVersion"], "8.0.999");
    let message = trusted["sdk"]["project"]["message"].as_str().unwrap();
    assert!(message.contains("8.0.999"));
    assert!(message.contains("does not rewrite global.json"));
    assert_eq!(trusted["sdk"]["project"]["resolvedVersion"], Value::Null);

    host.dispatch("sdk.redetect", json!({})).unwrap();
    assert_eq!(fs::read(&global_json).unwrap(), before);
    assert_eq!(snapshot(&host)["sdk"]["project"]["state"], "sdk_missing");
}

#[test]
fn redetect_is_allowed_before_and_after_trust_and_touches_no_project_file() {
    let host = Host::new();
    let before = snapshot(&host);
    assert_eq!(before["workspace"], Value::Null);
    assert_eq!(before["sdk"]["host"]["state"], "found");

    let workspace = temp_workspace(&["Redetect.csproj"]);
    open(&host, &workspace.path().join("Redetect.csproj"));
    let refreshed = host.dispatch("sdk.redetect", json!({})).unwrap();
    assert_eq!(refreshed["sdk"]["host"]["state"], "found");
    assert_eq!(refreshed["sdk"]["project"]["state"], "not_inspected");

    grant(&host);
    let refreshed = host.dispatch("sdk.redetect", json!({})).unwrap();
    assert_eq!(refreshed["trust"]["granted"], true);
    assert_eq!(refreshed["sdk"]["project"]["state"], "no_global_json");
    let artifacts: Vec<String> = fs::read_dir(workspace.path())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .filter(|name| name == "bin" || name == "obj")
        .collect();
    assert!(artifacts.is_empty(), "redetect created {artifacts:?}");
}

#[cfg(unix)]
#[test]
fn revoke_marks_a_real_child_and_terminate_reports_the_observed_exit() {
    std::env::set_var("OPENRIDR_ENABLE_PROBE", "1");
    let host = Host::new();
    let workspace = temp_workspace(&["Probe.csproj"]);
    open(&host, &workspace.path().join("Probe.csproj"));

    assert_eq!(
        error_code(host.dispatch("processes.probe", json!({ "kind": "sleep" }))),
        "trust_required"
    );
    assert_eq!(snapshot(&host)["processes"], json!([]));

    grant(&host);
    let spawned = host
        .dispatch("processes.probe", json!({ "kind": "sleep" }))
        .unwrap();
    let process = &spawned["processes"][0];
    assert_eq!(process["state"], "running");
    assert_eq!(process["exit"], Value::Null);
    assert_eq!(process["awaitingDecision"], false);
    assert!(process["pid"].as_u64().unwrap() > 0);
    let id = process_id(&spawned);

    let revoked = host.dispatch("trust.revoke", json!({})).unwrap();
    assert_eq!(revoked["processes"][0]["state"], "running");
    assert_eq!(revoked["processes"][0]["awaitingDecision"], true);
    assert_eq!(
        error_code(host.dispatch("build.start", json!({}))),
        "trust_required"
    );

    let started = Instant::now();
    let terminated = host
        .dispatch("processes.terminate", json!({ "id": id }))
        .unwrap();
    assert!(started.elapsed() < Duration::from_secs(4));
    let process = &terminated["processes"][0];
    assert_eq!(process["state"], "exited");
    assert_eq!(process["termination"]["observed"], "exited");
    assert_eq!(process["termination"]["killedAfterGrace"], false);
}

#[cfg(unix)]
#[test]
fn keep_leaves_the_process_running_and_terminate_escalates_after_the_grace_period() {
    std::env::set_var("OPENRIDR_ENABLE_PROBE", "1");
    let host = Host::new();
    let workspace = temp_workspace(&["ProbeKeep.csproj"]);
    open(&host, &workspace.path().join("ProbeKeep.csproj"));
    grant(&host);

    let spawned = host
        .dispatch("processes.probe", json!({ "kind": "ignore-term" }))
        .unwrap();
    let id = process_id(&spawned);
    // Give the shell time to install its TERM trap before the workspace boundary.
    std::thread::sleep(Duration::from_millis(400));
    host.dispatch("trust.revoke", json!({})).unwrap();

    let kept = host
        .dispatch("processes.keep", json!({ "id": id }))
        .unwrap();
    assert_eq!(kept["processes"][0]["awaitingDecision"], false);
    assert_eq!(kept["processes"][0]["kept"], true);
    assert_eq!(kept["processes"][0]["state"], "running");

    let started = Instant::now();
    let terminated = host
        .dispatch("processes.terminate", json!({ "id": id }))
        .unwrap();
    assert!(started.elapsed() >= Duration::from_millis(1900));
    let process = &terminated["processes"][0];
    assert_eq!(process["state"], "exited");
    assert_eq!(process["termination"]["killedAfterGrace"], true);
    assert!(process["termination"]["requested"]
        .as_str()
        .unwrap()
        .contains("SIGKILL"));

    grant(&host);
    let quick = host
        .dispatch("processes.probe", json!({ "kind": "quick" }))
        .unwrap();
    let quick_id = process_id(&quick);
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let state = snapshot(&host);
        let process = state["processes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|process| process["id"] == quick_id)
            .unwrap();
        if process["state"] == "exited" {
            assert_eq!(process["exit"], 3);
            break;
        }
        assert!(Instant::now() < deadline, "quick probe did not exit");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(unix)]
#[test]
fn switching_workspaces_marks_processes_pending_and_drops_trust() {
    std::env::set_var("OPENRIDR_ENABLE_PROBE", "1");
    let host = Host::new();
    let first = temp_workspace(&["First.csproj"]);
    let second = temp_workspace(&["Second.csproj"]);
    open(&host, &first.path().join("First.csproj"));
    grant(&host);
    let spawned = host
        .dispatch("processes.probe", json!({ "kind": "sleep" }))
        .unwrap();
    let id = process_id(&spawned);

    let switched = open(&host, &second.path().join("Second.csproj"));
    assert_eq!(switched["workspace"]["name"], "Second.csproj");
    assert_eq!(switched["trust"]["granted"], false);
    assert_eq!(switched["sdk"]["project"]["state"], "not_inspected");
    let process = switched["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|process| process["id"] == id)
        .unwrap();
    assert_eq!(process["awaitingDecision"], true);
    assert_eq!(process["state"], "running");
    assert_eq!(
        error_code(host.dispatch("processes.probe", json!({ "kind": "sleep" }))),
        "trust_required"
    );

    host.dispatch("processes.terminate", json!({ "id": id }))
        .unwrap();
}
