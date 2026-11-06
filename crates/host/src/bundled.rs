//! Analysis tools shipped inside the app bundle.
//!
//! macOS ARM64 only: `Contents/Resources/csharp-ls` holds the csharp-ls 0.28.0 payload and
//! `Contents/Resources/dotnet` holds the private `Microsoft.NETCore.App` 10 runtime that
//! executes it. The project SDK is never bundled and is still resolved from `global.json`.
//! Debug builds may point both roots elsewhere (`OPENRIDR_BUNDLED_CSHARP_LS`,
//! `OPENRIDR_BUNDLED_DOTNET_ROOT`) so tests and isolated runs exercise the bundled path without
//! a real app bundle. Nothing here downloads, installs or executes a process.

use crate::sdk::Version;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Test/dev override for the bundled csharp-ls payload directory. Never set by the app itself.
pub const CSHARP_LS_OVERRIDE: &str = "OPENRIDR_BUNDLED_CSHARP_LS";
/// Test/dev override for the bundled private runtime root. Never set by the app itself.
pub const RUNTIME_OVERRIDE: &str = "OPENRIDR_BUNDLED_DOTNET_ROOT";
/// Debug-only seam that makes a test binary behave like the distributed app: the pair must
/// ship with it, so a missing or damaged pair fails closed instead of falling back to a
/// download. Never set by the app itself.
pub const REQUIRED_OVERRIDE: &str = "OPENRIDR_BUNDLED_REQUIRED";

/// What the user has to do when the distributed app lost its analyzer. There is no download and
/// no external runtime to install: the app itself is the only source.
pub const REINSTALL_GUIDANCE: &str = "Reinstall OpenRidr; this app bundle must ship the analyzer and its private .NET runtime, and nothing is downloaded on first use.";

pub const CSHARP_LS_DIR: &str = "csharp-ls";
pub const RUNTIME_DIR: &str = "dotnet";
/// The bundled language server is the managed entry point, run by the bundled muxer.
pub const CSHARP_LS_ENTRY: &str = "CSharpLanguageServer.dll";

fn supported() -> bool {
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
}

#[cfg(debug_assertions)]
fn override_path(key: &str) -> Option<PathBuf> {
    env::var_os(key).map(PathBuf::from)
}

#[cfg(not(debug_assertions))]
fn override_path(_key: &str) -> Option<PathBuf> {
    // Release builds only ever read the app bundle; an environment variable cannot redirect
    // the analyzer or its runtime to a path the user controls.
    None
}

/// `Contents/Resources` of the running app. `None` outside the distributed platform, and for
/// binaries that are not inside an app bundle (`tauri dev`, `cargo test`, the HTTP bridge).
pub fn resources_root() -> Option<PathBuf> {
    if !supported() {
        return None;
    }
    let exe = env::current_exe().ok()?;
    Some(exe.parent()?.parent()?.join("Resources"))
}

/// The private runtime root, if it has a muxer plus a 10.x host/fxr and 10.x app runtime.
#[derive(Debug, Clone)]
pub struct Runtime {
    pub root: PathBuf,
    /// `Microsoft.NETCore.App` version directory (`10.0.14`), never the fxr version.
    pub version: String,
}

/// The bundled pair the host may use instead of the downloaded tool.
#[derive(Debug, Clone)]
pub struct Analysis {
    pub tool_dir: PathBuf,
    pub entry: PathBuf,
    pub runtime: Runtime,
}

/// The bundled pair, if this app bundle ships a complete one.
pub fn analysis() -> Option<Analysis> {
    match availability() {
        Availability::Bundled(analysis) => Some(analysis),
        _ => None,
    }
}

/// What the analyzer is in this build: shipped and complete, not shipped at all (a development
/// binary), or expected but missing/damaged (the distributed app). The last case must fail
/// closed: no plan, no download and no external runtime to mask it.
#[derive(Debug, Clone)]
pub enum Availability {
    Bundled(Analysis),
    Absent,
    Damaged(String),
}

/// True when the running binary lives inside an app bundle (`OpenRidr.app/Contents/MacOS`).
/// `tauri dev`, `cargo test` and the HTTP bridge run outside one and may fall back to the
/// download flow. The check is path-based, never environment-based, in release builds.
pub fn in_app_bundle() -> bool {
    if !supported() {
        return false;
    }
    let Ok(exe) = env::current_exe() else {
        return false;
    };
    exe.components()
        .any(|component| component.as_os_str().to_string_lossy().ends_with(".app"))
}

#[cfg(debug_assertions)]
fn required_seam() -> bool {
    env::var(REQUIRED_OVERRIDE).as_deref() == Ok("1")
}

#[cfg(not(debug_assertions))]
fn required_seam() -> bool {
    false
}

/// The distributed app (or a test pretending to be one) must ship the pair.
pub fn required() -> bool {
    in_app_bundle() || required_seam()
}

#[cfg(debug_assertions)]
fn override_expected() -> bool {
    env::var_os(CSHARP_LS_OVERRIDE).is_some() || env::var_os(RUNTIME_OVERRIDE).is_some()
}

#[cfg(not(debug_assertions))]
fn override_expected() -> bool {
    false
}

/// True when there is a pair to look for: a shipped app or an explicit debug override. A plain
/// `tauri dev`, Windows or Intel build has nothing to verify.
pub fn pair_expected() -> bool {
    required() || override_expected()
}

/// Resolves the pair once, with the reason when a shipped pair is incomplete.
pub fn availability() -> Availability {
    let tool_override = override_path(CSHARP_LS_OVERRIDE);
    let runtime_override = override_path(RUNTIME_OVERRIDE);
    let resource_root = resources_root();
    let tool_dir = tool_override.or_else(|| resource_root.as_ref().map(|root| root.join(CSHARP_LS_DIR)));
    let runtime_root = runtime_override.or_else(|| resource_root.as_ref().map(|root| root.join(RUNTIME_DIR)));
    // An explicit override is a claim that a pair exists there; a broken one is damage.
    let expects_pair = pair_expected();

    let mut missing: Vec<&str> = Vec::new();
    let tool = match tool_dir.as_deref() {
        Some(dir) if tool_complete(dir) => Some(dir),
        Some(_) => {
            missing.push(CSHARP_LS_DIR);
            None
        }
        None => {
            missing.push(CSHARP_LS_DIR);
            None
        }
    };
    let runtime = match runtime_root.as_deref().map(inspect_runtime) {
        Some(Some(runtime)) => Some(runtime),
        _ => {
            missing.push("private .NET runtime");
            None
        }
    };
    match (tool, runtime) {
        (Some(dir), Some(runtime)) => Availability::Bundled(Analysis {
            tool_dir: dir.to_path_buf(),
            entry: dir.join(CSHARP_LS_ENTRY),
            runtime,
        }),
        _ if expects_pair => Availability::Damaged(format!(
            "The app bundle is missing its {}; {REINSTALL_GUIDANCE}",
            missing.join(" and ")
        )),
        _ => Availability::Absent,
    }
}

fn tool_complete(dir: &Path) -> bool {
    dir.join(CSHARP_LS_ENTRY).is_file() && dir.join("CSharpLanguageServer.deps.json").is_file()
}

/// The bundled csharp-ls payload directory; only when the entry point is actually there.
pub fn csharp_ls_dir() -> Option<PathBuf> {
    let dir = match override_path(CSHARP_LS_OVERRIDE) {
        Some(path) => path,
        None => resources_root()?.join(CSHARP_LS_DIR),
    };
    dir.join(CSHARP_LS_ENTRY).is_file().then_some(dir)
}

/// The bundled private .NET runtime, validated by layout and reported by its 10.x version.
pub fn analysis_runtime() -> Option<Runtime> {
    let root = match override_path(RUNTIME_OVERRIDE) {
        Some(path) => path,
        None => resources_root()?.join(RUNTIME_DIR),
    };
    inspect_runtime(&root)
}

fn inspect_runtime(root: &Path) -> Option<Runtime> {
    if !muxer(root).is_file() {
        return None;
    }
    // Both the host resolver and the app runtime must be 10.x; a partial copy is not a bundle.
    max_major(&root.join("host/fxr"), 10)?;
    let version = max_major(&root.join("shared/Microsoft.NETCore.App"), 10)?;
    Some(Runtime {
        root: root.to_path_buf(),
        version,
    })
}

/// The muxer inside a runtime root (`dotnet` on macOS; the Windows name is kept for the
/// layout check in tests even though the bundle itself is macOS-only).
pub fn muxer(root: &Path) -> PathBuf {
    root.join(if cfg!(windows) {
        "dotnet.exe"
    } else {
        "dotnet"
    })
}

fn max_major(directory: &Path, major: u64) -> Option<String> {
    let mut best: Option<(Version, String)> = None;
    for entry in fs::read_dir(directory).ok()?.flatten() {
        if !entry.path().is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(version) = Version::parse(&name) else {
            continue;
        };
        if version.major != major || version.prerelease.is_some() {
            continue;
        }
        if best
            .as_ref()
            .map(|(current, _)| &version > current)
            .unwrap_or(true)
        {
            best = Some((version, name));
        }
    }
    best.map(|(_, name)| name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime(root: &Path, fxr: &str, app: &str) {
        fs::create_dir_all(root.join("host/fxr").join(fxr)).unwrap();
        fs::create_dir_all(root.join("shared/Microsoft.NETCore.App").join(app)).unwrap();
        fs::write(muxer(root), "").unwrap();
    }

    #[test]
    fn the_highest_10x_app_runtime_names_the_version() {
        let temp = tempfile::tempdir().unwrap();
        runtime(temp.path(), "10.0.9", "10.0.9");
        fs::create_dir_all(temp.path().join("shared/Microsoft.NETCore.App/10.0.14")).unwrap();
        fs::create_dir_all(temp.path().join("shared/Microsoft.NETCore.App/11.0.0")).unwrap();
        let inspected = inspect_runtime(temp.path()).expect("a 10.x runtime is present");
        assert_eq!(inspected.version, "10.0.14");
    }

    #[test]
    fn a_partial_or_non_10_runtime_is_not_a_bundle() {
        let temp = tempfile::tempdir().unwrap();
        assert!(inspect_runtime(temp.path()).is_none(), "no muxer");
        runtime(temp.path(), "10.0.14", "10.0.14");
        fs::remove_dir_all(temp.path().join("host/fxr/10.0.14")).unwrap();
        assert!(inspect_runtime(temp.path()).is_none(), "no host/fxr 10.x");
        let temp = tempfile::tempdir().unwrap();
        runtime(temp.path(), "10.0.14", "8.0.15");
        assert!(
            inspect_runtime(temp.path()).is_none(),
            "no app runtime 10.x"
        );
    }

    #[test]
    #[ignore = "isolated leaf: it writes OPENRIDR_* for its own store, so it runs in its own process"]
    fn the_override_requires_the_entry_dll_and_a_runtime_isolated() {
        let temp = tempfile::tempdir().unwrap();
        let tools = temp.path().join("csharp-ls");
        fs::create_dir_all(&tools).unwrap();
        let _tool = crate::test_harness::EnvVar::set(CSHARP_LS_OVERRIDE, &tools);
        let _runtime = crate::test_harness::EnvVar::set(RUNTIME_OVERRIDE, temp.path().join("dotnet"));
        assert!(
            csharp_ls_dir().is_none(),
            "an empty directory is not a payload"
        );
        fs::write(tools.join(CSHARP_LS_ENTRY), b"asm").unwrap();
        fs::write(tools.join("CSharpLanguageServer.deps.json"), b"{}").unwrap();
        assert_eq!(csharp_ls_dir().as_deref(), Some(tools.as_path()));
        assert!(
            analysis().is_none(),
            "a payload without a runtime is not a complete pair"
        );
    }
}

#[test]
fn the_override_requires_the_entry_dll_and_a_runtime() {
    crate::test_harness::run_isolated_leaf("bundled::tests::the_override_requires_the_entry_dll_and_a_runtime_isolated");
}
