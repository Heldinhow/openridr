//! Saved run configurations, one file per solution in the app home.
//!
//! The store never needs workspace trust: it reads and writes only its own file, starts no
//! process and touches no repository file. The active configuration is the source of truth
//! for the launch, so the same `.env` parser and path rules are reused by `run.start` and
//! `debug.start` instead of a second pipeline.

use crate::error::{HostError, HostResult};
use crate::scope::Scope;
use crate::tools::app_home;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

const MAX_CONFIGS: usize = 50;
const MAX_ARGS: usize = 200;
const MAX_ENV: usize = 100;
const MAX_ENV_FILES: usize = 20;
const MAX_SESSION_TABS: usize = 200;
const MAX_SESSION_BREAKPOINTS: usize = 500;
const MAX_SESSION_POSITION: u64 = 1_000_000;

/// `write` and `setStartup` both read-modify-write the same file; one lock keeps concurrent
/// calls from dropping each other's change. `read_at`/`write_at` stay lock-free because the
/// public entry points already hold it (and tests call them directly).
static STORE_LOCK: Mutex<()> = Mutex::new(());

fn lock_store() -> MutexGuard<'static, ()> {
    STORE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunConfig {
    pub id: String,
    pub name: String,
    pub project: String,
    pub configuration: String,
    #[serde(default)]
    pub target_framework: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub working_directory: String,
    #[serde(default)]
    pub env: serde_json::Map<String, Value>,
    #[serde(default)]
    pub env_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Store {
    solution: String,
    active_id: String,
    configs: Vec<RunConfig>,
    /// Chosen from the toolbar or the Build panel; kept even while no named config selects it.
    #[serde(default)]
    startup_project: Option<String>,
    #[serde(default)]
    launch_profiles: BTreeMap<String, String>,
    /// Editor tabs, cursor and breakpoints last saved for this solution.
    #[serde(default)]
    session: SessionState,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionTab {
    pub path: String,
    pub line: u64,
    pub character: u64,
}

/// What the editor shows for a solution: tab paths (content is never persisted), the cursor of
/// each tab and the breakpoint lines per source (1-based, as the debugger reports them).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionState {
    pub tabs: Vec<SessionTab>,
    pub active_path: Option<String>,
    pub breakpoints: BTreeMap<String, Vec<u64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub config_id: String,
    pub field: &'static str,
    pub reason: &'static str,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct LaunchOptions {
    pub args: Vec<String>,
    pub working_directory: Option<PathBuf>,
    /// `.env` files in order, then the explicit environment; later assignments win.
    pub env: Vec<(String, String)>,
    /// What the launch profile asks the browser to do; manual configurations never open one.
    pub browser: crate::browser::Launch,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct LaunchProfile {
    #[serde(skip_deserializing)]
    name: String,
    #[serde(default, deserialize_with = "null_string")]
    command_name: String,
    #[serde(default)]
    application_url: Option<String>,
    #[serde(default)]
    launch_browser: bool,
    #[serde(default)]
    launch_url: Option<String>,
    #[serde(default)]
    environment_variables: BTreeMap<String, String>,
    #[serde(default, deserialize_with = "null_string")]
    command_line_args: String,
    #[serde(default)]
    working_directory: Option<String>,
}

fn null_string<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Deserialize)]
struct LaunchSettings {
    #[serde(default, deserialize_with = "deserialize_profiles")]
    profiles: Vec<LaunchProfile>,
}

fn deserialize_profiles<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<LaunchProfile>, D::Error> {
    struct ProfilesVisitor;
    impl<'de> serde::de::Visitor<'de> for ProfilesVisitor {
        type Value = Vec<LaunchProfile>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an object of launch profiles")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(
            self,
            mut map: M,
        ) -> Result<Self::Value, M::Error> {
            let mut profiles = Vec::new();
            while let Some((name, mut profile)) = map.next_entry::<String, LaunchProfile>()? {
                profile.name = name;
                profiles.push(profile);
            }
            Ok(profiles)
        }
    }
    deserializer.deserialize_map(ProfilesVisitor)
}

fn read_launch_profiles(project: &Path) -> HostResult<Vec<LaunchProfile>> {
    let path = project
        .parent()
        .unwrap_or(project)
        .join("Properties/launchSettings.json");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(HostError::new(
                "invalid_launch_settings",
                format!("{}: {error}", path.display()),
            ))
        }
    };
    let text = normalize_launch_json(&text).map_err(|error| {
        HostError::new(
            "invalid_launch_settings",
            format!("{}: {error}", path.display()),
        )
    })?;
    serde_json::from_str::<LaunchSettings>(&text)
        .map(|settings| settings.profiles)
        .map_err(|error| {
            HostError::new(
                "invalid_launch_settings",
                format!("{}: {error}", path.display()),
            )
        })
}

fn normalize_launch_json(text: &str) -> Result<String, &'static str> {
    let mut bytes = text
        .strip_prefix('\u{feff}')
        .unwrap_or(text)
        .as_bytes()
        .to_vec();
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;
    while index < bytes.len() {
        if in_string {
            if escaped {
                escaped = false;
            } else if bytes[index] == b'\\' {
                escaped = true;
            } else if bytes[index] == b'"' {
                in_string = false;
            }
            index += 1;
        } else if bytes[index] == b'"' {
            in_string = true;
            index += 1;
        } else if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            while index < bytes.len() && bytes[index] != b'\n' && bytes[index] != b'\r' {
                bytes[index] = b' ';
                index += 1;
            }
        } else if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            bytes[index] = b' ';
            bytes[index + 1] = b' ';
            index += 2;
            let mut closed = false;
            while index < bytes.len() {
                if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    bytes[index] = b' ';
                    bytes[index + 1] = b' ';
                    index += 2;
                    closed = true;
                    break;
                }
                if bytes[index] != b'\n' && bytes[index] != b'\r' {
                    bytes[index] = b' ';
                }
                index += 1;
            }
            if !closed {
                return Err("Unterminated block comment.");
            }
        } else {
            index += 1;
        }
    }
    index = 0;
    in_string = false;
    escaped = false;
    while index < bytes.len() {
        if in_string {
            if escaped {
                escaped = false;
            } else if bytes[index] == b'\\' {
                escaped = true;
            } else if bytes[index] == b'"' {
                in_string = false;
            }
        } else if bytes[index] == b'"' {
            in_string = true;
        } else if bytes[index] == b',' {
            let mut next = index + 1;
            while next < bytes.len() && bytes[next].is_ascii_whitespace() {
                next += 1;
            }
            let previous = bytes[..index]
                .iter()
                .rev()
                .find(|byte| !byte.is_ascii_whitespace());
            if matches!(bytes.get(next), Some(b'}' | b']'))
                && !matches!(previous, None | Some(b'[' | b'{' | b',' | b':'))
            {
                bytes[index] = b' ';
            }
        }
        index += 1;
    }
    String::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in launch settings.")
}

fn project_param(params: &Value, root: &Path) -> HostResult<PathBuf> {
    let raw = params
        .get("project")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("A project is required."))?;
    let path = fs::canonicalize(resolve_path(root, raw))
        .map_err(|error| invalid(format!("Project {raw}: {error}")))?;
    if path.extension().and_then(|value| value.to_str()) != Some("csproj") {
        return Err(invalid("The project must be a .csproj."));
    }
    Ok(path)
}

pub fn validate_evaluated_project<'a>(
    params: &Value,
    root: &Path,
    projects: impl Iterator<Item = &'a str>,
) -> HostResult<()> {
    let requested = project_param(params, root)?;
    if !projects
        .filter_map(|path| fs::canonicalize(resolve_path(root, path)).ok())
        .any(|path| path == requested)
    {
        return Err(invalid(
            "The project is not present in the current evaluation.",
        ));
    }
    Ok(())
}

fn profile_key(project: &Path) -> String {
    project.to_string_lossy().to_string()
}

fn stored_launch_profile<'a>(store: &'a Store, root: &Path, project: &Path) -> Option<&'a str> {
    store
        .launch_profiles
        .get(&profile_key(project))
        .map(String::as_str)
        .or_else(|| {
            store.launch_profiles.iter().find_map(|(key, value)| {
                fs::canonicalize(resolve_path(root, key))
                    .ok()
                    .filter(|path| path == project)
                    .map(|_| value.as_str())
            })
        })
}

fn selected_launch_profile<'a>(
    profiles: &'a [LaunchProfile],
    selected: Option<&str>,
) -> Option<&'a str> {
    profiles
        .iter()
        .find(|profile| {
            profile.command_name == "Project" && Some(profile.name.as_str()) == selected
        })
        .or_else(|| {
            profiles
                .iter()
                .find(|profile| profile.command_name == "Project")
        })
        .map(|profile| profile.name.as_str())
}

pub fn launch_profiles(params: &Value) -> HostResult<Value> {
    let _guard = lock_store();
    let solution = solution_param(params)?;
    let root = solution.parent().unwrap_or(&solution);
    let project = project_param(params, root)?;
    let profiles = read_launch_profiles(&project)?;
    let store = read_at(&path_for(&solution), &solution)?;
    let stored = stored_launch_profile(&store, root, &project);
    let selected = selected_launch_profile(&profiles, stored);
    Ok(json!({"profiles": profiles, "selected": selected}))
}

pub fn set_launch_profile(params: &Value) -> HostResult<Value> {
    let _guard = lock_store();
    let solution = solution_param(params)?;
    let project = project_param(params, solution.parent().unwrap_or(&solution))?;
    let name = params
        .get("profile")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid("A profile is required."))?;
    let profiles = read_launch_profiles(&project)?;
    if !profiles
        .iter()
        .any(|profile| profile.name == name && profile.command_name == "Project")
    {
        return Err(invalid(format!(
            "Launch profile {name} is not supported or no longer exists."
        )));
    }
    let path = path_for(&solution);
    let mut store = read_at(&path, &solution)?;
    let root = solution.parent().unwrap_or(&solution);
    store
        .launch_profiles
        .retain(|key, _| fs::canonicalize(resolve_path(root, key)).ok().as_ref() != Some(&project));
    store
        .launch_profiles
        .insert(profile_key(&project), name.to_string());
    store.active_id = "default".to_string();
    write_at(&path, &store)?;
    Ok(view(&store, &path, &solution))
}

/// Expands `%NAME%` and `$(NAME)` against the host environment in one pass. An empty or
/// undefined name keeps the original text (`%%`, `%NOPE%`, `$(NOPE)`), a substituted value is not
/// expanded again, and a lone `%` (a percentage) survives.
fn expand_value(value: &str) -> String {
    expand_value_with(value, |name| std::env::var(name).ok())
}

fn expand_value_with(value: &str, lookup: impl Fn(&str) -> Option<String>) -> String {
    let mut expanded = String::with_capacity(value.len());
    let mut rest = value;
    while let Some(index) = rest.find(['%', '$']) {
        expanded.push_str(&rest[..index]);
        let tail = &rest[index..];
        let marker = if let Some(after) = tail.strip_prefix('%') {
            after.find('%').map(|end| ("%", after, end, '%'))
        } else if let Some(after) = tail.strip_prefix("$(") {
            after.find(')').map(|end| ("$(", after, end, ')'))
        } else {
            None
        };
        match marker {
            Some((opening, after, end, closing)) => {
                let name = &after[..end];
                match lookup(name) {
                    Some(found) if !name.is_empty() => expanded.push_str(&found),
                    _ => {
                        expanded.push_str(opening);
                        expanded.push_str(name);
                        expanded.push(closing);
                    }
                }
                rest = &after[end + 1..];
            }
            None => {
                expanded.push(tail.chars().next().unwrap());
                rest = &tail[1..];
            }
        }
    }
    expanded.push_str(rest);
    expanded
}

/// Splits `commandLineArgs` with the rules the .NET CLI uses: `dotnet run` hands the string to
/// `ProcessStartInfo.Arguments`, which the child process splits with `CommandLineToArgvW`
/// semantics on every platform. Whitespace separates outside quotes, a run of backslashes only
/// escapes the quote that follows it (2n → n backslashes and the quote is a delimiter, 2n+1 → n
/// backslashes and a literal quote), `""` inside quotes is a literal quote, and an unclosed quote
/// is tolerated instead of refused. `#` and `'` are literal. The matrix in the tests was measured
/// against the real `dotnet run` (SDK 8.0.408).
fn split_launch_args(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut args = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;
    let mut index = 0;
    while index < chars.len() {
        match chars[index] {
            '\\' => {
                let mut count = 0;
                while chars.get(index + count) == Some(&'\\') {
                    count += 1;
                }
                if chars.get(index + count) == Some(&'"') {
                    current.extend(std::iter::repeat_n('\\', count / 2));
                    if count % 2 == 1 {
                        current.push('"');
                    } else {
                        quoted = !quoted;
                    }
                    index += count + 1;
                } else {
                    current.extend(std::iter::repeat_n('\\', count));
                    index += count;
                }
                started = true;
            }
            '"' => {
                if quoted && chars.get(index + 1) == Some(&'"') {
                    current.push('"');
                    index += 2;
                } else {
                    quoted = !quoted;
                    index += 1;
                }
                started = true;
            }
            value if value.is_whitespace() && !quoted => {
                if started {
                    args.push(std::mem::take(&mut current));
                    started = false;
                }
                index += 1;
            }
            value => {
                current.push(value);
                started = true;
                index += 1;
            }
        }
    }
    if started {
        args.push(current);
    }
    args
}

pub fn read(params: &Value) -> HostResult<Value> {
    let _guard = lock_store();
    let solution = solution_param(params)?;
    let path = path_for(&solution);
    let store = read_at(&path, &solution)?;
    Ok(view(&store, &path, &solution))
}

/// E2E isolation: the shared Playwright host sets this off so one spec cannot restore another
/// spec's tabs. The desktop app and the dedicated session spec leave it on.
fn session_store_enabled() -> bool {
    !matches!(
        std::env::var("OPENRIDR_SESSION_STORE").as_deref(),
        Ok("off")
    )
}

pub fn write(params: &Value) -> HostResult<Value> {
    let _guard = lock_store();
    let solution = solution_param(params)?;
    let path = path_for(&solution);
    let store = store_from_params(&path, &solution, params)?;
    write_at(&path, &store)?;
    Ok(view(&store, &path, &solution))
}

/// `runConfigs.setStartup`: the toolbar and the Build panel share one choice per solution.
pub fn set_startup(params: &Value) -> HostResult<Value> {
    let _guard = lock_store();
    let solution = solution_param(params)?;
    let path = path_for(&solution);
    let store = set_startup_at(&path, &solution, params)?;
    Ok(view(&store, &path, &solution))
}

/// `session.read`: same view as `runConfigs.read`, so the UI can refresh one store shape.
pub fn read_session(params: &Value) -> HostResult<Value> {
    read(params)
}

/// `session.write`: the editor writes paths and positions only; configurations are preserved.
pub fn write_session(params: &Value) -> HostResult<Value> {
    if !session_store_enabled() {
        return read(params);
    }
    let _guard = lock_store();
    let solution = solution_param(params)?;
    let path = path_for(&solution);
    let store = write_session_at(&path, &solution, params)?;
    Ok(view(&store, &path, &solution))
}

fn write_session_at(path: &Path, solution: &Path, params: &Value) -> HostResult<Store> {
    let mut store = read_at(path, solution)?;
    store.session = session_param(params, &workspace_scope_for(solution))?;
    write_at(path, &store)?;
    Ok(store)
}

/// The scope the session's paths must stay inside: the solution's root plus the directories of the
/// projects it names. A tab of a project outside the root is stored by its absolute path.
fn workspace_scope_for(solution: &Path) -> Scope {
    let root = solution.parent().unwrap_or(solution);
    let is_solution = solution.extension().is_some_and(|extension| {
        extension.eq_ignore_ascii_case("sln") || extension.eq_ignore_ascii_case("slnx")
    });
    crate::outline::resolve_workspace_scope(root, solution, is_solution).scope
}

fn store_from_params(path: &Path, solution: &Path, params: &Value) -> HostResult<Store> {
    let active_id = params
        .get("activeId")
        .and_then(Value::as_str)
        .unwrap_or("default")
        .to_string();
    let configs = params
        .get("configs")
        .cloned()
        .map(serde_json::from_value::<Vec<RunConfig>>)
        .transpose()
        .map_err(|error| {
            HostError::new(
                "invalid_run_configs",
                format!("configs is not a valid run configuration list: {error}"),
            )
        })?
        .unwrap_or_default();
    validate(&configs, &active_id)?;
    // A write that only edits configurations must not erase the persisted startup choice.
    let startup_project = if params.get("startupProject").is_some() {
        startup_param(params)?
    } else {
        read_at(path, solution)
            .ok()
            .and_then(|store| store.startup_project)
    };
    // Likewise, editing configurations never changes the editor session.
    let session = read_at(path, solution)
        .ok()
        .map(|store| store.session)
        .unwrap_or_default();
    let launch_profiles = read_at(path, solution)
        .ok()
        .map(|store| store.launch_profiles)
        .unwrap_or_default();
    Ok(Store {
        solution: solution.to_string_lossy().to_string(),
        active_id,
        configs,
        startup_project,
        launch_profiles,
        session,
    })
}

fn session_param(params: &Value, scope: &Scope) -> HostResult<SessionState> {
    let session = params
        .get("session")
        .cloned()
        .map(serde_json::from_value::<SessionState>)
        .transpose()
        .map_err(|error| invalid(format!("session is not a valid editor session: {error}")))?
        .unwrap_or_default();
    validate_session(&session, scope)?;
    Ok(session)
}

fn validate_session(session: &SessionState, scope: &Scope) -> HostResult<()> {
    if session.tabs.len() > MAX_SESSION_TABS {
        return Err(invalid(format!(
            "At most {MAX_SESSION_TABS} editor tabs are supported."
        )));
    }
    for tab in &session.tabs {
        validate_session_path(&tab.path, scope)?;
        if tab.line >= MAX_SESSION_POSITION || tab.character >= MAX_SESSION_POSITION {
            return Err(invalid(
                "A tab cursor position is out of range; nothing was saved.",
            ));
        }
    }
    if let Some(path) = &session.active_path {
        validate_session_path(path, scope)?;
    }
    for (path, lines) in &session.breakpoints {
        validate_session_path(path, scope)?;
        if lines.len() > MAX_SESSION_BREAKPOINTS
            || lines
                .iter()
                .any(|line| *line == 0 || *line >= MAX_SESSION_POSITION)
        {
            return Err(invalid(format!(
                "Breakpoints per source must be 1..{MAX_SESSION_POSITION}, at most {MAX_SESSION_BREAKPOINTS}; nothing was saved."
            )));
        }
    }
    Ok(())
}

fn validate_session_path(path: &str, scope: &Scope) -> HostResult<()> {
    if path.trim().is_empty() || path.len() > 1024 || path.contains('\0') {
        return Err(invalid(
            "Session paths must be non-empty and at most 1024 characters; nothing was saved.",
        ));
    }
    // A file of a project the solution names outside its directory is stored by its absolute path;
    // it must stay inside the workspace scope, so the session cannot reopen something the host
    // would refuse anyway.
    if Path::new(path).is_absolute() && !scope.contains(Path::new(path)) {
        return Err(invalid(
            "A session path outside the workspace scope was refused; nothing was saved.",
        ));
    }
    Ok(())
}

fn set_startup_at(path: &Path, solution: &Path, params: &Value) -> HostResult<Store> {
    let mut store = read_at(path, solution)?;
    store.startup_project = startup_param(params)?;
    write_at(path, &store)?;
    Ok(store)
}

fn startup_param(params: &Value) -> HostResult<Option<String>> {
    match params.get("startupProject") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => {
            let value = value.trim();
            if value.is_empty() {
                return Ok(None);
            }
            if value.len() > 1024 || value.contains('\0') {
                return Err(invalid(
                    "startupProject must be a project path of at most 1024 characters.",
                ));
            }
            Ok(Some(value.to_string()))
        }
        Some(_) => Err(invalid("startupProject must be a string or null.")),
    }
}

/// Parses the launch options shared by `run.start` and `debug.start`. Paths are relative to
/// the solution root, argv is never split by a shell, and the working directory must exist
/// before anything is spawned.
pub fn launch_options(params: &Value, root: &Path) -> HostResult<LaunchOptions> {
    if let Some(name) = params.get("launchProfile").and_then(Value::as_str) {
        let project = project_param(params, root)?;
        let profile = read_launch_profiles(&project)?
            .into_iter()
            .find(|profile| profile.name == name)
            .ok_or_else(|| {
                invalid(format!(
                    "Launch profile {name} was not found in {}.",
                    project.display()
                ))
            })?;
        if profile.command_name != "Project" {
            return Err(invalid(format!(
                "Launch profile {name} uses unsupported commandName {}.",
                profile.command_name
            )));
        }
        // `%VAR%` is what `dotnet run` expands (in environment values only); `$(VAR)` is the
        // Visual Studio/Rider form the same issue asks for. Both read the host's environment,
        // like `Environment.ExpandEnvironmentVariables`: a profile value cannot see another.
        let mut env: BTreeMap<String, String> = profile
            .environment_variables
            .into_iter()
            .map(|(key, value)| (key, expand_value(&value)))
            .collect();
        if let Some(url) = profile.application_url {
            env.entry("ASPNETCORE_URLS".to_string())
                .or_insert_with(|| expand_value(&url));
        }
        // The profile's own `environmentVariables` win over `applicationUrl`, so the effective
        // list is what the app will actually listen on.
        let browser = crate::browser::plan(
            profile.launch_browser,
            profile.launch_url.as_deref(),
            env.get("ASPNETCORE_URLS").map(String::as_str),
        );
        let project_dir = project.parent().unwrap_or(root);
        let cwd = profile
            .working_directory
            .as_deref()
            .map(|value| resolve_path(project_dir, value));
        if let Some(path) = &cwd {
            if !path.is_dir() {
                return Err(option_error("workingDirectory", "working_directory_missing", format!("The working directory {} does not exist or is not a directory; fix workingDirectory in launchSettings.json before running.", path.display())));
            }
        }
        let mut options = launch_options(
            &json!({"args": split_launch_args(&profile.command_line_args), "env": env, "workingDirectory": cwd}),
            root,
        )?;
        options.browser = browser;
        return Ok(options);
    }
    let args = string_list(params.get("args"), "args", MAX_ARGS)?;
    for argument in &args {
        if argument.contains('\0') {
            return Err(option_error(
                "args",
                "argument_invalid",
                "Arguments cannot contain NUL bytes; no program was started.",
            ));
        }
    }

    let working_directory = match params.get("workingDirectory").and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => {
            let path = resolve_path(root, value);
            if !path.is_dir() {
                return Err(option_error(
                    "workingDirectory",
                    "working_directory_missing",
                    format!(
                        "The working directory {} does not exist or is not a directory; fix the run configuration before running.",
                        path.to_string_lossy()
                    ),
                ));
            }
            Some(fs::canonicalize(&path).unwrap_or(path))
        }
        _ => None,
    };

    let env_files = string_list(params.get("envFiles"), "envFiles", MAX_ENV_FILES)?;
    let mut env = Vec::new();
    for file in &env_files {
        if file.trim().is_empty() {
            return Err(option_error(
                "envFiles",
                "env_file_invalid",
                "An .env file path is empty; remove it or fix the run configuration.",
            ));
        }
        let path = resolve_path(root, file);
        let text = fs::read_to_string(&path).map_err(|error| {
            option_error(
                "envFiles",
                "env_file_missing",
                format!(
                    "Could not read the .env file {} ({error}); fix the run configuration before running.",
                    path.to_string_lossy()
                ),
            )
        })?;
        let values = parse_env(&text).map_err(|message| {
            option_error(
                "envFiles",
                "env_file_invalid",
                format!(
                    "{} {message} No program was started.",
                    path.to_string_lossy()
                ),
            )
        })?;
        env.extend(values);
    }

    if let Some(value) = params.get("env") {
        let object = value.as_object().ok_or_else(|| {
            option_error(
                "env",
                "env_invalid",
                "Environment variables must be an object of KEY: \"VALUE\" pairs.",
            )
        })?;
        if object.len() > MAX_ENV {
            return Err(option_error(
                "env",
                "env_invalid",
                format!("At most {MAX_ENV} environment variables are supported."),
            ));
        }
        for (key, value) in object {
            if !valid_env_name(key) {
                return Err(option_error(
                    "env",
                    "env_invalid",
                    format!(
                        "\"{key}\" is not a valid environment variable name; use letters, digits and underscores, starting with a letter or underscore."
                    ),
                ));
            }
            let value = value.as_str().ok_or_else(|| {
                option_error(
                    "env",
                    "env_invalid",
                    format!("The value of {key} must be a string."),
                )
            })?;
            if value.contains('\0') {
                return Err(option_error(
                    "env",
                    "env_invalid",
                    format!("The value of {key} cannot contain NUL bytes."),
                ));
            }
            env.push((key.clone(), value.to_string()));
        }
    }

    Ok(LaunchOptions {
        args,
        working_directory,
        env,
        browser: crate::browser::Launch::None,
    })
}

/// `KEY=VALUE` per line, full-line `#` comments, no expansion and no shell. The first `=`
/// splits the pair, so values may contain `=`. Surrounding whitespace is trimmed.
pub fn parse_env(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut values = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("line {}: expected KEY=VALUE.", index + 1));
        };
        let key = key.trim();
        if !valid_env_name(key) {
            return Err(format!(
                "line {}: \"{key}\" is not a valid environment variable name.",
                index + 1
            ));
        }
        values.push((key.to_string(), value.trim().to_string()));
    }
    Ok(values)
}

/// Deterministic 64-bit FNV-1a over the canonical solution path, used as the file name.
pub fn fnv1a64(value: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

pub fn path_for(solution: &Path) -> PathBuf {
    let key = solution.to_string_lossy();
    app_home()
        .join("run-configs")
        .join(format!("{:016x}.json", fnv1a64(key.as_bytes())))
}

fn solution_param(params: &Value) -> HostResult<PathBuf> {
    let raw = params
        .get("solution")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            HostError::new("invalid_run_configs", "runConfigs needs the solution path.")
        })?;
    let path = Path::new(raw);
    if !path.is_absolute() {
        return Err(HostError::new(
            "invalid_run_configs",
            "The solution path must be absolute.",
        ));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| HostError::new("invalid_run_configs", format!("Solution not found: {raw}")))?;
    if !matches!(
        canonical
            .extension()
            .and_then(|extension| extension.to_str()),
        Some("sln") | Some("slnx") | Some("csproj")
    ) {
        return Err(HostError::new(
            "invalid_run_configs",
            "Run configurations are stored for a .sln, .slnx or .csproj.",
        ));
    }
    Ok(canonical)
}

fn read_at(path: &Path, solution: &Path) -> HostResult<Store> {
    let empty = Store {
        solution: solution.to_string_lossy().to_string(),
        active_id: "default".to_string(),
        configs: Vec::new(),
        startup_project: None,
        launch_profiles: BTreeMap::new(),
        session: SessionState::default(),
    };
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(empty),
        Err(error) => {
            return Err(HostError::new(
                "invalid_run_configs",
                format!("Could not read {}: {error}", path.to_string_lossy()),
            ))
        }
    };
    let store: Store = serde_json::from_str(&text).map_err(|error| {
        HostError::new(
            "invalid_run_configs",
            format!(
                "{} is not a valid run configuration file: {error}",
                path.to_string_lossy()
            ),
        )
    })?;
    if store.solution != solution.to_string_lossy() {
        // The hash key is stable; a different solution here means a copied file, never the
        // current one. Returning empty avoids showing another solution's configurations.
        return Ok(empty);
    }
    let mut store = store;
    if store.active_id != "default"
        && !store
            .configs
            .iter()
            .any(|config| config.id == store.active_id)
    {
        // A stale active id from an older file is not a reason to lose the configurations.
        store.active_id = "default".to_string();
    }
    validate(&store.configs, &store.active_id)?;
    Ok(store)
}

fn write_at(path: &Path, store: &Store) -> HostResult<()> {
    crate::fsutil::write_json_atomic(path, store, "invalid_run_configs")
}

fn view(store: &Store, path: &Path, solution: &Path) -> Value {
    json!({
        "solution": solution.to_string_lossy(),
        "path": path.to_string_lossy(),
        "activeId": &store.active_id,
        "startupProject": &store.startup_project,
        "configs": &store.configs,
        "session": if session_store_enabled() {
            json!(&store.session)
        } else {
            json!(SessionState::default())
        },
        "issues": issues(&store.configs, solution.parent().unwrap_or(Path::new("/"))),
    })
}

fn validate(configs: &[RunConfig], active_id: &str) -> HostResult<()> {
    if configs.len() > MAX_CONFIGS {
        return Err(invalid("At most 50 run configurations are supported."));
    }
    let mut ids = HashSet::new();
    let mut names = HashSet::new();
    for config in configs {
        if config.id.trim().is_empty() || config.id.len() > 80 {
            return Err(shape_error(
                "id",
                "id_required",
                "Every run configuration needs an id of at most 80 characters.",
            ));
        }
        if !ids.insert(config.id.clone()) {
            return Err(shape_error(
                "id",
                "duplicate_id",
                "Two run configurations share an id; nothing was saved.",
            ));
        }
        let name = config.name.trim();
        if name.is_empty() || config.name.len() > 120 {
            return Err(shape_error(
                "name",
                "name_required",
                "Every run configuration needs a name of at most 120 characters.",
            ));
        }
        if !names.insert(name.to_lowercase()) {
            return Err(shape_error(
                "name",
                "duplicate_name",
                format!(
                    "Another run configuration is already named \"{name}\"; names must be unique."
                ),
            ));
        }
        if config.project.trim().is_empty() || config.project.len() > 1024 {
            return Err(shape_error(
                "project",
                "project_required",
                "Every run configuration needs a project.",
            ));
        }
        if config.configuration.trim().is_empty() || config.configuration.len() > 80 {
            return Err(shape_error(
                "configuration",
                "configuration_required",
                "Every run configuration needs a Configuration.",
            ));
        }
        if config
            .target_framework
            .as_deref()
            .is_some_and(|tfm| tfm.len() > 64)
        {
            return Err(shape_error(
                "targetFramework",
                "tfm_invalid",
                "The TFM is too long.",
            ));
        }
        if config.args.len() > MAX_ARGS {
            return Err(shape_error(
                "args",
                "args_invalid",
                format!("At most {MAX_ARGS} arguments are supported."),
            ));
        }
        if config.args.iter().any(|argument| argument.contains('\0')) {
            return Err(shape_error(
                "args",
                "args_invalid",
                "Arguments cannot contain NUL bytes.",
            ));
        }
        if config.working_directory.len() > 1024 {
            return Err(shape_error(
                "workingDirectory",
                "working_directory_invalid",
                "The working directory path is too long.",
            ));
        }
        if config.env.len() > MAX_ENV {
            return Err(shape_error(
                "env",
                "env_invalid",
                format!("At most {MAX_ENV} environment variables are supported."),
            ));
        }
        for (key, value) in &config.env {
            if !valid_env_name(key) {
                return Err(shape_error(
                    "env",
                    "env_invalid",
                    format!("\"{key}\" is not a valid environment variable name."),
                ));
            }
            let Some(value) = value.as_str() else {
                return Err(shape_error(
                    "env",
                    "env_invalid",
                    format!("The value of {key} must be a string."),
                ));
            };
            if value.contains('\0') {
                return Err(shape_error(
                    "env",
                    "env_invalid",
                    format!("The value of {key} cannot contain NUL bytes."),
                ));
            }
        }
        if config.env_files.len() > MAX_ENV_FILES {
            return Err(shape_error(
                "envFiles",
                "env_invalid",
                format!("At most {MAX_ENV_FILES} .env files are supported."),
            ));
        }
        if config
            .env_files
            .iter()
            .any(|file| file.trim().is_empty() || file.len() > 1024)
        {
            return Err(shape_error(
                "envFiles",
                "env_invalid",
                "Every .env file needs a non-empty path of at most 1024 characters.",
            ));
        }
    }
    if active_id != "default" && !active_id.is_empty() && !ids.contains(active_id) {
        return Err(shape_error(
            "activeId",
            "active_missing",
            "The active run configuration is not in the list; nothing was saved.",
        ));
    }
    Ok(())
}

fn issues(configs: &[RunConfig], root: &Path) -> Vec<Issue> {
    let mut issues = Vec::new();
    for config in configs {
        let directory = config.working_directory.trim();
        if !directory.is_empty() {
            let path = resolve_path(root, directory);
            if !path.is_dir() {
                issues.push(Issue {
                    config_id: config.id.clone(),
                    field: "workingDirectory",
                    reason: "working_directory_missing",
                    message: format!(
                        "The working directory {} does not exist or is not a directory.",
                        path.to_string_lossy()
                    ),
                });
            }
        }
        for file in &config.env_files {
            let path = resolve_path(root, file);
            match fs::read_to_string(&path) {
                Ok(text) => {
                    if let Err(message) = parse_env(&text) {
                        issues.push(Issue {
                            config_id: config.id.clone(),
                            field: "envFiles",
                            reason: "env_file_invalid",
                            message: format!("{} {message}", path.to_string_lossy()),
                        });
                    }
                }
                Err(error) => issues.push(Issue {
                    config_id: config.id.clone(),
                    field: "envFiles",
                    reason: "env_file_missing",
                    message: format!(
                        "Could not read the .env file {} ({error}).",
                        path.to_string_lossy()
                    ),
                }),
            }
        }
    }
    issues
}

fn invalid(message: impl Into<String>) -> HostError {
    HostError::new("invalid_run_configs", message)
}

fn shape_error(field: &'static str, reason: &'static str, message: impl Into<String>) -> HostError {
    HostError::with_details(
        "invalid_run_configs",
        message,
        json!({ "field": field, "reason": reason }),
    )
}

fn option_error(
    field: &'static str,
    reason: &'static str,
    message: impl Into<String>,
) -> HostError {
    HostError::with_details(
        "invalid_state",
        message,
        json!({ "field": field, "reason": reason }),
    )
}

fn resolve_path(root: &Path, value: &str) -> PathBuf {
    let path = Path::new(value);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn string_list(value: Option<&Value>, field: &'static str, max: usize) -> HostResult<Vec<String>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let list = value.as_array().ok_or_else(|| {
        option_error(
            field,
            "value_invalid",
            format!("{field} must be an array of strings."),
        )
    })?;
    if list.len() > max {
        return Err(option_error(
            field,
            "value_invalid",
            format!("{field} supports at most {max} entries."),
        ));
    }
    list.iter()
        .map(|item| {
            item.as_str().map(str::to_string).ok_or_else(|| {
                option_error(
                    field,
                    "value_invalid",
                    format!("{field} must contain only strings."),
                )
            })
        })
        .collect()
}

fn valid_env_name(name: &str) -> bool {
    let mut characters = name.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    characters.all(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile_fixture(text: &str) -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("App.csproj");
        fs::write(&project, "").unwrap();
        fs::create_dir(temp.path().join("Properties")).unwrap();
        fs::write(temp.path().join("Properties/launchSettings.json"), text).unwrap();
        (temp, project)
    }

    #[test]
    fn launch_profiles_accept_bom_comments_and_trailing_commas() {
        let (_temp, project) = profile_fixture("\u{feff}{\n// comment\n\"profiles\": { /* block */ \"http\": {\"commandName\":\"Project\",\"applicationUrl\":\"http://localhost:1/a/*b*/,//c\",},},}\n");
        let profiles = read_launch_profiles(&project).unwrap();
        assert_eq!(
            profiles[0].application_url.as_deref(),
            Some("http://localhost:1/a/*b*/,//c")
        );
        assert!(
            serde_json::from_str::<Value>(&normalize_launch_json("{\"items\":[,]}").unwrap())
                .is_err()
        );
        assert!(normalize_launch_json("{/* unterminated").is_err());
        assert!(serde_json::from_str::<Value>(
            &normalize_launch_json("{\"items\":[1, /* comment */ ]}").unwrap()
        )
        .is_ok());
    }

    #[test]
    fn launch_profiles_accept_null_command_fields() {
        let (_temp, project) = profile_fixture(
            r#"{"profiles":{"empty":{"commandName":null,"commandLineArgs":null},"ok":{"commandName":"Project","commandLineArgs":null}}}"#,
        );
        let profiles = read_launch_profiles(&project).unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[1].name, "ok");
        assert_eq!(selected_launch_profile(&profiles, None), Some("ok"));
        let options = launch_options(
            &json!({"project":project,"launchProfile":"ok"}),
            project.parent().unwrap(),
        )
        .unwrap();
        assert!(options.args.is_empty());
    }

    #[test]
    fn launch_profile_environment_overrides_application_url() {
        let (temp, project) = profile_fixture(
            r#"{"profiles":{"http":{"commandName":"Project","applicationUrl":"http://original","environmentVariables":{"ASPNETCORE_URLS":"http://override"}}}}"#,
        );
        let options = launch_options(
            &json!({"project":project,"launchProfile":"http"}),
            temp.path(),
        )
        .unwrap();
        assert!(options
            .env
            .contains(&("ASPNETCORE_URLS".into(), "http://override".into())));
    }

    #[test]
    fn launch_profile_can_resolve_project_outside_solution_directory() {
        let (temp, project) = profile_fixture(r#"{"profiles":{"http":{"commandName":"Project"}}}"#);
        let root = temp.path().join("solution");
        fs::create_dir(&root).unwrap();
        let options = launch_options(
            &json!({"project":"../App.csproj","launchProfile":"http"}),
            &root,
        )
        .unwrap();
        assert!(options.args.is_empty());
        assert_eq!(
            project_param(&json!({"project":"../App.csproj"}), &root).unwrap(),
            fs::canonicalize(project).unwrap()
        );
        let params = json!({"project":"../App.csproj"});
        assert!(validate_evaluated_project(&params, &root, ["../App.csproj"].into_iter()).is_ok());
        assert!(
            validate_evaluated_project(&params, &root, ["Unrelated.csproj"].into_iter()).is_err()
        );
    }

    #[test]
    fn launch_profile_missing_cwd_names_launch_settings() {
        let (temp, project) = profile_fixture(
            r#"{"profiles":{"http":{"commandName":"Project","workingDirectory":"missing"}}}"#,
        );
        let error = launch_options(
            &json!({"project":project,"launchProfile":"http"}),
            temp.path(),
        )
        .unwrap_err();
        assert!(
            error.message.contains("workingDirectory")
                && error.message.contains("launchSettings.json"),
            "{}",
            error.message
        );
    }

    #[test]
    fn launch_args_follow_the_dotnet_command_line_rules() {
        // Measured against the real `dotnet run` (SDK 8.0.408, macOS ARM64): each row is the
        // profile's commandLineArgs and the argv the program received. Before the fix the
        // splitter refused an unclosed quote and turned `""`, `""""` and backslash runs into
        // different arguments, so most of these rows failed.
        let matrix: &[(&str, Vec<&str>)] = &[
            ("a \"b c\" d", vec!["a", "b c", "d"]),
            ("a \"b\\\"c\" d", vec!["a", "b\"c", "d"]),
            ("a \"b\\\\\" c", vec!["a", "b\\", "c"]),
            ("a \"\" b", vec!["a", "", "b"]),
            ("a \"\"\"\" b", vec!["a", "\"", "b"]),
            ("a b\"c d\"e", vec!["a", "bc de"]),
            ("a b\\\\c", vec!["a", "b\\\\c"]),
            ("a \"b c", vec!["a", "b c"]),
            ("a #b c", vec!["a", "#b", "c"]),
            ("", vec![]),
            ("   ", vec![]),
            ("a\tb", vec!["a", "b"]),
            ("'a b' c", vec!["'a", "b'", "c"]),
            ("a b\\\\", vec!["a", "b\\\\"]),
            ("\"\"", vec![""]),
            ("a \"\"\" b", vec!["a", "\" b"]),
            ("a \\\"b\\\"", vec!["a", "\"b\""]),
            ("a \"b\"c", vec!["a", "bc"]),
            ("a b\"c\"", vec!["a", "bc"]),
            ("a \"b\"\"c\"", vec!["a", "b\"c"]),
            ("a \"b\\\\\\\"c\"", vec!["a", "b\\\"c"]),
            ("a b\\\\\\\\\"c\"", vec!["a", "b\\\\c"]),
            ("\"a b\"c d", vec!["a bc", "d"]),
            ("a  b", vec!["a", "b"]),
            ("a \"b\\\\\"c", vec!["a", "b\\c"]),
            ("a\\\\\"b c\"", vec!["a\\b c"]),
            ("a \"\"\"\"\"\" b", vec!["a", "\"\"", "b"]),
            ("a \\", vec!["a", "\\"]),
            ("a \"b\" \"c\"", vec!["a", "b", "c"]),
            ("a \"b\"c\"d\"", vec!["a", "bcd"]),
            ("a \"b\"\"", vec!["a", "b\""]),
            ("a \"\"\"", vec!["a", "\""]),
            ("a \"\"\"\" \"", vec!["a", "\"", ""]),
            ("a \"b\" \"\"", vec!["a", "b", ""]),
            ("a \"\"\"\"\"\"", vec!["a", "\"\""]),
            ("a \"b c\"\"d e\"", vec!["a", "b c\"d e"]),
            ("a\\ b", vec!["a\\", "b"]),
            ("a \\\\ b", vec!["a", "\\\\", "b"]),
            ("\"a\"b\"c\"d", vec!["abcd"]),
            ("a \"b\\\\\" c\"", vec!["a", "b\\", "c"]),
        ];
        for (input, expected) in matrix {
            assert_eq!(
                split_launch_args(input),
                expected.clone(),
                "commandLineArgs {input:?}"
            );
        }
        // The profile's command line is never expanded: `%VAR%` and `$(VAR)` reach the program.
        assert_eq!(
            split_launch_args("%HOME% $(HOME)"),
            vec!["%HOME%", "$(HOME)"]
        );
    }

    #[test]
    fn missing_selected_profile_uses_first_project_profile() {
        let (_temp, project) = profile_fixture(
            r#"{"profiles":{"iis":{"commandName":"IISExpress"},"first":{"commandName":"Project"},"second":{"commandName":"Project"}}}"#,
        );
        let profiles = read_launch_profiles(&project).unwrap();
        assert_eq!(
            selected_launch_profile(&profiles, Some("removed")),
            Some("first")
        );
        assert_eq!(
            selected_launch_profile(&profiles, Some("second")),
            Some("second")
        );
    }

    #[test]
    fn launch_profile_key_is_canonical_across_relative_and_absolute_paths() {
        let (temp, project) = profile_fixture(r#"{"profiles":{}}"#);
        let relative = project_param(&json!({"project":"./App.csproj"}), temp.path()).unwrap();
        let absolute = project_param(&json!({"project":project}), temp.path()).unwrap();
        assert_eq!(profile_key(&relative), profile_key(&absolute));
        let mut store = Store {
            solution: String::new(),
            active_id: "default".into(),
            configs: Vec::new(),
            startup_project: None,
            launch_profiles: BTreeMap::new(),
            session: SessionState::default(),
        };
        store
            .launch_profiles
            .insert("App.csproj".into(), "http".into());
        assert_eq!(
            stored_launch_profile(&store, temp.path(), &absolute),
            Some("http")
        );
    }

    fn run_test_dotnet(command: &mut std::process::Command) -> Option<std::process::Output> {
        match command.output() {
            Ok(output) => Some(output),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("SKIP: the .NET muxer is not installed");
                None
            }
            Err(error) => panic!("Could not launch dotnet: {error}"),
        }
    }

    #[test]
    fn launch_profile_dotnet_test_skips_missing_executable() {
        let temp = tempfile::tempdir().unwrap();
        assert!(run_test_dotnet(&mut std::process::Command::new(
            temp.path().join("missing-dotnet")
        ))
        .is_none());
    }

    /// The muxer of the SDK the project pins, exactly like Run and Debug: the `dotnet` from PATH is
    /// not a substitute, because it may not contain the version the project's `global.json` asks
    /// for. `None` means no detected root satisfies the pin, and the test skips with the reason
    /// instead of borrowing a different SDK.
    fn project_test_muxer(project_dir: &Path) -> Option<(PathBuf, PathBuf, String)> {
        let text = fs::read_to_string(project_dir.join("global.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        let requested = json["sdk"]["version"].as_str().unwrap().to_string();
        let roll_forward = json["sdk"]["rollForward"].as_str();
        let inventory = crate::sdk::inventory();
        match crate::sdk::resolve(&requested, roll_forward, None, &inventory.installs) {
            Ok(install) => {
                let muxer = install.root.join(if cfg!(windows) {
                    "dotnet.exe"
                } else {
                    "dotnet"
                });
                Some((muxer, install.root.clone(), install.version))
            }
            Err(message) => {
                let roots: Vec<&str> = inventory
                    .roots
                    .iter()
                    .map(|root| root.root.as_str())
                    .collect();
                let reason = format!(
                    "no detected .NET root satisfies global.json ({requested}, rollForward {}): {message}; roots seen: {roots:?}",
                    roll_forward.unwrap_or("default")
                );
                // A machine that must have the SDK fails instead of silently losing the coverage.
                if std::env::var("OPENRIDR_REQUIRE_DOTNET").as_deref() == Ok("1") {
                    panic!("{reason}");
                }
                eprintln!("SKIP: {reason}");
                None
            }
        }
    }

    /// A pin no detected root satisfies must skip, not borrow whichever SDK PATH happens to carry.
    #[test]
    fn a_project_pin_without_a_detected_root_skips_instead_of_borrowing_path() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("global.json"),
            r#"{"sdk":{"version":"99.0.1","rollForward":"disable"}}"#,
        )
        .unwrap();
        assert!(project_test_muxer(temp.path()).is_none());
    }

    #[test]
    fn launch_profile_values_expand_percent_and_dollar_variables_once() {
        let lookup = |name: &str| match name {
            "A" => Some("1".to_string()),
            "B" => Some("2".to_string()),
            "LOOP" => Some("%B%".to_string()),
            "EMPTY" => Some(String::new()),
            _ => None,
        };
        let cases = [
            ("%A%", "1"),
            ("$(A)", "1"),
            ("pre%A%post", "pre1post"),
            ("%A%%B%", "12"),
            ("$(A)$(B)", "12"),
            // A substituted value is never expanded again, like the CLI's one-pass expansion.
            ("%LOOP%", "%B%"),
            ("%EMPTY%", ""),
            ("%NOPE%", "%NOPE%"),
            ("$(NOPE)", "$(NOPE)"),
            ("%%", "%%"),
            ("100%", "100%"),
            ("$()", "$()"),
            ("$(A", "$(A"),
            ("a$b", "a$b"),
            ("", ""),
        ];
        for (input, expected) in cases {
            assert_eq!(expand_value_with(input, lookup), expected, "{input:?}");
        }
    }

    #[test]
    fn launch_profile_expands_environment_and_application_url_but_not_arguments() {
        // HOME is set on every platform this app supports; the test fails loudly if it is not.
        let home = std::env::var("HOME").expect("HOME is set on the supported platform");
        let (_temp, project) = profile_fixture(
            r#"{"profiles":{"expand":{"commandName":"Project","applicationUrl":"http://localhost:5084/?home=%HOME%","environmentVariables":{"PROFILE_ENV":"%HOME%","DOLLAR":"$(HOME)","LITERAL":"%NOPE%","PERCENT":"100%"},"commandLineArgs":"%HOME% $(HOME)"}}}"#,
        );
        let root = project.parent().unwrap();
        let options = launch_options(
            &json!({"project": project, "launchProfile": "expand"}),
            root,
        )
        .unwrap();
        assert!(options.env.contains(&("PROFILE_ENV".into(), home.clone())));
        assert!(options.env.contains(&("DOLLAR".into(), home.clone())));
        assert!(options.env.contains(&("LITERAL".into(), "%NOPE%".into())));
        assert!(options.env.contains(&("PERCENT".into(), "100%".into())));
        assert!(options.env.contains(&(
            "ASPNETCORE_URLS".into(),
            format!("http://localhost:5084/?home={home}")
        )));
        // The argv stays literal: `dotnet run` does not expand commandLineArgs either.
        assert_eq!(
            options.args,
            vec!["%HOME%".to_string(), "$(HOME)".to_string()]
        );
    }

    #[test]
    fn launch_profile_working_directory_is_not_expanded() {
        let (_temp, project) = profile_fixture(
            r#"{"profiles":{"cwd":{"commandName":"Project","workingDirectory":"%HOME%"}}}"#,
        );
        let root = project.parent().unwrap();
        let error =
            launch_options(&json!({"project": project, "launchProfile": "cwd"}), root).unwrap_err();
        assert!(error.message.contains("%HOME%"), "{}", error.message);
    }

    #[test]
    fn launch_profile_decides_the_browser_from_its_own_values() {
        let (_temp, project) = profile_fixture(
            r#"{"profiles":{
                "off":{"commandName":"Project","applicationUrl":"http://localhost:5083"},
                "on":{"commandName":"Project","launchBrowser":true,"applicationUrl":"http://localhost:5083","launchUrl":"/swagger"},
                "envwins":{"commandName":"Project","launchBrowser":true,"applicationUrl":"http://localhost:5083","environmentVariables":{"ASPNETCORE_URLS":"https://localhost:7259"}},
                "bad":{"commandName":"Project","launchBrowser":true,"launchUrl":"file:///etc/passwd"}
            }}"#,
        );
        let root = project.parent().unwrap();
        let browser = |name: &str| {
            launch_options(
                &json!({"project": project.clone(), "launchProfile": name}),
                root,
            )
            .unwrap()
            .browser
        };
        assert_eq!(browser("off"), crate::browser::Launch::None);
        assert_eq!(
            browser("on"),
            crate::browser::Launch::Open("http://localhost:5083/swagger".to_string())
        );
        // The profile's own environment wins over applicationUrl, so that is where the app listens.
        assert_eq!(
            browser("envwins"),
            crate::browser::Launch::Open("https://localhost:7259".to_string())
        );
        assert!(
            matches!(browser("bad"), crate::browser::Launch::Skip(reason) if reason.contains("http(s)"))
        );
    }

    #[test]
    fn launch_profiles_preserve_order_and_apply_project_values() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/LaunchProfiles");
        let project = root.join("LaunchProfiles.csproj");
        let profiles = read_launch_profiles(&project).unwrap();
        assert_eq!(
            profiles
                .iter()
                .map(|profile| profile.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "http",
                "https",
                "expand",
                "browser",
                "browser-off",
                "browser-bad",
                "IIS Express"
            ]
        );
        assert_eq!(profiles[6].command_name, "IISExpress");
        let options =
            launch_options(&json!({"project": project, "launchProfile": "http"}), &root).unwrap();
        assert_eq!(options.args, vec!["--message", "hello world"]);
        assert_eq!(
            options.working_directory,
            Some(fs::canonicalize(&root).unwrap())
        );
        assert!(options.env.contains(&("PROFILE_ENV".into(), "http".into())));
        assert!(options
            .env
            .contains(&("ASPNETCORE_URLS".into(), "http://localhost:5083".into())));
        assert!(launch_options(
            &json!({"project": project, "launchProfile": "IIS Express"}),
            &root
        )
        .is_err());
    }

    /// Copies the LaunchProfiles fixture into a temp directory, as the app would see it.
    fn launch_profile_fixture_copy(temp: &Path) {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/LaunchProfiles");
        fs::create_dir(temp.join("Properties")).unwrap();
        for file in [
            "global.json",
            "LaunchProfiles.csproj",
            "Program.cs",
            "Properties/launchSettings.json",
        ] {
            fs::copy(source.join(file), temp.join(file)).unwrap();
        }
    }

    /// Runs the fixture with a real profile through the same muxer Run uses. `None` means no
    /// detected .NET root satisfies the fixture pin: skip, never a pass.
    fn run_launch_profile_fixture(temp: &Path, profile: &str) -> Option<String> {
        let project = temp.join("LaunchProfiles.csproj");
        let options =
            launch_options(&json!({"project": project, "launchProfile": profile}), temp).unwrap();
        // Same resolution as the product: the project's global.json decides the SDK, not PATH.
        let (muxer, sdk_root, sdk_version) = project_test_muxer(temp)?;
        assert!(
            muxer.is_file(),
            "a detected .NET root has no muxer at {}; a broken install is a failure, not a skip",
            muxer.display()
        );
        let mut command = std::process::Command::new(&muxer);
        command
            .args(["run", "--project"])
            .arg(&project)
            .arg("--no-launch-profile")
            .arg("--");
        command
            .args(&options.args)
            .envs(options.env.iter().cloned())
            .env("DOTNET_ROOT", &sdk_root)
            .env("DOTNET_MULTILEVEL_LOOKUP", "0")
            .env("DOTNET_CLI_UI_LANGUAGE", "en")
            .env("DOTNET_NOLOGO", "1")
            .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
            .current_dir(options.working_directory.as_deref().unwrap_or(temp));
        let output = command.output().expect(
            "the resolved project muxer must start; a detected-but-broken SDK is a failure, not a skip",
        );
        eprintln!(
            "launch profile fixture ran on SDK {sdk_version} from {}",
            sdk_root.display()
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Some(String::from_utf8_lossy(&output.stdout).to_string())
    }

    #[test]
    fn launch_profile_runs_fixture_with_effective_environment_and_arguments() {
        let temp = tempfile::tempdir().unwrap();
        launch_profile_fixture_copy(temp.path());
        let Some(stdout) = run_launch_profile_fixture(temp.path(), "http") else {
            return;
        };
        assert!(stdout.contains("PROFILE_ENV=http"), "{stdout}");
        assert!(
            stdout.contains("ASPNETCORE_URLS=http://localhost:5083"),
            "{stdout}"
        );
        assert!(stdout.contains("ARGS=--message|hello world"), "{stdout}");
        assert!(
            stdout.contains(&format!(
                "CWD={}",
                fs::canonicalize(temp.path()).unwrap().display()
            )),
            "{stdout}"
        );
    }

    #[test]
    fn launch_profile_runs_fixture_with_expanded_values() {
        let home = std::env::var("HOME").expect("HOME is set on the supported platform");
        let temp = tempfile::tempdir().unwrap();
        launch_profile_fixture_copy(temp.path());
        let Some(stdout) = run_launch_profile_fixture(temp.path(), "expand") else {
            return;
        };
        assert!(stdout.contains(&format!("PROFILE_ENV={home}")), "{stdout}");
        assert!(stdout.contains(&format!("EXPANDED={home}")), "{stdout}");
        assert!(
            stdout.contains(&format!(
                "ASPNETCORE_URLS=http://localhost:5084/?home={home}"
            )),
            "{stdout}"
        );
        assert!(stdout.contains("ARGS=%HOME%|$(HOME)"), "{stdout}");
    }

    #[test]
    fn launch_profile_reports_invalid_json_and_reloads_edits() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("App.csproj");
        fs::write(&project, "").unwrap();
        fs::create_dir(temp.path().join("Properties")).unwrap();
        let path = temp.path().join("Properties/launchSettings.json");
        fs::write(&path, "{").unwrap();
        let error = read_launch_profiles(&project).unwrap_err();
        assert!(error.message.contains("launchSettings.json"));
        fs::write(&path, r#"{"profiles":{"new":{"commandName":"Project"}}}"#).unwrap();
        let profiles = read_launch_profiles(&project).unwrap();
        assert_eq!(profiles[0].name, "new");
        assert_eq!(serde_json::to_value(&profiles[0]).unwrap()["name"], "new");
    }

    #[test]
    fn manual_config_writes_preserve_per_project_profile_choices() {
        let temp = tempfile::tempdir().unwrap();
        let solution = temp.path().join("App.sln");
        fs::write(&solution, "").unwrap();
        let path = temp.path().join("store.json");
        let mut choices = BTreeMap::new();
        choices.insert("App.csproj".to_string(), "https".to_string());
        let manual = config("custom", "Custom");
        let store = Store {
            solution: solution.to_string_lossy().to_string(),
            active_id: "custom".to_string(),
            configs: vec![manual.clone()],
            startup_project: None,
            launch_profiles: choices.clone(),
            session: SessionState::default(),
        };
        write_at(&path, &store).unwrap();
        let updated = store_from_params(
            &path,
            &solution,
            &json!({"activeId":"custom", "configs":[manual]}),
        )
        .unwrap();
        assert_eq!(updated.launch_profiles, choices);
        assert_eq!(updated.configs, store.configs);
    }

    fn config(id: &str, name: &str) -> RunConfig {
        RunConfig {
            id: id.to_string(),
            name: name.to_string(),
            project: "src/App/App.csproj".to_string(),
            configuration: "Debug".to_string(),
            target_framework: Some("net8.0".to_string()),
            args: Vec::new(),
            working_directory: String::new(),
            env: serde_json::Map::new(),
            env_files: Vec::new(),
        }
    }

    #[test]
    fn solution_param_accepts_a_real_slnx() {
        let home = tempfile::tempdir().unwrap();
        let slnx = home.path().join("Modern.slnx");
        fs::write(&slnx, "<Solution />").unwrap();

        let accepted = solution_param(&json!({ "solution": slnx.to_str().unwrap() })).unwrap();
        assert_eq!(accepted, fs::canonicalize(&slnx).unwrap());
    }

    #[test]
    fn fnv1a64_is_stable() {
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn env_parses_key_value_comments_and_rejects_invalid_lines() {
        let values = parse_env(
            "# comment\n\nOPENRIDR_F4=from file\nEMPTY=\nWITH_EQUALS=a=b\n  PADDED = value  \n",
        )
        .unwrap();
        assert_eq!(
            values,
            vec![
                ("OPENRIDR_F4".to_string(), "from file".to_string()),
                ("EMPTY".to_string(), String::new()),
                ("WITH_EQUALS".to_string(), "a=b".to_string()),
                ("PADDED".to_string(), "value".to_string()),
            ]
        );
        assert!(parse_env("NOT A LINE\n").unwrap_err().contains("line 1"));
        assert!(parse_env("1KEY=value\n").unwrap_err().contains("line 1"));
        assert!(parse_env("export KEY=value\n")
            .unwrap_err()
            .contains("line 1"));
        assert!(parse_env("=value\n").unwrap_err().contains("line 1"));
        // No expansion and no shell: the dollar sign and inline hash stay literal.
        assert_eq!(
            parse_env("LITERAL=$HOME # not a comment\n").unwrap(),
            vec![("LITERAL".to_string(), "$HOME # not a comment".to_string())]
        );
    }

    #[test]
    fn launch_options_resolve_cwd_and_merge_files_then_explicit_env() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("work")).unwrap();
        fs::write(root.path().join("one.env"), "A=one\nB=one\n").unwrap();
        fs::write(root.path().join("two.env"), "B=two\n").unwrap();
        let params = json!({
            "args": ["two words", "--name=value"],
            "workingDirectory": "work",
            "envFiles": ["one.env", "two.env"],
            "env": { "B": "explicit", "C": "three" },
        });
        let options = launch_options(&params, root.path()).unwrap();
        assert_eq!(options.args, vec!["two words", "--name=value"]);
        assert_eq!(
            options.working_directory.unwrap(),
            fs::canonicalize(root.path().join("work")).unwrap()
        );
        let env: std::collections::HashMap<_, _> = options.env.into_iter().collect();
        assert_eq!(env.get("A").map(String::as_str), Some("one"));
        assert_eq!(env.get("B").map(String::as_str), Some("explicit"));
        assert_eq!(env.get("C").map(String::as_str), Some("three"));
    }

    #[test]
    fn launch_options_refuse_a_missing_directory_and_a_bad_env_file() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("bad.env"), "NOT A LINE\n").unwrap();
        let missing = launch_options(&json!({ "workingDirectory": "missing" }), root.path())
            .expect_err("missing cwd");
        assert_eq!(missing.code, "invalid_state");
        let details = missing.details.unwrap();
        assert_eq!(details["field"], "workingDirectory");
        assert_eq!(details["reason"], "working_directory_missing");

        let bad =
            launch_options(&json!({ "envFiles": ["bad.env"] }), root.path()).expect_err("bad env");
        let details = bad.details.unwrap();
        assert_eq!(details["field"], "envFiles");
        assert_eq!(details["reason"], "env_file_invalid");
        assert!(bad.message.contains("line 1"), "{}", bad.message);
    }

    #[test]
    fn store_roundtrip_is_atomic_and_keeps_issues_without_refusing_save() {
        let home = tempfile::tempdir().unwrap();
        let solution = home.path().join("App.sln");
        fs::write(&solution, "").unwrap();
        let path = home.path().join("run-configs").join("store.json");
        let mut stored = config("one", "One");
        stored.working_directory = "missing".to_string();
        stored.env_files = vec!["missing.env".to_string()];
        let store = Store {
            solution: fs::canonicalize(&solution)
                .unwrap()
                .to_string_lossy()
                .to_string(),
            active_id: "one".to_string(),
            configs: vec![stored],
            startup_project: None,
            launch_profiles: BTreeMap::new(),
            session: SessionState::default(),
        };
        validate(&store.configs, &store.active_id).unwrap();
        write_at(&path, &store).unwrap();
        let read = read_at(&path, &fs::canonicalize(&solution).unwrap()).unwrap();
        assert_eq!(read.configs, store.configs);

        let issues = issues(&store.configs, home.path());
        assert_eq!(issues.len(), 2);
        assert!(issues.iter().any(|issue| issue.field == "workingDirectory"));
        assert!(issues.iter().any(|issue| issue.field == "envFiles"));
        let leftovers: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[test]
    fn session_roundtrips_and_config_writes_preserve_it() {
        let home = tempfile::tempdir().unwrap();
        let solution_file = home.path().join("App.sln");
        fs::write(&solution_file, "").unwrap();
        let solution = fs::canonicalize(&solution_file).unwrap();
        let path = home.path().join("run-configs").join("store.json");

        let mut breakpoints = std::collections::BTreeMap::new();
        breakpoints.insert("src/Program.cs".to_string(), vec![3, 9]);
        let session = SessionState {
            tabs: vec![SessionTab {
                path: "src/Program.cs".to_string(),
                line: 2,
                character: 4,
            }],
            active_path: Some("src/Program.cs".to_string()),
            breakpoints,
        };
        let store = Store {
            solution: solution.to_string_lossy().to_string(),
            active_id: "default".to_string(),
            configs: Vec::new(),
            startup_project: None,
            launch_profiles: BTreeMap::new(),
            session: session.clone(),
        };
        write_at(&path, &store).unwrap();
        assert_eq!(read_at(&path, &solution).unwrap().session, session);

        // The UI writes the session separately; a run-config write must not erase it.
        let rewritten = store_from_params(&path, &solution, &json!({ "configs": [] })).unwrap();
        assert_eq!(rewritten.session, session);
        let startup = set_startup_at(&path, &solution, &json!({ "startupProject": null })).unwrap();
        assert_eq!(startup.session, session);
    }

    #[test]
    fn an_editor_session_accepts_a_file_of_a_project_outside_the_solution() {
        let home = tempfile::tempdir().unwrap();
        let app = home.path().join("app");
        let lib = home.path().join("lib");
        fs::create_dir_all(&app).unwrap();
        fs::create_dir_all(&lib).unwrap();
        fs::write(lib.join("Lib.csproj"), "<Project />\n").unwrap();
        let solution_file = app.join("App.sln");
        fs::write(
            &solution_file,
            "Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Lib\", \"..\\lib\\Lib.csproj\", \"{1}\"\nEndProject\nGlobal\nEndGlobal\n",
        )
        .unwrap();
        let solution = fs::canonicalize(&solution_file).unwrap();
        let lib = fs::canonicalize(&lib).unwrap();
        let path = home.path().join("run-configs").join("store.json");
        let outside = lib.join("Greeting.cs");
        fs::write(&outside, "").unwrap();

        let written = write_session_at(
            &path,
            &solution,
            &json!({
                "session": {
                    "tabs": [{ "path": outside.to_string_lossy(), "line": 1, "character": 0 }],
                    "activePath": outside.to_string_lossy(),
                    "breakpoints": { outside.to_string_lossy(): [3] },
                },
            }),
        )
        .unwrap();
        assert_eq!(written.session.tabs[0].path, outside.to_string_lossy());
        assert_eq!(
            written.session.breakpoints[&outside.to_string_lossy().to_string()],
            vec![3]
        );

        // Outside the scope the session is refused and the stored one is untouched.
        let outside_scope = home.path().join("Outside.cs");
        fs::write(&outside_scope, "").unwrap();
        let error = write_session_at(
            &path,
            &solution,
            &json!({
                "session": {
                    "tabs": [{ "path": outside_scope.to_string_lossy(), "line": 1, "character": 0 }],
                },
            }),
        )
        .unwrap_err();
        assert_eq!(error.code, "invalid_run_configs");
        assert_eq!(
            read_at(&path, &solution).unwrap().session.tabs[0].path,
            outside.to_string_lossy()
        );
    }

    #[test]
    fn startup_project_roundtrips_is_preserved_by_a_config_write_and_can_be_cleared() {
        let home = tempfile::tempdir().unwrap();
        let solution_file = home.path().join("App.sln");
        fs::write(&solution_file, "").unwrap();
        let solution = fs::canonicalize(&solution_file).unwrap();
        let path = home.path().join("run-configs").join("store.json");

        let written = store_from_params(
            &path,
            &solution,
            &json!({
                "activeId": "default",
                "configs": [],
                "startupProject": "src/Tool/Tool.csproj",
            }),
        )
        .unwrap();
        write_at(&path, &written).unwrap();
        let read = read_at(&path, &solution).unwrap();
        assert_eq!(
            read.startup_project.as_deref(),
            Some("src/Tool/Tool.csproj")
        );

        // A configuration-only write keeps the choice; an explicit null clears it.
        let preserved = store_from_params(&path, &solution, &json!({ "configs": [] })).unwrap();
        assert_eq!(
            preserved.startup_project.as_deref(),
            Some("src/Tool/Tool.csproj")
        );
        let cleared = set_startup_at(&path, &solution, &json!({ "startupProject": null })).unwrap();
        assert_eq!(cleared.startup_project, None);
        assert_eq!(read_at(&path, &solution).unwrap().startup_project, None);
    }

    #[test]
    fn shape_errors_are_refused_with_a_field_and_never_written() {
        let mut duplicate = config("one", "One");
        duplicate.id = "two".to_string();
        let invalid_cases = [
            vec![config("", "One")],
            vec![config("one", "")],
            vec![config("one", "One"), config("one", "Two")],
            vec![config("one", "One"), duplicate.clone()],
            vec![config("one", "One"), config("two", "one")],
        ];
        for configs in invalid_cases {
            let error = validate(&configs, "default").expect_err("invalid shape");
            assert_eq!(error.code, "invalid_run_configs");
            assert!(error.details.is_some(), "{error:?}");
        }
        let error = validate(&[config("one", "One")], "missing").expect_err("active id");
        assert_eq!(error.details.unwrap()["field"], "activeId");
    }
}
