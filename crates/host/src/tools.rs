//! csharp-ls installation with an explicit, revalidated plan.
//!
//! The host never installs a global tool: the only command is `dotnet tool install` with
//! `--tool-path` inside the app home. A candidate is installed into a staging directory,
//! promoted only after the process exits 0, then verified; no state becomes `ready` before
//! the launcher exists and its `--version` output contains the pinned version.

use crate::error::{HostError, HostResult};
use crate::logbuf::{self, LogBuffer};
use crate::process::{ProcessStatus, Registry, Termination};
use crate::sdk::{AnalysisRuntime, Install, Inventory, Version};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub const TOOL: &str = "csharp-ls";
pub const VERSION: &str = "0.28.0";
pub const NUGET_SOURCE: &str = "https://api.nuget.org/v3/index.json";
pub const NUGET_PACKAGE: &str = "https://www.nuget.org/packages/csharp-ls/0.28.0";
const SIGNATURE_LIMITATION: &str = "dotnet tool install reports \"Skipping NuGet package signature verification.\" OpenRidr records the SHA-256 of the installed launcher and the exact --version output; it does not claim a verified package signature or an official package hash.";

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

fn neutral_cwd() -> PathBuf {
    std::env::temp_dir().join("openridr-neutral")
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
fn probe_muxer() -> Option<PathBuf> {
    if std::env::var("OPENRIDR_ENABLE_PROBE").as_deref() != Ok("1") {
        return None;
    }
    std::env::var_os("OPENRIDR_TOOLS_MUXER").map(PathBuf::from)
}

/// Builds the plan from the detected runtimes and SDKs. `dotnet tool install` runs on any
/// installed SDK; the launcher is verified against the .NET 10 runtime root.
pub fn plan(inventory: &Inventory) -> HostResult<Plan> {
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
        install
            .root
            .join(if cfg!(windows) { "dotnet.exe" } else { "dotnet" })
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
            format!("network: {NUGET_SOURCE} (NuGet.org over HTTPS; the child runs with an isolated HOME and its default NuGet source)"),
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
    let output = Command::new(&binary)
        .arg("--version")
        .current_dir(neutral_cwd())
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
            reason: Some(format!("Could not run {} --version.", binary.to_string_lossy())),
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
    let digest = Sha256::digest(&bytes);
    Some(
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    )
}

/// Bytes currently on disk under `root`; symlinks are not followed. Progress only ever reports
/// what the installer actually wrote, never an estimated percentage.
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
    /// Bytes observed on disk during this attempt; NuGet reports no total, so it is never invented.
    pub downloaded: u64,
    pub total: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolView {
    pub state: &'static str,
    pub version: &'static str,
    pub destination: String,
    pub staging: String,
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
    plan: Option<Plan>,
    confirmed_plan_id: Option<String>,
    log: LogBuffer,
    progress: Option<Progress>,
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
}

pub struct Tools {
    data: Mutex<Data>,
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
                plan: None,
                confirmed_plan_id: None,
                log: LogBuffer::default(),
                progress: None,
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
        ToolView {
            state: data.state,
            version: VERSION,
            destination: destination.to_string_lossy().to_string(),
            staging: staging.to_string_lossy().to_string(),
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
            staging_present: staging.is_dir(),
            destination_present: destination.is_dir(),
        }
    }

    /// Re-checks the recorded launcher hash without starting anything. A replaced/corrupted
    /// launcher is not silently treated as the verified tool.
    pub fn still_ready(&self) -> bool {
        let data = self.data.lock().unwrap();
        if data.state != "ready" {
            return false;
        }
        let binary = Path::new(&data.destination).join(binary_name());
        match (&data.sha256, sha256_file(&binary)) {
            (Some(expected), Some(actual)) => expected == &actual,
            _ => false,
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

    /// Starts the real install. The caller has already validated trust, confirmation and that
    /// the plan id still matches the material fields.
    pub fn begin_install(
        self: &Arc<Self>,
        plan: Plan,
        workspace_id: u64,
        context_id: u64,
        processes: &mut Registry,
    ) -> HostResult<Value> {
        if let Some(process_id) = self.running_process_id() {
            return Err(HostError::new(
                "invalid_state",
                format!("An installation is already running (process {process_id})."),
            ));
        }
        // Confirmation is the executable step: only now is the existing launcher run (read-only
        // `--version`) to decide reuse versus replacement. A valid tool is never overwritten.
        let existing = verify_installed(
            Path::new(&plan.destination),
            Path::new(&plan.dotnet_root),
        );
        if existing.state == "ready" {
            let destination = plan.destination.clone();
            let staging = plan.staging.clone();
            let mut data = self.data.lock().unwrap();
            data.plan = Some(plan);
            data.confirmed_plan_id = None;
            data.destination = destination;
            data.staging = staging;
            data.state = "ready";
            data.reused = true;
            data.error = None;
            data.progress = None;
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
            data.log
                .push("Existing installation verified; reused without downloading or overwriting.".to_string());
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
        let cwd = PathBuf::from(&plan.cwd);
        fs::create_dir_all(&cwd).map_err(|error| {
            HostError::new(
                "operation_failed",
                format!("Could not create the neutral install directory: {error}"),
            )
        })?;
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

        let mut command = Command::new(&plan.argv[0]);
        command
            .args(&plan.argv[1..])
            .current_dir(&cwd)
            // Isolated HOME pins the child to NuGet's default official source and keeps all
            // writes under the app home; user feed configuration is neither read nor logged.
            .env("HOME", &home)
            .env("TMPDIR", &temp)
            .env("DOTNET_NOLOGO", "1")
            .env("DOTNET_CLI_UI_LANGUAGE", "en")
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
            .env_remove("DOTNET_ROOT")
            .env_remove("DOTNET_MULTILEVEL_LOOKUP")
            .env_remove("DOTNET_CLI_HOME")
            .env_remove("NUGET_PACKAGES")
            .env_remove("NUGET_HTTP_CACHE_PATH");

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
                data.log
                    .push(format!("Could not start {argv}: {error}"));
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
        data.reused = false;
        data.exit = None;
        data.termination = None;
        data.awaiting_decision = false;
        data.sha256 = None;
        data.verified_version = None;
        data.verified_output = None;
        data.error = None;
        data.progress = None;
        data.progress_base = progress_base;
        data.progress_peak = 0;
        data.note = Some(
            "dotnet tool install reports no percentage; progress is measured from bytes it writes to its isolated NuGet cache and staging."
                .to_string(),
        );
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
        if state == "downloading" {
            let staging = PathBuf::from(&data.staging);
            let temp = install_temp(&data.staging);
            let observed = directory_bytes(&staging)
                + directory_bytes(&child_home())
                + directory_bytes(&temp);
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
                        "Installation cancelled; no partial result is reported as ready.".to_string(),
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
                    return;
                }
                data.state = "installing";
                data.installing_since = Some(now_ms());
                data.log.push(
                    "Install process exited 0; promoting the staging directory.".to_string(),
                );
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
                    let failed_dir = PathBuf::from(format!(
                        "{}.failed-{}",
                        destination.to_string_lossy(),
                        now_ms()
                    ));
                    let moved = fs::rename(&destination, &failed_dir).is_ok();
                    data.state = "failed";
                    data.progress = None;
                    data.error = Some(format!(
                        "Verification failed: {}",
                        existing
                            .reason
                            .clone()
                            .unwrap_or_else(|| "no verified version".to_string())
                    ));
                    if let Some(output) = &existing.output {
                        data.log.push(output.clone());
                    }
                    data.note = Some(if moved {
                        format!(
                            "The unverified installation was moved to {} and is not ready.",
                            failed_dir.to_string_lossy()
                        )
                    } else {
                        "The unverified destination was left in place and is not ready."
                            .to_string()
                    });
                    data.finished_at = Some(now_ms());
                    return;
                }

                data.state = "ready";
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
                "Installation plan cancelled; no download and no process were started."
                    .to_string(),
            );
            data.finished_at = Some(now_ms());
            data.log
                .push("Plan cancelled; nothing was downloaded or executed.".to_string());
            return Ok(());
        }
        let Some(process_id) = data.process_id else {
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
            if !matches!(data.state, "downloading" | "installing" | "verifying")
                || data.sealed
            {
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

/// Same promotion semantics as a file save: the destination is replaced only after the
/// staged tree exists; a failed rename restores the previous destination.
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
    use crate::sdk::RuntimeInstall;

    fn inventory() -> (tempfile::TempDir, Inventory) {
        let temp = tempfile::tempdir().unwrap();
        let sdk_root = temp.path().join("sdk-root");
        fs::create_dir_all(&sdk_root).unwrap();
        fs::write(sdk_root.join("dotnet"), "").unwrap();
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
        assert!(plan.argv[0].ends_with("sdk-root/dotnet"));
        assert_eq!(plan.dotnet_root, inventory.runtimes[0].root.to_string_lossy());
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
        assert!(error.message.contains(".NET 10 runtime"), "{}", error.message);

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
}
