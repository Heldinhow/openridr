//! Composite DOTNET_ROOT for csharp-ls.
//!
//! The language server itself targets .NET 10 while the project is pinned to SDK 8.0.408 by
//! global.json. DOTNET_ROOT cannot point at either root alone: the 8 root lacks the 10 runtime
//! and the runtime root has no pinned SDK. The host builds a private root of symlinks under the
//! app home (the muxer is a copy because it resolves its own real path) and never edits the
//! SDK/runtime roots or global.json.

use crate::sdk::{parse_list_sdks, Install, Version};
use crate::tools::app_home;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

const MARKER: &str = ".openridr-hostroot.json";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostRoot {
    pub path: String,
    pub rebuilt: bool,
    pub forced: bool,
    pub inputs: Vec<String>,
    pub check: String,
    pub log: Vec<String>,
}

pub fn default_path() -> PathBuf {
    app_home().join("hostroot")
}

/// Test seam: `OPENRIDR_FORCE_HOSTROOT` uses an existing root as-is and skips composition.
#[cfg(debug_assertions)]
pub fn forced_path() -> Option<PathBuf> {
    std::env::var_os("OPENRIDR_FORCE_HOSTROOT").map(PathBuf::from)
}

#[cfg(not(debug_assertions))]
pub fn forced_path() -> Option<PathBuf> {
    None
}

/// Builds the composite root from the .NET 10 runtime root and the resolved project SDK.
pub fn ensure(runtime_root: &Path, project: &Install) -> Result<HostRoot, String> {
    if let Some(path) = forced_path() {
        return inspect_forced(&path, project);
    }

    let path = default_path();
    let (inputs, plan) = describe(runtime_root, project)?;
    if marker_matches(&path, &inputs) {
        let check = check_versions(&path, project)?;
        return Ok(HostRoot {
            path: path.to_string_lossy().to_string(),
            rebuilt: false,
            forced: false,
            inputs,
            check,
            log: vec!["Composite host root reused; inputs are unchanged.".to_string()],
        });
    }

    let build = PathBuf::from(format!("{}.build-{}", path.to_string_lossy(), std::process::id()));
    let _ = fs::remove_dir_all(&build);
    fs::create_dir_all(&build).map_err(|error| {
        format!(
            "Could not create the composite host root at {}: {error}",
            build.to_string_lossy()
        )
    })?;
    let mut log = Vec::new();
    compose(&build, &plan, &mut log)?;
    check_versions(&build, project)?;
    write_marker(&build, &inputs)?;

    let old = if path.exists() {
        let old = PathBuf::from(format!("{}.old-{}", path.to_string_lossy(), now_ms()));
        fs::rename(&path, &old)
            .map_err(|error| format!("Could not move the previous host root aside: {error}"))?;
        Some(old)
    } else {
        None
    };
    if let Err(error) = fs::rename(&build, &path) {
        if let Some(old) = &old {
            let _ = fs::rename(old, &path);
        }
        return Err(format!(
            "Could not activate the composite host root: {error}"
        ));
    }
    if let Some(old) = old {
        let _ = fs::remove_dir_all(old);
    }
    let check = check_versions(&path, project)?;
    log.push(format!(
        "Composite host root activated at {}.",
        path.to_string_lossy()
    ));
    log.push(check.clone());
    Ok(HostRoot {
        path: path.to_string_lossy().to_string(),
        rebuilt: true,
        forced: false,
        inputs,
        check,
        log,
    })
}

fn inspect_forced(path: &Path, project: &Install) -> Result<HostRoot, String> {
    let muxer = path.join(muxer_name());
    if !muxer.is_file() {
        return Err(format!(
            "OPENRIDR_FORCE_HOSTROOT points at {}, which has no .NET muxer.",
            path.to_string_lossy()
        ));
    }
    let output = run_list_sdks(path)?;
    let check = format!("$ {} --list-sdks\n{}", muxer.to_string_lossy(), output.trim());
    if !output.contains(&project.version) {
        return Ok(HostRoot {
            path: path.to_string_lossy().to_string(),
            rebuilt: false,
            forced: true,
            inputs: vec![format!("forced:{}", path.to_string_lossy())],
            check,
            log: vec![format!(
                "Forced host root {} does not list project SDK {}; initialize is expected to fail.",
                path.to_string_lossy(),
                project.version
            )],
        });
    }
    Ok(HostRoot {
        path: path.to_string_lossy().to_string(),
        rebuilt: false,
        forced: true,
        inputs: vec![format!("forced:{}", path.to_string_lossy())],
        check,
        log: vec![format!(
            "Forced host root {} used unchanged.",
            path.to_string_lossy()
        )],
    })
}

#[derive(Debug)]
struct Plan {
    muxer: PathBuf,
    links: Vec<(String, PathBuf)>,
}

fn describe(runtime_root: &Path, project: &Install) -> Result<(Vec<String>, Plan), String> {
    let muxer = runtime_root.join(muxer_name());
    if !muxer.is_file() {
        return Err(format!(
            "The .NET 10 runtime root {} has no .NET muxer.",
            runtime_root.to_string_lossy()
        ));
    }
    if !project.sdk_dir().is_dir() {
        return Err(format!(
            "The resolved project SDK path {} does not exist.",
            project.sdk_dir().to_string_lossy()
        ));
    }
    let host_fxr = pick_runtime(&runtime_root.join("host/fxr"), 10)?
        .ok_or_else(|| "The .NET 10 runtime root has no host/fxr 10.x directory.".to_string())?;
    let host_runtime = pick_runtime(&runtime_root.join("shared/Microsoft.NETCore.App"), 10)?
        .ok_or_else(|| {
            "The .NET 10 runtime root has no Microsoft.NETCore.App 10.x runtime.".to_string()
        })?;
    let worker_runtime = pick_runtime(
        &project.root.join("shared/Microsoft.NETCore.App"),
        8,
    )?
    .or_else(|| {
        pick_runtime(&runtime_root.join("shared/Microsoft.NETCore.App"), 8)
            .ok()
            .flatten()
    });
    let packs = project.root.join("packs");
    if !packs.is_dir() {
        return Err(format!(
            "The project SDK root {} has no packs directory; a partial SDK cannot analyze net8.0.",
            project.root.to_string_lossy()
        ));
    }
    let mut links = vec![
        (
            format!("sdk/{}", project.version),
            project.sdk_dir(),
        ),
        (
            format!(
                "host/fxr/{}",
                host_fxr.file_name().unwrap_or_default().to_string_lossy()
            ),
            host_fxr.clone(),
        ),
        (
            format!(
                "shared/Microsoft.NETCore.App/{}",
                host_runtime
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            ),
            host_runtime.clone(),
        ),
        ("packs".to_string(), packs),
    ];
    if let Some(runtime) = &worker_runtime {
        links.push((
            format!(
                "shared/Microsoft.NETCore.App/{}",
                runtime.file_name().unwrap_or_default().to_string_lossy()
            ),
            runtime.clone(),
        ));
    }
    for name in ["metadata", "sdk-manifests", "templates"] {
        let source = project.root.join(name);
        if source.is_dir() {
            links.push((name.to_string(), source));
        }
    }
    let inputs = vec![
        "layout=project-sdk-only-runtime10".to_string(),
        format!("runtimeRoot={}", runtime_root.to_string_lossy()),
        format!("projectRoot={}", project.root.to_string_lossy()),
        format!("projectVersion={}", project.version),
        format!("projectSdk={}", project.sdk_dir().to_string_lossy()),
        format!(
            "hostFxr={}",
            host_fxr.file_name().unwrap_or_default().to_string_lossy()
        ),
        format!(
            "hostRuntime={}",
            host_runtime
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        ),
        format!(
            "workerRuntime={}",
            worker_runtime
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_else(|| "none".to_string())
        ),
    ];
    Ok((
        inputs,
        Plan {
            muxer,
            links,
        },
    ))
}

fn compose(build: &Path, plan: &Plan, log: &mut Vec<String>) -> Result<(), String> {
    for dir in ["sdk", "host/fxr", "shared/Microsoft.NETCore.App"] {
        fs::create_dir_all(build.join(dir))
            .map_err(|error| format!("Could not create {dir}: {error}"))?;
    }
    let muxer = build.join(muxer_name());
    fs::copy(&plan.muxer, &muxer)
        .map_err(|error| format!("Could not copy the .NET muxer: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&muxer, fs::Permissions::from_mode(0o755));
    }
    log.push(format!(
        "Copied muxer {} -> {} (copy, not symlink).",
        plan.muxer.to_string_lossy(),
        muxer.to_string_lossy()
    ));
    for (relative, target) in &plan.links {
        let link = build.join(relative);
        if let Some(parent) = link.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create {}: {error}", parent.display()))?;
        }
        link_dir(target, &link).map_err(|error| {
            format!(
                "Could not link {} -> {}: {error}",
                link.to_string_lossy(),
                target.to_string_lossy()
            )
        })?;
        log.push(format!(
            "link {} -> {}",
            link.to_string_lossy(),
            target.to_string_lossy()
        ));
    }
    Ok(())
}

fn marker_matches(path: &Path, inputs: &[String]) -> bool {
    if !path.join(muxer_name()).is_file() {
        return false;
    }
    let Ok(text) = fs::read_to_string(path.join(MARKER)) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    let expected: Vec<serde_json::Value> = inputs
        .iter()
        .map(|input| serde_json::Value::String(input.clone()))
        .collect();
    value.get("inputs").and_then(|value| value.as_array()) == Some(&expected)
}

fn write_marker(build: &Path, inputs: &[String]) -> Result<(), String> {
    let value = serde_json::json!({
        "inputs": inputs,
        "createdAt": now_ms(),
    });
    fs::write(build.join(MARKER), value.to_string())
        .map_err(|error| format!("Could not write the host root marker: {error}"))
}

fn check_versions(path: &Path, project: &Install) -> Result<String, String> {
    let sdks = run_list_sdks(path)?;
    let runtimes = run_list_runtimes(path)?;
    let listed: Vec<String> = parse_list_sdks(&sdks)
        .into_iter()
        .map(|(version, _)| version)
        .collect();
    if !listed.iter().any(|version| version == &project.version) {
        return Err(format!(
            "The composite host root does not list project SDK {}: {}",
            project.version,
            sdks.trim()
        ));
    }
    if listed.iter().any(|version| version != &project.version) {
        return Err(format!(
            "The composite host root lists SDKs {:?}; only the pinned project SDK {} may stay in sdk/ so csharp-ls cannot register another MSBuild default.",
            listed, project.version
        ));
    }
    let host_runtime = runtimes.contains("10.");
    if !host_runtime {
        return Err(format!(
            "The composite host root does not expose the .NET 10 runtime csharp-ls needs: {}",
            runtimes.trim()
        ));
    }
    Ok(format!(
        "$ {} --list-sdks\n{}\n$ {} --list-runtimes\n{}",
        path.join(muxer_name()).to_string_lossy(),
        sdks.trim(),
        path.join(muxer_name()).to_string_lossy(),
        runtimes.trim()
    ))
}

fn run_list_runtimes(path: &Path) -> Result<String, String> {
    let muxer = path.join(muxer_name());
    let output = Command::new(&muxer)
        .arg("--list-runtimes")
        .current_dir(std::env::temp_dir())
        .env("DOTNET_ROOT", path)
        .env("DOTNET_MULTILEVEL_LOOKUP", "0")
        .env("DOTNET_CLI_UI_LANGUAGE", "en")
        .env("DOTNET_NOLOGO", "1")
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .env_remove("NUGET_PACKAGES")
        .env_remove("NUGET_HTTP_CACHE_PATH")
        .env_remove("DOTNET_CLI_HOME")
        .output()
        .map_err(|error| {
            format!(
                "Could not run {} --list-runtimes: {error}",
                muxer.to_string_lossy()
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "{} --list-runtimes exited {}",
            muxer.to_string_lossy(),
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn run_list_sdks(path: &Path) -> Result<String, String> {
    let muxer = path.join(muxer_name());
    let output = Command::new(&muxer)
        .arg("--list-sdks")
        .current_dir(std::env::temp_dir())
        .env("DOTNET_ROOT", path)
        .env("DOTNET_MULTILEVEL_LOOKUP", "0")
        .env("DOTNET_CLI_UI_LANGUAGE", "en")
        .env("DOTNET_NOLOGO", "1")
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .env_remove("NUGET_PACKAGES")
        .env_remove("NUGET_HTTP_CACHE_PATH")
        .env_remove("DOTNET_CLI_HOME")
        .output()
        .map_err(|error| {
            format!(
                "Could not run {} --list-sdks: {error}",
                muxer.to_string_lossy()
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "{} --list-sdks exited {}: {}",
            muxer.to_string_lossy(),
            output
                .status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn pick_runtime(root: &Path, major: u64) -> Result<Option<PathBuf>, String> {
    let Ok(entries) = fs::read_dir(root) else {
        return Ok(None);
    };
    let mut best: Option<(Version, PathBuf)> = None;
    for entry in entries.flatten() {
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
            best = Some((version, entry.path()));
        }
    }
    Ok(best.map(|(_, path)| path))
}

fn muxer_name() -> &'static str {
    if cfg!(windows) {
        "dotnet.exe"
    } else {
        "dotnet"
    }
}

/// The muxer inside a composed root; the bundled csharp-ls runs through this one so it sees the
/// project SDK and the private .NET 10 runtime at the same time.
pub fn muxer(root: &Path) -> PathBuf {
    root.join(muxer_name())
}

#[cfg(unix)]
fn link_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

/// Windows directory symlinks need either Developer Mode or an elevated token; this path is
/// implemented but not validated on Windows in this slice.
#[cfg(windows)]
fn link_dir(target: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(target, link)
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
    fn runtime_versions_are_picked_by_number_not_lexically() {
        let temp = tempfile::tempdir().unwrap();
        for name in ["8.0.9", "8.0.15", "10.0.0", "10.0.10", "not-a-version"] {
            fs::create_dir(temp.path().join(name)).unwrap();
        }
        assert!(pick_runtime(temp.path(), 8)
            .unwrap()
            .unwrap()
            .ends_with("8.0.15"));
        assert!(pick_runtime(temp.path(), 10)
            .unwrap()
            .unwrap()
            .ends_with("10.0.10"));
        assert!(pick_runtime(temp.path(), 9).unwrap().is_none());
    }

    fn fake_project(root: &Path, version: &str) -> Install {
        fs::create_dir_all(root.join("sdk").join(version)).unwrap();
        fs::create_dir_all(root.join("packs")).unwrap();
        Install {
            version: version.to_string(),
            root: root.to_path_buf(),
            path: root.join("sdk").to_string_lossy().to_string(),
        }
    }

    #[test]
    fn composite_takes_the_runtime_from_the_runtime_root_and_the_sdk_from_the_project() {
        let temp = tempfile::tempdir().unwrap();
        let runtime = temp.path().join("runtime");
        fs::create_dir_all(runtime.join("host/fxr/10.0.1")).unwrap();
        fs::create_dir_all(runtime.join("shared/Microsoft.NETCore.App/10.0.1")).unwrap();
        fs::write(runtime.join(muxer_name()), "").unwrap();
        let project_root = temp.path().join("project");
        let project = fake_project(&project_root, "8.0.408");
        let project_sdk = project_root.join("sdk/8.0.408");

        let (inputs, plan) = describe(&runtime, &project).unwrap();
        assert_eq!(plan.muxer, runtime.join(muxer_name()));
        assert!(inputs.contains(&format!("runtimeRoot={}", runtime.to_string_lossy())));
        assert!(plan
            .links
            .contains(&("sdk/8.0.408".to_string(), project_sdk)));
        assert!(plan.links.iter().any(|(relative, target)| {
            relative == "host/fxr/10.0.1" && target == &runtime.join("host/fxr/10.0.1")
        }));
        assert!(plan.links.iter().any(|(relative, target)| {
            relative == "shared/Microsoft.NETCore.App/10.0.1"
                && target == &runtime.join("shared/Microsoft.NETCore.App/10.0.1")
        }));
        assert!(plan
            .links
            .contains(&("packs".to_string(), project_root.join("packs"))));
    }

    #[test]
    fn describe_distinguishes_a_missing_runtime_from_a_missing_project_sdk() {
        let temp = tempfile::tempdir().unwrap();
        let runtime = temp.path().join("runtime");
        fs::create_dir_all(runtime.join("host/fxr/10.0.1")).unwrap();
        fs::write(runtime.join(muxer_name()), "").unwrap();
        let project = fake_project(&temp.path().join("project"), "8.0.408");

        let error = describe(&runtime, &project).unwrap_err();
        assert!(error.contains("Microsoft.NETCore.App 10"), "{error}");

        let project_root = temp.path().join("empty-project");
        fs::create_dir_all(&project_root).unwrap();
        let missing = Install {
            version: "8.0.408".to_string(),
            root: project_root.clone(),
            path: project_root.join("sdk").to_string_lossy().to_string(),
        };
        let error = describe(&runtime, &missing).unwrap_err();
        assert!(error.contains("resolved project SDK path"), "{error}");
        assert!(!error.contains("runtime"), "{error}");
    }
}
