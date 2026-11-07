//! Workspace file listing and text search. Both read files only: no process, no code from
//! the repository, no trust gate. Caps are part of the response, never silent.

use serde::Serialize;
use std::fs;
use std::path::Path;

pub const FILE_LIMIT: usize = 5_000;
pub const TEXT_FILE_LIMIT: usize = 200;
pub const TEXT_MATCH_LIMIT: usize = 2_000;
pub const TEXT_FILE_MAX_BYTES: u64 = 1_048_576;
pub const IGNORED_DIRS: &[&str] = &["bin", "obj", ".git", "node_modules"];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileList {
    pub files: Vec<String>,
    pub total: usize,
    pub truncated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextMatch {
    pub line: u64,
    pub character: u64,
    pub text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextFile {
    pub path: String,
    pub matches: Vec<TextMatch>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextSearch {
    pub files: Vec<TextFile>,
    pub file_count: usize,
    pub match_count: usize,
    pub truncated: bool,
}

/// Walks `root` depth-first, sorted for a stable answer, skipping build output, VCS data and
/// dependency folders. `visit` returns false to stop early; the walk then reports it.
fn walk(root: &Path, visit: &mut impl FnMut(&Path, &str) -> bool) -> bool {
    fn rec(dir: &Path, root: &Path, visit: &mut impl FnMut(&Path, &str) -> bool) -> bool {
        let Ok(read) = fs::read_dir(dir) else {
            return true;
        };
        let mut entries: Vec<_> = read.filter_map(Result::ok).collect();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if IGNORED_DIRS.contains(&name.as_ref()) {
                    continue;
                }
                if !rec(&entry.path(), root, visit) {
                    return false;
                }
                continue;
            }
            if !file_type.is_file() {
                continue;
            }
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if !visit(&path, &rel) {
                return false;
            }
        }
        true
    }
    rec(root, root, visit)
}

pub fn list_files(root: &Path, limit: usize) -> FileList {
    let mut files = Vec::new();
    let mut total = 0;
    let complete = walk(root, &mut |_, rel| {
        total += 1;
        if files.len() < limit {
            files.push(rel.to_string());
        }
        total <= limit
    });
    FileList {
        truncated: !complete || total > files.len(),
        files,
        total,
    }
}

/// One directory level for "Show all files". Hidden = ignored folder or dotfile; the UI decides
/// "no index" for files from the evaluated sources. `.git` is never listed. Dirs first, then A-Z.
/// `symlink` and `dir` describe the **entry itself** (`DirEntry::file_type` never follows a link),
/// so a link is listed as a link object and its target is never consulted to choose the glyph.
pub fn list_dir(root: &Path, dir: &Path) -> std::io::Result<Vec<serde_json::Value>> {
    let mut entries: Vec<(bool, bool, String)> = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name == ".git" {
            continue;
        }
        let file_type = entry.file_type()?;
        entries.push((file_type.is_dir(), file_type.is_symlink(), name));
    }
    entries.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then_with(|| a.2.to_lowercase().cmp(&b.2.to_lowercase()))
    });
    Ok(entries
        .into_iter()
        .map(|(is_dir, symlink, name)| {
            let rel = dir.join(&name);
            let rel = crate::fsutil::portable_path(rel.strip_prefix(root).unwrap_or(&rel));
            let hidden = name.starts_with('.') || (is_dir && IGNORED_DIRS.contains(&name.as_str()));
            serde_json::json!({ "name": name, "path": rel, "dir": is_dir, "symlink": symlink, "hidden": hidden })
        })
        .collect())
}

fn occurrences(line: &str, needle: &str, case_sensitive: bool) -> Vec<usize> {
    if needle.is_empty() {
        return Vec::new();
    }
    let haystack = if case_sensitive {
        line.to_string()
    } else {
        line.to_lowercase()
    };
    let needle = if case_sensitive {
        needle.to_string()
    } else {
        needle.to_lowercase()
    };
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(index) = haystack[from..].find(&needle) {
        let at = from + index;
        found.push(at);
        from = at + needle.len().max(1);
    }
    found
}

pub fn search_text(
    root: &Path,
    query: &str,
    case_sensitive: bool,
    file_limit: usize,
    match_limit: usize,
) -> TextSearch {
    let mut files: Vec<TextFile> = Vec::new();
    let mut match_count = 0;
    let mut truncated = false;
    let complete = walk(root, &mut |path, rel| {
        let Ok(metadata) = fs::metadata(path) else {
            return true;
        };
        if metadata.len() > TEXT_FILE_MAX_BYTES {
            return true;
        }
        let Ok(content) = fs::read_to_string(path) else {
            // Binary or not UTF-8: skipped, never listed with invented text.
            return true;
        };
        let mut matches = Vec::new();
        for (index, line) in content.lines().enumerate() {
            for column in occurrences(line, query, case_sensitive) {
                let remaining = match_limit.saturating_sub(match_count + matches.len());
                if remaining == 0 {
                    truncated = true;
                    break;
                }
                matches.push(TextMatch {
                    line: index as u64,
                    character: line
                        .char_indices()
                        .take_while(|(at, _)| *at < column)
                        .count() as u64,
                    text: line.trim_end().to_string(),
                });
            }
            if truncated {
                break;
            }
        }
        if matches.is_empty() {
            return true;
        }
        if files.len() >= file_limit {
            truncated = true;
            return false;
        }
        match_count += matches.len();
        files.push(TextFile {
            path: rel.to_string(),
            matches,
        });
        !truncated
    });
    if !complete {
        truncated = true;
    }
    TextSearch {
        file_count: files.len(),
        match_count,
        files,
        truncated,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> (tempfile::TempDir, std::path::PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        (temp, root)
    }

    #[test]
    fn files_skip_build_output_vcs_and_dependencies() {
        let (_temp, root) = root();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("bin/Debug")).unwrap();
        fs::create_dir_all(root.join("obj")).unwrap();
        fs::create_dir_all(root.join(".git/objects")).unwrap();
        fs::create_dir_all(root.join("src/node_modules/pkg")).unwrap();
        fs::write(root.join("src/Program.cs"), "class Program {}\n").unwrap();
        fs::write(root.join("bin/Debug/Program.dll"), "binary").unwrap();
        fs::write(root.join("obj/project.assets.json"), "{}").unwrap();
        fs::write(root.join(".git/config"), "[core]\n").unwrap();
        fs::write(root.join("src/node_modules/pkg/index.js"), "module\n").unwrap();

        let list = list_files(&root, 100);
        assert_eq!(list.files, vec!["src/Program.cs"]);
        assert_eq!(list.total, 1);
        assert!(!list.truncated);
    }

    #[test]
    fn files_stop_at_the_cap_and_declare_it() {
        let (_temp, root) = root();
        for name in ["c.cs", "a.cs", "b.cs"] {
            fs::write(root.join(name), "x\n").unwrap();
        }
        let list = list_files(&root, 2);
        assert_eq!(list.files, vec!["a.cs", "b.cs"]);
        assert_eq!(list.total, 3);
        assert!(list.truncated);

        let all = list_files(&root, 3);
        assert!(!all.truncated, "exactly at the cap is not truncated");
        assert_eq!(all.total, 3);
    }

    #[test]
    fn text_groups_by_file_with_line_and_column() {
        let (_temp, root) = root();
        fs::write(
            root.join("a.cs"),
            "class Order\n{\n    Order Create() => new Order();\n}\n",
        )
        .unwrap();
        fs::write(root.join("b.cs"), "class Other {}\n").unwrap();

        let search = search_text(&root, "order", false, 200, 2_000);
        assert_eq!(search.file_count, 1);
        assert_eq!(search.match_count, 3);
        assert!(!search.truncated);
        let file = &search.files[0];
        assert_eq!(file.path, "a.cs");
        assert_eq!(
            file.matches
                .iter()
                .map(|item| (item.line, item.character))
                .collect::<Vec<_>>(),
            vec![(0, 6), (2, 4), (2, 26)]
        );

        let sensitive = search_text(&root, "Order", true, 200, 2_000);
        assert_eq!(sensitive.match_count, 3);
        let absent = search_text(&root, "ORDER", true, 200, 2_000);
        assert_eq!(absent.match_count, 0);
        assert!(!absent.truncated);
    }

    #[test]
    fn text_columns_count_characters_not_bytes() {
        let (_temp, root) = root();
        fs::write(root.join("accent.cs"), "ação needle\n").unwrap();
        let search = search_text(&root, "needle", false, 200, 2_000);
        assert_eq!(search.files[0].matches[0].character, 5);
    }

    #[test]
    fn text_skips_large_and_binary_files() {
        let (_temp, root) = root();
        fs::write(root.join("big.cs"), "needle\n".repeat(200_000)).unwrap();
        fs::write(root.join("binary.cs"), [0xff, 0xfe, 0x00, 0x01]).unwrap();
        fs::write(root.join("small.cs"), "needle\n").unwrap();

        let search = search_text(&root, "needle", false, 200, 2_000);
        assert_eq!(search.files.len(), 1);
        assert_eq!(search.files[0].path, "small.cs");
    }

    #[test]
    fn text_declares_file_and_match_caps() {
        let (_temp, root) = root();
        fs::write(root.join("a.cs"), "needle needle needle\n").unwrap();
        fs::write(root.join("b.cs"), "needle\n").unwrap();

        let matches = search_text(&root, "needle", false, 200, 2);
        assert_eq!(matches.match_count, 2);
        assert!(matches.truncated);

        let files = search_text(&root, "needle", false, 1, 2_000);
        assert_eq!(files.file_count, 1);
        assert_eq!(files.match_count, 3);
        assert!(files.truncated);
    }
}
