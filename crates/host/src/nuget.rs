//! NuGet reads and confirmed updates through the project's resolved SDK.
//!
//! Listing runs `dotnet list <csproj> package` and `--outdated` as separate argv, with the
//! workspace root as cwd and the project SDK's own muxer. Updates only run
//! `dotnet add ... --no-restore`; the explicit restore is a separate user action. Central
//! Package Management is listed but never edited: an update refuses before spawning anything.

use crate::scope::Scope;
use crate::{HostError, HostResult};
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

pub const PACKAGE_LIMIT: usize = 500;
pub const LOG_LIMIT: usize = 32_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Package {
    pub id: String,
    pub requested: Option<String>,
    pub resolved: Option<String>,
    pub latest_version: Option<String>,
    pub outdated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameworkPackages {
    pub framework: String,
    pub packages: Vec<Package>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandResult {
    pub state: &'static str,
    pub message: Option<String>,
    pub command: String,
    pub exit: Option<i32>,
    pub log: String,
    pub frameworks: Vec<FrameworkPackages>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub name: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sources {
    pub state: &'static str,
    pub message: Option<String>,
    pub items: Vec<Source>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cpm {
    pub enabled: bool,
    pub path: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetsPackage {
    pub id: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetsFramework {
    pub framework: String,
    pub implicit: Vec<AssetsPackage>,
    pub transitive: Vec<AssetsPackage>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Assets {
    pub state: &'static str,
    pub message: Option<String>,
    pub path: String,
    pub frameworks: Vec<AssetsFramework>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Listing {
    pub state: &'static str,
    pub message: Option<String>,
    pub project: String,
    pub project_name: String,
    pub target_framework: Option<String>,
    pub sdk_version: String,
    pub sdk_root: String,
    pub cpm: Cpm,
    pub installed: CommandResult,
    pub outdated: CommandResult,
    pub sources: Sources,
    /// Installed packages with the latest version merged in, grouped by TFM.
    pub frameworks: Vec<FrameworkPackages>,
    pub total_packages: usize,
    pub outdated_count: usize,
    pub truncated: bool,
    pub prerelease: bool,
    pub selected_source: Option<String>,
    pub assets: Assets,
    pub probe: bool,
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub id: String,
    pub workspace_id: u64,
    pub context_id: u64,
    pub root: PathBuf,
    pub project: PathBuf,
    pub project_display: String,
    pub package: String,
    pub from_version: Option<String>,
    pub version: String,
    pub target_framework: Option<String>,
    pub muxer: PathBuf,
    pub sdk_root: PathBuf,
    pub install: bool,
    pub probe: bool,
}

pub struct PlanRequest {
    pub id: String,
    pub workspace_id: u64,
    pub context_id: u64,
    pub root: PathBuf,
    pub project: PathBuf,
    pub project_display: String,
    pub package: String,
    pub from_version: Option<String>,
    pub version: String,
    pub target_framework: Option<String>,
    pub muxer: PathBuf,
    pub sdk_root: PathBuf,
    pub install: bool,
    /// Batch plans defer the CPM refusal to `apply`, so one central project cannot block the
    /// whole batch. Single-project plans keep the early refusal and its UI reason.
    pub defer_cpm: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreOffer {
    pub command: String,
    pub argv: Vec<String>,
    pub note: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResult {
    pub state: &'static str,
    pub message: Option<String>,
    pub exit: Option<i32>,
    pub log: String,
    pub command: String,
    pub project: String,
    pub package: String,
    pub from_version: Option<String>,
    pub version: String,
    pub restore: RestoreOffer,
}

struct Exec {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    spawn_error: Option<String>,
}

/// Canonicalizes a project path and rejects anything outside the workspace scope. The containment
/// step is the host's single implementation (`crate::confine_raw`), so an absolute path whose prefix
/// is a symlink or differs in case is accepted, while a symlink that carries the path outside is
/// refused whether or not the project file exists.
pub fn resolve_project(scope: &Scope, raw: &str) -> HostResult<(PathBuf, String)> {
    if raw.trim().is_empty() {
        return Err(HostError::new("path_not_allowed", "Project path is empty."));
    }
    let canonical = crate::confine_raw(scope, raw, "Project not found")?;
    if !canonical.is_file()
        || canonical
            .extension()
            .and_then(|ext| ext.to_str())
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("csproj"))
    {
        return Err(HostError::new(
            "path_not_allowed",
            format!("Not a .csproj file: {raw}"),
        ));
    }
    let display = match scope.root() {
        Some(root) => {
            crate::fsutil::portable_path(canonical.strip_prefix(root).unwrap_or(&canonical))
        }
        None => crate::fsutil::portable_path(&canonical),
    };
    Ok((canonical, display))
}

pub fn project_name(path: &Path) -> String {
    path.file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Central Package Management is enabled by the nearest Directory.Packages.props unless that
/// file explicitly turns the property off. The MVP never edits central versions.
///
/// The walk stops at the project's scope directory: the workspace root for a project inside it, the
/// project's own directory for a project the solution names elsewhere. A props file above that
/// directory is outside the workspace scope and is never read.
pub fn detect_cpm(root: &Path, project: &Path) -> Cpm {
    let project_dir = project.parent();
    // The walk stops at the project's scope directory: the workspace root for a project inside it,
    // the project's own directory for a project the solution names elsewhere.
    let entry = match project_dir {
        Some(dir) if !dir.starts_with(root) => dir,
        _ => root,
    };
    let mut dir = project_dir;
    while let Some(current) = dir {
        if !current.starts_with(entry) {
            break;
        }
        let candidate = current.join("Directory.Packages.props");
        if candidate.is_file() {
            let text = fs::read_to_string(&candidate).unwrap_or_default();
            if central_property(&text) == Some(false) {
                return Cpm {
                    enabled: false,
                    path: None,
                    message: None,
                };
            }
            let rel =
                crate::fsutil::portable_path(candidate.strip_prefix(root).unwrap_or(&candidate));
            return Cpm {
                enabled: true,
                path: Some(rel.clone()),
                message: Some(format!(
                    "Central Package Management is enabled by {}; the MVP lists packages but refuses updates because it does not edit central versions.",
                    rel
                )),
            };
        }
        if current == entry {
            break;
        }
        dir = current.parent();
    }
    Cpm {
        enabled: false,
        path: None,
        message: None,
    }
}

fn central_property(text: &str) -> Option<bool> {
    let lower = text.to_ascii_lowercase();
    let open = lower.find("<managepackageversionscentrally>")?;
    let rest = &lower[open + "<managepackageversionscentrally>".len()..];
    let end = rest.find('<')?;
    Some(rest[..end].trim() == "true")
}

pub fn validate_package_id(id: &str) -> HostResult<()> {
    let valid = !id.is_empty()
        && id.len() <= 200
        && id
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_alphanumeric())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if !valid {
        return Err(HostError::new(
            "invalid_state",
            "A package id must start with a letter or digit and contain only letters, digits, '.', '-' or '_'; no command was run.",
        ));
    }
    Ok(())
}

pub fn validate_version(version: &str) -> HostResult<()> {
    let valid = !version.is_empty()
        && version.len() <= 80
        && version
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_alphanumeric())
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'));
    if !valid {
        return Err(HostError::new(
            "invalid_state",
            "A package version must start with a letter or digit and contain only letters, digits, '.', '-', '+' or '_'; no command was run.",
        ));
    }
    Ok(())
}

pub fn validate_tfm(tfm: &str) -> HostResult<()> {
    let valid = !tfm.is_empty()
        && tfm.len() <= 80
        && tfm
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if !valid {
        return Err(HostError::new(
            "invalid_state",
            "A target framework must contain only letters, digits, '.', '-' or '_'; no command was run.",
        ));
    }
    Ok(())
}

pub fn plan_id(seq: u64, context_id: u64, project: &str, package: &str, version: &str) -> String {
    format!(
        "nuget-{context_id}-{seq}-{:016x}",
        fnv1a(&format!("{project}|{package}|{version}"))
    )
}

fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn display(argv: &[String]) -> String {
    argv.iter()
        .map(|arg| {
            if arg.chars().any(char::is_whitespace) {
                format!("\"{arg}\"")
            } else {
                arg.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn add_argv(plan: &Plan) -> Vec<String> {
    vec![
        plan.muxer.to_string_lossy().into_owned(),
        "add".to_string(),
        plan.project_display.clone(),
        "package".to_string(),
        plan.package.clone(),
        "--version".to_string(),
        plan.version.clone(),
        "--no-restore".to_string(),
    ]
}

fn restore_argv(plan: &Plan) -> Vec<String> {
    vec![
        plan.muxer.to_string_lossy().into_owned(),
        "restore".to_string(),
        plan.project_display.clone(),
        "-nologo".to_string(),
        "-v:minimal".to_string(),
    ]
}

pub fn make_plan(request: PlanRequest) -> HostResult<Plan> {
    validate_package_id(&request.package)?;
    validate_version(&request.version)?;
    if let Some(tfm) = &request.target_framework {
        validate_tfm(tfm)?;
    }
    let cpm = detect_cpm(&request.root, &request.project);
    if cpm.enabled && !request.defer_cpm {
        return Err(HostError::with_details(
            "nuget_cpm_unsupported",
            cpm.message.clone().unwrap_or_else(|| {
                "Central Package Management is enabled; the MVP does not edit central versions."
                    .to_string()
            }),
            json!({ "cpm": cpm }),
        ));
    }
    let plan = Plan {
        id: request.id,
        workspace_id: request.workspace_id,
        context_id: request.context_id,
        root: request.root,
        project: request.project,
        project_display: request.project_display,
        package: request.package,
        from_version: request.from_version,
        version: request.version,
        target_framework: request.target_framework,
        muxer: request.muxer,
        sdk_root: request.sdk_root,
        install: request.install,
        probe: false,
    };
    let project_text = fs::read_to_string(&plan.project).map_err(|error| {
        HostError::new(
            "operation_failed",
            format!(
                "Could not read {} before planning the update: {error}",
                plan.project_display
            ),
        )
    })?;
    revalidate_text(&plan, &project_text)?;
    Ok(plan)
}

pub fn plan_view(plan: &Plan) -> Value {
    let argv = add_argv(plan);
    let restore = restore_argv(plan);
    json!({
        "planId": plan.id,
        "project": plan.project_display,
        "projectName": project_name(&plan.project),
        "package": plan.package,
        "fromVersion": plan.from_version,
        "version": plan.version,
        "targetFramework": plan.target_framework,
        "command": display(&argv),
        "argv": argv,
        "install": plan.install,
        "restore": RestoreOffer {
            command: display(&restore),
            argv: restore,
            note: "Restore is offered after the update; OpenRidr never restores implicitly.",
        },
        "probe": plan.probe,
    })
}

/// The plan must still describe the project on disk: same package, same previous version and
/// not already at the target version. This runs again immediately before the update.
pub fn revalidate_text(plan: &Plan, text: &str) -> HostResult<()> {
    let current = package_version(text, &plan.package);
    match current {
        None if plan.install => Ok(()),
        None => Err(HostError::new(
            "nuget_stale_plan",
            format!(
                "{} no longer references {}; the plan is stale and no command was run.",
                plan.project_display, plan.package
            ),
        )),
        Some(version) if version.is_empty() => Ok(()),
        Some(version) if version == plan.version => Err(HostError::new(
            "nuget_stale_plan",
            format!(
                "{} is already at {}; the plan is stale and no command was run.",
                plan.package, plan.version
            ),
        )),
        Some(version) => {
            if let Some(expected) = &plan.from_version {
                if &version != expected {
                    return Err(HostError::new(
                        "nuget_stale_plan",
                        format!(
                            "{} is now at {version}, not {expected}; the plan is stale and no command was run.",
                            plan.package
                        ),
                    ));
                }
            }
            Ok(())
        }
    }
}

pub fn apply(plan: &Plan) -> HostResult<UpdateResult> {
    let cpm = detect_cpm(&plan.root, &plan.project);
    if cpm.enabled {
        return Err(HostError::with_details(
            "nuget_cpm_unsupported",
            cpm.message.clone().unwrap_or_else(|| {
                "Central Package Management is enabled; the MVP does not edit central versions."
                    .to_string()
            }),
            json!({ "cpm": cpm }),
        ));
    }
    let text = fs::read_to_string(&plan.project).map_err(|error| {
        HostError::new(
            "operation_failed",
            format!(
                "Could not read {} before updating: {error}",
                plan.project_display
            ),
        )
    })?;
    revalidate_text(plan, &text)?;

    let argv = add_argv(plan);
    let command = display(&argv);
    let exec = run(&argv, &plan.root, &plan.sdk_root);
    let restore = restore_argv(plan);
    let (log, _) = cap_log(&format!(
        "$ {command}\nexit {}\n--- stdout\n{}\n--- stderr\n{}",
        exec.code
            .map(|code| code.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        exec.stdout.trim_end(),
        exec.stderr.trim_end()
    ));
    let restore = RestoreOffer {
        command: display(&restore),
        argv: restore,
        note: "Restore is offered after the update; OpenRidr never restores implicitly.",
    };
    let (state, message) = if let Some(error) = exec.spawn_error {
        ("failed", Some(format!("Could not run dotnet add: {error}")))
    } else if exec.code != Some(0) {
        (
            "failed",
            Some(first_line(&exec.stderr).unwrap_or_else(|| {
                format!(
                    "dotnet add exited with {}.",
                    exec.code
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| "an unknown status".to_string())
                )
            })),
        )
    } else {
        ("ok", None)
    };
    Ok(UpdateResult {
        state,
        message,
        exit: exec.code,
        log,
        command,
        project: plan.project_display.clone(),
        package: plan.package.clone(),
        from_version: plan.from_version.clone(),
        version: plan.version.clone(),
        restore,
    })
}

/// Reads the literal `Version`/`VersionOverride` on the project's PackageReference. A version
/// expressed as an MSBuild property returns `None`; absence of the package is distinguishable.
pub fn package_version(text: &str, package: &str) -> Option<String> {
    let mut found = None;
    for chunk in text.split('<') {
        if chunk.is_empty() {
            continue;
        }
        let head: String = chunk.chars().take(16).collect();
        if !head.to_ascii_lowercase().starts_with("packagereference") {
            continue;
        }
        let Some(end) = chunk.find('>') else {
            continue;
        };
        let body = &chunk[..end];
        let Some(name) = attribute(body, "include").or_else(|| attribute(body, "update")) else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case(package) {
            continue;
        }
        let version = attribute(body, "version").or_else(|| attribute(body, "versionoverride"));
        if let Some(version) = version {
            let version = version.trim().to_string();
            if !version.contains("$(") {
                found = Some(version);
            }
        }
        return found.or(Some(String::new()));
    }
    None
}

fn attribute(body: &str, name: &str) -> Option<String> {
    let lower = body.to_ascii_lowercase();
    let needle = format!("{name}=");
    let mut from = 0;
    while let Some(offset) = lower[from..].find(&needle) {
        let start = from + offset + needle.len();
        let rest = body[start..].trim_start();
        let quote = rest.chars().next()?;
        if quote == '"' || quote == '\'' {
            let rest = &rest[quote.len_utf8()..];
            if let Some(end) = rest.find(quote) {
                return Some(rest[..end].to_string());
            }
        }
        from = start;
    }
    None
}

pub fn parse_listing(stdout: &str) -> Result<Vec<FrameworkPackages>, String> {
    let value: Value =
        serde_json::from_str(stdout).map_err(|error| format!("invalid JSON: {error}"))?;
    let projects = value
        .get("projects")
        .and_then(Value::as_array)
        .ok_or_else(|| "the response has no projects array".to_string())?;
    if projects.is_empty() {
        return Err("the response lists no projects".to_string());
    }
    let mut frameworks: Vec<FrameworkPackages> = Vec::new();
    let mut omitted_frameworks = false;
    for project in projects {
        let Some(raw_list) = project.get("frameworks") else {
            // dotnet omits this field when a valid project has no PackageReference entries.
            omitted_frameworks = true;
            continue;
        };
        let list = raw_list
            .as_array()
            .ok_or_else(|| "a project's frameworks field is not an array".to_string())?;
        for framework in list {
            let name = framework
                .get("framework")
                .and_then(Value::as_str)
                .filter(|name| !name.trim().is_empty())
                .unwrap_or("unknown")
                .to_string();
            let target = match frameworks.iter_mut().find(|item| item.framework == name) {
                Some(item) => item,
                None => {
                    frameworks.push(FrameworkPackages {
                        framework: name.clone(),
                        packages: Vec::new(),
                    });
                    frameworks.last_mut().expect("just pushed")
                }
            };
            let Some(packages) = framework.get("topLevelPackages").and_then(Value::as_array) else {
                continue;
            };
            for package in packages {
                let Some(id) = package
                    .get("id")
                    .and_then(Value::as_str)
                    .filter(|id| !id.trim().is_empty())
                else {
                    continue;
                };
                // Older fixtures spell these without "Version"; keep both readable.
                let requested = text_field(package, "requested")
                    .or_else(|| text_field(package, "requestedVersion"));
                let resolved = text_field(package, "resolved")
                    .or_else(|| text_field(package, "resolvedVersion"));
                let latest = text_field(package, "latestVersion");
                if let Some(existing) = target
                    .packages
                    .iter_mut()
                    .find(|item| item.id.eq_ignore_ascii_case(id))
                {
                    if existing.requested.is_none() {
                        existing.requested = requested;
                    }
                    if existing.resolved.is_none() {
                        existing.resolved = resolved;
                    }
                    if existing.latest_version.is_none() {
                        existing.latest_version = latest;
                    }
                    continue;
                }
                target.packages.push(Package {
                    id: id.to_string(),
                    requested,
                    resolved,
                    latest_version: latest,
                    outdated: false,
                });
            }
        }
    }
    if frameworks.is_empty() && !omitted_frameworks {
        return Err("the response lists no frameworks".to_string());
    }
    for framework in &mut frameworks {
        framework
            .packages
            .sort_by_key(|left| left.id.to_lowercase());
    }
    Ok(frameworks)
}

fn text_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

pub fn parse_sources(stdout: &str) -> Result<Vec<Source>, String> {
    let mut sources = Vec::new();
    for line in stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let (enabled, value) = if let Some(value) = line.strip_prefix("E ") {
            (true, value.trim())
        } else if let Some(value) = line.strip_prefix("D ") {
            (false, value.trim())
        } else {
            return Err("unrecognized short source listing".to_string());
        };
        if value.is_empty() {
            return Err("source location is empty".to_string());
        }
        if enabled {
            sources.push(Source {
                name: value.to_string(),
                url: None,
            });
        }
    }
    Ok(sources)
}

/// Merges installed packages with the outdated report so each row carries the latest version.
pub fn merge(
    installed: &[FrameworkPackages],
    outdated: &[FrameworkPackages],
) -> Vec<FrameworkPackages> {
    let mut merged: Vec<FrameworkPackages> = installed
        .iter()
        .map(|framework| FrameworkPackages {
            framework: framework.framework.clone(),
            packages: framework.packages.clone(),
        })
        .collect();
    for framework in outdated {
        let target = match merged
            .iter_mut()
            .find(|item| item.framework == framework.framework)
        {
            Some(item) => item,
            None => {
                merged.push(FrameworkPackages {
                    framework: framework.framework.clone(),
                    packages: Vec::new(),
                });
                merged.last_mut().expect("just pushed")
            }
        };
        for package in &framework.packages {
            if let Some(existing) = target
                .packages
                .iter_mut()
                .find(|item| item.id.eq_ignore_ascii_case(&package.id))
            {
                if package.latest_version.is_some() {
                    existing.latest_version = package.latest_version.clone();
                }
                if existing.resolved.is_none() {
                    existing.resolved = package.resolved.clone();
                }
                if existing.requested.is_none() {
                    existing.requested = package.requested.clone();
                }
            } else {
                target.packages.push(package.clone());
            }
        }
    }
    for framework in &mut merged {
        for package in &mut framework.packages {
            package.outdated = package
                .latest_version
                .as_ref()
                .is_some_and(|latest| Some(latest) != package.resolved.as_ref());
        }
        framework
            .packages
            .sort_by_key(|left| left.id.to_lowercase());
    }
    merged
}

/// Assets target keys use the long TFI form for older frameworks
/// (`.NETStandard,Version=v2.0`); the UI and `dotnet list` use the short TFM.
fn assets_target_tfm(target: &str) -> String {
    let head = target.split('/').next().unwrap_or(target);
    if let Some(version) = head.strip_prefix(".NETStandard,Version=v") {
        return format!("netstandard{version}");
    }
    if let Some(version) = head.strip_prefix(".NETCoreApp,Version=v") {
        if version.starts_with('5') {
            return format!("net{version}");
        }
        return format!("net{}", version.split('.').next().unwrap_or(version));
    }
    if let Some(version) = head.strip_prefix(".NETPlatform,Version=v") {
        return format!("net{version}");
    }
    if let Some(version) = head.strip_prefix(".NETFramework,Version=v") {
        return format!("net{}", version.replace('.', ""));
    }
    head.to_string()
}

fn inherited_base_intermediate(project: &Path) -> bool {
    let mut current = project.parent();
    while let Some(directory) = current {
        for name in ["Directory.Build.props", "Directory.Build.targets"] {
            let path = directory.join(name);
            if let Ok(text) = fs::read_to_string(&path) {
                if text.contains("<BaseIntermediateOutputPath") {
                    return true;
                }
            }
        }
        current = directory.parent();
    }
    false
}

/// Reads the restore graph that MSBuild already wrote. It never restores: a missing or
/// unreadable assets file is reported as unavailable, not guessed from the project file.
///
/// Implicit packages come from `project.frameworks.<tfm>.dependencies` entries with
/// `autoReferenced: true` and from `downloadDependencies`; `targets` entries only provide the
/// transitive set. A RID-specific target (`net8.0/osx-arm64`) is folded into its base TFM so
/// the same framework is never listed twice. Only the default `obj/project.assets.json` is
/// read; a custom `BaseIntermediateOutputPath` is a documented limitation.
pub fn read_assets(project: &Path, target_framework: Option<&str>) -> Assets {
    let project_text = fs::read_to_string(project).unwrap_or_default();
    let property = xml_property(&project_text, "BaseIntermediateOutputPath");
    let inherited = inherited_base_intermediate(project);
    let expanded = property.as_deref().is_some_and(|value| value.contains('$'));
    if inherited || expanded {
        return Assets {
            state: "unsupported",
            message: Some(
                "Implicit and transitive packages are unavailable: BaseIntermediateOutputPath is inherited or expanded by MSBuild (Directory.Build.props/targets or a property), so obj/project.assets.json is outside the default path."
                    .to_string(),
            ),
            path: project
                .parent()
                .map(|parent| parent.join("obj/project.assets.json"))
                .unwrap_or_else(|| project.join("obj/project.assets.json"))
                .to_string_lossy()
                .to_string(),
            frameworks: Vec::new(),
        };
    }
    let intermediate = property
        .as_deref()
        .map(|value| {
            project
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(value)
        })
        .unwrap_or_else(|| {
            project
                .parent()
                .map(|parent| parent.join("obj"))
                .unwrap_or_else(|| project.join("obj"))
        });
    let path = intermediate.join("project.assets.json");
    let display = path.to_string_lossy().to_string();
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            return Assets {
                state: "missing",
                message: Some(format!(
                    "obj/project.assets.json is not readable ({error}); restore the project to list implicit and transitive packages."
                )),
                path: display,
                frameworks: Vec::new(),
            }
        }
    };
    let value: Value = match serde_json::from_str(&text) {
        Ok(value) => value,
        Err(error) => {
            return Assets {
                state: "failed",
                message: Some(format!(
                    "obj/project.assets.json is not valid JSON: {error}"
                )),
                path: display,
                frameworks: Vec::new(),
            }
        }
    };

    // direct[base][id] = autoReferenced; implicit[base] holds the SDK/auto packages.
    let mut direct: std::collections::HashMap<String, std::collections::HashMap<String, bool>> =
        std::collections::HashMap::new();
    let mut implicit: std::collections::HashMap<
        String,
        std::collections::HashMap<String, AssetsPackage>,
    > = std::collections::HashMap::new();
    if let Some(frameworks) = value
        .get("project")
        .and_then(|project| project.get("frameworks"))
        .and_then(Value::as_object)
    {
        for (tfm, framework) in frameworks {
            let base = tfm.split('/').next().unwrap_or(tfm).to_string();
            if target_framework.is_some_and(|selected| selected != base) {
                continue;
            }
            let direct_for = direct.entry(base.clone()).or_default();
            let implicit_for = implicit.entry(base.clone()).or_default();
            if let Some(dependencies) = framework.get("dependencies").and_then(Value::as_object) {
                for (id, dependency) in dependencies {
                    let auto = dependency
                        .get("autoReferenced")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    direct_for.insert(id.to_ascii_lowercase(), auto);
                    if auto {
                        implicit_for.insert(
                            id.to_ascii_lowercase(),
                            AssetsPackage {
                                id: id.clone(),
                                version: dependency
                                    .get("version")
                                    .and_then(Value::as_str)
                                    .map(str::to_string),
                            },
                        );
                    }
                }
            }
            if let Some(downloads) = framework
                .get("downloadDependencies")
                .and_then(Value::as_array)
            {
                for download in downloads {
                    let Some(name) = download.get("name").and_then(Value::as_str) else {
                        continue;
                    };
                    implicit_for.insert(
                        name.to_ascii_lowercase(),
                        AssetsPackage {
                            id: name.to_string(),
                            version: download
                                .get("version")
                                .and_then(Value::as_str)
                                .map(str::to_string),
                        },
                    );
                }
            }
        }
    }
    if let Some(downloads) = value
        .get("project")
        .and_then(|project| project.get("downloadDependencies"))
        .and_then(Value::as_array)
    {
        for download in downloads {
            let Some(name) = download.get("name").and_then(Value::as_str) else {
                continue;
            };
            let package = AssetsPackage {
                id: name.to_string(),
                version: download
                    .get("version")
                    .and_then(Value::as_str)
                    .map(str::to_string),
            };
            for implicit_for in implicit.values_mut() {
                implicit_for.insert(name.to_ascii_lowercase(), package.clone());
            }
        }
    }

    let mut frameworks: std::collections::BTreeMap<String, AssetsFramework> =
        std::collections::BTreeMap::new();
    if let Some(targets) = value.get("targets").and_then(Value::as_object) {
        for (target, packages) in targets {
            let framework = assets_target_tfm(target);
            if target_framework.is_some_and(|selected| selected != framework) {
                continue;
            }
            let entry = frameworks
                .entry(framework.clone())
                .or_insert_with(|| AssetsFramework {
                    framework: framework.clone(),
                    implicit: Vec::new(),
                    transitive: Vec::new(),
                });
            if let Some(packages) = packages.as_object() {
                for (key, package) in packages {
                    let kind = package
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or("package");
                    if kind != "package" {
                        continue;
                    }
                    let id = key.split('/').next().unwrap_or(key).to_string();
                    let version = key.split_once('/').map(|(_, version)| version.to_string());
                    let lower = id.to_ascii_lowercase();
                    if direct
                        .get(&framework)
                        .is_some_and(|direct| direct.contains_key(&lower))
                        || implicit
                            .get(&framework)
                            .is_some_and(|implicit| implicit.contains_key(&lower))
                    {
                        continue;
                    }
                    if !entry
                        .transitive
                        .iter()
                        .any(|package| package.id.eq_ignore_ascii_case(&id))
                    {
                        entry.transitive.push(AssetsPackage { id, version });
                    }
                }
            }
        }
    }
    for (tfm, framework) in &mut frameworks.iter_mut() {
        if let Some(implicit_for) = implicit.get(tfm) {
            framework.implicit = implicit_for.values().cloned().collect();
        }
        framework
            .implicit
            .sort_by_key(|left| left.id.to_lowercase());
        framework
            .transitive
            .sort_by_key(|left| left.id.to_lowercase());
    }

    let frameworks: Vec<AssetsFramework> = frameworks.into_values().collect();
    if frameworks.is_empty() {
        Assets {
            state: "missing",
            message: Some(
                "obj/project.assets.json has no package targets for this project/TFM.".to_string(),
            ),
            path: display,
            frameworks,
        }
    } else {
        Assets {
            state: "ok",
            message: None,
            path: display,
            frameworks,
        }
    }
}

fn xml_property(text: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let start = text.find(&open)? + open.len();
    let end = text[start..].find(&close)? + start;
    Some(text[start..end].trim().to_string())
}

// The NuGet plan is threaded as plain values on purpose: every argument is already named at the one
// call site, and grouping them would only move the same list into a struct.
#[allow(clippy::too_many_arguments)]
pub fn list(
    root: &Path,
    project: &Path,
    project_display: &str,
    target_framework: Option<&str>,
    muxer: &Path,
    sdk_root: &Path,
    sdk_version: &str,
    prerelease: bool,
    selected_source: Option<&str>,
) -> Listing {
    let mut base = vec![
        muxer.to_string_lossy().into_owned(),
        "list".to_string(),
        project_display.to_string(),
        "package".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    if let Some(tfm) = target_framework {
        base.push("--framework".to_string());
        base.push(tfm.to_string());
    }
    let installed_argv = base.clone();
    let mut outdated_argv = base.clone();
    outdated_argv.insert(4, "--outdated".to_string());
    if prerelease {
        outdated_argv.push("--include-prerelease".to_string());
    }
    if let Some(source) = selected_source {
        outdated_argv.push("--source".to_string());
        outdated_argv.push(source.to_string());
    }
    let sources = configured_sources(root, muxer, sdk_root);
    if let Some(source) = selected_source {
        let valid = sources.state == "ok"
            && source_matches(source, &sources.items)
            && validate_source_arg(source).is_ok();
        if !valid {
            return Listing {
                state: "failed",
                message: Some(
                    "The selected NuGet source is not configured for this project or is not a public HTTPS source; no package command was run.".to_string(),
                ),
                project: project_display.to_string(),
                project_name: project_name(project),
                target_framework: target_framework.map(str::to_string),
                sdk_version: sdk_version.to_string(),
                sdk_root: sdk_root.to_string_lossy().into_owned(),
                cpm: detect_cpm(root, project),
                installed: CommandResult {
                    state: "failed",
                    message: None,
                    command: String::new(),
                    exit: None,
                    log: String::new(),
                    frameworks: Vec::new(),
                    truncated: false,
                },
                outdated: CommandResult {
                    state: "failed",
                    message: None,
                    command: String::new(),
                    exit: None,
                    log: String::new(),
                    frameworks: Vec::new(),
                    truncated: false,
                },
                sources,
                frameworks: Vec::new(),
                total_packages: 0,
                outdated_count: 0,
                truncated: false,
                prerelease,
                selected_source: Some(source.to_string()),
                assets: read_assets(project, target_framework),
                probe: false,
            };
        }
    }
    let no_references = !declares_package_reference(project);
    let installed = command_result(
        run(&installed_argv, root, sdk_root),
        &installed_argv,
        parse_listing,
        "dotnet list package",
        no_references,
    );
    let outdated = command_result(
        run(&outdated_argv, root, sdk_root),
        &outdated_argv,
        parse_listing,
        "dotnet list package --outdated",
        no_references,
    );
    let cpm = detect_cpm(root, project);
    let (mut frameworks, merged_truncated) =
        cap(merge(&installed.frameworks, &outdated.frameworks));
    if let Some(tfm) = target_framework {
        frameworks.retain(|framework| framework.framework == tfm);
    }
    let total_packages = frameworks.iter().map(|item| item.packages.len()).sum();
    let outdated_count = frameworks
        .iter()
        .flat_map(|item| &item.packages)
        .filter(|package| package.outdated)
        .count();
    let state = match (installed.state, outdated.state) {
        ("ok", "ok") => "ok",
        ("failed", "failed") => "failed",
        _ => "partial",
    };
    let message = match state {
        "ok" => None,
        "partial" => Some(
            installed
                .message
                .clone()
                .or_else(|| outdated.message.clone())
                .unwrap_or_else(|| "Only part of the NuGet data was read.".to_string()),
        ),
        _ => Some(
            installed
                .message
                .clone()
                .or_else(|| outdated.message.clone())
                .unwrap_or_else(|| "NuGet could not list packages.".to_string()),
        ),
    };

    Listing {
        state,
        message,
        project: project_display.to_string(),
        project_name: project_name(project),
        target_framework: target_framework.map(str::to_string),
        sdk_version: sdk_version.to_string(),
        sdk_root: sdk_root.to_string_lossy().into_owned(),
        cpm,
        installed,
        outdated,
        sources,
        frameworks,
        total_packages,
        outdated_count,
        truncated: merged_truncated,
        prerelease,
        selected_source: selected_source.map(str::to_string),
        assets: read_assets(project, target_framework),
        probe: false,
    }
}

/// A project file without a `PackageReference` has no packages to list. dotnet still fails
/// on it when it was never restored, so that one problem is not an error for such projects.
/// Any other failure, or a project that references packages, keeps the real error.
fn command_result(
    exec: Exec,
    argv: &[String],
    parse: fn(&str) -> Result<Vec<FrameworkPackages>, String>,
    label: &str,
    no_references: bool,
) -> CommandResult {
    let command = display(argv);
    let (log, _) = cap_log(&format!(
        "$ {command}\nexit {}\n--- stdout\n{}\n--- stderr\n{}",
        exec.code
            .map(|code| code.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        exec.stdout.trim_end(),
        exec.stderr.trim_end()
    ));
    if let Some(error) = exec.spawn_error {
        return CommandResult {
            state: "failed",
            message: Some(format!("Could not run {label}: {error}")),
            command,
            exit: exec.code,
            log,
            frameworks: Vec::new(),
            truncated: false,
        };
    }
    if exec.code != Some(0) && !(no_references && only_missing_assets(&exec.stdout)) {
        return CommandResult {
            state: "failed",
            message: Some(first_line(&exec.stderr).unwrap_or_else(|| {
                format!(
                    "{label} exited with {}.",
                    exec.code
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| "an unknown status".to_string())
                )
            })),
            command,
            exit: exec.code,
            log,
            frameworks: Vec::new(),
            truncated: false,
        };
    }
    if exec.code != Some(0) {
        return CommandResult {
            state: "ok",
            message: None,
            command,
            exit: exec.code,
            log,
            frameworks: Vec::new(),
            truncated: false,
        };
    }
    match parse(&exec.stdout) {
        Ok(frameworks) => {
            let (frameworks, truncated) = cap(frameworks);
            CommandResult {
                state: "ok",
                message: None,
                command,
                exit: exec.code,
                log,
                frameworks,
                truncated,
            }
        }
        Err(error) => CommandResult {
            state: "failed",
            message: Some(format!("{label} returned invalid data: {error}")),
            command,
            exit: exec.code,
            log,
            frameworks: Vec::new(),
            truncated: false,
        },
    }
}

/// True when `dotnet list --format json` reports only the missing-assets problem.
fn only_missing_assets(stdout: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(stdout) else {
        return false;
    };
    value
        .get("problems")
        .and_then(Value::as_array)
        .is_some_and(|problems| {
            !problems.is_empty()
                && problems.iter().all(|problem| {
                    problem
                        .get("text")
                        .and_then(Value::as_str)
                        .is_some_and(|text| text.starts_with("No assets file was found"))
                })
        })
}

/// Unreadable project files count as referencing packages, so their failures stay visible.
// ponytail: plain text match on the project file; Directory.Build.props references are not seen.
fn declares_package_reference(project: &Path) -> bool {
    fs::read_to_string(project).map_or(true, |text| text.contains("PackageReference"))
}

fn url_host(url: &str) -> Option<&str> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    if rest.contains('@') {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next()?;
    let host = authority
        .strip_prefix('[')
        .and_then(|value| value.split(']').next())
        .or_else(|| authority.split(':').next())?;
    (!host.is_empty()).then_some(host)
}

/// SSRF guard shared by Discover and `dotnet list --source`: only HTTPS to a public host.
/// A local path is allowed for configured folder sources; plain HTTP and private/loopback/
/// link-local hosts are refused before any request or process.
pub fn validate_source_arg(source: &str) -> Result<(), String> {
    if source.starts_with("http://") {
        return Err("NuGet sources must use HTTPS; plain HTTP is refused.".to_string());
    }
    if !source.starts_with("https://") {
        return Err("NuGet sources must use HTTPS.".to_string());
    }
    let Some(host) = url_host(source) else {
        return Err("The NuGet source URL is invalid.".to_string());
    };
    if crate::netguard::private_host(host) {
        return Err(format!(
            "The NuGet source host {host} is loopback, private or link-local and is refused."
        ));
    }
    Ok(())
}

pub fn source_matches(source: &str, configured: &[Source]) -> bool {
    configured.iter().any(|item| {
        let value = item.url.as_deref().unwrap_or(&item.name);
        value.trim_end_matches('/') == source.trim_end_matches('/')
    })
}

pub fn configured_sources(root: &Path, muxer: &Path, sdk_root: &Path) -> Sources {
    let sources_argv = vec![
        muxer.to_string_lossy().into_owned(),
        "nuget".to_string(),
        "list".to_string(),
        "source".to_string(),
        "--format".to_string(),
        "short".to_string(),
    ];
    match run(&sources_argv, root, sdk_root) {
        exec if exec.spawn_error.is_some() => Sources {
            state: "failed",
            message: Some(format!(
                "Could not list NuGet sources: {}",
                exec.spawn_error.unwrap_or_default()
            )),
            items: Vec::new(),
        },
        exec if exec.code != Some(0) => Sources {
            state: "failed",
            message: Some(first_line(&exec.stderr).unwrap_or_else(|| {
                "dotnet nuget list source failed; the configured source is unknown.".to_string()
            })),
            items: Vec::new(),
        },
        exec => match parse_sources(&exec.stdout) {
            Ok(items) => Sources {
                state: "ok",
                message: None,
                items,
            },
            Err(error) => Sources {
                state: "failed",
                message: Some(format!("dotnet nuget list source returned {error}.")),
                items: Vec::new(),
            },
        },
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchItem {
    pub id: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub downloads: Option<u64>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Search {
    pub state: &'static str,
    pub message: Option<String>,
    pub source: String,
    pub query: String,
    pub prerelease: bool,
    pub items: Vec<SearchItem>,
}

fn percent_encode(text: &str) -> String {
    let mut encoded = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            b' ' => encoded.push('+'),
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    encoded
}

/// Discover uses the source's own v3 SearchQueryService. It never installs and never falls
/// back to another source when the selected one is not HTTP or does not advertise search.
pub fn search(source: &str, query: &str, prerelease: bool, take: usize) -> Search {
    let failed = |message: String| Search {
        state: "failed",
        message: Some(message),
        source: source.to_string(),
        query: query.to_string(),
        prerelease,
        items: Vec::new(),
    };
    if !source.starts_with("https://") {
        return failed(format!(
            "Discover needs an HTTPS NuGet v3 source; {source} is not one. Package installation still uses the configured sources."
        ));
    }
    if let Err(error) = validate_source_arg(source) {
        return failed(error);
    }
    let agent = match crate::http::agent_for(source) {
        Ok(builder) => builder
            .timeout(std::time::Duration::from_secs(20))
            .redirects(0)
            .build(),
        Err(error) => return failed(format!("Could not read the NuGet source index: {error}")),
    };
    let index: Value = match agent.get(source).call() {
        Ok(response) => match response.into_string() {
            Ok(text) => match serde_json::from_str::<Value>(&text) {
                Ok(value) => value,
                Err(error) => {
                    return failed(format!("Could not read the NuGet source index: {error}"))
                }
            },
            Err(error) => return failed(format!("Could not read the NuGet source index: {error}")),
        },
        Err(error) => {
            return failed(format!(
                "Could not read the NuGet source index: {}",
                crate::http::describe_error(source, &error)
            ))
        }
    };
    let service = index
        .get("resources")
        .and_then(Value::as_array)
        .and_then(|resources| {
            resources.iter().find(|resource| {
                resource
                    .get("@type")
                    .and_then(Value::as_str)
                    .is_some_and(|kind| {
                        kind == "SearchQueryService" || kind.starts_with("SearchQueryService/")
                    })
            })
        })
        .and_then(|resource| resource.get("@id"))
        .and_then(Value::as_str);
    let Some(service) = service else {
        return failed(
            "The selected NuGet source does not advertise a v3 SearchQueryService.".to_string(),
        );
    };
    if let Err(error) = validate_source_arg(service) {
        return failed(format!(
            "The source advertised an unsafe search endpoint: {error}"
        ));
    }
    let separator = if service.contains('?') { '&' } else { '?' };
    let url = format!(
        "{service}{separator}q={}&take={}&prerelease={}&semVerLevel=2.0.0",
        percent_encode(query),
        take.clamp(1, 100),
        prerelease
    );
    // The advertised endpoint is a different destination: route and guard it on its own.
    let agent = match crate::http::agent_for(&url) {
        Ok(builder) => builder
            .timeout(std::time::Duration::from_secs(20))
            .redirects(0)
            .build(),
        Err(error) => return failed(format!("NuGet search failed: {error}")),
    };
    let body: Value = match agent.get(&url).call() {
        Ok(response) => match response.into_string() {
            Ok(text) => match serde_json::from_str::<Value>(&text) {
                Ok(value) => value,
                Err(error) => {
                    return failed(format!("NuGet search returned invalid JSON: {error}"))
                }
            },
            Err(error) => return failed(format!("NuGet search failed: {error}")),
        },
        Err(error) => {
            return failed(format!(
                "NuGet search failed: {}",
                crate::http::describe_error(&url, &error)
            ))
        }
    };
    let items = body
        .get("data")
        .and_then(Value::as_array)
        .map(|data| {
            data.iter()
                .filter_map(|item| {
                    let id = item.get("id").and_then(Value::as_str)?.to_string();
                    let authors = match item.get("authors") {
                        Some(Value::String(text)) => text
                            .split(',')
                            .map(str::trim)
                            .filter(|author| !author.is_empty())
                            .map(str::to_string)
                            .collect(),
                        Some(Value::Array(values)) => values
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect(),
                        _ => Vec::new(),
                    };
                    Some(SearchItem {
                        id,
                        version: item
                            .get("version")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        description: item
                            .get("description")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        authors,
                        downloads: item.get("totalDownloads").and_then(Value::as_u64),
                        verified: item
                            .get("verified")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Search {
        state: "ok",
        message: None,
        source: source.to_string(),
        query: query.to_string(),
        prerelease,
        items,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreRun {
    pub state: &'static str,
    pub message: Option<String>,
    pub command: String,
    pub exit: Option<i32>,
    pub log: String,
}

/// One explicit restore after a confirmed batch. Its failure is reported, never hidden.
pub fn restore(plan: &Plan) -> RestoreRun {
    let argv = restore_argv(plan);
    let command = display(&argv);
    let exec = run(&argv, &plan.root, &plan.sdk_root);
    let (log, _) = cap_log(&format!(
        "$ {command}\nexit {}\n--- stdout\n{}\n--- stderr\n{}",
        exec.code
            .map(|code| code.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        exec.stdout.trim_end(),
        exec.stderr.trim_end()
    ));
    let (state, message) = if let Some(error) = exec.spawn_error {
        (
            "failed",
            Some(format!("Could not run dotnet restore: {error}")),
        )
    } else if exec.code != Some(0) {
        (
            "failed",
            Some(first_line(&exec.stderr).unwrap_or_else(|| {
                format!(
                    "dotnet restore exited with {}.",
                    exec.code
                        .map(|code| code.to_string())
                        .unwrap_or_else(|| "an unknown status".to_string())
                )
            })),
        )
    } else {
        ("ok", None)
    };
    RestoreRun {
        state,
        message,
        command,
        exit: exec.code,
        log,
    }
}

fn run(argv: &[String], cwd: &Path, sdk_root: &Path) -> Exec {
    let output = crate::dotnet_command(argv, cwd, sdk_root)
        .stdin(Stdio::null())
        .output();
    match output {
        Ok(output) => Exec {
            code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            spawn_error: None,
        },
        Err(error) => Exec {
            code: None,
            stdout: String::new(),
            stderr: String::new(),
            spawn_error: Some(error.to_string()),
        },
    }
}

fn first_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

fn cap(frameworks: Vec<FrameworkPackages>) -> (Vec<FrameworkPackages>, bool) {
    let mut total = 0;
    let mut truncated = false;
    let mut capped = Vec::with_capacity(frameworks.len());
    for mut framework in frameworks {
        if total >= PACKAGE_LIMIT {
            truncated = truncated || !framework.packages.is_empty();
            framework.packages.clear();
        } else if total + framework.packages.len() > PACKAGE_LIMIT {
            framework.packages.truncate(PACKAGE_LIMIT - total);
            truncated = true;
        }
        total += framework.packages.len();
        capped.push(framework);
    }
    (capped, truncated)
}

fn cap_log(text: &str) -> (String, bool) {
    if text.len() <= LOG_LIMIT {
        return (text.to_string(), false);
    }
    let mut end = LOG_LIMIT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (
        format!("{}\n… truncated at {LOG_LIMIT} characters", &text[..end]),
        true,
    )
}

/// Development-only deterministic data. It is reachable only with OPENRIDR_ENABLE_PROBE=1 and
/// never runs dotnet; every response says so.
// Same shape as `list`: the probe mirrors the real call so both share one parameter order.
#[allow(clippy::too_many_arguments)]
pub fn probe(
    project_display: &str,
    name: &str,
    target_framework: Option<&str>,
    cpm: bool,
    empty: bool,
    sources_failed: bool,
    prerelease: bool,
    selected_source: Option<&str>,
) -> Listing {
    let mut frameworks = if empty {
        Vec::new()
    } else {
        vec![
            FrameworkPackages {
                framework: "net8.0".to_string(),
                packages: vec![
                    Package {
                        id: "Newtonsoft.Json".to_string(),
                        requested: Some("13.0.1".to_string()),
                        resolved: Some("13.0.1".to_string()),
                        latest_version: Some("13.0.3".to_string()),
                        outdated: true,
                    },
                    Package {
                        id: "Serilog".to_string(),
                        requested: Some("3.1.1".to_string()),
                        resolved: Some("3.1.1".to_string()),
                        latest_version: None,
                        outdated: false,
                    },
                ],
            },
            FrameworkPackages {
                framework: "netstandard2.0".to_string(),
                packages: vec![Package {
                    id: "Humanizer".to_string(),
                    requested: Some("2.14.1".to_string()),
                    resolved: Some("2.14.1".to_string()),
                    latest_version: Some("2.14.1".to_string()),
                    outdated: false,
                }],
            },
        ]
    };
    if let Some(tfm) = target_framework {
        frameworks.retain(|framework| framework.framework == tfm);
    }
    let installed = frameworks.clone();
    let outdated = frameworks
        .iter()
        .map(|framework| FrameworkPackages {
            framework: framework.framework.clone(),
            packages: framework
                .packages
                .iter()
                .filter(|package| package.outdated)
                .cloned()
                .collect(),
        })
        .filter(|framework| !framework.packages.is_empty())
        .collect::<Vec<_>>();
    let total_packages = frameworks.iter().map(|item| item.packages.len()).sum();
    let outdated_count = frameworks
        .iter()
        .flat_map(|item| &item.packages)
        .filter(|package| package.outdated)
        .count();
    let probe_command = "probe: dotnet was not run";
    Listing {
        state: "ok",
        message: Some(
            "Development-only probe data; dotnet list was not run. Set OPENRIDR_ENABLE_PROBE=0 to hide it."
                .to_string(),
        ),
        project: project_display.to_string(),
        project_name: name.to_string(),
        target_framework: target_framework.map(str::to_string),
        sdk_version: "probe".to_string(),
        sdk_root: "probe".to_string(),
        cpm: Cpm {
            enabled: cpm,
            path: cpm.then(|| "Directory.Packages.props".to_string()),
            message: cpm.then(|| {
                "Central Package Management is enabled by Directory.Packages.props; the MVP lists packages but refuses updates because it does not edit central versions."
                    .to_string()
            }),
        },
        installed: CommandResult {
            state: "ok",
            message: None,
            command: probe_command.to_string(),
            exit: None,
            log: "$ probe\nno process was run".to_string(),
            frameworks: installed,
            truncated: false,
        },
        outdated: CommandResult {
            state: "ok",
            message: None,
            command: probe_command.to_string(),
            exit: None,
            log: "$ probe\nno process was run".to_string(),
            frameworks: outdated,
            truncated: false,
        },
        sources: if sources_failed {
            Sources {
                state: "failed",
                message: Some(
                    "dotnet nuget list source failed; the configured source is unknown.".to_string(),
                ),
                items: Vec::new(),
            }
        } else {
            Sources {
                state: "ok",
                message: None,
                items: vec![Source {
                    name: "nuget.org (probe)".to_string(),
                    url: Some("https://api.nuget.org/v3/index.json".to_string()),
                }],
            }
        },
        frameworks,
        total_packages,
        outdated_count,
        truncated: false,
        prerelease,
        selected_source: selected_source.map(str::to_string),
        assets: Assets {
            state: if empty { "missing" } else { "ok" },
            message: empty.then(|| {
                "obj/project.assets.json is not readable in the probe; restore the project to list implicit and transitive packages."
                    .to_string()
            }),
            path: "probe/obj/project.assets.json".to_string(),
            frameworks: if empty {
                Vec::new()
            } else {
                vec![AssetsFramework {
                    framework: "net8.0".to_string(),
                    implicit: vec![AssetsPackage {
                        id: "Microsoft.NETCore.App.Ref".to_string(),
                        version: Some("8.0.0".to_string()),
                    }],
                    transitive: vec![AssetsPackage {
                        id: "Microsoft.Extensions.Primitives".to_string(),
                        version: Some("8.0.0".to_string()),
                    }],
                }]
            },
        },
        probe: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    const INSTALLED: &str = include_str!("../tests/fixtures/nuget/installed.json");
    const OUTDATED: &str = include_str!("../tests/fixtures/nuget/outdated.json");

    fn plan_at(root: &Path) -> Plan {
        let project = root.join("App.csproj");
        fs::write(&project, PROJECT).unwrap();
        Plan {
            id: "plan-1".to_string(),
            workspace_id: 1,
            context_id: 2,
            root: root.to_path_buf(),
            project,
            project_display: "App.csproj".to_string(),
            package: "Newtonsoft.Json".to_string(),
            from_version: Some("13.0.1".to_string()),
            version: "13.0.3".to_string(),
            target_framework: Some("net8.0".to_string()),
            muxer: PathBuf::from("/sdk/dotnet"),
            sdk_root: PathBuf::from("/sdk"),
            install: false,
            probe: false,
        }
    }

    const PROJECT: &str = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup><TargetFramework>net8.0</TargetFramework></PropertyGroup>
  <ItemGroup>
    <PackageReference Include="Newtonsoft.Json" Version="13.0.1" />
  </ItemGroup>
</Project>
"#;

    #[cfg(unix)]
    #[test]
    fn resolve_project_refuses_a_missing_project_under_an_escaping_symlink() {
        let outer = tempfile::tempdir().unwrap();
        let root = outer.path().join("ws");
        fs::create_dir_all(&root).unwrap();
        let outside = outer.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        let root = fs::canonicalize(&root).unwrap();
        fs::write(root.join("App.csproj"), PROJECT).unwrap();
        std::os::unix::fs::symlink(&outside, root.join("Link")).unwrap();

        let (path, rel) = resolve_project(&Scope::new(&root), "App.csproj").unwrap();
        assert_eq!(path, root.join("App.csproj"));
        assert_eq!(rel, "App.csproj");

        // The project does not exist, but the symlink carries the path outside: the refusal must say
        // outside, exactly as it does when the file exists there.
        let error = resolve_project(&Scope::new(&root), "Link/missing.csproj").unwrap_err();
        assert_eq!(error.code, "outside_workspace");
        fs::write(outside.join("There.csproj"), PROJECT).unwrap();
        let error = resolve_project(&Scope::new(&root), "Link/There.csproj").unwrap_err();
        assert_eq!(error.code, "outside_workspace");
        let error = resolve_project(&Scope::new(&root), "missing.csproj").unwrap_err();
        assert_eq!(error.code, "not_found");

        // A dangling link exists but points nowhere provable: refused the same way whether or not
        // the target exists outside.
        std::os::unix::fs::symlink(outside.join("Missing.csproj"), root.join("Dangling.csproj"))
            .unwrap();
        fs::write(outside.join("There.csproj"), PROJECT).unwrap();
        std::os::unix::fs::symlink(
            outside.join("There.csproj"),
            root.join("DanglingThere.csproj"),
        )
        .unwrap();
        for project in ["Dangling.csproj", "DanglingThere.csproj"] {
            let error = resolve_project(&Scope::new(&root), project).unwrap_err();
            assert_eq!(error.code, "outside_workspace", "{project}");
        }
    }

    #[test]
    fn parser_reads_resolved_version_and_requested_version_fields() {
        // Newer dotnet list output uses *Version; Current must not stay "unknown".
        let frameworks = parse_listing(
            r#"{"projects":[{"path":"A.csproj","frameworks":[{"framework":"net8.0",
                "topLevelPackages":[
                    {"id":"Microsoft.NET.Test.Sdk","requestedVersion":"17.11.1","resolvedVersion":"17.11.1","latestVersion":"18.11.0"},
                    {"id":"xunit","requestedVersion":"2.9.2","resolvedVersion":"2.9.2"},
                    {"id":"xunit.runner.visualstudio","requestedVersion":"2.8.2","resolvedVersion":"2.8.2"}
                ]}]}]}"#,
        )
        .unwrap();
        let packages = &frameworks[0].packages;
        assert_eq!(packages[0].requested.as_deref(), Some("17.11.1"));
        assert_eq!(packages[0].resolved.as_deref(), Some("17.11.1"));
        assert_eq!(packages[0].latest_version.as_deref(), Some("18.11.0"));
        assert_eq!(packages[1].resolved.as_deref(), Some("2.9.2"));
        assert_eq!(packages[2].resolved.as_deref(), Some("2.8.2"));
    }

    #[test]
    fn recorded_listing_is_grouped_by_tfm_and_reads_all_versions() {
        let frameworks = parse_listing(INSTALLED).unwrap();
        assert_eq!(frameworks.len(), 2);
        assert_eq!(frameworks[0].framework, "net8.0");
        assert_eq!(frameworks[0].packages.len(), 3);
        let json = &frameworks[0].packages[0];
        assert_eq!(json.id, "Microsoft.NET.Test.Sdk");
        assert_eq!(json.requested.as_deref(), Some("17.11.1"));
        assert_eq!(json.resolved.as_deref(), Some("17.11.1"));
        assert_eq!(json.latest_version, None);
        let xunit = frameworks[0]
            .packages
            .iter()
            .find(|package| package.id == "xunit")
            .unwrap();
        assert_eq!(xunit.id, "xunit");
        assert_eq!(xunit.requested.as_deref(), Some("2.9.2"));
        assert_eq!(xunit.resolved.as_deref(), Some("2.9.2"));
        assert_eq!(frameworks[1].framework, "netstandard2.0");
    }

    #[test]
    fn recorded_outdated_merge_shows_current_and_latest_per_tfm() {
        let installed = parse_listing(INSTALLED).unwrap();
        let outdated = parse_listing(OUTDATED).unwrap();
        let merged = merge(&installed, &outdated);
        let net8 = &merged[0];
        assert_eq!(net8.framework, "net8.0");
        let xunit = net8
            .packages
            .iter()
            .find(|item| item.id == "xunit")
            .unwrap();
        assert_eq!(xunit.resolved.as_deref(), Some("2.9.2"));
        assert_eq!(xunit.latest_version.as_deref(), Some("2.9.4"));
        assert!(xunit.outdated);
        let testsdk = net8
            .packages
            .iter()
            .find(|item| item.id == "Microsoft.NET.Test.Sdk")
            .unwrap();
        assert!(!testsdk.outdated);
        // The outdated report can mention a TFM that the installed report grouped separately.
        assert!(merged.iter().any(|item| item.framework == "netstandard2.0"));
    }

    #[test]
    fn invalid_or_empty_json_is_an_error_never_an_all_up_to_date_list() {
        assert!(parse_listing("{not json").is_err());
        assert!(parse_listing(r#"{"version":1}"#).is_err());
        assert!(parse_listing(r#"{"projects":[]}"#).is_err());
        assert!(parse_listing(r#"{"projects":[{"frameworks":[]}]}"#).is_err());
        assert!(parse_listing(r#"{"projects":[{"frameworks":[{"framework":"net8.0"}]}]}"#).is_ok());
        assert!(parse_listing(r#"{"projects":[{"path":"App.csproj"}]}"#)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn parser_tolerates_partial_packages_and_unknown_keys() {
        let frameworks = parse_listing(
            r#"{"projects":[{"path":"A.csproj","unknown":true,"frameworks":[
                {"framework":"net8.0","transitivePackages":[{"id":"Transitive","resolved":"1.0.0"}],
                 "topLevelPackages":[
                    {"id":"A","resolved":"1.0.0","extra":"ignored"},
                    {"id":"B","requested":"2.0.0","resolved":null},
                    {"resolved":"3.0.0"},
                    {"id":"A","requested":"1.0.0","latestVersion":"1.2.0"}
                 ]}]}]}"#,
        )
        .unwrap();
        assert_eq!(frameworks.len(), 1);
        let packages = &frameworks[0].packages;
        assert_eq!(
            packages.len(),
            2,
            "transitive and id-less entries are ignored"
        );
        let a = packages.iter().find(|item| item.id == "A").unwrap();
        assert_eq!(a.requested.as_deref(), Some("1.0.0"));
        assert_eq!(a.latest_version.as_deref(), Some("1.2.0"));
        let b = packages.iter().find(|item| item.id == "B").unwrap();
        assert_eq!(b.requested.as_deref(), Some("2.0.0"));
        assert_eq!(b.resolved, None);
    }

    #[test]
    fn short_source_listing_only_reports_enabled_sources_and_rejects_unknown_rows() {
        let sources = parse_sources(
            "E https://api.nuget.org/v3/index.json\nD /tmp/disabled feed\nE /tmp/local feed\n",
        )
        .unwrap();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].name, "https://api.nuget.org/v3/index.json");
        assert_eq!(sources[1].name, "/tmp/local feed");
        assert!(parse_sources("").unwrap().is_empty());
        assert!(parse_sources("unexpected output").is_err());
    }

    #[test]
    fn cpm_is_detected_by_props_file_and_the_property_can_turn_it_off() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("App.csproj");
        fs::write(&project, PROJECT).unwrap();
        assert!(!detect_cpm(dir.path(), &project).enabled);

        fs::write(
            dir.path().join("Directory.Packages.props"),
            "<Project><PropertyGroup><ManagePackageVersionsCentrally>true</ManagePackageVersionsCentrally></PropertyGroup></Project>",
        )
        .unwrap();
        let cpm = detect_cpm(dir.path(), &project);
        assert!(cpm.enabled);
        assert_eq!(cpm.path.as_deref(), Some("Directory.Packages.props"));

        fs::write(
            dir.path().join("Directory.Packages.props"),
            "<Project><PropertyGroup><ManagePackageVersionsCentrally>false</ManagePackageVersionsCentrally></PropertyGroup></Project>",
        )
        .unwrap();
        assert!(!detect_cpm(dir.path(), &project).enabled);
    }

    #[test]
    fn cpm_detection_for_an_outside_project_stops_at_its_own_directory() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("app");
        let lib = temp.path().join("lib");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&lib).unwrap();
        fs::write(lib.join("Lib.csproj"), PROJECT).unwrap();
        let root = fs::canonicalize(&root).unwrap();
        let lib = fs::canonicalize(&lib).unwrap();
        let project = lib.join("Lib.csproj");
        let props = "<Project><PropertyGroup><ManagePackageVersionsCentrally>true</ManagePackageVersionsCentrally></PropertyGroup></Project>";

        // Above the project's own directory is outside its scope entry: never read.
        fs::write(temp.path().join("Directory.Packages.props"), props).unwrap();
        assert!(!detect_cpm(&root, &project).enabled);

        // Its own directory is the scope entry for a project outside the root.
        fs::write(lib.join("Directory.Packages.props"), props).unwrap();
        let cpm = detect_cpm(&root, &project);
        assert!(cpm.enabled);
        let expected = crate::fsutil::portable_path(&lib.join("Directory.Packages.props"));
        assert_eq!(cpm.path.as_deref(), Some(expected.as_str()));

        // A project inside the root still walks up to the root, as before.
        let inside = root.join("App.csproj");
        fs::write(&inside, PROJECT).unwrap();
        fs::write(root.join("Directory.Packages.props"), props).unwrap();
        assert!(detect_cpm(&root, &inside).enabled);
    }

    #[test]
    fn plan_uses_the_exact_add_command_and_refuses_cpm_before_any_process() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("App.csproj");
        fs::write(&project, PROJECT).unwrap();
        let request = PlanRequest {
            id: "plan-1".to_string(),
            workspace_id: 1,
            context_id: 2,
            root: dir.path().to_path_buf(),
            project: project.clone(),
            project_display: "App.csproj".to_string(),
            package: "Newtonsoft.Json".to_string(),
            from_version: Some("13.0.1".to_string()),
            version: "13.0.3".to_string(),
            target_framework: Some("net8.0".to_string()),
            muxer: PathBuf::from("/sdk/dotnet"),
            sdk_root: PathBuf::from("/sdk"),
            install: false,
            defer_cpm: false,
        };
        let plan = make_plan(request).unwrap();
        let view = plan_view(&plan);
        assert_eq!(
            view["command"],
            "/sdk/dotnet add App.csproj package Newtonsoft.Json --version 13.0.3 --no-restore"
        );
        assert!(view["restore"]["note"]
            .as_str()
            .unwrap()
            .contains("never restores implicitly"));

        fs::write(
            dir.path().join("Directory.Packages.props"),
            "<Project><PropertyGroup><ManagePackageVersionsCentrally>true</ManagePackageVersionsCentrally></PropertyGroup></Project>",
        )
        .unwrap();
        let error = make_plan(PlanRequest {
            id: "plan-2".to_string(),
            workspace_id: 1,
            context_id: 2,
            root: dir.path().to_path_buf(),
            project,
            project_display: "App.csproj".to_string(),
            package: "Newtonsoft.Json".to_string(),
            from_version: Some("13.0.1".to_string()),
            version: "13.0.3".to_string(),
            target_framework: None,
            muxer: PathBuf::from("/sdk/dotnet"),
            sdk_root: PathBuf::from("/sdk"),
            install: false,
            defer_cpm: false,
        })
        .expect_err("CPM refuses the update plan");
        assert_eq!(error.code, "nuget_cpm_unsupported");
        assert!(error.message.contains("Central Package Management"));
    }

    #[test]
    fn revalidation_rejects_a_missing_package_a_changed_version_and_the_target() {
        let dir = tempfile::tempdir().unwrap();
        let plan = plan_at(dir.path());
        assert!(revalidate_text(&plan, PROJECT).is_ok());

        let already = PROJECT.replace("13.0.1", "13.0.3");
        assert_eq!(
            revalidate_text(&plan, &already)
                .expect_err("already updated")
                .code,
            "nuget_stale_plan"
        );

        let changed = PROJECT.replace("13.0.1", "13.0.2");
        assert_eq!(
            revalidate_text(&plan, &changed).expect_err("changed").code,
            "nuget_stale_plan"
        );

        let removed = PROJECT.replace("Newtonsoft.Json", "Serilog");
        let error = revalidate_text(&plan, &removed).expect_err("removed");
        assert_eq!(error.code, "nuget_stale_plan");
        assert!(error.message.contains("no longer references"));

        let absent = PROJECT.replace(
            r#"<PackageReference Include="Newtonsoft.Json" Version="13.0.1" />"#,
            "",
        );
        assert!(revalidate_text(&plan, &absent).is_err());
    }

    #[test]
    fn package_version_handles_attributes_properties_and_other_packages() {
        assert_eq!(
            package_version(PROJECT, "newtonsoft.json").as_deref(),
            Some("13.0.1")
        );
        assert_eq!(
            package_version(
                r#"<ItemGroup><PackageReference Version="1.0.0" Include="A" /></ItemGroup>"#,
                "A"
            )
            .as_deref(),
            Some("1.0.0")
        );
        assert_eq!(
            package_version(
                r#"<PackageReference Include="A" Version="$(AVersion)" />"#,
                "A"
            )
            .as_deref(),
            Some("")
        );
        assert_eq!(package_version(PROJECT, "Serilog"), None);
    }

    #[test]
    fn package_limit_truncates_instead_of_reporting_everything() {
        let packages: Vec<Package> = (0..PACKAGE_LIMIT + 10)
            .map(|index| Package {
                id: format!("Package{index:04}"),
                requested: None,
                resolved: None,
                latest_version: None,
                outdated: false,
            })
            .collect();
        let (frameworks, truncated) = cap(vec![FrameworkPackages {
            framework: "net8.0".to_string(),
            packages,
        }]);
        assert!(truncated);
        assert_eq!(frameworks[0].packages.len(), PACKAGE_LIMIT);
    }

    const NO_ASSETS: &str = r#"{
  "version": 1,
  "parameters": "",
  "problems": [
    {
      "project": "/work/OrderDesk.Cli.csproj",
      "level": "error",
      "text": "No assets file was found for `/work/OrderDesk.Cli.csproj`. Please run restore before running this command."
    }
  ],
  "projects": [{ "path": "/work/OrderDesk.Cli.csproj" }]
}
"#;

    fn listing_exec(code: Option<i32>, stdout: &str, stderr: &str) -> Exec {
        Exec {
            code,
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
            spawn_error: None,
        }
    }

    #[test]
    fn a_project_without_package_references_is_an_empty_list_not_the_raw_error() {
        let argv = vec!["dotnet".to_string(), "list".to_string()];
        let result = command_result(
            listing_exec(Some(1), NO_ASSETS, ""),
            &argv,
            parse_listing,
            "dotnet list package --outdated",
            true,
        );
        assert_eq!(result.state, "ok");
        assert_eq!(result.message, None);
        assert_eq!(result.exit, Some(1));
        assert!(result.frameworks.is_empty());
        assert!(result.log.contains("No assets file"));
    }

    #[test]
    fn the_same_restore_failure_stays_an_error_when_the_project_references_packages() {
        let argv = vec!["dotnet".to_string(), "list".to_string()];
        let result = command_result(
            listing_exec(Some(1), NO_ASSETS, ""),
            &argv,
            parse_listing,
            "dotnet list package --outdated",
            false,
        );
        assert_eq!(result.state, "failed");
        assert_eq!(result.exit, Some(1));
    }

    #[test]
    fn a_real_dotnet_failure_is_never_an_empty_list_even_without_references() {
        let argv = vec!["dotnet".to_string(), "list".to_string()];
        let network = command_result(
            listing_exec(
                Some(1),
                "",
                "Unable to load the service index for source https://api.nuget.org\n",
            ),
            &argv,
            parse_listing,
            "dotnet list package --outdated",
            true,
        );
        assert_eq!(network.state, "failed");
        let mixed = command_result(
            listing_exec(
                Some(1),
                &NO_ASSETS.replace("No assets file was found", "Unable to read the project"),
                "",
            ),
            &argv,
            parse_listing,
            "dotnet list package --outdated",
            true,
        );
        assert_eq!(mixed.state, "failed");
    }

    #[test]
    fn an_unknown_exit_fails_and_preserves_stdout_and_stderr_in_the_log() {
        let argv = vec!["dotnet".to_string(), "list".to_string()];
        let result = command_result(
            Exec {
                code: None,
                stdout: "partial stdout\n".to_string(),
                stderr: "network unavailable\n".to_string(),
                spawn_error: None,
            },
            &argv,
            parse_listing,
            "dotnet list package",
            true,
        );
        assert_eq!(result.state, "failed");
        assert_eq!(result.exit, None);
        assert_eq!(result.message.as_deref(), Some("network unavailable"));
        assert!(result.log.contains("exit unknown"));
        assert!(result.log.contains("partial stdout"));
        assert!(result.log.contains("network unavailable"));
        assert!(result.frameworks.is_empty());
    }

    #[test]
    fn an_invalid_json_after_exit_zero_is_failed_not_empty_success() {
        let argv = vec!["dotnet".to_string(), "list".to_string()];
        let result = command_result(
            Exec {
                code: Some(0),
                stdout: "not json\n".to_string(),
                stderr: String::new(),
                spawn_error: None,
            },
            &argv,
            parse_listing,
            "dotnet list package",
            true,
        );
        assert_eq!(result.state, "failed");
        assert_eq!(result.exit, Some(0));
        assert!(result.message.unwrap().contains("invalid data"));
    }

    #[test]
    fn plan_id_is_stable_and_bound_to_the_context() {
        let first = plan_id(1, 2, "App.csproj", "A", "1.0.0");
        assert_eq!(first, plan_id(1, 2, "App.csproj", "A", "1.0.0"));
        assert_ne!(first, plan_id(1, 3, "App.csproj", "A", "1.0.0"));
        assert!(first.starts_with("nuget-2-1-"));
    }

    #[test]
    fn assets_read_implicit_and_transitive_without_guessing() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("App.csproj");
        fs::write(&project, "<Project />").unwrap();
        fs::create_dir_all(dir.path().join("obj")).unwrap();
        fs::write(
            dir.path().join("obj/project.assets.json"),
            r#"{
              "targets": {
                "net8.0": {
                  "Microsoft.NETCore.App.Ref/8.0.0": { "type": "package" },
                  "Newtonsoft.Json/13.0.1": { "type": "package" },
                  "Microsoft.Extensions.Primitives/8.0.0": { "type": "package" },
                  "Serilog/3.1.1": { "type": "package" }
                }
              },
              "project": {
                "frameworks": {
                  "net8.0": {
                    "dependencies": {
                      "Microsoft.NETCore.App.Ref": { "target": "Package", "version": "[8.0.0, )", "autoReferenced": true },
                      "Newtonsoft.Json": { "target": "Package", "version": "[13.0.1, )" },
                      "Serilog": { "target": "Package", "version": "[3.1.1, )" }
                    },
                    "downloadDependencies": [
                      { "name": "Microsoft.AspNetCore.App.Ref", "version": "[8.0.0, )" }
                    ]
                  }
                }
              }
            }"#,
        )
        .unwrap();
        let assets = read_assets(&project, Some("net8.0"));
        assert_eq!(assets.state, "ok");
        assert_eq!(assets.frameworks.len(), 1);
        assert_eq!(assets.frameworks[0].implicit.len(), 2);
        assert!(assets.frameworks[0]
            .implicit
            .iter()
            .any(|package| package.id == "Microsoft.NETCore.App.Ref"));
        assert!(assets.frameworks[0]
            .implicit
            .iter()
            .any(|package| package.id == "Microsoft.AspNetCore.App.Ref"));
        assert_eq!(assets.frameworks[0].transitive.len(), 1);
        assert_eq!(
            assets.frameworks[0].transitive[0].id,
            "Microsoft.Extensions.Primitives"
        );
        assert!(!assets.frameworks[0]
            .transitive
            .iter()
            .any(|package| package.id == "Newtonsoft.Json" || package.id == "Serilog"));

        let missing = read_assets(&project, Some("net9.0"));
        assert_eq!(missing.state, "missing");
        assert!(missing.message.unwrap().contains("no package targets"));
    }

    #[test]
    fn install_plan_allows_a_package_that_is_not_referenced_yet() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("App.csproj");
        fs::write(&project, "<Project Sdk=\"Microsoft.NET.Sdk\" />").unwrap();
        let request = |install: bool| PlanRequest {
            id: "plan-install".to_string(),
            workspace_id: 1,
            context_id: 2,
            root: dir.path().to_path_buf(),
            project: project.clone(),
            project_display: "App.csproj".to_string(),
            package: "Newtonsoft.Json".to_string(),
            from_version: None,
            version: "13.0.3".to_string(),
            target_framework: None,
            muxer: PathBuf::from("/sdk/dotnet"),
            sdk_root: PathBuf::from("/sdk"),
            install,
            defer_cpm: false,
        };
        let error = make_plan(request(false)).expect_err("an update needs the package");
        assert_eq!(error.code, "nuget_stale_plan");
        let plan = make_plan(request(true)).unwrap();
        assert!(plan.install);
        assert_eq!(
            plan_view(&plan)["command"],
            "/sdk/dotnet add App.csproj package Newtonsoft.Json --version 13.0.3 --no-restore"
        );
        assert_eq!(plan_view(&plan)["install"], true);
    }

    fn restore_project(project: &Path) {
        let output = Command::new("dotnet")
            .args([
                "restore",
                project.to_string_lossy().as_ref(),
                "-nologo",
                "-v:q",
            ])
            .env("DOTNET_NOLOGO", "1")
            .env("DOTNET_CLI_UI_LANGUAGE", "en")
            .output()
            .expect("dotnet is available for the real assets test");
        assert!(
            output.status.success(),
            "restore failed: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn real_assets_read_implicit_packages_from_project_dependencies() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("NetStandard.csproj");
        fs::write(
            &project,
            "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>netstandard2.0</TargetFramework></PropertyGroup></Project>",
        )
        .unwrap();
        restore_project(&project);
        let assets = read_assets(&project, Some("netstandard2.0"));
        assert_eq!(assets.state, "ok", "{:?}", assets.message);
        assert!(
            assets.frameworks[0]
                .implicit
                .iter()
                .any(|package| package.id == "NETStandard.Library"),
            "implicit: {:?}",
            assets.frameworks[0].implicit
        );
        assert!(!assets.frameworks[0]
            .transitive
            .iter()
            .any(|package| package.id == "NETStandard.Library"));
    }

    #[test]
    fn real_assets_fold_a_rid_target_into_one_framework() {
        if !cfg!(target_os = "macos") || !cfg!(target_arch = "aarch64") {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("Rid.csproj");
        fs::write(
            &project,
            "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net8.0</TargetFramework><RuntimeIdentifier>osx-arm64</RuntimeIdentifier></PropertyGroup></Project>",
        )
        .unwrap();
        restore_project(&project);
        let assets = read_assets(&project, None);
        assert_eq!(assets.state, "ok", "{:?}", assets.message);
        assert_eq!(
            assets
                .frameworks
                .iter()
                .map(|framework| framework.framework.as_str())
                .collect::<Vec<_>>(),
            vec!["net8.0"]
        );
    }

    #[test]
    fn source_validation_rejects_plain_http_and_private_hosts() {
        for source in [
            "http://api.nuget.org/v3/index.json",
            "https://localhost/v3/index.json",
            "https://127.0.0.1/v3/index.json",
            "https://10.0.0.2/v3/index.json",
            "https://192.168.1.2/v3/index.json",
            "https://169.254.1.2/v3/index.json",
            "https://[::1]/v3/index.json",
            "https://[::ffff:127.0.0.1]/v3/index.json",
            "https://[::ffff:10.0.0.1]/v3/index.json",
            "https://[::127.0.0.1]/v3/index.json",
            "https://[64:ff9b::7f00:1]/v3/index.json",
            "https://[64:ff9b::10.0.0.1]/v3/index.json",
            "https://0.0.0.0/v3/index.json",
            "https://100.64.0.1/v3/index.json",
            "https://198.18.0.1/v3/index.json",
            "https://224.0.0.1/v3/index.json",
            "https://[fc00::1]/v3/index.json",
            "https://[fe80::1]/v3/index.json",
            "https://nuget.local/v3/index.json",
        ] {
            assert!(validate_source_arg(source).is_err(), "{source}");
        }
        assert!(validate_source_arg("https://api.nuget.org/v3/index.json").is_ok());
        assert!(validate_source_arg("/tmp/local-feed").is_err());
        let unsafe_search = search("http://127.0.0.1:1/v3/index.json", "xunit", false, 5);
        assert_eq!(unsafe_search.state, "failed");
        assert!(unsafe_search.message.unwrap().contains("HTTPS"));
    }

    #[test]
    fn an_inherited_base_intermediate_props_file_marks_assets_unsupported() {
        let root = tempfile::tempdir().unwrap();
        let project_dir = root.path().join("src/App");
        fs::create_dir_all(&project_dir).unwrap();
        fs::write(
            root.path().join("Directory.Build.props"),
            "<Project><PropertyGroup><BaseIntermediateOutputPath>artifacts/obj/</BaseIntermediateOutputPath></PropertyGroup></Project>",
        )
        .unwrap();
        let project = project_dir.join("App.csproj");
        fs::write(&project, "<Project />").unwrap();
        let assets = read_assets(&project, Some("net8.0"));
        assert_eq!(assets.state, "unsupported");
        assert!(assets
            .message
            .unwrap()
            .contains("BaseIntermediateOutputPath"));
    }

    #[test]
    fn an_expanded_base_intermediate_path_marks_assets_unsupported_not_empty() {
        let dir = tempfile::tempdir().unwrap();
        let project = dir.path().join("App.csproj");
        fs::write(
            &project,
            "<Project><PropertyGroup><BaseIntermediateOutputPath>$(MSBuildProjectDirectory)/artifacts/obj/</BaseIntermediateOutputPath></PropertyGroup></Project>",
        )
        .unwrap();
        let assets = read_assets(&project, Some("net8.0"));
        assert_eq!(assets.state, "unsupported");
        assert!(assets.frameworks.is_empty());
        assert!(assets
            .message
            .unwrap()
            .contains("BaseIntermediateOutputPath"));
    }
}
