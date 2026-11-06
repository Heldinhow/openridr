//! Boundary tests for the netcoredbg installer (ticket 13).
//!
//! The real asset of the approved release cannot be installed on this macOS arm64 host (the
//! only osx-arm64 build is community supported and unverified). These tests therefore use a
//! deterministic local HTTP server and a controlled zip whose executable is a shell script that
//! prints `NET Core debugger 3.2.0-1 (controlled test artifact)`. It is executed only here, to
//! exercise extraction and verification; it is not an official Samsung asset.

#[path = "support/mod.rs"]
mod support;

use openridr_host::{Host, EXECUTABLE_METHODS};
use serde_json::{json, Value};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use support::EnvVar;

const ARTIFACT: &[u8] =
    b"#!/bin/sh\nprintf 'NET Core debugger 3.2.0-1 (controlled test artifact, Release)\\n'\n";
const WRONG_VERSION: &[u8] =
    b"#!/bin/sh\nprintf 'NET Core debugger 3.1.1-1000 (controlled test artifact, Release)\\n'\n";

fn temp_workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join("Debug.csproj"),
        "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup></Project>\n",
    )
    .unwrap();
    fs::write(dir.path().join("Program.cs"), "class C {}\n").unwrap();
    dir
}

fn open(host: &Host, dir: &Path) -> Value {
    let opened = host
        .dispatch("workspace.open", json!({ "path": dir.join("Debug.csproj") }))
        .expect("workspace opens");
    host.allow_setup_for_tests();
    opened
}

fn state(host: &Host) -> Value {
    host.dispatch("debug.state", json!({})).unwrap()["debug"].clone()
}

fn error_code(result: Result<Value, openridr_host::HostError>) -> String {
    result.expect_err("expected host error").code
}

fn wait_for(host: &Host, states: &[&str], timeout: Duration) -> Value {
    let deadline = Instant::now() + timeout;
    loop {
        let debug = state(host);
        if states.contains(&debug["adapterState"].as_str().unwrap_or("")) {
            return debug;
        }
        assert!(
            Instant::now() < deadline,
            "netcoredbg never reached {states:?}: state {} log {}",
            debug["adapterState"],
            debug["log"]
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

/// Minimal deterministic HTTP server: one route table, request counter and a clean shutdown.
struct Server {
    port: u16,
    requests: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl Server {
    fn start(routes: Vec<(&'static str, Route)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        listener.set_nonblocking(true).unwrap();
        let requests = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let counter = Arc::clone(&requests);
        let stopping = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            while !stopping.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        // On macOS the accepted socket inherits O_NONBLOCK from the listener;
                        // a blocking stream with timeouts keeps the harness deterministic.
                        let _ = stream.set_nonblocking(false);
                        let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                        let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
                        let mut buffer = [0u8; 2048];
                        let mut request = Vec::new();
                        loop {
                            match stream.read(&mut buffer) {
                                Ok(0) => break,
                                Ok(read) => {
                                    request.extend_from_slice(&buffer[..read]);
                                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                                        break;
                                    }
                                }
                                Err(_) => break,
                            }
                        }
                        let request = String::from_utf8_lossy(&request);
                        let path = request
                            .lines()
                            .next()
                            .and_then(|line| line.split_whitespace().nth(1))
                            .unwrap_or("/")
                            .to_string();
                        counter.fetch_add(1, Ordering::SeqCst);
                        let route = routes
                            .iter()
                            .find(|(name, _)| *name == path)
                            .map(|(_, route)| route);
                        match route {
                            Some(Route::Ok(body)) => {
                                let header = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                    body.len()
                                );
                                let _ = stream.write_all(header.as_bytes());
                                let _ = stream.write_all(body);
                            }
                            Some(Route::Slow(body)) => {
                                let header = format!(
                                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                    body.len()
                                );
                                let _ = stream.write_all(header.as_bytes());
                                let half = body.len() / 2;
                                let _ = stream.write_all(&body[..half]);
                                let _ = stream.flush();
                                std::thread::sleep(Duration::from_millis(400));
                                let _ = stream.write_all(&body[half..]);
                            }
                            Some(Route::Redirect(location)) => {
                                let header = format!(
                                    "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                                );
                                let _ = stream.write_all(header.as_bytes());
                            }
                            Some(Route::FailOnce(body, fail)) => {
                                if fail.swap(false, Ordering::SeqCst) {
                                    let _ = stream.write_all(
                                        b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                                    );
                                } else {
                                    let header = format!(
                                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                                        body.len()
                                    );
                                    let _ = stream.write_all(header.as_bytes());
                                    let _ = stream.write_all(body);
                                }
                            }
                            None => {
                                let _ = stream
                                    .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                            }
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    Err(_) => break,
                }
            }
        });
        Self {
            port,
            requests,
            stop,
            handle: Some(handle),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}{path}", self.port)
    }

    fn requests(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

enum Route {
    Ok(Vec<u8>),
    Slow(Vec<u8>),
    Redirect(&'static str),
    /// Answers 500 until the flag is taken, then serves the body; one connection per request.
    FailOnce(Vec<u8>, Arc<AtomicBool>),
}

/// Builds a zip with explicit names and unix modes, so traversal and symlink entries can be
/// expressed exactly as a hostile archive would.
fn zip_bytes(entries: &[(&str, &[u8], Option<u32>)]) -> Vec<u8> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buffer);
        for (name, data, mode) in entries {
            let mut options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            if let Some(mode) = mode {
                options = options.unix_permissions(*mode);
            }
            writer.start_file(*name, options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }
    buffer.into_inner()
}


/// A single stored entry with an explicit external mode. The zip crate normalizes permissions,
/// so a hostile symlink/absolute entry is expressed byte by byte here.
fn raw_zip(name: &str, data: &[u8], mode: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let name = name.as_bytes();
    let size = data.len() as u32;
    // Local file header, stored (method 0), CRC left 0: the mode is rejected before any read.
    out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
    out.extend_from_slice(&20u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(name);
    out.extend_from_slice(data);
    let central_offset = out.len() as u32;
    out.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
    out.extend_from_slice(&0x031eu16.to_le_bytes());
    out.extend_from_slice(&20u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&(name.len() as u16).to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(mode << 16).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(name);
    let central_size = out.len() as u32 - central_offset;
    out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&central_size.to_le_bytes());
    out.extend_from_slice(&central_offset.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

fn artifact_zip(executable: &[u8]) -> Vec<u8> {
    zip_bytes(&[
        ("netcoredbg.exe", executable, Some(0o755)),
        (
            "README.txt",
            b"controlled test artifact for ticket 13; not an official Samsung release\n",
            Some(0o644),
        ),
    ])
}

struct Seam {
    _home: EnvVar,
    _probe: EnvVar,
    _platform: EnvVar,
    _url: EnvVar,
    _sha: EnvVar,
    home: tempfile::TempDir,
}

impl Seam {
    /// Builds the controlled-installer environment. Only called from `_isolated` leaves, which own
    /// their process, so the env writes cannot reach another test.
    fn seam_isolated(url: &str, sha: &str, platform: &str) -> Self {
        let home = tempfile::tempdir().unwrap();
        Self {
            _home: EnvVar::set("OPENRIDR_HOME", home.path()),
            _probe: EnvVar::set("OPENRIDR_ENABLE_PROBE", "1"),
            _platform: EnvVar::set("OPENRIDR_DEBUG_PLATFORM", platform),
            _url: EnvVar::set("OPENRIDR_DEBUG_ASSET_URL", url),
            _sha: EnvVar::set("OPENRIDR_DEBUG_ASSET_SHA256", sha),
            home,
        }
    }
}

fn seam(url: &str, sha: &str) -> Seam {
    Seam::seam_isolated(url, sha, "windows/x86_64")
}

fn seam_platform(url: &str, sha: &str, platform: &str) -> Seam {
    Seam::seam_isolated(url, sha, platform)
}

fn tool_dir(home: &Path) -> PathBuf {
    home.join("tools/netcoredbg")
}

fn fake_netcoredbg(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(
        path,
        "#!/bin/sh\nprintf 'NET Core debugger 3.2.0-1 (controlled test artifact, Release)\\n'\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn macos_arm64_needs_the_experimental_flag_and_never_downloads_without_it_isolated() {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _probe = EnvVar::set("OPENRIDR_ENABLE_PROBE", "0");
    let host = support::host();
    let workspace = temp_workspace();
    let opened = open(&host, workspace.path());
    let debug = &opened["debug"];
    // The plan is read-only and allowed before trust.
    assert_eq!(debug["adapterState"], "experimental_disabled");
    assert_eq!(debug["appleSiliconExperimental"], false);
    assert_eq!(debug["version"], "3.2.0-1092");
    assert_eq!(
        debug["source"]["url"],
        "https://github.com/Samsung/netcoredbg/releases/tag/3.2.0-1092"
    );
    assert_eq!(
        debug["gateReason"].as_str().unwrap(),
        debug["adapterMessage"].as_str().unwrap()
    );
    assert!(debug["adapterMessage"]
        .as_str()
        .unwrap()
        .contains("osx-arm64 asset exists"));
    assert!(debug["adapterMessage"]
        .as_str()
        .unwrap()
        .contains("installation stays refused until you enable the experimental flag"));
    assert_eq!(debug["dapProven"], false);

    let planned = host.dispatch("debug.plan", json!({})).unwrap()["debug"].clone();
    assert_eq!(planned["platform"], "osx-arm64");
    assert_eq!(planned["asset"]["name"], "netcoredbg-osx-arm64.zip");
    assert_eq!(planned["availability"], "available");
    assert_eq!(planned["asset"]["support"], "community");
    assert_eq!(planned["plan"]["blockedCode"], "experimental_required");
    assert!(planned["plan"]["blockedReason"]
        .as_str()
        .unwrap()
        .contains("enable the experimental flag"));
    assert!(planned["plan"]["integrity"]
        .as_str()
        .unwrap()
        .contains("f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea"));
    assert!(planned["plan"]["destination"]
        .as_str()
        .unwrap()
        .ends_with("tools/netcoredbg/3.2.0-1092"));
    assert!(planned["plan"]["staging"]
        .as_str()
        .unwrap()
        .ends_with("3.2.0-1092.staging"));
    assert!(!tool_dir(home.path()).exists());

    // Trust + consent alone cannot install while the flag is off.
    host.dispatch("trust.grant", json!({})).unwrap();
    let plan_id = state(&host)["plan"]["planId"].as_str().unwrap().to_string();
    let refused = host
        .dispatch(
            "debug.install",
            json!({ "planId": plan_id, "confirm": true }),
        )
        .expect_err("the experimental flag is off");
    assert_eq!(refused.code, "experimental_required");
    assert!(refused.message.contains("Nothing was downloaded"));
    assert!(!tool_dir(home.path()).exists());
    assert_eq!(state(&host)["log"].as_str().unwrap().contains("attempt"), false);

    // The opt-in is independent of trust and installation; it replans, so the consent shown
    // afterwards is the one that authorizes the experimental install.
    assert_eq!(
        error_code(host.dispatch("debug.flag", json!({ "enabled": true }))),
        "confirmation_required"
    );
    let flagged = host
        .dispatch("debug.flag", json!({ "enabled": true, "confirm": true }))
        .unwrap();
    assert_eq!(flagged["debug"]["appleSiliconExperimental"], true);
    assert_eq!(flagged["debug"]["adapterState"], "planned");
    assert_eq!(flagged["debug"]["installable"], true);
    assert!(flagged["debug"]["plan"]["blockedReason"].is_null());
    assert!(flagged["debug"]["adapterMessage"]
        .as_str()
        .unwrap()
        .contains("available with explicit consent"));
    assert!(!tool_dir(home.path()).exists());
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn macos_arm64_installs_the_controlled_artifact_only_after_opt_in_isolated() {
    // The official macOS zip nests the executable and ships AppleDouble metadata.
    let zip = zip_bytes(&[
        ("netcoredbg/netcoredbg", ARTIFACT, Some(0o755)),
        ("__MACOSX/._netcoredbg", b"metadata", Some(0o644)),
    ]);
    let server = Server::start(vec![("/asset.zip", Route::Ok(zip.clone()))]);
    let seam = seam_platform(&server.url("/asset.zip"), &sha256(&zip), "macos/aarch64");
    let host = support::host();
    let workspace = temp_workspace();
    open(&host, workspace.path());
    host.dispatch("trust.grant", json!({})).unwrap();

    let blocked = host.dispatch("debug.plan", json!({})).unwrap()["debug"].clone();
    assert_eq!(blocked["plan"]["blockedCode"], "experimental_required");
    assert_eq!(server.requests(), 0);
    assert_eq!(
        error_code(host.dispatch(
            "debug.install",
            json!({ "planId": blocked["plan"]["planId"], "confirm": true })
        )),
        "experimental_required"
    );
    assert_eq!(server.requests(), 0);

    let flagged = host
        .dispatch("debug.flag", json!({ "enabled": true, "confirm": true }))
        .unwrap()["debug"]
        .clone();
    assert_eq!(flagged["adapterState"], "planned");
    assert_eq!(flagged["installable"], true);
    let plan_id = flagged["plan"]["planId"].as_str().unwrap().to_string();
    host.dispatch("debug.install", json!({ "planId": plan_id, "confirm": true }))
        .unwrap();

    let mut seen = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    let finished = loop {
        let debug = state(&host);
        let current = debug["adapterState"].as_str().unwrap().to_string();
        if seen.last() != Some(&current) {
            seen.push(current.clone());
        }
        if matches!(current.as_str(), "ready" | "failed" | "cancelled") {
            break debug;
        }
        assert!(Instant::now() < deadline, "install stuck: {}", debug["log"]);
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(finished["adapterState"], "ready", "{}", finished["log"]);
    assert_eq!(seen, ["downloading", "installing", "verifying", "ready"]);
    assert_eq!(finished["verifiedVersion"], "3.2.0");
    assert_eq!(finished["dapProven"], false);
    let destination = PathBuf::from(finished["destination"].as_str().unwrap());
    assert!(destination.join("netcoredbg/netcoredbg").is_file());
    assert!(!destination.join("__MACOSX").exists());
    assert!(destination.starts_with(seam.home.path()));
    // The recorded path must outlive promotion: the staging tree it was verified in is gone.
    let installed_binary = PathBuf::from(finished["installedBinary"].as_str().unwrap());
    assert_eq!(installed_binary, destination.join("netcoredbg/netcoredbg"));
    assert!(installed_binary.starts_with(&destination));
    assert!(!installed_binary.to_string_lossy().contains(".staging"));
    assert_eq!(server.requests(), 1);

    // Reuse after a successful install: the nested executable is found, nothing is downloaded.
    let planned = host.dispatch("debug.plan", json!({})).unwrap()["debug"].clone();
    assert_eq!(planned["adapterState"], "ready");
    assert_eq!(planned["plan"]["existing"]["state"], "present");
    let reused = host
        .dispatch(
            "debug.install",
            json!({ "planId": planned["plan"]["planId"], "confirm": true }),
        )
        .unwrap()["debug"]
        .clone();
    assert_eq!(reused["adapterState"], "ready");
    assert_eq!(reused["reused"], true);
    assert_eq!(reused["sha256"], finished["sha256"]);
    assert_eq!(server.requests(), 1, "reuse must not download");
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn install_requires_trust_confirmation_and_an_unchanged_plan_isolated() {
    let zip = artifact_zip(ARTIFACT);
    let server = Server::start(vec![("/asset.zip", Route::Ok(zip.clone()))]);
    let seam = seam(&server.url("/asset.zip"), &sha256(&zip));
    let host = support::host();
    let workspace = temp_workspace();
    open(&host, workspace.path());

    assert_eq!(
        error_code(host.dispatch("debug.install", json!({}))),
        "trust_required"
    );
    host.dispatch("trust.grant", json!({})).unwrap();
    let planned = host.dispatch("debug.plan", json!({})).unwrap()["debug"].clone();
    assert_eq!(planned["adapterState"], "planned");
    let plan = planned["plan"].clone();
    let plan_id = plan["planId"].as_str().unwrap().to_string();
    assert_eq!(plan["asset"]["testOverride"], true);
    assert_eq!(plan["os"], "windows");
    assert_eq!(plan["arch"], "x86_64");
    assert_eq!(server.requests(), 0, "planning must not download");

    assert_eq!(
        error_code(host.dispatch(
            "debug.install",
            json!({ "planId": plan_id, "confirm": false })
        )),
        "confirmation_required"
    );
    assert_eq!(
        error_code(host.dispatch(
            "debug.install",
            json!({ "planId": "fnv1a64:0000000000000000", "confirm": true })
        )),
        "confirmation_required"
    );
    assert_eq!(server.requests(), 0);
    assert!(!tool_dir(seam.home.path()).exists());
    assert_eq!(state(&host)["adapterState"], "planned");
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn a_changed_plan_after_confirmation_downloads_nothing_isolated() {
    let zip = artifact_zip(ARTIFACT);
    let server = Server::start(vec![("/asset.zip", Route::Ok(zip.clone()))]);
    let seam = seam(&server.url("/asset.zip"), &sha256(&zip));
    let host = support::host();
    let workspace = temp_workspace();
    open(&host, workspace.path());
    host.dispatch("trust.grant", json!({})).unwrap();
    let plan_id = host.dispatch("debug.plan", json!({})).unwrap()["debug"]["plan"]["planId"]
        .as_str()
        .unwrap()
        .to_string();

    // A material change between plan and confirm: the expected hash is different.
    std::env::set_var("OPENRIDR_DEBUG_ASSET_SHA256", "0".repeat(64));
    let changed = host
        .dispatch(
            "debug.install",
            json!({ "planId": plan_id, "confirm": true }),
        )
        .expect_err("the plan changed");
    assert_eq!(changed.code, "confirmation_required");
    assert!(changed.message.contains("Nothing was downloaded"));
    assert!(changed.details.unwrap()["plan"]["planId"].as_str().unwrap() != plan_id);
    assert_eq!(server.requests(), 0);
    assert!(!tool_dir(seam.home.path()).exists());
    assert_eq!(state(&host)["adapterState"], "planned");
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn a_controlled_install_reaches_ready_is_reused_and_records_no_dap_proof_isolated() {
    let zip = artifact_zip(ARTIFACT);
    let server = Server::start(vec![("/asset.zip", Route::Ok(zip.clone()))]);
    let seam = seam(&server.url("/asset.zip"), &sha256(&zip));
    let host = support::host();
    let workspace = temp_workspace();
    open(&host, workspace.path());
    host.dispatch("trust.grant", json!({})).unwrap();
    let plan_id = host.dispatch("debug.plan", json!({})).unwrap()["debug"]["plan"]["planId"]
        .as_str()
        .unwrap()
        .to_string();
    host.dispatch(
        "debug.install",
        json!({ "planId": plan_id, "confirm": true }),
    )
    .unwrap();

    let mut seen = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(30);
    let finished = loop {
        let debug = state(&host);
        let current = debug["adapterState"].as_str().unwrap().to_string();
        if seen.last() != Some(&current) {
            seen.push(current.clone());
        }
        if matches!(current.as_str(), "ready" | "failed" | "cancelled") {
            break debug;
        }
        assert!(Instant::now() < deadline, "install stuck: {}", debug["log"]);
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(finished["adapterState"], "ready", "{}", finished["log"]);
    eprintln!(
        "controlled netcoredbg install: states {seen:?}, verifiedVersion {:?}, sha256 {:?}, server requests {}",
        finished["verifiedVersion"], finished["sha256"], server.requests()
    );
    assert_eq!(
        seen,
        ["downloading", "installing", "verifying", "ready"],
        "states were not all observable: {seen:?}"
    );
    assert_eq!(finished["verifiedVersion"], "3.2.0");
    assert!(finished["verifiedOutput"]
        .as_str()
        .unwrap()
        .contains("controlled test artifact"));
    assert_eq!(finished["sha256"].as_str().unwrap().len(), 64);
    assert_eq!(finished["reused"], false);
    assert_eq!(finished["dapProven"], false);
    assert!(finished["log"]
        .as_str()
        .unwrap()
        .contains("sha256") );
    assert!(finished["log"].as_str().unwrap().contains("matched the plan"));
    assert_eq!(finished["destinationPresent"], true);
    assert_eq!(finished["stagingPresent"], false);
    let destination = PathBuf::from(finished["destination"].as_str().unwrap());
    assert!(destination.join("netcoredbg.exe").is_file());
    assert!(destination.join("README.txt").is_file());
    assert!(destination.starts_with(seam.home.path()));
    let installed_binary = PathBuf::from(finished["installedBinary"].as_str().unwrap());
    assert_eq!(installed_binary, destination.join("netcoredbg.exe"));
    assert!(!installed_binary.to_string_lossy().contains(".staging"));
    let requests = server.requests();
    assert_eq!(requests, 1);

    // A valid installation is reused; the server is not contacted again.
    let planned = host.dispatch("debug.plan", json!({})).unwrap()["debug"].clone();
    assert_eq!(planned["adapterState"], "ready");
    assert_eq!(planned["plan"]["existing"]["state"], "present");
    let reused = host
        .dispatch(
            "debug.install",
            json!({ "planId": planned["plan"]["planId"], "confirm": true }),
        )
        .unwrap()["debug"]
        .clone();
    assert_eq!(reused["adapterState"], "ready");
    assert_eq!(reused["reused"], true);
    assert_eq!(reused["sha256"], finished["sha256"]);
    assert_eq!(server.requests(), requests, "reuse must not download");
    assert_eq!(
        reused["gateReason"].as_str().unwrap(),
        "Adapter executable and version are verified. This does not prove DAP; the Debug tab reports the real session, exit and processes."
    );
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn a_fresh_host_reports_an_installed_adapter_without_claiming_the_platform_blocks_it_isolated() {
    let zip = zip_bytes(&[("netcoredbg/netcoredbg", ARTIFACT, Some(0o755))]);
    let server = Server::start(vec![("/asset.zip", Route::Ok(zip.clone()))]);
    let seam = seam_platform(&server.url("/asset.zip"), &sha256(&zip), "macos/aarch64");
    let workspace = temp_workspace();
    let first = support::host();
    open(&first, workspace.path());
    first.dispatch("trust.grant", json!({})).unwrap();
    first
        .dispatch("debug.flag", json!({ "enabled": true, "confirm": true }))
        .unwrap();
    let plan_id = first.dispatch("debug.plan", json!({})).unwrap()["debug"]["plan"]["planId"]
        .as_str()
        .unwrap()
        .to_string();
    first
        .dispatch(
            "debug.install",
            json!({ "planId": plan_id, "confirm": true }),
        )
        .unwrap();
    let installed = wait_for(&first, &["ready"], Duration::from_secs(30));
    let sha = installed["sha256"].as_str().unwrap().to_string();
    assert_eq!(server.requests(), 1);
    drop(first);

    // A new host process starts with the experimental flag off and no in-memory install. The
    // adapter on disk is still reported, and the plan names the flag as the remaining gate.
    let second = support::host();
    open(&second, workspace.path());
    let view = state(&second);
    assert_eq!(view["adapterState"], "experimental_disabled");
    assert_eq!(view["installed"], true);
    assert_eq!(view["installedSha256"], sha);
    assert!(view["installedBinary"]
        .as_str()
        .unwrap()
        .starts_with(seam.home.path().to_str().unwrap()));
    let message = view["adapterMessage"].as_str().unwrap();
    assert!(message.contains("already installed"), "{message}");
    assert!(!message.contains("stays blocked on this platform"), "{message}");

    let planned = second.dispatch("debug.plan", json!({})).unwrap()["debug"].clone();
    assert_eq!(planned["plan"]["existing"]["state"], "present");
    assert_eq!(planned["plan"]["existing"]["sha256"], sha);
    let note = planned["note"].as_str().unwrap();
    assert!(!note.contains("stays blocked on this platform"), "{note}");
    assert!(note.contains("already installed"), "{note}");
    assert!(note.contains("experimental flag"), "{note}");
    assert_eq!(server.requests(), 1, "planning and probing never download");

    // With the flag on, the same installed adapter is a normal, unblocked plan.
    let flagged = second
        .dispatch("debug.flag", json!({ "enabled": true, "confirm": true }))
        .unwrap()["debug"]
        .clone();
    assert!(flagged["plan"]["blockedReason"].is_null());
    assert_eq!(flagged["installed"], true);
    assert_eq!(flagged["installedSha256"], sha);
    let flagged_note = flagged["note"].as_str().unwrap();
    assert!(flagged_note.contains("already installed"), "{flagged_note}");
    assert!(!flagged_note.contains("stays blocked"), "{flagged_note}");
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn a_failed_download_shows_the_error_and_retry_completes_the_confirmed_plan_isolated() {
    let zip = artifact_zip(ARTIFACT);
    let fail = Arc::new(AtomicBool::new(true));
    let server = Server::start(vec![(
        "/asset.zip",
        Route::FailOnce(zip.clone(), Arc::clone(&fail)),
    )]);
    let seam = seam(&server.url("/asset.zip"), &sha256(&zip));
    let host = support::host();
    let workspace = temp_workspace();
    open(&host, workspace.path());
    host.dispatch("trust.grant", json!({})).unwrap();
    let plan_id = host.dispatch("debug.plan", json!({})).unwrap()["debug"]["plan"]["planId"]
        .as_str()
        .unwrap()
        .to_string();
    host.dispatch(
        "debug.install",
        json!({ "planId": plan_id, "confirm": true }),
    )
    .unwrap();

    let failed = wait_for(&host, &["failed"], Duration::from_secs(30));
    let error = failed["error"].as_str().unwrap();
    assert!(error.contains("HTTP 500"), "{error}");
    assert_eq!(failed["progress"], Value::Null);
    assert_eq!(failed["installed"], false);
    assert_eq!(failed["destinationPresent"], false);
    assert_eq!(server.requests(), 1);
    assert!(!tool_dir(seam.home.path()).join("3.2.0-1092").exists());

    // Retry reuses the already confirmed plan; no new confirmation and no new download URL.
    let retried = host.dispatch("debug.retry", json!({})).unwrap()["debug"].clone();
    assert_eq!(retried["adapterState"], "downloading");
    let finished = wait_for(&host, &["ready", "failed"], Duration::from_secs(30));
    assert_eq!(finished["adapterState"], "ready", "{}", finished["log"]);
    assert_eq!(finished["error"], Value::Null);
    assert_eq!(finished["installed"], true);
    assert_eq!(finished["installedSha256"], finished["sha256"]);
    assert_eq!(server.requests(), 2);
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn hash_version_corruption_and_traversal_are_rejected_without_promotion_isolated() {
    let zip = artifact_zip(ARTIFACT);

    // 1. Hash mismatch: the controlled expected digest is wrong on purpose.
    {
        let server = Server::start(vec![("/asset.zip", Route::Ok(zip.clone()))]);
        let seam = seam(&server.url("/asset.zip"), &"1".repeat(64));
        let host = support::host();
        let workspace = temp_workspace();
        open(&host, workspace.path());
        host.dispatch("trust.grant", json!({})).unwrap();
        let plan_id = host.dispatch("debug.plan", json!({})).unwrap()["debug"]["plan"]["planId"]
            .as_str()
            .unwrap()
            .to_string();
        host.dispatch("debug.install", json!({ "planId": plan_id, "confirm": true }))
            .unwrap();
        let failed = wait_for(&host, &["failed"], Duration::from_secs(30));
        assert!(failed["error"].as_str().unwrap().contains("SHA-256 mismatch"));
        assert_eq!(failed["stagingPresent"], false);
        assert!(!tool_dir(seam.home.path()).join("3.2.0-1092").exists());
    }

    // 2. Wrong version: the archive is valid but the executable is not 3.2.0.
    {
        let wrong = artifact_zip(WRONG_VERSION);
        let server = Server::start(vec![("/asset.zip", Route::Ok(wrong.clone()))]);
        let seam = seam(&server.url("/asset.zip"), &sha256(&wrong));
        let host = support::host();
        let workspace = temp_workspace();
        open(&host, workspace.path());
        host.dispatch("trust.grant", json!({})).unwrap();
        let plan_id = host.dispatch("debug.plan", json!({})).unwrap()["debug"]["plan"]["planId"]
            .as_str()
            .unwrap()
            .to_string();
        host.dispatch("debug.install", json!({ "planId": plan_id, "confirm": true }))
            .unwrap();
        let failed = wait_for(&host, &["failed"], Duration::from_secs(30));
        assert!(failed["error"].as_str().unwrap().contains("Verification failed"));
        assert!(failed["log"].as_str().unwrap().contains("3.1.1"));
        assert_eq!(failed["stagingPresent"], true, "staging stays as evidence");
        assert!(!tool_dir(seam.home.path()).join("3.2.0-1092").exists());
    }

    // 3. Corrupt archive: valid hash for bytes that are not a zip.
    {
        let corrupt = b"this is not a zip archive".to_vec();
        let server = Server::start(vec![("/asset.zip", Route::Ok(corrupt.clone()))]);
        let seam = seam(&server.url("/asset.zip"), &sha256(&corrupt));
        let host = support::host();
        let workspace = temp_workspace();
        open(&host, workspace.path());
        host.dispatch("trust.grant", json!({})).unwrap();
        let plan_id = host.dispatch("debug.plan", json!({})).unwrap()["debug"]["plan"]["planId"]
            .as_str()
            .unwrap()
            .to_string();
        host.dispatch("debug.install", json!({ "planId": plan_id, "confirm": true }))
            .unwrap();
        let failed = wait_for(&host, &["failed"], Duration::from_secs(30));
        assert!(failed["error"].as_str().unwrap().contains("not a readable zip"));
        assert!(!tool_dir(seam.home.path()).join("3.2.0-1092").exists());
    }

    // 4. Traversal entry: rejected before anything is written outside staging.
    {
        let evil = zip_bytes(&[("../escape.txt", b"escaped\n", Some(0o644))]);
        let server = Server::start(vec![("/asset.zip", Route::Ok(evil.clone()))]);
        let seam = seam(&server.url("/asset.zip"), &sha256(&evil));
        let host = support::host();
        let workspace = temp_workspace();
        open(&host, workspace.path());
        host.dispatch("trust.grant", json!({})).unwrap();
        let plan_id = host.dispatch("debug.plan", json!({})).unwrap()["debug"]["plan"]["planId"]
            .as_str()
            .unwrap()
            .to_string();
        host.dispatch("debug.install", json!({ "planId": plan_id, "confirm": true }))
            .unwrap();
        let failed = wait_for(&host, &["failed"], Duration::from_secs(30));
        assert!(
            failed["error"]
                .as_str()
                .unwrap()
                .contains("escapes the extraction directory"),
            "{}",
            failed["error"]
        );
        assert!(!tool_dir(seam.home.path()).join("escape.txt").exists());
        assert!(!seam.home.path().join("escape.txt").exists());
        assert!(!tool_dir(seam.home.path()).join("3.2.0-1092").exists());
    }

    // 5. Symlink entry: never extracted, not even as a link inside staging.
    {
        let link = raw_zip("link", b"/etc/passwd", 0o120777);
        let server = Server::start(vec![("/asset.zip", Route::Ok(link.clone()))]);
        let seam = seam(&server.url("/asset.zip"), &sha256(&link));
        let host = support::host();
        let workspace = temp_workspace();
        open(&host, workspace.path());
        host.dispatch("trust.grant", json!({})).unwrap();
        let plan_id = host.dispatch("debug.plan", json!({})).unwrap()["debug"]["plan"]["planId"]
            .as_str()
            .unwrap()
            .to_string();
        host.dispatch("debug.install", json!({ "planId": plan_id, "confirm": true }))
            .unwrap();
        let failed = wait_for(&host, &["failed"], Duration::from_secs(30));
        assert!(
            failed["error"].as_str().unwrap().contains("symlinks are not extracted"),
            "{}",
            failed["error"]
        );
        assert!(!tool_dir(seam.home.path()).join("3.2.0-1092").exists());
    }

    // 6. Absolute path entry: rejected as an escape. The target lives under a fresh temporary
    // directory, never a fixed name in /tmp that a concurrent run could create or remove.
    let outside = tempfile::tempdir().unwrap();
    {
        let evil = outside.path().join("openridr-evil.txt");
        let absolute = raw_zip(&evil.to_string_lossy(), b"x", 0o100644);
        let server = Server::start(vec![("/asset.zip", Route::Ok(absolute.clone()))]);
        let seam = seam(&server.url("/asset.zip"), &sha256(&absolute));
        let host = support::host();
        let workspace = temp_workspace();
        open(&host, workspace.path());
        host.dispatch("trust.grant", json!({})).unwrap();
        let plan_id = host.dispatch("debug.plan", json!({})).unwrap()["debug"]["plan"]["planId"]
            .as_str()
            .unwrap()
            .to_string();
        host.dispatch("debug.install", json!({ "planId": plan_id, "confirm": true }))
            .unwrap();
        let failed = wait_for(&host, &["failed"], Duration::from_secs(30));
        assert!(
            failed["error"].as_str().unwrap().contains("escapes the extraction directory"),
            "{}",
            failed["error"]
        );
        assert!(!evil.exists());
        assert!(!tool_dir(seam.home.path()).join("3.2.0-1092").exists());
    }
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn origins_and_redirects_are_checked_before_any_download_isolated() {
    let zip = artifact_zip(ARTIFACT);

    // A URL that is not the official host and not the loopback test seam is refused.
    {
        let seam = seam("https://example.invalid/netcoredbg-win64.zip", &sha256(&zip));
        let host = support::host();
        let workspace = temp_workspace();
        open(&host, workspace.path());
        host.dispatch("trust.grant", json!({})).unwrap();
        let plan_id = host.dispatch("debug.plan", json!({})).unwrap()["debug"]["plan"]["planId"]
            .as_str()
            .unwrap()
            .to_string();
        host.dispatch("debug.install", json!({ "planId": plan_id, "confirm": true }))
            .unwrap();
        let failed = wait_for(&host, &["failed"], Duration::from_secs(30));
        assert!(failed["error"]
            .as_str()
            .unwrap()
            .contains("only HTTPS on the official release hosts"));
        assert_eq!(failed["error"].as_str().unwrap().contains("example.invalid"), true);
        assert!(!tool_dir(seam.home.path()).join("3.2.0-1092").exists());
    }

    // A redirect that leaves the official hosts is rejected after the first hop.
    {
        let server = Server::start(vec![(
            "/asset.zip",
            Route::Redirect("https://example.invalid/payload.zip"),
        )]);
        let seam = seam(&server.url("/asset.zip"), &sha256(&zip));
        let host = support::host();
        let workspace = temp_workspace();
        open(&host, workspace.path());
        host.dispatch("trust.grant", json!({})).unwrap();
        let plan_id = host.dispatch("debug.plan", json!({})).unwrap()["debug"]["plan"]["planId"]
            .as_str()
            .unwrap()
            .to_string();
        host.dispatch("debug.install", json!({ "planId": plan_id, "confirm": true }))
            .unwrap();
        let failed = wait_for(&host, &["failed"], Duration::from_secs(30));
        assert!(failed["error"]
            .as_str()
            .unwrap()
            .contains("Refused to download from https://example.invalid/payload.zip"));
        assert_eq!(server.requests(), 1, "only the initial request was made");
        assert!(!tool_dir(seam.home.path()).join("3.2.0-1092").exists());
    }
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn cancel_keeps_partial_state_and_retry_completes_isolated() {
    let zip = artifact_zip(ARTIFACT);
    let server = Server::start(vec![
        ("/slow.zip", Route::Slow(zip.clone())),
        ("/asset.zip", Route::Ok(zip.clone())),
    ]);
    let seam = seam(&server.url("/slow.zip"), &sha256(&zip));
    let host = support::host();
    let workspace = temp_workspace();
    open(&host, workspace.path());

    // A planned install can be cancelled before trust: cancellation is a safety action.
    host.dispatch("debug.plan", json!({})).unwrap();
    assert_eq!(state(&host)["adapterState"], "planned");
    let cancelled = host.dispatch("debug.cancel", json!({})).unwrap()["debug"].clone();
    assert_eq!(cancelled["adapterState"], "cancelled");
    assert!(cancelled["log"].as_str().unwrap().contains("Plan cancelled"));
    assert_eq!(server.requests(), 0);

    host.dispatch("trust.grant", json!({})).unwrap();
    let planned = host.dispatch("debug.plan", json!({})).unwrap()["debug"].clone();
    let plan_id = planned["plan"]["planId"].as_str().unwrap().to_string();
    host.dispatch("debug.install", json!({ "planId": plan_id, "confirm": true }))
        .unwrap();
    wait_for(&host, &["downloading"], Duration::from_secs(10));
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let debug = state(&host);
        if debug["progress"]["downloaded"].as_u64().unwrap_or(0) > 0 {
            break;
        }
        assert!(Instant::now() < deadline, "no bytes were observed");
        std::thread::sleep(Duration::from_millis(5));
    }
    host.dispatch("debug.cancel", json!({})).unwrap();
    wait_for(&host, &["cancelled"], Duration::from_secs(10));
    let deadline = Instant::now() + Duration::from_secs(10);
    while state(&host)["busy"].as_bool().unwrap_or(false) {
        assert!(Instant::now() < deadline, "the cancelled worker never stopped");
        std::thread::sleep(Duration::from_millis(10));
    }
    let stopped = state(&host);
    assert!(stopped["error"].is_null());
    assert_eq!(stopped["stagingPresent"], false);
    assert_eq!(stopped["destinationPresent"], false);
    assert!(stopped["log"].as_str().unwrap().contains("Cancellation requested"));
    assert_eq!(
        error_code(host.dispatch("debug.cancel", json!({}))),
        "invalid_state"
    );

    // Retry with a changed plan: a new confirmation is required, nothing is downloaded.
    std::env::set_var("OPENRIDR_DEBUG_ASSET_URL", server.url("/asset.zip"));
    assert_eq!(
        error_code(host.dispatch("debug.retry", json!({}))),
        "confirmation_required"
    );
    let replanned = host.dispatch("debug.plan", json!({})).unwrap()["debug"].clone();
    assert!(replanned["plan"]["blockedReason"].is_null());
    let retried = host
        .dispatch(
            "debug.install",
            json!({ "planId": replanned["plan"]["planId"], "confirm": true }),
        )
        .unwrap();
    assert_eq!(retried["debug"]["adapterState"], "downloading");
    let finished = wait_for(&host, &["ready", "failed"], Duration::from_secs(30));
    assert_eq!(finished["adapterState"], "ready", "{}", finished["log"]);
    assert!(finished["revision"].as_u64().unwrap() >= 2);
    assert!(finished["log"].as_str().unwrap().contains("== attempt 2 =="));
    assert!(Path::new(finished["destination"].as_str().unwrap())
        .join("netcoredbg.exe")
        .is_file());
    assert!(Path::new(finished["destination"].as_str().unwrap()).starts_with(seam.home.path()));
}

#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn the_debugger_methods_are_gated_before_trust_like_the_other_executables_isolated() {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _probe = EnvVar::set("OPENRIDR_ENABLE_PROBE", "0");
    let host = support::host();
    let workspace = temp_workspace();
    open(&host, workspace.path());
    for method in EXECUTABLE_METHODS.iter().filter(|method| method.starts_with("debug.")) {
        assert_eq!(
            error_code(host.dispatch(method, json!({ "planId": "none", "confirm": true }))),
            "trust_required",
            "{method}"
        );
    }
    assert!(!tool_dir(home.path()).exists());
    // Planning and reading the state stay read-only and trust-free.
    assert_eq!(
        host.dispatch("debug.plan", json!({})).unwrap()["debug"]["adapterState"],
        "experimental_disabled"
    );
    assert_eq!(
        host.dispatch("debug.state", json!({})).unwrap()["debug"]["version"],
        "3.2.0-1092"
    );
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn a_bundled_adapter_is_ready_and_refuses_install_and_retry_isolated() {
    let home = tempfile::tempdir().unwrap();
    let bundle = tempfile::tempdir().unwrap();
    let bundled_binary = bundle.path().join("netcoredbg");
    fake_netcoredbg(&bundled_binary);
    // An older per-user install must lose to the bundle.
    fake_netcoredbg(&tool_dir(home.path()).join("3.2.0-1092/netcoredbg/netcoredbg"));
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _probe = EnvVar::set("OPENRIDR_ENABLE_PROBE", "0");
    let _bundle = EnvVar::set("OPENRIDR_BUNDLED_NETCOREDBG", bundle.path());

    let host = support::host();
    let workspace = temp_workspace();
    open(&host, workspace.path());
    host.dispatch("trust.grant", json!({})).unwrap();
    let flagged = host
        .dispatch("debug.flag", json!({ "enabled": true, "confirm": true }))
        .unwrap();
    let debug = &flagged["debug"];
    assert_eq!(debug["adapterState"], "ready");
    assert_eq!(debug["bundled"], true);
    assert_eq!(debug["version"], "3.2.0-1092");
    assert_eq!(debug["installable"], false);
    assert!(debug["progress"].is_null());
    assert_eq!(
        debug["installedBinary"].as_str().unwrap(),
        bundled_binary.to_string_lossy()
    );
    assert_eq!(debug["sha256"].as_str().unwrap().len(), 64);
    assert!(debug["adapterMessage"]
        .as_str()
        .unwrap()
        .contains("Bundled with OpenRidr"));

    let refused = host
        .dispatch("debug.install", json!({ "confirm": true }))
        .expect_err("bundled builds never download");
    assert_eq!(refused.code, "invalid_state");
    assert_eq!(
        refused.message,
        "netcoredbg is bundled with OpenRidr; there is nothing to download or install."
    );
    let retried = host
        .dispatch("debug.retry", json!({}))
        .expect_err("bundled builds never retry a download");
    assert_eq!(retried.code, "invalid_state");
    assert_eq!(retried.message, refused.message);
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn without_a_bundle_the_tools_install_is_reused_isolated() {
    let home = tempfile::tempdir().unwrap();
    let installed = tool_dir(home.path()).join("3.2.0-1092/netcoredbg/netcoredbg");
    fake_netcoredbg(&installed);
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _probe = EnvVar::set("OPENRIDR_ENABLE_PROBE", "0");
    let _bundle = EnvVar::remove("OPENRIDR_BUNDLED_NETCOREDBG");

    let host = support::host();
    let workspace = temp_workspace();
    open(&host, workspace.path());
    host.dispatch("trust.grant", json!({})).unwrap();
    host.dispatch("debug.flag", json!({ "enabled": true, "confirm": true }))
        .unwrap();
    let planned = host.dispatch("debug.plan", json!({})).unwrap();
    let plan_id = planned["debug"]["plan"]["planId"].as_str().unwrap().to_string();
    let installed_snapshot = host
        .dispatch("debug.install", json!({ "planId": plan_id, "confirm": true }))
        .unwrap();
    let debug = &installed_snapshot["debug"];
    assert_eq!(debug["bundled"], false);
    assert_eq!(debug["adapterState"], "ready");
    assert_eq!(debug["reused"], true);
    assert_eq!(
        debug["installedBinary"].as_str().unwrap(),
        installed.to_string_lossy()
    );
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
#[ignore = "isolated leaf: it writes OPENRIDR_* for its own host, so it runs in its own process"]
fn without_a_bundle_or_a_tools_install_the_adapter_is_not_ready_isolated() {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _probe = EnvVar::set("OPENRIDR_ENABLE_PROBE", "0");
    let _bundle = EnvVar::remove("OPENRIDR_BUNDLED_NETCOREDBG");

    let host = support::host();
    let workspace = temp_workspace();
    open(&host, workspace.path());
    host.dispatch("debug.flag", json!({ "enabled": true, "confirm": true }))
        .unwrap();
    let planned = host.dispatch("debug.plan", json!({})).unwrap();
    let debug = &planned["debug"];
    assert_eq!(debug["bundled"], false);
    assert_ne!(debug["adapterState"], "ready");
    assert_eq!(debug["plan"]["existing"]["state"], "absent");
    assert!(!tool_dir(home.path()).exists());
}

/// The ambient purge runs once per binary, so only a fresh process can prove that an exported
/// `OPENRIDR_BUNDLED_NETCOREDBG` no longer turns the controlled installer away.
#[test]
fn an_ambient_bundled_override_never_reaches_the_controlled_installer() {
    let bundle = tempfile::tempdir().unwrap();
    let adapter = bundle.path().join("netcoredbg");
    fs::write(&adapter, ARTIFACT).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&adapter, fs::Permissions::from_mode(0o755)).unwrap();
    }

    support::assert_leaf_passes_in_a_fresh_process(
        "install_requires_trust_confirmation_and_an_unchanged_plan",
        &[("OPENRIDR_BUNDLED_NETCOREDBG", bundle.path().as_os_str())],
    );
}

// Each leaf below writes `OPENRIDR_*` for its host, so it runs in its own process: the
// shared test process never changes its environment.
#[test]
fn macos_arm64_needs_the_experimental_flag_and_never_downloads_without_it() {
    support::run_isolated_leaf("macos_arm64_needs_the_experimental_flag_and_never_downloads_without_it_isolated");
}

#[test]
fn macos_arm64_installs_the_controlled_artifact_only_after_opt_in() {
    support::run_isolated_leaf("macos_arm64_installs_the_controlled_artifact_only_after_opt_in_isolated");
}

#[test]
fn install_requires_trust_confirmation_and_an_unchanged_plan() {
    support::run_isolated_leaf("install_requires_trust_confirmation_and_an_unchanged_plan_isolated");
}

#[test]
fn a_changed_plan_after_confirmation_downloads_nothing() {
    support::run_isolated_leaf("a_changed_plan_after_confirmation_downloads_nothing_isolated");
}

#[test]
fn a_controlled_install_reaches_ready_is_reused_and_records_no_dap_proof() {
    support::run_isolated_leaf("a_controlled_install_reaches_ready_is_reused_and_records_no_dap_proof_isolated");
}

#[test]
fn a_fresh_host_reports_an_installed_adapter_without_claiming_the_platform_blocks_it() {
    support::run_isolated_leaf("a_fresh_host_reports_an_installed_adapter_without_claiming_the_platform_blocks_it_isolated");
}

#[test]
fn a_failed_download_shows_the_error_and_retry_completes_the_confirmed_plan() {
    support::run_isolated_leaf("a_failed_download_shows_the_error_and_retry_completes_the_confirmed_plan_isolated");
}

#[test]
fn hash_version_corruption_and_traversal_are_rejected_without_promotion() {
    support::run_isolated_leaf("hash_version_corruption_and_traversal_are_rejected_without_promotion_isolated");
}

#[test]
fn origins_and_redirects_are_checked_before_any_download() {
    support::run_isolated_leaf("origins_and_redirects_are_checked_before_any_download_isolated");
}

#[test]
fn cancel_keeps_partial_state_and_retry_completes() {
    support::run_isolated_leaf("cancel_keeps_partial_state_and_retry_completes_isolated");
}

#[test]
fn the_debugger_methods_are_gated_before_trust_like_the_other_executables() {
    support::run_isolated_leaf("the_debugger_methods_are_gated_before_trust_like_the_other_executables_isolated");
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn a_bundled_adapter_is_ready_and_refuses_install_and_retry() {
    support::run_isolated_leaf("a_bundled_adapter_is_ready_and_refuses_install_and_retry_isolated");
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn without_a_bundle_the_tools_install_is_reused() {
    support::run_isolated_leaf("without_a_bundle_the_tools_install_is_reused_isolated");
}

#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn without_a_bundle_or_a_tools_install_the_adapter_is_not_ready() {
    support::run_isolated_leaf("without_a_bundle_or_a_tools_install_the_adapter_is_not_ready_isolated");
}
