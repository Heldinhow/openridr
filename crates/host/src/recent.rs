//! History of opened workspaces, kept in the app home so it survives restarts and is shared by
//! the desktop app and the dev bridge. It stores paths only and needs no trust.

use crate::tools::app_home;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const LIMIT: usize = 30;

fn path() -> PathBuf {
    app_home().join("recent.json")
}

fn read_all() -> Vec<String> {
    let text = std::fs::read_to_string(path()).unwrap_or_default();
    serde_json::from_str::<Vec<String>>(&text).unwrap_or_default()
}

/// Most recent first; entries whose file no longer exists are hidden.
pub fn list() -> Value {
    let projects: Vec<String> = read_all().into_iter().filter(|item| Path::new(item).exists()).collect();
    json!({ "projects": projects })
}

/// Best effort: a failed write never blocks opening a workspace.
pub fn record(workspace_path: &str) {
    let mut projects = read_all();
    projects.retain(|item| item != workspace_path);
    projects.insert(0, workspace_path.to_string());
    projects.truncate(LIMIT);
    let _ = crate::fsutil::write_json_atomic(&path(), &json!(projects), "recent_failed");
}
