#[path = "support/mod.rs"]
mod support;

#[cfg(target_os = "macos")]
use openridr_host::hostroot;
use openridr_host::{sdk, Host, EXECUTABLE_METHODS};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use support::EnvVar;

// Tool installation and LSP tests share the app home (or `OPENRIDR_HOME`) and child processes, so
// they run one at a time. The guard also removes the ambient `OPENRIDR_*` values of whoever ran
// `cargo test`, so a shell cannot change the result.

fn fixture() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/OrderDesk"))
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
    let opened = host
        .dispatch("workspace.open", json!({ "path": path }))
        .expect("workspace opens");
    host.allow_setup_for_tests();
    opened
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
    // Restore compares `obj/project.assets.json` with the project file by mtime. A copy keeps the
    // checkout timestamps, so under load a restore finishing in the same millisecond could look
    // older than its project and the next gate would say "restore_required". A fixed past time
    // makes every freshly written asset unambiguously newer.
    age_tree(&copy);
    copy
}

/// Sets every file and directory under `root` to a fixed timestamp in the past.
fn age_tree(root: &Path) {
    use std::time::{Duration, UNIX_EPOCH};
    let old = UNIX_EPOCH + Duration::from_secs(1_600_000_000);
    fn walk(dir: &Path, old: std::time::SystemTime) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, old);
            }
            if let Ok(file) = fs::File::open(&path) {
                let _ = file.set_modified(old);
            } else {
                let _ = fs::File::open(dir).and_then(|handle| handle.set_modified(old));
            }
        }
        if let Ok(handle) = fs::File::open(dir) {
            let _ = handle.set_modified(old);
        }
    }
    walk(root, old);
    if let Ok(handle) = fs::File::open(root) {
        let _ = handle.set_modified(old);
    }
}

/// Stages the two files the host reads to report `restored` (`obj/project.assets.json` and a
/// `success: true` `obj/project.nuget.cache`), written after the copy so their mtimes are newer
/// than the project files. NuGet itself is not run: a real restore needs network and an ambient
/// package cache, and `restore_reports_the_exact_command_and_creates_real_assets` already proves
/// a real restore, while `evaluation_of_a_fresh_copy_with_spaces_creates_no_restore_artifacts`
/// proves a copy without these artifacts is `not_restored`.
fn stage_restore_artifacts(root: &Path) {
    for project in [
        "src/OrderDesk.Cli",
        "src/OrderDesk.Core",
        "tests/OrderDesk.Tests",
    ] {
        let obj = root.join(project).join("obj");
        fs::create_dir_all(&obj).unwrap();
        fs::write(obj.join("project.assets.json"), "{}\n").unwrap();
        fs::write(obj.join("project.nuget.cache"), "{\"success\": true}\n").unwrap();
    }
}

#[test]
fn app_home_never_points_at_the_real_application_support_directory() {
    // Reads the app home from the environment, so the shared environment must be in place first.
    support::prepare();
    let home = openridr_host::tools::app_home();
    let real = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join("Library/Application Support/OpenRidr"));
    assert_ne!(
        Some(&home),
        real.as_ref(),
        "cargo tests must not touch the real app home"
    );
    // OPENRIDR_HOME is an explicit override; without it a test binary gets a per-process temp home.
    if std::env::var_os("OPENRIDR_HOME").is_none() {
        assert!(
            home.starts_with(std::env::temp_dir()),
            "test app home is not temporary: {}",
            home.display()
        );
    }
}

#[test]
fn text_outline_reads_solution_projects_without_evaluation() {
    let host = support::host();
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
    assert_eq!(snapshot["outline"]["globalJson"]["rollForward"], "disable");
}

/// A workspace inside a parent folder, so the paths one level up really exist. The escape cases
/// below then depend on the confinement rules, not on how deep the checkout happens to be or on
/// what `/etc` contains on the machine running the suite.
fn nested_workspace() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let outer = tempfile::tempdir().unwrap();
    let root = outer.path().join("ws");
    fs::create_dir_all(root.join("sub")).unwrap();
    fs::write(root.join("App.csproj"), "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n").unwrap();
    fs::write(root.join("App.cs"), "class C {}\n").unwrap();
    fs::write(outer.path().join("outside.txt"), "outside\n").unwrap();
    let outside = fs::canonicalize(outer.path()).unwrap();
    (outer, fs::canonicalize(&root).unwrap(), outside)
}

fn text(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

/// Sorted entry names of a folder, so a test can state exactly what must not have appeared there.
fn entry_names(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().to_string())
        .collect();
    names.sort();
    names
}

#[test]
fn file_access_is_confined_to_the_workspace() {
    let (_outer, root, outside) = nested_workspace();
    let outside_file = outside.join("outside.txt");
    let host = support::host();
    open(&host, &root.join("App.csproj"));

    let read = host
        .dispatch("file.read", json!({ "path": "App.cs" }))
        .unwrap();
    assert!(read["content"].as_str().unwrap().contains("class C"));

    // Every escape is refused by shape, whether or not its target exists: a missing path outside
    // must never answer `not_found`, because that answer would tell the caller what the filesystem
    // outside the workspace contains.
    let escapes = [
        text(&outside_file),
        text(&outside.join("does-not-exist.txt")),
        "../outside.txt".to_string(),
        "../does-not-exist.txt".to_string(),
        "/etc/hosts".to_string(),
        "/etc/openridr-does-not-exist".to_string(),
        "../../../../../../../../etc/hosts".to_string(),
        "sub/../../outside.txt".to_string(),
        "sub/../../../../../../../../etc/openridr-does-not-exist".to_string(),
    ];
    for escape in &escapes {
        assert_eq!(
            error_code(host.dispatch("file.read", json!({ "path": escape }))),
            "outside_workspace",
            "expected {escape} to be rejected"
        );
        assert_eq!(
            error_code(host.dispatch(
                "file.write",
                json!({
                    "path": escape,
                    "content": "overwritten\n",
                    "expected_hash": "fnv1a64:0000000000000000"
                })
            )),
            "outside_workspace",
            "expected a write to {escape} to be rejected"
        );
    }
    assert_eq!(fs::read_to_string(&outside_file).unwrap(), "outside\n");
    assert_eq!(
        error_code(host.dispatch("file.read", json!({ "path": "" }))),
        "path_not_allowed"
    );

    // A symlink inside the workspace is rejected after canonicalization, both as the final
    // component and as an intermediate one.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside_file, root.join("Link.cs")).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("LinkDir")).unwrap();
        std::os::unix::fs::symlink(&outside_file, root.join("sub/Link.cs")).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("sub/LinkDir")).unwrap();
        // A dangling symlink exists but points somewhere the host cannot resolve, so where it leads
        // is unprovable: the answer must not depend on whether the target outside exists.
        std::os::unix::fs::symlink(outside.join("missing.txt"), root.join("Dangling.cs")).unwrap();
        std::os::unix::fs::symlink(outside.join("outside.txt"), root.join("DanglingThere.cs"))
            .unwrap();
        // Even a dangling link whose target name would be inside the workspace is refused, so the
        // rule is "dangling is outside" and never leaks what exists on the other side.
        std::os::unix::fs::symlink("not-yet-created.cs", root.join("Pending.cs")).unwrap();
        std::os::unix::fs::symlink(outside.join("missing-dir"), root.join("DanglingDir")).unwrap();
        for escape in [
            "Link.cs",
            "LinkDir",
            "LinkDir/outside.txt",
            "sub/Link.cs",
            "sub/LinkDir/outside.txt",
            // Missing targets behind the same symlinks: the refusal must not change to `not_found`,
            // which would tell the caller what exists outside the workspace.
            "LinkDir/missing.txt",
            "LinkDir/deep/missing.txt",
            "sub/LinkDir/deep/missing.txt",
            // Dangling links: existing and missing targets outside answer alike.
            "Dangling.cs",
            "DanglingThere.cs",
            "Pending.cs",
            "DanglingDir",
            "DanglingDir/child.txt",
        ] {
            assert_eq!(
                error_code(host.dispatch("file.read", json!({ "path": escape }))),
                "outside_workspace",
                "expected {escape} to be rejected"
            );
            assert_eq!(
                error_code(host.dispatch(
                    "file.write",
                    json!({
                        "path": escape,
                        "content": "overwritten\n",
                        "expected_hash": "fnv1a64:0000000000000000"
                    })
                )),
                "outside_workspace",
                "expected a write through {escape} to be rejected"
            );
        }
        assert_eq!(fs::read_to_string(&outside_file).unwrap(), "outside\n");
    }
}

#[test]
fn file_operations_refuse_escapes_and_leave_the_outside_untouched() {
    let (_outer, root, outside) = nested_workspace();
    let outside_file = outside.join("outside.txt");
    let created = outside.join("created.txt");
    // The escape target is a fresh temporary directory, never a predictable name that two runs
    // could share; the relative path climbs out of the workspace and lands on it.
    let leaked_dir = tempfile::tempdir().unwrap();
    let leaked = leaked_dir.path().join("created.txt");
    let traversal = format!(
        "{}{}",
        "../".repeat(12),
        leaked.strip_prefix("/").unwrap().display()
    );
    let host = support::host();
    open(&host, &root.join("App.csproj"));

    for target in [
        created.clone(),
        outside.join("created-deep.txt"),
        PathBuf::from(&traversal),
    ] {
        assert_eq!(
            error_code(host.dispatch(
                "file.create",
                json!({ "path": text(&target), "content": "x" })
            )),
            "outside_workspace",
            "expected a create of {target:?} to be rejected"
        );
    }
    for target in [text(&outside.join("new.cs")), "../new.cs".to_string()] {
        assert_eq!(
            error_code(host.dispatch("file.create", json!({ "path": target, "content": "x" }))),
            "outside_workspace",
            "expected a create of {target} to be rejected"
        );
    }
    assert_eq!(entry_names(&outside), vec!["outside.txt", "ws"]);
    assert!(!created.exists());
    assert!(!leaked.exists(), "the escape created {}", leaked.display());

    for (from, to) in [
        ("App.cs".to_string(), text(&outside.join("moved.cs"))),
        ("App.cs".to_string(), "../moved.cs".to_string()),
        (text(&outside_file), "stolen.cs".to_string()),
        ("../outside.txt".to_string(), "stolen.cs".to_string()),
    ] {
        assert_eq!(
            error_code(host.dispatch("file.rename", json!({ "from": from, "to": to }))),
            "outside_workspace",
            "expected {from} -> {to} to be rejected"
        );
    }
    assert!(root.join("App.cs").exists());
    assert!(!root.join("stolen.cs").exists());
    assert!(outside_file.exists());

    for target in [
        text(&outside_file),
        "../outside.txt".to_string(),
        text(&outside),
        "..".to_string(),
    ] {
        assert_eq!(
            error_code(host.dispatch("file.delete", json!({ "path": target }))),
            "outside_workspace",
            "expected a delete of {target} to be rejected"
        );
    }
    assert_eq!(fs::read_to_string(&outside_file).unwrap(), "outside\n");

    // A symlinked folder inside the workspace carries the operation outside if it is followed.
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside, root.join("LinkDir")).unwrap();
        std::os::unix::fs::symlink(outside.join("missing-dir"), root.join("DanglingDir")).unwrap();
        assert_eq!(
            error_code(host.dispatch(
                "file.create",
                json!({ "path": "LinkDir/created.txt", "content": "x" })
            )),
            "outside_workspace"
        );
        assert_eq!(
            error_code(host.dispatch(
                "file.rename",
                json!({ "from": "App.cs", "to": "LinkDir/moved.cs" })
            )),
            "outside_workspace"
        );
        assert_eq!(entry_names(&outside), vec!["outside.txt", "ws"]);
        // The parent does not exist, but the symlink still carries the path outside.
        for path in ["LinkDir/sub/created.txt", "LinkDir/sub/deep/created.txt"] {
            assert_eq!(
                error_code(host.dispatch("file.create", json!({ "path": path, "content": "x" }))),
                "outside_workspace",
                "expected a create of {path} to be rejected"
            );
        }
        assert_eq!(
            error_code(host.dispatch(
                "file.rename",
                json!({ "from": "App.cs", "to": "LinkDir/sub/moved.cs" })
            )),
            "outside_workspace"
        );
        assert_eq!(
            error_code(host.dispatch("file.delete", json!({ "path": "LinkDir/sub/missing.cs" }))),
            "outside_workspace"
        );
        // A dangling folder link is unprovable, so a create or rename through it is refused.
        for path in ["DanglingDir/new.cs", "DanglingDir/deep/new.cs"] {
            assert_eq!(
                error_code(host.dispatch("file.create", json!({ "path": path, "content": "x" }))),
                "outside_workspace",
                "expected a create of {path} to be rejected"
            );
        }
        assert_eq!(
            error_code(host.dispatch(
                "file.rename",
                json!({ "from": "App.cs", "to": "DanglingDir/moved.cs" })
            )),
            "outside_workspace"
        );
        assert_eq!(entry_names(&outside), vec!["outside.txt", "ws"]);
        // Deleting the link itself is inside the workspace and must not follow it.
        host.dispatch("file.delete", json!({ "path": "LinkDir" }))
            .expect("a symlink inside the workspace can be deleted");
        assert_eq!(fs::read_to_string(&outside_file).unwrap(), "outside\n");
        assert!(outside.is_dir());
    }

    // The refusals above are confinement, not a broken surface.
    host.dispatch(
        "file.create",
        json!({ "path": "New.cs", "content": "class N {}\n" }),
    )
    .expect("a file inside the workspace is created");
    host.dispatch(
        "file.rename",
        json!({ "from": "New.cs", "to": "Renamed.cs" }),
    )
    .expect("a file inside the workspace is renamed");
    host.dispatch("file.delete", json!({ "path": "Renamed.cs" }))
        .expect("a file inside the workspace is deleted");
}

/// The UI hands the host absolute paths whenever `workspaceRelative` cannot strip the prefix
/// (`src/solutionNodes.ts`): a stack trace, a build log or a DAP frame from a tool that spells the
/// folder differently. Containment must be decided by the canonical path, not by the text, or a
/// save through `/tmp/...` would be refused for a workspace canonicalized to `/private/tmp/...`.
#[cfg(unix)]
#[test]
fn absolute_paths_reach_the_workspace_through_a_symlinked_prefix() {
    let (_outer, root, outside) = nested_workspace();
    // `outside/alias` points at the workspace, so the textual prefix differs from the canonical
    // root while the target is the very same file.
    std::os::unix::fs::symlink(&root, outside.join("alias")).unwrap();
    let host = support::host();
    open(&host, &root.join("App.csproj"));

    let via_alias = outside.join("alias/App.cs");
    let read = host
        .dispatch("file.read", json!({ "path": text(&via_alias) }))
        .expect("an absolute path inside the workspace opens through a symlinked prefix");
    assert!(read["content"].as_str().unwrap().contains("class C"));
    assert_eq!(read["path"], "App.cs");

    let hash = read["hash"].as_str().unwrap().to_string();
    host.dispatch(
        "file.write",
        json!({ "path": text(&via_alias), "content": "class Alias {}\n", "expected_hash": hash }),
    )
    .expect("a save through a symlinked prefix lands inside the workspace");
    assert_eq!(
        fs::read_to_string(root.join("App.cs")).unwrap(),
        "class Alias {}\n"
    );

    // Creating through the alias is inside the workspace too, and the parent exists.
    host.dispatch(
        "file.create",
        json!({ "path": text(&outside.join("alias/New.cs")), "content": "class N {}\n" }),
    )
    .expect("a create through a symlinked prefix lands inside the workspace");
    assert!(root.join("New.cs").is_file());

    // Absolute paths that really are outside stay refused, existing or not.
    for escape in [
        text(&outside.join("outside.txt")),
        text(&outside.join("missing.txt")),
    ] {
        assert_eq!(
            error_code(host.dispatch("file.read", json!({ "path": escape }))),
            "outside_workspace",
            "expected {escape} to be rejected"
        );
    }

    // macOS APFS ignores case by default: when the disk resolves either spelling, so must the host.
    #[cfg(target_os = "macos")]
    {
        let shouted = PathBuf::from(root.to_string_lossy().to_uppercase());
        if fs::canonicalize(&shouted).ok().as_deref() == Some(root.as_path()) {
            let read = host
                .dispatch(
                    "file.read",
                    json!({ "path": text(&shouted.join("App.cs")) }),
                )
                .expect("a case-different spelling of the workspace opens");
            assert!(read["content"].as_str().unwrap().contains("class Alias"));
        }
    }
}

#[test]
fn the_directory_listing_refuses_escapes_by_shape() {
    let (_outer, root, outside) = nested_workspace();
    let host = support::host();
    open(&host, &root.join("App.csproj"));

    for dir in [
        "..".to_string(),
        "../..".to_string(),
        text(&outside),
        "/etc".to_string(),
        "/etc/openridr-does-not-exist".to_string(),
    ] {
        assert_eq!(
            error_code(host.dispatch("workspace.tree", json!({ "dir": dir }))),
            "outside_workspace",
            "expected the listing of {dir} to be rejected"
        );
    }

    let listing = host
        .dispatch("workspace.tree", json!({ "dir": "" }))
        .unwrap();
    assert!(listing["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|entry| entry["name"] == "App.cs"));

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside, root.join("LinkDir")).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("sub/LinkDir")).unwrap();
        std::os::unix::fs::symlink(outside.join("missing-dir"), root.join("DanglingDir")).unwrap();
        for dir in [
            "LinkDir",
            "sub/LinkDir",
            "LinkDir/missing",
            "sub/LinkDir/deep/missing",
            "DanglingDir",
            "DanglingDir/child",
        ] {
            assert_eq!(
                error_code(host.dispatch("workspace.tree", json!({ "dir": dir }))),
                "outside_workspace",
                "expected the listing of {dir} to be rejected"
            );
        }
    }
}

#[test]
fn directory_resolution_requires_an_explicit_choice() {
    let host = support::host();
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
    let host = support::host();
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
            }),
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

fn utf16_bytes(text: &str, little_endian: bool) -> Vec<u8> {
    let mut bytes = if little_endian {
        vec![0xFF, 0xFE]
    } else {
        vec![0xFE, 0xFF]
    };
    for unit in text.encode_utf16() {
        bytes.extend(if little_endian {
            unit.to_le_bytes()
        } else {
            unit.to_be_bytes()
        });
    }
    bytes
}

fn read_file(host: &Host, path: &str) -> Value {
    host.dispatch("file.read", json!({ "path": path }))
        .expect("file reads")
}

#[test]
fn save_preserves_crlf_lf_and_mixed_line_endings() {
    let host = support::host();
    let workspace = temp_workspace(&["App.csproj"]);
    open(&host, &workspace.path().join("App.csproj"));
    let crlf = workspace.path().join("Crlf.cs");
    let lf = workspace.path().join("Lf.cs");
    let mixed = workspace.path().join("Mixed.cs");
    fs::write(&crlf, "one\r\ntwo\r\n").unwrap();
    fs::write(&lf, "one\ntwo\n").unwrap();
    fs::write(&mixed, "one\r\ntwo\nthree\n").unwrap();

    let read = read_file(&host, "Crlf.cs");
    assert_eq!(read["content"], "one\ntwo\n");
    let saved = host
        .dispatch(
            "file.write",
            json!({
                "path": "Crlf.cs",
                "content": "one\ntwo\nthree\n",
                "expected_hash": read["hash"]
            }),
        )
        .unwrap();
    assert_eq!(fs::read(&crlf).unwrap(), b"one\r\ntwo\r\nthree\r\n");
    assert_eq!(read_file(&host, "Crlf.cs")["hash"], saved["hash"]);

    let read = read_file(&host, "Lf.cs");
    assert_eq!(read["content"], "one\ntwo\n");
    host.dispatch(
        "file.write",
        json!({
            "path": "Lf.cs",
            "content": "one\ntwo\nthree\n",
            "expected_hash": read["hash"]
        }),
    )
    .unwrap();
    assert_eq!(fs::read(&lf).unwrap(), b"one\ntwo\nthree\n");

    // A mixed file keeps CRLF because the first CRLF found defines the style.
    let read = read_file(&host, "Mixed.cs");
    assert_eq!(read["content"], "one\ntwo\nthree\n");
    host.dispatch(
        "file.write",
        json!({
            "path": "Mixed.cs",
            "content": "one\ntwo\nthree\nfour\n",
            "expected_hash": read["hash"]
        }),
    )
    .unwrap();
    assert_eq!(
        fs::read(&mixed).unwrap(),
        b"one\r\ntwo\r\nthree\r\nfour\r\n"
    );
}

#[test]
fn save_preserves_utf16_and_utf8_bom_encodings() {
    let host = support::host();
    let workspace = temp_workspace(&["App.csproj"]);
    open(&host, &workspace.path().join("App.csproj"));
    let text = "olá 🌎\r\nfim\r\n";
    fs::write(workspace.path().join("Le.cs"), utf16_bytes(text, true)).unwrap();
    fs::write(workspace.path().join("Be.cs"), utf16_bytes(text, false)).unwrap();
    fs::write(
        workspace.path().join("Bom.cs"),
        [&[0xEF, 0xBB, 0xBF][..], b"a\r\nb\r\n"].concat(),
    )
    .unwrap();
    fs::write(workspace.path().join("Plain.cs"), "a\nb\n").unwrap();

    for (name, little_endian) in [("Le.cs", true), ("Be.cs", false)] {
        let read = read_file(&host, name);
        assert_eq!(read["content"], "olá 🌎\nfim\n");
        let saved = host
            .dispatch(
                "file.write",
                json!({
                    "path": name,
                    "content": "olá 🌎\nfim\nagain\n",
                    "expected_hash": read["hash"]
                }),
            )
            .unwrap();
        let bytes = fs::read(workspace.path().join(name)).unwrap();
        let bom = if little_endian {
            [0xFF, 0xFE]
        } else {
            [0xFE, 0xFF]
        };
        assert_eq!(bytes[..2], bom);
        let units: Vec<u16> = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| {
                if little_endian {
                    u16::from_le_bytes([pair[0], pair[1]])
                } else {
                    u16::from_be_bytes([pair[0], pair[1]])
                }
            })
            .collect();
        assert_eq!(
            String::from_utf16(&units).unwrap(),
            "olá 🌎\r\nfim\r\nagain\r\n"
        );
        assert_eq!(read_file(&host, name)["hash"], saved["hash"]);
    }

    let read = read_file(&host, "Bom.cs");
    assert_eq!(read["content"], "a\nb\n");
    host.dispatch(
        "file.write",
        json!({
            "path": "Bom.cs",
            "content": "a\nb\nc\n",
            "expected_hash": read["hash"]
        }),
    )
    .unwrap();
    assert_eq!(
        fs::read(workspace.path().join("Bom.cs")).unwrap(),
        [&[0xEF, 0xBB, 0xBF][..], b"a\r\nb\r\nc\r\n"].concat()
    );

    // A plain UTF-8 file must not gain a BOM.
    let read = read_file(&host, "Plain.cs");
    host.dispatch(
        "file.write",
        json!({
            "path": "Plain.cs",
            "content": "x\ny\n",
            "expected_hash": read["hash"]
        }),
    )
    .unwrap();
    assert_eq!(
        fs::read(workspace.path().join("Plain.cs")).unwrap(),
        b"x\ny\n"
    );
}

#[test]
fn conflict_hash_is_over_normalized_text() {
    let host = support::host();
    let workspace = temp_workspace(&["App.csproj"]);
    open(&host, &workspace.path().join("App.csproj"));
    let file = workspace.path().join("App.cs");
    fs::write(&file, "one\r\ntwo\r\n").unwrap();
    fs::write(workspace.path().join("Other.cs"), "one\ntwo\n").unwrap();

    let crlf = read_file(&host, "App.cs");
    let lf = read_file(&host, "Other.cs");
    assert_eq!(crlf["hash"], lf["hash"]);

    fs::write(&file, "one\r\ntwo\nthree\r\n").unwrap();
    let current = read_file(&host, "App.cs");
    let conflict = host
        .dispatch(
            "file.write",
            json!({
                "path": "App.cs",
                "content": "my buffer\n",
                "expected_hash": crlf["hash"]
            }),
        )
        .expect_err("stale expected hash conflicts");
    assert_eq!(conflict.code, "conflict");
    let details = conflict.details.unwrap();
    assert_eq!(details["currentContent"], "one\ntwo\nthree\n");
    assert_eq!(details["currentHash"], current["hash"]);
    assert_eq!(fs::read(&file).unwrap(), b"one\r\ntwo\nthree\r\n");
}

#[test]
fn invalid_encodings_are_clear_errors_and_never_write() {
    let host = support::host();
    let workspace = temp_workspace(&["App.csproj"]);
    open(&host, &workspace.path().join("App.csproj"));
    let bad_utf8 = workspace.path().join("Bad.cs");
    let bad_utf16 = workspace.path().join("Odd.cs");
    fs::write(&bad_utf8, [0xC3, 0x28]).unwrap();
    fs::write(&bad_utf16, [0xFF, 0xFE, 0x41]).unwrap();

    let error = host
        .dispatch("file.read", json!({ "path": "Bad.cs" }))
        .expect_err("invalid UTF-8 fails");
    assert_eq!(error.code, "operation_failed");
    assert!(error.message.contains("UTF-8"), "{}", error.message);

    let error = host
        .dispatch(
            "file.write",
            json!({
                "path": "Bad.cs",
                "content": "x\n",
                "expected_hash": "fnv1a64:0000000000000000"
            }),
        )
        .expect_err("invalid UTF-8 fails before writing");
    assert_eq!(error.code, "operation_failed");
    assert_eq!(fs::read(&bad_utf8).unwrap(), [0xC3, 0x28]);

    let error = host
        .dispatch("file.read", json!({ "path": "Odd.cs" }))
        .expect_err("odd UTF-16LE fails");
    assert_eq!(error.code, "operation_failed");
    assert!(error.message.contains("UTF-16LE"), "{}", error.message);
}

#[test]
fn executable_methods_fail_before_trust_and_spawn_nothing() {
    let host = support::host();
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
    let host = support::host();
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
    host.dispatch("trust.grant", json!({}))
        .expect("trust is granted")
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
    let without_workspace = support::host();
    assert_eq!(
        error_code(without_workspace.dispatch("trust.grant", json!({}))),
        "invalid_state"
    );
    assert_eq!(
        error_code(without_workspace.dispatch("trust.revoke", json!({}))),
        "invalid_state"
    );

    let host = support::host();
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
            && **method != "lsp.start"
            && **method != "lsp.restart"
            && **method != "lsp.didOpen"
            && **method != "lsp.didChange"
            && **method != "tools.install"
            && **method != "tools.retry"
            && **method != "debug.install"
            && **method != "debug.retry"
            && **method != "terminal.start"
            && **method != "debug.start"
            && **method != "nuget.applyUpdate"
            && **method != "nuget.probe"
            && !method.starts_with("git.")
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
    for (method, code) in [
        ("tools.install", "confirmation_required"),
        ("tools.retry", "invalid_state"),
        ("debug.install", "confirmation_required"),
        ("debug.retry", "invalid_state"),
        ("nuget.applyUpdate", "confirmation_required"),
        ("lsp.start", "tool_missing"),
        ("lsp.restart", "tool_missing"),
        ("lsp.didOpen", "invalid_state"),
        ("lsp.didChange", "invalid_state"),
    ] {
        assert_eq!(
            error_code(host.dispatch(method, json!({}))),
            code,
            "{method} must fail without starting a process"
        );
    }
    assert_eq!(
        error_code(host.dispatch("evaluation.run", json!({}))),
        "invalid_state",
        "evaluation.run needs a resolved project SDK"
    );
    assert_eq!(
        error_code(host.dispatch("run.start", json!({}))),
        "restore_required",
        "run.start must pass the shared restore gate before evaluation/build checks"
    );
    // Git reads return data instead of pretending an unavailable repository is an error; a
    // write still refuses before running anything.
    let status = host.dispatch("git.status", json!({})).unwrap();
    assert_eq!(status["state"], "not_a_repository");
    assert_eq!(status["message"], "Not a git repository");
    let info = host.dispatch("git.info", json!({})).unwrap();
    assert_eq!(info["state"], "found");
    assert!(info["path"].as_str().unwrap().ends_with("git"));
    assert!(info["version"].as_str().unwrap().contains("git version"));
    assert_eq!(
        error_code(host.dispatch("git.checkout", json!({ "name": "main" }))),
        "git_not_a_repository"
    );
    let debug_start = host
        .dispatch("debug.start", json!({}))
        .expect_err("debug.start needs the platform flag, a verified adapter and a build");
    assert!(
        matches!(
            debug_start.code.as_str(),
            "experimental_required" | "tool_missing" | "unsupported_platform" | "restore_required"
        ),
        "{}: {}",
        debug_start.code,
        debug_start.message
    );
    assert_eq!(snapshot(&host)["processes"], json!([]));
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
fn nuget_update_requires_confirmation_and_a_current_plan_before_any_process() {
    let host = support::host();
    let workspace = temp_workspace(&["App.csproj"]);
    open(&host, &workspace.path().join("App.csproj"));
    host.dispatch("trust.grant", json!({})).unwrap();

    assert_eq!(
        error_code(host.dispatch("nuget.applyUpdate", json!({}))),
        "confirmation_required"
    );
    let stale = host
        .dispatch(
            "nuget.applyUpdate",
            json!({ "planId": "missing", "confirm": true }),
        )
        .expect_err("an unknown plan never runs");
    assert_eq!(stale.code, "nuget_stale_plan");
    assert_eq!(snapshot(&host)["processes"], json!([]));
}

#[test]
fn a_welcome_trust_decision_still_requires_setup_before_execution() {
    let host = support::host();
    let workspace = temp_workspace(&["Welcome.csproj"]);
    let project = workspace.path().join("Welcome.csproj");
    // Welcome-screen decision, before any workspace is open.
    host.dispatch(
        "trust.set",
        json!({ "path": project, "decision": "trusted" }),
    )
    .unwrap();
    let opened = host
        .dispatch("workspace.open", json!({ "path": project }))
        .unwrap();
    assert_eq!(opened["trust"]["granted"], true);
    assert_ne!(opened["setup"]["state"], "ready");
    assert_eq!(
        error_code(host.dispatch("build.start", json!({}))),
        "setup_required"
    );
    assert_eq!(
        error_code(host.dispatch("lsp.start", json!({}))),
        "setup_required"
    );
}

#[test]
fn trust_set_refuses_another_path_while_a_workspace_is_open() {
    let host = support::host();
    let workspace = temp_workspace(&["Open.csproj"]);
    let other = temp_workspace(&["Other.csproj"]);
    let open = workspace.path().join("Open.csproj");
    let other = other.path().join("Other.csproj");
    host.dispatch("workspace.open", json!({ "path": open }))
        .unwrap();

    assert_eq!(
        error_code(host.dispatch("trust.set", json!({ "path": other, "decision": "trusted" }),)),
        "invalid_state"
    );
    assert_eq!(
        error_code(host.dispatch("trust.set", json!({ "path": open, "decision": "trusted" }),)),
        "setup_required"
    );
    host.allow_setup_for_tests();
    host.dispatch(
        "trust.set",
        json!({ "path": workspace.path().join("Open.csproj"), "decision": "trusted" }),
    )
    .unwrap();
    assert_eq!(
        error_code(host.dispatch(
            "trust.set",
            json!({ "path": other, "decision": "restricted" }),
        )),
        "invalid_state"
    );
}

#[test]
fn trust_and_executable_methods_are_refused_until_setup_is_ready() {
    let host = support::host();
    let workspace = temp_workspace(&["Gate.csproj"]);
    host.dispatch(
        "workspace.open",
        json!({ "path": workspace.path().join("Gate.csproj") }),
    )
    .unwrap();
    assert_eq!(
        error_code(host.dispatch("trust.grant", json!({}))),
        "setup_required"
    );
    assert_eq!(
        error_code(host.dispatch(
            "trust.set",
            json!({ "path": workspace.path().join("Gate.csproj"), "decision": "trusted" }),
        )),
        "setup_required"
    );

    // A remembered/direct trust still cannot run anything while setup is not ready.
    host.allow_setup_for_tests();
    host.dispatch("trust.grant", json!({})).unwrap();
    host.deny_setup_for_tests();
    for method in [
        "build.start",
        "restore.start",
        "tests.discover",
        "lsp.start",
    ] {
        assert_eq!(
            error_code(host.dispatch(method, json!({}))),
            "setup_required",
            "{method}"
        );
    }
    assert_eq!(snapshot(&host)["processes"], json!([]));
}

#[test]
fn a_cpm_item_fails_alone_instead_of_blocking_the_batch_plan() {
    let host = support::host();
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk cpm batch");
    fs::write(
        copy.join("Directory.Packages.props"),
        "<Project><PropertyGroup><ManagePackageVersionsCentrally>true</ManagePackageVersionsCentrally></PropertyGroup></Project>",
    )
    .unwrap();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    let planned = host
        .dispatch(
            "nuget.planBatch",
            json!({ "items": [{
                "project": "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
                "id": "xunit",
                "version": "2.9.3",
                "fromVersion": "2.9.2",
                "targetFramework": "net8.0"
            }] }),
        )
        .expect("CPM is deferred to the item, not the batch plan");
    assert_eq!(planned["count"], 1);
    let applied = host
        .dispatch(
            "nuget.applyBatch",
            json!({ "planId": planned["planId"], "confirm": true }),
        )
        .unwrap();
    assert_eq!(applied["state"], "failed");
    assert_eq!(applied["failed"], 1);
    assert_eq!(applied["items"][0]["state"], "failed");
    assert!(applied["items"][0]["message"]
        .as_str()
        .unwrap()
        .contains("Central Package Management"));
    assert_eq!(applied["restores"], json!([]));
}

#[test]
fn nuget_list_rejects_a_source_that_is_not_configured_or_not_https() {
    let host = support::host();
    open(&host, &fixture().join("OrderDesk.sln"));
    grant(&host);
    for source in [
        "https://evil.example/v3/index.json",
        "/tmp/local-feed",
        "http://127.0.0.1:1/v3/index.json",
    ] {
        let listing = host
            .dispatch(
                "nuget.list",
                json!({
                    "project": "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
                    "source": source
                }),
            )
            .unwrap();
        assert_eq!(listing["state"], "failed", "{source}");
        assert!(
            listing["message"]
                .as_str()
                .unwrap()
                .contains("not configured"),
            "{source}: {}",
            listing["message"]
        );
    }
}

#[test]
fn discover_only_accepts_a_configured_https_source() {
    let host = support::host();
    open(&host, &fixture().join("OrderDesk.sln"));
    grant(&host);
    let rejected = host
        .dispatch(
            "nuget.search",
            json!({
                "project": "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
                "source": "https://evil.example/v3/index.json",
                "query": "xunit"
            }),
        )
        .expect_err("an unconfigured source is refused before any request");
    assert_eq!(rejected.code, "source_not_allowed");
    let plain = host
        .dispatch(
            "nuget.search",
            json!({
                "project": "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
                "source": "http://127.0.0.1:1/v3/index.json",
                "query": "xunit"
            }),
        )
        .expect_err("plain HTTP is refused");
    assert_eq!(plain.code, "source_not_allowed");
}

#[test]
fn a_failed_dotnet_add_is_counted_as_failed_in_the_batch() {
    let host = support::host();
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk broken batch");
    let project = copy.join("tests/OrderDesk.Tests/OrderDesk.Tests.csproj");
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    let planned = host
        .dispatch(
            "nuget.planBatch",
            json!({ "items": [{
                "project": "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
                "id": "xunit",
                "version": "2.9.3",
                "fromVersion": "2.9.2",
                "targetFramework": "net8.0"
            }] }),
        )
        .unwrap();
    // Keep the PackageReference readable for revalidation but make MSBuild reject the file.
    let broken = fs::read_to_string(&project)
        .unwrap()
        .replace("</Project>", "");
    fs::write(&project, broken).unwrap();
    let applied = host
        .dispatch(
            "nuget.applyBatch",
            json!({ "planId": planned["planId"], "confirm": true }),
        )
        .unwrap();
    assert_eq!(applied["state"], "failed");
    assert_eq!(applied["failed"], 1);
    assert_eq!(applied["updated"], 0);
    assert_eq!(applied["items"][0]["state"], "failed");
    assert!(applied["items"][0]["message"].is_string());
    assert_eq!(applied["restores"], json!([]));
}

#[test]
fn nuget_batch_plans_each_item_and_requires_confirmation_and_the_current_plan() {
    let host = support::host();
    open(&host, &fixture().join("OrderDesk.sln"));
    host.dispatch("trust.grant", json!({})).unwrap();
    let project = "tests/OrderDesk.Tests/OrderDesk.Tests.csproj";
    let planned = host
        .dispatch(
            "nuget.planBatch",
            json!({ "items": [
                {
                    "project": project,
                    "id": "xunit",
                    "version": "2.9.3",
                    "fromVersion": "2.9.2",
                    "targetFramework": "net8.0"
                },
                {
                    "project": project,
                    "id": "xunit.runner.visualstudio",
                    "version": "4.0.1",
                    "fromVersion": "2.8.2",
                    "targetFramework": "net8.0"
                }
            ] }),
        )
        .unwrap();
    assert_eq!(planned["count"], 2);
    assert_eq!(planned["items"].as_array().unwrap().len(), 2);
    assert!(planned["items"][0]["command"]
        .as_str()
        .unwrap()
        .contains("add tests/OrderDesk.Tests/OrderDesk.Tests.csproj package xunit"));
    assert!(planned["items"][1]["command"]
        .as_str()
        .unwrap()
        .contains("package xunit.runner.visualstudio"));
    assert_eq!(snapshot(&host)["processes"], json!([]));

    let plan_id = planned["planId"].as_str().unwrap();
    assert_eq!(
        error_code(host.dispatch("nuget.applyBatch", json!({ "planId": plan_id }))),
        "confirmation_required"
    );
    let stale = host
        .dispatch(
            "nuget.applyBatch",
            json!({ "planId": "stale-batch", "confirm": true }),
        )
        .expect_err("a replaced batch never runs");
    assert_eq!(stale.code, "nuget_stale_plan");
    assert_eq!(snapshot(&host)["processes"], json!([]));
}

#[test]
fn closing_a_trusted_workspace_clears_trust_and_the_executable_context() {
    let host = support::host();
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
    let host = support::host();
    open(&host, &fixture());
    grant(&host);
    let old_token = host.context_token().unwrap();
    host.accept_result(old_token)
        .expect("current token is accepted");

    let other = temp_workspace(&["Other.csproj"]);
    let switched = open(&host, &other.path().join("Other.csproj"));
    assert_eq!(switched["workspace"]["name"], "Other.csproj");
    assert_eq!(switched["trust"]["granted"], false);

    let stale = host
        .accept_result(old_token)
        .expect_err("old workspace token is stale");
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
fn sdk_inventory_reads_the_real_roots_and_reports_a_runtime_and_the_project_sdk() {
    // The fixture pins 8.0.408; analysis needs the .NET 10 runtime, not an SDK 10. Any detected root may hold them (no machine-specific paths).
    let inventory = sdk::inventory();
    let versions: Vec<&str> = inventory
        .installs
        .iter()
        .map(|install| install.version.as_str())
        .collect();
    assert!(
        versions.contains(&"8.0.408"),
        "expected SDK 8.0.408 in the real inventory, found {versions:?}"
    );
    assert!(inventory
        .roots
        .iter()
        .all(|root| !root.root.is_empty() && !root.source.is_empty()));
    assert!(inventory
        .roots
        .iter()
        .all(|root| root.exit == Some(0) && root.error.is_none()));

    let runtime = sdk::AnalysisRuntime::from_runtimes(&inventory.runtimes);
    assert_eq!(runtime.state, "found");
    let runtime_version = runtime.version.as_deref().expect("runtime version");
    assert!(runtime_version.starts_with("10."), "{runtime_version}");

    let host = support::host();
    let opened = open(&host, &fixture());
    assert_eq!(
        opened["sdk"]["host"]["version"].as_str(),
        Some(runtime_version)
    );
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
    assert!(Path::new(project_root).is_absolute(), "{project_root}");
    assert!(
        Path::new(project_root).join("sdk/8.0.408").is_dir(),
        "{project_root}"
    );
    // The runtime may live in any detected root; the contract is the resolved version below.
    assert_eq!(
        trusted["sdk"]["host"]["version"].as_str(),
        Some(runtime_version)
    );
}

#[cfg(target_os = "macos")]
#[test]
fn project_sdk_missing_keeps_a_distinct_state_and_does_not_rewrite_global_json() {
    let fixture =
        fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/MissingSdk"))
            .expect("fixture MissingSdk exists");
    let global_json = fixture.join("global.json");
    let before = fs::read(&global_json).unwrap();

    let host = support::host();
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
    let host = support::host();
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
    let host = support::host();
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
    let host = support::host();
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
    let host = support::host();
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
    let host = support::host();
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
    let fixture =
        fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/MissingSdk"))
            .expect("fixture MissingSdk exists");
    let global_json = fixture.join("global.json");
    let before = fs::read(&global_json).unwrap();

    let host = support::host();
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
    let temp = tempfile::tempdir().unwrap();
    let workspace = fixture_copy(temp.path(), "OrderDesk");
    stage_restore_artifacts(&workspace);

    let host = support::host();
    open(&host, &workspace.join("OrderDesk.sln"));
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
            assert!(
                !source.contains("/obj/") && !source.contains("/bin/"),
                "{source}"
            );
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

fn outside_fixture() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/OutsideSolution"))
        .expect("fixture OutsideSolution exists")
}

/// A solution whose `.sln` names `..\lib\Lib.csproj`: the project is outside the workspace root,
/// which is the shape of the bug this fixture exists for.
#[test]
fn evaluation_includes_a_project_outside_the_solution_directory() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = temp.path().join("OutsideSolution");
    copy_tree(&outside_fixture(), &fixture);
    let app = fs::canonicalize(fixture.join("app")).unwrap();
    let lib = fs::canonicalize(fixture.join("lib")).unwrap();

    let host = support::host();
    open(&host, &app.join("App.sln"));
    grant(&host);
    let evaluated = host.dispatch("evaluation.run", json!({})).unwrap();
    let evaluation = &evaluated["evaluation"];
    assert_eq!(evaluation["state"], "ok", "{}", evaluation["log"]);

    let projects = evaluation["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 4, "{projects:?}");
    let project = |name: &str| {
        projects
            .iter()
            .find(|project| project["name"] == name)
            .unwrap()
    };

    let inside = project("App");
    assert_eq!(inside["state"], "ok", "{}", inside["log"]);
    assert_eq!(inside["kind"], "console");
    assert_eq!(inside["path"], "App.csproj");
    assert_eq!(inside["sources"], json!(["Program.cs"]));
    assert!(inside["targetPath"]
        .as_str()
        .unwrap()
        .ends_with("app/bin/Debug/net8.0/App.dll"));

    let outside = project("Lib");
    assert_eq!(outside["state"], "ok", "{}", outside["log"]);
    assert_eq!(outside["kind"], "library");
    // Outside the root the path is absolute and canonical: no `..` crosses the host API.
    assert_eq!(
        outside["path"],
        lib.join("Lib.csproj").to_string_lossy().as_ref()
    );
    assert_eq!(outside["tfms"], json!(["net8.0"]));
    assert!(outside["targetPath"]
        .as_str()
        .unwrap()
        .ends_with("lib/bin/Debug/net8.0/Lib.dll"));
    assert_eq!(
        outside["sources"],
        json!([lib.join("Greeting.cs").to_string_lossy()])
    );

    let outside_app = fs::canonicalize(fixture.join("outside-app")).unwrap();
    let tests = fs::canonicalize(fixture.join("tests")).unwrap();
    let console = project("OutsideApp");
    assert_eq!(console["state"], "ok", "{}", console["log"]);
    assert_eq!(console["kind"], "console");
    assert!(console["targetPath"]
        .as_str()
        .unwrap()
        .ends_with("outside-app/bin/Debug/net8.0/OutsideApp.dll"));
    assert_eq!(
        console["sources"],
        json!([outside_app.join("Program.cs").to_string_lossy()])
    );
    assert_eq!(project("Lib.Tests")["kind"], "test");

    // The scope is the root plus every outside project directory, canonical and in file order.
    assert_eq!(
        evaluated["workspace"]["scope"],
        json!([
            app.to_string_lossy(),
            lib.to_string_lossy(),
            outside_app.to_string_lossy(),
            tests.to_string_lossy()
        ])
    );
}

/// The scope is the only thing between the UI and the filesystem: the outside project's directory
/// is admitted, everything else keeps answering exactly as before.
#[test]
fn the_scope_admits_the_outside_project_and_refuses_everything_else() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = temp.path().join("OutsideSolution");
    copy_tree(&outside_fixture(), &fixture);
    let app = fs::canonicalize(fixture.join("app")).unwrap();
    let lib = fs::canonicalize(fixture.join("lib")).unwrap();
    let outside = temp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("Secret.cs"), "class Secret {}\n").unwrap();
    let outside = fs::canonicalize(&outside).unwrap();
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&outside, lib.join("LinkOut")).unwrap();
        std::os::unix::fs::symlink(outside.join("Missing.cs"), lib.join("Dangling.cs")).unwrap();
        std::os::unix::fs::symlink(&lib, app.join("LinkIn")).unwrap();
    }

    let host = support::host();
    open(&host, &app.join("App.sln"));
    grant(&host);

    let greeting = lib.join("Greeting.cs");
    let read = host
        .dispatch("file.read", json!({ "path": greeting.to_string_lossy() }))
        .unwrap();
    assert!(read["content"].as_str().unwrap().contains("class Greeting"));

    let saved = "namespace Lib;\n\npublic static class Greeting\n{\n    public static string For(string name) => name;\n}\n";
    host.dispatch(
        "file.write",
        json!({
            "path": greeting.to_string_lossy(),
            "content": saved,
            "expected_hash": read["hash"],
        }),
    )
    .unwrap();
    assert_eq!(fs::read_to_string(&greeting).unwrap(), saved);

    let created = lib.join("Extra.cs");
    host.dispatch(
        "file.create",
        json!({ "path": created.to_string_lossy(), "content": "class Extra {}\n" }),
    )
    .unwrap();
    assert!(created.is_file());
    let renamed = lib.join("Renamed.cs");
    host.dispatch(
        "file.rename",
        json!({ "from": created.to_string_lossy(), "to": renamed.to_string_lossy() }),
    )
    .unwrap();
    assert!(renamed.is_file() && !created.exists());
    host.dispatch("file.delete", json!({ "path": renamed.to_string_lossy() }))
        .unwrap();
    assert!(!renamed.exists());

    // A symlink inside the scope that carries the path outside is refused, and the refusal does not
    // depend on whether the target exists.
    for path in [lib.join("LinkOut/Secret.cs"), lib.join("Dangling.cs")] {
        assert_eq!(
            error_code(host.dispatch("file.read", json!({ "path": path.to_string_lossy() }))),
            "outside_workspace",
            "{}",
            path.display()
        );
    }
    // A symlink inside the root that points at an in-scope directory is admitted: the target is in
    // the scope, which is what containment is decided on.
    assert!(
        host.dispatch(
            "file.read",
            json!({ "path": app.join("LinkIn/Greeting.cs").to_string_lossy() })
        )
        .is_ok(),
        "a link to an in-scope directory is inside the scope"
    );

    // Everything outside the scope is refused, by absolute path and by shape.
    for raw in [
        outside.join("Secret.cs").to_string_lossy().to_string(),
        "../outside/Secret.cs".to_string(),
        "../outside".to_string(),
    ] {
        assert_eq!(
            error_code(host.dispatch("file.read", json!({ "path": raw }))),
            "outside_workspace",
            "{raw}"
        );
    }
    // A write outside the scope changes nothing.
    let secret = fs::read_to_string(outside.join("Secret.cs")).unwrap();
    assert_eq!(
        error_code(host.dispatch(
            "file.write",
            json!({
                "path": outside.join("Secret.cs").to_string_lossy(),
                "content": "class Pwned {}\n",
                "expected_hash": "fnv1a64:0000000000000000",
            })
        )),
        "outside_workspace"
    );
    assert_eq!(
        fs::read_to_string(outside.join("Secret.cs")).unwrap(),
        secret
    );
}

#[cfg(unix)]
#[test]
fn restore_build_and_run_work_for_a_project_outside_the_solution() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = temp.path().join("OutsideSolution");
    copy_tree(&outside_fixture(), &fixture);
    let app = fs::canonicalize(fixture.join("app")).unwrap();
    let outside_app = fs::canonicalize(fixture.join("outside-app"))
        .unwrap()
        .join("OutsideApp.csproj");

    let host = support::host();
    open(&host, &app.join("App.sln"));
    grant(&host);

    // A real restore of the solution covers the projects outside its directory, so the gate that
    // used to block every executable surface stops blocking.
    host.dispatch("restore.start", json!({})).unwrap();
    let restored = wait_for_restore(&host, Duration::from_secs(240));
    assert_eq!(restored["state"], "ok", "{}", restored["log"]);
    assert!(
        restored["projects"]
            .as_array()
            .unwrap()
            .iter()
            .any(
                |project| project["path"] == json!(outside_app.to_string_lossy())
                    && project["state"] == "restored"
            ),
        "{}",
        restored["projects"]
    );

    let evaluated = host.dispatch("evaluation.run", json!({})).unwrap();
    assert_eq!(
        evaluated["evaluation"]["state"], "ok",
        "{}",
        evaluated["evaluation"]["log"]
    );

    host.dispatch(
        "build.start",
        json!({ "project": outside_app.to_string_lossy() }),
    )
    .unwrap();
    let build = wait_for_build(&host, Duration::from_secs(240));
    assert_eq!(build["state"], "ok", "{}", build["log"]);
    assert_eq!(build["exit"], 0);
    assert!(
        build["command"]
            .as_array()
            .unwrap()
            .iter()
            .any(|argument| argument == &json!(outside_app.to_string_lossy())),
        "the build names the outside project file: {}",
        build["command"]
    );
    assert!(build["project"]["targetPath"]
        .as_str()
        .unwrap()
        .ends_with("outside-app/bin/Debug/net8.0/OutsideApp.dll"));

    let started = host
        .dispatch(
            "run.start",
            json!({ "project": outside_app.to_string_lossy() }),
        )
        .unwrap();
    assert_eq!(started["run"]["state"], "running");
    let finished = wait_for_run(&host, Duration::from_secs(120));
    assert_eq!(finished["state"], "exited", "{}", finished["log"]);
    assert_eq!(finished["exit"], 0);
    assert!(
        finished["log"]
            .as_str()
            .unwrap()
            .contains("outside app: Hello, outside!"),
        "{}",
        finished["log"]
    );
}

#[test]
fn breakpoints_accept_a_file_of_the_outside_project_and_nothing_beyond_the_scope() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = temp.path().join("OutsideSolution");
    copy_tree(&outside_fixture(), &fixture);
    let app = fs::canonicalize(fixture.join("app")).unwrap();
    let lib = fs::canonicalize(fixture.join("lib")).unwrap();
    let outside = temp.path().join("Outside.cs");
    fs::write(&outside, "class Outside {}\n").unwrap();

    let host = support::host();
    open(&host, &app.join("App.sln"));
    grant(&host);

    let source = lib.join("Greeting.cs");
    let snapshot = host
        .dispatch(
            "debug.setBreakpoints",
            json!({ "path": source.to_string_lossy(), "lines": [5] }),
        )
        .unwrap();
    let entry = snapshot["dap"]["breakpoints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|breakpoint| breakpoint["path"] == json!(source.to_string_lossy()))
        .expect("the breakpoint of the outside project is stored");
    assert_eq!(entry["line"], 5);

    host.dispatch(
        "debug.setBreakpointCondition",
        json!({
            "path": source.to_string_lossy(),
            "line": 5,
            "condition": "name != null",
        }),
    )
    .unwrap();

    for raw in [
        outside.to_string_lossy().to_string(),
        "../Outside.cs".to_string(),
    ] {
        assert_eq!(
            error_code(host.dispatch("debug.setBreakpoints", json!({ "path": raw, "lines": [1] }))),
            "outside_workspace",
            "{raw}"
        );
    }
}

#[test]
fn the_tree_and_nuget_see_the_outside_project_inside_the_scope_only() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = temp.path().join("OutsideSolution");
    copy_tree(&outside_fixture(), &fixture);
    let app = fs::canonicalize(fixture.join("app")).unwrap();
    let lib = fs::canonicalize(fixture.join("lib")).unwrap();
    let outside = temp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("Secret.cs"), "class Secret {}\n").unwrap();
    fs::write(
        outside.join("Other.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n",
    )
    .unwrap();
    let outside = fs::canonicalize(&outside).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, lib.join("LinkOut")).unwrap();

    let host = support::host();
    let opened = open(&host, &app.join("App.sln"));

    // The text outline reads the outside project and lists its sources, before any evaluation.
    let outline = opened["outline"]["projects"].as_array().unwrap();
    let lib_outline = outline
        .iter()
        .find(|project| project["name"] == "Lib")
        .unwrap();
    assert_eq!(lib_outline["kind"], "library");
    // Before an evaluation the outline already names the outside project by its canonical absolute
    // path, the same form the evaluated model uses.
    assert_eq!(
        lib_outline["path"],
        json!(lib.join("Lib.csproj").to_string_lossy())
    );
    assert_eq!(
        lib_outline["files"],
        json!([lib.join("Greeting.cs").to_string_lossy()])
    );

    grant(&host);
    let tree = host
        .dispatch("workspace.tree", json!({ "dir": lib.to_string_lossy() }))
        .unwrap();
    assert!(
        tree["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["name"] == "Greeting.cs"
                && entry["path"] == json!(lib.join("Greeting.cs").to_string_lossy())),
        "{tree}"
    );
    for raw in [
        outside.to_string_lossy().to_string(),
        "../outside".to_string(),
    ] {
        assert_eq!(
            error_code(host.dispatch("workspace.tree", json!({ "dir": raw }))),
            "outside_workspace",
            "{raw}"
        );
    }

    // Symlink metadata answers for an in-scope path and omits an out-of-scope one.
    let links = host
        .dispatch(
            "workspace.symlinks",
            json!({
                "paths": [
                    lib.join("LinkOut").to_string_lossy(),
                    outside.join("Secret.cs").to_string_lossy(),
                ],
            }),
        )
        .unwrap();
    let entries = links["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 1, "{links}");
    assert_eq!(
        entries[0]["path"],
        json!(lib.join("LinkOut").to_string_lossy())
    );
    assert_eq!(entries[0]["symlink"], true);

    let kinds = host
        .dispatch(
            "workspace.symbolKinds",
            json!({ "paths": [lib.join("Greeting.cs").to_string_lossy()] }),
        )
        .unwrap();
    assert_eq!(kinds["entries"][0]["kind"], "class");

    // NuGet plans an update for the outside project and refuses a project outside the scope.
    let plan = host
        .dispatch(
            "nuget.planUpdate",
            json!({
                "project": lib.join("Lib.csproj").to_string_lossy(),
                "id": "Newtonsoft.Json",
                "version": "13.0.3",
                "install": true,
            }),
        )
        .unwrap();
    assert_eq!(
        plan["project"],
        json!(lib.join("Lib.csproj").to_string_lossy())
    );
    assert_eq!(
        error_code(host.dispatch(
            "nuget.planUpdate",
            json!({
                "project": outside.join("Other.csproj").to_string_lossy(),
                "id": "Newtonsoft.Json",
                "version": "13.0.3",
                "install": true,
            })
        )),
        "outside_workspace"
    );
}

#[test]
fn a_slnx_solution_evaluates_projects_outside_its_directory() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = temp.path().join("OutsideSolution");
    copy_tree(&outside_fixture(), &fixture);
    let app = fs::canonicalize(fixture.join("app")).unwrap();
    fs::write(
        app.join("App.slnx"),
        "<Solution>\n  <Project Path=\"App.csproj\" />\n  <Project Path=\"../lib/Lib.csproj\" />\n  <Project Path=\"../outside-app/OutsideApp.csproj\" />\n  <Project Path=\"../tests/Lib.Tests.csproj\" />\n</Solution>\n",
    )
    .unwrap();

    let host = support::host();
    open(&host, &app.join("App.slnx"));
    grant(&host);
    let evaluated = host.dispatch("evaluation.run", json!({})).unwrap();
    let evaluation = &evaluated["evaluation"];
    assert_eq!(evaluation["state"], "ok", "{}", evaluation["log"]);
    let projects = evaluation["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 4, "{projects:?}");
    let lib = projects
        .iter()
        .find(|project| project["name"] == "Lib")
        .unwrap();
    assert_eq!(lib["state"], "ok", "{}", lib["log"]);
    assert_eq!(
        lib["path"],
        json!(fs::canonicalize(fixture.join("lib"))
            .unwrap()
            .join("Lib.csproj")
            .to_string_lossy())
    );
    assert_eq!(
        evaluated["workspace"]["scope"].as_array().unwrap().len(),
        4,
        "the .slnx scope names the root and the three outside project directories"
    );
}

#[test]
fn an_edit_in_a_project_outside_the_solution_marks_the_model_stale() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = temp.path().join("OutsideSolution");
    copy_tree(&outside_fixture(), &fixture);
    let app = fs::canonicalize(fixture.join("app")).unwrap();
    let project_file = fs::canonicalize(fixture.join("lib"))
        .unwrap()
        .join("Lib.csproj");

    let host = support::host();
    open(&host, &app.join("App.sln"));
    grant(&host);
    host.dispatch("evaluation.run", json!({})).unwrap();

    // A real edit after the model was stored: the next poll must see it instead of masking it.
    // A project file is watched directly; editing an existing `.cs` is deliberately not a change
    // (the Compile list is a snapshot), which is why the project file is the one edited here.
    let mut text = fs::read_to_string(&project_file).unwrap();
    text.push_str("<!-- edited after the evaluation -->\n");
    fs::write(&project_file, text).unwrap();

    let state = host.dispatch("app.state", json!({})).unwrap();
    assert_eq!(state["evaluation"]["stale"], true);
    assert!(
        state["evaluation"]["staleReason"]
            .as_str()
            .unwrap()
            .contains("Lib.csproj"),
        "{}",
        state["evaluation"]["staleReason"]
    );
}

#[test]
fn the_scope_is_recomputed_when_the_solution_gains_a_project() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = temp.path().join("OutsideSolution");
    copy_tree(&outside_fixture(), &fixture);
    let app = fs::canonicalize(fixture.join("app")).unwrap();
    let lib = fs::canonicalize(fixture.join("lib")).unwrap();

    let host = support::host();
    open(&host, &app.join("App.sln"));
    grant(&host);
    let first = host.dispatch("evaluation.run", json!({})).unwrap();
    let outside_app = fs::canonicalize(fixture.join("outside-app")).unwrap();
    let tests = fs::canonicalize(fixture.join("tests")).unwrap();
    assert_eq!(
        first["workspace"]["scope"],
        json!([
            app.to_string_lossy(),
            lib.to_string_lossy(),
            outside_app.to_string_lossy(),
            tests.to_string_lossy()
        ])
    );

    let extra = fixture.join("lib2");
    fs::create_dir_all(&extra).unwrap();
    fs::write(
        extra.join("Lib2.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n",
    )
    .unwrap();
    fs::write(
        extra.join("Extra.cs"),
        "namespace Lib2;\n\npublic static class Extra { }\n",
    )
    .unwrap();
    let solution = app.join("App.sln");
    let text = fs::read_to_string(&solution).unwrap();
    let entry = "Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Lib2\", \"..\\lib2\\Lib2.csproj\", \"{11111111-1111-1111-1111-111111111111}\"\r\nEndProject\r\n";
    fs::write(
        &solution,
        text.replacen("Global", &format!("{entry}Global"), 1),
    )
    .unwrap();

    let second = host.dispatch("evaluation.run", json!({})).unwrap();
    let extra = fs::canonicalize(&extra).unwrap();
    assert_eq!(
        second["workspace"]["scope"],
        json!([
            app.to_string_lossy(),
            lib.to_string_lossy(),
            outside_app.to_string_lossy(),
            tests.to_string_lossy(),
            extra.to_string_lossy()
        ])
    );
    let added = second["evaluation"]["projects"]
        .as_array()
        .unwrap()
        .iter()
        .find(|project| project["name"] == "Lib2")
        .unwrap();
    assert_eq!(added["state"], "ok", "{}", added["log"]);
}

#[cfg(target_os = "macos")]
#[test]
fn evaluation_of_a_fresh_copy_with_spaces_creates_no_restore_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk ; & copy");

    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    let evaluated = host.dispatch("evaluation.run", json!({})).unwrap();
    let evaluation = &evaluated["evaluation"];
    assert_eq!(evaluation["state"], "ok", "{}", evaluation["log"]);
    for project in evaluation["projects"].as_array().unwrap() {
        assert_eq!(project["restored"], "not_restored", "{}", project["name"]);
        assert!(project["log"].as_str().unwrap().contains("exit 0"));
        let target = project["targetPath"].as_str().unwrap();
        assert!(target.contains("Order Desk ; & copy"), "{target}");
        assert!(target.contains("/bin/Debug/net8.0/"), "{target}");
        assert!(!project["sources"].as_array().unwrap().is_empty());
    }
    for project in [
        "src/OrderDesk.Cli",
        "src/OrderDesk.Core",
        "tests/OrderDesk.Tests",
    ] {
        assert!(
            !copy
                .join(project)
                .join("obj")
                .join("project.assets.json")
                .exists(),
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

    let host = support::host();
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
Global\n\tGlobalSection(SolutionConfigurationPlatforms) = preSolution\n\t\tDebug|Any CPU = Debug|Any CPU\n\tEndGlobalSection\n\tGlobalSection(ProjectConfigurationPlatforms) = postSolution\n\t\t{11111111-1111-1111-1111-111111111111}.Debug|Any CPU.ActiveCfg = Debug|Any CPU\n\t\t{11111111-1111-1111-1111-111111111111}.Debug|Any CPU.Build.0 = Debug|Any CPU\n\t\t{22222222-2222-2222-2222-222222222222}.Debug|Any CPU.ActiveCfg = Debug|Any CPU\n\t\t{22222222-2222-2222-2222-222222222222}.Debug|Any CPU.Build.0 = Debug|Any CPU\n\tEndGlobalSection\nEndGlobal\n",
    )
    .unwrap();

    let host = support::host();
    open(&host, &root.join("Mixed.sln"));
    grant(&host);
    let evaluated = host.dispatch("evaluation.run", json!({})).unwrap();
    let evaluation = &evaluated["evaluation"];
    assert_eq!(evaluation["state"], "partial", "{}", evaluation["log"]);
    let projects = evaluation["projects"].as_array().unwrap();
    assert_eq!(projects.len(), 2);
    let good = projects
        .iter()
        .find(|project| project["name"] == "Good")
        .unwrap();
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
    assert!(evaluation["log"]
        .as_str()
        .unwrap()
        .contains("[Broken] failed"));
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
        "Microsoft Visual Studio Solution File, Format Version 12.00\nGlobal\n\tGlobalSection(SolutionConfigurationPlatforms) = preSolution\n\t\tDebug|Any CPU = Debug|Any CPU\n\tEndGlobalSection\n\tGlobalSection(ProjectConfigurationPlatforms) = postSolution\n\t\t{11111111-1111-1111-1111-111111111111}.Debug|Any CPU.ActiveCfg = Debug|Any CPU\n\t\t{11111111-1111-1111-1111-111111111111}.Debug|Any CPU.Build.0 = Debug|Any CPU\n\t\t{22222222-2222-2222-2222-222222222222}.Debug|Any CPU.ActiveCfg = Debug|Any CPU\n\t\t{22222222-2222-2222-2222-222222222222}.Debug|Any CPU.Build.0 = Debug|Any CPU\n\tEndGlobalSection\nEndGlobal\n",
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

    let host = support::host();
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
            assert!(
                Instant::now() < deadline,
                "evaluation never reported running"
            );
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

#[test]
fn file_operations_create_rename_and_delete_inside_the_workspace() {
    let host = support::host();
    let workspace = temp_workspace(&["App.csproj"]);
    open(&host, &workspace.path().join("App.csproj"));

    let created = host
        .dispatch(
            "file.create",
            json!({ "path": "Extra.cs", "content": "class Extra {}\n" }),
        )
        .unwrap();
    assert_eq!(created["path"], "Extra.cs");
    assert_eq!(
        fs::read_to_string(workspace.path().join("Extra.cs")).unwrap(),
        "class Extra {}\n"
    );

    // A second create never overwrites the existing file.
    assert_eq!(
        error_code(host.dispatch("file.create", json!({ "path": "Extra.cs" }))),
        "already_exists"
    );
    assert_eq!(
        fs::read_to_string(workspace.path().join("Extra.cs")).unwrap(),
        "class Extra {}\n"
    );

    let renamed = host
        .dispatch(
            "file.rename",
            json!({ "from": "Extra.cs", "to": "Renamed.cs" }),
        )
        .unwrap();
    assert_eq!(renamed["from"], "Extra.cs");
    assert_eq!(renamed["to"], "Renamed.cs");
    assert!(!workspace.path().join("Extra.cs").exists());
    assert_eq!(
        fs::read_to_string(workspace.path().join("Renamed.cs")).unwrap(),
        "class Extra {}\n"
    );

    // Folders move with their contents and delete recursively.
    fs::create_dir(workspace.path().join("Folder")).unwrap();
    host.dispatch("file.create", json!({ "path": "Folder/Inside.cs" }))
        .unwrap();
    let moved = host
        .dispatch("file.rename", json!({ "from": "Folder", "to": "Moved" }))
        .unwrap();
    assert_eq!(moved["to"], "Moved");
    assert!(workspace.path().join("Moved/Inside.cs").is_file());
    let deleted_folder = host
        .dispatch("file.delete", json!({ "path": "Moved" }))
        .unwrap();
    assert_eq!(deleted_folder["directory"], true);
    assert!(!workspace.path().join("Moved").exists());

    host.dispatch("file.delete", json!({ "path": "Renamed.cs" }))
        .unwrap();
    assert!(!workspace.path().join("Renamed.cs").exists());
}

#[test]
fn file_operations_refuse_traversal_missing_parents_and_the_workspace_root() {
    let host = support::host();
    let workspace = temp_workspace(&["App.csproj"]);
    open(&host, &workspace.path().join("App.csproj"));
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("Elsewhere.cs");
    let absolute_outside = outside_file.to_string_lossy().to_string();

    for (method, params, expected) in [
        (
            "file.create",
            json!({ "path": "../Escape.cs" }),
            "outside_workspace",
        ),
        (
            "file.create",
            json!({ "path": absolute_outside.clone() }),
            "outside_workspace",
        ),
        (
            "file.create",
            json!({ "path": "Missing/Extra.cs" }),
            "not_found",
        ),
        (
            "file.rename",
            json!({ "from": "../App.cs", "to": "Moved.cs" }),
            "outside_workspace",
        ),
        (
            "file.rename",
            json!({ "from": "App.cs", "to": "Missing/Moved.cs" }),
            "not_found",
        ),
        (
            "file.delete",
            json!({ "path": "../App.cs" }),
            "outside_workspace",
        ),
        (
            "file.delete",
            json!({ "path": absolute_outside.clone() }),
            "outside_workspace",
        ),
        ("file.delete", json!({ "path": "Missing.cs" }), "not_found"),
        ("file.delete", json!({ "path": "" }), "path_not_allowed"),
    ] {
        assert_eq!(
            error_code(host.dispatch(method, params)),
            expected,
            "{method} did not refuse the path"
        );
    }

    assert!(!workspace.path().join("../Escape.cs").exists());
    assert!(!outside_file.exists());
    assert!(workspace.path().join("App.cs").is_file());

    // Renaming over an existing path is refused instead of replacing it.
    fs::write(workspace.path().join("Target.cs"), "target\n").unwrap();
    assert_eq!(
        error_code(host.dispatch(
            "file.rename",
            json!({ "from": "App.cs", "to": "Target.cs" })
        )),
        "already_exists"
    );
    assert_eq!(
        fs::read_to_string(workspace.path().join("Target.cs")).unwrap(),
        "target\n"
    );

    // A folder cannot be moved inside itself.
    fs::create_dir_all(workspace.path().join("Folder/Inner")).unwrap();
    assert_eq!(
        error_code(host.dispatch(
            "file.rename",
            json!({ "from": "Folder", "to": "Folder/Inner/Deep" })
        )),
        "path_not_allowed"
    );

    // A symlinked directory inside the workspace must not become an escape hatch; deleting the
    // link itself removes only the link.
    #[cfg(unix)]
    {
        let secret = outside.path().join("Secret.cs");
        fs::write(&secret, "secret\n").unwrap();
        std::os::unix::fs::symlink(outside.path(), workspace.path().join("Link")).unwrap();
        assert_eq!(
            error_code(host.dispatch("file.create", json!({ "path": "Link/Extra.cs" }))),
            "outside_workspace"
        );
        host.dispatch("file.delete", json!({ "path": "Link" }))
            .unwrap();
        assert!(!workspace.path().join("Link").exists());
        assert_eq!(fs::read_to_string(&secret).unwrap(), "secret\n");
    }
}

#[cfg(target_os = "macos")]
#[test]
fn file_operations_mark_the_evaluated_model_stale() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "OrderDesk file ops");
    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    let first = host.dispatch("evaluation.run", json!({})).unwrap();
    assert_eq!(first["evaluation"]["stale"], false);
    let context_before = first["contextId"].as_u64().unwrap();

    host.dispatch(
        "file.create",
        json!({ "path": "Directory.Build.props", "content": "<Project />\n" }),
    )
    .unwrap();
    let stale = snapshot(&host);
    assert_eq!(stale["evaluation"]["stale"], true);
    assert!(stale["evaluation"]["staleReason"]
        .as_str()
        .unwrap()
        .contains("was created"));
    assert!(stale["contextId"].as_u64().unwrap() > context_before);
}

#[cfg(target_os = "macos")]
#[test]
fn saving_a_project_file_or_an_external_edit_makes_the_evaluation_stale() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "OrderDesk stale");
    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    let first = host.dispatch("evaluation.run", json!({})).unwrap();
    assert_eq!(first["evaluation"]["revision"], 1);
    assert_eq!(first["evaluation"]["stale"], false);
    let context_before = first["contextId"].as_u64().unwrap();

    // A source edit does not invalidate the project model.
    let program = host
        .dispatch(
            "file.read",
            json!({ "path": "src/OrderDesk.Cli/Program.cs" }),
        )
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
    let content = read["content"].as_str().unwrap().replace(
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
fn external_source_changes_make_the_evaluation_stale_and_reappear_in_the_evaluated_sources() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "OrderDesk external sources");
    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    let first = host.dispatch("evaluation.run", json!({})).unwrap();
    assert_eq!(first["evaluation"]["stale"], false);
    let core_sources = || {
        let state = snapshot(&host);
        state["evaluation"]["projects"]
            .as_array()
            .unwrap()
            .iter()
            .find(|project| project["name"] == "OrderDesk.Core")
            .unwrap()["sources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|source| source.as_str().unwrap().to_string())
            .collect::<Vec<String>>()
    };
    assert!(core_sources().contains(&"src/OrderDesk.Core/Order.cs".to_string()));

    // Creating a .cs externally flips the directory fingerprint and the next run lists it.
    std::thread::sleep(Duration::from_millis(20));
    fs::write(
        copy.join("src/OrderDesk.Core/Extra.cs"),
        "namespace OrderDesk.Core;\npublic static class Extra {}\n",
    )
    .unwrap();
    let created = snapshot(&host);
    assert_eq!(created["evaluation"]["stale"], true);
    assert!(created["evaluation"]["staleReason"]
        .as_str()
        .unwrap()
        .contains("src/OrderDesk.Core"));
    let reevaluated = host.dispatch("evaluation.run", json!({})).unwrap();
    assert_eq!(reevaluated["evaluation"]["stale"], false);
    assert!(core_sources().contains(&"src/OrderDesk.Core/Extra.cs".to_string()));

    // Renaming and deleting are directory changes too.
    std::thread::sleep(Duration::from_millis(20));
    fs::rename(
        copy.join("src/OrderDesk.Core/Extra.cs"),
        copy.join("src/OrderDesk.Core/Renamed.cs"),
    )
    .unwrap();
    assert_eq!(snapshot(&host)["evaluation"]["stale"], true);
    host.dispatch("evaluation.run", json!({})).unwrap();
    let renamed = core_sources();
    assert!(renamed.contains(&"src/OrderDesk.Core/Renamed.cs".to_string()));
    assert!(!renamed.contains(&"src/OrderDesk.Core/Extra.cs".to_string()));

    std::thread::sleep(Duration::from_millis(20));
    fs::remove_file(copy.join("src/OrderDesk.Core/Renamed.cs")).unwrap();
    assert_eq!(snapshot(&host)["evaluation"]["stale"], true);
    host.dispatch("evaluation.run", json!({})).unwrap();
    assert!(!core_sources().contains(&"src/OrderDesk.Core/Renamed.cs".to_string()));
    assert_eq!(snapshot(&host)["evaluation"]["stale"], false);
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

    let host = support::host();
    open(&host, &root.join("Many.sln"));
    grant(&host);
    std::thread::scope(|scope| {
        let older = scope.spawn(|| host.dispatch("evaluation.run", json!({})));
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if snapshot(&host)["evaluation"]["state"] == "running" {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "evaluation never reported running"
            );
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
    let host = support::host();
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
    let host = support::host();
    let workspace = temp_workspace(&["Policy.csproj"]);
    open(&host, &workspace.path().join("Policy.csproj"));
    // Cancellation is a safety action: it is not blocked by the trust gate, only by the
    // absence of a running build.
    assert_eq!(
        error_code(host.dispatch("build.cancel", json!({}))),
        "invalid_state"
    );
    // Cancelling a run is a safety action too: not trust-gated, only "nothing is running".
    assert_eq!(
        error_code(host.dispatch("run.cancel", json!({}))),
        "invalid_state"
    );
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

    let host = support::host();
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
    assert_eq!(restore["cwd"].as_str().unwrap(), root.to_string_lossy());
    assert_eq!(restore["sdkVersion"], "8.0.408");
    let restore_root = Path::new(restore["sdkRoot"].as_str().unwrap());
    assert!(restore_root.is_absolute());
    assert!(
        restore_root.join("sdk/8.0.408").is_dir(),
        "{restore_root:?}"
    );
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
    assert!(finished["log"].as_str().unwrap().contains("Honest.csproj"));
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
    let host = support::host();
    let opened = open(&host, &copy.join("OrderDesk.sln"));
    assert_eq!(opened["restore"]["state"], "idle");
    grant(&host);
    host.dispatch("evaluation.run", json!({})).unwrap();
    let state = snapshot(&host);
    assert_eq!(state["restore"]["revision"], 0);
    assert_eq!(state["restore"]["state"], "idle");
    assert_eq!(state["processes"], json!([]));
    for project in [
        "src/OrderDesk.Cli",
        "src/OrderDesk.Core",
        "tests/OrderDesk.Tests",
    ] {
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

    let host = support::host();
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
        assert!(
            Instant::now() < deadline,
            "restore never reached NuGet: {log}"
        );
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

    let host = support::host();
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

    let host = support::host();
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
Global\n\tGlobalSection(SolutionConfigurationPlatforms) = preSolution\n\t\tDebug|Any CPU = Debug|Any CPU\n\tEndGlobalSection\n\tGlobalSection(ProjectConfigurationPlatforms) = postSolution\n\t\t{11111111-1111-1111-1111-111111111111}.Debug|Any CPU.ActiveCfg = Debug|Any CPU\n\t\t{11111111-1111-1111-1111-111111111111}.Debug|Any CPU.Build.0 = Debug|Any CPU\n\t\t{22222222-2222-2222-2222-222222222222}.Debug|Any CPU.ActiveCfg = Debug|Any CPU\n\t\t{22222222-2222-2222-2222-222222222222}.Debug|Any CPU.Build.0 = Debug|Any CPU\n\tEndGlobalSection\nEndGlobal\n",
    )
    .unwrap();

    let host = support::host();
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

    let host = support::host();
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

fn real_install(version: &str) -> sdk::Install {
    sdk::inventory()
        .installs
        .into_iter()
        .find(|install| install.version == version)
        .unwrap_or_else(|| panic!("expected SDK {version} on this machine"))
}

fn runtime_10() -> sdk::RuntimeInstall {
    let mut runtimes = sdk::inventory().runtimes;
    runtimes.retain(|runtime| {
        sdk::Version::parse(&runtime.version)
            .is_some_and(|version| version.major == 10 && version.prerelease.is_none())
    });
    runtimes
        .into_iter()
        .max_by_key(|runtime| sdk::Version::parse(&runtime.version).unwrap())
        .expect("a .NET 10 runtime exists")
}

fn tool_plan(host: &Host) -> Value {
    host.dispatch("tools.plan", json!({ "tool": "csharp-ls" }))
        .expect("the install plan is built")["tools"]["csharp-ls"]
        .clone()
}

fn wait_tool(host: &Host, timeout: Duration) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let tool = snapshot(host)["tools"]["csharp-ls"].clone();
        if matches!(
            tool["state"].as_str(),
            Some("ready" | "failed" | "cancelled")
        ) {
            return tool;
        }
        assert!(
            Instant::now() < deadline,
            "tool install did not finish: {}",
            tool["log"]
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn ensure_tool(host: &Host) {
    let tool = tool_plan(host);
    if tool["state"] == "ready" {
        return;
    }
    let plan_id = tool["plan"]["planId"].as_str().expect("plan id");
    host.dispatch(
        "tools.install",
        json!({ "planId": plan_id, "confirm": true }),
    )
    .expect("the confirmed install starts");
    let done = wait_tool(host, Duration::from_secs(180));
    assert_eq!(done["state"], "ready", "{}", done["log"]);
}

fn wait_lsp(host: &Host, timeout: Duration, states: &[&str]) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let lsp = snapshot(host)["lsp"].clone();
        if states.contains(&lsp["state"].as_str().unwrap_or("")) {
            return lsp;
        }
        assert!(
            Instant::now() < deadline,
            "LSP never reached {states:?}; state {} log {}",
            lsp["state"],
            lsp["log"]
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn wait_publication(host: &Host, timeout: Duration, predicate: impl Fn(&Value) -> bool) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let state = snapshot(host);
        if let Some(publication) = state["diagnostics"]["snapshots"]
            .as_array()
            .and_then(|snapshots| snapshots.iter().find(|item| predicate(item)))
        {
            return publication.clone();
        }
        assert!(
            Instant::now() < deadline,
            "no matching publication; lsp {} diagnostics {} log {}",
            state["lsp"]["state"],
            state["diagnostics"],
            state["lsp"]["log"]
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn tools_plan_cancel_is_inert_and_install_needs_trust_and_confirmation_isolated() {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _forced = EnvVar::remove("OPENRIDR_FORCE_HOSTROOT");
    let host = support::host();
    let workspace = temp_workspace(&["Tool.csproj"]);
    open(&host, &workspace.path().join("Tool.csproj"));

    // The plan is read-only and may be shown before trust.
    assert_eq!(
        error_code(host.dispatch("tools.install", json!({}))),
        "trust_required"
    );
    let planned = tool_plan(&host);
    assert_eq!(planned["state"], "planned");
    let plan = &planned["plan"];
    assert_eq!(plan["tool"], "csharp-ls");
    assert_eq!(plan["version"], "0.28.0");
    assert_eq!(plan["source"]["url"], "https://api.nuget.org/v3/index.json");
    assert_eq!(plan["source"]["name"], "NuGet.org");
    assert_eq!(plan["os"], std::env::consts::OS);
    assert_eq!(plan["arch"], std::env::consts::ARCH);
    let destination = plan["destination"].as_str().unwrap();
    assert!(Path::new(destination).is_absolute());
    assert!(destination.starts_with(home.path().to_string_lossy().as_ref()));
    assert!(
        destination.ends_with("tools/csharp-ls/0.28"),
        "{destination}"
    );
    let staging = plan["staging"].as_str().unwrap();
    assert!(staging.ends_with("0.28.staging"));
    let argv: Vec<&str> = plan["argv"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(argv.len(), 8, "{argv:?}");
    assert!(argv[0].ends_with("/dotnet"));
    assert_eq!(
        &argv[1..],
        [
            "tool",
            "install",
            "csharp-ls",
            "--version",
            "0.28.0",
            "--tool-path",
            staging
        ]
    );
    assert!(argv.iter().all(|arg| !arg.contains("--global")));
    assert_eq!(
        plan["cwd"].as_str().unwrap(),
        std::env::temp_dir()
            .join("openridr-neutral")
            .to_string_lossy()
    );
    assert!(plan["permissions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|permission| permission
            .as_str()
            .unwrap()
            .contains("no global tool install")));
    assert_eq!(snapshot(&host)["processes"], json!([]));

    // Cancelling a plan starts nothing and leaves no tool directory.
    let cancelled = host.dispatch("tools.cancel", json!({})).unwrap();
    assert_eq!(cancelled["tools"]["csharp-ls"]["state"], "cancelled");
    assert!(!Path::new(destination).exists());
    assert!(!Path::new(staging).exists());
    assert_eq!(snapshot(&host)["processes"], json!([]));
    assert_eq!(
        error_code(host.dispatch("tools.cancel", json!({}))),
        "invalid_state"
    );

    grant(&host);
    let planned = tool_plan(&host);
    let plan_id = planned["plan"]["planId"].as_str().unwrap().to_string();
    assert_eq!(
        error_code(host.dispatch(
            "tools.install",
            json!({ "planId": plan_id, "confirm": false })
        )),
        "confirmation_required"
    );
    assert_eq!(
        error_code(host.dispatch(
            "tools.install",
            json!({ "planId": "fnv1a64:0000000000000000", "confirm": true })
        )),
        "confirmation_required"
    );
    assert_eq!(snapshot(&host)["processes"], json!([]));

    // A material change (another app home) requires a new confirmation.
    let other = tempfile::tempdir().unwrap();
    let _other_home = EnvVar::set("OPENRIDR_HOME", other.path());
    let stale = host
        .dispatch(
            "tools.install",
            json!({ "planId": plan_id, "confirm": true }),
        )
        .expect_err("the destination changed");
    assert_eq!(stale.code, "confirmation_required");
    assert!(stale.message.contains("review and confirm the new plan"));
    assert!(!other.path().join("tools").exists());
    assert_eq!(snapshot(&host)["processes"], json!([]));
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn tools_install_is_real_verified_and_reused_isolated() {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _forced = EnvVar::remove("OPENRIDR_FORCE_HOSTROOT");
    let host = support::host();
    let workspace = temp_workspace(&["Install.csproj"]);
    open(&host, &workspace.path().join("Install.csproj"));
    grant(&host);

    let planned = tool_plan(&host);
    let plan = planned["plan"].clone();
    let plan_id = plan["planId"].as_str().unwrap().to_string();
    let started = Instant::now();
    host.dispatch(
        "tools.install",
        json!({ "planId": plan_id, "confirm": true }),
    )
    .unwrap();

    let mut seen = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(180);
    let finished = loop {
        let tool = snapshot(&host)["tools"]["csharp-ls"].clone();
        let state = tool["state"].as_str().unwrap().to_string();
        if seen.last() != Some(&state) {
            seen.push(state.clone());
        }
        if matches!(state.as_str(), "ready" | "failed" | "cancelled") {
            break tool;
        }
        assert!(Instant::now() < deadline, "install stuck: {}", tool["log"]);
        std::thread::sleep(Duration::from_millis(10));
    };
    let elapsed = started.elapsed();
    assert_eq!(finished["state"], "ready", "{}", finished["log"]);
    support::assert_install_progress(&seen);
    eprintln!(
        "real csharp-ls install: {:?}, states {seen:?}, sha256 {}",
        elapsed, finished["sha256"]
    );

    let log = finished["log"].as_str().unwrap();
    assert!(
        log.contains("Tool 'csharp-ls' (version '0.28.0') was successfully installed"),
        "{log}"
    );
    assert!(
        log.contains("Skipping NuGet package signature verification"),
        "{log}"
    );
    assert!(log.contains("--version"), "{log}");
    assert_eq!(finished["exit"], 0);
    assert_eq!(finished["verifiedVersion"], "0.28.0");
    assert!(finished["verifiedOutput"]
        .as_str()
        .unwrap()
        .contains("csharp-ls, 0.28.0"));
    let sha = finished["sha256"].as_str().unwrap();
    assert_eq!(sha.len(), 64);
    assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));

    let destination = PathBuf::from(finished["destination"].as_str().unwrap());
    let staging = PathBuf::from(finished["staging"].as_str().unwrap());
    assert!(destination.join("csharp-ls").is_file());
    // The plan was built while the launcher was absent; after the verified install it must say so.
    let after = snapshot(&host)["tools"]["csharp-ls"]["plan"]["existing"].clone();
    assert_eq!(
        after["state"], "present",
        "the plan kept the pre-install state: {after}"
    );
    assert_eq!(
        after["sha256"],
        json!(sha),
        "the plan does not describe the verified launcher"
    );
    assert!(!staging.exists());
    assert!(destination.starts_with(home.path()));
    let processes_before = snapshot(&host)["processes"].as_array().unwrap().len();

    // A valid installation is reused, never overwritten, and starts no process.
    let planned = tool_plan(&host);
    assert_eq!(planned["state"], "ready");
    assert_eq!(planned["plan"]["existing"]["state"], "present");
    let reused = host
        .dispatch(
            "tools.install",
            json!({ "planId": planned["plan"]["planId"], "confirm": true }),
        )
        .unwrap();
    assert_eq!(reused["tools"]["csharp-ls"]["state"], "ready");
    assert_eq!(reused["tools"]["csharp-ls"]["reused"], true);
    assert_eq!(reused["tools"]["csharp-ls"]["sha256"], sha);
    assert_eq!(
        snapshot(&host)["processes"].as_array().unwrap().len(),
        processes_before
    );
    assert!(destination.join("csharp-ls").is_file());

    let reopened = support::host();
    open(&reopened, &workspace.path().join("Install.csproj"));
    grant(&reopened);
    let reopened_plan = tool_plan(&reopened);
    let reopened_install = reopened
        .dispatch(
            "tools.install",
            json!({ "planId": reopened_plan["plan"]["planId"], "confirm": true }),
        )
        .unwrap();
    assert_eq!(reopened_install["tools"]["csharp-ls"]["reused"], true);
    assert_eq!(reopened_install["tools"]["csharp-ls"]["sha256"], sha);
    assert!(snapshot(&reopened)["processes"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[cfg(unix)]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn setup_starts_before_trust_without_running_project_code_isolated() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let adapter = temp.path().join("netcoredbg");
    fs::write(&adapter, "#!/bin/sh\necho 'netcoredbg 3.2.0-1092'\n").unwrap();
    fs::set_permissions(&adapter, fs::Permissions::from_mode(0o755)).unwrap();
    let installer = temp.path().join("installer");
    fs::write(&installer, "#!/bin/sh\nsleep 2\nexit 1\n").unwrap();
    fs::set_permissions(&installer, fs::Permissions::from_mode(0o755)).unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", temp.path().join("home"));
    let _muxer = EnvVar::set("OPENRIDR_TOOLS_MUXER", &installer);
    let _adapter = EnvVar::set("OPENRIDR_BUNDLED_NETCOREDBG", temp.path());
    let host = support::host();
    let workspace = temp_workspace(&["Tools.csproj"]);
    host.dispatch(
        "workspace.open",
        json!({ "path": workspace.path().join("Tools.csproj") }),
    )
    .unwrap();
    let setup = host.dispatch("setup.start", json!({})).unwrap();
    assert_eq!(setup["trust"]["granted"], false);
    assert_eq!(setup["tools"]["csharp-ls"]["state"], "downloading");
    assert_eq!(
        error_code(host.dispatch("build.start", json!({}))),
        "trust_required"
    );
    assert_eq!(
        error_code(host.dispatch("lsp.start", json!({}))),
        "trust_required"
    );
    host.dispatch("workspace.close", json!({})).unwrap();
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn setup_reuses_the_verified_installation_and_never_asks_again_isolated() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let adapter = temp.path().join("netcoredbg");
    fs::write(&adapter, "#!/bin/sh\necho 'netcoredbg 3.2.0-1092'\n").unwrap();
    fs::set_permissions(&adapter, fs::Permissions::from_mode(0o755)).unwrap();
    let installer = temp.path().join("installer");
    fs::write(
        &installer,
        r#"#!/bin/sh
echo run >> "$OPENRIDR_HOME/installer-runs"
staging="$OPENRIDR_HOME/tools/csharp-ls/0.28.staging"
mkdir -p "$staging"
printf '#!/bin/sh
printf "csharp-ls, version 0.28.0\n"
' > "$staging/csharp-ls"
chmod +x "$staging/csharp-ls"
exit 0
"#,
    )
    .unwrap();
    fs::set_permissions(&installer, fs::Permissions::from_mode(0o755)).unwrap();
    let home = temp.path().join("home");
    let _home = EnvVar::set("OPENRIDR_HOME", &home);
    let _muxer = EnvVar::set("OPENRIDR_TOOLS_MUXER", &installer);
    let _adapter = EnvVar::set("OPENRIDR_BUNDLED_NETCOREDBG", temp.path());
    let host = support::host();
    let workspace = temp_workspace(&["Setup.csproj"]);
    let opened = host
        .dispatch(
            "workspace.open",
            json!({ "path": workspace.path().join("Setup.csproj") }),
        )
        .unwrap();
    assert_eq!(opened["setup"]["state"], "idle");
    host.dispatch("setup.start", json!({})).unwrap();

    let mut ready = false;
    for _ in 0..400 {
        let state = host.dispatch("workspace.state", json!({})).unwrap();
        if state["setup"]["state"] == "ready" {
            assert_eq!(state["tools"]["csharp-ls"]["state"], "ready");
            assert_eq!(state["debug"]["adapterState"], "ready");
            assert_eq!(state["trust"]["granted"], false);
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(ready, "setup did not reach ready");
    let runs = fs::read_to_string(home.join("installer-runs")).unwrap();
    assert_eq!(runs.lines().count(), 1);

    // Second opening: the launcher and integrity record are reused, with no installer run.
    host.dispatch("workspace.close", json!({})).unwrap();
    host.dispatch(
        "workspace.open",
        json!({ "path": workspace.path().join("Setup.csproj") }),
    )
    .unwrap();
    let again = host.dispatch("setup.start", json!({})).unwrap();
    assert_eq!(again["setup"]["state"], "ready");
    assert_eq!(again["tools"]["csharp-ls"]["reused"], true);
    assert_eq!(again["tools"]["csharp-ls"]["state"], "ready");
    let runs = fs::read_to_string(home.join("installer-runs")).unwrap();
    assert_eq!(runs.lines().count(), 1, "the installer must not run again");
}

#[cfg(unix)]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn setup_reports_a_missing_bundled_adapter_and_keeps_trust_closed_isolated() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let empty = temp.path().join("no-adapter");
    fs::create_dir_all(&empty).unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", temp.path().join("home"));
    let _adapter = EnvVar::set("OPENRIDR_BUNDLED_NETCOREDBG", &empty);
    let installer = temp.path().join("installer");
    fs::write(&installer, "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(&installer, fs::Permissions::from_mode(0o755)).unwrap();
    let _muxer = EnvVar::set("OPENRIDR_TOOLS_MUXER", &installer);
    let host = support::host();
    let workspace = temp_workspace(&["Setup.csproj"]);
    host.dispatch(
        "workspace.open",
        json!({ "path": workspace.path().join("Setup.csproj") }),
    )
    .unwrap();
    host.dispatch("setup.start", json!({})).unwrap();
    let mut failed = None;
    for _ in 0..200 {
        let state = host.dispatch("workspace.state", json!({})).unwrap();
        if state["setup"]["state"] == "failed" {
            failed = Some(state);
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let failed = failed.expect("setup did not fail");
    assert!(failed["setup"]["error"]
        .as_str()
        .unwrap()
        .contains("debug adapter"));
    assert_eq!(failed["trust"]["granted"], false);
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn failed_reinstall_keeps_the_previous_verified_tool_ready_isolated() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let adapter = temp.path().join("netcoredbg");
    fs::write(&adapter, "#!/bin/sh\necho 'netcoredbg 3.2.0-1092'\n").unwrap();
    fs::set_permissions(&adapter, fs::Permissions::from_mode(0o755)).unwrap();
    let installer = temp.path().join("installer");
    fs::write(
        &installer,
        r#"#!/bin/sh
echo run >> "$OPENRIDR_HOME/installer-runs"
if [ -f "$OPENRIDR_HOME/already-installed" ]; then
  echo "deliberate reinstall failure" >&2
  exit 1
fi
staging="$OPENRIDR_HOME/tools/csharp-ls/0.28.staging"
mkdir -p "$staging"
printf '#!/bin/sh\nprintf "csharp-ls, version 0.28.0\n"\n' > "$staging/csharp-ls"
chmod +x "$staging/csharp-ls"
touch "$OPENRIDR_HOME/already-installed"
exit 0
"#,
    )
    .unwrap();
    fs::set_permissions(&installer, fs::Permissions::from_mode(0o755)).unwrap();
    let home = temp.path().join("home");
    let _home = EnvVar::set("OPENRIDR_HOME", &home);
    let _muxer = EnvVar::set("OPENRIDR_TOOLS_MUXER", &installer);
    let _adapter = EnvVar::set("OPENRIDR_BUNDLED_NETCOREDBG", temp.path());
    let host = support::host();
    let workspace = temp_workspace(&["Reinstall.csproj"]);
    host.dispatch(
        "workspace.open",
        json!({ "path": workspace.path().join("Reinstall.csproj") }),
    )
    .unwrap();
    host.dispatch("setup.start", json!({})).unwrap();
    let mut ready = None;
    for _ in 0..400 {
        let state = host.dispatch("workspace.state", json!({})).unwrap();
        if state["setup"]["state"] == "ready" {
            ready = Some(state);
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let ready = ready.expect("first setup did not reach ready");
    let launcher = home.join("tools/csharp-ls/0.28/csharp-ls");
    let before = fs::read_to_string(&launcher).unwrap();
    let sha = ready["tools"]["csharp-ls"]["sha256"].clone();

    let started = host.dispatch("setup.reinstall", json!({})).unwrap();
    assert_eq!(started["setup"]["state"], "running");
    let mut failed = false;
    for _ in 0..400 {
        let state = host.dispatch("workspace.state", json!({})).unwrap();
        let tool = &state["tools"]["csharp-ls"];
        if tool["note"]
            .as_str()
            .is_some_and(|note| note.contains("previous verified installation was kept"))
        {
            assert_eq!(tool["state"], "ready");
            assert_eq!(state["setup"]["state"], "ready");
            assert_eq!(fs::read_to_string(&launcher).unwrap(), before);
            assert_eq!(tool["sha256"], sha);
            failed = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(
        failed,
        "reinstall did not report the kept previous installation"
    );
    let runs = fs::read_to_string(home.join("installer-runs")).unwrap();
    assert_eq!(
        runs.lines().count(),
        2,
        "the reinstall attempted exactly once"
    );
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn a_reinstall_survives_a_workspace_switch_and_never_loses_both_copies_isolated() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let adapter = temp.path().join("netcoredbg");
    fs::write(&adapter, "#!/bin/sh\necho 'netcoredbg 3.2.0-1092'\n").unwrap();
    fs::set_permissions(&adapter, fs::Permissions::from_mode(0o755)).unwrap();
    let installer = temp.path().join("installer");
    fs::write(
        &installer,
        r#"#!/bin/sh
staging="$OPENRIDR_HOME/tools/csharp-ls/0.28.staging"
mkdir -p "$staging"
printf '#!/bin/sh\nprintf "csharp-ls, version 0.28.0\n"\n' > "$staging/csharp-ls"
chmod +x "$staging/csharp-ls"
exit 0
"#,
    )
    .unwrap();
    fs::set_permissions(&installer, fs::Permissions::from_mode(0o755)).unwrap();
    let home = temp.path().join("home");
    let _home = EnvVar::set("OPENRIDR_HOME", &home);
    let _muxer = EnvVar::set("OPENRIDR_TOOLS_MUXER", &installer);
    let _adapter = EnvVar::set("OPENRIDR_BUNDLED_NETCOREDBG", temp.path());
    let host = support::host();
    let workspace = temp_workspace(&["Tools.csproj"]);
    let other = temp_workspace(&["Other.csproj"]);
    let project = workspace.path().join("Tools.csproj");
    host.dispatch("workspace.open", json!({ "path": project }))
        .unwrap();
    host.dispatch("setup.start", json!({})).unwrap();
    let mut ready = false;
    for _ in 0..400 {
        if host.dispatch("workspace.state", json!({})).unwrap()["setup"]["state"] == "ready" {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(ready, "initial setup did not become ready");
    let launcher = home.join("tools/csharp-ls/0.28/csharp-ls");
    assert!(launcher.is_file());

    host.dispatch("setup.reinstall", json!({})).unwrap();
    let mut swapped = false;
    for _ in 0..400 {
        let state = host.dispatch("workspace.state", json!({})).unwrap();
        if state["tools"]["csharp-ls"]["state"] == "verifying" {
            swapped = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        swapped,
        "reinstall never reached the post-swap verification"
    );
    // Tools are global: switching workspaces while verifying must not seal or delete either copy.
    host.dispatch(
        "workspace.open",
        json!({ "path": other.path().join("Other.csproj") }),
    )
    .unwrap();
    let mut ready_after_switch = false;
    for _ in 0..400 {
        let state = host.dispatch("workspace.state", json!({})).unwrap();
        if state["tools"]["csharp-ls"]["state"] == "ready" {
            assert!(
                launcher.is_file(),
                "neither the new nor the previous launcher survived"
            );
            ready_after_switch = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(
        ready_after_switch,
        "reinstall did not finish after the workspace switch"
    );
    // A subsequent setup attempt reuses the good installation instead of deleting it.
    host.dispatch("setup.start", json!({})).unwrap();
    assert_eq!(
        host.dispatch("workspace.state", json!({})).unwrap()["tools"]["csharp-ls"]["state"],
        "ready"
    );
    assert!(launcher.is_file());
}

#[cfg(unix)]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn tools_download_does_not_block_host_calls_isolated() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let installer = home.path().join("installer");
    fs::write(&installer, "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(&installer, fs::Permissions::from_mode(0o755)).unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _muxer = EnvVar::set("OPENRIDR_TOOLS_MUXER", &installer);
    let _delay = EnvVar::set("OPENRIDR_TOOLS_PROBE_DOWNLOAD_DELAY_MS", "1500");
    let host = support::host();
    let workspace = temp_workspace(&["Tools.csproj"]);
    open(&host, &workspace.path().join("Tools.csproj"));
    grant(&host);
    let plan = tool_plan(&host);
    let start = Instant::now();
    host.dispatch(
        "tools.install",
        json!({"planId": plan["plan"]["planId"], "confirm": true}),
    )
    .unwrap();
    assert_eq!(
        snapshot(&host)["tools"]["csharp-ls"]["state"],
        "downloading"
    );
    assert!(
        start.elapsed() < Duration::from_millis(700),
        "download blocked host dispatch"
    );
    host.dispatch("workspace.close", json!({})).unwrap();
    // Tools are global: closing the workspace must not seal this install. The delayed worker
    // proceeds, runs the (failing) controlled installer, and lands in failed, not cancelled.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let state = snapshot(&host)["tools"]["csharp-ls"]["state"].clone();
        if state == "failed" {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "global install did not finish: {state}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
    let processes = snapshot(&host)["processes"].as_array().unwrap().clone();
    assert!(
        processes.iter().any(|process| process["state"] == "exited"),
        "the workspace close sealed the global install instead of letting it finish"
    );
}

/// The verified download owns the progress number with or without `Content-Length`: the bytes read
/// from the socket are a measurement, the copies `dotnet tool install` writes to its cache are not.
/// Drives the real installer (`tools.install` -> worker -> download -> install) against a local
/// server that never sends `Content-Length`, which is the case the disk fallback used to take over.
#[cfg(all(unix, debug_assertions))]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn a_download_without_content_length_keeps_the_measured_bytes_isolated() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let installer = home.path().join("slow-installer");
    // A package the size of nothing that matters, and an installer that copies 400 KiB into the
    // NuGet cache before failing: the old progress summed those copies onto the download.
    fs::write(
        &installer,
        r#"#!/bin/sh
tmp="$OPENRIDR_HOME/tools/csharp-ls/tmp/packages/csharp-ls/0.28.0"
mkdir -p "$tmp"
dd if=/dev/zero of="$tmp/csharp-ls.0.28.0.nupkg" bs=1024 count=400 2>/dev/null
sleep 2
exit 1
"#,
    )
    .unwrap();
    fs::set_permissions(&installer, fs::Permissions::from_mode(0o755)).unwrap();

    // 100 KiB of deterministic bytes; the digest is the pinned-package override for this attempt.
    let payload: Vec<u8> = (0..102_400u32).map(|index| (index % 251) as u8).collect();
    let payload_len = payload.len() as u64;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let body = payload.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut request = [0u8; 2048];
            let _ = stream.read(&mut request);
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
            );
            let _ = stream.write_all(&body);
            let _ = stream.flush();
        }
    });

    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _muxer = EnvVar::set("OPENRIDR_TOOLS_MUXER", &installer);
    let _url = EnvVar::set(
        "OPENRIDR_TOOLS_PACKAGE_URL",
        format!("http://{address}/csharp-ls.0.28.0.nupkg"),
    );
    let _sha = EnvVar::set(
        "OPENRIDR_TOOLS_PACKAGE_SHA512",
        "2acda2d1386c8cd9ef01c797cfd154b073e7ea26e4c5741e9f2aee089dc8106cb887526d5bbb04920c2b742b2dab945e2a8db4cd31f58ac945aec9df89b1f18d",
    );
    let host = support::host();
    let workspace = temp_workspace(&["Tools.csproj"]);
    open(&host, &workspace.path().join("Tools.csproj"));
    grant(&host);
    let plan = tool_plan(&host);
    host.dispatch(
        "tools.install",
        json!({ "planId": plan["plan"]["planId"], "confirm": true }),
    )
    .unwrap();

    // Every observation during the attempt is the measured download: a partial read is legitimate,
    // so samples only have to be monotonic and never reach the installer's copies on disk.
    let mut samples = 0;
    let mut previous = 0;
    let mut last = 0;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let tool = snapshot(&host)["tools"]["csharp-ls"].clone();
        if let Some(progress) = tool["progress"].as_object() {
            samples += 1;
            assert_eq!(progress["total"], Value::Null, "no total was sent: {tool}");
            let downloaded = progress["downloaded"]
                .as_u64()
                .expect("progress is a byte count");
            assert!(
                downloaded >= previous,
                "progress went backwards ({previous} -> {downloaded}): {tool}"
            );
            assert!(
                downloaded <= payload_len,
                "progress counted the installer's copies on disk ({downloaded} > {payload_len}): {tool}"
            );
            previous = downloaded;
            last = downloaded;
        }
        if tool["state"] == "failed" {
            break;
        }
        assert!(Instant::now() < deadline, "install stuck: {}", tool["log"]);
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(
        samples > 0,
        "no progress was observed for the measured download"
    );
    assert_eq!(
        last, payload_len,
        "the download never reported the whole package"
    );
}

#[cfg(unix)]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn tools_failed_install_is_not_ready_and_retry_starts_from_clean_staging_isolated() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().unwrap();
    let probe = tempfile::tempdir().unwrap();
    let installer = probe.path().join("probe-installer");
    // First run writes partial bytes, then fails like an unreachable NuGet source; the retry
    // succeeds with a fake launcher so the whole retry path reaches verified/ready.
    fs::write(
        &installer,
        r#"#!/bin/sh
staging="$OPENRIDR_HOME/tools/csharp-ls/0.28.staging"
mkdir -p "$staging"
dd if=/dev/zero of="$staging/partial.bin" bs=1024 count=512 2>/dev/null
sleep 2
if [ -f "$OPENRIDR_HOME/probe-tools-failed" ]; then
  printf '#!/bin/sh\necho "csharp-ls, version 0.28.0 (probe)"\n' > "$staging/csharp-ls"
  chmod +x "$staging/csharp-ls"
  exit 0
fi
touch "$OPENRIDR_HOME/probe-tools-failed"
echo "error: Unable to load the service index for source https://api.nuget.org/v3/index.json." >&2
exit 1
"#,
    )
    .unwrap();
    fs::set_permissions(&installer, fs::Permissions::from_mode(0o755)).unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _muxer = EnvVar::set("OPENRIDR_TOOLS_MUXER", &installer);
    let _forced = EnvVar::remove("OPENRIDR_FORCE_HOSTROOT");
    let host = support::host();
    let workspace = temp_workspace(&["Tools.csproj"]);
    open(&host, &workspace.path().join("Tools.csproj"));
    grant(&host);

    let planned = tool_plan(&host);
    assert!(planned["plan"]["argv"][0]
        .as_str()
        .unwrap()
        .ends_with("probe-installer"));
    let plan_id = planned["plan"]["planId"].as_str().unwrap().to_string();
    host.dispatch(
        "tools.install",
        json!({ "planId": plan_id, "confirm": true }),
    )
    .unwrap();

    // Observed bytes, then a failure that never reports ready.
    let mut saw_progress = false;
    let deadline = Instant::now() + Duration::from_secs(20);
    let failed = loop {
        let tool = snapshot(&host)["tools"]["csharp-ls"].clone();
        if tool["progress"]["downloaded"].as_u64().unwrap_or(0) > 0 {
            saw_progress = true;
        }
        if tool["state"] == "failed" {
            break tool;
        }
        assert!(Instant::now() < deadline, "install stuck: {}", tool["log"]);
        std::thread::sleep(Duration::from_millis(20));
    };
    assert!(saw_progress, "no progress was observed: {failed}");
    assert_eq!(failed["state"], "failed");
    assert_eq!(failed["sha256"], Value::Null);
    assert_eq!(failed["verifiedVersion"], Value::Null);
    assert!(!Path::new(failed["destination"].as_str().unwrap()).exists());
    let error = failed["error"].as_str().unwrap();
    assert!(error.contains("Try again"), "{error}");
    assert!(error.contains("network"), "{error}");
    let staging = PathBuf::from(failed["staging"].as_str().unwrap());
    assert!(staging.join("partial.bin").is_file());
    fs::write(staging.join("leftover"), "x").unwrap();

    // The retry removes the partial staging before starting again, then reaches a verified ready.
    host.dispatch("tools.retry", json!({})).unwrap();
    let done = wait_tool(&host, Duration::from_secs(30));
    assert!(
        !staging.join("leftover").exists(),
        "the retry reused the partial staging directory"
    );
    assert_eq!(done["state"], "ready", "{}", done["log"]);
    assert_eq!(done["reused"], false);
    assert_eq!(done["verifiedVersion"], "0.28.0");
    assert_eq!(done["sha256"].as_str().unwrap().len(), 64);
    assert!(!staging.exists());

    // A verified installation is reused after confirmation and starts no process.
    let replanned = tool_plan(&host);
    assert_eq!(replanned["state"], "ready");
    let processes = snapshot(&host)["processes"].as_array().unwrap().len();
    let reused = host
        .dispatch(
            "tools.install",
            json!({ "planId": replanned["plan"]["planId"], "confirm": true }),
        )
        .unwrap();
    assert_eq!(reused["tools"]["csharp-ls"]["reused"], true);
    assert_eq!(
        snapshot(&host)["processes"].as_array().unwrap().len(),
        processes
    );
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn hostroot_composes_runtime10_with_project_sdk_outside_the_repo_and_is_idempotent_isolated() {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _forced = EnvVar::remove("OPENRIDR_FORCE_HOSTROOT");

    let runtime = runtime_10();
    let project = real_install("8.0.408");
    let first = hostroot::ensure(&runtime.root, &project).expect("the composite root is built");
    assert!(first.rebuilt);
    assert!(!first.forced);
    assert!(Path::new(&first.path).starts_with(home.path()));
    assert!(Path::new(&first.path).join("dotnet").is_file());
    assert!(first.check.contains("8.0.408"));
    assert!(first.check.contains("10.0."), "{}", first.check);
    assert!(first.check.contains(
        Path::new(&first.path)
            .join("dotnet")
            .to_string_lossy()
            .as_ref()
    ));

    let muxer = Path::new(&first.path).join("dotnet");
    let output = std::process::Command::new(&muxer)
        .arg("--list-sdks")
        .env("DOTNET_ROOT", &first.path)
        .env_remove("DOTNET_MULTILEVEL_LOOKUP")
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    let sdks: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(
        sdks.len(),
        1,
        "the project SDK must be the only MSBuild SDK in sdk/: {text}"
    );
    assert!(
        sdks[0].starts_with("8.0.408"),
        "the project SDK must be the only MSBuild SDK: {text}"
    );

    let runtimes = std::process::Command::new(&muxer)
        .arg("--list-runtimes")
        .env("DOTNET_ROOT", &first.path)
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(runtimes.status.success());
    let runtimes = String::from_utf8_lossy(&runtimes.stdout).to_string();
    assert!(runtimes.contains("Microsoft.NETCore.App 10."), "{runtimes}");

    let version = std::process::Command::new(&muxer)
        .arg("--version")
        .env("DOTNET_ROOT", &first.path)
        .current_dir(fixture())
        .output()
        .unwrap();
    assert!(version.status.success());
    assert_eq!(String::from_utf8_lossy(&version.stdout).trim(), "8.0.408");

    let second = hostroot::ensure(&runtime.root, &project).expect("the composite root is reused");
    assert!(!second.rebuilt);
    assert_eq!(first.path, second.path);
    assert_eq!(second.check, first.check);
    assert!(!Path::new(&first.path).starts_with(fixture()));
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn lsp_start_requires_tool_sdk_and_current_restore_isolated() {
    let _forced = EnvVar::remove("OPENRIDR_FORCE_HOSTROOT");

    // No verified tool: the host says tool_missing and points to the plan.
    let no_tool_home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", no_tool_home.path());
    let host = support::host();
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Lsp no tool");
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    let error = host.dispatch("lsp.start", json!({})).expect_err("no tool");
    assert_eq!(error.code, "tool_missing");
    assert_eq!(error.details.unwrap()["action"]["method"], "tools.plan");
    drop(_home);

    // With the real tool: unresolved project SDK and missing restore are distinct.
    let host = support::host();
    let missing =
        fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/MissingSdk"))
            .unwrap();
    open(&host, &missing.join("MissingSdk.csproj"));
    grant(&host);
    ensure_tool(&host);
    let error = host
        .dispatch("lsp.start", json!({}))
        .expect_err("missing project SDK");
    assert_eq!(error.code, "sdk_missing");
    assert_eq!(error.details.unwrap()["action"]["method"], "sdk.redetect");

    let fresh_temp = tempfile::tempdir().unwrap();
    let fresh = fixture_copy(fresh_temp.path(), "Lsp restore required");
    open(&host, &fresh.join("OrderDesk.sln"));
    grant(&host);
    let error = host
        .dispatch("lsp.start", json!({}))
        .expect_err("restore is missing");
    assert_eq!(error.code, "restore_required");
    let details = error.details.unwrap();
    assert_eq!(details["action"]["method"], "restore.start");
    assert!(!fresh
        .join("src/OrderDesk.Cli/obj/project.assets.json")
        .exists());
    assert_eq!(snapshot(&host)["lsp"]["state"], "unavailable");
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn lsp_loads_the_real_solution_republishes_diagnostics_and_stops_on_revoke_isolated() {
    let _forced = EnvVar::remove("OPENRIDR_FORCE_HOSTROOT");
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk lsp");
    let program = copy.join("src/OrderDesk.Cli/Program.cs");
    let clean = fs::read_to_string(&program).unwrap();
    let broken = format!("{clean}\nvar report = new OrderReport();\n");
    fs::write(&program, &broken).unwrap();

    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    ensure_tool(&host);
    host.dispatch("evaluation.run", json!({})).unwrap();
    host.dispatch("restore.start", json!({})).unwrap();
    let restore = wait_for_restore(&host, Duration::from_secs(120));
    assert_eq!(restore["state"], "ok", "{}", restore["log"]);
    let evaluated = host.dispatch("evaluation.run", json!({})).unwrap();
    assert_eq!(evaluated["evaluation"]["state"], "ok");

    let started = host.dispatch("lsp.start", json!({})).unwrap();
    assert_eq!(started["lsp"]["state"], "starting");
    let ready = wait_lsp(
        &host,
        Duration::from_secs(60),
        &["ready", "partial", "awaiting_diagnostics", "failed"],
    );
    assert_eq!(ready["state"], "ready", "{}", ready["log"]);
    assert_eq!(ready["coverage"], "full");
    assert_eq!(ready["capabilities"]["completion"], true);
    assert_eq!(ready["capabilities"]["definition"], true);
    assert_eq!(ready["capabilities"]["rename"], true);
    assert_eq!(ready["capabilities"]["codeActions"], true);
    assert_eq!(ready["serverInfo"]["name"], "csharp-ls");
    assert_eq!(ready["logLevel"], "info");
    let host_root = ready["hostRoot"].as_str().unwrap().to_string();
    assert!(host_root.contains("hostroot"));
    assert!(ready["hostRootCheck"].as_str().unwrap().contains("8.0.408"));

    let opened = host
        .dispatch(
            "lsp.didOpen",
            json!({ "path": "src/OrderDesk.Cli/Program.cs" }),
        )
        .unwrap();
    assert_eq!(opened["version"], 1);
    let publication = wait_publication(&host, Duration::from_secs(30), |item| {
        item["clientRevision"] == 1
    });
    assert_eq!(publication["serverVersion"], Value::Null);
    let items = publication["items"].as_array().unwrap();
    assert_eq!(items.len(), 1, "{}", publication);
    assert_eq!(items[0]["code"], "CS0246");
    assert_eq!(items[0]["severity"], 1);
    assert_eq!(items[0]["severityLabel"], "error");
    assert_eq!(items[0]["source"], "lsp");
    assert_eq!(items[0]["category"], "unknown");
    assert!(items[0]["message"]
        .as_str()
        .unwrap()
        .contains("OrderReport"));
    assert!(items[0]["range"]["start"]["line"].as_u64().unwrap() >= 4);
    let diagnostics = snapshot(&host)["diagnostics"].clone();
    assert_eq!(diagnostics["session"], 1);
    assert_eq!(diagnostics["origin"]["label"], "LSP · csharp-ls");
    assert_eq!(diagnostics["analyzerCoverage"], "not_verified");
    assert_eq!(diagnostics["snapshots"][0]["state"], "current");
    assert_eq!(diagnostics["snapshots"][0]["clientRevision"], 1);
    assert_eq!(diagnostics["snapshots"][0]["workspaceId"], 1);
    assert_eq!(
        diagnostics["snapshots"][0]["items"][0]["position"]["line"],
        items[0]["range"]["start"]["line"]
    );
    assert_eq!(diagnostics["counts"]["error"], 1);

    let changed = host
        .dispatch(
            "lsp.didChange",
            json!({ "path": "src/OrderDesk.Cli/Program.cs", "text": clean }),
        )
        .unwrap();
    assert_eq!(changed["version"], 2);
    let fixed = wait_publication(&host, Duration::from_secs(30), |item| {
        item["clientRevision"] == 2 && item["items"].as_array().is_some_and(Vec::is_empty)
    });
    assert_eq!(fixed["items"], json!([]));
    assert_eq!(snapshot(&host)["lsp"]["state"], "ready");
    let current = snapshot(&host)["diagnostics"].clone();
    assert_eq!(current["snapshots"][0]["state"], "current");
    assert_eq!(current["snapshots"][0]["items"], json!([]));
    assert_eq!(current["currentItems"], 0);

    // A newer buffer than the last publication is awaiting, not resolved.
    host.dispatch(
        "lsp.didChange",
        json!({ "path": "src/OrderDesk.Cli/Program.cs", "text": broken }),
    )
    .unwrap();
    let republished_broken = wait_publication(&host, Duration::from_secs(30), |item| {
        item["clientRevision"] == 3
            && item["items"]
                .as_array()
                .is_some_and(|items| items.len() == 1)
    });
    assert_eq!(republished_broken["items"][0]["code"], "CS0246");

    let restarted = host
        .dispatch("lsp.restart", json!({ "verbose": true }))
        .unwrap();
    assert_eq!(restarted["lsp"]["session"], 2);
    assert_eq!(restarted["lsp"]["logLevel"], "debug");
    let ready = wait_lsp(
        &host,
        Duration::from_secs(60),
        &["ready", "partial", "awaiting_diagnostics", "failed"],
    );
    assert_eq!(ready["state"], "ready", "{}", ready["log"]);
    assert_eq!(ready["publicationCount"], 0, "restart starts a new session");
    let after_restart = snapshot(&host)["diagnostics"].clone();
    assert_eq!(after_restart["session"], 2);
    let stale = after_restart["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["session"] == 1)
        .expect("the previous session stays visible");
    assert_eq!(stale["state"], "stale");
    assert_eq!(stale["items"][0]["code"], "CS0246");
    assert_eq!(stale["items"][0]["stale"], true);
    assert_eq!(after_restart["currentItems"], 0);
    host.dispatch(
        "lsp.didOpen",
        json!({ "path": "src/OrderDesk.Cli/Program.cs", "text": clean }),
    )
    .unwrap();
    let republished = wait_publication(&host, Duration::from_secs(30), |item| {
        item["clientRevision"] == 1
    });
    assert_eq!(republished["items"], json!([]));
    let current = snapshot(&host)["diagnostics"].clone();
    let current_session = current["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["session"] == 2)
        .unwrap();
    assert_eq!(current_session["state"], "current");
    assert_eq!(current_session["items"], json!([]));

    // Developer boundary: a publication without a range must not invent a position.
    let probed = host
        .dispatch(
            "diagnostics.probe",
            json!({ "path": "src/OrderDesk.Cli/Program.cs" }),
        )
        .unwrap();
    let probe_item = &probed["diagnostics"]["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["items"][0]["code"] == "PROBE0001")
        .expect("the probe publication is stored")["items"][0];
    assert_eq!(probe_item["position"], Value::Null);
    assert_eq!(probe_item["range"], Value::Null);

    let lsp_process = snapshot(&host)["lsp"]["processId"].as_u64().unwrap();
    let revoked = host.dispatch("trust.revoke", json!({})).unwrap();
    assert_eq!(revoked["lsp"]["state"], "stopped");
    let revoked_diagnostics = &revoked["diagnostics"];
    assert_eq!(
        revoked_diagnostics["analysisUnavailable"],
        "Trust was revoked; analysis is unavailable and these diagnostics are not current. Editing stays available."
    );
    assert!(revoked_diagnostics["snapshots"]
        .as_array()
        .unwrap()
        .iter()
        .all(|item| item["state"] == "stale"));
    assert_eq!(revoked_diagnostics["currentItems"], 0);
    let process = revoked["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|process| process["id"].as_u64() == Some(lsp_process))
        .unwrap();
    assert_eq!(process["state"], "exited");
    assert_eq!(revoked["lsp"]["exit"], process["exit"]);
    assert_eq!(revoked["lsp"]["exit"], 0);
    assert!(revoked["lsp"]["finishedAt"].as_u64().is_some());

    // Editing stays available after analysis stops.
    let read = host
        .dispatch(
            "file.read",
            json!({ "path": "src/OrderDesk.Cli/Program.cs" }),
        )
        .unwrap();
    host.dispatch(
        "file.write",
        json!({
            "path": "src/OrderDesk.Cli/Program.cs",
            "content": read["content"],
            "expected_hash": read["hash"],
        }),
    )
    .unwrap();
    assert_eq!(
        error_code(host.dispatch("lsp.start", json!({}))),
        "trust_required"
    );
}

/// Stub launcher: answers `--version` like csharp-ls 0.28.0, then rejects `initialize` with an SDK message on stderr and idles.
#[cfg(unix)]
const REJECTING_LAUNCHER: &str = r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "csharp-ls, version 0.28.0 (stub)"; exit 0; fi
len=0
while IFS= read -r line; do
  line=$(printf %s "$line" | tr -d '\r')
  [ -z "$line" ] && break
  case "$line" in Content-Length:*) len=${line#Content-Length: };; esac
done
body=$(head -c "$len")
id=$(printf %s "$body" | sed -n 's/.*"id":\([0-9]*\).*/\1/p')
echo "A compatible .NET SDK was not found. Requested SDK version: 8.0.408 global.json file: stub" >&2
msg='{"jsonrpc":"2.0","id":'$id',"error":{"code":-32603,"message":"initialize failed"}}'
printf 'Content-Length: %d\r\n\r\n%s' ${#msg} "$msg"
exec sleep 60
"#;

#[cfg(unix)]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn initialize_failure_surfaces_the_sdk_message_and_kills_the_process_isolated() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    let _home = EnvVar::set("OPENRIDR_HOME", &home);
    let _forced = EnvVar::remove("OPENRIDR_FORCE_HOSTROOT");
    // Probe seam: the stub is executable proof for the LSP path; no package is downloaded.
    let noop = temp.path().join("noop-installer");
    fs::write(&noop, "#!/bin/sh\nexit 1\n").unwrap();
    fs::set_permissions(&noop, fs::Permissions::from_mode(0o755)).unwrap();
    let _muxer = EnvVar::set("OPENRIDR_TOOLS_MUXER", &noop);
    // The stub stands in for csharp-ls, so the contract does not depend on the real server or the machine's SDKs.
    let destination = home.join("tools/csharp-ls/0.28");
    fs::create_dir_all(&destination).unwrap();
    let launcher = destination.join("csharp-ls");
    fs::write(&launcher, REJECTING_LAUNCHER).unwrap();
    fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).unwrap();

    let copy = fixture_copy(temp.path(), "Order Desk lsp failure");
    let host_api = support::host();
    open(&host_api, &copy.join("OrderDesk.sln"));
    grant(&host_api);
    let tool = tool_plan(&host_api);
    assert_eq!(tool["plan"]["existing"]["state"], "present", "{tool}");
    let reused = host_api
        .dispatch(
            "tools.install",
            json!({ "planId": tool["plan"]["planId"], "confirm": true }),
        )
        .unwrap();
    assert_eq!(reused["tools"]["csharp-ls"]["state"], "ready");
    host_api.dispatch("restore.start", json!({})).unwrap();
    let restore = wait_for_restore(&host_api, Duration::from_secs(120));
    assert_eq!(restore["state"], "ok", "{}", restore["log"]);
    host_api.dispatch("evaluation.run", json!({})).unwrap();

    host_api.dispatch("lsp.start", json!({})).unwrap();
    let failed = wait_lsp(&host_api, Duration::from_secs(30), &["failed"]);
    let message = failed["error"].as_str().unwrap();
    assert!(
        message.contains("A compatible .NET SDK was not found"),
        "{message}"
    );
    assert!(message.contains("8.0.408"), "{message}");
    assert!(failed["log"]
        .as_str()
        .unwrap()
        .contains("A compatible .NET SDK was not found"));
    assert_eq!(failed["publicationCount"], 0);

    let process_id = failed["processId"].as_u64().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let process = snapshot(&host_api)["processes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|process| process["id"].as_u64() == Some(process_id))
            .cloned()
            .unwrap();
        if process["state"] == "exited" {
            break;
        }
        assert!(Instant::now() < deadline, "the failed LSP process survived");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(unix)]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn cancelling_a_running_install_reports_termination_and_never_ready_isolated() {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _forced = EnvVar::remove("OPENRIDR_FORCE_HOSTROOT");
    let installer = home.path().join("slow-installer");
    fs::write(&installer, "#!/bin/sh\nsleep 30\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(&installer, fs::Permissions::from_mode(0o755)).unwrap();
    let _muxer = EnvVar::set("OPENRIDR_TOOLS_MUXER", &installer);
    let host = support::host();
    let workspace = temp_workspace(&["CancelInstall.csproj"]);
    open(&host, &workspace.path().join("CancelInstall.csproj"));
    grant(&host);

    let planned = tool_plan(&host);
    let plan_id = planned["plan"]["planId"].as_str().unwrap().to_string();
    let destination = planned["plan"]["destination"].as_str().unwrap().to_string();
    let staging = planned["plan"]["staging"].as_str().unwrap().to_string();
    host.dispatch(
        "tools.install",
        json!({ "planId": plan_id, "confirm": true }),
    )
    .unwrap();
    // The download is async: wait for the controlled installer process before cancelling it.
    let mut started = false;
    for _ in 0..200 {
        let state = host.dispatch("workspace.state", json!({})).unwrap();
        if state["tools"]["csharp-ls"]["processId"].is_number() {
            started = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(started, "the install process did not start");
    let cancelled = host.dispatch("tools.cancel", json!({})).unwrap();
    let tool = &cancelled["tools"]["csharp-ls"];
    assert_eq!(tool["state"], "cancelled");
    assert_ne!(tool["state"], "ready");
    assert_eq!(tool["reused"], false);
    assert!(
        tool["note"].as_str().unwrap().contains("not ready"),
        "{}",
        tool["note"]
    );
    assert!(!Path::new(&destination).exists() || tool["sha256"].is_null());
    let process = cancelled["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|process| process["id"] == tool["processId"])
        .expect("the install process is tracked");
    assert_eq!(process["state"], "exited");
    assert_eq!(tool["exit"], process["exit"]);
    assert!(Path::new(&staging).exists() || !Path::new(&destination).exists());
    assert!(tool["log"].as_str().unwrap().contains("cancelled"));
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn lsp_completion_and_definition_answer_from_the_real_session_and_reject_unknown_documents_isolated(
) {
    let _forced = EnvVar::remove("OPENRIDR_FORCE_HOSTROOT");
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk complete");
    let program = "using OrderDesk.Core;\n\nvar order = new Order(\"A-104\", 42m);\norder.\n";
    let path = "src/OrderDesk.Cli/Program.cs";

    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    ensure_tool(&host);
    host.dispatch("evaluation.run", json!({})).unwrap();
    host.dispatch("restore.start", json!({})).unwrap();
    let restore = wait_for_restore(&host, Duration::from_secs(120));
    assert_eq!(restore["state"], "ok", "{}", restore["log"]);
    host.dispatch("evaluation.run", json!({})).unwrap();

    let position = json!({ "path": path, "line": 3, "character": 6, "revision": 0 });
    let offline = host.dispatch("lsp.completion", position.clone()).unwrap();
    assert_eq!(offline["state"], "unavailable");
    assert_eq!(offline["documentKnown"], false);
    assert_eq!(offline["items"], json!([]));
    assert_eq!(offline["session"], 0);
    assert_eq!(offline["line"], 3);
    assert_eq!(offline["character"], 6);

    let offline_actions = host
        .dispatch(
            "lsp.codeActions",
            json!({ "path": path, "line": 2, "character": 16, "revision": 0 }),
        )
        .unwrap();
    assert_eq!(offline_actions["state"], "unavailable");
    assert_eq!(offline_actions["actions"], json!([]));
    assert_eq!(offline_actions["actionCount"], 0);

    host.dispatch("lsp.start", json!({})).unwrap();
    let starting = host.dispatch("lsp.completion", position.clone()).unwrap();
    assert_eq!(starting["state"], "unavailable");
    assert_eq!(starting["items"], json!([]));
    assert_eq!(starting["documentKnown"], false);

    let ready = wait_lsp(
        &host,
        Duration::from_secs(60),
        &["ready", "partial", "awaiting_diagnostics", "failed"],
    );
    assert_eq!(ready["state"], "ready", "{}", ready["log"]);
    let session = ready["session"].as_u64().unwrap();

    let not_open = host.dispatch("lsp.completion", position.clone()).unwrap();
    assert_eq!(not_open["state"], "unavailable");
    assert_eq!(not_open["documentKnown"], false);
    assert_eq!(not_open["session"], session);
    assert!(
        not_open["message"].as_str().unwrap().contains("not open"),
        "{}",
        not_open["message"]
    );

    host.dispatch("lsp.didOpen", json!({ "path": path, "text": program }))
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(30);
    let completion = loop {
        let answer = host.dispatch("lsp.completion", position.clone()).unwrap();
        assert_eq!(answer["state"], "ok", "{}", answer["message"]);
        if answer["itemCount"].as_u64().unwrap_or(0) > 0 {
            break answer;
        }
        assert!(
            Instant::now() < deadline,
            "completion stayed empty: {}",
            answer
        );
        std::thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(completion["session"], session);
    assert_eq!(completion["clientRevision"], 1);
    assert_eq!(completion["revision"], 0);
    assert_eq!(completion["documentKnown"], true);
    assert_eq!(completion["coverage"], "full");
    let labels: Vec<String> = completion["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["label"].as_str().unwrap().to_string())
        .collect();
    assert!(
        labels.iter().any(|label| label == "Id" || label == "Total"),
        "members of OrderDesk.Core.Order missing: {labels:?}"
    );
    assert_eq!(completion["truncated"], false);
    assert_eq!(completion["totalItems"], completion["itemCount"]);

    let definition = host
        .dispatch(
            "lsp.definition",
            json!({ "path": path, "line": 2, "character": 20 }),
        )
        .unwrap();
    assert_eq!(definition["state"], "ok", "{}", definition);
    let locations = definition["locations"].as_array().unwrap();
    assert!(!locations.is_empty(), "{}", definition);
    assert_eq!(locations[0]["path"], "src/OrderDesk.Core/Order.cs");
    let target_line = locations[0]["line"].as_u64().unwrap() as usize;
    let target_character = locations[0]["character"].as_u64().unwrap() as usize;
    let library = fs::read_to_string(copy.join("src/OrderDesk.Core/Order.cs")).unwrap();
    let declaration = library.lines().nth(target_line).expect("the line exists");
    assert!(
        declaration[target_character..].starts_with("Order"),
        "definition does not point at the Order declaration: {declaration}"
    );
    assert_eq!(definition["rejectedTargets"], 0);
    assert_eq!(definition["clientRevision"], 1);

    let actions = host
        .dispatch(
            "lsp.codeActions",
            json!({ "path": path, "line": 2, "character": 16, "revision": 0 }),
        )
        .unwrap();
    assert_eq!(actions["state"], "ok", "{}", actions["message"]);
    assert_eq!(actions["session"], session);
    assert_eq!(actions["documentVersion"], 1);
    assert!(
        !actions["actions"].as_array().unwrap().is_empty(),
        "csharp-ls returned no actions at 2:16: {actions}"
    );
    assert!(
        actions["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["supported"] == true
                && !action["documents"].as_array().unwrap().is_empty()),
        "no action carried a usable WorkspaceEdit: {actions}"
    );

    // A newer buffer means the definition request must use that revision, and a restart
    // forgets every open document instead of reusing the previous session's.
    host.dispatch("lsp.didChange", json!({ "path": path, "text": program }))
        .unwrap();
    let after_change = host.dispatch("lsp.completion", position.clone()).unwrap();
    assert_eq!(after_change["state"], "ok", "{}", after_change["message"]);
    assert_eq!(after_change["clientRevision"], 2);

    let restarted = host.dispatch("lsp.restart", json!({})).unwrap();
    let new_session = restarted["lsp"]["session"].as_u64().unwrap();
    assert_eq!(new_session, session + 1);
    let after_restart = host.dispatch("lsp.completion", position.clone()).unwrap();
    assert_eq!(after_restart["state"], "unavailable");
    assert_eq!(after_restart["documentKnown"], false);
    assert_eq!(after_restart["session"], new_session);
    assert_eq!(after_restart["items"], json!([]));

    host.dispatch("trust.revoke", json!({})).unwrap();
    assert_eq!(
        error_code(host.dispatch("lsp.completion", position.clone())),
        "trust_required"
    );
    assert_eq!(
        error_code(host.dispatch("lsp.definition", position)),
        "trust_required"
    );
    assert_eq!(snapshot(&host)["trust"]["granted"], false);
}

fn write_console_project(dir: &Path, name: &str, extra: &str) {
    fs::write(
        dir.join(format!("{name}.csproj")),
        format!(
            "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework></PropertyGroup>{extra}</Project>\n"
        ),
    )
    .unwrap();
    fs::write(
        dir.join(format!("{name}.cs")),
        "System.Console.WriteLine(\"ready\");\n",
    )
    .unwrap();
}

fn wait_for_build(host: &Host, timeout: Duration) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let state = snapshot(host);
        let build = state["build"].clone();
        if build["state"] != "running" {
            return build;
        }
        assert!(
            Instant::now() < deadline,
            "build did not finish: {}",
            build["log"]
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Waits until the DAP record leaves `building`/`preparing`: the session has real state.
fn wait_for_dap(host: &Host, timeout: Duration) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let dap = snapshot(host)["dap"].clone();
        if !matches!(dap["state"].as_str(), Some("building" | "preparing")) {
            return dap;
        }
        assert!(
            Instant::now() < deadline,
            "dap did not leave building/preparing: {}",
            dap["log"]
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn wait_for_build_output(host: &Host, needle: &str, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    loop {
        let state = snapshot(host);
        let log = state["build"]["log"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        if log.contains(needle) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "build never logged {needle}: {log}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(target_os = "macos")]
#[test]
fn build_start_needs_the_current_evaluation_and_accepts_an_explicit_library() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk build gate");
    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    host.dispatch("evaluation.run", json!({})).unwrap();
    host.dispatch("restore.start", json!({})).unwrap();
    wait_for_restore(&host, Duration::from_secs(60));
    host.dispatch("evaluation.run", json!({})).unwrap();
    let processes_before = snapshot(&host)["processes"].as_array().unwrap().len();

    let unevaluated = host
        .dispatch(
            "build.start",
            json!({
                "project": "src/OrderDesk.Cli/OrderDesk.Cli.csproj",
                "configuration": "Release"
            }),
        )
        .expect_err("a configuration that was not evaluated must not be reused");
    assert_eq!(unevaluated.code, "invalid_state");
    assert!(unevaluated.message.contains("has not been evaluated"));

    assert_eq!(snapshot(&host)["build"]["revision"], 0);
    assert_eq!(
        snapshot(&host)["processes"].as_array().unwrap().len(),
        processes_before,
        "rejected build.start calls must not spawn a process"
    );

    // An explicit tree build may target a library, which is not an executable startup project.
    let started = host
        .dispatch(
            "build.start",
            json!({ "project": "src/OrderDesk.Core/OrderDesk.Core.csproj" }),
        )
        .expect("an explicit library build starts");
    assert_eq!(started["build"]["state"], "running");
    assert_eq!(started["build"]["project"]["name"], "OrderDesk.Core");
    assert_eq!(started["build"]["project"]["kind"], "library");
    let finished = wait_for_build(&host, Duration::from_secs(60));
    assert_eq!(finished["state"], "ok", "{}", finished["log"]);
    assert!(finished["log"]
        .as_str()
        .unwrap()
        .contains("Build succeeded"));

    // A successful restore invalidates the evaluated model, so a later build must not reuse it.
    host.dispatch("restore.start", json!({})).unwrap();
    wait_for_restore(&host, Duration::from_secs(60));
    let stale = snapshot(&host);
    assert_eq!(stale["evaluation"]["stale"], true);
    let denied = host
        .dispatch(
            "build.start",
            json!({ "project": "src/OrderDesk.Cli/OrderDesk.Cli.csproj" }),
        )
        .expect_err("a stale evaluation cannot start a build");
    assert_eq!(denied.code, "invalid_state");
    assert!(denied.message.contains("stale"), "{}", denied.message);
}

#[cfg(target_os = "macos")]
#[test]
fn build_reports_the_exact_command_raw_log_exit_and_real_problems() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    global_json_8(&root);
    write_console_project(&root, "Honest", "");
    let source = root.join("Honest.cs");

    let host = support::host();
    open(&host, &root.join("Honest.csproj"));
    grant(&host);
    host.dispatch("evaluation.run", json!({})).unwrap();
    host.dispatch("restore.start", json!({})).unwrap();
    wait_for_restore(&host, Duration::from_secs(60));
    host.dispatch("evaluation.run", json!({})).unwrap();

    let started = host
        .dispatch(
            "build.start",
            json!({ "savedNote": "Built saved content; no unsaved buffers." }),
        )
        .unwrap();
    let build = &started["build"];
    assert_eq!(build["state"], "running");
    assert_eq!(build["project"]["name"], "Honest");
    assert_eq!(build["project"]["kind"], "console");
    assert_eq!(
        build["project"]["targetPath"].as_str().unwrap(),
        root.join("bin/Debug/net8.0/Honest.dll").to_string_lossy()
    );
    assert_eq!(build["configuration"], "Debug");
    assert_eq!(build["targetFramework"], Value::Null);
    assert_eq!(build["cwd"].as_str().unwrap(), root.to_string_lossy());
    assert_eq!(build["sdkVersion"], "8.0.408");
    let build_root = Path::new(build["sdkRoot"].as_str().unwrap());
    assert!(build_root.is_absolute());
    assert!(build_root.join("sdk/8.0.408").is_dir(), "{build_root:?}");
    assert!(build["buildId"].as_str().unwrap().starts_with("build:"));
    assert_eq!(
        build["savedNote"],
        "Built saved content; no unsaved buffers."
    );
    let command: Vec<String> = build["command"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_string())
        .collect();
    assert_eq!(command.len(), 8, "{command:?}");
    assert!(command[0].ends_with("/dotnet"), "{}", command[0]);
    assert_eq!(command[1], "build");
    assert_eq!(command[2], root.join("Honest.csproj").to_string_lossy());
    assert_eq!(command[3], "-c");
    assert_eq!(command[4], "Debug");
    assert_eq!(command[5], "-nologo");
    assert_eq!(command[6], "-v:minimal");
    assert_eq!(command[7], "--no-restore");
    assert!(!command.iter().any(|arg| arg.contains("-p:TargetFramework")));

    let finished = wait_for_build(&host, Duration::from_secs(60));
    assert_eq!(finished["state"], "ok", "{}", finished["log"]);
    assert_eq!(finished["exit"], 0);
    let log = finished["log"].as_str().unwrap();
    assert!(log.contains("Build succeeded"), "{log}");
    assert!(log.contains("Honest.dll"), "{log}");
    assert_eq!(finished["problems"], json!([]));

    // A real compiler error on disk: the next build reads the saved content and fails.
    fs::write(&source, "System.Console.WriteLine(Missing);\n").unwrap();
    host.dispatch("build.start", json!({})).unwrap();
    let failed = wait_for_build(&host, Duration::from_secs(60));
    assert_eq!(failed["state"], "failed", "{}", failed["log"]);
    assert_eq!(failed["exit"], 1);
    assert!(failed["log"].as_str().unwrap().contains("error CS"));
    let problem = &failed["problems"][0];
    assert_eq!(problem["severity"], "error");
    assert_eq!(problem["severityLabel"], "error");
    assert_eq!(problem["severityRaw"], 1);
    assert_eq!(problem["source"], "dotnet");
    assert_eq!(problem["category"], "unknown");
    assert_eq!(problem["path"], "Honest.cs");
    assert_eq!(problem["position"]["line"], 0);
    assert!(problem["position"]["character"].as_u64().unwrap() > 0);
    assert!(problem["code"].as_str().unwrap().starts_with("CS"));
    assert!(problem["raw"].as_str().unwrap().contains("error CS"));
    assert_eq!(problem["buildId"], failed["buildId"]);
    // The log carries a host-made span for the same location; the UI only renders it.
    let link = &failed["logLinks"][0];
    assert_eq!(link["path"], "Honest.cs");
    assert_eq!(link["position"]["line"], problem["position"]["line"]);
    assert_eq!(
        link["position"]["character"],
        problem["position"]["character"]
    );
    let link_line = link["line"].as_u64().unwrap() as usize;
    let log_line = failed["log"]
        .as_str()
        .unwrap()
        .lines()
        .nth(link_line)
        .unwrap();
    let start = link["start"].as_u64().unwrap() as usize;
    let end = link["end"].as_u64().unwrap() as usize;
    let span: String = log_line.chars().skip(start).take(end - start).collect();
    assert_eq!(
        span,
        format!(
            "{}({},{})",
            source.to_string_lossy(),
            problem["position"]["line"].as_u64().unwrap() + 1,
            problem["position"]["character"].as_u64().unwrap() + 1
        ),
        "{log_line}"
    );
    // Build problems live in the build snapshot, never merged into the LSP store.
    let state = snapshot(&host);
    assert_eq!(state["diagnostics"]["snapshots"], json!([]));

    // Fixing and saving produces a new, verifiable run.
    fs::write(&source, "System.Console.WriteLine(\"fixed\");\n").unwrap();
    host.dispatch("build.start", json!({})).unwrap();
    let fixed = wait_for_build(&host, Duration::from_secs(60));
    assert_eq!(fixed["state"], "ok", "{}", fixed["log"]);
    assert_eq!(fixed["exit"], 0);
    assert_eq!(fixed["problems"], json!([]));
    assert_ne!(fixed["buildId"], failed["buildId"]);
}

#[cfg(target_os = "macos")]
#[test]
fn build_requires_an_explicit_startup_when_more_than_one_console_project_exists() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    global_json_8(&root);
    fs::create_dir_all(root.join("alpha")).unwrap();
    fs::create_dir_all(root.join("beta")).unwrap();
    write_console_project(&root.join("alpha"), "Alpha", "");
    write_console_project(&root.join("beta"), "Beta", "");
    fs::write(
        root.join("Pair.sln"),
        "Microsoft Visual Studio Solution File, Format Version 12.00\n\
Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Alpha\", \"alpha\\Alpha.csproj\", \"{11111111-1111-1111-1111-111111111111}\"\nEndProject\n\
Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Beta\", \"beta\\Beta.csproj\", \"{22222222-2222-2222-2222-222222222222}\"\nEndProject\n\
Global\n\tGlobalSection(SolutionConfigurationPlatforms) = preSolution\n\t\tDebug|Any CPU = Debug|Any CPU\n\tEndGlobalSection\n\tGlobalSection(ProjectConfigurationPlatforms) = postSolution\n\t\t{11111111-1111-1111-1111-111111111111}.Debug|Any CPU.ActiveCfg = Debug|Any CPU\n\t\t{11111111-1111-1111-1111-111111111111}.Debug|Any CPU.Build.0 = Debug|Any CPU\n\t\t{22222222-2222-2222-2222-222222222222}.Debug|Any CPU.ActiveCfg = Debug|Any CPU\n\t\t{22222222-2222-2222-2222-222222222222}.Debug|Any CPU.Build.0 = Debug|Any CPU\n\tEndGlobalSection\nEndGlobal\n",
    )
    .unwrap();

    let host = support::host();
    open(&host, &root.join("Pair.sln"));
    grant(&host);
    host.dispatch("evaluation.run", json!({})).unwrap();
    host.dispatch("restore.start", json!({})).unwrap();
    wait_for_restore(&host, Duration::from_secs(60));
    host.dispatch("evaluation.run", json!({})).unwrap();

    let ambiguous = host
        .dispatch("build.start", json!({}))
        .expect_err("two console projects need an explicit startup");
    assert_eq!(ambiguous.code, "invalid_state");
    let details = ambiguous.details.unwrap();
    assert_eq!(details["reason"], "ambiguous");
    assert_eq!(details["candidates"].as_array().unwrap().len(), 2);
    assert_eq!(snapshot(&host)["build"]["revision"], 0);

    let started = host
        .dispatch("build.start", json!({ "project": "beta/Beta.csproj" }))
        .unwrap();
    assert_eq!(started["build"]["project"]["name"], "Beta");
    let finished = wait_for_build(&host, Duration::from_secs(60));
    assert_eq!(finished["state"], "ok", "{}", finished["log"]);
    assert!(finished["command"]
        .as_array()
        .unwrap()
        .iter()
        .any(|arg| arg == &json!(root.join("beta/Beta.csproj").to_string_lossy())));
}

#[cfg(unix)]
fn hanging_build_workspace() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    global_json_8(&root);
    write_console_project(
        &root,
        "Slow",
        "<Target Name=\"Hang\" AfterTargets=\"Build\"><Exec Command=\"sleep 30\" /></Target>",
    );
    temp
}

#[cfg(unix)]
fn ready_project(host: &Host, root: &Path, project: &str) {
    open(host, &root.join(project));
    grant(host);
    host.dispatch("evaluation.run", json!({})).unwrap();
    host.dispatch("restore.start", json!({})).unwrap();
    wait_for_restore(host, Duration::from_secs(60));
    host.dispatch("evaluation.run", json!({})).unwrap();
}

#[cfg(unix)]
fn ready_for_build(host: &Host, root: &Path) {
    ready_project(host, root, "Slow.csproj");
}

#[cfg(unix)]
#[test]
fn cancelling_a_real_hanging_build_reports_requested_and_observed_termination() {
    let temp = hanging_build_workspace();
    let root = fs::canonicalize(temp.path()).unwrap();
    let host = support::host();
    ready_for_build(&host, &root);
    let started = host.dispatch("build.start", json!({})).unwrap();
    let process_id = started["build"]["processId"]
        .as_u64()
        .unwrap_or_else(|| panic!("no build process: {}", started["build"]));

    wait_for_build_output(&host, "Slow.dll", Duration::from_secs(30));

    let cancelled = host.dispatch("build.cancel", json!({})).unwrap();
    let build = &cancelled["build"];
    assert_eq!(build["state"], "cancelled");
    assert_eq!(build["awaitingDecision"], false);
    assert!(build["termination"]["requested"]
        .as_str()
        .unwrap()
        .contains("SIGTERM"));
    assert_eq!(build["termination"]["observed"], "exited");
    assert_ne!(build["exit"], json!(0));
    assert!(build["log"].as_str().unwrap().contains("Slow.dll"));
    let process = cancelled["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|process| process["id"].as_u64() == Some(process_id))
        .unwrap();
    assert_eq!(process["state"], "exited");
    assert_eq!(build["exit"], process["exit"]);
    assert_eq!(
        error_code(host.dispatch("build.cancel", json!({}))),
        "invalid_state"
    );
}

#[cfg(unix)]
#[test]
fn revoking_trust_seals_a_running_build_and_drops_its_late_exit() {
    let temp = hanging_build_workspace();
    let root = fs::canonicalize(temp.path()).unwrap();
    let host = support::host();
    ready_for_build(&host, &root);
    host.dispatch("build.start", json!({})).unwrap();
    wait_for_build_output(&host, "Slow.dll", Duration::from_secs(30));

    let revoked = host.dispatch("trust.revoke", json!({})).unwrap();
    let build = &revoked["build"];
    assert_eq!(build["sealed"], true);
    assert_eq!(build["stale"], true);
    assert!(build["staleReason"].as_str().unwrap().contains("Trust"));
    assert_eq!(build["state"], "cancelled");
    assert_eq!(build["termination"]["observed"], "exited");
    let finished_at = build["finishedAt"].clone();

    std::thread::sleep(Duration::from_millis(500));
    let later = snapshot(&host);
    assert_eq!(later["build"]["sealed"], true);
    assert_eq!(later["build"]["state"], "cancelled");
    assert_eq!(later["build"]["finishedAt"], finished_at);
    assert_eq!(
        error_code(host.dispatch("build.start", json!({}))),
        "trust_required"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn run_launches_the_identified_build_and_rejects_a_changed_source_or_failed_build() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk run");
    let program = copy.join("src/OrderDesk.Cli/Program.cs");
    let dll = copy.join("src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll");
    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    host.dispatch("evaluation.run", json!({})).unwrap();
    host.dispatch("restore.start", json!({})).unwrap();
    wait_for_restore(&host, Duration::from_secs(60));
    host.dispatch("evaluation.run", json!({})).unwrap();

    let built = host.dispatch("build.start", json!({})).unwrap();
    let build_id = built["build"]["buildId"].clone();
    let built = wait_for_build(&host, Duration::from_secs(60));
    assert_eq!(built["state"], "ok", "{}", built["log"]);

    let library = host
        .dispatch(
            "run.start",
            json!({ "project": "src/OrderDesk.Core/OrderDesk.Core.csproj" }),
        )
        .expect_err("a library is not runnable");
    assert_eq!(library.code, "invalid_state");
    assert!(
        library.message.contains("library") && library.message.contains("console"),
        "{}",
        library.message
    );

    let started = host.dispatch("run.start", json!({})).unwrap();
    let run = &started["run"];
    assert_eq!(run["state"], "running");
    assert_eq!(run["buildId"], build_id);
    assert!(run["targetPath"]
        .as_str()
        .unwrap()
        .ends_with("src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll"));
    let command: Vec<String> = run["command"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_string())
        .collect();
    assert_eq!(command.len(), 2, "{command:?}");
    assert!(command[0].ends_with("/dotnet"), "{}", command[0]);
    assert_eq!(command[1], run["targetPath"].as_str().unwrap());
    assert!(
        run["cwd"]
            .as_str()
            .unwrap()
            .ends_with("Order Desk run/src/OrderDesk.Cli"),
        "{}",
        run["cwd"]
    );

    let finished = wait_for_run(&host, Duration::from_secs(30));
    assert_eq!(finished["state"], "exited", "{}", finished["log"]);
    assert_eq!(finished["exit"], 0);
    assert!(
        finished["log"].as_str().unwrap().contains("Order A-104:"),
        "{}",
        finished["log"]
    );

    // A saved source change after the build starts the same build pipeline and only then runs:
    // the host never launches code it cannot prove is current.
    let original = fs::read_to_string(&program).unwrap();
    fs::write(&program, format!("{original}\n// edited after the build\n")).unwrap();
    let implicit = host.dispatch("run.start", json!({})).unwrap();
    assert_eq!(implicit["run"]["state"], "building");
    assert_ne!(implicit["build"]["revision"], 0);
    let rebuilt_for_change = wait_for_build(&host, Duration::from_secs(60));
    assert_eq!(
        rebuilt_for_change["state"], "ok",
        "{}",
        rebuilt_for_change["log"]
    );
    assert_ne!(rebuilt_for_change["buildId"], build_id);
    let after_change = wait_for_run(&host, Duration::from_secs(30));
    assert_eq!(after_change["state"], "exited", "{}", after_change["log"]);
    assert_eq!(after_change["exit"], 0);
    assert_eq!(after_change["buildId"], rebuilt_for_change["buildId"]);

    // The rebuild produces new output, which proves the run below used the new build.
    fs::write(
        &program,
        "using OrderDesk.Core;\nvar order = new Order(\"A-999\", 7m);\nSystem.Console.WriteLine($\"Order {order.Id}: {order.Total:C}\");\nSystem.Console.WriteLine($\"args=[{string.Join(\"|\", args)}]\");\nSystem.Console.WriteLine($\"env={System.Environment.GetEnvironmentVariable(\"OPENRIDR_F4\")}\");\nSystem.Console.WriteLine($\"override={System.Environment.GetEnvironmentVariable(\"OPENRIDR_OVERRIDE\")}\");\nSystem.Console.WriteLine($\"cwd={System.Environment.CurrentDirectory}\");\n",
    )
    .unwrap();
    fs::write(
        copy.join("run.env"),
        "# values for the active configuration\nOPENRIDR_F4=from-file\nOPENRIDR_OVERRIDE=file\n",
    )
    .unwrap();
    let configured = host
        .dispatch(
            "run.start",
            json!({
                "args": ["--first", "two words"],
                "workingDirectory": "src",
                "envFiles": ["run.env"],
                "env": { "OPENRIDR_F4": "explicit" },
            }),
        )
        .unwrap();
    assert_eq!(configured["run"]["state"], "building");
    let rerun = wait_for_run(&host, Duration::from_secs(60));
    assert_eq!(rerun["state"], "exited", "{}", rerun["log"]);
    let configured_command: Vec<&str> = rerun["command"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    assert_eq!(
        &configured_command[2..],
        ["--first", "two words"],
        "{configured_command:?}"
    );
    assert!(
        rerun["cwd"].as_str().unwrap().ends_with("/src"),
        "{}",
        rerun["cwd"]
    );
    let log = rerun["log"].as_str().unwrap();
    assert!(log.contains("Order A-999:"), "{log}");
    assert!(log.contains("args=[--first|two words]"), "{log}");
    assert!(
        log.contains("env=explicit"),
        "explicit env must win over .env: {log}"
    );
    assert!(
        log.contains("override=file"),
        "the .env file must be applied: {log}"
    );
    assert!(
        log.lines()
            .any(|line| line.starts_with("cwd=") && line.ends_with("/src")),
        "{log}"
    );
    assert_ne!(rerun["buildId"], build_id);

    // A failed implicit build never runs and keeps its diagnostics in the Build tab.
    let run_processes = |host: &Host| {
        snapshot(host)["processes"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|process| {
                process["kind"]
                    .as_str()
                    .is_some_and(|kind| kind.starts_with("run:"))
            })
            .count()
    };
    let runs_before = run_processes(&host);
    fs::write(&program, "class Broken {\n").unwrap();
    let started = host.dispatch("run.start", json!({})).unwrap();
    assert_eq!(started["run"]["state"], "building");
    let failed = wait_for_run(&host, Duration::from_secs(60));
    assert_eq!(failed["state"], "failed", "{}", failed["log"]);
    assert!(
        failed["log"]
            .as_str()
            .unwrap()
            .contains("Build failed; see the Build tab"),
        "{}",
        failed["log"]
    );
    let failed_build = snapshot(&host)["build"].clone();
    assert_eq!(failed_build["state"], "failed");
    assert!(
        failed_build["log"].as_str().unwrap().contains("error CS"),
        "{}",
        failed_build["log"]
    );
    assert_eq!(
        run_processes(&host),
        runs_before,
        "a failed build must not spawn the program"
    );

    // A real successful build whose TargetPath disappeared is rebuilt, not launched into a
    // stale path and not refused.
    fs::write(&program, original).unwrap();
    host.dispatch("build.start", json!({})).unwrap();
    let fixed = wait_for_build(&host, Duration::from_secs(60));
    assert_eq!(fixed["state"], "ok", "{}", fixed["log"]);
    fs::remove_file(&dll).unwrap();
    let missing = host.dispatch("run.start", json!({})).unwrap();
    assert_eq!(missing["run"]["state"], "building");
    let rebuilt = wait_for_run(&host, Duration::from_secs(60));
    assert_eq!(rebuilt["state"], "exited", "{}", rebuilt["log"]);
    assert_eq!(rebuilt["exit"], 0);
    assert!(
        dll.is_file(),
        "the implicit build must restore the TargetPath"
    );
}

#[cfg(unix)]
#[test]
fn run_implicitly_builds_and_runs_when_there_is_no_build() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    global_json_8(&root);
    write_console_project(&root, "Implicit", "");
    let host = support::host();
    ready_project(&host, &root, "Implicit.csproj");
    assert_eq!(snapshot(&host)["build"]["revision"], 0);

    let started = host.dispatch("run.start", json!({})).unwrap();
    assert_eq!(started["run"]["state"], "building");
    assert_ne!(started["build"]["revision"], 0, "{}", started["build"]);
    assert_eq!(started["run"]["buildId"], started["build"]["buildId"]);
    assert!(
        started["run"]["targetPath"]
            .as_str()
            .unwrap()
            .ends_with("Implicit.dll"),
        "{}",
        started["run"]["targetPath"]
    );
    assert!(
        !started["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|process| process["kind"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("run:"))),
        "the program must not start before its build"
    );

    let finished = wait_for_run(&host, Duration::from_secs(60));
    assert_eq!(finished["state"], "exited", "{}", finished["log"]);
    assert_eq!(finished["exit"], 0);
    assert!(
        finished["log"].as_str().unwrap().contains("ready"),
        "{}",
        finished["log"]
    );
    let build = snapshot(&host)["build"].clone();
    assert_eq!(build["state"], "ok", "{}", build["log"]);
    assert_eq!(build["buildId"], finished["buildId"]);
}

#[cfg(unix)]
#[test]
fn a_failed_implicit_build_fails_the_run_and_keeps_its_diagnostics() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    global_json_8(&root);
    write_console_project(&root, "BrokenRun", "");
    fs::write(root.join("BrokenRun.cs"), "class Broken {\n").unwrap();
    let host = support::host();
    ready_project(&host, &root, "BrokenRun.csproj");

    let started = host.dispatch("run.start", json!({})).unwrap();
    assert_eq!(started["run"]["state"], "building");
    let failed = wait_for_run(&host, Duration::from_secs(60));
    assert_eq!(failed["state"], "failed", "{}", failed["log"]);
    assert!(
        failed["log"]
            .as_str()
            .unwrap()
            .contains("Build failed; see the Build tab"),
        "{}",
        failed["log"]
    );
    let state = snapshot(&host);
    assert_eq!(state["build"]["state"], "failed");
    assert!(
        state["build"]["log"].as_str().unwrap().contains("error CS"),
        "{}",
        state["build"]["log"]
    );
    assert!(
        !state["build"]["problems"].as_array().unwrap().is_empty(),
        "the failed build keeps its diagnostics in the Build view"
    );
    assert!(
        !state["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|process| process["kind"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("run:"))),
        "a failed build must not spawn the program"
    );
}

#[cfg(unix)]
#[test]
fn cancelling_a_run_waiting_for_its_build_stops_the_build_and_never_launches() {
    let temp = hanging_build_workspace();
    let root = fs::canonicalize(temp.path()).unwrap();
    let host = support::host();
    ready_for_build(&host, &root);

    let started = host.dispatch("run.start", json!({})).unwrap();
    assert_eq!(started["run"]["state"], "building");
    wait_for_build_output(&host, "Slow.dll", Duration::from_secs(30));

    let cancelled = host.dispatch("run.cancel", json!({})).unwrap();
    assert_eq!(cancelled["run"]["state"], "cancelled");
    assert_eq!(cancelled["run"]["exit"], Value::Null);
    assert_eq!(cancelled["build"]["state"], "cancelled");
    assert!(
        cancelled["build"]["termination"]["requested"]
            .as_str()
            .is_some(),
        "{}",
        cancelled["build"]["termination"]
    );
    let state = snapshot(&host);
    assert!(
        !state["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|process| process["kind"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("run:"))),
        "the program must never start after a cancelled build"
    );
}

#[cfg(unix)]
#[test]
fn a_second_run_start_while_waiting_for_the_build_does_not_start_another() {
    let temp = hanging_build_workspace();
    let root = fs::canonicalize(temp.path()).unwrap();
    let host = support::host();
    ready_for_build(&host, &root);

    let first = host.dispatch("run.start", json!({})).unwrap();
    assert_eq!(first["run"]["state"], "building");
    let second = host
        .dispatch("run.start", json!({}))
        .expect_err("a second run must not start another build");
    assert_eq!(second.code, "invalid_state");
    assert!(
        second.message.contains("already waiting"),
        "{}",
        second.message
    );
    let builds = snapshot(&host)["processes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|process| {
            process["kind"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("build:"))
        })
        .count();
    assert_eq!(builds, 1, "a double start must not start two builds");
    let _ = host.dispatch("run.cancel", json!({}));
}

/// A successful build proves compilation, not that the restore inputs it depended on still
/// exist. Removing them after the build must stop Run at the same explicit gate as Build.
#[cfg(target_os = "macos")]
#[test]
fn run_start_reports_restore_required_when_assets_are_invalidated_after_a_build() {
    let temp = tempfile::tempdir().unwrap();
    let copy = fixture_copy(temp.path(), "Order Desk run restore");
    let assets = copy.join("src/OrderDesk.Cli/obj/project.assets.json");
    let host = support::host();
    open(&host, &copy.join("OrderDesk.sln"));
    grant(&host);
    host.dispatch("evaluation.run", json!({})).unwrap();
    host.dispatch("restore.start", json!({})).unwrap();
    wait_for_restore(&host, Duration::from_secs(60));
    host.dispatch("evaluation.run", json!({})).unwrap();

    host.dispatch("build.start", json!({})).unwrap();
    let built = wait_for_build(&host, Duration::from_secs(60));
    assert_eq!(built["state"], "ok", "{}", built["log"]);
    assert!(
        assets.is_file(),
        "restore must have produced the assets file"
    );

    fs::remove_file(&assets).unwrap();
    let refused = host
        .dispatch("run.start", json!({}))
        .expect_err("run must not bypass the restore policy after a build");
    assert_eq!(refused.code, "restore_required");
    assert!(refused.message.contains("never restores implicitly"));
    let details = refused.details.unwrap();
    assert_eq!(details["consumer"], "run.start");
    assert_eq!(details["action"]["method"], "restore.start");
    assert_eq!(details["projects"][0]["restored"], "not_restored");
    assert!(!assets.exists(), "run.start must never restore implicitly");

    let state = snapshot(&host);
    assert_eq!(state["run"]["state"], "idle");
    assert!(
        !state["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|process| process["kind"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("run:"))),
        "run.start must not start a process"
    );
}

#[cfg(unix)]
fn sleeper_project(dir: &Path) {
    global_json_8(dir);
    write_console_project(dir, "Sleeper", "");
    fs::write(
        dir.join("Sleeper.cs"),
        "using System.Diagnostics;\n\
         var child = Process.Start(\"/bin/sleep\", \"60\");\n\
         System.Console.WriteLine($\"parent {System.Environment.ProcessId} child {child!.Id}\");\n\
         System.Console.WriteLine(\"sleeping\");\n\
         System.Threading.Thread.Sleep(60_000);\n",
    )
    .unwrap();
}

#[cfg(unix)]
fn wait_for_run(host: &Host, timeout: Duration) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let state = snapshot(host);
        let run = state["run"].clone();
        // `building` is the implicit build this run waits for, not a finished state.
        if !matches!(run["state"].as_str(), Some("running" | "building")) {
            return run;
        }
        assert!(
            Instant::now() < deadline,
            "run did not finish: {}",
            run["log"]
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(unix)]
fn wait_for_run_output(host: &Host, needle: &str, timeout: Duration) -> String {
    let deadline = Instant::now() + timeout;
    loop {
        let state = snapshot(host);
        let log = state["run"]["log"].as_str().unwrap_or_default().to_string();
        if log.contains(needle) {
            return log;
        }
        assert!(
            Instant::now() < deadline,
            "run never logged {needle}: {log}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(unix)]
fn process_gone(pid: &str) -> bool {
    std::process::Command::new("/bin/ps")
        .args(["-o", "pid=", "-p", pid])
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().is_empty())
        .unwrap_or(false)
}

#[cfg(unix)]
#[test]
fn cancelling_a_real_run_observes_termination_of_the_group_including_its_child() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    sleeper_project(&root);
    let host = support::host();
    ready_project(&host, &root, "Sleeper.csproj");
    host.dispatch("build.start", json!({})).unwrap();
    let built = wait_for_build(&host, Duration::from_secs(60));
    assert_eq!(built["state"], "ok", "{}", built["log"]);

    let started = host.dispatch("run.start", json!({})).unwrap();
    let process_id = started["run"]["processId"].as_u64().unwrap();
    let log = wait_for_run_output(&host, "sleeping", Duration::from_secs(20));
    let pids: Vec<&str> = log
        .lines()
        .find(|line| line.starts_with("parent "))
        .expect("pid line")
        .split_whitespace()
        .collect();
    let parent = pids[1].to_string();
    let child = pids[3].to_string();
    assert_eq!(
        started["processes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|process| process["id"].as_u64() == Some(process_id))
            .unwrap()["pid"]
            .as_u64(),
        Some(parent.parse().unwrap())
    );

    assert_eq!(
        error_code(host.dispatch("run.start", json!({}))),
        "invalid_state",
        "only one program runs at a time"
    );

    let cancelled = host.dispatch("run.cancel", json!({})).unwrap();
    let run = &cancelled["run"];
    assert_eq!(run["state"], "cancelled");
    assert_eq!(run["awaitingDecision"], false);
    assert!(run["termination"]["requested"]
        .as_str()
        .unwrap()
        .contains("SIGTERM"));
    assert_eq!(run["termination"]["observed"], "exited");
    assert_ne!(run["exit"], json!(0));
    assert!(run["log"].as_str().unwrap().contains("sleeping"));
    let process = cancelled["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|process| process["id"].as_u64() == Some(process_id))
        .unwrap();
    assert_eq!(process["state"], "exited");
    assert_eq!(run["exit"], process["exit"]);
    assert_eq!(
        error_code(host.dispatch("run.cancel", json!({}))),
        "invalid_state"
    );

    let deadline = Instant::now() + Duration::from_secs(5);
    while !(process_gone(&parent) && process_gone(&child)) {
        assert!(
            Instant::now() < deadline,
            "process tree still alive: parent {parent} child {child}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(unix)]
#[test]
fn revoking_trust_seals_a_running_program_and_drops_its_late_exit() {
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    sleeper_project(&root);
    let host = support::host();
    ready_project(&host, &root, "Sleeper.csproj");
    host.dispatch("build.start", json!({})).unwrap();
    let built = wait_for_build(&host, Duration::from_secs(60));
    assert_eq!(built["state"], "ok", "{}", built["log"]);
    host.dispatch("run.start", json!({})).unwrap();
    wait_for_run_output(&host, "sleeping", Duration::from_secs(20));

    let revoked = host.dispatch("trust.revoke", json!({})).unwrap();
    let run = &revoked["run"];
    assert_eq!(run["sealed"], true);
    assert_eq!(run["stale"], true);
    assert!(run["staleReason"].as_str().unwrap().contains("Trust"));
    assert_eq!(run["state"], "cancelled");
    assert_eq!(run["termination"]["observed"], "exited");
    assert!(run["log"].as_str().unwrap().contains("sleeping"));
    let finished_at = run["finishedAt"].clone();

    std::thread::sleep(Duration::from_millis(500));
    let later = snapshot(&host);
    assert_eq!(later["run"]["sealed"], true);
    assert_eq!(later["run"]["state"], "cancelled");
    assert_eq!(later["run"]["finishedAt"], finished_at);
    assert_eq!(
        error_code(host.dispatch("run.start", json!({}))),
        "trust_required"
    );
}

#[cfg(unix)]
fn write_netcoredbg(path: &Path, version: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        path,
        format!(
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then\n  echo \"NET Core debugger {version}-1 (controlled test artifact, Release)\"\n  exit 0\nfi\nexit 0\n"
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn debug_start_gates_the_experimental_flag_adapter_current_debug_build_and_sources_isolated() {
    if std::env::consts::ARCH != "aarch64" {
        return;
    }
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let adapter = home
        .path()
        .join("tools/netcoredbg/3.2.0-1092/netcoredbg/netcoredbg");
    write_netcoredbg(&adapter, "3.2.0");

    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    global_json_8(&root);
    write_console_project(&root, "Debugged", "");
    let source = root.join("Debugged.cs");
    let host = support::host();
    open(&host, &root.join("Debugged.csproj"));
    grant(&host);
    host.dispatch("evaluation.run", json!({})).unwrap();

    // Debug consumes restored assets and never restores implicitly.
    let needs_restore = host
        .dispatch("debug.start", json!({}))
        .expect_err("no restore");
    assert_eq!(needs_restore.code, "restore_required");
    assert_eq!(needs_restore.details.unwrap()["consumer"], "debug.start");
    host.dispatch("restore.start", json!({})).unwrap();
    wait_for_restore(&host, Duration::from_secs(60));
    host.dispatch("evaluation.run", json!({})).unwrap();

    // The platform flag is a real gate even with a verified adapter and trust.
    let flag_off = host
        .dispatch("debug.start", json!({}))
        .expect_err("flag off");
    assert_eq!(flag_off.code, "experimental_required");
    assert!(
        flag_off.message.contains("experimental"),
        "{}",
        flag_off.message
    );
    host.dispatch("debug.flag", json!({ "enabled": true, "confirm": true }))
        .unwrap();

    // No build yet: Debug starts the implicit build and waits; Stop cancels both, and no
    // adapter is ever spawned.
    let building = host.dispatch("debug.start", json!({})).unwrap();
    assert_eq!(building["dap"]["state"], "building");
    assert_ne!(building["build"]["revision"], 0);
    let cancelled = host.dispatch("debug.stop", json!({})).unwrap();
    assert_eq!(cancelled["dap"]["state"], "cancelled");
    assert!(cancelled["dap"]["session"].as_u64().unwrap() > 0);
    assert!(
        !snapshot(&host)["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|process| process["kind"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("debug:"))),
        "a session waiting for its build must not spawn the adapter"
    );

    // Breakpoints can be prepared before the session; without an adapter they stay pending.
    let pending = host
        .dispatch(
            "debug.setBreakpoints",
            json!({ "path": "Debugged.cs", "lines": [2] }),
        )
        .unwrap();
    assert_eq!(pending["dap"]["breakpoints"][0]["line"], 2);
    assert_eq!(pending["dap"]["breakpoints"][0]["verified"], false);
    assert!(
        !snapshot(&host)["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|process| process["kind"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("debug:"))),
        "preparing breakpoints must not spawn the adapter"
    );

    host.dispatch("build.start", json!({})).unwrap();
    let built = wait_for_build(&host, Duration::from_secs(60));
    assert_eq!(built["state"], "ok", "{}", built["log"]);
    assert_eq!(built["configuration"], "Debug");

    // A saved source changed after the build starts the implicit rebuild; Stop cancels it
    // before any adapter exists.
    fs::write(&source, "System.Console.WriteLine(\"changed\");\n").unwrap();
    let changed = host.dispatch("debug.start", json!({})).unwrap();
    assert_eq!(changed["dap"]["state"], "building");
    let stopped = host.dispatch("debug.stop", json!({})).unwrap();
    assert_eq!(stopped["dap"]["state"], "cancelled");

    // A failed implicit build fails the pending session and never spawns the adapter.
    fs::write(&source, "this is not C#\n").unwrap();
    let failed_start = host.dispatch("debug.start", json!({})).unwrap();
    assert_eq!(failed_start["dap"]["state"], "building");
    let failed = wait_for_dap(&host, Duration::from_secs(60));
    assert_eq!(failed["state"], "failed", "{}", failed["log"]);
    assert!(
        failed["log"]
            .as_str()
            .unwrap()
            .contains("Build failed; see the Build tab"),
        "{}",
        failed["log"]
    );
    assert!(
        !snapshot(&host)["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|process| process["kind"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("debug:"))),
        "a failed build must not spawn the adapter"
    );

    let failed_session = failed["session"].clone();
    // A version that is not the approved one is not a debugger.
    write_netcoredbg(&adapter, "3.1.0");
    let wrong_version = host
        .dispatch("debug.start", json!({}))
        .expect_err("wrong version");
    assert_eq!(wrong_version.code, "tool_missing");
    assert!(
        wrong_version.message.contains("3.2.0"),
        "{}",
        wrong_version.message
    );

    // A re-detected SDK/context invalidates the evaluated model before any launch.
    write_netcoredbg(&adapter, "3.2.0");
    fs::write(&source, "System.Console.WriteLine(\"ready\");\n").unwrap();
    host.dispatch("build.start", json!({})).unwrap();
    let rebuilt = wait_for_build(&host, Duration::from_secs(60));
    assert_eq!(rebuilt["state"], "ok", "{}", rebuilt["log"]);
    host.dispatch("sdk.redetect", json!({})).unwrap();
    let stale = host
        .dispatch("debug.start", json!({}))
        .expect_err("stale evaluation");
    assert_eq!(stale.code, "invalid_state");
    assert_eq!(stale.details.unwrap()["reason"], "evaluation_stale");
    let unchanged = snapshot(&host);
    assert_eq!(unchanged["dap"]["session"], failed_session);
    assert_eq!(unchanged["dap"]["state"], "failed");
}

#[cfg(target_os = "macos")]
fn copy_dir(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.path().is_file() {
            fs::copy(entry.path(), destination.join(entry.file_name())).unwrap();
        }
    }
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn stopping_a_debug_session_waiting_for_its_build_stops_the_build_isolated() {
    if std::env::consts::ARCH != "aarch64" {
        return;
    }
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let adapter = home
        .path()
        .join("tools/netcoredbg/3.2.0-1092/netcoredbg/netcoredbg");
    write_netcoredbg(&adapter, "3.2.0");

    let temp = hanging_build_workspace();
    let root = fs::canonicalize(temp.path()).unwrap();
    let host = support::host();
    ready_for_build(&host, &root);
    host.dispatch("debug.flag", json!({ "enabled": true, "confirm": true }))
        .unwrap();

    let started = host.dispatch("debug.start", json!({})).unwrap();
    assert_eq!(started["dap"]["state"], "building");
    wait_for_build_output(&host, "Slow.dll", Duration::from_secs(30));

    let stopped = host.dispatch("debug.stop", json!({})).unwrap();
    assert_eq!(stopped["dap"]["state"], "cancelled");
    assert_eq!(stopped["build"]["state"], "cancelled");
    assert!(
        stopped["dap"]["termination"].is_null(),
        "no termination is invented for a session that never started"
    );
    assert!(
        !snapshot(&host)["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|process| process["kind"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("debug:"))),
        "no adapter is spawned for a cancelled implicit build"
    );

    // Cancel and restart during the build: the new pending session must not repeat the
    // session number the cancelled `building` record already showed.
    let cancelled_session = stopped["dap"]["session"].as_u64().unwrap();
    let restarted = host.dispatch("debug.start", json!({})).unwrap();
    assert_eq!(restarted["dap"]["state"], "building");
    assert_ne!(
        restarted["dap"]["session"].as_u64().unwrap(),
        cancelled_session
    );
    let stopped_again = host.dispatch("debug.stop", json!({})).unwrap();
    assert_eq!(stopped_again["dap"]["state"], "cancelled");
}

#[cfg(target_os = "macos")]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn debug_implicitly_builds_and_starts_the_session_when_there_is_no_build_isolated() {
    if std::env::consts::ARCH != "aarch64" {
        return;
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap();
    let installed =
        home.join("Library/Application Support/OpenRidr/tools/netcoredbg/3.2.0-1092/netcoredbg");
    if !installed.join("netcoredbg").is_file() {
        eprintln!("not verified: netcoredbg 3.2.0-1092 is not installed in the default app home");
        return;
    }
    let app_home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", app_home.path());
    copy_dir(
        &installed,
        &app_home
            .path()
            .join("tools/netcoredbg/3.2.0-1092/netcoredbg"),
    );

    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    global_json_8(&root);
    write_console_project(&root, "ImplicitDebug", "");
    let host = support::host();
    open(&host, &root.join("ImplicitDebug.csproj"));
    grant(&host);
    host.dispatch("evaluation.run", json!({})).unwrap();
    host.dispatch("restore.start", json!({})).unwrap();
    wait_for_restore(&host, Duration::from_secs(60));
    host.dispatch("evaluation.run", json!({})).unwrap();
    host.dispatch("debug.flag", json!({ "enabled": true, "confirm": true }))
        .unwrap();
    assert_eq!(snapshot(&host)["build"]["revision"], 0);

    let started = host.dispatch("debug.start", json!({})).unwrap();
    assert_eq!(started["dap"]["state"], "building");
    assert_ne!(started["build"]["revision"], 0);
    assert_eq!(started["dap"]["buildId"], started["build"]["buildId"]);

    let session = wait_for_dap(&host, Duration::from_secs(90));
    assert!(
        matches!(
            session["state"].as_str(),
            Some("running" | "paused" | "terminated")
        ),
        "{}",
        session["log"]
    );
    assert!(session["session"].as_u64().unwrap() > 0);
    assert_eq!(session["buildId"], snapshot(&host)["build"]["buildId"]);
    assert_eq!(
        snapshot(&host)["build"]["state"],
        "ok",
        "{}",
        snapshot(&host)["build"]["log"]
    );
    assert!(
        snapshot(&host)["processes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|process| process["kind"]
                .as_str()
                .is_some_and(|kind| kind.starts_with("debug:"))),
        "the adapter is an observed process"
    );
    let _ = host.dispatch("debug.stop", json!({}));
}

fn wait_for_terminal_output(host: &Host, needle: &str, timeout: Duration) -> String {
    let deadline = Instant::now() + timeout;
    loop {
        let log = snapshot(host)["terminal"]["log"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        if log.contains(needle) {
            return log;
        }
        assert!(
            Instant::now() < deadline,
            "terminal never logged {needle}: {log}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Stops the shell even when an assertion fails, so a failed test leaves no stray child.
struct TerminalCleanup<'a>(&'a Host);

impl Drop for TerminalCleanup<'_> {
    fn drop(&mut self) {
        let _ = self.0.dispatch("terminal.stop", json!({ "confirm": true }));
    }
}

#[test]
fn terminal_start_needs_trust_and_spawns_no_shell() {
    let host = support::host();
    let workspace = temp_workspace(&["Gate.csproj"]);
    open(&host, &workspace.path().join("Gate.csproj"));

    for method in ["terminal.start", "terminal.write", "terminal.resize"] {
        assert_eq!(
            error_code(host.dispatch(method, json!({}))),
            "trust_required",
            "{method} must be denied by the host gate"
        );
    }
    let state = snapshot(&host);
    assert_eq!(state["terminal"]["state"], "idle");
    assert_eq!(state["terminal"]["revision"], 0);
    assert_eq!(state["terminal"]["processId"], Value::Null);
    assert_eq!(state["terminal"]["shell"], "");
    assert_eq!(state["processes"], json!([]));
    assert!(!workspace.path().join("bin").exists());
}

#[cfg(unix)]
#[test]
#[ignore = "isolated leaf: it sets SHELL for its own shell, so it runs in its own process"]
fn a_real_pty_shell_reads_input_writes_output_and_ctrl_c_interrupts_the_child_isolated() {
    let _shell = EnvVar::set("SHELL", "/bin/sh");
    let temp = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(temp.path()).unwrap();
    fs::write(
        root.join("Pty.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n",
    )
    .unwrap();
    let host = support::host();
    open(&host, &root.join("Pty.csproj"));
    grant(&host);
    let _cleanup = TerminalCleanup(&host);

    let started = host
        .dispatch("terminal.start", json!({ "cols": 120, "rows": 40 }))
        .unwrap();
    let terminal = &started["terminal"];
    assert_eq!(terminal["state"], "running");
    assert_eq!(terminal["shell"], "/bin/sh");
    assert_eq!(terminal["command"], json!(["/bin/sh"]));
    assert_eq!(terminal["cwd"], root.to_string_lossy().to_string());
    assert_eq!(terminal["cols"], 120);
    assert_eq!(terminal["rows"], 40);
    assert_eq!(terminal["exit"], Value::Null);
    let process_id = terminal["processId"].as_u64().unwrap();
    let tracked = started["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|process| process["id"].as_u64() == Some(process_id))
        .unwrap();
    assert_eq!(tracked["kind"], "terminal:sh");
    assert_eq!(tracked["state"], "running");
    assert_eq!(tracked["workspaceId"], started["workspace"]["id"]);

    host.dispatch(
        "terminal.write",
        json!({ "data": "pwd; echo terminal-$((6*7))\r" }),
    )
    .unwrap();
    let log = wait_for_terminal_output(&host, "terminal-42", Duration::from_secs(10));
    assert!(
        log.contains(&root.to_string_lossy().to_string()),
        "pwd must print the workspace root: {log}"
    );

    // Ctrl+C is byte 0x03 to the PTY: the foreground child dies and the prompt comes back.
    let began = Instant::now();
    host.dispatch(
        "terminal.write",
        json!({ "data": "sleep 30 && echo slept\r" }),
    )
    .unwrap();
    std::thread::sleep(Duration::from_millis(500));
    host.dispatch("terminal.write", json!({ "data": "\u{3}" }))
        .unwrap();
    host.dispatch("terminal.write", json!({ "data": "echo prompt-back\r" }))
        .unwrap();
    let log = wait_for_terminal_output(&host, "prompt-back", Duration::from_secs(10));
    assert!(
        began.elapsed() < Duration::from_secs(20),
        "Ctrl+C did not interrupt sleep"
    );
    // The typed line is echoed once; a second occurrence would be the finished command.
    assert_eq!(
        log.matches("slept").count(),
        1,
        "the interrupted command must not finish: {log}"
    );
    assert_eq!(snapshot(&host)["terminal"]["state"], "running");

    host.dispatch("terminal.resize", json!({ "cols": 100, "rows": 30 }))
        .unwrap();
    let resized = snapshot(&host);
    assert_eq!(resized["terminal"]["cols"], 100);
    assert_eq!(resized["terminal"]["rows"], 30);

    // A foreground job is a real child of the shell; closing the terminal must reach it.
    host.dispatch(
        "terminal.write",
        json!({ "data": "sh -c 'echo sleep-pid=$$; exec sleep 30'\r" }),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let sleep_pid = loop {
        let log = snapshot(&host)["terminal"]["log"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        if let Some(pid) = log.lines().find_map(|line| {
            line.trim()
                .strip_prefix("sleep-pid=")
                .filter(|pid| !pid.is_empty() && pid.chars().all(|c| c.is_ascii_digit()))
        }) {
            break pid.to_string();
        }
        assert!(
            Instant::now() < deadline,
            "the foreground child never printed its pid: {log}"
        );
        std::thread::sleep(Duration::from_millis(50));
    };
    std::thread::sleep(Duration::from_millis(300));
    assert!(!process_gone(&sleep_pid), "the foreground child is running");

    // Closing needs the explicit decision; the observed result and the log stay.
    assert_eq!(
        error_code(host.dispatch("terminal.stop", json!({}))),
        "confirmation_required"
    );
    assert_eq!(snapshot(&host)["terminal"]["state"], "running");
    let stopped = host
        .dispatch("terminal.stop", json!({ "confirm": true }))
        .unwrap();
    let terminal = &stopped["terminal"];
    assert_eq!(terminal["state"], "stopped");
    assert_eq!(terminal["termination"]["observed"], "exited");
    assert!(terminal["termination"]["requested"]
        .as_str()
        .unwrap()
        .contains("SIGTERM"));
    assert!(terminal["log"].as_str().unwrap().contains("prompt-back"));
    let deadline = Instant::now() + Duration::from_secs(5);
    while !process_gone(&sleep_pid) {
        assert!(
            Instant::now() < deadline,
            "the foreground child survived the close: {sleep_pid}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    let process = stopped["processes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|process| process["id"].as_u64() == Some(process_id))
        .unwrap();
    assert_eq!(process["state"], "exited");
    assert_eq!(terminal["exit"], process["exit"]);
    assert_eq!(
        error_code(host.dispatch("terminal.stop", json!({ "confirm": true }))),
        "invalid_state"
    );

    host.dispatch("terminal.write", json!({ "data": "echo late\r" }))
        .expect_err("a stopped shell accepts no input");
}

#[cfg(unix)]
#[test]
fn terminal_read_returns_raw_pty_bytes_by_offset_and_keeps_them_after_seal() {
    let host = support::host();
    let workspace = temp_workspace(&["Raw.csproj"]);
    open(&host, &workspace.path().join("Raw.csproj"));
    grant(&host);
    let _cleanup = TerminalCleanup(&host);
    host.dispatch("terminal.start", json!({})).unwrap();
    host.dispatch(
        "terminal.write",
        json!({ "data": "printf '\\033[31mraw-marker\\033[0m\\n'\r" }),
    )
    .unwrap();

    let read = |since: u64| {
        let value = host
            .dispatch("terminal.read", json!({ "since": since }))
            .unwrap();
        let chunk = value["chunk"].clone();
        let bytes: Vec<u8> = chunk["bytes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|byte| byte.as_u64().unwrap() as u8)
            .collect();
        (
            value["revision"].as_u64().unwrap(),
            chunk["from"].as_u64().unwrap(),
            chunk["end"].as_u64().unwrap(),
            bytes,
        )
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    let (revision, mut from, mut end, mut seen) = read(0);
    // The typed command echoes "raw-marker" too; wait for the ESC-prefixed output the shell printed.
    while !seen
        .windows(b"\x1b[31mraw-marker".len())
        .any(|window| window == b"\x1b[31mraw-marker")
    {
        assert!(
            std::time::Instant::now() < deadline,
            "no raw output within 10s"
        );
        std::thread::sleep(Duration::from_millis(50));
        (_, from, end, seen) = read(0);
    }
    assert!(revision > 0);
    assert_eq!(from, 0);
    assert!(
        seen.contains(&0x1b),
        "ESC bytes must reach the client unfiltered"
    );

    let (_, next_from, next_end, empty) = read(end);
    assert_eq!((next_from, next_end), (end, end));
    assert!(empty.is_empty());

    host.dispatch("trust.revoke", json!({})).unwrap();
    // Shell output may still arrive while the seal lands; what was read before must be kept as is.
    let (_, _, sealed_end, kept) = read(0);
    assert!(sealed_end >= end);
    assert!(kept.starts_with(&seen));
}

#[cfg(unix)]
#[test]
fn revoking_trust_stops_and_seals_the_real_shell_and_switching_workspaces_drops_it() {
    let host = support::host();
    let first = temp_workspace(&["A.csproj"]);
    let second = temp_workspace(&["B.csproj"]);
    open(&host, &first.path().join("A.csproj"));
    grant(&host);
    let _cleanup = TerminalCleanup(&host);
    host.dispatch("terminal.start", json!({})).unwrap();
    host.dispatch("terminal.write", json!({ "data": "echo sealed-marker\r" }))
        .unwrap();
    wait_for_terminal_output(&host, "sealed-marker", Duration::from_secs(10));

    let revoked = host.dispatch("trust.revoke", json!({})).unwrap();
    let terminal = &revoked["terminal"];
    assert_eq!(terminal["sealed"], true);
    assert_eq!(terminal["state"], "stopped");
    assert!(terminal["staleReason"].as_str().unwrap().contains("Trust"));
    assert!(terminal["log"].as_str().unwrap().contains("sealed-marker"));
    let finished_at = terminal["finishedAt"].clone();
    std::thread::sleep(Duration::from_millis(300));
    let later = snapshot(&host);
    assert_eq!(later["terminal"]["sealed"], true);
    assert_eq!(later["terminal"]["finishedAt"], finished_at);
    assert_eq!(
        error_code(host.dispatch("terminal.start", json!({}))),
        "trust_required"
    );

    // Switching workspaces starts from an idle record; the old shell stays in the process
    // panel with the termination the registry observed.
    grant(&host);
    host.dispatch("terminal.start", json!({})).unwrap();
    let switched = host
        .dispatch(
            "workspace.open",
            json!({ "path": second.path().join("B.csproj") }),
        )
        .unwrap();
    assert_eq!(switched["terminal"]["state"], "idle");
    assert_eq!(switched["terminal"]["revision"], 0);
    assert!(switched["processes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|process| process["kind"]
            .as_str()
            .unwrap_or_default()
            .starts_with("terminal:"))
        .all(|process| process["state"] == "exited"));
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn settings_roundtrip_needs_no_trust_and_rejects_invalid_payloads_isolated() {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let host = support::host();

    // No workspace and no trust: settings are app-scoped, not workspace-scoped.
    let empty = host.dispatch("settings.read", json!({})).unwrap();
    assert_eq!(empty["settings"], json!({}));
    assert_eq!(
        empty["path"],
        home.path().join("settings.json").to_string_lossy().as_ref()
    );

    let settings = json!({
        "keymap": {
            "profile": "intellij",
            "overrides": { "run.start": [{ "key": "f5", "control": true }] }
        },
        "appearance": { "zoom": 110, "density": "compact" },
        "editor": { "fontFamily": "Menlo", "fontSize": 14, "indentSize": 2, "useTabs": true }
    });
    let written = host
        .dispatch("settings.write", json!({ "settings": settings }))
        .unwrap();
    assert_eq!(written["settings"], settings);
    let text = fs::read_to_string(home.path().join("settings.json")).unwrap();
    assert!(serde_json::from_str::<Value>(&text).is_ok());
    assert_eq!(
        host.dispatch("settings.read", json!({})).unwrap()["settings"],
        settings
    );

    for invalid in [
        json!({ "settings": [] }),
        json!({ "settings": { "keymap": { "profile": "vscode" } } }),
        json!({ "settings": { "appearance": { "zoom": 300 } } }),
        json!({ "settings": { "editor": { "indentSize": 0 } } }),
    ] {
        assert_eq!(
            error_code(host.dispatch("settings.write", invalid)),
            "invalid_settings"
        );
    }
    assert_eq!(
        host.dispatch("settings.read", json!({})).unwrap()["settings"],
        settings,
        "an invalid payload replaced the previous settings"
    );
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn run_configs_are_stored_per_solution_in_the_app_home_and_need_no_trust_isolated() {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let workspaces = tempfile::tempdir().unwrap();
    let solution =
        fs::canonicalize(fixture_copy(workspaces.path(), "Configs").join("OrderDesk.sln")).unwrap();
    let other = fixture_copy(workspaces.path(), "Other");
    let host = support::host();

    // No workspace and no trust: the store is app-scoped but keyed by the solution.
    let empty = host
        .dispatch("runConfigs.read", json!({ "solution": solution }))
        .unwrap();
    assert_eq!(empty["configs"], json!([]));
    assert_eq!(empty["activeId"], "default");
    assert!(empty["path"]
        .as_str()
        .unwrap()
        .starts_with(home.path().join("run-configs").to_string_lossy().as_ref()));
    assert!(!empty["path"]
        .as_str()
        .unwrap()
        .starts_with(fixture().to_string_lossy().as_ref()));

    fs::write(
        solution.parent().unwrap().join("local.env"),
        "OPENRIDR_F4=file\n",
    )
    .unwrap();
    let configs = json!([{
        "id": "cfg-1",
        "name": "Local run",
        "project": "src/OrderDesk.Cli/OrderDesk.Cli.csproj",
        "configuration": "Debug",
        "targetFramework": "net8.0",
        "args": ["--first"],
        "workingDirectory": "src",
        "env": { "OPENRIDR_F4": "explicit" },
        "envFiles": ["local.env"]
    }]);
    let written = host
        .dispatch(
            "runConfigs.write",
            json!({ "solution": solution, "activeId": "cfg-1", "configs": configs }),
        )
        .unwrap();
    assert_eq!(written["activeId"], "cfg-1");
    assert_eq!(written["configs"], configs);
    assert_eq!(written["issues"], json!([]));
    let path = written["path"].as_str().unwrap().to_string();
    assert!(Path::new(&path).is_file());
    let stored: Value = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(stored["solution"], solution.to_string_lossy().as_ref());

    // Another solution never sees this store; the file is not written inside the repository.
    let separate = host
        .dispatch(
            "runConfigs.read",
            json!({ "solution": other.join("OrderDesk.sln") }),
        )
        .unwrap();
    assert_eq!(separate["configs"], json!([]));
    assert_ne!(separate["path"], written["path"]);
    assert!(fs::read_dir(fixture()).unwrap().all(|entry| {
        let name = entry.unwrap().file_name();
        name != "run-configs" && !name.to_string_lossy().ends_with(".run-config.json")
    }));

    // An invalid shape is refused with its field and leaves the saved store untouched.
    let before = fs::read_to_string(&path).unwrap();
    for (payload, field) in [
        (
            json!([{ "id": "cfg-1", "name": "", "project": "a.csproj", "configuration": "Debug" }]),
            "name",
        ),
        (
            json!([{ "id": "cfg-1", "name": "One", "project": "a.csproj", "configuration": "Debug", "env": { "BAD KEY": "x" } }]),
            "env",
        ),
        (
            json!([
                { "id": "cfg-1", "name": "One", "project": "a.csproj", "configuration": "Debug" },
                { "id": "cfg-1", "name": "Two", "project": "a.csproj", "configuration": "Debug" }
            ]),
            "id",
        ),
    ] {
        let error = host
            .dispatch(
                "runConfigs.write",
                json!({ "solution": solution, "activeId": "default", "configs": payload }),
            )
            .expect_err("invalid store");
        assert_eq!(error.code, "invalid_run_configs", "{error:?}");
        assert_eq!(error.details.unwrap()["field"], field);
    }
    assert_eq!(fs::read_to_string(&path).unwrap(), before);

    // Missing files are reported as issues on save instead of blocking the edit.
    let issues = host
        .dispatch(
            "runConfigs.write",
            json!({
                "solution": solution,
                "activeId": "cfg-1",
                "configs": [{
                    "id": "cfg-1",
                    "name": "Local run",
                    "project": "src/OrderDesk.Cli/OrderDesk.Cli.csproj",
                    "configuration": "Debug",
                    "workingDirectory": "missing",
                    "envFiles": ["missing.env"]
                }]
            }),
        )
        .unwrap();
    let issues = issues["issues"].as_array().unwrap();
    assert_eq!(issues.len(), 2);
    assert!(issues
        .iter()
        .any(|issue| issue["field"] == "workingDirectory"));
    assert!(issues.iter().any(|issue| issue["field"] == "envFiles"));
}

#[test]
fn run_and_debug_validate_launch_options_before_any_process() {
    let temp = temp_workspace(&["App.csproj"]);
    fs::write(temp.path().join("bad.env"), "NOT A LINE\n").unwrap();
    let host = support::host();
    open(&host, &temp.path().join("App.csproj"));
    grant(&host);

    let missing = host
        .dispatch("run.start", json!({ "workingDirectory": "missing" }))
        .expect_err("a missing cwd is refused before build/restore");
    assert_eq!(missing.code, "invalid_state");
    assert_eq!(missing.details.unwrap()["field"], "workingDirectory");

    let bad_env = host
        .dispatch("run.start", json!({ "envFiles": ["bad.env"] }))
        .expect_err("an invalid .env line is refused");
    assert_eq!(bad_env.code, "invalid_state");
    let details = bad_env.details.unwrap();
    assert_eq!(details["field"], "envFiles");
    assert!(bad_env.message.contains("line 1"), "{}", bad_env.message);

    let debug_missing = host
        .dispatch("debug.start", json!({ "workingDirectory": "missing" }))
        .expect_err("debug validates the same options");
    assert_eq!(debug_missing.code, "invalid_state");
    assert_eq!(debug_missing.details.unwrap()["field"], "workingDirectory");

    let state = snapshot(&host);
    assert_eq!(state["run"]["state"], "idle");
    assert!(state["processes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|process| process["state"] == "exited"));
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn remembered_trust_survives_a_new_host_and_revoke_forgets_it_isolated() {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _store = EnvVar::remove("OPENRIDR_TRUST_STORE");
    let workspace = temp_workspace(&["Remember.csproj"]);
    let project = workspace.path().join("Remember.csproj");
    let granted = |host: &Host| {
        host.dispatch("workspace.state", json!({})).unwrap()["trust"]["granted"] == true
    };

    // A grant without `remember` is session-only.
    let first = support::host();
    open(&first, &project);
    first.dispatch("trust.grant", json!({})).unwrap();
    let second = support::host();
    open(&second, &project);
    assert!(!granted(&second));

    // `remember` persists per path; a new host (app restart) opens it trusted, other paths do not.
    second
        .dispatch("trust.grant", json!({ "remember": true }))
        .unwrap();
    let third = support::host();
    open(&third, &project);
    assert!(granted(&third));
    let other = temp_workspace(&["Other.csproj"]);
    open(&third, &other.path().join("Other.csproj"));
    assert!(!granted(&third));

    // Revoking forgets the decision.
    open(&third, &project);
    third.dispatch("trust.revoke", json!({})).unwrap();
    let fourth = support::host();
    open(&fourth, &project);
    assert!(!granted(&fourth));
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn trust_decision_persists_per_project_and_gates_workspace_open_isolated() {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _trust = EnvVar::remove("OPENRIDR_TRUST_STORE");
    let workspace = temp_workspace(&["App.csproj"]);
    let path = fs::canonicalize(workspace.path()).unwrap();
    let host = support::host();

    // Never decided: opens restricted with a null decision.
    let opened = open(&host, &path);
    assert_eq!(opened["trust"]["decision"], Value::Null);
    assert_eq!(opened["trust"]["granted"], false);
    let key = opened["workspace"]["path"].as_str().unwrap().to_string();
    assert_eq!(
        host.dispatch("trust.get", json!({ "path": key })).unwrap()["decision"],
        Value::Null
    );

    // set(trusted) on the open workspace applies trust and survives a reopen.
    let granted = host
        .dispatch("trust.set", json!({ "path": key, "decision": "trusted" }))
        .unwrap();
    assert_eq!(granted["trust"]["granted"], true);
    assert_eq!(granted["trust"]["decision"], "trusted");
    let reopened = open(&host, &path);
    assert_eq!(reopened["trust"]["granted"], true);

    // revoke records `restricted`, which is remembered but does not trust on open.
    let revoked = host.dispatch("trust.revoke", json!({})).unwrap();
    assert_eq!(revoked["trust"]["granted"], false);
    assert_eq!(revoked["trust"]["decision"], "restricted");
    let reopened = open(&host, &path);
    assert_eq!(reopened["trust"]["granted"], false);
    assert_eq!(reopened["trust"]["decision"], "restricted");

    // set for a path that is not open only persists; with no workspace open it is welcome-screen state.
    let other = temp_workspace(&["Other.csproj"]);
    let other_path = fs::canonicalize(other.path()).unwrap();
    host.dispatch("workspace.close", json!({})).unwrap();
    let stored = host
        .dispatch(
            "trust.set",
            json!({ "path": other_path, "decision": "trusted" }),
        )
        .unwrap();
    assert_eq!(stored["decision"], "trusted");
    assert_eq!(reopened["trust"]["granted"], false);
    assert_eq!(
        error_code(host.dispatch(
            "trust.set",
            json!({ "path": other_path, "decision": "maybe" })
        )),
        "invalid_params"
    );
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn recent_list_carries_decision_and_remove_keeps_it_isolated() {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _trust = EnvVar::remove("OPENRIDR_TRUST_STORE");
    let workspace = temp_workspace(&["App.csproj"]);
    let path = fs::canonicalize(workspace.path()).unwrap();
    let host = support::host();
    let key = open(&host, &path)["workspace"]["path"]
        .as_str()
        .unwrap()
        .to_string();
    host.dispatch("trust.set", json!({ "path": key, "decision": "trusted" }))
        .unwrap();

    let listed = host.dispatch("recent.list", json!({})).unwrap();
    assert_eq!(listed["projects"][0]["path"], key.as_str());
    assert_eq!(listed["projects"][0]["decision"], "trusted");
    assert!(listed["projects"][0]["openedAt"].as_u64().is_some());

    let after = host
        .dispatch("recent.remove", json!({ "path": key }))
        .unwrap();
    assert_eq!(after["projects"].as_array().unwrap().len(), 0);
    assert_eq!(
        host.dispatch("trust.get", json!({ "path": key })).unwrap()["decision"],
        "trusted"
    );
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn undecided_open_leaves_no_trace_until_decided_isolated() {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _trust = EnvVar::remove("OPENRIDR_TRUST_STORE");
    let workspace = temp_workspace(&["App.csproj"]);
    let host = support::host();
    let key = open(&host, &fs::canonicalize(workspace.path()).unwrap())["workspace"]["path"]
        .as_str()
        .unwrap()
        .to_string();

    // Open + close (Cancel) is not in Recents and has no stored decision.
    host.dispatch("workspace.close", json!({})).unwrap();
    assert_eq!(
        host.dispatch("recent.list", json!({})).unwrap()["projects"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        host.dispatch("trust.get", json!({ "path": key })).unwrap()["decision"],
        Value::Null
    );

    // Deciding for the open workspace records it.
    open(&host, &fs::canonicalize(workspace.path()).unwrap());
    host.dispatch("trust.revoke", json!({})).unwrap();
    assert_eq!(
        host.dispatch("recent.list", json!({})).unwrap()["projects"][0]["path"],
        key.as_str()
    );
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn trust_set_rejects_missing_paths_and_reports_unsaved_decisions_isolated() {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _trust = EnvVar::remove("OPENRIDR_TRUST_STORE");
    let host = support::host();
    let missing = home.path().join("does-not-exist");
    assert_eq!(
        error_code(host.dispatch(
            "trust.set",
            json!({ "path": missing, "decision": "trusted" })
        )),
        "not_found"
    );
    // A vanished project can still be forgotten from Recents.
    host.dispatch("recent.remove", json!({ "path": missing }))
        .unwrap();

    // An unreadable trusted.json is never overwritten and the failure reaches the caller.
    fs::write(home.path().join("trusted.json"), "{ not json").unwrap();
    let workspace = temp_workspace(&["App.csproj"]);
    let path = fs::canonicalize(workspace.path()).unwrap();
    let opened = open(&host, &path);
    let open_key = opened["workspace"]["path"].as_str().unwrap();
    assert_eq!(
        error_code(host.dispatch(
            "trust.set",
            json!({ "path": open_key, "decision": "trusted" })
        )),
        "trusted_unreadable"
    );
    assert_eq!(
        fs::read_to_string(home.path().join("trusted.json")).unwrap(),
        "{ not json"
    );
}

/// The ambient purge runs once per binary, so the only honest way to test it is a fresh process:
/// an exported bundled adapter must not reach a host whose test never asked for one.
#[test]
fn an_ambient_bundled_override_never_reaches_the_host() {
    let bundle = tempfile::tempdir().unwrap();
    let adapter = bundle.path().join("netcoredbg");
    fs::write(&adapter, "#!/bin/sh\nprintf 'netcoredbg 3.2.0-1092\\n'\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&adapter, fs::Permissions::from_mode(0o755)).unwrap();
    }

    support::assert_leaf_passes_in_a_fresh_process(
        "trust_grant_needs_an_open_workspace_and_gates_every_direct_call",
        &[("OPENRIDR_BUNDLED_NETCOREDBG", bundle.path().as_os_str())],
    );
}

// Each leaf below writes `OPENRIDR_*` for its host, so it runs in its own process: the
// shared test process never changes its environment.
#[test]
fn tools_plan_cancel_is_inert_and_install_needs_trust_and_confirmation() {
    support::run_isolated_leaf(
        "tools_plan_cancel_is_inert_and_install_needs_trust_and_confirmation_isolated",
    );
}

#[test]
fn tools_install_is_real_verified_and_reused() {
    support::run_isolated_leaf("tools_install_is_real_verified_and_reused_isolated");
}

#[cfg(unix)]
#[test]
fn setup_starts_before_trust_without_running_project_code() {
    support::run_isolated_leaf("setup_starts_before_trust_without_running_project_code_isolated");
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn setup_reuses_the_verified_installation_and_never_asks_again() {
    support::run_isolated_leaf(
        "setup_reuses_the_verified_installation_and_never_asks_again_isolated",
    );
}

#[cfg(unix)]
#[test]
fn setup_reports_a_missing_bundled_adapter_and_keeps_trust_closed() {
    support::run_isolated_leaf(
        "setup_reports_a_missing_bundled_adapter_and_keeps_trust_closed_isolated",
    );
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn failed_reinstall_keeps_the_previous_verified_tool_ready() {
    support::run_isolated_leaf("failed_reinstall_keeps_the_previous_verified_tool_ready_isolated");
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn a_reinstall_survives_a_workspace_switch_and_never_loses_both_copies() {
    support::run_isolated_leaf(
        "a_reinstall_survives_a_workspace_switch_and_never_loses_both_copies_isolated",
    );
}

#[cfg(unix)]
#[test]
fn tools_download_does_not_block_host_calls() {
    support::run_isolated_leaf("tools_download_does_not_block_host_calls_isolated");
}

#[cfg(all(unix, debug_assertions))]
#[test]
fn a_download_without_content_length_keeps_the_measured_bytes() {
    support::run_isolated_leaf(
        "a_download_without_content_length_keeps_the_measured_bytes_isolated",
    );
}

#[cfg(unix)]
#[test]
fn tools_failed_install_is_not_ready_and_retry_starts_from_clean_staging() {
    support::run_isolated_leaf(
        "tools_failed_install_is_not_ready_and_retry_starts_from_clean_staging_isolated",
    );
}

#[cfg(target_os = "macos")]
#[test]
fn hostroot_composes_runtime10_with_project_sdk_outside_the_repo_and_is_idempotent() {
    support::run_isolated_leaf(
        "hostroot_composes_runtime10_with_project_sdk_outside_the_repo_and_is_idempotent_isolated",
    );
}

#[test]
fn lsp_start_requires_tool_sdk_and_current_restore() {
    support::run_isolated_leaf("lsp_start_requires_tool_sdk_and_current_restore_isolated");
}

#[cfg(target_os = "macos")]
#[test]
fn lsp_loads_the_real_solution_republishes_diagnostics_and_stops_on_revoke() {
    support::run_isolated_leaf(
        "lsp_loads_the_real_solution_republishes_diagnostics_and_stops_on_revoke_isolated",
    );
}

#[cfg(unix)]
#[test]
fn initialize_failure_surfaces_the_sdk_message_and_kills_the_process() {
    support::run_isolated_leaf(
        "initialize_failure_surfaces_the_sdk_message_and_kills_the_process_isolated",
    );
}

#[cfg(unix)]
#[test]
fn cancelling_a_running_install_reports_termination_and_never_ready() {
    support::run_isolated_leaf(
        "cancelling_a_running_install_reports_termination_and_never_ready_isolated",
    );
}

#[cfg(target_os = "macos")]
#[test]
fn stopping_a_debug_session_waiting_for_its_build_stops_the_build() {
    support::run_isolated_leaf(
        "stopping_a_debug_session_waiting_for_its_build_stops_the_build_isolated",
    );
}

#[test]
fn settings_roundtrip_needs_no_trust_and_rejects_invalid_payloads() {
    support::run_isolated_leaf(
        "settings_roundtrip_needs_no_trust_and_rejects_invalid_payloads_isolated",
    );
}

#[test]
fn run_configs_are_stored_per_solution_in_the_app_home_and_need_no_trust() {
    support::run_isolated_leaf(
        "run_configs_are_stored_per_solution_in_the_app_home_and_need_no_trust_isolated",
    );
}

#[test]
fn remembered_trust_survives_a_new_host_and_revoke_forgets_it() {
    support::run_isolated_leaf(
        "remembered_trust_survives_a_new_host_and_revoke_forgets_it_isolated",
    );
}

#[test]
fn trust_decision_persists_per_project_and_gates_workspace_open() {
    support::run_isolated_leaf(
        "trust_decision_persists_per_project_and_gates_workspace_open_isolated",
    );
}

#[test]
fn recent_list_carries_decision_and_remove_keeps_it() {
    support::run_isolated_leaf("recent_list_carries_decision_and_remove_keeps_it_isolated");
}

#[test]
fn undecided_open_leaves_no_trace_until_decided() {
    support::run_isolated_leaf("undecided_open_leaves_no_trace_until_decided_isolated");
}

#[test]
fn trust_set_rejects_missing_paths_and_reports_unsaved_decisions() {
    support::run_isolated_leaf(
        "trust_set_rejects_missing_paths_and_reports_unsaved_decisions_isolated",
    );
}

#[cfg(target_os = "macos")]
#[test]
fn lsp_completion_and_definition_answer_from_the_real_session_and_reject_unknown_documents() {
    support::run_isolated_leaf("lsp_completion_and_definition_answer_from_the_real_session_and_reject_unknown_documents_isolated");
}

#[cfg(target_os = "macos")]
#[test]
fn debug_start_gates_the_experimental_flag_adapter_current_debug_build_and_sources() {
    support::run_isolated_leaf(
        "debug_start_gates_the_experimental_flag_adapter_current_debug_build_and_sources_isolated",
    );
}

#[cfg(target_os = "macos")]
#[test]
fn debug_implicitly_builds_and_starts_the_session_when_there_is_no_build() {
    support::run_isolated_leaf(
        "debug_implicitly_builds_and_starts_the_session_when_there_is_no_build_isolated",
    );
}

/// The test process is shared, so no test may write its environment: a test that needs different
/// values runs its body in its own process (`_isolated` leaf + `support::run_isolated_leaf`). This
/// guard reads the sources so a new write cannot quietly reintroduce the race the harness removed.
#[test]
fn env_writes_are_confined_to_isolated_leaves() {
    // Built at runtime so this test's own source does not match its needles.
    let writes = [
        format!("{}_var(", "set"),
        format!("{}_var(", "remove"),
        format!("{}::", "EnvVar"),
    ];
    let prepares = [
        "support::prepare(",
        "support::host(",
        "support::assert_leaf_passes_in_a_fresh_process(",
        "run_isolated_leaf(",
    ];
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut sources = Vec::new();
    collect_rust_sources(&root.join("src"), &mut sources);
    collect_rust_sources(&root.join("tests"), &mut sources);

    let mut offenders = Vec::new();
    let mut unprepared = Vec::new();
    let mut missing_ignore = Vec::new();
    let mut leaves = Vec::new();
    let mut wrappers = Vec::new();
    for path in &sources {
        let name = path.to_string_lossy().to_string();
        let is_harness = name.ends_with("test_harness.rs") || name.ends_with("support/mod.rs");
        let is_test_binary = name.contains("/tests/");
        let text = fs::read_to_string(path).unwrap();
        for function in source_functions(&text) {
            if function.name == "env_writes_are_confined_to_isolated_leaves" {
                continue;
            }
            // Split at the call, not at `run_isolated_leaf("`: rustfmt may put the leaf name on the
            // next line, and this guard must not depend on how the call is wrapped.
            if let Some(rest) = function.body.split("run_isolated_leaf(").nth(1) {
                if let Some(leaf) = rest.split('"').nth(1) {
                    // The wrapper may name the leaf by its module path (`trusted::tests::x_isolated`).
                    let name = leaf.rsplit("::").next().unwrap_or(leaf);
                    wrappers.push(name.to_string());
                }
            }
            let isolated = function.name.ends_with("_isolated");
            if isolated && function.is_test && !function.is_ignored {
                missing_ignore.push(format!("{name}::{}", function.name));
            }
            if isolated && function.is_test {
                leaves.push(function.name.clone());
            }
            if writes
                .iter()
                .any(|needle| function.body.contains(needle.as_str()))
                && !isolated
                && !is_harness
            {
                offenders.push(format!("{name}::{}", function.name));
            }
            if is_test_binary
                && function.is_test
                && !isolated
                && !prepares.iter().any(|needle| function.body.contains(needle))
            {
                unprepared.push(format!("{name}::{}", function.name));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "tests write the shared process environment; move the body to an `_isolated` leaf:\n{}",
        offenders.join("\n")
    );
    assert!(
        missing_ignore.is_empty(),
        "these isolated leaves must be `#[ignore]`d so the shared process never runs them: {missing_ignore:?}"
    );
    assert!(
        unprepared.is_empty(),
        "these tests touch the host or env without `support::prepare()`/`support::host()` first: {unprepared:?}"
    );
    let orphans: Vec<&String> = leaves
        .iter()
        .filter(|leaf| !wrappers.contains(leaf))
        .collect();
    assert!(
        orphans.is_empty(),
        "these isolated leaves have no `run_isolated_leaf` wrapper: {orphans:?}"
    );
}

fn collect_rust_sources(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_rust_sources(&path, found);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            found.push(path);
        }
    }
}

struct SourceFunction {
    name: String,
    body: String,
    is_test: bool,
    is_ignored: bool,
}

/// Attributes belong to the next `fn`, so they are buffered until one appears.
fn source_functions(text: &str) -> Vec<SourceFunction> {
    let mut functions = Vec::new();
    let mut pending: Vec<String> = Vec::new();
    let mut current: Option<(String, Vec<String>, String)> = None;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("#[") || trimmed.starts_with("///") {
            pending.push(trimmed.to_string());
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("fn ") {
            if let Some((name, attributes, body)) = current.take() {
                functions.push(SourceFunction {
                    name,
                    body,
                    is_test: attributes.iter().any(|line| line.starts_with("#[test]")),
                    is_ignored: attributes.iter().any(|line| line.starts_with("#[ignore")),
                });
            }
            current = Some((
                rest.split('(').next().unwrap_or("").to_string(),
                std::mem::take(&mut pending),
                String::new(),
            ));
            continue;
        }
        match current.as_mut() {
            Some((_, _, body)) => {
                body.push_str(line);
                body.push('\n');
            }
            None => pending.clear(),
        }
    }
    if let Some((name, attributes, body)) = current {
        functions.push(SourceFunction {
            name,
            body,
            is_test: attributes.iter().any(|line| line.starts_with("#[test]")),
            is_ignored: attributes.iter().any(|line| line.starts_with("#[ignore")),
        });
    }
    functions
}

#[cfg(unix)]
#[test]
fn a_real_pty_shell_reads_input_writes_output_and_ctrl_c_interrupts_the_child() {
    support::run_isolated_leaf(
        "a_real_pty_shell_reads_input_writes_output_and_ctrl_c_interrupts_the_child_isolated",
    );
}
