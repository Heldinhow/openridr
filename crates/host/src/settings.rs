//! Global app settings. The file lives in the app home, is validated before it replaces the
//! previous copy, and never needs workspace trust: it starts no process and touches no repo.

use crate::error::{HostError, HostResult};
use crate::tools::app_home;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn path() -> PathBuf {
    app_home().join("settings.json")
}

pub fn read() -> HostResult<Value> {
    let path = path();
    let settings = read_at(&path)?;
    Ok(json!({ "path": path.to_string_lossy(), "settings": settings }))
}

pub fn write(params: &Value) -> HostResult<Value> {
    let settings = params.get("settings").ok_or_else(|| {
        HostError::new("invalid_settings", "settings.write requires a settings object.")
    })?;
    let path = path();
    write_at(&path, settings)?;
    Ok(json!({ "path": path.to_string_lossy(), "settings": settings }))
}

pub fn read_at(path: &Path) -> HostResult<Value> {
    match fs::read_to_string(path) {
        Ok(text) => {
            let value: Value = serde_json::from_str(&text).map_err(|error| {
                HostError::new(
                    "invalid_settings",
                    format!("{} is not valid JSON: {error}", path.to_string_lossy()),
                )
            })?;
            validate(&value)?;
            Ok(value)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(json!({})),
        Err(error) => Err(HostError::new(
            "operation_failed",
            format!("Could not read {}: {error}", path.to_string_lossy()),
        )),
    }
}

pub fn write_at(path: &Path, settings: &Value) -> HostResult<()> {
    validate(settings)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            HostError::new(
                "operation_failed",
                format!("Could not create {}: {error}", parent.to_string_lossy()),
            )
        })?;
    }
    let mut text = serde_json::to_string_pretty(settings)
        .map_err(|error| HostError::new("invalid_settings", error.to_string()))?;
    text.push('\n');
    let temp = path.with_file_name(format!(
        ".settings-{}-{}.tmp",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default()
    ));
    if let Err(error) = fs::write(&temp, text) {
        return Err(HostError::new(
            "operation_failed",
            format!("Could not write {}: {error}", temp.to_string_lossy()),
        ));
    }
    if let Err(error) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(HostError::new(
            "operation_failed",
            format!("Could not replace {}: {error}", path.to_string_lossy()),
        ));
    }
    Ok(())
}

fn validate(settings: &Value) -> HostResult<()> {
    let object = settings.as_object().ok_or_else(|| {
        HostError::new("invalid_settings", "Settings must be a JSON object.")
    })?;
    let Some(keymap) = object.get("keymap") else {
        return Ok(());
    };
    let keymap = keymap
        .as_object()
        .ok_or_else(|| HostError::new("invalid_settings", "keymap must be an object."))?;
    if let Some(profile) = keymap.get("profile") {
        let profile = profile
            .as_str()
            .ok_or_else(|| HostError::new("invalid_settings", "keymap.profile must be a string."))?;
        if !matches!(profile, "rider-macos" | "intellij") {
            return Err(HostError::new(
                "invalid_settings",
                "keymap.profile must be \"rider-macos\" or \"intellij\".",
            ));
        }
    }
    let Some(overrides) = keymap.get("overrides") else {
        return Ok(());
    };
    let overrides = overrides
        .as_object()
        .ok_or_else(|| HostError::new("invalid_settings", "keymap.overrides must be an object."))?;
    for (id, list) in overrides {
        if id.is_empty() || id.len() > 80 {
            return Err(HostError::new(
                "invalid_settings",
                "keymap.overrides keys must be non-empty action ids.",
            ));
        }
        let list = list.as_array().ok_or_else(|| {
            HostError::new(
                "invalid_settings",
                format!("keymap.overrides.{id} must be an array of shortcuts."),
            )
        })?;
        if list.is_empty() || list.len() > 4 {
            return Err(HostError::new(
                "invalid_settings",
                format!("keymap.overrides.{id} must list 1 to 4 shortcuts."),
            ));
        }
        for shortcut in list {
            validate_shortcut(id, shortcut)?;
        }
    }
    Ok(())
}

fn validate_shortcut(id: &str, value: &Value) -> HostResult<()> {
    let object = value.as_object().ok_or_else(|| {
        HostError::new(
            "invalid_settings",
            format!("keymap.overrides.{id} contains a shortcut that is not an object."),
        )
    })?;
    let key = object.get("key").and_then(Value::as_str).unwrap_or_default();
    if key.is_empty() || key.len() > 24 {
        return Err(HostError::new(
            "invalid_settings",
            format!("keymap.overrides.{id} needs a non-empty key of at most 24 characters."),
        ));
    }
    for flag in ["primary", "control", "alt", "shift"] {
        if let Some(value) = object.get(flag) {
            if !value.is_boolean() {
                return Err(HostError::new(
                    "invalid_settings",
                    format!("keymap.overrides.{id}.{flag} must be a boolean."),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_roundtrip_is_valid_json_and_leaves_no_temp_file() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("settings.json");
        assert_eq!(read_at(&path).unwrap(), json!({}));
        let settings = json!({
            "keymap": {
                "profile": "intellij",
                "overrides": { "build.start": [{ "key": "f9", "primary": true }] }
            },
            "editor": { "fontSize": 14 }
        });
        write_at(&path, &settings).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(serde_json::from_str::<Value>(&text).is_ok());
        assert_eq!(read_at(&path).unwrap(), settings);
        let leftovers: Vec<_> = fs::read_dir(home.path())
            .unwrap()
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[test]
    fn settings_reject_invalid_payloads_without_writing() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join("settings.json");
        let valid = json!({ "keymap": { "profile": "rider-macos", "overrides": {} } });
        write_at(&path, &valid).unwrap();
        let before = fs::read_to_string(&path).unwrap();

        for invalid in [
            json!([]),
            json!("settings"),
            json!({ "keymap": [] }),
            json!({ "keymap": { "profile": "vscode" } }),
            json!({ "keymap": { "overrides": [] } }),
            json!({ "keymap": { "overrides": { "build.start": [] } } }),
            json!({ "keymap": { "overrides": { "build.start": [{ "key": "" }] } } }),
            json!({ "keymap": { "overrides": { "build.start": [{ "key": "f9", "alt": "yes" }] } } }),
        ] {
            let error = write_at(&path, &invalid).expect_err("invalid payload");
            assert_eq!(error.code, "invalid_settings", "{invalid}");
        }
        assert_eq!(fs::read_to_string(&path).unwrap(), before);
    }
}
