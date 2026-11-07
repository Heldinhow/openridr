//! Shared file helpers for the host's own state files and editor text files.

use crate::error::{HostError, HostResult};
use crate::scope::Scope;
use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};
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
                .as_chunks::<2>()
                .0
                .iter()
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

/// The command that selects `path` in the platform file manager. Only macOS is verified;
/// another platform gets a clear error instead of a silent no-op.
pub(crate) fn reveal_command(
    platform: &str,
    path: &Path,
) -> HostResult<(&'static str, Vec<String>)> {
    if platform != "macos" {
        return Err(HostError::new(
            "unsupported_platform",
            format!("Show in Finder is only implemented on macOS; this build runs on {platform}."),
        ));
    }
    Ok((
        "open",
        vec!["-R".to_string(), path.to_string_lossy().into_owned()],
    ))
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

/// Normalizes `.` and `..` without touching the disk, so containment can be decided by shape
/// alone. `..` above the filesystem root stays at the root, the way the operating system resolves
/// it.
pub(crate) fn normalize_lexically(path: &Path) -> PathBuf {
    let mut parts: Vec<Component> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match parts.last() {
                Some(Component::Normal(_)) => {
                    parts.pop();
                }
                Some(Component::ParentDir) => parts.push(component),
                // `/..` and `C:..` are already at the root of that path.
                _ => {}
            },
            other => parts.push(other),
        }
    }
    parts.iter().collect()
}

/// Whether `candidate` stays inside `root` by shape, before any filesystem call. Every resolver
/// checks this first for a relative path and canonicalizes second: this check refuses an escaping
/// path whether or not it exists, and the canonical step afterwards is what stops a symlink inside
/// the root from resolving outside it (which a text comparison cannot see).
pub(crate) fn stays_inside(root: &Path, candidate: &Path) -> bool {
    normalize_lexically(candidate).starts_with(normalize_lexically(root))
}

/// Why a confined path was refused. Each frontier turns this into its own `HostError`, so the
/// wording stays specific (`File not found`, `Folder not found`, `Project not found`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ConfineError {
    /// The path, or the deepest existing ancestor of it, resolves outside the root.
    Outside,
    /// Nothing at the path, and every existing ancestor resolves inside the root.
    Missing,
}

/// Whether the deepest existing ancestor of `candidate` — the path itself when it exists — resolves
/// inside the scope. Deciding on the nearest *existing* ancestor is what stops a symlink that
/// carries a missing target outside from hiding behind `not_found`.
///
/// A dangling symlink (or a loop) exists but cannot be resolved, so where it points is unknown and
/// containment cannot be proven: it is refused, and it is refused the same way whether or not the
/// missing target exists — otherwise the answer would reveal what lives outside the workspace.
fn existing_ancestor_resolves_inside(scope: &Scope, candidate: &Path) -> bool {
    let mut probe = candidate;
    loop {
        match fs::canonicalize(probe) {
            Ok(canonical) => return scope.contains(&canonical),
            Err(_) if fs::symlink_metadata(probe).is_ok() => return false,
            Err(_) => match probe.parent() {
                Some(parent) if parent != probe => probe = parent,
                // Nothing above the path exists, so containment cannot be proven: refuse.
                _ => return false,
            },
        }
    }
}

/// The canonical form of a candidate that already passed `stays_inside` (when it applies).
///
/// A target that does not exist is `Missing` only after its nearest existing ancestor was proven to
/// be inside the scope; when that ancestor resolves outside (a symlink pointing out, with any number
/// of missing components after it) the answer is `Outside`, the same as for an existing target.
/// That is what keeps the refusal independent of what exists outside the workspace.
pub(crate) fn canonical_confined(scope: &Scope, candidate: &Path) -> Result<PathBuf, ConfineError> {
    match fs::canonicalize(candidate) {
        Ok(canonical) if scope.contains(&canonical) => Ok(canonical),
        Ok(_) => Err(ConfineError::Outside),
        Err(_) if existing_ancestor_resolves_inside(scope, candidate) => Err(ConfineError::Missing),
        Err(_) => Err(ConfineError::Outside),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexical_normalization_decides_containment_without_the_disk() {
        let root = Path::new("/work/app");
        assert!(stays_inside(root, Path::new("/work/app/src/App.cs")));
        assert!(stays_inside(root, Path::new("/work/app")));
        assert!(stays_inside(root, Path::new("/work/app/src/../App.cs")));
        assert!(stays_inside(
            root,
            Path::new("/work/app/src/deep/../../App.cs")
        ));
        assert!(!stays_inside(root, Path::new("/work/app/../app2/App.cs")));
        assert!(!stays_inside(root, Path::new("/work/other/App.cs")));
        assert!(!stays_inside(root, Path::new("/etc/hosts")));
        assert!(!stays_inside(root, Path::new("/work/applications/App.cs")));
        // A path that does not exist is still refused by shape; `..` saturates at the root.
        assert!(!stays_inside(root, Path::new("/nope/../../etc/missing")));
        assert_eq!(
            normalize_lexically(Path::new("/../../etc/hosts")),
            Path::new("/etc/hosts")
        );
        assert_eq!(
            normalize_lexically(Path::new("/a/./b/../c")),
            Path::new("/a/c")
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_missing_target_under_an_escaping_symlink_is_outside_not_missing() {
        let outer = tempfile::tempdir().unwrap();
        let root = outer.path().join("ws");
        fs::create_dir_all(&root).unwrap();
        let outside = outer.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(root.join("App.cs"), "class C {}\n").unwrap();
        let root = fs::canonicalize(&root).unwrap();
        let scope = Scope::new(&root);
        std::os::unix::fs::symlink(&outside, root.join("Link")).unwrap();

        assert_eq!(
            canonical_confined(&scope, &root.join("App.cs")).unwrap(),
            root.join("App.cs")
        );
        // Existing escape and missing escape answer alike: no existence oracle.
        assert_eq!(
            canonical_confined(&scope, &root.join("Link")),
            Err(ConfineError::Outside)
        );
        assert_eq!(
            canonical_confined(&scope, &root.join("Link/missing.txt")),
            Err(ConfineError::Outside)
        );
        assert_eq!(
            canonical_confined(&scope, &root.join("Link/deep/missing.txt")),
            Err(ConfineError::Outside)
        );
        // Missing inside the workspace stays missing.
        assert_eq!(
            canonical_confined(&scope, &root.join("missing.txt")),
            Err(ConfineError::Missing)
        );
        assert_eq!(
            canonical_confined(&scope, &root.join("missing/deep.txt")),
            Err(ConfineError::Missing)
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_dangling_symlink_is_outside_whether_or_not_its_target_exists() {
        let outer = tempfile::tempdir().unwrap();
        let root = outer.path().join("ws");
        fs::create_dir_all(&root).unwrap();
        let outside = outer.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("There.cs"), "class T {}\n").unwrap();
        let root = fs::canonicalize(&root).unwrap();
        let scope = Scope::new(&root);

        // Same link, two targets: one that exists outside and one that does not.
        std::os::unix::fs::symlink(outside.join("There.cs"), root.join("There.cs")).unwrap();
        std::os::unix::fs::symlink(outside.join("Missing.cs"), root.join("Missing.cs")).unwrap();
        // A dangling link whose target name would be inside the workspace: the host cannot prove
        // where it points, so it is refused like the others (never `not_found`).
        std::os::unix::fs::symlink("not-yet-created.cs", root.join("Pending.cs")).unwrap();
        // A loop exists but never resolves.
        std::os::unix::fs::symlink(root.join("Loop.cs"), root.join("Loop.cs")).unwrap();

        for link in ["There.cs", "Missing.cs", "Pending.cs", "Loop.cs"] {
            assert_eq!(
                canonical_confined(&scope, &root.join(link)),
                Err(ConfineError::Outside),
                "{link} must be refused as outside"
            );
            assert_eq!(
                canonical_confined(&scope, &root.join(link).join("child.cs")),
                Err(ConfineError::Outside),
                "{link} as an intermediate component must be refused as outside"
            );
        }
        // Nothing inside the workspace changed hands. (`absent.cs`: `missing.cs` would be the same
        // name as the dangling `Missing.cs` on a case-insensitive filesystem.)
        assert_eq!(
            canonical_confined(&scope, &root.join("absent.cs")),
            Err(ConfineError::Missing)
        );
        assert_eq!(
            canonical_confined(&scope, &root.join("absent/deep.cs")),
            Err(ConfineError::Missing)
        );
    }

    #[test]
    fn reveal_selects_the_item_in_finder_on_macos() {
        let (program, args) = reveal_command("macos", Path::new("/work/App.cs")).unwrap();
        assert_eq!(program, "open");
        assert_eq!(args, vec!["-R".to_string(), "/work/App.cs".to_string()]);
    }

    #[test]
    fn reveal_is_a_clear_error_off_macos() {
        let error = reveal_command("windows", Path::new("C:\\work\\App.cs")).unwrap_err();
        assert_eq!(error.code, "unsupported_platform");
        assert!(
            error.message.contains("only implemented on macOS"),
            "{}",
            error.message
        );
    }
}
