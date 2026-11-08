//! MSBuild evaluation of the open solution/project.
//!
//! One process per project: `.sln` files reject `-getProperty`/`-getItem` (MSB1063), so the
//! solution only supplies the project list, configurations and folders. Evaluation never
//! restores or builds; `obj/project.assets.json` is only inspected to report partial context.

use crate::scope::{ResolvedScope, Scope};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_CONFIGURATION: &str = "Debug";
const MAX_LOG_CHARS: usize = 32_000;
/// One directory listing per watched directory per state poll; the cap keeps huge trees cheap.
const MAX_WATCHED_DIRS: usize = 256;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluationContext {
    pub configuration: String,
    pub target_framework: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRef {
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageRef {
    pub name: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluatedProject {
    pub name: String,
    pub path: String,
    pub folder: Option<String>,
    pub state: &'static str,
    pub kind: &'static str,
    pub tfms: Vec<String>,
    /// The TFM actually used for this evaluation's TargetPath, straight from MSBuild.
    pub evaluated_tfm: Option<String>,
    pub target_path: Option<String>,
    /// Managed assembly to launch under the debugger: `TargetPath` or `TargetDir/TargetFileName`.
    pub assembly_path: Option<String>,
    pub output_type: Option<String>,
    pub project_refs: Vec<ProjectRef>,
    pub package_refs: Vec<PackageRef>,
    pub sources: Vec<String>,
    pub error: Option<String>,
    pub log: String,
    pub exit: Option<i32>,
    /// `restored` | `not_restored` | `unknown`; a missing/stale assets file never means clean.
    pub restored: &'static str,
    pub restored_reason: Option<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Watched {
    path: PathBuf,
    fingerprint: Fingerprint,
}

/// Files keep the mtime/size fingerprint; directories fingerprint the entries that can change
/// the evaluated tree, so an atomic save (temp file + rename) of an existing source is not a
/// change while a create/rename/delete is.
#[derive(Debug, Clone, PartialEq)]
enum Fingerprint {
    File(Option<(u64, u64, u64)>),
    Dir(Vec<String>),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Evaluation {
    pub state: &'static str,
    pub context: EvaluationContext,
    pub configurations: Vec<String>,
    pub available_tfms: Vec<String>,
    pub projects: Vec<EvaluatedProject>,
    pub log: String,
    pub stale: bool,
    pub stale_reason: Option<String>,
    pub revision: u64,
    pub workspace_id: Option<u64>,
    pub context_id: Option<u64>,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    #[serde(skip)]
    watched: Vec<Watched>,
}

impl Evaluation {
    pub fn idle() -> Self {
        Self {
            state: "idle",
            context: EvaluationContext {
                configuration: DEFAULT_CONFIGURATION.to_string(),
                target_framework: None,
            },
            configurations: Vec::new(),
            available_tfms: Vec::new(),
            projects: Vec::new(),
            log: String::new(),
            stale: false,
            stale_reason: None,
            revision: 0,
            workspace_id: None,
            context_id: None,
            started_at: None,
            finished_at: None,
            watched: Vec::new(),
        }
    }

    /// Options that come from text only. Reading them before trust is allowed; they are not
    /// presented as evaluated values.
    pub fn for_workspace(workspace_file: &Path, is_solution: bool, tfms: Vec<String>) -> Self {
        let mut evaluation = Self::idle();
        evaluation.configurations = if is_solution {
            let text = fs::read_to_string(workspace_file).unwrap_or_default();
            let configurations = parse_configurations(&text);
            if configurations.is_empty() {
                vec![DEFAULT_CONFIGURATION.to_string()]
            } else {
                configurations
            }
        } else {
            vec!["Debug".to_string(), "Release".to_string()]
        };
        evaluation.available_tfms = tfms;
        evaluation
    }

    pub fn failed_context(context: EvaluationContext, message: String) -> Self {
        let mut evaluation = Self::idle();
        evaluation.context = context;
        evaluation.state = "failed";
        evaluation.log = message;
        evaluation
    }

    pub fn begin(
        &mut self,
        context: EvaluationContext,
        configurations: Vec<String>,
        available_tfms: Vec<String>,
        workspace_id: u64,
        context_id: u64,
    ) {
        self.state = "running";
        self.context = context;
        self.configurations = configurations;
        self.available_tfms = available_tfms;
        self.projects.clear();
        self.stale = false;
        self.stale_reason = None;
        self.revision += 1;
        self.workspace_id = Some(workspace_id);
        self.context_id = Some(context_id);
        self.started_at = Some(now_ms());
        self.finished_at = None;
        self.watched.clear();
    }

    pub fn complete(
        &mut self,
        projects: Vec<EvaluatedProject>,
        log: String,
        watched: Vec<Watched>,
        available_tfms: Vec<String>,
    ) {
        let failed = projects
            .iter()
            .filter(|project| project.state == "failed")
            .count();
        self.state = if projects.is_empty() {
            "failed"
        } else if failed == 0 {
            "ok"
        } else if failed == projects.len() {
            "failed"
        } else {
            "partial"
        };
        if projects.is_empty() {
            self.log = format!("{log}\nNo projects were evaluated.");
        } else {
            self.log = log;
        }
        self.projects = projects;
        self.available_tfms = available_tfms;
        self.watched = watched;
        self.finished_at = Some(now_ms());
    }

    /// Keeps the observed model for the UI but marks it as not current. Returns true when a
    /// context change should also invalidate in-flight results.
    pub fn mark_stale(&mut self, reason: impl Into<String>) -> bool {
        if self.state == "idle" && self.revision == 0 && self.projects.is_empty() {
            return false;
        }
        self.stale = true;
        self.stale_reason = Some(reason.into());
        if self.state == "running" {
            // The in-flight result will be rejected by the context token; nothing may stay
            // labelled as running or as current.
            self.state = "idle";
            self.projects.clear();
            self.watched.clear();
            self.finished_at = Some(now_ms());
        }
        true
    }

    /// Detects external edits to project/import files. Cheap enough for the 400 ms state poll.
    pub fn check_external_changes(&mut self, root: &Path) {
        if self.stale || self.watched.is_empty() || self.state == "running" {
            return;
        }
        for watched in &self.watched {
            if fingerprint(&watched.path) != watched.fingerprint {
                let label = display_path(root, &watched.path);
                self.stale = true;
                self.stale_reason = Some(format!(
                    "{label} changed on disk; re-evaluate the project model."
                ));
                return;
            }
        }
    }
}

/// A project the host will evaluate. `error` covers entries that cannot be evaluated at all,
/// such as a project file outside the workspace.
#[derive(Debug, Clone)]
pub struct PlanProject {
    pub name: String,
    pub path: String,
    pub abs_path: Option<PathBuf>,
    pub folder: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EvalPlan {
    pub projects: Vec<PlanProject>,
    pub configurations: Vec<String>,
    /// The directories this workspace may touch: its root plus the projects' directories.
    pub scope: Scope,
}

#[derive(Debug, Clone, Default)]
pub struct MsbuildResult {
    pub target_framework: Option<String>,
    pub target_frameworks: Option<String>,
    pub target_path: Option<String>,
    pub target_dir: Option<String>,
    pub target_file_name: Option<String>,
    pub output_type: Option<String>,
    pub is_test_project: Option<String>,
    pub project_refs: Vec<(String, String)>, // (name, absolute path)
    pub packages: Vec<(String, Option<String>)>,
    pub compile: Vec<PathBuf>,
}

impl MsbuildResult {
    pub fn tfms(&self) -> Vec<String> {
        if let Some(multi) = self
            .target_frameworks
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            return multi
                .split(';')
                .map(str::trim)
                .filter(|tfm| !tfm.is_empty())
                .map(str::to_string)
                .collect();
        }
        self.target_framework
            .as_deref()
            .filter(|tfm| !tfm.trim().is_empty())
            .map(|tfm| vec![tfm.to_string()])
            .unwrap_or_default()
    }
}

struct Invoked {
    result: MsbuildResult,
    exit: Option<i32>,
    log: String,
}

struct InvokeError {
    error: String,
    exit: Option<i32>,
    log: String,
}

pub fn plan_workspace(workspace_file: &Path, root: &Path, is_solution: bool) -> EvalPlan {
    let mut projects = Vec::new();
    let configurations;
    let resolved;
    if is_solution {
        let text = fs::read_to_string(workspace_file).unwrap_or_default();
        let model = parse_solution(&text);
        configurations = model.configurations;
        // The scope comes from the workspace file itself: a project it names outside the root is
        // part of the workspace, and nothing else is. The same helper builds the outline's scope,
        // so the two can never disagree.
        resolved = crate::outline::resolve_workspace_scope(root, workspace_file, true);
        if workspace_file
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("slnx"))
        {
            // `.slnx` lists projects as XML, which the text parser above does not read: the project
            // list comes from the same parser the outline and the scope use.
            for (name, path) in crate::outline::solution_projects(workspace_file) {
                projects.push(plan_project(root, &resolved, name, path, None));
            }
        } else {
            for project in model.projects {
                projects.push(plan_project(
                    root,
                    &resolved,
                    project.name,
                    project.path,
                    project.folder,
                ));
            }
        }
    } else {
        configurations = vec!["Debug".to_string(), "Release".to_string()];
        let rel = relative_path(root, workspace_file);
        let name = workspace_file
            .file_stem()
            .map(|stem| stem.to_string_lossy().to_string())
            .unwrap_or_else(|| rel.clone());
        resolved = crate::outline::resolve_workspace_scope(root, workspace_file, false);
        projects.push(plan_project(root, &resolved, name, rel, None));
    }
    let mut configurations = configurations;
    if configurations.is_empty() {
        configurations.push(DEFAULT_CONFIGURATION.to_string());
    }
    EvalPlan {
        projects,
        configurations,
        scope: resolved.scope,
    }
}

fn plan_project(
    root: &Path,
    resolved: &ResolvedScope,
    name: String,
    rel: String,
    folder: Option<String>,
) -> PlanProject {
    let normalized = normalize_rel(&rel);
    let candidate = root.join(&normalized);
    let canonical = fs::canonicalize(&candidate).ok();
    let (abs_path, error) = match canonical {
        Some(path) if resolved.scope.contains(&path) => (Some(path), None),
        Some(path) => {
            let reason = resolved.refused.get(&path).cloned().unwrap_or_else(|| {
                format!("{normalized} is outside the workspace scope; it was not evaluated.")
            });
            (None, Some(reason))
        }
        None => (
            None,
            Some(format!(
                "{normalized} was not found on disk; it was not evaluated."
            )),
        ),
    };
    // A project outside the root is addressed by its canonical absolute path, so no `..` ever
    // crosses the host API; a project that could not be evaluated keeps the text the file used.
    let path = match &abs_path {
        Some(canonical) if !canonical.starts_with(root) => {
            canonical.to_string_lossy().replace('\\', "/")
        }
        _ => normalized,
    };
    PlanProject {
        name,
        path,
        abs_path,
        folder,
        error,
    }
}

pub fn watch_paths(
    workspace_file: &Path,
    root: &Path,
    scope: &Scope,
    projects: &[PlanProject],
    sources: &[String],
) -> Vec<Watched> {
    let mut paths: Vec<PathBuf> = Vec::new();
    paths.push(workspace_file.to_path_buf());
    for project in projects {
        if let Some(path) = &project.abs_path {
            paths.push(path.clone());
            if let Some(dir) = path.parent() {
                for name in [
                    "global.json",
                    "Directory.Build.props",
                    "Directory.Build.targets",
                    "Directory.Packages.props",
                ] {
                    paths.push(dir.join(name));
                }
            }
        }
    }
    for name in [
        "global.json",
        "Directory.Build.props",
        "Directory.Build.targets",
        "Directory.Packages.props",
    ] {
        paths.push(root.join(name));
    }

    // MSBuild's Compile items are a snapshot; a directory fingerprint makes an external
    // create/rename/delete flip `stale` without a second tree scanner. Build output is skipped
    // so a build does not mark its own evaluation stale.
    // shortcut: watches at most MAX_WATCHED_DIRS directories; raise it when a real solution needs more.
    let mut pending: Vec<PathBuf> = sources
        .iter()
        .filter_map(|source| root.join(source).parent().map(Path::to_path_buf))
        .chain(
            projects
                .iter()
                .filter_map(|project| project.abs_path.as_deref()?.parent().map(Path::to_path_buf)),
        )
        .collect();
    pending.sort();
    pending.dedup();
    let mut dirs: Vec<PathBuf> = Vec::new();
    while let Some(dir) = pending.pop() {
        if dirs.len() >= MAX_WATCHED_DIRS {
            break;
        }
        if !scope.contains(&dir) || !dir.is_dir() {
            continue;
        }
        dirs.push(dir.clone());
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let skip = name.to_string_lossy().starts_with('.')
                || name.eq_ignore_ascii_case("bin")
                || name.eq_ignore_ascii_case("obj");
            if !skip && entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                pending.push(entry.path());
            }
        }
    }
    paths.extend(dirs);

    let mut seen = BTreeSet::new();
    let mut watched = Vec::new();
    for path in paths {
        let key = path.to_string_lossy().to_string();
        if !seen.insert(key) {
            continue;
        }
        watched.push(Watched {
            fingerprint: fingerprint(&path),
            path,
        });
    }
    watched
}

/// Keeps the fingerprints taken before MSBuild and appends paths discovered only by this run,
/// so a file created while MSBuild was running is still a change on the next poll.
pub fn merge_watched(earlier: Vec<Watched>, later: Vec<Watched>) -> Vec<Watched> {
    let known: BTreeSet<String> = earlier
        .iter()
        .map(|watched| watched.path.to_string_lossy().to_string())
        .collect();
    let mut merged = earlier;
    for watched in later {
        if !known.contains(&watched.path.to_string_lossy().to_string()) {
            merged.push(watched);
        }
    }
    merged
}

/// One MSBuild process. Arguments are passed as a vector; nothing is interpolated into a shell.
pub fn evaluate_project(
    muxer: &Path,
    sdk_root: &Path,
    root: &Path,
    scope: &Scope,
    project: &PlanProject,
    configuration: &str,
    target_framework: Option<&str>,
) -> EvaluatedProject {
    let (restored, restored_reason) = match project.abs_path.as_deref() {
        Some(path) => crate::restore::state(path),
        None => (
            "unknown",
            Some("The project was not evaluated.".to_string()),
        ),
    };

    if let Some(error) = &project.error {
        return failed_project(
            project,
            error.clone(),
            None,
            String::new(),
            restored,
            restored_reason,
        );
    }
    let Some(abs_path) = project.abs_path.as_deref() else {
        return failed_project(
            project,
            "The project was not evaluated.".to_string(),
            None,
            String::new(),
            restored,
            restored_reason,
        );
    };

    let mut invoked = match invoke(muxer, sdk_root, root, abs_path, configuration, None) {
        Ok(invoked) => invoked,
        Err(failure) => {
            return failed_project(
                project,
                failure.error,
                failure.exit,
                failure.log,
                restored,
                restored_reason,
            )
        }
    };

    let mut tfms = invoked.result.tfms();
    let mut note = None;
    if tfms.len() > 1 {
        match target_framework {
            Some(selected) if tfms.iter().any(|tfm| tfm == selected) => {
                match invoke(
                    muxer,
                    sdk_root,
                    root,
                    abs_path,
                    configuration,
                    Some(selected),
                ) {
                    Ok(second) => {
                        invoked = second;
                        tfms = invoked.result.tfms();
                    }
                    Err(failure) => {
                        return failed_project(
                            project,
                            failure.error,
                            failure.exit,
                            failure.log,
                            restored,
                            restored_reason,
                        )
                    }
                }
            }
            Some(selected) => {
                note = Some(format!(
                    "Selected TFM {selected} is not declared by this project ({}); its TargetPath was not evaluated for that selection.",
                    tfms.join(", ")
                ));
            }
            None => {
                note = Some(format!(
                    "Select a TFM to evaluate sources and TargetPath (this project targets {}).",
                    tfms.join(", ")
                ));
            }
        }
    }

    let result = &invoked.result;
    let evaluated_tfm = result
        .target_framework
        .as_deref()
        .filter(|tfm| !tfm.trim().is_empty())
        .map(str::to_string);
    let target_path = result
        .target_path
        .as_deref()
        .filter(|path| !path.trim().is_empty())
        .map(str::to_string);
    let assembly_path = target_path.clone().or_else(|| {
        let dir = result
            .target_dir
            .as_deref()
            .filter(|dir| !dir.trim().is_empty())?;
        let name = result
            .target_file_name
            .as_deref()
            .filter(|name| !name.trim().is_empty())?;
        Some(PathBuf::from(dir).join(name).to_string_lossy().to_string())
    });
    let output_type = result
        .output_type
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string);
    let kind = project_kind(result.is_test_project.as_deref(), output_type.as_deref());

    let project_refs = result
        .project_refs
        .iter()
        .map(|(name, path)| ProjectRef {
            name: name.clone(),
            path: relative_path(root, Path::new(path)),
        })
        .collect();
    let package_refs = result
        .packages
        .iter()
        .map(|(name, version)| PackageRef {
            name: name.clone(),
            version: version.clone(),
        })
        .collect();
    let sources = result
        .compile
        .iter()
        .filter(|path| scope.contains(path))
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("cs"))
        })
        .filter(|path| !has_dir(path, scope, "bin") && !has_dir(path, scope, "obj"))
        .map(|path| relative_path(root, path))
        .collect();

    EvaluatedProject {
        name: project.name.clone(),
        path: project.path.clone(),
        folder: project.folder.clone(),
        state: "ok",
        kind,
        tfms,
        evaluated_tfm,
        target_path,
        assembly_path,
        output_type,
        project_refs,
        package_refs,
        sources,
        error: None,
        log: invoked.log,
        exit: invoked.exit,
        restored,
        restored_reason,
        note,
    }
}

fn failed_project(
    project: &PlanProject,
    error: String,
    exit: Option<i32>,
    log: String,
    restored: &'static str,
    restored_reason: Option<String>,
) -> EvaluatedProject {
    EvaluatedProject {
        name: project.name.clone(),
        path: project.path.clone(),
        folder: project.folder.clone(),
        state: "failed",
        kind: "unknown",
        tfms: Vec::new(),
        evaluated_tfm: None,
        target_path: None,
        assembly_path: None,
        output_type: None,
        project_refs: Vec::new(),
        package_refs: Vec::new(),
        sources: Vec::new(),
        error: Some(error),
        log: cap_log(&log),
        exit,
        restored,
        restored_reason,
        note: None,
    }
}

fn invoke(
    muxer: &Path,
    sdk_root: &Path,
    root: &Path,
    project: &Path,
    configuration: &str,
    target_framework: Option<&str>,
) -> Result<Invoked, InvokeError> {
    let started = Instant::now();
    let mut command = Command::new(muxer);
    command
        .arg("msbuild")
        .arg(project)
        .arg("-nologo")
        .arg("-getProperty:Configuration")
        .arg("-getProperty:TargetFramework")
        .arg("-getProperty:TargetFrameworks")
        .arg("-getProperty:TargetPath")
        .arg("-getProperty:TargetDir")
        .arg("-getProperty:TargetFileName")
        .arg("-getProperty:OutputType")
        .arg("-getProperty:MSBuildProjectName")
        .arg("-getProperty:IsTestProject")
        // Names the SDK the muxer actually picked, next to the one the snapshot resolved.
        .arg("-getProperty:NETCoreSdkVersion")
        .arg("-getItem:ProjectReference")
        .arg("-getItem:PackageReference")
        .arg("-getItem:Compile")
        .arg(format!("-p:Configuration={configuration}"))
        // The workspace root is the cwd of restore/build/run/test, so the muxer reads the same
        // global.json for evaluation as for everything else.
        .current_dir(root)
        .env("DOTNET_ROOT", sdk_root)
        .env("DOTNET_MULTILEVEL_LOOKUP", "0")
        .env("DOTNET_CLI_UI_LANGUAGE", "en")
        .env("DOTNET_NOLOGO", "1")
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .stdin(Stdio::null());
    if let Some(tfm) = target_framework {
        command.arg(format!("-p:TargetFramework={tfm}"));
    }

    let command_line = format!(
        "{} msbuild {} -nologo -getProperty:… -getItem:… -p:Configuration={}{}",
        muxer.to_string_lossy(),
        display_path(sdk_root, project),
        configuration,
        target_framework
            .map(|tfm| format!(" -p:TargetFramework={tfm}"))
            .unwrap_or_default()
    );
    let output = command.output().map_err(|error| InvokeError {
        error: format!("Could not start MSBuild: {error}"),
        exit: None,
        log: cap_log(&format!("$ {command_line}\nerror: {error}")),
    })?;
    let exit = output.status.code();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let elapsed = started.elapsed().as_millis();
    let log = cap_log(&format!(
        "$ {command_line}\nexit {}\n{} ms\n--- stdout\n{}\n--- stderr\n{}",
        exit.map(|code| code.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        elapsed,
        stdout.trim_end(),
        stderr.trim_end()
    ));

    if !output.status.success() {
        return Err(InvokeError {
            error: first_error_line(&stderr)
                .or_else(|| first_error_line(&stdout))
                .unwrap_or_else(|| {
                    format!(
                        "MSBuild exited with {}.",
                        exit.map(|code| code.to_string())
                            .unwrap_or_else(|| "an unknown status".to_string())
                    )
                }),
            exit,
            log,
        });
    }

    let result = parse_msbuild_json(&stdout).map_err(|error| InvokeError {
        error: format!("MSBuild did not return the expected JSON: {error}"),
        exit,
        log: log.clone(),
    })?;
    Ok(Invoked { result, exit, log })
}

pub fn parse_msbuild_json(stdout: &str) -> Result<MsbuildResult, String> {
    let value: Value =
        serde_json::from_str(stdout).map_err(|error| format!("invalid JSON: {error}"))?;
    let properties = value.get("Properties").cloned().unwrap_or(Value::Null);
    let items = value.get("Items").cloned().unwrap_or(Value::Null);

    let property = |name: &str| {
        properties
            .get(name)
            .and_then(Value::as_str)
            .filter(|text| !text.trim().is_empty())
            .map(str::to_string)
    };

    let item_array = |name: &str| -> Vec<Value> {
        match items.get(name) {
            Some(Value::Array(values)) => values.clone(),
            Some(Value::Object(_)) => vec![items.get(name).cloned().unwrap()],
            _ => Vec::new(),
        }
    };

    let project_refs = item_array("ProjectReference")
        .into_iter()
        .filter_map(|item| {
            let identity = item.get("Identity").and_then(Value::as_str)?;
            let full_path = item
                .get("FullPath")
                .and_then(Value::as_str)
                .filter(|path| !path.trim().is_empty());
            let name = item
                .get("Filename")
                .and_then(Value::as_str)
                .filter(|name| !name.trim().is_empty())
                .map(str::to_string)
                .or_else(|| {
                    Path::new(identity)
                        .file_stem()
                        .map(|stem| stem.to_string_lossy().to_string())
                })?;
            Some((name, full_path.unwrap_or(identity).to_string()))
        })
        .collect();

    let packages = item_array("PackageReference")
        .into_iter()
        .filter_map(|item| {
            let name = item.get("Identity").and_then(Value::as_str)?;
            let version = item
                .get("Version")
                .and_then(Value::as_str)
                .filter(|version| !version.trim().is_empty())
                .map(str::to_string);
            Some((name.to_string(), version))
        })
        .collect();

    let compile = item_array("Compile")
        .into_iter()
        .filter_map(|item| {
            item.get("FullPath")
                .and_then(Value::as_str)
                .filter(|path| !path.trim().is_empty())
                .map(PathBuf::from)
        })
        .collect();

    Ok(MsbuildResult {
        target_framework: property("TargetFramework"),
        target_frameworks: property("TargetFrameworks"),
        target_path: property("TargetPath"),
        target_dir: property("TargetDir"),
        target_file_name: property("TargetFileName"),
        output_type: property("OutputType"),
        is_test_project: property("IsTestProject"),
        project_refs,
        packages,
        compile,
    })
}

pub fn first_error_line(text: &str) -> Option<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .find(|line| line.to_ascii_lowercase().contains("error"))
        .map(str::to_string)
}

fn project_kind(is_test: Option<&str>, output_type: Option<&str>) -> &'static str {
    if is_test.is_some_and(|value| value.eq_ignore_ascii_case("true")) {
        return "test";
    }
    match output_type {
        Some("Exe") | Some("WinExe") => "console",
        Some("Library") => "library",
        _ => "unknown",
    }
}

fn has_dir(path: &Path, scope: &Scope, name: &str) -> bool {
    // Below the scope directory that owns the path, so a root or project directory that happens to
    // be called `bin`/`obj` does not hide every source of the project.
    let base = scope.entry_of(path).unwrap_or(path);
    path.strip_prefix(base)
        .unwrap_or(path)
        .components()
        .any(|component| component.as_os_str().eq_ignore_ascii_case(name))
}

fn cap_log(log: &str) -> String {
    if log.chars().count() <= MAX_LOG_CHARS {
        return log.to_string();
    }
    let skip = log.chars().count() - MAX_LOG_CHARS;
    let tail: String = log.chars().skip(skip).collect();
    format!("…log truncated…\n{tail}")
}

fn fingerprint(path: &Path) -> Fingerprint {
    let Ok(meta) = fs::metadata(path) else {
        return Fingerprint::File(None);
    };
    if meta.is_dir() {
        let mut names: Vec<String> = fs::read_dir(path)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                let skip = name.starts_with('.')
                    || name.eq_ignore_ascii_case("bin")
                    || name.eq_ignore_ascii_case("obj");
                let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
                let is_source = name.to_ascii_lowercase().ends_with(".cs");
                (!skip && (is_dir || is_source)).then_some(name)
            })
            .collect();
        names.sort();
        return Fingerprint::Dir(names);
    }
    let Some(modified) = meta
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
    else {
        return Fingerprint::File(None);
    };
    Fingerprint::File(Some((
        modified.as_secs(),
        u64::from(modified.subsec_nanos()),
        meta.len(),
    )))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

pub fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub fn display_path(root: &Path, path: &Path) -> String {
    relative_path(root, path)
}

fn normalize_rel(rel: &str) -> String {
    rel.replace('\\', "/").trim_start_matches("./").to_string()
}

fn quoted(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('"') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('"') else { break };
        values.push(after[..end].to_string());
        rest = &after[end + 1..];
    }
    values
}

fn normalize_guid(raw: &str) -> String {
    raw.trim()
        .trim_matches(|c| c == '{' || c == '}')
        .to_ascii_uppercase()
}

#[derive(Debug, Clone)]
pub struct SolutionProject {
    pub name: String,
    pub path: String,
    pub folder: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct SolutionModel {
    pub projects: Vec<SolutionProject>,
    pub configurations: Vec<String>,
}

const SOLUTION_FOLDER_TYPE: &str = "2150E333-8FDC-42A3-9474-1A3956D46DE8";

pub fn parse_solution(text: &str) -> SolutionModel {
    let mut entries: Vec<(String, String, String, String)> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("Project(") {
            continue;
        }
        let parts = quoted(trimmed);
        if parts.len() < 4 {
            continue;
        }
        entries.push((
            normalize_guid(&parts[0]),
            parts[1].clone(),
            parts[2].clone(),
            normalize_guid(&parts[3]),
        ));
    }

    let folders: BTreeMap<String, String> = entries
        .iter()
        .filter(|(kind, _, _, _)| kind == SOLUTION_FOLDER_TYPE)
        .map(|(_, name, _, guid)| (guid.clone(), name.clone()))
        .collect();

    let mut nested: BTreeMap<String, String> = BTreeMap::new();
    let mut in_nested = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("GlobalSection(NestedProjects)") {
            in_nested = true;
            continue;
        }
        if !in_nested {
            continue;
        }
        if trimmed == "EndGlobalSection" {
            in_nested = false;
            continue;
        }
        if let Some((child, parent)) = trimmed.split_once('=') {
            nested.insert(normalize_guid(child), normalize_guid(parent));
        }
    }

    let folder_path = |guid: &str| -> Option<String> {
        let mut names: Vec<String> = Vec::new();
        let mut current = guid.to_string();
        for _ in 0..8 {
            let Some(name) = folders.get(&current) else {
                break;
            };
            names.push(name.clone());
            match nested.get(&current) {
                Some(parent) if folders.contains_key(parent) => current = parent.clone(),
                _ => break,
            }
        }
        if names.is_empty() {
            return None;
        }
        names.reverse();
        Some(names.join("/"))
    };

    let projects = entries
        .iter()
        .filter(|(kind, _, path, _)| {
            kind != SOLUTION_FOLDER_TYPE && path.to_ascii_lowercase().ends_with(".csproj")
        })
        .map(|(_, name, path, guid)| SolutionProject {
            name: name.clone(),
            path: normalize_rel(path),
            folder: nested.get(guid).and_then(|parent| folder_path(parent)),
        })
        .collect();

    SolutionModel {
        projects,
        configurations: parse_configurations(text),
    }
}

pub fn parse_configurations(text: &str) -> Vec<String> {
    let mut configurations: Vec<String> = Vec::new();
    let mut in_section = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("GlobalSection(SolutionConfigurationPlatforms)") {
            in_section = true;
            continue;
        }
        if !in_section {
            continue;
        }
        if trimmed == "EndGlobalSection" {
            break;
        }
        let Some((left, _)) = trimmed.split_once('=') else {
            continue;
        };
        let Some((configuration, _)) = left.split_once('|') else {
            continue;
        };
        let configuration = configuration.trim();
        if !configuration.is_empty()
            && !configurations
                .iter()
                .any(|known| known.eq_ignore_ascii_case(configuration))
        {
            configurations.push(configuration.to_string());
        }
    }
    configurations
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLI_JSON: &str = r#"{
      "Properties": {
        "Configuration": "Debug",
        "TargetFramework": "net8.0",
        "TargetFrameworks": "",
        "TargetPath": "/work/Order Desk/src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll",
        "OutputType": "Exe",
        "MSBuildProjectName": "OrderDesk.Cli",
        "IsTestProject": ""
      },
      "Items": {
        "ProjectReference": [
          { "Identity": "../OrderDesk.Core/OrderDesk.Core.csproj",
            "FullPath": "/work/Order Desk/src/OrderDesk.Core/OrderDesk.Core.csproj",
            "Filename": "OrderDesk.Core" }
        ],
        "PackageReference": [],
        "Compile": [
          { "Identity": "Program.cs",
            "FullPath": "/work/Order Desk/src/OrderDesk.Cli/Program.cs" }
        ]
      }
    }"#;

    #[test]
    fn parses_the_verified_msbuild_json_shape() {
        let parsed = parse_msbuild_json(CLI_JSON).unwrap();
        assert_eq!(parsed.tfms(), vec!["net8.0"]);
        assert_eq!(
            parsed.target_path.as_deref(),
            Some("/work/Order Desk/src/OrderDesk.Cli/bin/Debug/net8.0/OrderDesk.Cli.dll")
        );
        assert_eq!(parsed.output_type.as_deref(), Some("Exe"));
        assert_eq!(parsed.project_refs.len(), 1);
        assert_eq!(parsed.project_refs[0].0, "OrderDesk.Core");
        assert_eq!(parsed.compile.len(), 1);
        assert!(parsed.packages.is_empty());
    }

    #[test]
    fn multi_tfm_outer_evaluation_has_no_target_path() {
        let json = r#"{"Properties":{"TargetFramework":"","TargetFrameworks":"net8.0;netstandard2.0","TargetPath":""},"Items":{}}"#;
        let parsed = parse_msbuild_json(json).unwrap();
        assert_eq!(parsed.tfms(), vec!["net8.0", "netstandard2.0"]);
        assert_eq!(parsed.target_path, None);
    }

    #[test]
    fn package_versions_may_be_unknown() {
        let json = r#"{"Properties":{},"Items":{"PackageReference":[
            {"Identity":"xunit","Version":"2.9.2"},
            {"Identity":"Central.Package","Version":""}]}}"#;
        let parsed = parse_msbuild_json(json).unwrap();
        assert_eq!(parsed.packages[0].1.as_deref(), Some("2.9.2"));
        assert_eq!(parsed.packages[1].1, None);
    }

    #[test]
    fn parses_solution_folders_configurations_and_skips_solution_items() {
        let text = "\u{feff}\nMicrosoft Visual Studio Solution File, Format Version 12.00\n\
Project(\"{2150E333-8FDC-42A3-9474-1A3956D46DE8}\") = \"src\", \"src\", \"{AAAA}\"\nEndProject\n\
Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Cli\", \"src\\Cli\\Cli.csproj\", \"{BBBB}\"\nEndProject\n\
Project(\"{2150E333-8FDC-42A3-9474-1A3956D46DE8}\") = \"tests\", \"tests\", \"{CCCC}\"\nEndProject\n\
Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Tests\", \"tests\\Tests.csproj\", \"{DDDD}\"\nEndProject\n\
Global\n\
\tGlobalSection(SolutionConfigurationPlatforms) = preSolution\n\
\t\tDebug|Any CPU = Debug|Any CPU\n\
\t\tRelease|Any CPU = Release|Any CPU\n\
\tEndGlobalSection\n\
\tGlobalSection(NestedProjects) = preSolution\n\
\t\t{BBBB} = {AAAA}\n\
\t\t{DDDD} = {CCCC}\n\
\tEndGlobalSection\n\
EndGlobal\n";
        let model = parse_solution(text);
        assert_eq!(model.configurations, vec!["Debug", "Release"]);
        assert_eq!(model.projects.len(), 2);
        assert_eq!(model.projects[0].path, "src/Cli/Cli.csproj");
        assert_eq!(model.projects[0].folder.as_deref(), Some("src"));
        assert_eq!(model.projects[1].folder.as_deref(), Some("tests"));
    }

    #[test]
    fn first_error_line_prefers_the_msbuild_error() {
        let stderr = "some warning\n/work/Broken.csproj(5,3): error MSB4019: The imported project \"/work/Missing.props\" was not found.\n";
        let line = first_error_line(stderr).unwrap();
        assert!(line.contains("MSB4019"));
        assert!(line.contains("Missing.props"));
    }

    #[test]
    fn project_kind_uses_evaluated_properties_only() {
        assert_eq!(project_kind(None, Some("Exe")), "console");
        assert_eq!(project_kind(None, Some("Library")), "library");
        assert_eq!(project_kind(Some("true"), Some("Exe")), "test");
        assert_eq!(project_kind(None, None), "unknown");
    }

    #[test]
    fn watch_paths_include_evaluated_source_directories_and_skip_build_output() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("src/App/Models")).unwrap();
        fs::create_dir_all(root.join("src/App/obj")).unwrap();
        fs::write(root.join("src/App/Program.cs"), "class P {}\n").unwrap();
        fs::write(root.join("src/App/Models/Order.cs"), "class O {}\n").unwrap();
        let workspace_file = root.join("App.sln");
        fs::write(&workspace_file, "").unwrap();
        let project_file = root.join("src/App/App.csproj");
        fs::write(&project_file, "<Project />\n").unwrap();

        let projects = [PlanProject {
            name: "App".into(),
            path: "src/App/App.csproj".into(),
            abs_path: Some(project_file),
            folder: None,
            error: None,
        }];
        let sources = [
            "src/App/Program.cs".to_string(),
            "src/App/Models/Order.cs".to_string(),
        ];
        let scope = Scope::new(root);
        let watched = watch_paths(&workspace_file, root, &scope, &projects, &sources);
        let paths: Vec<&Path> = watched
            .iter()
            .map(|watched| watched.path.as_path())
            .collect();

        assert!(paths.contains(&root.join("src/App").as_path()));
        assert!(paths.contains(&root.join("src/App/Models").as_path()));
        assert!(!paths.contains(&root.join("src/App/obj").as_path()));
        assert!(!paths.contains(&root.join("src/App/bin").as_path()));
    }

    #[test]
    fn a_solution_project_outside_the_root_joins_the_scope_and_a_missing_one_does_not() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("app");
        let lib = temp.path().join("lib");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&lib).unwrap();
        fs::write(lib.join("Lib.csproj"), "<Project />\n").unwrap();
        let solution = root.join("App.sln");
        fs::write(
            &solution,
            "Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Lib\", \"..\\lib\\Lib.csproj\", \"{1}\"\nEndProject\nProject(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Nope\", \"..\\nope\\Nope.csproj\", \"{2}\"\nEndProject\nGlobal\nEndGlobal\n",
        )
        .unwrap();
        let root = fs::canonicalize(&root).unwrap();
        let lib = fs::canonicalize(&lib).unwrap();

        let plan = plan_workspace(&solution, &root, true);
        assert_eq!(plan.projects.len(), 2);
        let outside = &plan.projects[0];
        assert_eq!(outside.name, "Lib");
        assert!(outside.error.is_none(), "{:?}", outside.error);
        assert_eq!(
            outside.abs_path.as_deref(),
            Some(lib.join("Lib.csproj").as_path())
        );
        assert_eq!(outside.path, lib.join("Lib.csproj").to_string_lossy());
        let missing = &plan.projects[1];
        assert!(missing.abs_path.is_none());
        assert!(
            missing
                .error
                .as_deref()
                .unwrap()
                .contains("was not found on disk"),
            "{:?}",
            missing.error
        );
        assert_eq!(plan.scope.roots(), &[root, lib]);
    }

    #[test]
    fn a_project_beyond_the_scope_cap_is_reported_with_the_reason() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("app");
        fs::create_dir_all(&root).unwrap();
        let mut text = String::new();
        for index in 0..=crate::scope::MAX_EXTRA_ROOTS {
            let dir = temp.path().join(format!("p{index}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("P.csproj"), "<Project />\n").unwrap();
            text.push_str(&format!(
                "Project(\"{{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}}\") = \"P{index}\", \"..\\p{index}\\P.csproj\", \"{{{index}}}\"\nEndProject\n"
            ));
        }
        let solution = root.join("App.sln");
        fs::write(&solution, format!("{text}Global\nEndGlobal\n")).unwrap();
        let root = fs::canonicalize(&root).unwrap();

        let plan = plan_workspace(&solution, &root, true);
        assert_eq!(plan.projects.len(), crate::scope::MAX_EXTRA_ROOTS + 1);
        let refused = plan.projects.last().unwrap();
        assert!(refused.abs_path.is_none());
        assert!(
            refused
                .error
                .as_deref()
                .unwrap()
                .contains(&crate::scope::MAX_EXTRA_ROOTS.to_string()),
            "{:?}",
            refused.error
        );
        assert_eq!(
            plan.scope.roots().len(),
            crate::scope::MAX_EXTRA_ROOTS + 1,
            "the root plus the accepted extras"
        );
    }

    #[test]
    fn merge_watched_keeps_earlier_fingerprints_and_adds_new_paths() {
        let early = Watched {
            path: PathBuf::from("/work/src"),
            fingerprint: Fingerprint::Dir(vec!["Order.cs".into()]),
        };
        let late = Watched {
            path: PathBuf::from("/work/src"),
            fingerprint: Fingerprint::Dir(vec!["Extra.cs".into(), "Order.cs".into()]),
        };
        let added = Watched {
            path: PathBuf::from("/work/src/Models"),
            fingerprint: Fingerprint::Dir(vec!["Cart.cs".into()]),
        };

        let merged = merge_watched(vec![early], vec![late, added]);
        assert_eq!(merged.len(), 2);
        assert_eq!(
            merged[0].fingerprint,
            Fingerprint::Dir(vec!["Order.cs".into()])
        );
        assert_eq!(merged[1].path, PathBuf::from("/work/src/Models"));
    }

    #[test]
    fn a_directory_fingerprint_ignores_content_edits_but_sees_create_and_delete() {
        let temp = tempfile::tempdir().unwrap();
        let dir = temp.path().join("src");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("Order.cs");
        fs::write(&file, "class Order {}\n").unwrap();

        // A plain save of an existing file keeps the directory entry unchanged.
        let before_edit = fingerprint(&dir);
        fs::write(&file, "class Order { int Id; }\n").unwrap();
        assert_eq!(fingerprint(&dir), before_edit);

        std::thread::sleep(std::time::Duration::from_millis(20));
        let before_create = fingerprint(&dir);
        fs::write(dir.join("Extra.cs"), "class Extra {}\n").unwrap();
        assert_ne!(fingerprint(&dir), before_create);

        std::thread::sleep(std::time::Duration::from_millis(20));
        let before_delete = fingerprint(&dir);
        fs::remove_file(dir.join("Extra.cs")).unwrap();
        assert_ne!(fingerprint(&dir), before_delete);
    }
}
