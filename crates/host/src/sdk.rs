use serde::Serialize;
use std::cmp::Ordering;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

pub const SDK_INSTRUCTIONS_URL: &str = "https://dotnet.microsoft.com/download/dotnet";
/// The analysis tool needs the runtime, so the missing-runtime link goes to the 10.0 page.
pub const RUNTIME_INSTRUCTIONS_URL: &str = "https://dotnet.microsoft.com/download/dotnet/10.0";
pub const ANALYSIS_RUNTIME_NAME: &str = "Microsoft.NETCore.App";

/// One SDK installation reported by one root's own muxer.
#[derive(Debug, Clone)]
pub struct Install {
    pub version: String,
    pub root: PathBuf,
    pub path: String,
}

/// One `Microsoft.NETCore.App` runtime reported by one root's own muxer.
#[derive(Debug, Clone)]
pub struct RuntimeInstall {
    pub version: String,
    pub root: PathBuf,
}

impl Install {
    /// `--list-sdks` reports the SDK parent directory (`<root>/sdk`); the version directory is
    /// `<path>/<version>`. The composite host root links that directory by version.
    pub fn sdk_dir(&self) -> PathBuf {
        std::path::Path::new(&self.path).join(&self.version)
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkVersion {
    pub version: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SdkRoot {
    pub root: String,
    pub source: &'static str,
    pub versions: Vec<SdkVersion>,
    pub exit: Option<i32>,
    pub error: Option<String>,
}

/// The .NET 10 runtime that executes the analysis tool (csharp-ls). It is a runtime, not the
/// project SDK and not an SDK install: a project pinned to SDK 8 runs on an installed 10 runtime.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisRuntime {
    pub state: &'static str,
    /// `Microsoft.NETCore.App` 10.x, never an SDK version.
    pub version: Option<String>,
    pub root: Option<String>,
    /// Always `None`: a runtime root has no SDK directory. Kept so the snapshot shape is stable.
    pub sdk_path: Option<String>,
    pub instructions_url: &'static str,
    pub message: Option<String>,
    /// True when the runtime comes from the app bundle instead of a detected installation.
    pub bundled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSdk {
    pub state: &'static str,
    pub global_json_path: Option<String>,
    pub requested_version: Option<String>,
    pub roll_forward: Option<String>,
    pub allow_prerelease: Option<bool>,
    pub resolved_version: Option<String>,
    pub root: Option<String>,
    pub sdk_path: Option<String>,
    pub instructions_url: &'static str,
    pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GlobalJsonRequest {
    pub path: String,
    pub version: Option<String>,
    pub roll_forward: Option<String>,
    pub allow_prerelease: Option<bool>,
}

impl ProjectSdk {
    fn empty(state: &'static str, message: impl Into<String>) -> Self {
        Self {
            state,
            global_json_path: None,
            requested_version: None,
            roll_forward: None,
            allow_prerelease: None,
            resolved_version: None,
            root: None,
            sdk_path: None,
            instructions_url: SDK_INSTRUCTIONS_URL,
            message: Some(message.into()),
        }
    }

    pub fn no_workspace() -> Self {
        Self::empty("no_workspace", "Open a workspace to inspect the project SDK.")
    }

    pub fn not_inspected(global_json: Option<&GlobalJsonRequest>) -> Self {
        Self {
            state: "not_inspected",
            global_json_path: global_json.map(|request| request.path.clone()),
            requested_version: global_json.and_then(|request| request.version.clone()),
            roll_forward: global_json.and_then(|request| request.roll_forward.clone()),
            allow_prerelease: global_json.and_then(|request| request.allow_prerelease),
            resolved_version: None,
            root: None,
            sdk_path: None,
            instructions_url: SDK_INSTRUCTIONS_URL,
            message: Some(
                "Trust the workspace to resolve global.json against the detected SDKs."
                    .to_string(),
            ),
        }
    }

    pub fn no_global_json() -> Self {
        Self::empty(
            "no_global_json",
            "No global.json at the workspace root: no project SDK is pinned. OpenRidr does not substitute the .NET 10 analysis runtime.",
        )
    }

    pub fn not_pinned(path: &str) -> Self {
        Self {
            state: "not_pinned",
            global_json_path: Some(path.to_string()),
            requested_version: None,
            roll_forward: None,
            allow_prerelease: None,
            resolved_version: None,
            root: None,
            sdk_path: None,
            instructions_url: SDK_INSTRUCTIONS_URL,
            message: Some(
                "global.json does not declare sdk.version, so this build does not claim a project SDK."
                    .to_string(),
            ),
        }
    }

    pub fn invalid(path: &str, message: impl Into<String>) -> Self {
        let mut view = Self::empty("invalid_global_json", message);
        view.global_json_path = Some(path.to_string());
        view
    }

    pub fn resolved(request: &GlobalJsonRequest, install: &Install) -> Self {
        Self {
            state: "resolved",
            global_json_path: Some(request.path.clone()),
            requested_version: request.version.clone(),
            roll_forward: Some(
                request
                    .roll_forward
                    .clone()
                    .unwrap_or_else(|| "patch".to_string()),
            ),
            allow_prerelease: Some(
                request
                    .allow_prerelease
                    .unwrap_or_else(|| request.version.as_deref().is_some_and(|v| v.contains('-'))),
            ),
            resolved_version: Some(install.version.clone()),
            root: Some(install.root.to_string_lossy().to_string()),
            sdk_path: Some(install.path.clone()),
            instructions_url: SDK_INSTRUCTIONS_URL,
            message: None,
        }
    }

    pub fn missing(request: &GlobalJsonRequest, message: impl Into<String>) -> Self {
        Self {
            state: "sdk_missing",
            global_json_path: Some(request.path.clone()),
            requested_version: request.version.clone(),
            roll_forward: Some(
                request
                    .roll_forward
                    .clone()
                    .unwrap_or_else(|| "patch".to_string()),
            ),
            allow_prerelease: request.allow_prerelease,
            resolved_version: None,
            root: None,
            sdk_path: None,
            instructions_url: SDK_INSTRUCTIONS_URL,
            message: Some(message.into()),
        }
    }
}

impl AnalysisRuntime {
    /// The runtime analysis actually uses: the app bundle wins, the detected inventory is the
    /// fallback for a development binary. A distributed app that lost its private runtime
    /// reports `missing` and never borrows an external 10.x to mask it.
    pub fn for_analysis(runtimes: &[RuntimeInstall]) -> Self {
        match crate::bundled::availability() {
            crate::bundled::Availability::Bundled(bundled) => Self {
                state: "found",
                version: Some(bundled.runtime.version.clone()),
                root: Some(bundled.runtime.root.to_string_lossy().to_string()),
                sdk_path: None,
                instructions_url: RUNTIME_INSTRUCTIONS_URL,
                message: Some(
                    "Bundled with OpenRidr: the .NET 10 runtime csharp-ls needs ships inside the app; the project SDK still comes from global.json."
                        .to_string(),
                ),
                bundled: true,
            },
            crate::bundled::Availability::Damaged(reason) => Self {
                state: "missing",
                version: None,
                root: None,
                sdk_path: None,
                instructions_url: RUNTIME_INSTRUCTIONS_URL,
                message: Some(reason),
                bundled: true,
            },
            crate::bundled::Availability::Absent => Self::from_runtimes(runtimes),
        }
    }

    pub fn from_runtimes(runtimes: &[RuntimeInstall]) -> Self {
        match analysis_runtime(runtimes) {
            Some(runtime) => Self {
                state: "found",
                version: Some(runtime.version.clone()),
                root: Some(runtime.root.to_string_lossy().to_string()),
                sdk_path: None,
                instructions_url: RUNTIME_INSTRUCTIONS_URL,
                message: None,
                bundled: false,
            },
            None => Self {
                state: "missing",
                version: None,
                root: None,
                sdk_path: None,
                instructions_url: RUNTIME_INSTRUCTIONS_URL,
                message: Some(
                    "No .NET 10 runtime (Microsoft.NETCore.App) was found in the detected roots. C# analysis needs that runtime; the project SDK is separate and is never used as a substitute."
                        .to_string(),
                ),
                bundled: false,
            },
        }
    }
}

pub struct Inventory {
    pub roots: Vec<SdkRoot>,
    pub installs: Vec<Install>,
    pub runtimes: Vec<RuntimeInstall>,
    pub inspected_at: u64,
}

fn analysis_runtime(runtimes: &[RuntimeInstall]) -> Option<&RuntimeInstall> {
    runtimes
        .iter()
        .filter(|runtime| {
            Version::parse(&runtime.version)
                .is_some_and(|version| version.major == 10 && version.prerelease.is_none())
        })
        .max_by(|left, right| {
            Version::parse(&left.version)
                .unwrap()
                .cmp(&Version::parse(&right.version).unwrap())
                .then_with(|| right.root.cmp(&left.root))
        })
}

/// Runs each detected root's own muxer from a neutral cwd. DOTNET_ROOT is never set because
/// it does not merge roots on this platform.
pub fn inventory() -> Inventory {
    let mut roots = Vec::new();
    let mut installs = Vec::new();
    let mut runtimes = Vec::new();

    for (root, source) in discover_roots() {
        let muxer = root.join(dotnet_exe());
        match run_muxer(&root, "--list-sdks") {
            Ok(output) => {
                let text = String::from_utf8_lossy(&output.stdout).to_string();
                let parsed = parse_list_sdks(&text);
                let versions = parsed
                    .iter()
                    .map(|(version, path)| SdkVersion {
                        version: version.clone(),
                        path: path.clone(),
                    })
                    .collect();
                for (version, path) in parsed {
                    installs.push(Install {
                        version,
                        root: root.clone(),
                        path,
                    });
                }
                let error = (!output.status.success()).then(|| {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    let stderr = stderr.trim();
                    if stderr.is_empty() {
                        "dotnet --list-sdks failed without stderr output.".to_string()
                    } else {
                        stderr.to_string()
                    }
                });
                roots.push(SdkRoot {
                    root: root.to_string_lossy().to_string(),
                    source,
                    versions,
                    exit: output.status.code(),
                    error,
                });
            }
            Err(error) => roots.push(SdkRoot {
                root: root.to_string_lossy().to_string(),
                source,
                versions: Vec::new(),
                exit: None,
                error: Some(format!("Could not run {}: {error}", muxer.to_string_lossy())),
            }),
        }

        // A runtime-only root lists no SDKs, so the runtime is read independently.
        if let Ok(output) = run_muxer(&root, "--list-runtimes") {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                for (name, version) in parse_list_runtimes(&text) {
                    if name == ANALYSIS_RUNTIME_NAME {
                        runtimes.push(RuntimeInstall {
                            version,
                            root: root.clone(),
                        });
                    }
                }
            }
        }
    }

    Inventory {
        roots,
        installs,
        runtimes,
        inspected_at: now_ms(),
    }
}

fn run_muxer(root: &Path, arg: &str) -> std::io::Result<std::process::Output> {
    Command::new(root.join(dotnet_exe()))
        .arg(arg)
        .current_dir(env::temp_dir())
        .env_remove("DOTNET_ROOT")
        .env_remove("DOTNET_MULTILEVEL_LOOKUP")
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .env("DOTNET_NOLOGO", "1")
        .env("DOTNET_CLI_UI_LANGUAGE", "en")
        .output()
}

pub fn parse_list_sdks(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            let open = line.find(" [")?;
            let version = line[..open].trim();
            let rest = &line[open + 2..];
            let end = rest.rfind(']')?;
            let path = rest[..end].trim();
            if Version::parse(version).is_none() || path.is_empty() {
                return None;
            }
            Some((version.to_string(), path.to_string()))
        })
        .collect()
}

pub fn parse_list_runtimes(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let name = parts.next()?;
            let version = parts.next()?;
            Version::parse(version)?;
            Some((name.to_string(), version.to_string()))
        })
        .collect()
}

fn discover_roots() -> Vec<(PathBuf, &'static str)> {
    let system: Vec<PathBuf> = SYSTEM_ROOTS.iter().map(PathBuf::from).collect();
    candidate_roots(
        home_dir(),
        env::var_os("DOTNET_ROOT").map(PathBuf::from),
        env::var_os("PATH"),
        &system,
    )
}

#[cfg(target_os = "macos")]
const SYSTEM_ROOTS: &[&str] = &[
    "/usr/local/share/dotnet",
    "/opt/homebrew/opt/dotnet",
    "/opt/homebrew/opt/dotnet/libexec",
];
#[cfg(not(target_os = "macos"))]
const SYSTEM_ROOTS: &[&str] = &["/usr/local/share/dotnet"];

/// Ordered candidates: DOTNET_ROOT, fixed system roots, ~/.dotnet, managed SDKs and every
/// PATH entry. Only directories with a muxer are kept, already resolved to their real root.
fn candidate_roots(
    home: Option<PathBuf>,
    dotnet_root: Option<PathBuf>,
    path: Option<OsString>,
    system: &[PathBuf],
) -> Vec<(PathBuf, &'static str)> {
    let mut roots: Vec<(PathBuf, &'static str)> = Vec::new();

    if let Some(root) = dotnet_root.as_deref().and_then(muxer_root) {
        roots.push((root, "dotnet-root"));
    }
    for dir in system {
        if let Some(root) = muxer_root(dir) {
            roots.push((root, "system"));
        }
    }

    if let Some(home) = home {
        if let Some(root) = muxer_root(&home.join(".dotnet")) {
            roots.push((root, "dotnet-home"));
        }
        let user_sdks = home.join("Library/Application Support/OpenRidr/sdks");
        if let Ok(entries) = fs::read_dir(&user_sdks) {
            let mut dirs: Vec<PathBuf> = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.is_dir())
                .collect();
            dirs.sort();
            for dir in dirs {
                if let Some(root) = muxer_root(&dir) {
                    roots.push((root, "user-sdks"));
                }
            }
        }
    }

    if let Some(path) = path {
        for dir in env::split_paths(&path) {
            if let Some(root) = muxer_root(&dir) {
                roots.push((root, "path"));
            }
        }
    }
    dedupe_roots(roots)
}

/// Resolves a candidate directory to the real installation root of its muxer, so a PATH
/// symlink and the root it points at dedupe to the same root.
fn muxer_root(dir: &Path) -> Option<PathBuf> {
    let muxer = dir.join(dotnet_exe());
    if !muxer.is_file() {
        return None;
    }
    let canonical = fs::canonicalize(&muxer).unwrap_or(muxer);
    canonical.parent().map(Path::to_path_buf)
}

fn dedupe_roots(roots: Vec<(PathBuf, &'static str)>) -> Vec<(PathBuf, &'static str)> {
    let mut deduped: Vec<(PathBuf, &'static str)> = Vec::new();
    for (root, source) in roots {
        if !deduped.iter().any(|(known, _)| *known == root) {
            deduped.push((root, source));
        }
    }
    deduped
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn dotnet_exe() -> &'static str {
    if cfg!(windows) {
        "dotnet.exe"
    } else {
        "dotnet"
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub prerelease: Option<String>,
}

impl Version {
    // shortcut: prerelease ordering is lexical and global.json is parsed as strict JSON;
    // upgrade when fixtures need SemVer build metadata or JSON comments.
    pub fn parse(raw: &str) -> Option<Self> {
        let core = raw.split('+').next().unwrap_or(raw);
        let (numbers, prerelease) = match core.split_once('-') {
            Some((numbers, suffix)) => (numbers, Some(suffix.to_string())),
            None => (core, None),
        };
        let mut parts = numbers.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next().unwrap_or("0").parse().ok()?;
        Some(Self {
            major,
            minor,
            patch,
            prerelease,
        })
    }

    fn band(&self) -> u64 {
        self.major * 1000 + self.minor * 100 + self.patch / 100
    }

    fn same_band(&self, other: &Self) -> bool {
        self.major == other.major
            && self.minor == other.minor
            && self.patch / 100 == other.patch / 100
    }

    fn same_minor(&self, other: &Self) -> bool {
        self.major == other.major && self.minor == other.minor
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (&self.prerelease, &other.prerelease) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(left), Some(right)) => left.cmp(right),
            })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollForward {
    Disable,
    Patch,
    Feature,
    Minor,
    Major,
    LatestPatch,
    LatestFeature,
    LatestMinor,
    LatestMajor,
}

impl RollForward {
    pub fn parse(value: Option<&str>) -> Option<Self> {
        match value.unwrap_or("patch").to_lowercase().as_str() {
            "disable" => Some(Self::Disable),
            "patch" => Some(Self::Patch),
            "feature" => Some(Self::Feature),
            "minor" => Some(Self::Minor),
            "major" => Some(Self::Major),
            "latestpatch" => Some(Self::LatestPatch),
            "latestfeature" => Some(Self::LatestFeature),
            "latestminor" => Some(Self::LatestMinor),
            "latestmajor" => Some(Self::LatestMajor),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disable => "disable",
            Self::Patch => "patch",
            Self::Feature => "feature",
            Self::Minor => "minor",
            Self::Major => "major",
            Self::LatestPatch => "latestPatch",
            Self::LatestFeature => "latestFeature",
            Self::LatestMinor => "latestMinor",
            Self::LatestMajor => "latestMajor",
        }
    }
}

/// Resolves global.json against the detected installs. Staged selection mirrors .NET:
/// a lower stage is only used when the preferred stage has no forward candidate.
pub fn resolve(
    request_version: &str,
    roll_forward: Option<&str>,
    allow_prerelease: Option<bool>,
    installs: &[Install],
) -> Result<Install, String> {
    let requested = Version::parse(request_version)
        .ok_or_else(|| format!("`{request_version}` is not a valid .NET SDK version."))?;
    let policy = RollForward::parse(roll_forward).ok_or_else(|| {
        format!(
            "rollForward `{}` is not a policy this build understands.",
            roll_forward.unwrap_or("")
        )
    })?;
    let allow = allow_prerelease.unwrap_or(requested.prerelease.is_some());

    let mut candidates: Vec<(&Install, Version)> = installs
        .iter()
        .filter_map(|install| {
            let version = Version::parse(&install.version)?;
            if version < requested || (!allow && version.prerelease.is_some()) {
                return None;
            }
            Some((install, version))
        })
        .collect();
    candidates.sort_by(|(left_install, left), (right_install, right)| {
        right
            .cmp(left)
            .then_with(|| left_install.root.cmp(&right_install.root))
    });

    let best = |filter: &dyn Fn(&Version) -> bool| -> Option<Install> {
        candidates
            .iter()
            .find(|(_, version)| filter(version))
            .map(|(install, _)| (*install).clone())
    };

    let same_band = |version: &Version| version.same_band(&requested);
    let same_minor = |version: &Version| version.same_minor(&requested);
    let higher_band = |version: &Version| {
        version.same_minor(&requested) && version.band() > requested.band()
    };
    let higher_minor = |version: &Version| {
        version.major == requested.major && version.minor > requested.minor
    };

    let selected = match policy {
        RollForward::Disable => best(&|version| *version == requested),
        // .NET's default `patch` keeps the exact pinned version when it is installed and
        // only rolls to the latest patch in the same band when it is absent.
        RollForward::Patch => best(&|version| *version == requested).or_else(|| best(&same_band)),
        RollForward::LatestPatch => best(&same_band),
        RollForward::Feature => best(&same_band).or_else(|| best(&higher_band)),
        RollForward::LatestFeature => best(&same_minor),
        RollForward::Minor => best(&same_band)
            .or_else(|| best(&higher_band))
            .or_else(|| best(&higher_minor)),
        RollForward::LatestMinor => best(&|version| version.major == requested.major),
        RollForward::Major => best(&same_band)
            .or_else(|| best(&higher_band))
            .or_else(|| best(&higher_minor))
            .or_else(|| best(&|version| version.major > requested.major)),
        RollForward::LatestMajor => best(&|_| true),
    };

    selected.ok_or_else(|| {
        let detected = if installs.is_empty() {
            "none".to_string()
        } else {
            installs
                .iter()
                .map(|install| install.version.clone())
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!(
            "No detected .NET SDK satisfies global.json version {request_version} with rollForward {} (detected: {detected}). OpenRidr does not rewrite global.json and does not substitute the .NET 10 analysis runtime.",
            policy.as_str()
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn installs(items: &[(&str, &str)]) -> Vec<Install> {
        items
            .iter()
            .map(|(version, root)| Install {
                version: version.to_string(),
                root: PathBuf::from(root),
                path: format!("{root}/sdk/{version}"),
            })
            .collect()
    }

    #[test]
    fn exact_with_disable_picks_only_the_pinned_version() {
        let detected = installs(&[
            ("8.0.408", "/sdks/8"),
            ("8.0.416", "/system"),
            ("10.0.100", "/system"),
        ]);
        let resolved = resolve("8.0.408", Some("disable"), None, &detected).unwrap();
        assert_eq!(resolved.version, "8.0.408");
        assert_eq!(resolved.root, PathBuf::from("/sdks/8"));

        let error = resolve("8.0.409", Some("disable"), None, &detected).unwrap_err();
        assert!(error.contains("8.0.409"));
        assert!(error.contains("does not rewrite global.json"));
    }

    #[test]
    fn feature_prefers_the_pinned_band_then_rolls_to_the_next_band() {
        let detected = installs(&[("8.0.408", "/a"), ("8.0.416", "/b"), ("8.0.500", "/c")]);
        assert_eq!(
            resolve("8.0.408", Some("feature"), None, &detected)
                .unwrap()
                .version,
            "8.0.416"
        );
        let missing_band = installs(&[("8.0.500", "/c")]);
        assert_eq!(
            resolve("8.0.408", Some("feature"), None, &missing_band)
                .unwrap()
                .version,
            "8.0.500"
        );
    }

    #[test]
    fn patch_keeps_the_pin_then_uses_the_same_band() {
        let detected = installs(&[("8.0.408", "/a"), ("8.0.416", "/b")]);
        assert_eq!(
            resolve("8.0.408", Some("patch"), None, &detected)
                .unwrap()
                .version,
            "8.0.408"
        );
        let missing_pin = installs(&[("8.0.416", "/b"), ("8.0.500", "/c")]);
        assert_eq!(
            resolve("8.0.408", Some("patch"), None, &missing_pin)
                .unwrap()
                .version,
            "8.0.416"
        );
    }

    #[test]
    fn minor_rolls_forward_within_the_same_major() {
        let detected = installs(&[
            ("8.0.408", "/a"),
            ("8.1.100", "/b"),
            ("9.0.100", "/c"),
        ]);
        assert_eq!(
            resolve("8.0.408", Some("minor"), None, &detected)
                .unwrap()
                .version,
            "8.0.408"
        );
        let next_minor = installs(&[("8.1.100", "/b"), ("9.0.100", "/c")]);
        assert_eq!(
            resolve("8.0.408", Some("minor"), None, &next_minor)
                .unwrap()
                .version,
            "8.1.100"
        );
    }

    #[test]
    fn latest_major_picks_the_highest_install() {
        let detected = installs(&[("8.0.416", "/a"), ("10.0.100", "/b")]);
        let resolved = resolve("8.0.408", Some("latestMajor"), None, &detected).unwrap();
        assert_eq!(resolved.version, "10.0.100");
        assert_eq!(resolved.root, PathBuf::from("/b"));
    }

    #[test]
    fn prerelease_is_excluded_unless_allowed() {
        let detected = installs(&[("8.0.408", "/a"), ("8.0.409-preview.1", "/b")]);
        assert_eq!(
            resolve("8.0.408", Some("latestPatch"), None, &detected)
                .unwrap()
                .version,
            "8.0.408"
        );
        assert_eq!(
            resolve("8.0.408", Some("latestPatch"), Some(true), &detected)
                .unwrap()
                .version,
            "8.0.409-preview.1"
        );
    }

    #[test]
    fn missing_has_a_distinct_message_and_never_falls_back() {
        let detected = installs(&[("10.0.100", "/system")]);
        let error = resolve("8.0.999", Some("disable"), None, &detected).unwrap_err();
        assert!(error.contains("8.0.999"));
        assert!(error.contains("rollForward disable"));
        assert!(error.contains("10.0.100"));
    }

    #[test]
    fn highest_satisfying_install_wins_across_roots() {
        let detected = installs(&[("8.0.408", "/a"), ("8.0.416", "/b")]);
        let resolved = resolve("8.0.408", Some("latestPatch"), None, &detected).unwrap();
        assert_eq!(resolved.version, "8.0.416");
        assert_eq!(resolved.root, PathBuf::from("/b"));
    }

    #[test]
    fn list_sdks_parsing_keeps_versions_with_paths_only() {
        let text = "8.0.408 [/Users/me/sdks/8.0.408/sdk]\n10.0.100 [/usr/local/share/dotnet/sdk]\n\nnot an sdk line\n";
        let parsed = parse_list_sdks(text);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].0, "8.0.408");
        assert_eq!(parsed[1].1, "/usr/local/share/dotnet/sdk");
    }

    fn root_with_muxer(dir: &Path) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(dotnet_exe()), "").unwrap();
    }

    #[test]
    fn candidate_roots_cover_every_source_once_in_order() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("home");
        let managed = home.join("Library/Application Support/OpenRidr/sdks/8.0.408");
        let dotnet_home = home.join(".dotnet");
        let dotnet_root = temp.path().join("dotnet-root");
        let system = temp.path().join("system");
        let path_a = temp.path().join("path-a");
        let path_b = temp.path().join("path-b");
        for dir in [&managed, &dotnet_home, &dotnet_root, &system, &path_a, &path_b] {
            root_with_muxer(dir);
        }
        // A directory without a muxer, a duplicate entry and a repeated PATH entry are all skipped.
        let empty = temp.path().join("empty");
        fs::create_dir_all(&empty).unwrap();
        let path = env::join_paths([&path_a, &empty, &path_a, &path_b]).unwrap();

        let roots = candidate_roots(Some(home), Some(dotnet_root), Some(path), &[system]);
        let got: Vec<(PathBuf, &str)> = roots
            .iter()
            .map(|(root, source)| (root.clone(), *source))
            .collect();
        let expected: Vec<(PathBuf, &str)> = [
            (&temp.path().join("dotnet-root"), "dotnet-root"),
            (&temp.path().join("system"), "system"),
            (&dotnet_home, "dotnet-home"),
            (&managed, "user-sdks"),
            (&path_a, "path"),
            (&path_b, "path"),
        ]
        .iter()
        .map(|(dir, source)| (fs::canonicalize(dir).unwrap(), *source))
        .collect();
        assert_eq!(got, expected);
    }

    #[cfg(unix)]
    #[test]
    fn path_symlinks_resolve_to_one_canonical_root() {
        let temp = tempfile::tempdir().unwrap();
        let real = temp.path().join("real");
        root_with_muxer(&real);
        let bin = temp.path().join("bin");
        fs::create_dir_all(&bin).unwrap();
        std::os::unix::fs::symlink(real.join(dotnet_exe()), bin.join(dotnet_exe())).unwrap();
        let path = env::join_paths([&bin, &real]).unwrap();

        let roots = candidate_roots(None, None, Some(path), &[]);
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].0, fs::canonicalize(&real).unwrap());
        assert_eq!(roots[0].1, "path");
    }

    #[test]
    fn list_runtimes_parsing_keeps_name_and_version() {
        let text = "Microsoft.NETCore.App 10.0.1 [/usr/local/share/dotnet/shared/Microsoft.NETCore.App]\nMicrosoft.AspNetCore.App 10.0.1 [/usr/local/share/dotnet/shared/Microsoft.AspNetCore.App]\nnot a runtime\n";
        let parsed = parse_list_runtimes(text);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0], ("Microsoft.NETCore.App".to_string(), "10.0.1".to_string()));
        assert_eq!(parsed[1].0, "Microsoft.AspNetCore.App");
    }

    #[test]
    fn runtime_10_is_selected_without_an_sdk_10() {
        let runtimes = vec![
            RuntimeInstall {
                version: "9.0.8".to_string(),
                root: PathBuf::from("/nine"),
            },
            RuntimeInstall {
                version: "10.0.1-preview.1".to_string(),
                root: PathBuf::from("/preview"),
            },
            RuntimeInstall {
                version: "10.0.0".to_string(),
                root: PathBuf::from("/older"),
            },
            RuntimeInstall {
                version: "10.0.1".to_string(),
                root: PathBuf::from("/ten"),
            },
        ];
        let runtime = AnalysisRuntime::from_runtimes(&runtimes);
        assert_eq!(runtime.state, "found");
        assert_eq!(runtime.version.as_deref(), Some("10.0.1"));
        assert_eq!(runtime.root.as_deref(), Some("/ten"));
        assert_eq!(runtime.sdk_path, None);
        assert_eq!(runtime.instructions_url, RUNTIME_INSTRUCTIONS_URL);
        assert_eq!(
            AnalysisRuntime::from_runtimes(&runtimes[..2]).state,
            "missing"
        );
    }

    #[test]
    fn project_pin_resolves_with_runtime_10_present() {
        let installs = installs(&[("8.0.408", "/sdks/8")]);
        let runtimes = vec![RuntimeInstall {
            version: "10.0.1".to_string(),
            root: PathBuf::from("/runtime"),
        }];
        let resolved = resolve("8.0.408", Some("disable"), None, &installs).unwrap();
        assert_eq!(resolved.version, "8.0.408");
        assert_eq!(resolved.root, PathBuf::from("/sdks/8"));
        assert_eq!(
            AnalysisRuntime::from_runtimes(&runtimes).version.as_deref(),
            Some("10.0.1")
        );
    }

    #[test]
    fn runtime_missing_and_project_sdk_missing_are_distinct() {
        let runtime = AnalysisRuntime::from_runtimes(&[]);
        assert_eq!(runtime.state, "missing");
        let message = runtime.message.as_deref().unwrap();
        assert!(message.contains(".NET 10 runtime"), "{message}");
        assert!(!message.contains("SDK 10"), "{message}");
        assert_eq!(runtime.instructions_url, RUNTIME_INSTRUCTIONS_URL);

        let project = ProjectSdk::missing(
            &GlobalJsonRequest {
                path: "global.json".to_string(),
                version: Some("8.0.408".to_string()),
                roll_forward: Some("disable".to_string()),
                allow_prerelease: None,
            },
            "No detected .NET SDK satisfies global.json version 8.0.408 with rollForward disable.",
        );
        assert_eq!(project.state, "sdk_missing");
        assert_eq!(project.instructions_url, SDK_INSTRUCTIONS_URL);
        assert_ne!(project.instructions_url, runtime.instructions_url);
    }
}
