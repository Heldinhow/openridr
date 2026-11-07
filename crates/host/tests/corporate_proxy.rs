//! The NuGet Discover boundary through a corporate proxy, proved against a local proxy only.
//!
//! The environment of the shared test process never carries a proxy: the leaf that needs one runs
//! in its own process (`support::run_isolated_leaf`), so the rest of the suite keeps reaching the
//! destinations it means to reach.

mod support;

use openridr_host::Host;
use serde_json::json;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// A destination that is a public literal, so the guard needs no DNS to accept it and the test
/// stays offline. The proxy refuses every CONNECT, so nothing leaves this machine.
const PROXIED_SOURCE: &str = "https://1.1.1.1/v3/index.json";

struct LocalProxy {
    port: u16,
    hits: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl LocalProxy {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let hits = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread_hits = Arc::clone(&hits);
        let thread_stop = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming() {
                if thread_stop.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(mut stream) = stream else { continue };
                let mut buffer = [0u8; 4096];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                let first = request
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .trim_end()
                    .to_string();
                thread_hits.lock().unwrap().push(first);
                let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n");
                let _ = stream.flush();
            }
        });
        Self {
            port,
            hits,
            stop,
            handle: Some(handle),
        }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn label(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }

    fn hits(&self) -> Vec<String> {
        self.hits.lock().unwrap().clone()
    }
}

impl Drop for LocalProxy {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
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
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A solution whose only NuGet source is the public literal above, so Discover is accepted by the
/// source gate without any external configuration.
fn workspace(parent: &Path) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/OrderDesk");
    let copy = parent.join("OrderDesk");
    copy_tree(&source, &copy);
    fs::write(
        copy.join("NuGet.config"),
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<configuration>\n  <packageSources>\n    <clear />\n    <add key=\"corporate\" value=\"{PROXIED_SOURCE}\" />\n  </packageSources>\n</configuration>\n"
        ),
    )
    .unwrap();
    copy
}

fn dotnet_is_available() -> bool {
    std::process::Command::new("dotnet")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn open_and_trust(host: &Host, solution: &Path) {
    host.dispatch("workspace.open", json!({ "path": solution }))
        .expect("the workspace opens");
    host.allow_setup_for_tests();
    host.dispatch("trust.grant", json!({}))
        .expect("trust is granted");
}

#[test]
fn discover_through_a_local_proxy_reports_the_proxy() {
    support::run_isolated_leaf("discover_through_a_local_proxy_reports_the_proxy_isolated");
}

#[test]
#[ignore = "isolated leaf: it sets HTTPS_PROXY and OPENRIDR_* for its own host"]
fn discover_through_a_local_proxy_reports_the_proxy_isolated() {
    if !dotnet_is_available() {
        eprintln!("not verified: no dotnet on PATH, so the configured source cannot be listed");
        return;
    }
    let proxy = LocalProxy::start();
    let _proxy_env = support::EnvVar::set("HTTPS_PROXY", proxy.url());
    let host = support::host();
    let temp = tempfile::tempdir().unwrap();
    let copy = workspace(temp.path());
    open_and_trust(&host, &copy.join("OrderDesk.sln"));

    let result = host
        .dispatch(
            "nuget.search",
            json!({
                "project": "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
                "source": PROXIED_SOURCE,
                "query": "xunit"
            }),
        )
        .expect("a configured source is accepted");
    assert_eq!(result["state"], "failed", "{result}");
    let message = result["message"].as_str().unwrap_or_default().to_string();
    assert!(
        message.contains(&proxy.label()),
        "the failure must name the proxy: {message}"
    );
    let hits = proxy.hits();
    assert_eq!(
        hits.len(),
        1,
        "one request, and it went to the proxy: {hits:?}"
    );
    assert!(
        hits[0].starts_with("CONNECT 1.1.1.1:443"),
        "the proxy must be asked to reach the source: {hits:?}"
    );
}

/// A program that prints the `scutil --proxy` dictionary pointing at `proxy`. Pointing the seam at
/// it lets the product path read a "system setting" without touching the machine's network
/// configuration.
fn fake_scutil(proxy: &LocalProxy) -> (tempfile::TempDir, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let program = directory.path().join("fake-scutil");
    fs::write(
        &program,
        format!(
            "#!/bin/sh\nprintf '%s\\n' '<dictionary> {{' '  HTTPSEnable : 1' '  HTTPSPort : {}' '  HTTPSProxy : 127.0.0.1' '}}'\n",
            proxy.port
        ),
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    (directory, program)
}

#[test]
fn discover_through_the_system_proxy_reports_the_proxy() {
    support::run_isolated_leaf("discover_through_the_system_proxy_reports_the_proxy_isolated");
}

/// The issue in one test: no proxy variable anywhere, only the macOS setting, and Discover still
/// reaches the proxy instead of trying the destination directly.
#[test]
#[ignore = "isolated leaf: it points OPENRIDR_SYSTEM_PROXY at a fake scutil for its own host"]
fn discover_through_the_system_proxy_reports_the_proxy_isolated() {
    if !dotnet_is_available() {
        eprintln!("not verified: no dotnet on PATH, so the configured source cannot be listed");
        return;
    }
    let proxy = LocalProxy::start();
    let (_directory, program) = fake_scutil(&proxy);
    let host = support::host();
    // After `host()`: `prepare()` turns the source off for the whole suite, and this leaf is the one
    // that wants it on.
    let _seam = support::EnvVar::set("OPENRIDR_SYSTEM_PROXY", &program);
    let temp = tempfile::tempdir().unwrap();
    let copy = workspace(temp.path());
    open_and_trust(&host, &copy.join("OrderDesk.sln"));

    let result = host
        .dispatch(
            "nuget.search",
            json!({
                "project": "tests/OrderDesk.Tests/OrderDesk.Tests.csproj",
                "source": PROXIED_SOURCE,
                "query": "xunit"
            }),
        )
        .expect("a configured source is accepted");
    assert_eq!(result["state"], "failed", "{result}");
    let message = result["message"].as_str().unwrap_or_default().to_string();
    assert!(
        message.contains(&proxy.label()),
        "the failure must name the proxy from the system settings: {message}"
    );
    let hits = proxy.hits();
    assert_eq!(
        hits.len(),
        1,
        "one request, and it went to the proxy: {hits:?}"
    );
    assert!(
        hits[0].starts_with("CONNECT 1.1.1.1:443"),
        "the proxy must be asked to reach the source: {hits:?}"
    );
}
