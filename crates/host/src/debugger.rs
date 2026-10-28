//! netcoredbg installation from the approved official release, with platform gates.
//!
//! The approved release is Samsung's `3.2.0-1092`. Installing it is allowed only on a platform
//! with a published asset that this build is allowed to use. macOS arm64 is experimental and
//! unverified, macOS Intel has no asset in that release, and Linux ships a `.tar.gz` this build
//! does not extract. The plan is read-only and lists the asset and the published SHA-256; the
//! archive is downloaded over HTTPS from the official hosts, hashed, extracted with traversal,
//! symlink and size checks, and only then verified (`--version`) and promoted.

use crate::error::{HostError, HostResult};
use crate::logbuf::LogBuffer;
use crate::tools::app_home;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const TOOL: &str = "netcoredbg";
pub const VERSION: &str = "3.2.0-1092";
pub const RELEASE_URL: &str = "https://github.com/Samsung/netcoredbg/releases/tag/3.2.0-1092";
const DOWNLOAD_BASE: &str = "https://github.com/Samsung/netcoredbg/releases/download/3.2.0-1092/";

/// Hosts the official release asset may be read from, including GitHub's asset redirect
/// targets. Everything else is refused before a connection is opened.
const ALLOWED_HOSTS: &[&str] = &[
    "github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
];

const MAX_DOWNLOAD_BYTES: u64 = 20 * 1024 * 1024;
const MAX_EXTRACTED_BYTES: u64 = 200 * 1024 * 1024;
const MAX_ENTRIES: usize = 200;
const MAX_REDIRECTS: usize = 5;
const CHUNK: usize = 64 * 1024;

/// Metadata read from the official release on 2026-10-28 (GitHub API
/// `releases/tags/3.2.0-1092`). Only assets actually published there are listed; the SHA-256 is
/// the digest GitHub reports for the uploaded asset, not a digest OpenRidr computed.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishedAsset {
    pub platform: &'static str,
    pub name: &'static str,
    pub size: u64,
    pub sha256: &'static str,
    pub archive: &'static str,
    pub support: &'static str,
    pub availability: &'static str,
    pub reason: Option<&'static str>,
}

const COMMUNITY: &str = "The official release publishes this macOS arm64 build as community supported (upstream README: \"may not work as expected\"); OpenRidr has no verified compatible adapter or DAP evidence for it (ticket 14).";

const PUBLISHED: &[PublishedAsset] = &[
    PublishedAsset {
        platform: "osx-arm64",
        name: "netcoredbg-osx-arm64.zip",
        size: 3_389_504,
        sha256: "f4fa33b3ff874910cc184b4bb3b9c56d0abdf5c6521cee0b144d7c6e4a6e59ea",
        archive: "zip",
        support: "community",
        availability: "unverified",
        reason: Some(COMMUNITY),
    },
    PublishedAsset {
        platform: "win64",
        name: "netcoredbg-win64.zip",
        size: 3_524_161,
        sha256: "3c410a45fa502415203a94fcb88654af65bf8e3dac158a5527a722e7a6b9274a",
        archive: "zip",
        support: "official",
        availability: "available",
        reason: Some(
            "Official Windows x64 asset of the approved release. Installation is gated by OS/arch and has not been executed on this macOS host; DAP is ticket 14.",
        ),
    },
    PublishedAsset {
        platform: "linux-amd64",
        name: "netcoredbg-linux-amd64.tar.gz",
        size: 3_510_717,
        sha256: "080eb3b2d2152465f599d3b33d1ee6e747794e11cc0a3773ec689f5e5f2c5afa",
        archive: "tar.gz",
        support: "official",
        availability: "unsupported_archive",
        reason: Some(
            "Linux is outside the P0 matrix and this build extracts zip archives only; a .tar.gz is never substituted or unpacked by another tool.",
        ),
    },
    PublishedAsset {
        platform: "linux-arm64",
        name: "netcoredbg-linux-arm64.tar.gz",
        size: 3_424_271,
        sha256: "065ff49badec8a695dbea2de6ab6a330c774a191e426a217ab8cc05250627ccb",
        archive: "tar.gz",
        support: "official",
        availability: "unsupported_archive",
        reason: Some(
            "Linux is outside the P0 matrix and this build extracts zip archives only; a .tar.gz is never substituted or unpacked by another tool.",
        ),
    },
];

const INTEGRITY_NOTE: &str = "The download is hashed and compared with the SHA-256 published in the official release metadata; the extracted executable must print 3.2.0 from --version before promotion. OpenRidr verifies no code signature or notarization and claims none. The archive layout inside the release is not verified here.";

const OPERATION: &str = "Download the official asset over HTTPS, verify its published SHA-256, extract the zip into staging (no traversal, absolute path, symlink or special file; bounded size and entry count), run --version on the extracted executable, and only then promote staging to the destination.";

/// App-scoped tool root, outside the repository.
pub fn tool_root() -> PathBuf {
    app_home().join("tools/netcoredbg")
}

pub fn destination() -> PathBuf {
    tool_root().join(VERSION)
}

pub fn staging() -> PathBuf {
    tool_root().join(format!("{VERSION}.staging"))
}

fn binary_name(platform: &str) -> &'static str {
    if platform == "win64" {
        "netcoredbg.exe"
    } else {
        "netcoredbg"
    }
}

/// Maps the host OS/arch to the release asset name. `None` for combinations the release does
/// not cover; no other asset is substituted.
fn platform_key(os: &str, arch: &str) -> &'static str {
    match (os, arch) {
        ("macos", "aarch64") => "osx-arm64",
        ("macos", "x86_64") => "osx-x64",
        ("windows", "x86_64") => "win64",
        ("linux", "x86_64") => "linux-amd64",
        ("linux", "aarch64") => "linux-arm64",
        _ => "unsupported",
    }
}

/// Test seam, enabled only with `OPENRIDR_ENABLE_PROBE=1` (the same dev-only switch as the
/// process probe): forces another platform and a controlled asset served by a local test
/// server. The hash is always required, so the pipeline still verifies what it downloaded.
fn probe() -> bool {
    std::env::var("OPENRIDR_ENABLE_PROBE").as_deref() == Ok("1")
}

fn forced_platform() -> Option<(String, String)> {
    if !probe() {
        return None;
    }
    let raw = std::env::var("OPENRIDR_DEBUG_PLATFORM").ok()?;
    let (os, arch) = raw.split_once('/')?;
    Some((os.to_string(), arch.to_string()))
}

struct ControlledAsset {
    url: String,
    sha256: String,
}

fn controlled_asset() -> Option<ControlledAsset> {
    if !probe() {
        return None;
    }
    let url = std::env::var("OPENRIDR_DEBUG_ASSET_URL").ok()?;
    let sha256 = std::env::var("OPENRIDR_DEBUG_ASSET_SHA256").ok()?;
    Some(ControlledAsset { url, sha256 })
}

fn base_name(url: &str) -> String {
    url.rsplit('/').next().unwrap_or("asset.zip").to_string()
}

fn origin(url: &str) -> Option<(String, String, Option<String>)> {
    let (scheme, rest) = url.split_once("://")?;
    let authority = rest.split('/').next()?;
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) => {
            (host.to_string(), Some(port.to_string()))
        }
        _ => (authority.to_string(), None),
    };
    Some((scheme.to_string(), host.to_lowercase(), port))
}

fn loopback(host: &str) -> bool {
    matches!(host, "127.0.0.1" | "localhost" | "::1" | "[::1]")
}

/// An HTTPS URL on an official release host, or (only for the probe seam) an HTTP URL on
/// loopback.
fn allowed_url(url: &str) -> bool {
    let Some((scheme, host, _)) = origin(url) else {
        return false;
    };
    if scheme == "https" && ALLOWED_HOSTS.contains(&host.as_str()) {
        return true;
    }
    probe() && scheme == "http" && loopback(&host)
}

fn resolve_location(base: &str, location: &str) -> String {
    if location.contains("://") {
        return location.to_string();
    }
    let (scheme, rest) = base.split_once("://").unwrap_or(("https", base));
    let authority = rest.split('/').next().unwrap_or_default();
    format!("{scheme}://{authority}{location}")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub name: &'static str,
    pub url: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetView {
    pub platform: String,
    pub name: String,
    pub size: u64,
    pub sha256: String,
    pub archive: &'static str,
    pub support: &'static str,
    pub availability: &'static str,
    pub reason: Option<String>,
    pub url: String,
    /// A controlled artifact from the test seam, never an official asset.
    pub test_override: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Existing {
    /// `ready` (verified in this session) | `present` (exists, not verified) | `absent`
    pub state: &'static str,
    pub binary: String,
    pub verified_version: Option<String>,
    pub sha256: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub tool: &'static str,
    pub version: &'static str,
    pub source: Source,
    pub os: String,
    pub arch: String,
    pub platform: String,
    pub asset: Option<AssetView>,
    pub availability: &'static str,
    pub availability_reason: Option<String>,
    pub destination: String,
    pub staging: String,
    pub operation: &'static str,
    pub permissions: Vec<String>,
    pub integrity: String,
    pub limits: Vec<String>,
    pub blocked_reason: Option<String>,
    pub existing: Existing,
    pub plan_id: String,
}

fn material(plan: &Plan) -> Vec<String> {
    let asset = plan.asset.as_ref();
    vec![
        plan.tool.to_string(),
        plan.version.to_string(),
        plan.source.name.to_string(),
        plan.source.url.to_string(),
        plan.os.clone(),
        plan.arch.clone(),
        plan.platform.clone(),
        asset.map(|a| a.name.clone()).unwrap_or_default(),
        asset.map(|a| a.url.clone()).unwrap_or_default(),
        asset.map(|a| a.sha256.clone()).unwrap_or_default(),
        asset.map(|a| a.platform.clone()).unwrap_or_default(),
        plan.availability.to_string(),
        plan.destination.clone(),
        plan.staging.clone(),
        plan.operation.to_string(),
        plan.permissions.join("\n"),
        plan.integrity.clone(),
        plan.limits.join("\n"),
        plan.blocked_reason.clone().unwrap_or_default(),
    ]
}

fn plan_id(plan: &Plan) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for part in material(plan) {
        for byte in part.as_bytes().iter().chain(std::iter::once(&0u8)) {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    format!("fnv1a64:{hash:016x}")
}

/// The platform gate, computed from real metadata. `installable` is the only thing that can
/// allow the installer to run.
struct Gate {
    state: &'static str,
    installable: bool,
    message: String,
}

fn gate(os: &str, arch: &str, experimental: bool, controlled: bool) -> Gate {
    if controlled {
        return Gate {
            state: "not_checked",
            installable: true,
            message: "Controlled test artifact: the installer pipeline is forced to this platform by OPENRIDR_ENABLE_PROBE=1. It is not an official asset."
                .to_string(),
        };
    }
    match (os, arch) {
        ("macos", "aarch64") if experimental => Gate {
            state: "unavailable",
            installable: false,
            message: format!(
                "Apple Silicon debugger: experimental · enabled. Unavailable on macOS arm64: no verified compatible {TOOL} {VERSION} adapter. {COMMUNITY} The experimental opt-in neither installs the adapter nor proves compatibility."
            ),
        },
        ("macos", "aarch64") => Gate {
            state: "experimental_disabled",
            installable: false,
            message: format!(
                "Apple Silicon debugger: experimental · disabled. Unavailable on macOS arm64: no verified compatible {TOOL} {VERSION} adapter. {COMMUNITY} Opting in only acknowledges the experiment; it does not install the adapter and does not prove DAP."
            ),
        },
        ("macos", "x86_64") => Gate {
            state: "unavailable",
            installable: false,
            message: format!(
                "Unavailable on macOS Intel: the {VERSION} release publishes no osx-x64 asset. Installation and Debug are blocked and no other version or asset is substituted."
            ),
        },
        ("windows", "x86_64") => Gate {
            state: "not_checked",
            installable: true,
            message: format!(
                "Windows x64: the official {VERSION} win64 asset is published. Installation is available and gated by OS/arch; it has not been executed here, and Debug still needs the DAP work of ticket 14."
            ),
        },
        ("linux", _) => Gate {
            state: "unavailable",
            installable: false,
            message: format!(
                "Unavailable on Linux: {VERSION} ships a .tar.gz and this build extracts zip archives only. Linux is outside the P0 matrix; no other asset is substituted."
            ),
        },
        _ => Gate {
            state: "unavailable",
            installable: false,
            message: format!(
                "Unsupported platform {os}/{arch}: the {VERSION} release publishes no verified compatible asset for it."
            ),
        },
    }
}

fn published(platform: &str) -> Option<&'static PublishedAsset> {
    PUBLISHED.iter().find(|asset| asset.platform == platform)
}

fn current(
    os: &str,
    arch: &str,
    experimental: bool,
) -> (String, String, String, Gate, Option<ControlledAsset>) {
    let (os, arch) = forced_platform().unwrap_or_else(|| (os.to_string(), arch.to_string()));
    let platform = platform_key(&os, &arch).to_string();
    let controlled = controlled_asset();
    let gate = gate(&os, &arch, experimental, controlled.is_some());
    (os, arch, platform, gate, controlled)
}

fn asset_view(
    platform: &str,
    controlled: Option<&ControlledAsset>,
) -> Option<AssetView> {
    if let Some(controlled) = controlled {
        return Some(AssetView {
            platform: platform.to_string(),
            name: base_name(&controlled.url),
            size: 0,
            sha256: controlled.sha256.clone(),
            archive: "zip",
            support: "controlled",
            availability: "available",
            reason: Some(
                "Controlled test artifact served by the probe seam; not an official Samsung asset."
                    .to_string(),
            ),
            url: controlled.url.clone(),
            test_override: true,
        });
    }
    published(platform).map(|asset| AssetView {
        platform: asset.platform.to_string(),
        name: asset.name.to_string(),
        size: asset.size,
        sha256: asset.sha256.to_string(),
        archive: asset.archive,
        support: asset.support,
        availability: asset.availability,
        reason: asset.reason.map(str::to_string),
        url: format!("{DOWNLOAD_BASE}{}", asset.name),
        test_override: false,
    })
}

/// Builds the read-only plan. Nothing is downloaded, executed or created.
pub fn plan(os: &str, arch: &str, experimental: bool) -> Plan {
    let (os, arch, platform, gate, controlled) = current(os, arch, experimental);
    let asset = asset_view(&platform, controlled.as_ref());
    let availability = match (&asset, gate.installable) {
        (Some(asset), _) => asset.availability,
        (None, _) => "absent",
    };
    let availability_reason = asset
        .as_ref()
        .and_then(|asset| asset.reason.clone())
        .or_else(|| {
            Some(format!(
                "The {VERSION} release publishes no asset for {os}/{arch}."
            ))
        });
    let destination = destination().to_string_lossy().to_string();
    let staging = staging().to_string_lossy().to_string();
    let blocked_reason = if gate.installable {
        None
    } else {
        Some(gate.message.clone())
    };
    let existing = inspect_existing(&platform);

    let integrity = match (&asset, controlled.is_some()) {
        (Some(asset), true) => format!(
            "Controlled test artifact: the expected SHA-256 was provided by the probe seam ({}) and is not an official Samsung digest. Extraction and --version checks are the same as the real path.",
            asset.sha256
        ),
        (Some(asset), false) => format!(
            "Official release metadata publishes SHA-256 {} for {}. {INTEGRITY_NOTE}",
            asset.sha256, asset.name
        ),
        (None, _) => {
            "No asset is available for this platform, so no hash, extraction or verification exists."
                .to_string()
        }
    };
    let mut plan = Plan {
        tool: TOOL,
        version: VERSION,
        source: Source {
            name: "Samsung/netcoredbg (official GitHub release)",
            url: RELEASE_URL,
        },
        os: os.clone(),
        arch: arch.clone(),
        platform: platform.clone(),
        asset,
        availability,
        availability_reason,
        destination,
        staging,
        operation: OPERATION,
        permissions: vec![
            format!(
                "network: {RELEASE_URL} asset hosts over HTTPS ({})",
                ALLOWED_HOSTS.join(", ")
            ),
            format!("write: {}", tool_root().to_string_lossy()),
            "create a temporary .download file next to the staging directory".to_string(),
            "no elevation, no global install, no SDK installer, no PATH change, no repository changes"
                .to_string(),
        ],
        integrity,
        limits: vec![
            format!(
                "download ≤ {} MiB, extracted tree ≤ {} MiB and ≤ {MAX_ENTRIES} entries",
                MAX_DOWNLOAD_BYTES / (1024 * 1024),
                MAX_EXTRACTED_BYTES / (1024 * 1024)
            ),
            format!("≤ {MAX_REDIRECTS} redirects, each to an official host and HTTPS"),
            "zip entries with traversal, absolute paths, symlinks or special modes are rejected; corruption fails the install"
                .to_string(),
            "progress is reported only as bytes observed; no percentage is invented".to_string(),
        ],
        blocked_reason: None,
        existing,
        plan_id: String::new(),
    };
    plan.blocked_reason = blocked_reason;
    plan.plan_id = plan_id(&plan);
    plan
}

/// Cheap inspection of an existing installation. Never runs the binary.
fn inspect_existing(platform: &str) -> Existing {
    let binary = destination().join(binary_name(platform));
    let present = binary.is_file();
    Existing {
        state: if present { "present" } else { "absent" },
        binary: binary.to_string_lossy().to_string(),
        verified_version: None,
        sha256: None,
        reason: Some(if present {
            "A netcoredbg executable already exists. Confirming reuses it only when --version verifies 3.2.0; otherwise a verified staging install replaces it."
                .to_string()
        } else {
            "No netcoredbg executable at the destination.".to_string()
        }),
    }
}

fn sha256_file(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; CHUNK];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            return Some(format!("{:x}", hasher.finalize()));
        }
        hasher.update(&buffer[..read]);
    }
}

/// Runs the extracted executable's read-only `--version`. It is only executed after the archive
/// hash matched and never before the user confirmed the install.
fn verify_existing(platform: &str) -> Existing {
    let binary = destination().join(binary_name(platform));
    let text = binary.to_string_lossy().to_string();
    if !binary.is_file() {
        return Existing {
            state: "absent",
            binary: text,
            verified_version: None,
            sha256: None,
            reason: Some("No netcoredbg executable at the destination.".to_string()),
        };
    }
    verify_binary(&binary)
}

fn verify_binary(binary: &Path) -> Existing {
    let text = binary.to_string_lossy().to_string();
    let output = Command::new(binary).arg("--version").output();
    let Ok(output) = output else {
        return Existing {
            state: "invalid",
            binary: text,
            verified_version: None,
            sha256: None,
            reason: Some("Could not run netcoredbg --version.".to_string()),
        };
    };
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let combined = combined.trim().to_string();
    if !output.status.success() || !combined.contains("3.2.0") {
        return Existing {
            state: "invalid",
            binary: text,
            verified_version: None,
            sha256: None,
            reason: Some(format!(
                "--version did not report 3.2.0 (exit {}). Output: {combined}",
                output
                    .status
                    .code()
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            )),
        };
    }
    Existing {
        state: "ready",
        binary: text,
        verified_version: Some("3.2.0".to_string()),
        sha256: sha256_file(binary),
        reason: Some(combined),
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub adapter_state: &'static str,
    pub adapter_message: String,
    pub apple_silicon_experimental: bool,
    pub version: &'static str,
    pub source: Source,
    pub platform: String,
    pub os: String,
    pub arch: String,
    pub assets: Vec<AssetView>,
    pub asset: Option<AssetView>,
    pub availability: &'static str,
    pub availability_reason: Option<String>,
    pub installable: bool,
    pub gate_reason: Option<String>,
    /// A verified executable is not DAP evidence; this build has not proven DAP.
    pub dap_proven: bool,
    pub destination: String,
    pub staging: String,
    pub plan: Option<Plan>,
    pub log: String,
    pub log_truncated: bool,
    pub progress: Option<Progress>,
    pub sha256: Option<String>,
    pub verified_version: Option<String>,
    pub verified_output: Option<String>,
    pub error: Option<String>,
    pub note: Option<String>,
    pub reused: bool,
    pub revision: u64,
    pub workspace_id: Option<u64>,
    pub context_id: Option<u64>,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub sealed: bool,
    pub stale_reason: Option<String>,
    pub staging_present: bool,
    pub destination_present: bool,
    /// A worker thread is still winding down after a cancel or failure.
    pub busy: bool,
}

struct Data {
    revision: u64,
    state: &'static str,
    destination: String,
    staging: String,
    platform: String,
    plan: Option<Plan>,
    confirmed_plan_id: Option<String>,
    log: LogBuffer,
    progress: Option<Progress>,
    sha256: Option<String>,
    verified_version: Option<String>,
    verified_output: Option<String>,
    error: Option<String>,
    note: Option<String>,
    reused: bool,
    workspace_id: Option<u64>,
    context_id: Option<u64>,
    started_at: Option<u64>,
    finished_at: Option<u64>,
    sealed: bool,
    stale_reason: Option<String>,
    /// A worker thread is still running (download/extract/verify), even after a cancel request.
    active: bool,
}

pub struct Debugger {
    data: Mutex<Data>,
    cancel: AtomicBool,
}

impl Debugger {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(Data {
                revision: 0,
                state: "not_checked",
                destination: destination().to_string_lossy().to_string(),
                staging: staging().to_string_lossy().to_string(),
                platform: String::new(),
                plan: None,
                confirmed_plan_id: None,
                log: LogBuffer::default(),
                progress: None,
                sha256: None,
                verified_version: None,
                verified_output: None,
                error: None,
                note: None,
                reused: false,
                workspace_id: None,
                context_id: None,
                started_at: None,
                finished_at: None,
                sealed: false,
                stale_reason: None,
                active: false,
            }),
            cancel: AtomicBool::new(false),
        }
    }

    /// Full state for the snapshot. `experimental` is the session-scoped opt-in.
    pub fn view(&self, experimental: bool) -> View {
        let data = self.data.lock().unwrap();
        let (os, arch) = (
            std::env::consts::OS.to_string(),
            std::env::consts::ARCH.to_string(),
        );
        let forced = forced_platform();
        let (os, arch) = forced.clone().unwrap_or((os, arch));
        let platform = platform_key(&os, &arch).to_string();
        let controlled = controlled_asset();
        let gate = gate(&os, &arch, experimental, controlled.is_some());
        let asset = asset_view(&platform, controlled.as_ref());
        let install_state = data.state;
        let (state, message) = if matches!(
            install_state,
            "not_checked" | "unavailable" | "experimental_disabled"
        ) {
            (gate.state, gate.message.clone())
        } else {
            (install_state, data.note.clone().unwrap_or_else(|| gate.message.clone()))
        };
        let (log, log_truncated) = data.log.text();
        let gate_reason = if state == "ready" {
            Some(
                "Adapter executable and version are verified, but DAP launch is not implemented in this build (ticket 14)."
                    .to_string(),
            )
        } else {
            Some(message.clone())
        };
        let availability = asset
            .as_ref()
            .map(|asset| asset.availability)
            .unwrap_or("absent");
        View {
            adapter_state: state,
            adapter_message: message,
            apple_silicon_experimental: experimental,
            version: VERSION,
            source: Source {
                name: "Samsung/netcoredbg (official GitHub release)",
                url: RELEASE_URL,
            },
            os: os.clone(),
            arch: arch.clone(),
            platform: platform.clone(),
            // Only assets actually published by the release; the controlled test artifact is
            // reported through `asset.testOverride`, never as an official row.
            assets: PUBLISHED
                .iter()
                .filter_map(|asset| asset_view(asset.platform, None))
                .collect(),
            asset,
            availability,
            availability_reason: data
                .plan
                .as_ref()
                .and_then(|plan| plan.availability_reason.clone())
                .or_else(|| {
                    Some(format!(
                        "The {VERSION} release publishes no asset for {os}/{arch}."
                    ))
                }),
            installable: gate.installable,
            gate_reason,
            dap_proven: false,
            destination: data.destination.clone(),
            staging: data.staging.clone(),
            plan: data.plan.clone(),
            log,
            log_truncated,
            progress: data.progress.clone(),
            sha256: data.sha256.clone(),
            verified_version: data.verified_version.clone(),
            verified_output: data.verified_output.clone(),
            error: data.error.clone(),
            note: data.note.clone(),
            reused: data.reused,
            revision: data.revision,
            workspace_id: if data.revision > 0 {
                data.workspace_id
            } else {
                None
            },
            context_id: if data.revision > 0 {
                data.context_id
            } else {
                None
            },
            started_at: data.started_at,
            finished_at: data.finished_at,
            sealed: data.sealed,
            stale_reason: data.stale_reason.clone(),
            staging_present: Path::new(&data.staging).is_dir(),
            destination_present: Path::new(&data.destination).is_dir(),
            busy: data.active,
        }
    }

    /// Records the freshly built plan. Downloads nothing.
    pub fn set_plan(&self, plan: Plan) {
        let mut data = self.data.lock().unwrap();
        if matches!(data.state, "downloading" | "installing" | "verifying") {
            return;
        }
        data.destination = plan.destination.clone();
        data.staging = plan.staging.clone();
        data.platform = plan.platform.clone();
        let still_installed = data.state == "ready"
            && data.sha256.as_deref()
                == sha256_file(
                    &Path::new(&plan.destination).join(binary_name(&plan.platform)),
                )
                .as_deref();
        // A blocked plan never becomes an advertised install state: the platform gate stays
        // visible instead of implying that only a confirmation is missing.
        data.state = if plan.blocked_reason.is_some() {
            "not_checked"
        } else if still_installed {
            "ready"
        } else {
            "planned"
        };
        data.progress = None;
        data.finished_at = None;
        data.note = Some(match &plan.blocked_reason {
            Some(_) => "The plan is read-only and stays blocked on this platform; see the adapter reason and the plan's blocked field."
                .to_string(),
            None if still_installed => "The verified installation remains ready; confirming the plan reuses it without downloading or overwriting.".to_string(),
            None => "The plan did not download or execute anything. Confirming downloads the official asset, verifies its hash and only then promotes it."
                .to_string(),
        });
        if Path::new(&plan.staging).is_dir() {
            data.note = Some(format!(
                "A partial staging directory exists at {}; it is not ready and a confirmed install replaces it.",
                plan.staging
            ));
        }
        data.plan = Some(plan);
        data.confirmed_plan_id = None;
    }

    pub fn confirmed_plan_id(&self) -> Option<String> {
        self.data.lock().unwrap().confirmed_plan_id.clone()
    }

    /// Starts the real download/extract/verify pipeline. Trust, consent, platform and plan
    /// revalidation happened in the caller.
    pub fn begin_install(
        self: &Arc<Self>,
        plan: Plan,
        workspace_id: u64,
        context_id: u64,
    ) -> HostResult<Value> {
        if let Some(reason) = &plan.blocked_reason {
            return Err(HostError::with_details(
                "unsupported_platform",
                format!(
                    "Installation of {TOOL} {VERSION} is not available on {}/{}: {reason} Nothing was downloaded.",
                    plan.os, plan.arch
                ),
                json!({ "plan": serde_json::to_value(&plan).unwrap_or(Value::Null) }),
            ));
        }
        if plan.asset.is_none() {
            return Err(HostError::new(
                "unsupported_platform",
                format!(
                    "No verified compatible {TOOL} {VERSION} asset exists for {}/{}; nothing was downloaded.",
                    plan.os, plan.arch
                ),
            ));
        }
        {
            let data = self.data.lock().unwrap();
            if data.active || matches!(data.state, "downloading" | "installing" | "verifying") {
                return Err(HostError::new(
                    "invalid_state",
                    "A netcoredbg installation is already running.",
                ));
            }
        }
        // A verified installation is reused; nothing is downloaded or replaced.
        let existing = verify_existing(&plan.platform);
        if existing.state == "ready" {
            let mut data = self.data.lock().unwrap();
            data.state = "ready";
            data.platform = plan.platform.clone();
            data.destination = plan.destination.clone();
            data.staging = plan.staging.clone();
            data.sha256 = existing.sha256.clone();
            data.verified_version = existing.verified_version.clone();
            data.verified_output = existing.reason.clone();
            data.reused = true;
            data.error = None;
            data.workspace_id = Some(workspace_id);
            data.context_id = Some(context_id);
            data.finished_at = Some(now_ms());
            data.note = Some(
                "Reused the verified installation. Nothing was downloaded, replaced or started."
                    .to_string(),
            );
            data.log.push(
                "Existing netcoredbg installation verified; reused without downloading."
                    .to_string(),
            );
            if let Some(output) = &existing.reason {
                data.log.push(format!("$ {} --version", existing.binary));
                data.log.push(output.clone());
            }
            data.plan = Some(plan);
            data.confirmed_plan_id = None;
            return Ok(json!({ "reused": true }));
        }

        self.cancel.store(false, Ordering::SeqCst);
        {
            let mut data = self.data.lock().unwrap();
            data.revision += 1;
            data.state = "downloading";
            data.platform = plan.platform.clone();
            data.destination = plan.destination.clone();
            data.staging = plan.staging.clone();
            data.plan = Some(plan.clone());
            data.confirmed_plan_id = Some(plan.plan_id.clone());
            data.progress = Some(Progress {
                downloaded: 0,
                total: plan.asset.as_ref().map(|asset| asset.size).filter(|size| *size > 0),
            });
            data.sha256 = None;
            data.verified_version = None;
            data.verified_output = None;
            data.error = None;
            data.reused = false;
            data.workspace_id = Some(workspace_id);
            data.context_id = Some(context_id);
            data.started_at = Some(now_ms());
            data.finished_at = None;
            data.sealed = false;
            data.stale_reason = None;
            data.active = true;
            data.note = Some(
                "Downloading the asset and hashing it. Progress is the bytes observed; no percentage is reported."
                    .to_string(),
            );
            let attempt = data.revision;
            data.log.push(format!("== attempt {attempt} =="));
            data.log.push(format!(
                "source {} · asset {}",
                plan.source.url,
                plan.asset.as_ref().map(|asset| asset.name.as_str()).unwrap_or("none")
            ));
            data.log.push(format!("destination {} · staging {}", plan.destination, plan.staging));
            if let Some(asset) = &plan.asset {
                data.log.push(format!("expected sha256 {}", asset.sha256));
            }
        }

        let debugger = Arc::clone(self);
        std::thread::spawn(move || debugger.run(plan));
        Ok(json!({ "started": true }))
    }

    fn run(self: Arc<Self>, plan: Plan) {
        self.install_step(&plan);
        let mut data = self.data.lock().unwrap();
        data.active = false;
    }

    fn install_step(&self, plan: &Plan) {
        let Some(asset) = plan.asset.clone() else {
            self.fail("No asset was selected; nothing was downloaded.");
            return;
        };
        let staging = PathBuf::from(&plan.staging);
        let download = PathBuf::from(format!("{}.download", plan.staging));
        let _ = fs::remove_dir_all(&staging);
        let _ = fs::remove_file(&download);
        if let Some(parent) = staging.parent() {
            if let Err(error) = fs::create_dir_all(parent) {
                self.fail(&format!("Could not create the tools directory: {error}"));
                return;
            }
        }

        let hashed = match self.download(&asset, &download) {
            Ok(hashed) => hashed,
            Err(Stop::Cancelled) => {
                let _ = fs::remove_file(&download);
                self.cancelled("Download cancelled; the partial download was discarded and the staging directory is not ready.");
                return;
            }
            Err(Stop::Failed(message)) => {
                let _ = fs::remove_file(&download);
                self.fail(&message);
                return;
            }
        };
        if hashed != asset.sha256.to_lowercase() {
            let _ = fs::remove_file(&download);
            self.fail(&format!(
                "SHA-256 mismatch: downloaded {hashed}, expected {}. Nothing was extracted or promoted.",
                asset.sha256
            ));
            return;
        }
        self.log(&format!("sha256 {hashed} matched the plan"));
        self.set_state("installing", "Extracting the verified archive into staging.");

        if let Err(message) = extract_zip(&download, &staging) {
            let _ = fs::remove_file(&download);
            self.fail(&format!("{message} The staging directory is not ready."));
            return;
        }
        let _ = fs::remove_file(&download);
        if self.cancel.load(Ordering::SeqCst) {
            self.cancelled("Installation cancelled; staging is partial and is not ready.");
            return;
        }
        // Keep `installing` observable for at least one poll interval.
        self.dwell(300);

        self.set_state("verifying", "Verifying the extracted executable with --version.");
        let Some(binary) = find_executable(&staging, binary_name(&plan.platform)) else {
            self.fail(&format!(
                "The archive did not contain exactly one {}; staging is not ready.",
                binary_name(&plan.platform)
            ));
            return;
        };
        let verified = verify_binary(&binary);
        if verified.state != "ready" {
            self.fail(&format!(
                "Verification failed: {}",
                verified.reason.unwrap_or_else(|| "no verified version".to_string())
            ));
            return;
        }
        if self.cancel.load(Ordering::SeqCst) || self.sealed() {
            self.cancelled("Installation was stopped before promotion; staging is not ready.");
            return;
        }
        if let Err(error) = promote(&staging, Path::new(&plan.destination)) {
            self.fail(&format!("Could not promote the verified tool: {error}"));
            return;
        }
        let mut data = self.data.lock().unwrap();
        data.state = "ready";
        data.sha256 = verified.sha256.clone();
        data.verified_version = verified.verified_version.clone();
        data.verified_output = verified.reason.clone();
        data.progress = None;
        data.error = None;
        data.note = Some(
            "Executable and version verified after a hash-checked extraction. This is not DAP evidence; debugging is ticket 14."
                .to_string(),
        );
        data.finished_at = Some(now_ms());
        if let Some(output) = &verified.reason {
            data.log.push(format!("$ {} --version", verified.binary));
            data.log.push(output.clone());
        }
        if let Some(sha) = &verified.sha256 {
            data.log.push(format!("executable sha256 {sha}"));
        }
        data.log.push(
            "Verified executable promoted to the destination. DAP compatibility is not proven."
                .to_string(),
        );
    }

    /// Downloads the asset to `path`, validating every origin and redirect, while hashing.
    fn download(&self, asset: &AssetView, path: &Path) -> Result<String, Stop> {
        let agent = ureq::AgentBuilder::new()
            .redirects(0)
            .timeout_connect(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .build();
        let mut url = asset.url.clone();
        for _ in 0..=MAX_REDIRECTS {
            if !allowed_url(&url) {
                return Err(Stop::Failed(format!(
                    "Refused to download from {url}: only HTTPS on the official release hosts ({}) is allowed. Nothing was downloaded.",
                    ALLOWED_HOSTS.join(", ")
                )));
            }
            self.log(&format!("GET {url}"));
            match agent.get(&url).call() {
                Ok(response) if (300..400).contains(&response.status()) => {
                    let Some(location) = response.header("location") else {
                        return Err(Stop::Failed(format!(
                            "The server answered {} without a Location; nothing was downloaded.",
                            response.status()
                        )));
                    };
                    url = resolve_location(&url, location);
                }
                Ok(response) => {
                    let total = response
                        .header("content-length")
                        .and_then(|value| value.parse::<u64>().ok())
                        .or(Some(asset.size).filter(|size| *size > 0));
                    if let Some(total) = total {
                        if total > MAX_DOWNLOAD_BYTES {
                            return Err(Stop::Failed(format!(
                                "The asset reports {total} bytes, above the {MAX_DOWNLOAD_BYTES}-byte limit."
                            )));
                        }
                    }
                    return self.copy_bounded(response.into_reader(), path, total, asset);
                }
                Err(ureq::Error::Status(code, response)) if (300..400).contains(&code) => {
                    let Some(location) = response.header("location") else {
                        return Err(Stop::Failed(format!(
                            "The server answered {code} without a Location; nothing was downloaded."
                        )));
                    };
                    url = resolve_location(&url, location);
                }
                Err(ureq::Error::Status(code, _)) => {
                    return Err(Stop::Failed(format!(
                        "The download failed with HTTP {code}; nothing was extracted."
                    )))
                }
                Err(ureq::Error::Transport(error)) => {
                    return Err(Stop::Failed(format!(
                        "The download failed: {error}; nothing was extracted."
                    )))
                }
            }
        }
        Err(Stop::Failed(format!(
            "More than {MAX_REDIRECTS} redirects; the download was refused."
        )))
    }

    fn copy_bounded(
        &self,
        mut reader: impl Read,
        path: &Path,
        total: Option<u64>,
        asset: &AssetView,
    ) -> Result<String, Stop> {
        let mut file = fs::File::create(path)
            .map_err(|error| Stop::Failed(format!("Could not create the download file: {error}")))?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; CHUNK];
        let mut downloaded: u64 = 0;
        loop {
            if self.cancel.load(Ordering::SeqCst) || self.sealed() {
                return Err(Stop::Cancelled);
            }
            let read = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) => {
                    return Err(Stop::Failed(format!(
                        "The download was interrupted: {error}; nothing was extracted."
                    )))
                }
            };
            downloaded += read as u64;
            if downloaded > MAX_DOWNLOAD_BYTES {
                return Err(Stop::Failed(format!(
                    "The download exceeded the {MAX_DOWNLOAD_BYTES}-byte limit; it was stopped."
                )));
            }
            hasher.update(&buffer[..read]);
            file.write_all(&buffer[..read])
                .map_err(|error| Stop::Failed(format!("Could not write the download: {error}")))?;
            {
                let mut data = self.data.lock().unwrap();
                data.progress = Some(Progress { downloaded, total });
            }
        }
        file.flush()
            .map_err(|error| Stop::Failed(format!("Could not flush the download: {error}")))?;
        if asset.size > 0 && downloaded != asset.size {
            return Err(Stop::Failed(format!(
                "The download is {downloaded} bytes but the release metadata says {}; the archive was rejected.",
                asset.size
            )));
        }
        let mut data = self.data.lock().unwrap();
        data.progress = Some(Progress { downloaded, total });
        data.log
            .push(format!("downloaded {downloaded} bytes (measured; no percentage)"));
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn dwell(&self, ms: u64) {
        let deadline = now_ms() + ms;
        while now_ms() < deadline {
            if self.cancel.load(Ordering::SeqCst) {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn set_state(&self, state: &'static str, note: &str) {
        let mut data = self.data.lock().unwrap();
        if data.sealed {
            return;
        }
        data.state = state;
        data.note = Some(note.to_string());
        data.log.push(note.to_string());
    }

    fn log(&self, line: &str) {
        let mut data = self.data.lock().unwrap();
        data.log.push(line.to_string());
    }

    fn sealed(&self) -> bool {
        self.data.lock().unwrap().sealed
    }

    fn fail(&self, message: &str) {
        let mut data = self.data.lock().unwrap();
        if data.sealed {
            return;
        }
        data.state = "failed";
        data.error = Some(message.to_string());
        data.note = Some(format!(
            "{} The staging directory {} is not ready.",
            message, data.staging
        ));
        data.finished_at = Some(now_ms());
        data.progress = None;
        data.log.push(message.to_string());
    }

    fn cancelled(&self, note: &str) {
        let mut data = self.data.lock().unwrap();
        if data.sealed {
            return;
        }
        data.state = "cancelled";
        data.note = Some(note.to_string());
        data.finished_at = Some(now_ms());
        data.progress = None;
        data.log.push(note.to_string());
    }

    /// Safety action; never requires trust.
    pub fn cancel(&self) -> HostResult<()> {
        let mut data = self.data.lock().unwrap();
        if data.state == "planned" || data.state == "failed" {
            data.state = "cancelled";
            data.note = Some(
                "Plan cancelled; no download and no process were started.".to_string(),
            );
            data.finished_at = Some(now_ms());
            data.log
                .push("Plan cancelled; nothing was downloaded or executed.".to_string());
            return Ok(());
        }
        if !matches!(data.state, "downloading" | "installing" | "verifying") {
            return Err(HostError::new(
                "invalid_state",
                "No netcoredbg plan or installation to cancel.",
            ));
        }
        self.cancel.store(true, Ordering::SeqCst);
        data.state = "cancelled";
        // `active` stays true until the worker observes the request; retry waits for it.
        data.note = Some(format!(
            "Cancellation requested; the staging directory {} is not ready and is never promoted as-is.",
            data.staging
        ));
        data.finished_at = Some(now_ms());
        data.progress = None;
        data.log
            .push("Cancellation requested; no partial result is reported as ready.".to_string());
        Ok(())
    }

    /// Records the session-scoped experimental opt-in in the durable log.
    pub fn log_flag(&self, enabled: bool) {
        let mut data = self.data.lock().unwrap();
        data.log.push(if enabled {
            "Apple Silicon experimental flag enabled by explicit opt-in. It neither installs netcoredbg nor proves compatibility."
                .to_string()
        } else {
            "Apple Silicon experimental flag disabled.".to_string()
        });
    }

    /// A workspace boundary stops a real download/extraction and keeps what was observed.
    pub fn seal(&self, reason: &str) {
        {
            let mut data = self.data.lock().unwrap();
            if !matches!(data.state, "downloading" | "installing" | "verifying") || data.sealed {
                return;
            }
            data.sealed = true;
            data.stale_reason = Some(reason.to_string());
            data.state = "cancelled";
            data.finished_at = Some(now_ms());
            data.progress = None;
            data.note = Some(format!(
                "Stopped at a workspace boundary; {} is partial and is not ready.",
                data.staging
            ));
        }
        self.cancel.store(true, Ordering::SeqCst);
    }
}

impl Default for Debugger {
    fn default() -> Self {
        Self::new()
    }
}

enum Stop {
    Cancelled,
    Failed(String),
}

/// Extracts a zip archive under `target` with explicit traversal, symlink, entry and size
/// checks. A rejected entry aborts the whole extraction.
fn extract_zip(archive: &Path, target: &Path) -> Result<(), String> {
    let file = fs::File::open(archive).map_err(|error| format!("Could not open the archive: {error}"))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|error| format!("The archive is not a readable zip: {error}."))?;
    if zip.len() > MAX_ENTRIES {
        return Err(format!(
            "The archive has {} entries, above the {MAX_ENTRIES}-entry limit.",
            zip.len()
        ));
    }
    fs::create_dir_all(target).map_err(|error| format!("Could not create staging: {error}"))?;
    let mut declared: u64 = 0;
    let mut written: u64 = 0;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| format!("The archive entry {index} is unreadable: {error}"))?;
        let Some(relative) = entry.enclosed_name() else {
            return Err(format!(
                "Rejected archive entry {:?}: it escapes the extraction directory.",
                entry.name()
            ));
        };
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|part| !matches!(part, Component::Normal(_) | Component::CurDir))
        {
            return Err(format!(
                "Rejected archive entry {:?}: absolute paths and parent segments are not allowed.",
                entry.name()
            ));
        }
        if let Some(mode) = entry.unix_mode() {
            let kind = mode & 0o170000;
            if kind == 0o120000 {
                return Err(format!(
                    "Rejected archive entry {:?}: symlinks are not extracted.",
                    entry.name()
                ));
            }
            if kind != 0 && kind != 0o100000 && kind != 0o040000 {
                return Err(format!(
                    "Rejected archive entry {:?}: only regular files and directories are extracted.",
                    entry.name()
                ));
            }
        }
        let destination = target.join(&relative);
        if !destination.starts_with(target) {
            return Err(format!(
                "Rejected archive entry {:?}: the target is outside staging.",
                entry.name()
            ));
        }
        if entry.is_dir() {
            fs::create_dir_all(&destination)
                .map_err(|error| format!("Could not create {destination:?}: {error}"))?;
            continue;
        }
        declared = declared.saturating_add(entry.size());
        if declared > MAX_EXTRACTED_BYTES {
            return Err(format!(
                "The extracted tree would exceed the {MAX_EXTRACTED_BYTES}-byte limit."
            ));
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create {parent:?}: {error}"))?;
        }
        let mut out = fs::File::create(&destination)
            .map_err(|error| format!("Could not write {destination:?}: {error}"))?;
        // The declared size is a cheap guard; the bytes actually written are counted too.
        let room = MAX_EXTRACTED_BYTES.saturating_sub(written).saturating_add(1);
        let copied = std::io::copy(&mut entry.by_ref().take(room), &mut out)
            .map_err(|error| format!("Could not extract {destination:?}: {error}"))?;
        written += copied;
        if written > MAX_EXTRACTED_BYTES {
            return Err("The extracted tree exceeds the size limit.".to_string());
        }
        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&destination, fs::Permissions::from_mode(mode & 0o777));
        }
    }
    Ok(())
}

/// Finds exactly one executable by name, at any depth of the extracted tree.
fn find_executable(root: &Path, name: &str) -> Option<PathBuf> {
    fn walk(dir: &Path, name: &str, found: &mut Vec<PathBuf>, visited: &mut usize) {
        if *visited > 500 {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            *visited += 1;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, name, found, visited);
            } else if path.file_name().and_then(|value| value.to_str()) == Some(name) {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    let mut visited = 0;
    walk(root, name, &mut found, &mut visited);
    if found.len() == 1 {
        found.pop()
    } else {
        None
    }
}

/// Same promotion semantics as csharp-ls: the destination is replaced only after the staged
/// tree is verified, and a failed rename restores the previous destination.
fn promote(staging: &Path, destination: &Path) -> Result<(), String> {
    if !staging.is_dir() {
        return Err("the staging directory does not exist".to_string());
    }
    let backup = if destination.exists() {
        let backup = PathBuf::from(format!(
            "{}.old-{}-{}",
            destination.to_string_lossy(),
            std::process::id(),
            now_ms()
        ));
        fs::rename(destination, &backup)
            .map_err(|error| format!("could not move the previous destination aside: {error}"))?;
        Some(backup)
    } else {
        None
    };
    if let Err(error) = fs::rename(staging, destination) {
        if let Some(backup) = &backup {
            let _ = fs::rename(backup, destination);
        }
        return Err(format!("could not move staging into place: {error}"));
    }
    if let Some(backup) = backup {
        let _ = fs::remove_dir_all(backup);
    }
    Ok(())
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

    #[test]
    fn plan_lists_only_the_assets_published_by_the_official_release() {
        assert_eq!(PUBLISHED.len(), 4);
        for asset in PUBLISHED {
            assert_eq!(asset.sha256.len(), 64, "{}", asset.name);
            assert!(asset.sha256.chars().all(|c| c.is_ascii_hexdigit()));
            assert!(asset.name.starts_with("netcoredbg-"), "{}", asset.name);
        }
        assert!(published("osx-x64").is_none());
        assert_eq!(published("win64").unwrap().size, 3_524_161);
    }

    #[test]
    fn macos_arm64_is_blocked_and_explains_the_community_build() {
        let plan = plan("macos", "aarch64", false);
        assert_eq!(plan.platform, "osx-arm64");
        assert!(plan.blocked_reason.is_some());
        let asset = plan.asset.unwrap();
        assert_eq!(asset.availability, "unverified");
        assert!(asset.reason.unwrap().contains("community supported"));
        assert!(plan.destination.ends_with("tools/netcoredbg/3.2.0-1092"));
        assert!(plan.staging.ends_with("3.2.0-1092.staging"));
        assert_eq!(plan.availability, "unverified");
    }

    #[test]
    fn intel_and_unknown_platforms_have_no_asset_and_no_substitution() {
        for (os, arch) in [
            ("macos", "x86_64"),
            ("windows", "aarch64"),
            ("freebsd", "x86_64"),
        ] {
            let plan = plan(os, arch, true);
            assert!(plan.asset.is_none(), "{os}/{arch}");
            assert!(plan.blocked_reason.is_some(), "{os}/{arch}");
        }
        let plan = plan("macos", "x86_64", false);
        assert!(plan.blocked_reason.unwrap().contains("no osx-x64 asset"));
    }

    #[test]
    fn windows_x64_selects_the_published_win64_asset() {
        let plan = plan("windows", "x86_64", false);
        assert!(plan.blocked_reason.is_none());
        let asset = plan.asset.unwrap();
        assert_eq!(asset.name, "netcoredbg-win64.zip");
        assert_eq!(asset.url, format!("{DOWNLOAD_BASE}netcoredbg-win64.zip"));
        assert_eq!(plan.availability, "available");
    }

    #[test]
    fn plan_id_changes_with_every_material_field() {
        let plan = plan("windows", "x86_64", false);
        let mut changed = plan.clone();
        changed.destination = "/tmp/other".to_string();
        assert_ne!(plan.plan_id, plan_id(&changed));
        changed = plan.clone();
        changed.asset.as_mut().unwrap().sha256 = "0".repeat(64);
        assert_ne!(plan.plan_id, plan_id(&changed));
        changed = plan.clone();
        changed.permissions.push("extra".to_string());
        assert_ne!(plan.plan_id, plan_id(&changed));
        assert_eq!(plan.plan_id, plan_id(&plan));
    }

    #[test]
    fn only_official_https_hosts_are_fetched() {
        assert!(allowed_url(&format!("{DOWNLOAD_BASE}netcoredbg-win64.zip")));
        assert!(allowed_url(
            "https://objects.githubusercontent.com/github-production-release-asset/x"
        ));
        assert!(!allowed_url("http://github.com/Samsung/netcoredbg/x.zip"));
        assert!(!allowed_url("https://example.invalid/x.zip"));
        assert!(!allowed_url("https://github.com.evil.invalid/x.zip"));
        assert!(!allowed_url("file:///etc/passwd"));
        assert_eq!(
            resolve_location("https://github.com/a/b", "/c/d"),
            "https://github.com/c/d"
        );
        assert_eq!(
            resolve_location("https://github.com/a/b", "https://objects.githubusercontent.com/x"),
            "https://objects.githubusercontent.com/x"
        );
    }

    #[test]
    fn gate_states_are_honest_per_platform() {
        assert_eq!(gate("macos", "aarch64", false, false).state, "experimental_disabled");
        assert_eq!(gate("macos", "aarch64", true, false).state, "unavailable");
        assert!(!gate("macos", "aarch64", true, false).installable);
        assert!(!gate("macos", "x86_64", true, false).installable);
        assert!(gate("windows", "x86_64", false, false).installable);
        assert!(!gate("linux", "x86_64", true, false).installable);
    }

    #[test]
    fn archive_names_are_rejected_before_they_escape() {
        for name in ["../evil.txt", "a/../../evil.txt", "/etc/passwd"] {
            let path = Path::new(name);
            let safe = path
                .components()
                .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
            assert!(!safe, "{name}");
        }
    }
}
