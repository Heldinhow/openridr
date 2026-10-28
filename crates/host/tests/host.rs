use openridr_host::{Host, EXECUTABLE_METHODS};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

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
