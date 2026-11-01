//! History of opened workspaces, kept in the app home so it survives restarts and is shared by
//! the desktop app and the dev bridge. `recent.json` holds `{path, openedAt}` entries (epoch ms);
//! the old format, a list of path strings, reads as entries without a date. Removing an entry
//! never touches the project's Trust decision.

use crate::tools::app_home;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const LIMIT: usize = 30;

fn path() -> PathBuf {
    app_home().join("recent.json")
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

/// `(path, openedAt)`, most recent first. `None` means the file exists but is unreadable or
/// unrecognised: writers must not overwrite it.
fn read_all() -> Option<Vec<(String, Option<u64>)>> {
    let text = match std::fs::read_to_string(path()) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Some(Vec::new()),
        Err(_) => return None,
    };
    let Value::Array(items) = serde_json::from_str::<Value>(&text).ok()? else { return None };
    Some(
        items
            .iter()
            .filter_map(|item| match item {
                Value::String(path) => Some((path.clone(), None)),
                Value::Object(_) => Some((item["path"].as_str()?.to_string(), item["openedAt"].as_u64())),
                _ => None,
            })
            .collect(),
    )
}

fn write_all(projects: &[(String, Option<u64>)]) {
    let items: Vec<Value> = projects
        .iter()
        .map(|(path, opened_at)| json!({ "path": path, "openedAt": opened_at }))
        .collect();
    let _ = crate::fsutil::write_json_atomic(&path(), &json!(items), "recent_failed");
}

/// Most recent first; entries whose file no longer exists are hidden. Each carries the stored
/// Trust decision (`null` when never decided).
pub fn list() -> Value {
    let projects: Vec<Value> = read_all()
        .unwrap_or_default()
        .into_iter()
        .filter(|(item, _)| Path::new(item).exists())
        .map(|(item, opened_at)| {
            let decision = crate::trusted::get(&item);
            match opened_at {
                Some(at) => json!({ "path": item, "openedAt": at, "decision": decision }),
                None => json!({ "path": item, "decision": decision }),
            }
        })
        .collect();
    json!({ "projects": projects })
}

/// Best effort: a failed write never blocks opening a workspace.
pub fn record(workspace_path: &str) {
    let Some(mut projects) = read_all() else { return };
    projects.retain(|(item, _)| item != workspace_path);
    projects.insert(0, (workspace_path.to_string(), Some(now_ms())));
    projects.truncate(LIMIT);
    write_all(&projects);
}

/// Forgets the entry only; the Trust decision stays.
pub fn remove(workspace_path: &str) {
    let Some(mut projects) = read_all() else { return };
    projects.retain(|(item, _)| item != workspace_path);
    write_all(&projects);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trusted::tests::ENV;

    fn with_home(run: impl FnOnce(&Path)) {
        let _guard = ENV.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let home = tempfile::tempdir().unwrap();
        std::env::set_var("OPENRIDR_HOME", home.path());
        std::env::remove_var("OPENRIDR_TRUST_STORE");
        run(home.path());
        std::env::remove_var("OPENRIDR_HOME");
    }

    fn paths(listed: &Value) -> Vec<&str> {
        listed["projects"].as_array().unwrap().iter().map(|p| p["path"].as_str().unwrap()).collect()
    }

    #[test]
    fn old_list_format_reads_without_dates_and_upgrades_on_record() {
        with_home(|home| {
            let a = tempfile::tempdir().unwrap();
            let b = tempfile::tempdir().unwrap();
            let (a, b) = (a.path().to_str().unwrap(), b.path().to_str().unwrap());
            std::fs::write(home.join("recent.json"), json!([a, b, "/nope/missing"]).to_string()).unwrap();
            let listed = list();
            assert_eq!(paths(&listed), vec![a, b]);
            assert!(listed["projects"][0].get("openedAt").is_none());
            record(b);
            let listed = list();
            assert_eq!(paths(&listed), vec![b, a]);
            assert!(listed["projects"][0]["openedAt"].as_u64().unwrap() > 0);
        });
    }

    #[test]
    fn record_dedupes_and_caps_at_the_limit() {
        with_home(|_| {
            let dir = tempfile::tempdir().unwrap();
            let base = dir.path().to_str().unwrap();
            for index in 0..(LIMIT + 5) {
                record(&format!("{base}/p{index}"));
            }
            record(&format!("{base}/p10"));
            let all = read_all().unwrap();
            assert_eq!(all.len(), LIMIT);
            assert_eq!(all[0].0, format!("{base}/p10"));
            assert_eq!(all.iter().filter(|(path, _)| path.ends_with("/p10")).count(), 1);
        });
    }

    #[test]
    fn remove_forgets_the_entry_but_keeps_its_decision() {
        with_home(|_| {
            let a = tempfile::tempdir().unwrap();
            let b = tempfile::tempdir().unwrap();
            let (a, b) = (a.path().to_str().unwrap(), b.path().to_str().unwrap());
            crate::trusted::set(a, crate::trusted::TRUSTED).unwrap();
            record(a);
            record(b);
            let listed = list();
            assert_eq!(listed["projects"][0]["decision"], Value::Null);
            assert_eq!(listed["projects"][1]["decision"], "trusted");
            remove(a);
            assert_eq!(paths(&list()), vec![b]);
            assert_eq!(crate::trusted::get(a), Some("trusted"));
        });
    }

    #[test]
    fn unreadable_file_is_not_overwritten() {
        with_home(|home| {
            std::fs::write(home.join("recent.json"), "{ not json").unwrap();
            assert!(list()["projects"].as_array().unwrap().is_empty());
            record("/a");
            remove("/a");
            assert_eq!(std::fs::read_to_string(home.join("recent.json")).unwrap(), "{ not json");
        });
    }
}
