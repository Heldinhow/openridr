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

/// A fixture copy outside the repository, used whenever a test saves project files.
fn fixture_copy(parent: &Path, name: &str) -> PathBuf {
    let copy = parent.join(name);
    copy_tree(&fixture(), &copy);
    copy
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

    for method in EXECUTABLE_METHODS.iter().filter(|method| {
        **method != "processes.probe"
            && **method != "evaluation.run"
            && **method != "restore.start"
            && **method != "restore.retry"
    }) {
        let code = error_code(host.dispatch(method, json!({})));
        if openridr_host::RESTORE_GATED_METHODS.contains(method) {
            assert_eq!(
                code, "restore_required",
                "{method} must require an explicit restore instead of running"
            );
        } else {
            assert_eq!(
                code, "invalid_state",
                "{method} passes the gate but is not implemented in this slice"
            );
        }
    }
    assert_eq!(
        error_code(host.dispatch("evaluation.run", json!({}))),
        "invalid_state",
        "evaluation.run needs a resolved project SDK"
    );
    assert_eq!(snapshot(&host)["evaluation"]["state"], "failed");

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

#[test]
fn evaluation_never_runs_before_trust_or_without_a_resolved_project_sdk() {
    let host = Host::new();
    let workspace = temp_workspace(&["NoSdk.csproj"]);
    open(&host, &workspace.path().join("NoSdk.csproj"));

    assert_eq!(
        error_code(host.dispatch("evaluation.run", json!({}))),
        "trust_required"
    );
    let trusted = grant(&host);
    assert_eq!(trusted["sdk"]["project"]["state"], "no_global_json");
    for params in [
        json!({ "configuration": "Debug;TargetPath=/tmp/injected" }),
        json!({ "targetFramework": "net8.0;Configuration=Release" }),
    ] {
        assert_eq!(
            error_code(host.dispatch("evaluation.run", params)),
            "invalid_state"
        );
    }
    assert_eq!(
        error_code(host.dispatch("evaluation.run", json!({}))),
        "invalid_state"
    );
    let state = snapshot(&host);
    assert_eq!(state["evaluation"]["state"], "failed");
    assert_eq!(state["evaluation"]["projects"], json!([]));
    assert!(state["evaluation"]["log"]
        .as_str()
        .unwrap()
        .contains("not resolved"));
    for artifact in ["bin", "obj"] {
        assert!(
            !workspace.path().join(artifact).exists(),
            "{artifact} was created without a resolved SDK"
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn evaluation_with_a_missing_project_sdk_reports_sdk_missing() {
    let fixture = fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/MissingSdk"),
    )
    .expect("fixture MissingSdk exists");
    let global_json = fixture.join("global.json");
    let before = fs::read(&global_json).unwrap();

    let host = Host::new();
    open(&host, &fixture.join("MissingSdk.csproj"));
    grant(&host);
    let error = host
        .dispatch("evaluation.run", json!({}))
        .expect_err("a missing project SDK blocks evaluation");
    assert_eq!(error.code, "sdk_missing");
    assert!(error.message.contains("8.0.999"));
    let state = snapshot(&host);
    assert_eq!(state["evaluation"]["state"], "failed");
    assert_eq!(state["evaluation"]["projects"], json!([]));
    assert_eq!(fs::read(&global_json).unwrap(), before);
}

#[cfg(target_os = "macos")]
#[test]
fn evaluation_returns_the_real_msbuild_model_for_every_project() {
    let host = Host::new();
    open(&host, &fixture());
    grant(&host);
    let evaluated = host.dispatch("evaluation.run", json!({})).unwrap();
    let evaluation = &evaluated["evaluation"];

    assert_eq!(evaluation["state"], "ok", "{}", evaluation["log"]);
    assert_eq!(evaluation["context"]["configuration"], "Debug");
    assert_eq!(evaluation["context"]["targetFramework"], Value::Null);
    assert_eq!(evaluation["configurations"], json!(["Debug", "Release"]));
    assert_eq!(evaluation["revision"], 1);
    assert_eq!(evaluation["stale"], false);
    assert!(evaluation["workspaceId"].as_u64().is_some());
    assert!(evaluation["contextId"].as_u64().is_some());
    let projects = evaluation["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 3);

    let project = |name: &str| {
        projects
            .iter()
            .find(|project| project["name"] == name)
            .unwrap()
    };
    let cli = project("OrderDesk.Cli");
    assert_eq!(cli["state"], "ok");
    assert_eq!(cli["kind"], "console");
    assert_eq!(cli["folder"], "src");
    assert_eq!(cli["tfms"], json!(["net8.0"]));
    assert_eq!(cli["evaluatedTfm"], "net8.0");
    assert_eq!(cli["outputType"], "Exe");
    assert_eq!(cli["restored"], "restored");
    assert!(cli["targetPath"]
        .as_str()
        .unwrap()
        .ends_with("src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll"));
    assert_eq!(cli["projectRefs"][0]["name"], "OrderDesk.Core");
    assert_eq!(
        cli["projectRefs"][0]["path"],
        "src/OrderDesk.Core/OrderDesk.Core.csproj"
    );
    assert_eq!(cli["packageRefs"], json!([]));
    assert!(cli["sources"]
        .as_array()
        .unwrap()
        .iter()
        .any(|source| source == "src/OrderDesk.Cli/Program.cs"));

    let core = project("OrderDesk.Core");
    assert_eq!(core["kind"], "library");
    assert_eq!(core["folder"], "src");

    let tests = project("OrderDesk.Tests");
    assert_eq!(tests["kind"], "test");
    assert_eq!(tests["folder"], "tests");
    assert_eq!(tests["packageRefs"].as_array().unwrap().len(), 3);
    assert_eq!(tests["packageRefs"][1]["name"], "xunit");
    assert_eq!(tests["packageRefs"][1]["version"], "2.9.2");
    assert!(tests["targetPath"]
        .as_str()
        .unwrap()
        .ends_with("tests/OrderDesk.Tests/bin/Debug/net8.0/OrderDesk.Tests.dll"));

    for project in projects {
        for source in project["sources"].as_array().unwrap() {
            let source = source.as_str().unwrap();
            assert!(!source.contains("/obj/") && !source.contains("/bin/"), "{source}");
        }
    }

    let release = host
        .dispatch("evaluation.run", json!({ "configuration": "Release" }))
        .unwrap();
    assert_eq!(release["evaluation"]["context"]["configuration"], "Release");
    assert_eq!(release["evaluation"]["revision"], 2);
    let cli = release["evaluation"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|project| project["name"] == "OrderDesk.Cli")
        .unwrap();
    assert!(cli["targetPath"]
        .as_str()
        .unwrap()
        .contains("/bin/Release/net8.0/OrderDesk.Cli.dll"));
}

#[cfg(target_os = "macos")]
#[test]
fn evaluation_of_a_fresh_copy_with_spaces_creates_no_restore_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk ; & copy");

    let host = Host::new();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    let evaluated = host.dispatch("evaluation.run", json!({})).unwrap();
    let evaluation = &evaluated["evaluation"];
    assert_eq!(evaluation["state"], "ok", "{}", evaluation["log"]);
    for project in evaluation["projects"].as_array().unwrap() {
        assert_eq!(project["restored"], "not_restored", "{}", project["name"]);
        assert_eq!(project["log"].as_str().unwrap().contains("exit 0"), true);
        let target = project["targetPath"].as_str().unwrap();
        assert!(target.contains("Order Desk ; & copy"), "{target}");
        assert!(target.contains("/bin/Debug/net8.0/"), "{target}");
        assert!(!project["sources"].as_array().unwrap().is_empty());
    }
    for project in ["src/OrderDesk.Cli", "src/OrderDesk.Core", "tests/OrderDesk.Tests"] {
        assert!(
            !copy.join(project).join("obj").join("project.assets.json").exists(),
            "evaluation restored {project}"
        );
    }
    assert!(evaluation["log"]
        .as_str()
        .unwrap()
        .contains("Restore not requested"));
}

#[cfg(target_os = "macos")]
#[test]
fn multi_tfm_needs_an_explicit_selection_for_target_path_and_sources() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("global.json"),
        "{\"sdk\":{\"version\":\"8.0.408\",\"rollForward\":\"disable\"}}\n",
    )
    .unwrap();
    fs::write(
        root.join("Multi.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFrameworks>net8.0;netstandard2.0</TargetFrameworks></PropertyGroup></Project>\n",
    )
    .unwrap();
    fs::write(root.join("C.cs"), "class C {}\n").unwrap();

    let host = Host::new();
    open(&host, &root.join("Multi.csproj"));
    grant(&host);
    let first = host.dispatch("evaluation.run", json!({})).unwrap();
    assert_eq!(first["evaluation"]["state"], "ok");
    let project = &first["evaluation"]["projects"][0];
    assert_eq!(project["tfms"], json!(["net8.0", "netstandard2.0"]));
    assert_eq!(project["evaluatedTfm"], Value::Null);
    assert_eq!(project["targetPath"], Value::Null);
    assert_eq!(project["sources"], json!([]));
    assert!(project["note"].as_str().unwrap().contains("Select a TFM"));

    let selected = host
        .dispatch(
            "evaluation.run",
            json!({ "targetFramework": "netstandard2.0" }),
        )
        .unwrap();
    assert_eq!(
        selected["evaluation"]["context"]["targetFramework"],
        "netstandard2.0"
    );
    let project = &selected["evaluation"]["projects"][0];
    assert_eq!(project["evaluatedTfm"], "netstandard2.0");
    assert!(project["targetPath"]
        .as_str()
        .unwrap()
        .ends_with("/bin/Debug/netstandard2.0/Multi.dll"));
    assert_eq!(project["sources"], json!(["C.cs"]));

    let not_declared = host
        .dispatch("evaluation.run", json!({ "targetFramework": "net9.0" }))
        .unwrap();
    let project = &not_declared["evaluation"]["projects"][0];
    assert_eq!(project["targetPath"], Value::Null);
    assert!(project["note"].as_str().unwrap().contains("not declared"));
}

#[cfg(target_os = "macos")]
#[test]
fn a_failed_evaluation_stays_in_the_model_while_other_projects_evaluate() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("global.json"),
        "{\"sdk\":{\"version\":\"8.0.408\",\"rollForward\":\"disable\"}}\n",
    )
    .unwrap();
    fs::write(
        root.join("Good.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n",
    )
    .unwrap();
    fs::write(root.join("Good.cs"), "class Good {}\n").unwrap();
    fs::write(
        root.join("Broken.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup><Import Project=\"build/Missing.props\" /></Project>\n",
    )
    .unwrap();
    fs::write(root.join("Broken.cs"), "class Broken {}\n").unwrap();
    fs::write(
        root.join("Mixed.sln"),
        "Microsoft Visual Studio Solution File, Format Version 12.00\n\
Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Good\", \"Good.csproj\", \"{11111111-1111-1111-1111-111111111111}\"\nEndProject\n\
Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Broken\", \"Broken.csproj\", \"{22222222-2222-2222-2222-222222222222}\"\nEndProject\n\
Global\n\tGlobalSection(SolutionConfigurationPlatforms) = preSolution\n\t\tDebug|Any CPU = Debug|Any CPU\n\tEndGlobalSection\nEndGlobal\n",
    )
    .unwrap();

    let host = Host::new();
    open(&host, &root.join("Mixed.sln"));
    grant(&host);
    let evaluated = host.dispatch("evaluation.run", json!({})).unwrap();
    let evaluation = &evaluated["evaluation"];
    assert_eq!(evaluation["state"], "partial", "{}", evaluation["log"]);
    let projects = evaluation["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 2);
    let good = projects.iter().find(|project| project["name"] == "Good").unwrap();
    assert_eq!(good["state"], "ok");
    assert!(good["targetPath"].as_str().unwrap().ends_with("Good.dll"));
    let broken = projects
        .iter()
        .find(|project| project["name"] == "Broken")
        .unwrap();
    assert_eq!(broken["state"], "failed");
    assert_eq!(broken["kind"], "unknown");
    assert_eq!(broken["targetPath"], Value::Null);
    assert_eq!(broken["restored"], "not_restored");
    assert_eq!(broken["exit"], 1);
    assert!(broken["error"].as_str().unwrap().contains("MSB4019"));
    assert!(broken["log"].as_str().unwrap().contains("Missing.props"));
    assert!(evaluation["log"].as_str().unwrap().contains("[Broken] failed"));
}

#[cfg(target_os = "macos")]
#[test]
fn revoking_trust_while_msbuild_runs_drops_the_late_result() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("global.json"),
        "{\"sdk\":{\"version\":\"8.0.408\",\"rollForward\":\"disable\"}}\n",
    )
    .unwrap();
    let mut sln = String::from(
        "Microsoft Visual Studio Solution File, Format Version 12.00\nGlobal\n\tGlobalSection(SolutionConfigurationPlatforms) = preSolution\n\t\tDebug|Any CPU = Debug|Any CPU\n\tEndGlobalSection\nEndGlobal\n",
    );
    for index in 0..8 {
        let name = format!("P{index}");
        fs::write(
            root.join(format!("{name}.csproj")),
            "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n",
        )
        .unwrap();
        fs::write(root.join(format!("{name}.cs")), "class C {}\n").unwrap();
        sln.push_str(&format!(
            "Project(\"{{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}}\") = \"{name}\", \"{name}.csproj\", \"{{00000000-0000-0000-0000-{index:012}}}\"\nEndProject\n"
        ));
    }
    fs::write(root.join("Many.sln"), sln).unwrap();

    let host = Host::new();
    open(&host, &root.join("Many.sln"));
    grant(&host);
    std::thread::scope(|scope| {
        let handle = scope.spawn(|| host.dispatch("evaluation.run", json!({})));
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let state = snapshot(&host);
            if state["evaluation"]["state"] == "running" {
                break;
            }
            assert!(Instant::now() < deadline, "evaluation never reported running");
            std::thread::sleep(Duration::from_millis(5));
        }
        host.dispatch("trust.revoke", json!({})).unwrap();
        let late = handle.join().unwrap();
        assert_eq!(error_code(late), "stale_context");
    });

    let state = snapshot(&host);
    assert_eq!(state["trust"]["granted"], false);
    assert_eq!(state["evaluation"]["state"], "idle");
    assert_eq!(state["evaluation"]["projects"], json!([]));
    assert_eq!(
        error_code(host.dispatch("evaluation.run", json!({}))),
        "trust_required"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn saving_a_project_file_or_an_external_edit_makes_the_evaluation_stale() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "OrderDesk stale");
    let host = Host::new();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    let first = host.dispatch("evaluation.run", json!({})).unwrap();
    assert_eq!(first["evaluation"]["revision"], 1);
    assert_eq!(first["evaluation"]["stale"], false);
    let context_before = first["contextId"].as_u64().unwrap();

    // A source edit does not invalidate the project model.
    let program = host
        .dispatch("file.read", json!({ "path": "src/OrderDesk.Cli/Program.cs" }))
        .unwrap();
    host.dispatch(
        "file.write",
        json!({
            "path": "src/OrderDesk.Cli/Program.cs",
            "content": format!("// harmless\n{}", program["content"].as_str().unwrap()),
            "expected_hash": program["hash"],
        }),
    )
    .unwrap();
    assert_eq!(snapshot(&host)["evaluation"]["stale"], false);

    // Saving the project file through the app marks the model stale and moves the context.
    let project_file = "src/OrderDesk.Cli/OrderDesk.Cli.csproj";
    let read = host
        .dispatch("file.read", json!({ "path": project_file }))
        .unwrap();
    let content = read["content"]
        .as_str()
        .unwrap()
        .replace(
            "</Project>",
            "  <PropertyGroup><Description>stale check</Description></PropertyGroup>\n</Project>",
        );
    host.dispatch(
        "file.write",
        json!({ "path": project_file, "content": content, "expected_hash": read["hash"] }),
    )
    .unwrap();
    let stale = snapshot(&host);
    assert_eq!(stale["evaluation"]["stale"], true);
    assert!(stale["evaluation"]["staleReason"]
        .as_str()
        .unwrap()
        .contains("was saved"));
    assert!(stale["contextId"].as_u64().unwrap() > context_before);
    assert_eq!(stale["evaluation"]["projects"].as_array().unwrap().len(), 3);

    let second = host.dispatch("evaluation.run", json!({})).unwrap();
    assert_eq!(second["evaluation"]["stale"], false);
    assert_eq!(second["evaluation"]["revision"], 2);

    // An external edit is detected by the mtime watch on the state refresh.
    std::thread::sleep(Duration::from_millis(20));
    let file = copy.join(project_file);
    let mut text = fs::read_to_string(&file).unwrap();
    text.push('\n');
    fs::write(&file, text).unwrap();
    let external = snapshot(&host);
    assert_eq!(external["evaluation"]["stale"], true);
    assert!(external["evaluation"]["staleReason"]
        .as_str()
        .unwrap()
        .contains("changed on disk"));

    // Changing the configuration re-evaluates; the old TargetPath is not current anymore.
    let release = host
        .dispatch("evaluation.run", json!({ "configuration": "Release" }))
        .unwrap();
    assert_eq!(release["evaluation"]["stale"], false);
    assert_eq!(release["evaluation"]["context"]["configuration"], "Release");
    let cli = release["evaluation"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|project| project["name"] == "OrderDesk.Cli")
        .unwrap();
    assert!(cli["targetPath"]
        .as_str()
        .unwrap()
        .contains("/bin/Release/net8.0/OrderDesk.Cli.dll"));
}

#[cfg(target_os = "macos")]
#[test]
fn a_late_evaluation_does_not_replace_a_newer_selection() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fs::write(
        root.join("global.json"),
        "{\"sdk\":{\"version\":\"8.0.408\",\"rollForward\":\"disable\"}}\n",
    )
    .unwrap();

    let project = |name: &str| {
        fs::write(
            root.join(format!("{name}.csproj")),
            "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n",
        )
        .unwrap();
        fs::write(root.join(format!("{name}.cs")), "class C {}\n").unwrap();
    };
    project("Cli");
    let mut sln = String::from(
        "Microsoft Visual Studio Solution File, Format Version 12.00\nProject(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Cli\", \"Cli.csproj\", \"{11111111-1111-1111-1111-111111111111}\"\nEndProject\nGlobal\n\tGlobalSection(SolutionConfigurationPlatforms) = preSolution\n\t\tDebug|Any CPU = Debug|Any CPU\n\t\tRelease|Any CPU = Release|Any CPU\n\tEndGlobalSection\nEndGlobal\n",
    );
    for index in 0..5 {
        let name = format!("Lib{index}");
        project(&name);
        sln.push_str(&format!(
            "Project(\"{{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}}\") = \"{name}\", \"{name}.csproj\", \"{{00000000-0000-0000-0000-{index:012}}}\"\nEndProject\n"
        ));
    }
    fs::write(root.join("Many.sln"), sln).unwrap();

    let host = Host::new();
    open(&host, &root.join("Many.sln"));
    grant(&host);
    std::thread::scope(|scope| {
        let older = scope.spawn(|| host.dispatch("evaluation.run", json!({})));
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if snapshot(&host)["evaluation"]["state"] == "running" {
                break;
            }
            assert!(Instant::now() < deadline, "evaluation never reported running");
            std::thread::sleep(Duration::from_millis(5));
        }
        let newer = host
            .dispatch("evaluation.run", json!({ "configuration": "Release" }))
            .unwrap();
        assert_eq!(newer["evaluation"]["revision"], 2);
        assert_eq!(newer["evaluation"]["context"]["configuration"], "Release");
        assert_eq!(error_code(older.join().unwrap()), "stale_context");
    });

    let state = snapshot(&host);
    assert_eq!(state["evaluation"]["revision"], 2);
    assert_eq!(state["evaluation"]["context"]["configuration"], "Release");
    assert_eq!(state["evaluation"]["state"], "ok");
}

fn global_json_8(dir: &Path) {
    fs::write(
        dir.join("global.json"),
        "{\"sdk\":{\"version\":\"8.0.408\",\"rollForward\":\"disable\"}}\n",
    )
    .unwrap();
}

fn write_project(dir: &Path, name: &str, extra: &str) {
    fs::write(
        dir.join(format!("{name}.csproj")),
        format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup>{extra}</Project>\n"
        ),
    )
    .unwrap();
    fs::write(dir.join(format!("{name}.cs")), "class C {}\n").unwrap();
}

/// Local TCP server that accepts NuGet requests and never answers, so `dotnet restore`
/// waits on it until the host terminates the process.
#[cfg(unix)]
fn holding_server() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        let mut held = Vec::new();
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => held.push(stream),
                Err(_) => break,
            }
        }
        drop(held);
    });
    format!("http://{address}/v3/index.json")
}

#[cfg(unix)]
fn write_hanging_project(dir: &Path, source: &str) {
    global_json_8(dir);
    fs::write(
        dir.join("NuGet.config"),
        format!(
            "<configuration><packageSources><clear /><add key=\"hold\" value=\"{source}\" /></packageSources></configuration>\n"
        ),
    )
    .unwrap();
    write_project(
        dir,
        "Hang",
        "<ItemGroup><PackageReference Include=\"xunit\" Version=\"2.9.2\" /></ItemGroup>",
    );
}

fn wait_for_restore(host: &Host, timeout: Duration) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let state = snapshot(host);
        let restore = state["restore"].clone();
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

#[test]
fn restore_start_and_retry_are_denied_before_trust() {
    let host = Host::new();
    let workspace = temp_workspace(&["RestoreGate.csproj"]);
    open(&host, &workspace.path().join("RestoreGate.csproj"));

    for method in ["restore.start", "restore.retry"] {
        assert_eq!(
            error_code(host.dispatch(method, json!({}))),
            "trust_required",
            "{method} must be denied before trust"
        );
    }
    assert_eq!(snapshot(&host)["restore"]["revision"], 0);
    assert!(!workspace.path().join("obj").exists());
}

#[test]
fn build_start_reports_restore_required_with_the_explicit_action() {
    let host = Host::new();
    let workspace = temp_workspace(&["Policy.csproj"]);
    open(&host, &workspace.path().join("Policy.csproj"));
    grant(&host);

    let error = host
        .dispatch("build.start", json!({}))
        .expect_err("a project without assets must not build");
    assert_eq!(error.code, "restore_required");
    assert!(error.message.contains("never restores implicitly"));
    let details = error.details.unwrap();
    assert_eq!(details["consumer"], "build.start");
    assert_eq!(details["action"]["method"], "restore.start");
    assert_eq!(details["projects"][0]["name"], "Policy");
    assert_eq!(details["projects"][0]["restored"], "not_restored");
    assert!(
        !workspace.path().join("obj").exists(),
        "the policy must not restore"
    );

    for method in ["run.start", "debug.start", "tests.discover", "tests.run"] {
        assert_eq!(
            error_code(host.dispatch(method, json!({}))),
            "restore_required",
            "{method} must refuse to run without restored assets"
        );
    }
    assert!(!workspace.path().join("obj").exists());
}

#[cfg(target_os = "macos")]
#[test]
fn restore_reports_the_exact_command_and_creates_real_assets() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    global_json_8(&root);
    write_project(&root, "Honest", "");

    let host = Host::new();
    open(&host, &root.join("Honest.csproj"));
    grant(&host);
    let started = host.dispatch("restore.start", json!({})).unwrap();
    let restore = &started["restore"];
    assert_eq!(restore["state"], "running");
    assert_eq!(restore["scope"], "project");
    assert_eq!(restore["revision"], 1);
    assert_eq!(
        restore["target"].as_str().unwrap(),
        root.join("Honest.csproj").to_string_lossy()
    );
    assert_eq!(
        restore["cwd"].as_str().unwrap(),
        root.to_string_lossy()
    );
    assert_eq!(restore["sdkVersion"], "8.0.408");
    assert!(restore["sdkRoot"]
        .as_str()
        .unwrap()
        .ends_with("OpenRidr/sdks/8.0.408"));
    let command: Vec<String> = restore["command"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_string())
        .collect();
    assert_eq!(command.len(), 5, "{command:?}");
    assert!(command[0].ends_with("/dotnet"), "{}", command[0]);
    assert_eq!(command[1], "restore");
    assert_eq!(command[2], root.join("Honest.csproj").to_string_lossy());
    assert_eq!(command[3], "-nologo");
    assert_eq!(command[4], "-v:minimal");
    assert!(!command.iter().any(|arg| arg.contains("--no-restore")));

    let finished = wait_for_restore(&host, Duration::from_secs(60));
    assert_eq!(finished["state"], "ok", "{}", finished["log"]);
    assert_eq!(finished["exit"], 0);
    assert!(finished["log"]
        .as_str()
        .unwrap()
        .contains("Honest.csproj"));
    assert_eq!(finished["projects"][0]["state"], "restored");
    assert!(root.join("obj/project.assets.json").exists());

    // The policy now passes; the consumer itself is still a ticket-11+ feature.
    assert_eq!(
        error_code(host.dispatch("build.start", json!({}))),
        "invalid_state"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn opening_and_evaluating_never_start_a_restore() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk no restore");
    let host = Host::new();
    let opened = open(&host, &copy.join("OrderDesk.sln"));
    assert_eq!(opened["restore"]["state"], "idle");
    grant(&host);
    host.dispatch("evaluation.run", json!({})).unwrap();
    let state = snapshot(&host);
    assert_eq!(state["restore"]["revision"], 0);
    assert_eq!(state["restore"]["state"], "idle");
    assert_eq!(state["processes"], json!([]));
    for project in ["src/OrderDesk.Cli", "src/OrderDesk.Core", "tests/OrderDesk.Tests"] {
        assert!(
            !copy.join(project).join("obj/project.assets.json").exists(),
            "evaluation restored {project}"
        );
    }
}

#[cfg(unix)]
#[test]
fn cancelling_a_real_hanging_restore_reports_requested_and_observed_termination() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    let source = holding_server();
    write_hanging_project(&root, &source);

    let host = Host::new();
    open(&host, &root.join("Hang.csproj"));
    grant(&host);
    let started = host.dispatch("restore.start", json!({})).unwrap();
    let process_id = started["restore"]["processId"].as_u64().unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let state = snapshot(&host);
        let log = state["restore"]["log"].as_str().unwrap().to_string();
        if log.contains("Determining projects to restore") {
            break;
        }
        assert!(Instant::now() < deadline, "restore never reached NuGet: {log}");
        std::thread::sleep(Duration::from_millis(50));
    }

    let started = Instant::now();
    let cancelled = host.dispatch("restore.cancel", json!({})).unwrap();
    assert!(started.elapsed() < Duration::from_secs(6));
    let restore = &cancelled["restore"];
    assert_eq!(restore["state"], "cancelled");
    assert_eq!(restore["awaitingDecision"], false);
    assert!(restore["termination"]["requested"]
        .as_str()
        .unwrap()
        .contains("SIGTERM"));
    assert_eq!(restore["termination"]["observed"], "exited");
    assert!(restore["log"]
        .as_str()
        .unwrap()
        .contains("Determining projects to restore"));
    let process = cancelled["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|process| process["id"].as_u64() == Some(process_id))
        .unwrap();
    assert_eq!(process["state"], "exited");
    // The reported exit is exactly what the registry observed; a signal kill stays unknown.
    assert_eq!(restore["exit"], process["exit"]);
    assert_ne!(restore["exit"], json!(0));
    assert_eq!(
        error_code(host.dispatch("restore.cancel", json!({}))),
        "invalid_state"
    );
}

#[cfg(unix)]
#[test]
fn revoking_trust_seals_a_running_restore_and_drops_late_results() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    let source = holding_server();
    write_hanging_project(&root, &source);

    let host = Host::new();
    open(&host, &root.join("Hang.csproj"));
    grant(&host);
    let started = host.dispatch("restore.start", json!({})).unwrap();
    let process_id = started["restore"]["processId"].as_u64().unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let state = snapshot(&host);
        if state["restore"]["log"]
            .as_str()
            .unwrap()
            .contains("Determining projects to restore")
        {
            break;
        }
        assert!(Instant::now() < deadline, "restore never reached NuGet");
        std::thread::sleep(Duration::from_millis(50));
    }

    let revoked = host.dispatch("trust.revoke", json!({})).unwrap();
    assert_eq!(revoked["trust"]["granted"], false);
    let restore = &revoked["restore"];
    assert_eq!(restore["sealed"], true);
    assert!(restore["staleReason"].as_str().unwrap().contains("Trust"));
    assert_eq!(restore["state"], "cancelled");
    assert_eq!(restore["termination"]["observed"], "exited");
    let process = revoked["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|process| process["id"].as_u64() == Some(process_id))
        .unwrap();
    assert_eq!(process["state"], "exited");

    // A late write from the old process must not rewrite the sealed record.
    std::thread::sleep(Duration::from_millis(500));
    let later = snapshot(&host);
    assert_eq!(later["restore"]["state"], "cancelled");
    assert_eq!(later["restore"]["sealed"], true);
    assert_eq!(later["restore"]["finishedAt"], restore["finishedAt"]);
    assert_eq!(later["trust"]["granted"], false);
    assert_eq!(later["evaluation"]["state"], "idle");
    assert_eq!(
        error_code(host.dispatch("restore.start", json!({}))),
        "trust_required"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn a_failed_restore_keeps_its_log_and_retry_succeeds_after_the_fix() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    global_json_8(&root);
    fs::write(
        root.join("NuGet.config"),
        "<configuration><packageSources><clear /></packageSources></configuration>\n",
    )
    .unwrap();
    write_project(
        &root,
        "Broken",
        "<ItemGroup><PackageReference Include=\"xunit\" Version=\"9.9.999\" /></ItemGroup>",
    );

    let host = Host::new();
    open(&host, &root.join("Broken.csproj"));
    grant(&host);
    host.dispatch("evaluation.run", json!({})).unwrap();
    host.dispatch("restore.start", json!({})).unwrap();
    let failed = wait_for_restore(&host, Duration::from_secs(60));
    assert_eq!(failed["state"], "failed", "{}", failed["log"]);
    assert_eq!(failed["exit"], 1);
    let log = failed["log"].as_str().unwrap();
    assert!(log.contains("error NU1100"), "{log}");
    assert!(log.contains("Failed to restore"), "{log}");
    assert_eq!(failed["projects"][0]["state"], "not_restored");
    // NuGet writes an assets file even on failure; the cache records the failed result and
    // the host must not present it as restored.
    assert!(fs::read_to_string(root.join("obj/project.nuget.cache"))
        .unwrap()
        .contains("\"success\": false"));

    let stale = snapshot(&host);
    assert_eq!(stale["evaluation"]["stale"], true);
    assert!(stale["evaluation"]["staleReason"]
        .as_str()
        .unwrap()
        .contains("Restore failed"));

    write_project(
        &root,
        "Broken",
        "<ItemGroup><PackageReference Include=\"xunit\" Version=\"2.9.2\" /></ItemGroup>",
    );
    host.dispatch("restore.retry", json!({})).unwrap();
    let retried = wait_for_restore(&host, Duration::from_secs(60));
    assert_eq!(retried["state"], "ok", "{}", retried["log"]);
    assert_eq!(retried["revision"], 2);
    assert_eq!(retried["exit"], 0);
    assert_eq!(retried["projects"][0]["state"], "restored");
    assert!(root.join("obj/project.assets.json").exists());
}

#[cfg(target_os = "macos")]
#[test]
fn restore_can_target_one_project_of_a_solution() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    global_json_8(&root);
    write_project(&root, "One", "");
    fs::create_dir_all(root.join("lib")).unwrap();
    write_project(&root.join("lib"), "Two", "");
    fs::write(
        root.join("Pair.sln"),
        "Microsoft Visual Studio Solution File, Format Version 12.00\n\
Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"One\", \"One.csproj\", \"{11111111-1111-1111-1111-111111111111}\"\nEndProject\n\
Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Two\", \"lib\\Two.csproj\", \"{22222222-2222-2222-2222-222222222222}\"\nEndProject\n\
Global\n\tGlobalSection(SolutionConfigurationPlatforms) = preSolution\n\t\tDebug|Any CPU = Debug|Any CPU\n\tEndGlobalSection\nEndGlobal\n",
    )
    .unwrap();

    let host = Host::new();
    open(&host, &root.join("Pair.sln"));
    grant(&host);
    let started = host
        .dispatch("restore.start", json!({ "project": "lib/Two.csproj" }))
        .unwrap();
    let restore = &started["restore"];
    assert_eq!(restore["scope"], "project");
    assert_eq!(
        restore["target"].as_str().unwrap(),
        root.join("lib/Two.csproj").to_string_lossy()
    );
    assert_eq!(restore["projects"].as_array().unwrap().len(), 0);
    let command = restore["command"].as_array().unwrap();
    assert_eq!(command[2], restore["target"]);
    let finished = wait_for_restore(&host, Duration::from_secs(60));
    assert_eq!(finished["state"], "ok", "{}", finished["log"]);
    assert_eq!(finished["projects"].as_array().unwrap().len(), 1);
    assert_eq!(finished["projects"][0]["name"], "Two");
    assert!(root.join("lib/obj/project.assets.json").exists());
    assert!(!root.join("obj/project.assets.json").exists());

    let outside = host
        .dispatch("restore.start", json!({ "project": "../Outside.csproj" }))
        .expect_err("a project outside the solution is rejected");
    assert_eq!(outside.code, "invalid_state");
}

#[cfg(unix)]
#[test]
fn switching_workspaces_stops_a_running_restore_and_drops_its_record() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    let source = holding_server();
    write_hanging_project(&root, &source);
    let other = temp_workspace(&["Other.csproj"]);

    let host = Host::new();
    open(&host, &root.join("Hang.csproj"));
    grant(&host);
    let started = host.dispatch("restore.start", json!({})).unwrap();
    let process_id = started["restore"]["processId"].as_u64().unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if snapshot(&host)["restore"]["log"]
            .as_str()
            .unwrap()
            .contains("Determining projects to restore")
        {
            break;
        }
        assert!(Instant::now() < deadline, "restore never reached NuGet");
        std::thread::sleep(Duration::from_millis(50));
    }

    let switched = open(&host, &other.path().join("Other.csproj"));
    assert_eq!(switched["workspace"]["name"], "Other.csproj");
    assert_eq!(switched["trust"]["granted"], false);
    assert_eq!(switched["restore"]["revision"], 0);
    assert_eq!(switched["restore"]["state"], "idle");
    let process = switched["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|process| process["id"].as_u64() == Some(process_id))
        .unwrap();
    assert_eq!(process["state"], "exited");
    assert_eq!(process["termination"]["observed"], "exited");
    assert_eq!(
        error_code(host.dispatch("restore.start", json!({}))),
        "trust_required"
    );
}
