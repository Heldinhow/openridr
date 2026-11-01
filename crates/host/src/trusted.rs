//! One Trust decision per project, kept in the app home so it survives restarts (like Rider's
//! trusted paths). `trusted.json` maps the canonical workspace path to `trusted` or `restricted`;
//! a missing entry means the user was never asked. The old format (a list of paths) reads as
//! `trusted`. A file that cannot be read is never overwritten.
//! `OPENRIDR_TRUST_STORE=off` disables reads and writes (isolated E2E runs).

use crate::tools::app_home;
use crate::{HostError, HostResult};
use serde_json::{json, Map, Value};
use std::path::PathBuf;

pub const TRUSTED: &str = "trusted";
pub const RESTRICTED: &str = "restricted";

fn path() -> PathBuf {
    app_home().join("trusted.json")
}

fn enabled() -> bool {
    std::env::var("OPENRIDR_TRUST_STORE").map_or(true, |value| value != "off")
}

/// `None` means the file exists but is unreadable or unrecognised: callers must not write over it.
fn read_all() -> Option<Map<String, Value>> {
    let text = match std::fs::read_to_string(path()) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Some(Map::new()),
        Err(_) => return None,
    };
    match serde_json::from_str::<Value>(&text).ok()? {
        Value::Object(map) => Some(map),
        Value::Array(items) => Some(
            items
                .into_iter()
                .filter_map(|item| Some((item.as_str()?.to_string(), json!(TRUSTED))))
                .collect(),
        ),
        _ => None,
    }
}

/// The stored decision (`trusted` | `restricted`) or `None` when never decided.
pub fn get(workspace_path: &str) -> Option<&'static str> {
    if !enabled() {
        return None;
    }
    match read_all()?.get(workspace_path)?.as_str()? {
        TRUSTED => Some(TRUSTED),
        RESTRICTED => Some(RESTRICTED),
        _ => None,
    }
}

/// Persists the decision. A store that is off is a no-op; an unreadable file or a failed write is
/// an error so the caller can tell the user the decision was not saved.
pub fn set(workspace_path: &str, decision: &str) -> HostResult<()> {
    if !enabled() {
        return Ok(());
    }
    let Some(mut map) = read_all() else {
        return Err(HostError::new(
            "trusted_unreadable",
            "trusted.json could not be read, so the decision was not saved. Fix or remove the file.",
        ));
    };
    if map.get(workspace_path).and_then(Value::as_str) == Some(decision) {
        return Ok(());
    }
    map.insert(workspace_path.to_string(), json!(decision));
    crate::fsutil::write_json_atomic(&path(), &Value::Object(map), "trusted_failed")
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::sync::Mutex;

    // The store reads process-wide env vars, so the tests take turns.
    pub(crate) static ENV: Mutex<()> = Mutex::new(());

    fn with_home(run: impl FnOnce(&std::path::Path)) {
        let _guard = ENV.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let home = tempfile::tempdir().unwrap();
        std::env::set_var("OPENRIDR_HOME", home.path());
        std::env::remove_var("OPENRIDR_TRUST_STORE");
        run(&home.path().join("trusted.json"));
        std::env::remove_var("OPENRIDR_HOME");
        std::env::remove_var("OPENRIDR_TRUST_STORE");
    }

    #[test]
    fn set_then_get_and_never_decided_is_none() {
        with_home(|_| {
            assert_eq!(get("/a"), None);
            set("/a", TRUSTED).unwrap();
            set("/b", RESTRICTED).unwrap();
            assert_eq!(get("/a"), Some(TRUSTED));
            assert_eq!(get("/b"), Some(RESTRICTED));
            set("/a", RESTRICTED).unwrap();
            assert_eq!(get("/a"), Some(RESTRICTED));
            assert_eq!(get("/c"), None);
        });
    }

    #[test]
    fn old_list_format_reads_as_trusted_and_migrates_on_write() {
        with_home(|file| {
            std::fs::write(file, r#"["/old","/older"]"#).unwrap();
            assert_eq!(get("/old"), Some(TRUSTED));
            assert_eq!(get("/older"), Some(TRUSTED));
            set("/new", RESTRICTED).unwrap();
            let stored: Value = serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
            assert_eq!(
                stored,
                json!({ "/old": "trusted", "/older": "trusted", "/new": "restricted" })
            );
        });
    }

    #[test]
    fn unreadable_file_is_not_overwritten() {
        with_home(|file| {
            std::fs::write(file, "{ not json").unwrap();
            assert_eq!(get("/a"), None);
            assert_eq!(set("/a", TRUSTED).unwrap_err().code, "trusted_unreadable");
            assert_eq!(std::fs::read_to_string(file).unwrap(), "{ not json");
        });
    }

    #[test]
    fn store_off_reads_nothing_and_writes_nothing() {
        with_home(|file| {
            set("/a", TRUSTED).unwrap();
            std::env::set_var("OPENRIDR_TRUST_STORE", "off");
            assert_eq!(get("/a"), None);
            set("/b", TRUSTED).unwrap();
            std::env::remove_var("OPENRIDR_TRUST_STORE");
            assert_eq!(get("/b"), None);
            assert_eq!(get("/a"), Some(TRUSTED));
            assert!(file.exists());
        });
    }
}
