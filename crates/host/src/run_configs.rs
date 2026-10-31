//! Saved run configurations, one file per solution in the app home.
//!
//! The store never needs workspace trust: it reads and writes only its own file, starts no
//! process and touches no repository file. The active configuration is the source of truth
//! for the launch, so the same `.env` parser and path rules are reused by `run.start` and
//! `debug.start` instead of a second pipeline.

use crate::error::{HostError, HostResult};
use crate::tools::app_home;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_CONFIGS: usize = 50;
const MAX_ARGS: usize = 200;
const MAX_ENV: usize = 100;
const MAX_ENV_FILES: usize = 20;

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
}

pub fn read(params: &Value) -> HostResult<Value> {
    let solution = solution_param(params)?;
    let path = path_for(&solution);
    let store = read_at(&path, &solution)?;
    Ok(view(&store, &path, &solution))
}

pub fn write(params: &Value) -> HostResult<Value> {
    let solution = solution_param(params)?;
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
    let path = path_for(&solution);
    let store = Store {
        solution: solution.to_string_lossy().to_string(),
        active_id,
        configs,
    };
    write_at(&path, &store)?;
    Ok(view(&store, &path, &solution))
}

/// Parses the launch options shared by `run.start` and `debug.start`. Paths are relative to
/// the solution root, argv is never split by a shell, and the working directory must exist
/// before anything is spawned.
pub fn launch_options(params: &Value, root: &Path) -> HostResult<LaunchOptions> {
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
            return Err(format!(
                "line {}: expected KEY=VALUE.",
                index + 1
            ));
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
            HostError::new(
                "invalid_run_configs",
                "runConfigs needs the solution path.",
            )
        })?;
    let path = Path::new(raw);
    if !path.is_absolute() {
        return Err(HostError::new(
            "invalid_run_configs",
            "The solution path must be absolute.",
        ));
    }
    let canonical = fs::canonicalize(path).map_err(|_| {
        HostError::new(
            "invalid_run_configs",
            format!("Solution not found: {raw}"),
        )
    })?;
    if !matches!(
        canonical.extension().and_then(|extension| extension.to_str()),
        Some("sln") | Some("csproj")
    ) {
        return Err(HostError::new(
            "invalid_run_configs",
            "Run configurations are stored for a .sln or .csproj.",
        ));
    }
    Ok(canonical)
}

fn read_at(path: &Path, solution: &Path) -> HostResult<Store> {
    let empty = Store {
        solution: solution.to_string_lossy().to_string(),
        active_id: "default".to_string(),
        configs: Vec::new(),
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
            format!("{} is not a valid run configuration file: {error}", path.to_string_lossy()),
        )
    })?;
    if store.solution != solution.to_string_lossy() {
        // The hash key is stable; a different solution here means a copied file, never the
        // current one. Returning empty avoids showing another solution's configurations.
        return Ok(empty);
    }
    let mut store = store;
    if store.active_id != "default"
        && !store.configs.iter().any(|config| config.id == store.active_id)
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
        "configs": &store.configs,
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
                format!("Another run configuration is already named \"{name}\"; names must be unique."),
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

fn option_error(field: &'static str, reason: &'static str, message: impl Into<String>) -> HostError {
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
        assert!(parse_env("export KEY=value\n").unwrap_err().contains("line 1"));
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
        let missing = launch_options(
            &json!({ "workingDirectory": "missing" }),
            root.path(),
        )
        .expect_err("missing cwd");
        assert_eq!(missing.code, "invalid_state");
        let details = missing.details.unwrap();
        assert_eq!(details["field"], "workingDirectory");
        assert_eq!(details["reason"], "working_directory_missing");

        let bad = launch_options(&json!({ "envFiles": ["bad.env"] }), root.path())
            .expect_err("bad env");
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
            solution: fs::canonicalize(&solution).unwrap().to_string_lossy().to_string(),
            active_id: "one".to_string(),
            configs: vec![stored],
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
