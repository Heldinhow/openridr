//! Workspaces the user chose to trust, kept in the app home so the decision survives restarts
//! (like Rider's trusted paths). Keyed by the canonical workspace path; revoking forgets it.
//! `OPENRIDR_TRUST_STORE=off` disables reads and writes (isolated E2E runs).

use crate::tools::app_home;
use serde_json::json;
use std::path::PathBuf;

fn path() -> PathBuf {
    app_home().join("trusted.json")
}

fn enabled() -> bool {
    std::env::var("OPENRIDR_TRUST_STORE").map_or(true, |value| value != "off")
}

fn read_all() -> Vec<String> {
    let text = std::fs::read_to_string(path()).unwrap_or_default();
    serde_json::from_str::<Vec<String>>(&text).unwrap_or_default()
}

pub fn is_trusted(workspace_path: &str) -> bool {
    enabled() && read_all().iter().any(|item| item == workspace_path)
}

/// Best effort: a failed write only means the question is asked again next time.
pub fn remember(workspace_path: &str) {
    if !enabled() || is_trusted(workspace_path) {
        return;
    }
    let mut paths = read_all();
    paths.push(workspace_path.to_string());
    let _ = crate::fsutil::write_json_atomic(&path(), &json!(paths), "trusted_failed");
}

pub fn forget(workspace_path: &str) {
    if !enabled() {
        return;
    }
    let mut paths = read_all();
    let before = paths.len();
    paths.retain(|item| item != workspace_path);
    if paths.len() != before {
        let _ = crate::fsutil::write_json_atomic(&path(), &json!(paths), "trusted_failed");
    }
}
