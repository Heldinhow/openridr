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
    let tool_dir = csharp_ls_dir()?;
    let entry = tool_dir.join(CSHARP_LS_ENTRY);
    let runtime = analysis_runtime()?;
    Some(Analysis {
        tool_dir,
        entry,
        runtime,
    })
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

    struct Override {
        key: &'static str,
        old: Option<std::ffi::OsString>,
    }

    impl Override {
        fn set(key: &'static str, value: impl AsRef<std::ffi::OsStr>) -> Self {
            let old = env::var_os(key);
            env::set_var(key, value);
            Self { key, old }
        }
    }

    impl Drop for Override {
        fn drop(&mut self) {
            match &self.old {
                Some(value) => env::set_var(self.key, value),
                None => env::remove_var(self.key),
            }
        }
    }

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
    fn the_override_requires_the_entry_dll_and_a_runtime() {
        let temp = tempfile::tempdir().unwrap();
        let tools = temp.path().join("csharp-ls");
        fs::create_dir_all(&tools).unwrap();
        let _tool = Override::set(CSHARP_LS_OVERRIDE, &tools);
        let _runtime = Override::set(RUNTIME_OVERRIDE, temp.path().join("dotnet"));
        assert!(
            csharp_ls_dir().is_none(),
            "an empty directory is not a payload"
        );
        fs::write(tools.join(CSHARP_LS_ENTRY), b"asm").unwrap();
        assert_eq!(csharp_ls_dir().as_deref(), Some(tools.as_path()));
        assert!(
            analysis().is_none(),
            "a payload without a runtime is not a complete pair"
        );
    }
}
