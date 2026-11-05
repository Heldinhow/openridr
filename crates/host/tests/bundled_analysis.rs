use openridr_host::{bundled, sdk, Host};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The bundled-analysis tests share the app home and the two override variables with every
/// other tool/LSP test, so they hold the same lock and run one at a time.
static ENV_LOCK: Mutex<()> = Mutex::new(());

struct EnvVar {
    key: &'static str,
    old: Option<std::ffi::OsString>,
}

impl EnvVar {
    fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
        let old = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, old }
    }

    fn remove(key: &'static str) -> Self {
        let old = std::env::var_os(key);
        std::env::remove_var(key);
        Self { key, old }
    }
}

impl Drop for EnvVar {
    fn drop(&mut self) {
        match &self.old {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}

fn fixture() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/OrderDesk"))
        .expect("fixture OrderDesk exists")
}

fn open(host: &Host, path: &Path) -> Value {
    let opened = host
        .dispatch("workspace.open", json!({ "path": path }))
        .expect("workspace opens");
    host.allow_setup_for_tests();
    opened
}

fn snapshot(host: &Host) -> Value {
    host.dispatch("workspace.state", json!({})).unwrap()
}

#[cfg(unix)]
fn write_script(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// A bundle-shaped pair: `Resources/csharp-ls/CSharpLanguageServer.dll` next to a private
/// runtime root with a muxer that answers `--version` like the real one.
#[cfg(unix)]
fn fake_pair(root: &Path) -> (PathBuf, PathBuf) {
    let tools = root.join("csharp-ls");
    fs::create_dir_all(&tools).unwrap();
    fs::write(
        tools.join("CSharpLanguageServer.dll"),
        b"not a real assembly",
    )
    .unwrap();
    let runtime = root.join("dotnet");
    fs::create_dir_all(runtime.join("host/fxr/10.0.14")).unwrap();
    fs::create_dir_all(runtime.join("shared/Microsoft.NETCore.App/10.0.14")).unwrap();
    write_script(
        &runtime.join("dotnet"),
        "#!/bin/sh\nprintf 'csharp-ls, 0.28.0 (probe)\\n'\n",
    );
    (tools, runtime)
}

#[cfg(unix)]
#[test]
fn a_bundled_pair_is_ready_without_installing_or_downloading() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let (tools, runtime) = fake_pair(&temp.path().join("Resources"));
    let _home = EnvVar::set("OPENRIDR_HOME", &home);
    let _tool = EnvVar::set(bundled::CSHARP_LS_OVERRIDE, &tools);
    let _runtime = EnvVar::set(bundled::RUNTIME_OVERRIDE, &runtime);

    let host = Host::new();
    let opened = open(&host, &fixture());
    assert_eq!(opened["tools"]["csharp-ls"]["state"], "ready");

    let state = snapshot(&host);
    let tool = &state["tools"]["csharp-ls"];
    assert_eq!(tool["state"], "ready");
    assert_eq!(tool["bundled"], true);
    assert_eq!(
        tool["destination"].as_str(),
        Some(tools.to_string_lossy().as_ref())
    );
    assert_eq!(tool["verifiedVersion"], "0.28.0");
    assert_eq!(tool["plan"], Value::Null);
    assert_eq!(tool["progress"], Value::Null);
    assert_eq!(tool["sha256"].as_str().map(str::len), Some(64));
    let note = tool["note"].as_str().unwrap_or_default();
    assert!(note.contains("Bundled"), "{note}");
    assert!(
        !home.join("tools").exists(),
        "nothing may be installed into the app home"
    );

    // The bundle has no plan: every install path refuses instead of downloading.
    let refused = host
        .dispatch("tools.plan", json!({ "tool": "csharp-ls" }))
        .expect_err("a bundled tool has no plan");
    assert!(refused.message.contains("bundled"), "{}", refused.message);
    host.dispatch("trust.grant", json!({ "path": fixture() }))
        .or_else(|_| host.dispatch("trust.set", json!({ "path": fixture(), "granted": true })))
        .ok();
    let install = host
        .dispatch(
            "tools.install",
            json!({ "planId": "none", "confirm": true }),
        )
        .expect_err("install is refused when bundled");
    assert!(install.message.contains("bundled"), "{}", install.message);
    let retry = host
        .dispatch("tools.retry", json!({}))
        .expect_err("retry is refused when bundled");
    assert!(retry.message.contains("bundled"), "{}", retry.message);
    let reinstall = host
        .dispatch("setup.reinstall", json!({}))
        .expect_err("reinstall is refused when bundled");
    assert!(
        reinstall.message.contains("bundled"),
        "{}",
        reinstall.message
    );

    // The runtime view reports the bundle as the analysis runtime.
    assert_eq!(state["sdk"]["host"]["state"], "found");
    assert_eq!(state["sdk"]["host"]["bundled"], true);
    assert_eq!(state["sdk"]["host"]["version"], "10.0.14");
    assert_eq!(
        state["sdk"]["host"]["root"].as_str(),
        Some(runtime.to_string_lossy().as_ref())
    );
}

#[cfg(unix)]
#[test]
fn a_modified_bundled_entry_is_no_longer_ready() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let (tools, runtime) = fake_pair(&temp.path().join("Resources"));
    let _home = EnvVar::set("OPENRIDR_HOME", &home);
    let _tool = EnvVar::set(bundled::CSHARP_LS_OVERRIDE, &tools);
    let _runtime = EnvVar::set(bundled::RUNTIME_OVERRIDE, &runtime);

    let host = Host::new();
    open(&host, &fixture());
    assert_eq!(snapshot(&host)["tools"]["csharp-ls"]["state"], "ready");

    fs::write(tools.join(bundled::CSHARP_LS_ENTRY), b"tampered").unwrap();
    let state = snapshot(&host);
    let tool = &state["tools"]["csharp-ls"];
    assert_eq!(tool["state"], "failed", "{tool}");
    assert!(tool["error"]
        .as_str()
        .unwrap_or_default()
        .contains("changed"));
}

#[test]
fn analysis_runtime_prefers_the_bundle_and_falls_back_to_the_inventory() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    #[cfg(unix)]
    {
        let temp = tempfile::tempdir().unwrap();
        let (_tools, runtime) = fake_pair(&temp.path().join("Resources"));
        let _runtime = EnvVar::set(bundled::RUNTIME_OVERRIDE, &runtime);
        let bundled_runtime = sdk::AnalysisRuntime::for_analysis(&[]);
        assert_eq!(bundled_runtime.state, "found");
        assert_eq!(bundled_runtime.bundled, true);
        assert_eq!(bundled_runtime.version.as_deref(), Some("10.0.14"));
        assert_eq!(
            bundled_runtime.root.as_deref(),
            Some(runtime.to_string_lossy().as_ref())
        );
    }

    let _absent = EnvVar::remove(bundled::RUNTIME_OVERRIDE);
    let detected = sdk::AnalysisRuntime::from_runtimes(&sdk::inventory().runtimes);
    let preferred = sdk::AnalysisRuntime::for_analysis(&sdk::inventory().runtimes);
    assert_eq!(preferred.bundled, false);
    assert_eq!(preferred.state, detected.state);
    assert_eq!(preferred.version, detected.version);
}

#[cfg(target_os = "macos")]
#[test]
fn the_vendored_bundle_composes_the_project_sdk_with_its_private_runtime_10() {
    // Requires `sh scripts/vendor-analysis.sh`; without resources there is nothing to compose.
    let resources = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../src-tauri/resources");
    let runtime = resources.join("dotnet");
    if !runtime.join("dotnet").is_file() {
        eprintln!("skip: {runtime:?} is not vendored (run scripts/vendor-analysis.sh)");
        return;
    }
    let _guard = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
    let temp = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", temp.path());
    let _forced = EnvVar::remove("OPENRIDR_FORCE_HOSTROOT");
    let install = sdk::inventory()
        .installs
        .into_iter()
        .find(|install| install.version == "8.0.408")
        .expect("SDK 8.0.408 is installed for the fixture");
    let hostroot = openridr_host::hostroot::ensure(&runtime, &install).expect("hostroot composes");
    assert!(hostroot.check.contains("8.0.408"), "{}", hostroot.check);
    assert!(hostroot.check.contains("10.0.14"), "{}", hostroot.check);
    assert!(hostroot
        .path
        .starts_with(temp.path().to_string_lossy().as_ref()));
}
