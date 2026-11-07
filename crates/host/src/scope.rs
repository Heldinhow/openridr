//! The directories a workspace may touch.
//!
//! A workspace's root is the directory of its solution (or project) file, but a `.sln` may name
//! projects that live elsewhere (`..\lib\Lib.csproj`). Those directories are part of the workspace
//! **scope**: the host may read, write, analyse, build and debug inside them, and nowhere else.
//!
//! The scope is derived only from the workspace file the user trusted, never from a caller-supplied
//! path, so a request cannot widen it. Each entry is canonical, which is what lets every frontier
//! decide containment on real paths (a symlinked project directory contributes its real target).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Extra directories beyond the root. A solution that names more is refused project by project
/// instead of turning a large part of the disk into a scope.
pub const MAX_EXTRA_ROOTS: usize = 64;

/// Ordered canonical directories: the root first, then the projects' directories in file order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope {
    roots: Vec<PathBuf>,
}

/// The scope plus the projects that could not join it, keyed by canonical project path, with the
/// reason each one is reported with.
#[derive(Debug, Clone, Default)]
pub struct ResolvedScope {
    pub scope: Scope,
    pub refused: BTreeMap<PathBuf, String>,
}

impl Scope {
    pub fn new(root: &Path) -> Self {
        Self {
            roots: vec![root.to_path_buf()],
        }
    }

    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// Whether `path` is inside one of the scope directories. Both sides are expected canonical,
    /// and the comparison is by component, so `/work/app` does not contain `/work/applications`.
    pub fn contains(&self, path: &Path) -> bool {
        self.roots().iter().any(|root| path.starts_with(root))
    }

    /// The innermost scope directory that contains `path`. Used where a path has to be described
    /// relative to the directory it belongs to (build output components, tree rows).
    pub fn entry_of(&self, path: &Path) -> Option<&Path> {
        self.roots()
            .iter()
            .filter(|root| path.starts_with(root))
            .max_by_key(|root| root.components().count())
            .map(PathBuf::as_path)
    }

    /// The scope as the snapshot and the UI see it: absolute canonical directories, root first.
    pub fn paths(&self) -> Vec<String> {
        self.roots()
            .iter()
            .map(|root| root.to_string_lossy().to_string())
            .collect()
    }
}

/// Resolves the scope of a workspace from the project paths its file names (relative to the root,
/// or absolute). A path that does not exist is skipped: it never becomes a scope entry, and the
/// evaluation reports it as not found.
pub fn resolve(root: &Path, project_paths: impl IntoIterator<Item = String>) -> ResolvedScope {
    let mut scope = Scope::new(root);
    let mut refused = BTreeMap::new();
    let mut extra = 0;
    for raw in project_paths {
        // The `.sln` text uses backslashes; `Path::join` with an absolute path keeps it absolute.
        let candidate = root.join(raw.replace('\\', "/"));
        let Ok(canonical) = fs::canonicalize(&candidate) else {
            continue;
        };
        let Some(dir) = canonical.parent() else {
            continue;
        };
        if let Some(reason) = directory_refusal(dir) {
            refused.insert(canonical, reason);
            continue;
        }
        if scope.contains(dir) {
            continue;
        }
        if extra >= MAX_EXTRA_ROOTS {
            refused.insert(
                canonical,
                format!(
                    "The solution names more than {MAX_EXTRA_ROOTS} project directories outside its own; this project was not evaluated."
                ),
            );
            continue;
        }
        scope.roots.push(dir.to_path_buf());
        extra += 1;
    }
    ResolvedScope { scope, refused }
}

/// Why a project directory cannot join the scope, if it cannot. `/` (or `C:\`) as a project
/// directory would put the whole disk in scope.
fn directory_refusal(dir: &Path) -> Option<String> {
    dir.parent()
        .is_none()
        .then(|| "The project lives at the filesystem root; it was not evaluated.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dirs(paths: &[PathBuf]) -> Vec<String> {
        paths
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect()
    }

    #[test]
    fn the_root_alone_is_a_scope_that_contains_only_itself() {
        let scope = Scope::new(Path::new("/work/app"));
        assert!(scope.contains(Path::new("/work/app/src/App.cs")));
        assert!(scope.contains(Path::new("/work/app")));
        assert!(!scope.contains(Path::new("/work/applications/App.cs")));
        assert!(!scope.contains(Path::new("/work/lib/Lib.cs")));
        assert_eq!(scope.roots(), &[PathBuf::from("/work/app")]);
    }

    #[test]
    fn a_nested_entry_does_not_duplicate_and_the_innermost_wins() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("app");
        let lib = temp.path().join("lib");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::create_dir_all(&lib).unwrap();
        fs::write(root.join("App.csproj"), "<Project />").unwrap();
        fs::write(root.join("sub/Sub.csproj"), "<Project />").unwrap();
        fs::write(lib.join("Lib.csproj"), "<Project />").unwrap();

        let resolved = resolve(
            &fs::canonicalize(&root).unwrap(),
            [
                "App.csproj".to_string(),
                "sub/Sub.csproj".to_string(),
                "../lib/Lib.csproj".to_string(),
            ],
        );
        let canonical_root = fs::canonicalize(&root).unwrap();
        let canonical_lib = fs::canonicalize(&lib).unwrap();
        assert_eq!(
            dirs(resolved.scope.roots()),
            dirs(&[canonical_root.clone(), canonical_lib.clone()]),
            "the project inside the root adds nothing and the outside one is appended"
        );
        assert!(resolved.refused.is_empty());
        assert!(resolved.scope.contains(&canonical_lib.join("Greeting.cs")));
        assert!(resolved
            .scope
            .contains(&canonical_root.join("sub/Sub.csproj")));
    }

    #[test]
    fn a_project_above_the_root_widens_the_scope_instead_of_being_special_cased() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("app");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("App.csproj"), "<Project />").unwrap();
        fs::write(temp.path().join("Sibling.csproj"), "<Project />").unwrap();

        let resolved = resolve(
            &fs::canonicalize(&root).unwrap(),
            ["../Sibling.csproj".to_string()],
        );
        let parent = fs::canonicalize(temp.path()).unwrap();
        assert_eq!(resolved.scope.roots().len(), 2);
        assert!(resolved.scope.contains(&parent.join("Sibling.csproj")));
        assert_eq!(
            resolved.scope.roots()[0],
            fs::canonicalize(&root).unwrap(),
            "the root stays first"
        );
    }

    #[test]
    fn a_missing_project_never_becomes_a_scope_entry() {
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        let resolved = resolve(&root, ["../nope/Missing.csproj".to_string()]);
        assert_eq!(resolved.scope.roots(), &[root]);
        assert!(resolved.refused.is_empty());
    }

    #[test]
    fn the_scope_stops_at_the_extra_root_cap_and_says_why() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("app");
        fs::create_dir_all(&root).unwrap();
        let mut paths = Vec::new();
        for index in 0..=MAX_EXTRA_ROOTS {
            let dir = temp.path().join(format!("p{index}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("P.csproj"), "<Project />").unwrap();
            paths.push(format!("../p{index}/P.csproj"));
        }
        let resolved = resolve(&fs::canonicalize(&root).unwrap(), paths);
        assert_eq!(resolved.scope.roots().len(), MAX_EXTRA_ROOTS + 1);
        assert_eq!(resolved.refused.len(), 1);
        let reason = resolved.refused.values().next().unwrap();
        assert!(reason.contains(&MAX_EXTRA_ROOTS.to_string()), "{reason}");
    }

    #[test]
    fn a_project_at_the_filesystem_root_is_refused_with_a_reason() {
        let reason = directory_refusal(Path::new("/")).expect("the filesystem root is refused");
        assert!(reason.contains("filesystem root"), "{reason}");
        assert!(
            directory_refusal(Path::new("/work/app")).is_none(),
            "an ordinary directory is accepted"
        );
    }
}
