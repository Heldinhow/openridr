//! csharp-ls installation with an explicit, revalidated plan.
//!
//! The host never installs a global tool: the only command is `dotnet tool install` with
//! `--tool-path` inside the app home. A candidate is installed into a staging directory,
//! promoted only after the process exits 0, then verified; no state becomes `ready` before
//! the launcher exists and its `--version` output contains the pinned version.

use crate::bundled;
use crate::error::{HostError, HostResult};
use crate::logbuf::{self, LogBuffer};
use crate::process::{ProcessStatus, Registry, Termination};
use crate::sdk::{AnalysisRuntime, Install, Inventory, Version};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256, Sha512};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const TOOL: &str = "csharp-ls";
pub const VERSION: &str = "0.28.0";
pub const NUGET_SOURCE: &str = "https://api.nuget.org/v3/index.json";
pub const NUGET_PACKAGE: &str = "https://www.nuget.org/packages/csharp-ls/0.28.0";
const PACKAGE_URL: &str =
    "https://api.nuget.org/v3-flatcontainer/csharp-ls/0.28.0/csharp-ls.0.28.0.nupkg";
const PACKAGE_SHA512: &str = "a138b9c8f9de9dd6343e9b0d0c8f4d51637bba7e38606b7c2b08b1d91e8eefa39f12123057f6625f425812b4361e52841bd2856704ec127ff50144ce05170a99";
const PACKAGE_MAX_BYTES: u64 = 32 * 1024 * 1024;

/// A stuck launcher must fail visibly instead of keeping setup on "Preparando…" forever.
const VERIFY_TIMEOUT: Duration = Duration::from_secs(60);

/// The pinned package: URL and expected SHA-512. A debug build may point both at a local server so
/// the installer's real download path (measured progress, size limit, hash check) can be exercised
/// without NuGet.org; release builds always use the compiled-in pin.
fn package_source() -> (String, String) {
    #[cfg(debug_assertions)]
    if let (Ok(url), Ok(sha512)) = (
        std::env::var("OPENRIDR_TOOLS_PACKAGE_URL"),
        std::env::var("OPENRIDR_TOOLS_PACKAGE_SHA512"),
    ) {
        return (url, sha512);
    }
    (PACKAGE_URL.to_string(), PACKAGE_SHA512.to_string())
}

/// True when a debug build redirected the pinned package. The probe installer is then only the
/// install step: the verified download still runs, because it is what is being exercised.
fn package_redirected() -> bool {
    #[cfg(debug_assertions)]
    {
        std::env::var_os("OPENRIDR_TOOLS_PACKAGE_URL").is_some()
            && std::env::var_os("OPENRIDR_TOOLS_PACKAGE_SHA512").is_some()
    }
    #[cfg(not(debug_assertions))]
    {
        false
    }
}

/// Lowercase hex for a digest. `sha2` 0.11 hands back a hybrid-array, which has no `LowerHex`.
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn verify_package_hash(bytes: &[u8], expected: &str) -> Result<(), String> {
    let actual = hex(&Sha512::digest(bytes));
    if actual == expected {
        Ok(())
    } else {
        Err(
            "csharp-ls package SHA-512 mismatch; installation aborted. Retry the setup."
                .to_string(),
        )
    }
}

fn store_verified_package(
    temp: &Path,
    bytes: &[u8],
    expected: &str,
) -> Result<(PathBuf, PathBuf), String> {
    verify_package_hash(bytes, expected)?;
    let feed = temp.join("verified-feed");
    if feed.exists() {
        fs::remove_dir_all(&feed).map_err(|error| error.to_string())?;
    }
    fs::create_dir_all(&feed).map_err(|error| error.to_string())?;
    fs::write(feed.join("csharp-ls.0.28.0.nupkg"), bytes).map_err(|error| error.to_string())?;
    let config = temp.join("offline.nuget.config");
    fs::write(
        &config,
        "<configuration><packageSources><clear /></packageSources></configuration>",
    )
    .map_err(|error| error.to_string())?;
    Ok((feed, config))
}

fn prepare_local_package(
    temp: &Path,
    progress: impl Fn(u64, Option<u64>) -> bool,
) -> Result<(PathBuf, PathBuf), String> {
    let (url, sha512) = package_source();
    // `fetch` follows the redirects itself so every hop is routed and guarded again. The download is
    // bounded by inactivity, never by a total deadline: a slow transfer that keeps receiving bytes
    // must finish (#161).
    let response = crate::http::fetch(&url, crate::http::DOWNLOAD_STALL)
        .map_err(|error| format!("Could not download csharp-ls: {error}"))?;
    let total = response
        .header("Content-Length")
        .and_then(|value| value.parse().ok());
    let mut reader = response.into_reader().take(PACKAGE_MAX_BYTES + 1);
    let mut bytes = Vec::new();
    let mut chunk = [0; 65536];
    loop {
        let count = reader
            .read(&mut chunk)
            .map_err(|error| download_read_error(&url, &error))?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..count]);
        if bytes.len() as u64 > PACKAGE_MAX_BYTES {
            return Err("csharp-ls package exceeds the size limit".to_string());
        }
        if !progress(bytes.len() as u64, total) {
            return Err("Installation cancelled.".to_string());
        }
    }
    store_verified_package(temp, &bytes, &sha512)
}
/// A body read that timed out is the same stall as a server that stopped answering: the person gets
/// the download sentence, not the transport's.
fn download_read_error(url: &str, error: &std::io::Error) -> String {
    if crate::http::is_stall(error) {
        format!(
            "Could not download csharp-ls: {}",
            crate::http::stalled(url, crate::http::DOWNLOAD_STALL)
        )
    } else {
        format!("Could not read csharp-ls package: {error}")
    }
}

const SIGNATURE_LIMITATION: &str = "The NuGet package is checked against the pinned SHA-512 before local installation. dotnet reports skipping package signature verification. The installed launcher SHA-256 and version are recorded separately.";

/// App-scoped root. Tests and isolated E2E runs override it with `OPENRIDR_HOME`.
pub fn app_home() -> PathBuf {
    if let Some(value) = std::env::var_os("OPENRIDR_HOME") {
        return PathBuf::from(value);
    }
    // A test binary must never read or write the real app home of whoever ran cargo test.
    if cfg!(test) || cargo_test_binary() {
        return test_app_home();
    }
    #[cfg(target_os = "macos")]
    {
        home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Application Support/OpenRidr")
    }
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .or_else(home_dir)
            .unwrap_or_else(|| PathBuf::from("."))
            .join("OpenRidr")
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join(".local/share")))
            .unwrap_or_else(|| PathBuf::from("."))
            .join("openridr")
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Integration tests link the library without `cfg(test)`, so the Cargo layout is the signal.
fn cargo_test_binary() -> bool {
    std::env::current_exe().is_ok_and(|exe| {
        exe.parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some("deps")
            && exe
                .components()
                .any(|part| part.as_os_str().to_str() == Some("target"))
    })
}

fn test_app_home() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("openridr-test-home-{}", std::process::id()));
    fs::create_dir_all(&dir).expect("create the per-process test app home");
    dir
}

pub fn tool_root() -> PathBuf {
    app_home().join("tools/csharp-ls")
}

pub fn destination() -> PathBuf {
    tool_root().join("0.28")
}

pub fn staging() -> PathBuf {
    tool_root().join("0.28.staging")
}

fn child_home() -> PathBuf {
    app_home().join("toolhome")
}

/// The child `dotnet` resolves its home and NuGet state from `USERPROFILE`/`APPDATA` on Windows,
/// not from `HOME`; without them `NuGet.Configuration.ConfigurationDefaults` throws before the
/// install starts. The isolated home keeps every write under the app home on both platforms.
fn isolate_child_home(command: &mut Command, home: &Path, temp: &Path) {
    #[cfg(windows)]
    {
        command
            .env("USERPROFILE", home)
            .env("APPDATA", home.join("AppData/Roaming"))
            .env("LOCALAPPDATA", home.join("AppData/Local"))
            .env("TEMP", temp)
            .env("TMP", temp);
    }
    #[cfg(not(windows))]
    {
        let _ = (home, temp);
    }
}

/// The launcher runs outside the workspace. The plan only announces this path; anything that
/// spawns uses `neutral_cwd_ready()`.
fn neutral_cwd() -> PathBuf {
    std::env::temp_dir().join("openridr-neutral")
}

/// `current_dir` of a missing path fails the spawn before the launcher runs, which on a fresh
/// machine reported a valid tool as invalid (issue #93). Created here, in the one place that
/// spawns use, so no caller has to remember it.
fn neutral_cwd_ready() -> PathBuf {
    neutral_cwd_in(&std::env::temp_dir())
}

fn neutral_cwd_in(root: &Path) -> PathBuf {
    let dir = root.join("openridr-neutral");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn binary_name() -> &'static str {
    if cfg!(windows) {
        "csharp-ls.exe"
    } else {
        "csharp-ls"
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub name: &'static str,
    pub url: &'static str,
    pub package_url: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Existing {
    /// `ready` (verified in this session) | `present` (exists, not verified) | `absent`
    pub state: &'static str,
    pub binary: String,
    pub verified_version: Option<String>,
    pub output: Option<String>,
    pub sha256: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Plan {
    pub tool: &'static str,
    pub version: &'static str,
    pub package: &'static str,
    pub source: Source,
    pub os: &'static str,
    pub arch: &'static str,
    pub destination: String,
    pub staging: String,
    pub dotnet_root: String,
    pub argv: Vec<String>,
    pub cwd: String,
    pub permissions: Vec<String>,
    pub integrity: &'static str,
    pub existing: Existing,
    pub plan_id: String,
}

impl Plan {
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).unwrap()
    }

    /// A verified install or reuse changes what the destination holds; the plan must say so.
    fn record_verified(&mut self, verified: &Existing) {
        self.existing = Existing {
            state: "present",
            binary: verified.binary.clone(),
            verified_version: verified.verified_version.clone(),
            output: verified.output.clone(),
            sha256: verified.sha256.clone(),
            reason: Some(
                "Verified by the last confirmed install; confirming reuses it while it stays valid."
                    .to_string(),
            ),
        };
    }
}

fn material(plan: &Plan) -> Vec<String> {
    vec![
        plan.tool.to_string(),
        plan.version.to_string(),
        plan.package.to_string(),
        plan.source.name.to_string(),
        plan.source.url.to_string(),
        plan.source.package_url.to_string(),
        plan.os.to_string(),
        plan.arch.to_string(),
        plan.destination.clone(),
        plan.staging.clone(),
        plan.dotnet_root.clone(),
        plan.argv.join("\n"),
        plan.cwd.clone(),
        plan.permissions.join("\n"),
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

/// Test seam, enabled only with `OPENRIDR_ENABLE_PROBE=1`: a controlled installer stands in
/// for the SDK muxer so an E2E can observe progress, failure and retry without NuGet.org.
#[cfg(debug_assertions)]
fn probe_muxer() -> Option<PathBuf> {
    if std::env::var("OPENRIDR_ENABLE_PROBE").as_deref() != Ok("1") {
        return None;
    }
    std::env::var_os("OPENRIDR_TOOLS_MUXER").map(PathBuf::from)
}

#[cfg(not(debug_assertions))]
fn probe_muxer() -> Option<PathBuf> {
    // Release builds never let an environment variable replace the verified package feed.
    None
}

/// Builds the plan from the detected runtimes and SDKs. `dotnet tool install` runs on any
/// installed SDK; the launcher is verified against the .NET 10 runtime root. A bundled pair has
/// no plan at all: the bundle is the only source and it cannot be installed or reinstalled.
pub fn plan(inventory: &Inventory) -> HostResult<Plan> {
    match bundled::availability() {
        bundled::Availability::Bundled(_) => {
            return Err(HostError::with_details(
                "invalid_state",
                "csharp-ls and its .NET runtime are bundled with OpenRidr; there is nothing to download or install.",
                json!({}),
            ));
        }
        bundled::Availability::Damaged(reason) => {
            // Fail closed: a distributed app with a broken bundle never plans a download.
            return Err(HostError::with_details(
                "bundled_incomplete",
                reason,
                json!({}),
            ));
        }
        bundled::Availability::Absent => {}
    }
    let runtime = AnalysisRuntime::from_runtimes(&inventory.runtimes);
    let Some(runtime_root) = runtime.root.clone() else {
        return Err(HostError::with_details(
            "sdk_missing",
            "The csharp-ls plan needs the .NET 10 runtime; no plan was produced.",
            json!({ "action": { "method": "sdk.redetect", "params": {} } }),
        ));
    };
    let Some(install) = tool_install(&inventory.installs) else {
        return Err(HostError::with_details(
            "sdk_missing",
            "The csharp-ls plan needs a .NET SDK to run `dotnet tool install`; no plan was produced.",
            json!({ "action": { "method": "sdk.redetect", "params": {} } }),
        ));
    };
    let muxer = probe_muxer().unwrap_or_else(|| {
        install.root.join(if cfg!(windows) {
            "dotnet.exe"
        } else {
            "dotnet"
        })
    });
    if !muxer.is_file() {
        return Err(HostError::new(
            "sdk_missing",
            format!(
                "The install command {} does not exist.",
                muxer.to_string_lossy()
            ),
        ));
    }
    let destination = destination();
    let staging = staging();
    let existing = inspect_existing(&destination);
    let mut plan = Plan {
        tool: TOOL,
        version: VERSION,
        package: TOOL,
        source: Source {
            name: "NuGet.org",
            url: NUGET_SOURCE,
            package_url: NUGET_PACKAGE,
        },
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        destination: destination.to_string_lossy().to_string(),
        staging: staging.to_string_lossy().to_string(),
        dotnet_root: runtime_root,
        argv: vec![
            muxer.to_string_lossy().to_string(),
            "tool".to_string(),
            "install".to_string(),
            TOOL.to_string(),
            "--version".to_string(),
            VERSION.to_string(),
            "--tool-path".to_string(),
            staging.to_string_lossy().to_string(),
        ],
        cwd: neutral_cwd().to_string_lossy().to_string(),
        permissions: vec![
            format!("network: {PACKAGE_URL} (NuGet.org over HTTPS; the installer uses only the verified local package)"),
            format!("write: {}", tool_root().to_string_lossy()),
            format!(
                "write: {} (isolated HOME, NuGet cache and first-run files)",
                child_home().to_string_lossy()
            ),
            "no global tool install (`--tool-path` only), no elevation, no system SDK install, no repository changes"
                .to_string(),
        ],
        integrity: SIGNATURE_LIMITATION,
        existing,
        plan_id: String::new(),
    };
    plan.plan_id = plan_id(&plan);
    Ok(plan)
}

/// `dotnet tool install` only needs an SDK, any version; the tool runs on the 10 runtime.
fn tool_install(installs: &[Install]) -> Option<&Install> {
    installs
        .iter()
        .filter_map(|install| Version::parse(&install.version).map(|version| (version, install)))
        .max_by(|(left, _), (right, _)| left.cmp(right))
        .map(|(_, install)| install)
}

/// NuGet scratch lives under the app home, next to staging, so observed bytes stay local.
fn install_temp(staging: &str) -> PathBuf {
    Path::new(staging)
        .parent()
        .map(|parent| parent.join("tmp"))
        .unwrap_or_else(|| tool_root().join("tmp"))
}

/// Cheap, non-executable inspection used by the plan. It never runs the launcher.
pub fn inspect_existing(destination: &Path) -> Existing {
    let binary = destination.join(binary_name());
    if binary.is_file() {
        return Existing {
            state: "present",
            binary: binary.to_string_lossy().to_string(),
            verified_version: None,
            output: None,
            sha256: None,
            reason: Some(
                "A launcher already exists. It is verified only after you confirm; a valid installation is reused, an invalid one is replaced after a verified staging install."
                    .to_string(),
            ),
        };
    }
    Existing {
        state: "absent",
        binary: binary.to_string_lossy().to_string(),
        verified_version: None,
        output: None,
        sha256: None,
        reason: Some("No launcher exists at the destination.".to_string()),
    }
}

/// Runs the installed launcher without touching the workspace. Used by the plan, before
/// reusing an existing tool, and after promotion. The launcher needs the .NET 10 runtime root,
/// not the SDK that installed it.
pub fn verify_installed(destination: &Path, runtime_root: &Path) -> Existing {
    let binary = destination.join(binary_name());
    let binary_text = binary.to_string_lossy().to_string();
    if !binary.is_file() {
        return Existing {
            state: "absent",
            binary: binary_text,
            verified_version: None,
            output: None,
            sha256: None,
            reason: Some("No launcher exists at the destination.".to_string()),
        };
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(&binary)
            .map(|meta| meta.permissions().mode() & 0o111 == 0)
            .unwrap_or(true)
        {
            return Existing {
                state: "invalid",
                binary: binary_text,
                verified_version: None,
                output: None,
                sha256: None,
                reason: Some("The launcher is not executable.".to_string()),
            };
        }
    }

    let home = child_home();
    let _ = fs::create_dir_all(&home);
    let cwd = neutral_cwd_ready();
    let output = Command::new(&binary)
        .arg("--version")
        .current_dir(&cwd)
        .env("HOME", &home)
        .env("DOTNET_ROOT", runtime_root)
        .env("DOTNET_MULTILEVEL_LOOKUP", "0")
        .env("DOTNET_CLI_UI_LANGUAGE", "en")
        .env("DOTNET_NOLOGO", "1")
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .env_remove("NUGET_PACKAGES")
        .env_remove("NUGET_HTTP_CACHE_PATH")
        .env_remove("DOTNET_CLI_HOME")
        .output();

    let Ok(output) = output else {
        return Existing {
            state: "invalid",
            binary: binary_text,
            verified_version: None,
            output: None,
            sha256: None,
            reason: Some(format!(
                "Could not run {} --version.",
                binary.to_string_lossy()
            )),
        };
    };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let text = text.trim().to_string();
    let exit = output.status.code();
    if !output.status.success() || !text.contains(VERSION) {
        return Existing {
            state: "invalid",
            binary: binary_text,
            verified_version: None,
            output: Some(text),
            sha256: None,
            reason: Some(format!(
                "--version did not report {VERSION} (exit {}).",
                exit.map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            )),
        };
    }
    Existing {
        state: "ready",
        sha256: sha256_file(&binary),
        binary: binary_text,
        verified_version: Some(VERSION.to_string()),
        output: Some(text),
        reason: None,
    }
}

fn sha256_file(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(hex(&Sha256::digest(&bytes)))
}

/// Records the verified launcher. The package SHA-512 pins the source archive, but there is no
/// per-DLL manifest: a modified launcher is detected by its hash; other files in the tool
/// directory are not individually re-hashed. Documented limitation in docs/decisions.md.
fn record_installation(destination: &Path, launcher_hash: &str) -> std::io::Result<()> {
    fs::write(
        destination.join(".openridr-integrity"),
        format!("{PACKAGE_SHA512}\n{launcher_hash}\n"),
    )
}

fn persisted_installation_valid(destination: &Path) -> bool {
    let Ok(record) = fs::read_to_string(destination.join(".openridr-integrity")) else {
        return false;
    };
    let mut lines = record.lines();
    let package = lines.next();
    let launcher = lines.next();
    package == Some(PACKAGE_SHA512)
        && launcher.is_some()
        && launcher == sha256_file(&destination.join(binary_name())).as_deref()
        && lines.next().is_none()
}

/// Bytes currently on disk under `root`; symlinks are not followed.
/// Progress may be published from bytes on disk only while the install is still fetching and no
/// measured download owns the number. `dotnet tool install` writes several copies of the same
/// package, so its disk writes are not a download and never replace `Content-Length` progress.
fn observes_disk_bytes(state: &str, measured: bool) -> bool {
    state == "downloading" && !measured
}

fn observed_bytes(roots: &[PathBuf]) -> u64 {
    // A root inside another root would count the same files twice; only the outermost stay.
    let mut outermost: Vec<&PathBuf> = Vec::new();
    for root in roots {
        if roots.iter().any(|other| {
            other != root && root.starts_with(other) && root.as_path() != other.as_path()
        }) {
            continue;
        }
        outermost.push(root);
    }
    outermost.iter().map(|root| directory_bytes(root)).sum()
}

fn directory_bytes(root: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(root) else {
        return 0;
    };
    let mut total = 0;
    for entry in entries.flatten() {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            total += directory_bytes(&entry.path());
        } else if kind.is_file() {
            if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    /// Bytes of the pinned package actually read, or, when no download was measured, bytes
    /// observed on disk. Copies written by NuGet to its cache and its staging directory are the
    /// same bytes, so they are never summed into this number.
    pub downloaded: u64,
    /// `Content-Length` of the pinned package when the download measured it; never invented.
    pub total: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolView {
    pub state: &'static str,
    pub version: &'static str,
    pub destination: String,
    pub staging: String,
    pub bundled: bool,
    pub plan: Option<Plan>,
    pub log: String,
    pub log_truncated: bool,
    pub progress: Option<Progress>,
    pub exit: Option<i32>,
    pub termination: Option<Termination>,
    pub process_id: Option<u64>,
    pub awaiting_decision: bool,
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
}

struct Data {
    revision: u64,
    state: &'static str,
    destination: String,
    staging: String,
    runtime_root: String,
    /// A verified pair inside the app bundle; it replaces the download plan entirely.
    bundled: Option<BundledTool>,
    /// This build ships (or must ship) the pair, even when verification failed. The UI uses it
    /// to refuse Reinstall instead of offering an action that cannot work.
    bundle_expected: bool,
    /// The bundled pair is verified at most once per session: it cannot be reinstalled.
    bundled_attempted: bool,
    plan: Option<Plan>,
    confirmed_plan_id: Option<String>,
    log: LogBuffer,
    progress: Option<Progress>,
    /// True while `progress` is the measured package download. The disk observation must not
    /// overwrite it: NuGet writes several copies of the same bytes (verified feed, package
    /// cache, extracted content), so summing directories reports a size that is not a download.
    progress_measured: bool,
    progress_base: u64,
    progress_peak: u64,
    exit: Option<i32>,
    termination: Option<Termination>,
    process_id: Option<u64>,
    awaiting_decision: bool,
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
    installing_since: Option<u64>,
    verifying_since: Option<u64>,
    cancel_requested: bool,
    sealed: bool,
    stale_reason: Option<String>,
    force: bool,
    download_dir: Option<String>,
}

pub struct Tools {
    data: Mutex<Data>,
}

/// A verified pair shipped inside the app bundle, with informational audit digests.
#[derive(Debug, Clone)]
struct BundledTool {
    sha256: String,
    output: String,
    content_sha256: String,
    file_count: usize,
}

#[derive(Clone)]
enum BundledResult {
    Absent,
    Damaged(String),
    Failed(String),
    Checked(bundled::Analysis, Result<BundledTool, String>),
}

struct BundledCheck(Arc<Mutex<Option<BundledResult>>>);

impl BundledCheck {
    fn start(check: impl FnOnce() -> BundledResult + Send + 'static) -> Self {
        let result = Arc::new(Mutex::new(None));
        let target = result.clone();
        std::thread::spawn(move || {
            // A panic must still fill the slot: leaving it empty would keep setup in
            // "verifying" forever instead of showing a failure.
            let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(check))
                .unwrap_or_else(|panic| {
                    let reason = panic
                        .downcast_ref::<&str>()
                        .map(|text| text.to_string())
                        .or_else(|| panic.downcast_ref::<String>().cloned())
                        .unwrap_or_else(|| "no message".to_string());
                    BundledResult::Failed(format!(
                        "The bundled analysis check stopped unexpectedly: {reason}"
                    ))
                });
            *target.lock().unwrap() = Some(checked);
        });
        Self(result)
    }

    fn result(&self) -> Option<BundledResult> {
        self.0.lock().unwrap().clone()
    }
}

fn start_bundled_check() -> BundledCheck {
    BundledCheck::start(|| match bundled::availability() {
        bundled::Availability::Absent => BundledResult::Absent,
        bundled::Availability::Damaged(reason) => BundledResult::Damaged(reason),
        bundled::Availability::Bundled(analysis) => {
            let result = verify_bundled(&analysis);
            BundledResult::Checked(analysis, result)
        }
    })
}

#[cfg(not(debug_assertions))]
fn bundled_check() -> Arc<BundledCheck> {
    static CHECK: OnceLock<Arc<BundledCheck>> = OnceLock::new();
    CHECK
        .get_or_init(|| Arc::new(start_bundled_check()))
        .clone()
}

#[cfg(debug_assertions)]
fn bundled_check() -> Arc<BundledCheck> {
    use std::collections::HashMap;
    type DebugKey = (
        Option<std::ffi::OsString>,
        Option<std::ffi::OsString>,
        Option<std::ffi::OsString>,
    );
    static CHECKS: OnceLock<Mutex<HashMap<DebugKey, Arc<BundledCheck>>>> = OnceLock::new();
    let key = (
        std::env::var_os(bundled::CSHARP_LS_OVERRIDE),
        std::env::var_os(bundled::RUNTIME_OVERRIDE),
        std::env::var_os(bundled::REQUIRED_OVERRIDE),
    );
    CHECKS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap()
        .entry(key)
        .or_insert_with(|| Arc::new(start_bundled_check()))
        .clone()
}

/// Enumerates the files used for the one-time informational digest.
fn fingerprint(tool_dir: &Path, runtime_root: &Path) -> Vec<(String, u64, i64)> {
    let mut rows = Vec::new();
    collect_fingerprint(tool_dir, bundled::CSHARP_LS_DIR, &mut rows);
    collect_fingerprint(runtime_root, bundled::RUNTIME_DIR, &mut rows);
    rows.sort();
    rows
}

fn collect_fingerprint(root: &Path, prefix: &str, rows: &mut Vec<(String, u64, i64)>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let relative = format!("{prefix}/{name}");
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => collect_fingerprint(&path, &relative, rows),
            Ok(kind) if kind.is_file() => {
                let Ok(meta) = entry.metadata() else {
                    continue;
                };
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_nanos() as i64)
                    .unwrap_or_default();
                rows.push((relative, meta.len(), modified));
            }
            _ => {}
        }
    }
}

/// One-time content digest of the pair (`path\0size\0bytes` in sorted order).
fn content_digest(analysis: &bundled::Analysis) -> Option<String> {
    use std::io::Read;
    let rows = fingerprint(&analysis.tool_dir, &analysis.runtime.root);
    let mut hasher = Sha256::new();
    for (relative, size, _) in rows {
        hasher.update(relative.as_bytes());
        hasher.update(size.to_le_bytes());
        let (root, stripped) =
            if let Some(rest) = relative.strip_prefix(&format!("{}/", bundled::CSHARP_LS_DIR)) {
                (analysis.tool_dir.as_path(), rest)
            } else {
                let rest = relative.strip_prefix(&format!("{}/", bundled::RUNTIME_DIR))?;
                (analysis.runtime.root.as_path(), rest)
            };
        let mut file = fs::File::open(root.join(stripped)).ok()?;
        let mut buffer = vec![0u8; 64 * 1024];
        loop {
            let read = file.read(&mut buffer).ok()?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
    }
    Some(hex(&hasher.finalize()))
}

/// Runs the bundled pair once: `<bundle>/dotnet <entry> --version`. No network, no installation
/// and no write outside the isolated child home; the project SDK is not involved.
fn verify_bundled(analysis: &bundled::Analysis) -> Result<BundledTool, String> {
    let home = child_home();
    fs::create_dir_all(&home)
        .map_err(|error| format!("Could not create the isolated tool HOME: {error}"))?;
    let cwd = neutral_cwd_ready();
    let muxer = bundled::muxer(&analysis.runtime.root);
    let mut command = Command::new(&muxer);
    command
        .arg(&analysis.entry)
        .arg("--version")
        .current_dir(&cwd)
        .env("HOME", &home);
    isolate_child_home(&mut command, &home, &std::env::temp_dir());
    let child = command
        .env("DOTNET_ROOT", &analysis.runtime.root)
        .env("DOTNET_MULTILEVEL_LOOKUP", "0")
        .env("DOTNET_CLI_UI_LANGUAGE", "en")
        .env("DOTNET_NOLOGO", "1")
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .env_remove("NUGET_PACKAGES")
        .env_remove("NUGET_HTTP_CACHE_PATH")
        .env_remove("DOTNET_CLI_HOME")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            format!(
                "Could not run the bundled csharp-ls {}: {error}",
                analysis.entry.to_string_lossy()
            )
        })?;
    let output = output_with_timeout(child, VERIFY_TIMEOUT)?;
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let text = text.trim().to_string();
    if !output.status.success() || !text.contains(VERSION) {
        return Err(format!(
            "The bundled csharp-ls did not report {VERSION} (exit {}): {text}",
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ));
    }
    let sha256 = sha256_file(&analysis.entry)
        .ok_or_else(|| "Could not hash the bundled csharp-ls entry point.".to_string())?;
    let content_sha256 = content_digest(analysis)
        .ok_or_else(|| "Could not hash the bundled analysis files.".to_string())?;
    Ok(BundledTool {
        sha256,
        output: text,
        file_count: fingerprint(&analysis.tool_dir, &analysis.runtime.root).len(),
        content_sha256,
    })
}

/// Waits for the probe, killing it at the deadline so a hung launcher cannot leave setup in
/// `verifying` forever.
fn output_with_timeout(mut child: Child, timeout: Duration) -> Result<Output, String> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(error) => return Err(format!("Could not wait for the bundled csharp-ls: {error}")),
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "The bundled csharp-ls did not report {VERSION} within {} s and was stopped.",
                timeout.as_secs()
            ));
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    child
        .wait_with_output()
        .map_err(|error| format!("Could not read the bundled csharp-ls output: {error}"))
}

impl Tools {
    pub fn new() -> Self {
        Self {
            data: Mutex::new(Data {
                revision: 0,
                state: "absent",
                destination: destination().to_string_lossy().to_string(),
                staging: staging().to_string_lossy().to_string(),
                runtime_root: String::new(),
                bundled: None,
                bundle_expected: false,
                bundled_attempted: false,
                plan: None,
                confirmed_plan_id: None,
                log: LogBuffer::default(),
                progress: None,
                progress_measured: false,
                progress_base: 0,
                progress_peak: 0,
                exit: None,
                termination: None,
                process_id: None,
                awaiting_decision: false,
                sha256: None,
                verified_version: None,
                verified_output: None,
                error: None,
                note: Some(
                    "No installation has been verified in this session. Review the plan to check or install csharp-ls 0.28.0."
                        .to_string(),
                ),
                reused: false,
                workspace_id: None,
                context_id: None,
                started_at: None,
                finished_at: None,
                installing_since: None,
                verifying_since: None,
                cancel_requested: false,
                sealed: false,
                stale_reason: None,
                force: false,
                download_dir: None,
            }),
        }
    }

    pub fn view(&self) -> ToolView {
        let data = self.data.lock().unwrap();
        let (log, log_truncated) = data.log.text();
        let destination = if data.destination.is_empty() {
            destination()
        } else {
            PathBuf::from(&data.destination)
        };
        let staging = if data.staging.is_empty() {
            staging()
        } else {
            PathBuf::from(&data.staging)
        };
        let (state, error) = (data.state, data.error.clone());
        ToolView {
            state,
            version: VERSION,
            destination: destination.to_string_lossy().to_string(),
            staging: staging.to_string_lossy().to_string(),
            bundled: data.bundle_expected || data.bundled.is_some(),
            plan: data.plan.clone(),
            log,
            log_truncated,
            progress: data.progress.clone(),
            exit: data.exit,
            termination: data.termination.clone(),
            process_id: data.process_id,
            awaiting_decision: data.awaiting_decision,
            sha256: data.sha256.clone(),
            verified_version: data.verified_version.clone(),
            verified_output: data.verified_output.clone(),
            error,
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
            staging_present: staging.is_dir(),
            destination_present: destination.is_dir(),
        }
    }

    /// Checks the cached bundled result or re-checks the downloaded launcher hash.
    pub fn still_ready(&self) -> bool {
        let data = self.data.lock().unwrap();
        if data.state != "ready" {
            return false;
        }
        if data.bundled.is_some() {
            return true;
        }
        let binary = Path::new(&data.destination).join(binary_name());
        match (&data.sha256, sha256_file(&binary)) {
            (Some(expected), Some(actual)) => expected == &actual,
            _ => false,
        }
    }

    /// Verifies the pair inside the app bundle, once per session. Without a bundle (`tauri dev`,
    /// host tests, non-macOS builds) this is a no-op and the download plan is used as before.
    /// The bundled pair is never installed, downloaded, replaced or reinstalled.
    pub fn sync_bundled(&self) {
        if self.data.lock().unwrap().bundled_attempted {
            return;
        }
        let result = bundled_check().result();
        let (analysis, result) = match result {
            None => {
                // `setup` reads "verifying" as an install already in progress; a build that
                // ships no pair would then skip `begin_install` until the check finishes.
                if bundled::pair_expected() {
                    let mut data = self.data.lock().unwrap();
                    if data.state == "absent" {
                        data.state = "verifying";
                        data.bundle_expected = true;
                    }
                }
                return;
            }
            Some(BundledResult::Absent) => {
                let mut data = self.data.lock().unwrap();
                data.bundled_attempted = true;
                if data.state == "verifying" {
                    data.state = "absent";
                }
                return;
            }
            Some(BundledResult::Failed(reason)) => {
                let mut data = self.data.lock().unwrap();
                data.bundled_attempted = true;
                data.state = "failed";
                data.progress = None;
                data.error = Some(reason.clone());
                data.note = Some(
                    "The analysis setup could not be checked in this session; no download or runtime was used."
                        .to_string(),
                );
                data.log.push(reason);
                return;
            }
            Some(BundledResult::Checked(analysis, result)) => (analysis, result),
            Some(BundledResult::Damaged(reason)) => {
                let mut data = self.data.lock().unwrap();
                data.bundled_attempted = true;
                data.bundle_expected = true;
                data.state = "failed";
                data.bundled = None;
                data.plan = None;
                data.progress = None;
                data.destination = bundled::resources_root()
                    .map(|root| root.join(bundled::CSHARP_LS_DIR))
                    .unwrap_or_else(destination)
                    .to_string_lossy()
                    .to_string();
                data.error = Some(reason.clone());
                data.note = Some(
                    "This app ships the analyzer; the bundle is incomplete, so no external download or runtime is used."
                        .to_string(),
                );
                data.log.push(reason);
                return;
            }
        };
        let mut data = self.data.lock().unwrap();
        if data.bundled_attempted {
            return;
        }
        data.bundled_attempted = true;
        data.bundle_expected = true;
        data.destination = analysis.tool_dir.to_string_lossy().to_string();
        data.staging = staging().to_string_lossy().to_string();
        data.plan = None;
        data.progress = None;
        data.runtime_root = analysis.runtime.root.to_string_lossy().to_string();
        match result {
            Ok(verified) => {
                data.state = "ready";
                data.bundled = Some(verified.clone());
                data.sha256 = Some(verified.sha256.clone());
                data.verified_version = Some(VERSION.to_string());
                data.verified_output = Some(verified.output.clone());
                data.error = None;
                data.note = Some(format!(
                    "Bundled with OpenRidr: csharp-ls {VERSION} and the .NET runtime {} ship inside the app. Nothing is downloaded or installed.",
                    analysis.runtime.version
                ));
                data.log.push(format!(
                    "Bundled analysis tool checked: {} --version",
                    analysis.entry.to_string_lossy()
                ));
                data.log.push(verified.output);
                data.log.push(format!(
                    "Bundled pair content SHA-256 {} ({} files); informational, the integrity reference is the app's code signature",
                    verified.content_sha256,
                    verified.file_count
                ));
            }
            Err(message) => {
                data.state = "failed";
                data.bundled = None;
                data.error = Some(message.clone());
                data.note = Some(
                    "The bundled analysis tool did not verify; the app bundle is incomplete or damaged. Reinstall OpenRidr instead of downloading a different tool."
                        .to_string(),
                );
                data.log.push(message);
            }
        }
    }

    /// Records a freshly built plan. Does not start anything.
    pub fn set_plan(&self, plan: Plan) {
        let mut data = self.data.lock().unwrap();
        if data.state == "downloading" {
            return;
        }
        // A new plan starts from a clean message state; the previous attempt's error stays in its log.
        data.error = None;
        let same_destination = data.destination == plan.destination;
        data.destination = plan.destination.clone();
        data.staging = plan.staging.clone();
        data.runtime_root = plan.dotnet_root.clone();
        let still_installed = same_destination
            && data.state == "ready"
            && data.sha256.as_deref()
                == sha256_file(&Path::new(&plan.destination).join(binary_name())).as_deref();
        match plan.existing.state {
            "present" if still_installed => {
                data.state = "ready";
                data.error = None;
                data.note = Some(
                    "The verified installation remains ready; confirming the plan reuses it without downloading or overwriting."
                        .to_string(),
                );
            }
            "present" => {
                data.state = "planned";
                data.note = Some(format!(
                    "A destination exists but the plan did not execute it. Confirming verifies it and reuses a valid {VERSION} installation; otherwise the tool is installed into staging and replaces it only after verification."
                ));
            }
            _ => {
                data.state = "planned";
                data.note = Some(
                    "No verified installation exists. The plan does not download or run anything until you confirm it."
                        .to_string(),
                );
            }
        }
        if Path::new(&plan.staging).is_dir() && data.state != "ready" {
            data.note = Some(format!(
                "A partial staging directory exists at {}; it is not ready and will be replaced by a confirmed install.",
                plan.staging
            ));
        }
        data.plan = Some(plan);
        data.confirmed_plan_id = None;
    }

    /// Only a recorded, still-matching launcher that really answers the pinned version is
    /// reusable. A stale record is removed so the caller downloads the verified package.
    fn reusable_installation(destination: &Path, runtime_root: &Path) -> bool {
        if !persisted_installation_valid(destination) {
            return false;
        }
        let existing = verify_installed(destination, runtime_root);
        if existing.state == "ready" {
            return true;
        }
        let _ = fs::remove_file(destination.join(".openridr-integrity"));
        false
    }

    /// Starts the real install. The caller has already validated trust, confirmation and that
    /// the plan id still matches the material fields.
    pub fn begin_install(
        self: &Arc<Self>,
        plan: Plan,
        workspace_id: u64,
        context_id: u64,
        processes: Arc<Mutex<Registry>>,
        force: bool,
    ) -> HostResult<Value> {
        let destination = Path::new(&plan.destination);
        let binary = destination.join(binary_name());
        // Reuse only an installation whose recorded integrity is still valid AND whose
        // launcher really answers the pinned version. A stale record is discarded; an old
        // launcher without a record is never adopted, it is reinstalled from the verified
        // package instead. `force` always goes through the staging download.
        if !force && Self::reusable_installation(destination, Path::new(&plan.dotnet_root)) {
            return self.begin_install_worker(
                plan,
                workspace_id,
                context_id,
                &mut processes.lock().unwrap(),
                None,
                false,
            );
        }
        if !force && probe_muxer().is_some() && binary.is_file() {
            return self.begin_install_worker(
                plan,
                workspace_id,
                context_id,
                &mut processes.lock().unwrap(),
                None,
                false,
            );
        }
        let attempt = {
            let mut data = self.data.lock().unwrap();
            if matches!(data.state, "downloading" | "installing" | "verifying") && !data.sealed {
                return Err(HostError::new(
                    "invalid_state",
                    "An installation is already running.",
                ));
            }
            data.revision += 1;
            data.state = "downloading";
            data.plan = Some(plan.clone());
            data.confirmed_plan_id = Some(plan.plan_id.clone());
            data.destination = plan.destination.clone();
            data.staging = plan.staging.clone();
            data.runtime_root = plan.dotnet_root.clone();
            data.workspace_id = Some(workspace_id);
            data.context_id = Some(context_id);
            data.started_at = Some(now_ms());
            data.finished_at = None;
            data.process_id = None;
            data.progress = None;
            data.progress_measured = false;
            data.error = None;
            data.sealed = false;
            data.cancel_requested = false;
            data.force = force;
            data.download_dir = None;
            data.note = Some("Downloading and verifying the pinned csharp-ls package.".into());
            data.revision
        };
        let tools = Arc::clone(self);
        std::thread::spawn(move || {
            let temp = install_temp(&plan.staging).join(format!("download-{attempt}"));
            cleanup_stale_downloads(&temp);
            {
                let mut data = tools.data.lock().unwrap();
                if data.revision == attempt && !data.sealed {
                    data.download_dir = Some(temp.to_string_lossy().to_string());
                }
            }
            let package = if probe_muxer().is_some() && !package_redirected() {
                #[cfg(debug_assertions)]
                if let Ok(delay) = std::env::var("OPENRIDR_TOOLS_PROBE_DOWNLOAD_DELAY_MS") {
                    if let Ok(milliseconds) = delay.parse::<u64>() {
                        std::thread::sleep(std::time::Duration::from_millis(
                            milliseconds.min(5_000),
                        ));
                    }
                }
                Ok(None)
            } else {
                fs::create_dir_all(&temp)
                    .map_err(|error| error.to_string())
                    .and_then(|_| {
                        prepare_local_package(&temp, |downloaded, total| {
                            let mut data = tools.data.lock().unwrap();
                            if data.cancel_requested || data.sealed || data.revision != attempt {
                                return false;
                            }
                            data.progress = Some(Progress { downloaded, total });
                            // Measured, with or without a total: bytes read from the socket are not a
                            // disk estimate, so the observation below must never replace them.
                            data.progress_measured = true;
                            true
                        })
                        .map(Some)
                    })
            };
            let mut registry = processes.lock().unwrap();
            {
                let data = tools.data.lock().unwrap();
                if data.sealed || data.cancel_requested || data.revision != attempt {
                    let _ = fs::remove_dir_all(&temp);
                    return;
                }
            }
            let result = package
                .map_err(|error| HostError::new("operation_failed", error))
                .and_then(|package| {
                    tools.begin_install_worker(
                        plan,
                        workspace_id,
                        context_id,
                        &mut registry,
                        package,
                        force,
                    )
                });
            if let Err(error) = result {
                let mut data = tools.data.lock().unwrap();
                if !data.sealed && data.revision == attempt {
                    data.state = "failed";
                    data.error = Some(error.message);
                    data.finished_at = Some(now_ms());
                    cleanup_download(&mut data);
                    if force {
                        restore_previous(&mut data);
                    }
                }
            }
        });
        Ok(json!({"spawned": true}))
    }

    fn begin_install_worker(
        self: &Arc<Self>,
        plan: Plan,
        workspace_id: u64,
        context_id: u64,
        processes: &mut Registry,
        local_package: Option<(PathBuf, PathBuf)>,
        force: bool,
    ) -> HostResult<Value> {
        if let Some(process_id) = self.running_process_id() {
            return Err(HostError::new(
                "invalid_state",
                format!("An installation is already running (process {process_id})."),
            ));
        }
        // Confirmation is the executable step: only now is the existing launcher run (read-only
        // `--version`) to decide reuse versus replacement. A valid tool is never overwritten.
        let can_reuse = !force
            && (persisted_installation_valid(Path::new(&plan.destination))
                || probe_muxer().is_some());
        let existing = if can_reuse {
            verify_installed(Path::new(&plan.destination), Path::new(&plan.dotnet_root))
        } else {
            inspect_existing(Path::new(&plan.destination))
        };
        if can_reuse && existing.state == "ready" {
            let destination = plan.destination.clone();
            let staging = plan.staging.clone();
            let mut plan = plan;
            plan.record_verified(&existing);
            let mut data = self.data.lock().unwrap();
            data.plan = Some(plan);
            data.confirmed_plan_id = None;
            data.destination = destination;
            data.staging = staging;
            data.state = "ready";
            data.reused = true;
            data.error = None;
            data.progress = None;
            data.progress_measured = false;
            data.progress_base = 0;
            data.progress_peak = 0;
            data.sha256 = existing.sha256.clone();
            data.verified_version = existing.verified_version.clone();
            data.verified_output = existing.output.clone();
            data.note = Some(
                "Reused the verified installation. Nothing was downloaded, replaced or started."
                    .to_string(),
            );
            data.workspace_id = Some(workspace_id);
            data.context_id = Some(context_id);
            data.finished_at = Some(now_ms());
            data.force = false;
            data.download_dir = None;
            data.log.push(
                "Existing installation verified; reused without downloading or overwriting."
                    .to_string(),
            );
            if let Some(output) = &existing.output {
                data.log.push(format!("$ {} --version", existing.binary));
                data.log.push(output.clone());
            }
            return Ok(json!({ "reused": true }));
        }

        let staging = PathBuf::from(&plan.staging);
        if staging.is_dir() {
            let _ = fs::remove_dir_all(&staging);
        }
        if let Some(parent) = staging.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                HostError::new(
                    "operation_failed",
                    format!("Could not create the tools directory: {error}"),
                )
            })?;
        }
        let home = child_home();
        fs::create_dir_all(&home).map_err(|error| {
            HostError::new(
                "operation_failed",
                format!("Could not create the isolated tool HOME: {error}"),
            )
        })?;
        let cwd = neutral_cwd_ready();
        // Keep NuGet's scratch files under the app home so fetched bytes are observable and never
        // land in the user's shared temp directory.
        let temp = install_temp(&plan.staging);
        fs::create_dir_all(&temp).map_err(|error| {
            HostError::new(
                "operation_failed",
                format!("Could not create the tool install temp directory: {error}"),
            )
        })?;
        let progress_base = directory_bytes(&child_home()) + directory_bytes(&temp);

        if local_package.is_none() && probe_muxer().is_none() {
            return Err(HostError::new(
                "operation_failed",
                "Refusing to run dotnet tool install without the verified local package feed and its SHA-512; the previous installation was not touched.",
            ));
        }
        if let Some((feed, _config)) = &local_package {
            let package_path = feed.join("csharp-ls.0.28.0.nupkg");
            let bytes = fs::read(&package_path).map_err(|error| {
                HostError::new(
                    "operation_failed",
                    format!(
                        "Could not re-read the staged csharp-ls package {}: {error}",
                        package_path.to_string_lossy()
                    ),
                )
            })?;
            // The same pin the download verified: a redirected debug package must survive the
            // recheck, otherwise only the download half of the attempt is exercised.
            verify_package_hash(&bytes, &package_source().1).map_err(|error| {
                HostError::new(
                    "operation_failed",
                    format!("{error} No dotnet tool install was started."),
                )
            })?;
        }
        let mut command = Command::new(&plan.argv[0]);
        command
            .args(&plan.argv[1..])
            .current_dir(&cwd)
            // Isolated HOME pins the child to NuGet's default official source and keeps all
            // writes under the app home; user feed configuration is neither read nor logged.
            .env("HOME", &home)
            .env("TMPDIR", &temp);
        isolate_child_home(&mut command, &home, &temp);
        command
            .env("DOTNET_NOLOGO", "1")
            .env("DOTNET_CLI_UI_LANGUAGE", "en")
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
            .env_remove("DOTNET_ROOT")
            .env_remove("DOTNET_MULTILEVEL_LOOKUP")
            .env_remove("DOTNET_CLI_HOME")
            .env_remove("NUGET_PACKAGES")
            .env_remove("NUGET_HTTP_CACHE_PATH");
        if let Some((feed, config)) = local_package {
            command
                .arg("--add-source")
                .arg(feed)
                .arg("--configfile")
                .arg(config)
                .arg("--no-cache")
                .env("NUGET_PACKAGES", temp.join("packages"));
        }

        let piped = match processes.spawn_piped("tools:csharp-ls", workspace_id, command) {
            Ok(piped) => piped,
            Err(error) => {
                let mut data = self.data.lock().unwrap();
                data.revision += 1;
                data.state = "failed";
                data.confirmed_plan_id = Some(plan.plan_id.clone());
                data.plan = Some(plan);
                data.progress = None;
                data.error = Some(format!("Could not start dotnet tool install: {error}"));
                data.note = Some(format!(
                    "Nothing was installed. The staging directory {} is not ready.",
                    staging.to_string_lossy()
                ));
                data.started_at = Some(now_ms());
                data.finished_at = Some(now_ms());
                data.workspace_id = Some(workspace_id);
                data.context_id = Some(context_id);
                let argv = data
                    .plan
                    .as_ref()
                    .map(|plan| plan.argv.join(" "))
                    .unwrap_or_default();
                data.log.push(format!("Could not start {argv}: {error}"));
                return Ok(json!({ "spawned": false }));
            }
        };

        let mut data = self.data.lock().unwrap();
        data.revision += 1;
        data.state = "downloading";
        data.plan = Some(plan.clone());
        data.confirmed_plan_id = Some(plan.plan_id.clone());
        data.destination = plan.destination.clone();
        data.staging = plan.staging.clone();
        data.runtime_root = plan.dotnet_root.clone();
        data.force = force;
        data.reused = false;
        data.exit = None;
        data.termination = None;
        data.awaiting_decision = false;
        data.sha256 = None;
        data.verified_version = None;
        data.verified_output = None;
        data.error = None;
        if !data.progress_measured {
            // No measured download (the debug probe installer): observe bytes only from here.
            data.progress = None;
            data.progress_base = progress_base;
            data.progress_peak = 0;
        }
        data.note = Some(if data.progress_measured {
            // The package is already downloaded and verified; `dotnet tool install` writes
            // several copies of it, so its disk writes are not a download to report.
            "The pinned csharp-ls package is verified; installing it into the app home with \
             dotnet tool install."
                .to_string()
        } else {
            "dotnet tool install reports no percentage; progress is measured from bytes it writes to its isolated NuGet cache and staging."
                .to_string()
        });
        data.workspace_id = Some(workspace_id);
        data.context_id = Some(context_id);
        data.started_at = Some(now_ms());
        data.finished_at = None;
        data.installing_since = None;
        data.verifying_since = None;
        data.cancel_requested = false;
        data.sealed = false;
        data.stale_reason = None;
        data.process_id = Some(piped.id);
        let attempt = data.revision;
        data.log.push(format!("== attempt {attempt} =="));
        data.log.push(format!("$ {}", plan.argv.join(" ")));
        data.log.push(format!("cwd {}", plan.cwd));
        data.log.push(format!(
            "source {} · destination {} · staging {}",
            plan.source.url, plan.destination, plan.staging
        ));
        drop(data);

        let tools = Arc::clone(self);
        if let Some(stdout) = piped.stdout {
            let sink = Arc::clone(&tools);
            std::thread::spawn(move || {
                logbuf::pump_lines(stdout, false, |line, stderr| sink.append_line(line, stderr))
            });
        }
        if let Some(stderr) = piped.stderr {
            let sink = Arc::clone(&tools);
            std::thread::spawn(move || {
                logbuf::pump_lines(stderr, true, |line, stderr| sink.append_line(line, stderr))
            });
        }
        Ok(json!({ "spawned": true, "processId": piped.id }))
    }

    pub fn running_process_id(&self) -> Option<u64> {
        let data = self.data.lock().unwrap();
        if data.state == "downloading" && !data.sealed {
            data.process_id
        } else {
            None
        }
    }

    pub fn append_line(&self, line: &str, stderr: bool) {
        let mut data = self.data.lock().unwrap();
        if data.sealed {
            return;
        }
        data.log.push(if stderr {
            format!("[stderr] {line}")
        } else {
            line.to_string()
        });
    }

    /// Observes the install process and performs promotion/verification exactly once.
    /// The stages are separate snapshots so `installing` and `verifying` are observable instead
    /// of being collapsed into a single state transition.
    pub fn sync(&self, processes: &mut Registry) {
        let (state, process_id) = {
            let data = self.data.lock().unwrap();
            (data.state, data.process_id)
        };
        if !matches!(state, "downloading" | "installing" | "verifying") {
            cleanup_download(&mut self.data.lock().unwrap());
            return;
        }
        let Some(process_id) = process_id else {
            return;
        };
        let Some(status) = processes.status(process_id) else {
            return;
        };

        let mut data = self.data.lock().unwrap();
        if data.state != state || data.sealed {
            return;
        }
        data.exit = status.exit;
        data.termination = status.termination.clone();
        data.awaiting_decision = status.awaiting_decision;
        if observes_disk_bytes(state, data.progress_measured) {
            // Only the probe path reaches here without a measured download; the observed bytes
            // are de-duplicated by root so a nested directory is never counted twice.
            let roots = [
                PathBuf::from(&data.staging),
                child_home(),
                install_temp(&data.staging),
            ];
            let observed = observed_bytes(&roots);
            let downloaded = observed
                .saturating_sub(data.progress_base)
                .max(data.progress_peak);
            if downloaded > 0 {
                data.progress_peak = downloaded;
                data.progress = Some(Progress {
                    downloaded,
                    total: None,
                });
            }
        }
        if status.state != "exited" {
            return;
        }
        data.awaiting_decision = false;

        match state {
            "downloading" => {
                if data.cancel_requested || data.termination.is_some() {
                    data.state = "cancelled";
                    data.progress = None;
                    data.note = Some(format!(
                        "Installation was cancelled; {} is partial and is not ready.",
                        data.staging
                    ));
                    data.finished_at = Some(now_ms());
                    data.log.push(
                        "Installation cancelled; no partial result is reported as ready."
                            .to_string(),
                    );
                    return;
                }
                if status.exit != Some(0) {
                    let source = data
                        .plan
                        .as_ref()
                        .map(|plan| plan.source.url)
                        .unwrap_or(NUGET_SOURCE);
                    data.state = "failed";
                    data.progress = None;
                    data.error = Some(format!(
                        "dotnet tool install exited with {}; nothing was promoted or reported ready. Check the network connection to {source}, then use Try again; retrying starts from a clean staging directory. The log above shows what dotnet reported.",
                        status
                            .exit
                            .map(|code| code.to_string())
                            .unwrap_or_else(|| "unknown (no exit code)".to_string())
                    ));
                    data.note = Some(format!(
                        "The staging directory {} is not ready.",
                        data.staging
                    ));
                    data.finished_at = Some(now_ms());
                    cleanup_download(&mut data);
                    if data.force {
                        restore_previous(&mut data);
                    }
                    return;
                }
                data.state = "installing";
                data.installing_since = Some(now_ms());
                data.log
                    .push("Install process exited 0; promoting the staging directory.".to_string());
            }
            "installing" => {
                // Like verifying, keep promotion observable for at least one poll interval.
                if let Some(since) = data.installing_since {
                    if now_ms().saturating_sub(since) < 300 {
                        return;
                    }
                }
                let destination = PathBuf::from(&data.destination);
                let staging = PathBuf::from(&data.staging);
                // Verify the new copy and write its integrity record BEFORE the swap. A bad
                // package or an unwritable record never touches the working installation.
                if let Err(error) =
                    verify_staged_installation(&staging, Path::new(&data.runtime_root))
                {
                    data.state = "failed";
                    data.progress = None;
                    data.error = Some(format!("Verification failed before promotion: {error}"));
                    data.note = Some(
                        "The previous installation was kept; the new staging directory is not ready."
                            .to_string(),
                    );
                    let _ = fs::remove_dir_all(&staging);
                    data.finished_at = Some(now_ms());
                    cleanup_download(&mut data);
                    restore_previous(&mut data);
                    return;
                }
                if let Err(error) = promote(&staging, &destination) {
                    data.state = "failed";
                    data.progress = None;
                    data.error = Some(format!(
                        "Could not promote the installed tool to {}: {error}",
                        destination.to_string_lossy()
                    ));
                    data.note = Some(format!(
                        "The staging directory {} was kept and is not ready.",
                        staging.to_string_lossy()
                    ));
                    data.finished_at = Some(now_ms());
                    return;
                }
                data.state = "verifying";
                data.verifying_since = Some(now_ms());
                data.log
                    .push("Verifying the launcher and its --version output.".to_string());
            }
            "verifying" => {
                // Keep the phase observable for at least one short interval instead of
                // collapsing installing→verifying→ready inside consecutive snapshot calls.
                if let Some(since) = data.verifying_since {
                    if now_ms().saturating_sub(since) < 250 {
                        return;
                    }
                }
                let destination = PathBuf::from(&data.destination);
                data.log.push(format!(
                    "Verifying the launcher and its --version output at {}.",
                    destination.to_string_lossy()
                ));
                let existing = verify_installed(&destination, Path::new(&data.runtime_root));
                if existing.state != "ready" {
                    let staging = PathBuf::from(&data.staging);
                    let recovered = rollback_promotion(&staging, &destination);
                    data.state = "failed";
                    data.progress = None;
                    data.error = Some(format!(
                        "Verification after promotion failed: {}",
                        existing
                            .reason
                            .clone()
                            .unwrap_or_else(|| "no verified version".to_string())
                    ));
                    if let Some(output) = &existing.output {
                        data.log.push(output.clone());
                    }
                    data.note = Some(if recovered {
                        "The new installation failed verification; the previous verified installation was restored."
                            .to_string()
                    } else {
                        "The unverified destination was left in place and is not ready.".to_string()
                    });
                    data.finished_at = Some(now_ms());
                    cleanup_download(&mut data);
                    if recovered {
                        restore_previous(&mut data);
                    }
                    return;
                }
                let staging = PathBuf::from(&data.staging);
                if staging.is_dir() {
                    let _ = fs::remove_dir_all(&staging);
                }
                data.state = "ready";
                if let Some(plan) = data.plan.as_mut() {
                    plan.record_verified(&existing);
                }
                data.sha256 = existing.sha256.clone();
                data.verified_version = existing.verified_version.clone();
                data.verified_output = existing.output.clone();
                data.error = None;
                data.progress = None;
                data.note = Some(
                    "Executable and version verified. This proves the tool installation, not that the language server loads a solution."
                        .to_string(),
                );
                data.finished_at = Some(now_ms());
                data.force = false;
                cleanup_download(&mut data);
                if let Some(output) = &existing.output {
                    data.log.push(format!("$ {} --version", existing.binary));
                    data.log.push(output.clone());
                }
                if let Some(sha) = &existing.sha256 {
                    data.log.push(format!("sha256 {sha}"));
                }
                data.log.push(SIGNATURE_LIMITATION.to_string());
            }
            _ => {}
        }
    }

    pub fn cancel(&self, processes: &mut Registry) -> HostResult<()> {
        let mut data = self.data.lock().unwrap();
        if data.state == "planned" {
            data.state = "cancelled";
            data.progress = None;
            data.note = Some(
                "Installation plan cancelled; no download and no process were started.".to_string(),
            );
            data.finished_at = Some(now_ms());
            data.log
                .push("Plan cancelled; nothing was downloaded or executed.".to_string());
            return Ok(());
        }
        let Some(process_id) = data.process_id else {
            if data.state == "downloading" {
                data.cancel_requested = true;
                data.state = "cancelled";
                data.progress = None;
                data.note = Some(format!(
                    "Installation was cancelled before the download finished; {} is not ready.",
                    data.staging
                ));
                data.finished_at = Some(now_ms());
                cleanup_download(&mut data);
                data.log.push(
                    "Installation cancelled before the download finished; no partial result is reported as ready."
                        .to_string(),
                );
                return Ok(());
            }
            return Err(HostError::new(
                "invalid_state",
                "No installation plan or running installation to cancel.",
            ));
        };
        if data.state != "downloading" || data.sealed {
            return Err(HostError::new(
                "invalid_state",
                "No installation plan or running installation to cancel.",
            ));
        }
        data.cancel_requested = true;
        let status = processes.terminate_status(process_id);
        if let Some(status) = &status {
            data.exit = status.exit;
            data.termination = status.termination.clone();
            data.awaiting_decision = status.state == "running";
        }
        data.state = "cancelled";
        data.progress = None;
        data.finished_at = Some(now_ms());
        cleanup_download(&mut data);
        let (note, log) = if data.awaiting_decision {
            (
                format!(
                    "Cancellation requested; termination is not confirmed and {} remains partial. The process stays visible in the panel.",
                    data.staging
                ),
                "Cancellation requested; termination was not confirmed and no partial result is ready."
                    .to_string(),
            )
        } else {
            (
                format!(
                    "Installation cancelled; {} is partial and is not ready.",
                    data.staging
                ),
                "Installation cancelled; no partial result is reported as ready.".to_string(),
            )
        };
        data.note = Some(note);
        data.log.push(log);
        Ok(())
    }

    /// A workspace boundary: stop a real install and keep what was observed.
    pub fn seal(&self, reason: &str, status: Option<&ProcessStatus>, processes: &mut Registry) {
        let (state, process_id) = {
            let data = self.data.lock().unwrap();
            if !matches!(data.state, "downloading" | "installing" | "verifying") || data.sealed {
                return;
            }
            (data.state, data.process_id)
        };
        let status = match process_id {
            Some(id) => status.cloned().or_else(|| processes.terminate_status(id)),
            None => None,
        };
        let mut data = self.data.lock().unwrap();
        data.cancel_requested = true;
        data.sealed = true;
        data.stale_reason = Some(reason.to_string());
        data.state = "cancelled";
        data.progress = None;
        if let Some(status) = status {
            data.exit = status.exit;
            data.termination = status.termination.clone();
            data.awaiting_decision = status.state == "running";
        }
        data.finished_at = Some(now_ms());
        cleanup_download(&mut data);
        data.note = Some(if state == "verifying" {
            format!(
                "Stopped at a workspace boundary after promotion; {} is not reported ready until a new plan verifies and reuses it.",
                data.destination
            )
        } else {
            format!(
                "Stopped at a workspace boundary; {} is partial and is not ready.",
                data.staging
            )
        });
    }

    pub fn confirmed_plan_id(&self) -> Option<String> {
        self.data.lock().unwrap().confirmed_plan_id.clone()
    }
}

impl Default for Tools {
    fn default() -> Self {
        Self::new()
    }
}

/// Same promotion semantics as a file save: the destination is replaced only after the
/// staged tree exists; a failed rename restores the previous destination.
fn cleanup_stale_downloads(current: &Path) {
    let Some(root) = current.parent() else {
        return;
    };
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.path() != current && entry.file_name().to_string_lossy().starts_with("download-") {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
}

fn cleanup_download(data: &mut Data) {
    if let Some(directory) = data.download_dir.take() {
        let _ = fs::remove_dir_all(directory);
    }
}

/// A failed force-reinstall must leave the previous verified launcher usable. It is only a
/// recovery path: the new files never touched the destination before verification.
fn restore_previous(data: &mut Data) -> bool {
    let destination = PathBuf::from(&data.destination);
    let existing = verify_installed(&destination, Path::new(&data.runtime_root));
    if existing.state != "ready" {
        return false;
    }
    data.state = "ready";
    data.progress = None;
    data.sha256 = existing.sha256.clone();
    data.verified_version = existing.verified_version.clone();
    data.verified_output = existing.output.clone();
    data.note = Some(
        "Reinstall failed; the previous verified installation was kept and is still ready."
            .to_string(),
    );
    data.finished_at = Some(now_ms());
    true
}

fn verify_staged_installation(staging: &Path, runtime_root: &Path) -> Result<Existing, String> {
    let existing = verify_installed(staging, runtime_root);
    if existing.state != "ready" {
        return Err(existing
            .reason
            .unwrap_or_else(|| "the staged launcher did not verify".to_string()));
    }
    let Some(sha) = existing.sha256.as_deref() else {
        return Err("could not hash the staged launcher".to_string());
    };
    record_installation(staging, sha)
        .map_err(|error| format!("could not save the staged integrity record: {error}"))?;
    Ok(existing)
}

/// After a successful swap, `staging` holds the previous directory. A failed post-swap check
/// swaps it back; the new directory ends in staging and is removed.
fn rollback_promotion(staging: &Path, destination: &Path) -> bool {
    if !staging.is_dir() {
        return false;
    }
    if promote(staging, destination).is_err() {
        return false;
    }
    let _ = fs::remove_dir_all(staging);
    true
}

fn promote(staging: &Path, destination: &Path) -> Result<(), String> {
    if !staging.is_dir() {
        return Err("the staging directory does not exist".to_string());
    }
    if !destination.exists() {
        return fs::rename(staging, destination)
            .map_err(|error| format!("could not move staging into place: {error}"));
    }
    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let from = CString::new(staging.as_os_str().as_bytes())
            .map_err(|_| "the staging path is not valid".to_string())?;
        let to = CString::new(destination.as_os_str().as_bytes())
            .map_err(|_| "the destination path is not valid".to_string())?;
        // One atomic swap: at every instant either the old or the new directory is at the
        // destination. A failed swap leaves the previous installation untouched.
        let swapped = unsafe { libc::renamex_np(from.as_ptr(), to.as_ptr(), libc::RENAME_SWAP) };
        if swapped != 0 {
            return Err(format!(
                "could not atomically swap the installation: {}",
                std::io::Error::last_os_error()
            ));
        }
        // staging now holds the previous directory; the caller keeps it until post-swap
        // verification succeeds (or swaps it back on failure).
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let backup = PathBuf::from(format!(
            "{}.old-{}-{}",
            destination.to_string_lossy(),
            std::process::id(),
            now_ms()
        ));
        fs::rename(destination, &backup)
            .map_err(|error| format!("could not move the previous destination aside: {error}"))?;
        if let Err(error) = fs::rename(staging, destination) {
            let _ = fs::rename(&backup, destination);
            return Err(format!("could not move staging into place: {error}"));
        }
        let _ = fs::remove_dir_all(backup);
        Ok(())
    }
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
    fn a_body_read_timeout_is_the_download_stall_and_other_read_errors_are_not() {
        let timeout =
            std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out reading response");
        let message =
            download_read_error("https://api.nuget.org/v3-flatcontainer/x.nupkg", &timeout);
        assert!(
            message.starts_with("Could not download csharp-ls: "),
            "{message}"
        );
        assert!(message.contains("the connection stalled"), "{message}");
        assert!(
            message.contains("Nothing was installed or replaced"),
            "{message}"
        );
        let other = std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "short body");
        let message = download_read_error("https://api.nuget.org/v3-flatcontainer/x.nupkg", &other);
        assert!(
            message.starts_with("Could not read csharp-ls package: "),
            "{message}"
        );
        assert!(!message.contains("stalled"), "{message}");
    }
    use crate::sdk::RuntimeInstall;

    #[test]
    fn bundled_check_is_nonblocking_and_runs_only_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        let calls = Arc::new(AtomicUsize::new(0));
        let (release, wait) = mpsc::channel();
        let (started_worker, worker_started) = mpsc::channel();
        let counter = calls.clone();
        let check = BundledCheck::start(move || {
            counter.fetch_add(1, Ordering::SeqCst);
            started_worker.send(()).unwrap();
            wait.recv().unwrap();
            BundledResult::Absent
        });
        worker_started.recv().unwrap();
        let started = Instant::now();
        for _ in 0..100 {
            assert!(check.result().is_none());
        }
        assert!(started.elapsed() < Duration::from_secs(1));
        release.send(()).unwrap();
        for _ in 0..1000 {
            if check.result().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        assert!(matches!(check.result(), Some(BundledResult::Absent)));
        assert!(matches!(check.result(), Some(BundledResult::Absent)));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn bundled_check_records_a_panic_as_a_visible_failure() {
        use std::time::Duration;

        let check = BundledCheck::start(|| panic!("verification exploded"));
        for _ in 0..1000 {
            if check.result().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        match check.result() {
            Some(BundledResult::Failed(reason)) => {
                assert!(reason.contains("verification exploded"), "{reason}");
            }
            _ => panic!("a panicking check must still fill the result slot"),
        }
    }

    #[test]
    #[cfg(unix)]
    fn bundled_output_timeout_kills_the_probe() {
        use std::time::{Duration, Instant};

        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join("finished");
        let child = Command::new("/bin/sh")
            .arg("-c")
            .arg("sleep 1; touch \"$0\"")
            .arg(&marker)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let started = Instant::now();
        let error = output_with_timeout(child, Duration::from_millis(100)).unwrap_err();
        assert!(error.contains("within 0 s"), "{error}");
        assert!(started.elapsed() < Duration::from_millis(900));
        std::thread::sleep(Duration::from_millis(1200));
        assert!(
            !marker.exists(),
            "the timed-out child must not keep running"
        );
    }

    #[test]
    fn package_hash_must_match_exactly() {
        let expected = "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f";
        assert!(verify_package_hash(b"abc", expected).is_ok());
        assert!(verify_package_hash(b"modified", expected).is_err());
        let temp = tempfile::tempdir().unwrap();
        assert!(store_verified_package(temp.path(), b"modified", expected).is_err());
        assert!(!temp.path().join("verified-feed").exists());
    }

    #[test]
    fn observed_bytes_never_counts_a_nested_root_twice() {
        // The installer writes the same bytes to its verified feed, to NUGET_PACKAGES and to
        // staging; a root inside another root is the same file counted again.
        let temp = tempfile::tempdir().unwrap();
        let staging = temp.path().join("0.28.staging");
        let temp_dir = temp.path().join("tmp");
        fs::create_dir_all(staging.join("inner")).unwrap();
        fs::create_dir_all(&temp_dir).unwrap();
        fs::write(staging.join("launcher"), vec![0u8; 1000]).unwrap();
        fs::write(staging.join("inner/package.nupkg"), vec![0u8; 4000]).unwrap();
        fs::write(temp_dir.join("scratch.bin"), vec![0u8; 500]).unwrap();

        assert_eq!(directory_bytes(&staging), 5000);
        // The old sum of every directory reported 5000 + 5000 + 500 bytes for 5500 real bytes.
        assert_eq!(
            observed_bytes(&[staging.clone(), temp_dir.clone(), staging.join("inner")]),
            5500
        );
        assert_eq!(observed_bytes(&[]), 0);
    }

    #[test]
    fn a_measured_download_owns_the_progress_number() {
        assert!(observes_disk_bytes("downloading", false));
        assert!(!observes_disk_bytes("downloading", true));
        assert!(!observes_disk_bytes("installing", false));
        assert!(!observes_disk_bytes("ready", false));
    }

    #[test]
    fn persisted_installation_detects_launcher_changes() {
        let directory = tempfile::tempdir().unwrap();
        let launcher = directory.path().join(binary_name());
        fs::write(&launcher, b"verified launcher").unwrap();
        assert!(!persisted_installation_valid(directory.path()));
        record_installation(directory.path(), &sha256_file(&launcher).unwrap()).unwrap();
        assert!(persisted_installation_valid(directory.path()));
        fs::write(&launcher, b"modified launcher").unwrap();
        assert!(!persisted_installation_valid(directory.path()));
    }

    fn inventory() -> (tempfile::TempDir, Inventory) {
        let temp = tempfile::tempdir().unwrap();
        let sdk_root = temp.path().join("sdk-root");
        fs::create_dir_all(&sdk_root).unwrap();
        fs::write(sdk_root.join(crate::sdk::dotnet_exe()), "").unwrap();
        let runtime_root = temp.path().join("runtime-root");
        fs::create_dir_all(&runtime_root).unwrap();
        (
            temp,
            Inventory {
                roots: Vec::new(),
                installs: vec![Install {
                    version: "8.0.408".to_string(),
                    root: sdk_root.clone(),
                    path: sdk_root.join("sdk").to_string_lossy().to_string(),
                }],
                runtimes: vec![RuntimeInstall {
                    version: "10.0.1".to_string(),
                    root: runtime_root,
                }],
                inspected_at: 0,
            },
        )
    }

    #[test]
    fn plan_id_changes_with_every_material_field() {
        let (_temp, inventory) = inventory();
        let plan = plan(&inventory).unwrap();
        let mut changed = plan.clone();
        changed.staging = "/tmp/other.staging".to_string();
        assert_ne!(plan.plan_id, plan_id(&changed));
        changed = plan.clone();
        changed.argv[5] = "0.28.1".to_string();
        assert_ne!(plan.plan_id, plan_id(&changed));
        changed = plan.clone();
        changed.permissions.push("extra".to_string());
        assert_ne!(plan.plan_id, plan_id(&changed));
        assert_eq!(plan.plan_id, plan_id(&plan.clone()));
    }

    #[test]
    fn plan_runs_the_install_on_any_sdk_and_verifies_on_the_runtime() {
        let (_temp, inventory) = inventory();
        let plan = plan(&inventory).unwrap();
        assert_eq!(plan.tool, TOOL);
        assert_eq!(plan.version, VERSION);
        assert_eq!(plan.source.url, NUGET_SOURCE);
        let argv0 = std::path::Path::new(&plan.argv[0]);
        assert_eq!(argv0.file_name().unwrap(), crate::sdk::dotnet_exe());
        assert_eq!(argv0.parent().unwrap().file_name().unwrap(), "sdk-root");
        assert_eq!(
            plan.dotnet_root,
            inventory.runtimes[0].root.to_string_lossy()
        );
        assert_eq!(&plan.argv[1..3], ["tool", "install"]);
        assert_eq!(plan.argv[3], TOOL);
        assert_eq!(&plan.argv[4..6], ["--version", VERSION]);
        assert_eq!(plan.argv[6], "--tool-path");
        assert!(plan.argv[7].ends_with("0.28.staging"));
        assert!(plan.argv.iter().all(|arg| !arg.contains("--global")));
    }

    #[test]
    fn plan_requires_the_runtime_and_an_sdk_separately() {
        let (_temp, mut detected) = inventory();
        detected.runtimes.clear();
        let error = plan(&detected).unwrap_err();
        assert_eq!(error.code, "sdk_missing");
        assert!(
            error.message.contains(".NET 10 runtime"),
            "{}",
            error.message
        );

        let (_temp, mut detected) = inventory();
        detected.installs.clear();
        let error = plan(&detected).unwrap_err();
        assert_eq!(error.code, "sdk_missing");
        assert!(
            error.message.contains("needs a .NET SDK"),
            "{}",
            error.message
        );
    }

    #[test]
    fn a_persisted_launcher_that_fails_version_verification_is_not_reused() {
        let dir = tempfile::tempdir().unwrap();
        let launcher = dir.path().join(binary_name());
        fs::write(&launcher, "#!/bin/sh\necho 'csharp-ls, version 0.27.0'\n").unwrap();
        let hash = sha256_file(&launcher).unwrap();
        record_installation(dir.path(), &hash).unwrap();
        assert!(persisted_installation_valid(dir.path()));
        assert!(!Tools::reusable_installation(dir.path(), Path::new("/tmp")));
        assert!(!dir.path().join(".openridr-integrity").exists());
    }

    #[test]
    fn cleanup_removes_only_the_attempt_download_directory() {
        let dir = tempfile::tempdir().unwrap();
        let current = dir.path().join("download-2");
        let stale = dir.path().join("download-1");
        fs::create_dir_all(&current).unwrap();
        fs::create_dir_all(&stale).unwrap();
        fs::write(current.join("keep"), "x").unwrap();
        cleanup_stale_downloads(&current);
        assert!(current.exists());
        assert!(!stale.exists());
    }

    #[test]
    fn the_worker_refuses_an_install_without_the_verified_package_feed() {
        crate::test_harness::prepare();
        let temp = tempfile::tempdir().unwrap();
        let plan = Plan {
            tool: TOOL,
            version: VERSION,
            package: TOOL,
            source: Source {
                name: "NuGet.org",
                url: NUGET_SOURCE,
                package_url: NUGET_PACKAGE,
            },
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            destination: temp
                .path()
                .join("destination")
                .to_string_lossy()
                .to_string(),
            staging: temp.path().join("staging").to_string_lossy().to_string(),
            dotnet_root: "/tmp".to_string(),
            argv: vec![
                "dotnet".to_string(),
                "tool".to_string(),
                "install".to_string(),
            ],
            cwd: temp.path().to_string_lossy().to_string(),
            permissions: vec![],
            integrity: "",
            existing: inspect_existing(&temp.path().join("destination")),
            plan_id: "test-plan".to_string(),
        };
        let tools = std::sync::Arc::new(Tools::new());
        let mut processes = Registry::new();
        let error = tools
            .begin_install_worker(plan, 1, 1, &mut processes, None, false)
            .expect_err("without local_package the worker must refuse");
        assert!(
            error.message.contains("verified local package"),
            "{}",
            error.message
        );
        assert!(processes.views().is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn promote_swaps_the_new_directory_without_a_missing_installation_window() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("0.28");
        let staging = temp.path().join("0.28.staging");
        fs::create_dir_all(&destination).unwrap();
        fs::create_dir_all(&staging).unwrap();
        fs::write(destination.join("old"), "old").unwrap();
        fs::write(staging.join("new"), "new").unwrap();
        promote(&staging, &destination).unwrap();
        assert!(destination.join("new").is_file());
        assert!(!destination.join("old").exists());
        // The previous directory stays in staging until post-swap verification succeeds.
        assert!(staging.join("old").is_file());
    }

    #[cfg(unix)]
    #[test]
    fn a_failed_atomic_swap_keeps_the_previous_destination() {
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("tools");
        fs::create_dir_all(&root).unwrap();
        let destination = root.join("0.28");
        let staging = root.join("0.28.staging");
        fs::create_dir_all(&destination).unwrap();
        fs::create_dir_all(&staging).unwrap();
        fs::write(destination.join("old"), "old").unwrap();
        fs::write(staging.join("new"), "new").unwrap();
        let permissions = fs::metadata(&root).unwrap().permissions();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o555)).unwrap();
        let result = promote(&staging, &destination);
        fs::set_permissions(&root, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(result.is_err());
        assert!(destination.join("old").is_file());
        assert!(!destination.join("new").exists());
        let _ = permissions;
    }

    #[test]
    fn promote_failure_keeps_the_previous_destination() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("0.28");
        fs::create_dir_all(&destination).unwrap();
        fs::write(destination.join("old"), "old").unwrap();
        let missing = temp.path().join("missing-staging");
        assert!(promote(&missing, &destination).is_err());
        assert!(destination.join("old").is_file());
    }

    fn worker_test_plan(temp: &Path) -> Plan {
        Plan {
            tool: TOOL,
            version: VERSION,
            package: TOOL,
            source: Source {
                name: "NuGet.org",
                url: NUGET_SOURCE,
                package_url: NUGET_PACKAGE,
            },
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            destination: temp.join("destination").to_string_lossy().to_string(),
            staging: temp.join("staging").to_string_lossy().to_string(),
            dotnet_root: "/tmp".to_string(),
            argv: vec![
                "dotnet".to_string(),
                "tool".to_string(),
                "install".to_string(),
            ],
            cwd: temp.to_string_lossy().to_string(),
            permissions: vec![],
            integrity: "",
            existing: inspect_existing(&temp.join("destination")),
            plan_id: "test-plan".to_string(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn the_worker_rechecks_the_package_hash_before_installing() {
        crate::test_harness::prepare();
        let temp = tempfile::tempdir().unwrap();
        let feed = temp.path().join("feed");
        fs::create_dir_all(&feed).unwrap();
        fs::write(
            feed.join("csharp-ls.0.28.0.nupkg"),
            b"not the pinned package",
        )
        .unwrap();
        let config = temp.path().join("offline.config");
        fs::write(&config, "<configuration />").unwrap();
        let tools = std::sync::Arc::new(Tools::new());
        let mut processes = Registry::new();
        let error = tools
            .begin_install_worker(
                worker_test_plan(temp.path()),
                1,
                1,
                &mut processes,
                Some((feed, config)),
                false,
            )
            .expect_err("a wrong package hash must stop before dotnet is spawned");
        assert!(
            error.message.contains("SHA-512 mismatch"),
            "{}",
            error.message
        );
        assert!(processes.views().is_empty());
    }

    #[test]
    fn the_neutral_cwd_is_created_before_the_launcher_runs() {
        // A fresh root has no `openridr-neutral`, and `current_dir` of a missing path fails the
        // spawn before the launcher runs (issue #93). This reproves it on a warm machine too.
        let root = tempfile::tempdir().unwrap();
        let neutral = root.path().join("openridr-neutral");
        assert!(!neutral.exists());
        assert_eq!(neutral_cwd_in(root.path()), neutral);
        assert!(neutral.is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn staged_verification_writes_integrity_and_a_write_failure_keeps_the_old_copy() {
        // `TMPDIR=$(mktemp -d)` makes this test fail on a warm machine too: a fresh temp root has no
        // `openridr-neutral`, the directory the launcher runs in (issue #93).
        use std::os::unix::fs::PermissionsExt;
        let temp = tempfile::tempdir().unwrap();
        let staging = temp.path().join("staging");
        fs::create_dir_all(&staging).unwrap();
        let launcher = staging.join(binary_name());
        fs::write(
            &launcher,
            "#!/bin/sh\nprintf 'csharp-ls, version 0.28.0\n'\n",
        )
        .unwrap();
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).unwrap();
        let existing = verify_staged_installation(&staging, Path::new("/tmp")).unwrap();
        assert_eq!(existing.state, "ready");
        assert!(staging.join(".openridr-integrity").is_file());

        // A read-only staging directory cannot receive the integrity record.
        let unwritable = temp.path().join("unwritable");
        fs::create_dir_all(&unwritable).unwrap();
        let launcher = unwritable.join(binary_name());
        fs::write(
            &launcher,
            "#!/bin/sh\nprintf 'csharp-ls, version 0.28.0\n'\n",
        )
        .unwrap();
        fs::set_permissions(&launcher, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(&unwritable, fs::Permissions::from_mode(0o555)).unwrap();
        let error = verify_staged_installation(&unwritable, Path::new("/tmp")).unwrap_err();
        fs::set_permissions(&unwritable, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(error.contains("integrity"), "{error}");
        assert!(!unwritable.join(".openridr-integrity").exists());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn rollback_after_a_failed_post_swap_check_restores_the_previous_directory() {
        let temp = tempfile::tempdir().unwrap();
        let destination = temp.path().join("0.28");
        let staging = temp.path().join("0.28.staging");
        fs::create_dir_all(&destination).unwrap();
        fs::create_dir_all(&staging).unwrap();
        fs::write(destination.join("old"), "old").unwrap();
        fs::write(staging.join("new"), "new").unwrap();
        promote(&staging, &destination).unwrap();
        assert!(destination.join("new").is_file());
        assert!(rollback_promotion(&staging, &destination));
        assert!(destination.join("old").is_file());
        assert!(!destination.join("new").exists());
        assert!(!staging.exists());
    }
}
