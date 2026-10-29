//! Shared file helpers for the host's own state files.

use crate::error::{HostError, HostResult};
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Writes JSON through a sibling temporary file and a rename, so an interrupted write never
/// leaves a partial file. `invalid_code` is the caller's code for a value that cannot
/// serialize; every filesystem failure is `operation_failed`.
pub(crate) fn write_json_atomic<T: Serialize>(
    path: &Path,
    value: &T,
    invalid_code: &str,
) -> HostResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            HostError::new(
                "operation_failed",
                format!("Could not create {}: {error}", parent.to_string_lossy()),
            )
        })?;
    }
    let mut text = serde_json::to_string_pretty(value)
        .map_err(|error| HostError::new(invalid_code, error.to_string()))?;
    text.push('\n');
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("store");
    let temp = path.with_file_name(format!(
        ".{name}-{}-{}.tmp",
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
