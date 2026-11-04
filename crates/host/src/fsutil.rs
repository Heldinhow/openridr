//! Shared file helpers for the host's own state files and editor text files.

use crate::error::{HostError, HostResult};
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Encoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
}

impl Encoding {
    fn name(self) -> &'static str {
        match self {
            Encoding::Utf8 | Encoding::Utf8Bom => "UTF-8",
            Encoding::Utf16Le => "UTF-16LE",
            Encoding::Utf16Be => "UTF-16BE",
        }
    }
}

/// How the file was encoded on disk, so a save can put it back the same way.
#[derive(Clone, Copy)]
pub(crate) struct TextFormat {
    encoding: Encoding,
    crlf: bool,
}

/// The editor always works in LF; the host restores the file's line ending on write.
pub(crate) fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Decodes the bytes the editor can show and remembers how to write them back.
/// Invalid data is a clear error, never a lossy replacement, so a save cannot corrupt it.
pub(crate) fn decode_text(bytes: &[u8]) -> Result<(String, TextFormat), String> {
    let (encoding, rest) = if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        (Encoding::Utf8Bom, rest)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        (Encoding::Utf16Le, rest)
    } else if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        (Encoding::Utf16Be, rest)
    } else {
        (Encoding::Utf8, bytes)
    };
    let text = match encoding {
        Encoding::Utf8 | Encoding::Utf8Bom => String::from_utf8(rest.to_vec())
            .map_err(|_| format!("the file is not valid {} text", encoding.name()))?,
        Encoding::Utf16Le | Encoding::Utf16Be => {
            if rest.len() % 2 != 0 {
                return Err(format!(
                    "the file is not valid {} text: odd byte count",
                    encoding.name()
                ));
            }
            let units: Vec<u16> = rest
                .chunks_exact(2)
                .map(|pair| match encoding {
                    Encoding::Utf16Le => u16::from_le_bytes([pair[0], pair[1]]),
                    _ => u16::from_be_bytes([pair[0], pair[1]]),
                })
                .collect();
            String::from_utf16(&units)
                .map_err(|_| format!("the file is not valid {} text", encoding.name()))?
        }
    };
    // A mixed file keeps CRLF: the first CRLF found defines the style.
    let format = TextFormat {
        encoding,
        crlf: text.contains("\r\n"),
    };
    Ok((normalize_newlines(&text), format))
}

impl TextFormat {
    /// Encodes the editor's LF text with the original encoding, BOM and line ending.
    pub(crate) fn encode(self, text: &str) -> Vec<u8> {
        let text = normalize_newlines(text);
        let text = if self.crlf {
            text.replace('\n', "\r\n")
        } else {
            text
        };
        let mut bytes = Vec::new();
        match self.encoding {
            Encoding::Utf8 => bytes.extend_from_slice(text.as_bytes()),
            Encoding::Utf8Bom => {
                bytes.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
                bytes.extend_from_slice(text.as_bytes());
            }
            Encoding::Utf16Le | Encoding::Utf16Be => {
                bytes.extend_from_slice(match self.encoding {
                    Encoding::Utf16Le => &[0xFF, 0xFE],
                    _ => &[0xFE, 0xFF],
                });
                for unit in text.encode_utf16() {
                    match self.encoding {
                        Encoding::Utf16Le => bytes.extend_from_slice(&unit.to_le_bytes()),
                        _ => bytes.extend_from_slice(&unit.to_be_bytes()),
                    }
                }
            }
        }
        bytes
    }
}

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
