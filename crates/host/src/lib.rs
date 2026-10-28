mod error;
mod outline;

pub use error::{HostError, HostResult};
pub use outline::{GlobalJson, Outline, OutlineProject};

use outline::build_outline;
use serde::Serialize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Methods that will eventually start a process or touch executable project state.
/// Until trust is granted they must fail before doing anything observable.
pub const EXECUTABLE_METHODS: &[&str] = &[
    "evaluation.run",
    "restore.start",
    "lsp.start",
    "tools.install",
    "terminal.start",
    "build.start",
    "run.start",
    "debug.start",
    "tests.discover",
    "tests.run",
];

pub struct Host {
    inner: Mutex<Inner>,
}

struct Inner {
    workspace: Option<Workspace>,
    trusted: bool,
}

struct Workspace {
    path: PathBuf,
    root: PathBuf,
    name: String,
    kind: WorkspaceKind,
    outline: Outline,
}

#[derive(Clone, Copy)]
enum WorkspaceKind {
    Solution,
    Project,
}

impl WorkspaceKind {
    fn as_str(self) -> &'static str {
        match self {
            WorkspaceKind::Solution => "solution",
            WorkspaceKind::Project => "project",
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    path: String,
    name: String,
    kind: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    home: String,
    platform: &'static str,
    arch: &'static str,
    version: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkspaceView {
    path: String,
    name: String,
    kind: &'static str,
    mode: &'static str,
    candidates: Vec<Candidate>,
}

#[derive(Serialize)]
struct TrustView {
    granted: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Snapshot {
    app: AppInfo,
    workspace: Option<WorkspaceView>,
    outline: Option<Outline>,
    trust: TrustView,
}

impl Default for Host {
    fn default() -> Self {
        Self::new()
    }
}

impl Host {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Inner {
                workspace: None,
                trusted: false,
            }),
        }
    }

    pub fn dispatch(&self, method: &str, params: Value) -> HostResult<Value> {
        if EXECUTABLE_METHODS.contains(&method) {
            return self.executable_stub(method);
        }

        match method {
            "app.state" | "workspace.state" => {
                let inner = self.inner.lock().unwrap();
                Ok(serde_json::to_value(self.snapshot(&inner)).unwrap())
            }
            "workspace.open" => self.open_workspace(params),
            "workspace.close" => {
                let mut inner = self.inner.lock().unwrap();
                inner.workspace = None;
                inner.trusted = false;
                Ok(serde_json::to_value(self.snapshot(&inner)).unwrap())
            }
            "file.read" => self.read_file(params),
            "file.write" => self.write_file(params),
            _ => Err(HostError::new(
                "not_found",
                format!("Unknown host method: {method}"),
            )),
        }
    }

    fn executable_stub(&self, method: &str) -> HostResult<Value> {
        let trusted = self.inner.lock().unwrap().trusted;
        if trusted {
            Err(HostError::new(
                "invalid_state",
                format!("{method} is not implemented in this build."),
            ))
        } else {
            Err(HostError::new(
                "trust_required",
                format!("{method} requires workspace trust; no process was started."),
            ))
        }
    }

    fn snapshot(&self, inner: &Inner) -> Snapshot {
        Snapshot {
            app: AppInfo {
                home: std::env::var("HOME").unwrap_or_default(),
                platform: std::env::consts::OS,
                arch: std::env::consts::ARCH,
                version: env!("CARGO_PKG_VERSION"),
            },
            workspace: inner.workspace.as_ref().map(|workspace| WorkspaceView {
                path: workspace.path.to_string_lossy().to_string(),
                name: workspace.name.clone(),
                kind: workspace.kind.as_str(),
                mode: if inner.trusted { "trusted" } else { "restricted" },
                candidates: Vec::new(),
            }),
            outline: inner.workspace.as_ref().map(|workspace| workspace.outline.clone()),
            trust: TrustView {
                granted: inner.trusted,
            },
        }
    }

    fn open_workspace(&self, params: Value) -> HostResult<Value> {
        let raw = required_str(&params, "path")?;
        let requested = Path::new(&raw);
        if !requested.is_absolute() {
            return Err(HostError::new(
                "path_not_allowed",
                "Workspace path must be absolute.",
            ));
        }
        let canonical = fs::canonicalize(requested).map_err(|_| {
            HostError::new("not_found", format!("Workspace path not found: {raw}"))
        })?;
        let workspace_path = if canonical.is_dir() {
            resolve_directory(&canonical)?
        } else {
            workspace_file(&canonical)?
        };

        let workspace = load_workspace(&workspace_path)?;
        let mut inner = self.inner.lock().unwrap();
        inner.workspace = Some(workspace);
        inner.trusted = false;
        Ok(serde_json::to_value(self.snapshot(&inner)).unwrap())
    }

    fn read_file(&self, params: Value) -> HostResult<Value> {
        let raw = required_str(&params, "path")?;
        let inner = self.inner.lock().unwrap();
        let (path, rel) = resolve_in_workspace(&inner, &raw)?;
        let content = fs::read_to_string(&path).map_err(|error| {
            HostError::new(
                "operation_failed",
                format!("Could not read {rel} as UTF-8 text: {error}"),
            )
        })?;
        let hash = content_hash(&content);
        Ok(json!({ "path": rel, "content": content, "hash": hash }))
    }

    fn write_file(&self, params: Value) -> HostResult<Value> {
        let raw = required_str(&params, "path")?;
        let content = required_str(&params, "content")?;
        let expected_hash = required_str(&params, "expected_hash")?;
        let inner = self.inner.lock().unwrap();
        let (path, rel) = resolve_in_workspace(&inner, &raw)?;

        let current = fs::read_to_string(&path).map_err(|error| {
            HostError::new(
                "operation_failed",
                format!("Could not read {rel} before saving: {error}"),
            )
        })?;
        let current_hash = content_hash(&current);
        if current_hash != expected_hash {
            return Err(HostError::with_details(
                "conflict",
                format!("{rel} changed on disk since it was read; the buffer was not saved."),
                json!({ "currentContent": current, "currentHash": current_hash }),
            ));
        }

        let temp = path.with_file_name(format!(
            ".openridr-save-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        if let Err(error) = fs::write(&temp, content.as_bytes()) {
            return Err(HostError::new(
                "operation_failed",
                format!("Could not write {rel}: {error}"),
            ));
        }
        if let Err(error) = fs::rename(&temp, &path) {
            let _ = fs::remove_file(&temp);
            return Err(HostError::new(
                "operation_failed",
                format!("Could not replace {rel}: {error}"),
            ));
        }
        Ok(json!({ "path": rel, "hash": content_hash(&content) }))
    }
}

fn required_str(params: &Value, key: &str) -> HostResult<String> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            HostError::new(
                "invalid_state",
                format!("Missing string parameter `{key}`."),
            )
        })
}

fn workspace_file(path: &Path) -> HostResult<PathBuf> {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("sln") | Some("csproj") => Ok(path.to_path_buf()),
        _ => Err(HostError::new(
            "path_not_allowed",
            "Open a .sln, a .csproj or a directory that contains one.",
        )),
    }
}

fn resolve_directory(dir: &Path) -> HostResult<PathBuf> {
    let solutions = files_with_extension(dir, "sln");
    if solutions.len() == 1 {
        return Ok(solutions[0].clone());
    }
    if solutions.len() > 1 {
        return Err(ambiguous_workspace(dir, &solutions, "solution"));
    }

    let projects = files_with_extension(dir, "csproj");
    match projects.len() {
        1 => Ok(projects[0].clone()),
        0 => Err(HostError::new(
            "not_found",
            format!(
                "No .sln or .csproj found in {}.",
                dir.to_string_lossy()
            ),
        )),
        _ => Err(ambiguous_workspace(dir, &projects, "project")),
    }
}

fn files_with_extension(dir: &Path, extension: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.symlink_metadata().is_ok_and(|meta| meta.is_file())
                && path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
        })
        .collect();
    files.sort();
    files
}

fn ambiguous_workspace(dir: &Path, files: &[PathBuf], kind: &'static str) -> HostError {
    let candidates: Vec<Candidate> = files
        .iter()
        .map(|path| Candidate {
            path: path.to_string_lossy().to_string(),
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default(),
            kind,
        })
        .collect();
    HostError::with_details(
        "ambiguous_workspace",
        format!(
            "Multiple solutions or projects found in {}; choose one explicitly.",
            dir.to_string_lossy()
        ),
        json!({ "candidates": candidates }),
    )
}

fn load_workspace(path: &Path) -> HostResult<Workspace> {
    let kind = match path.extension().and_then(|ext| ext.to_str()) {
        Some("sln") => WorkspaceKind::Solution,
        Some("csproj") => WorkspaceKind::Project,
        _ => {
            return Err(HostError::new(
                "path_not_allowed",
                "Workspace must be a .sln or .csproj file.",
            ))
        }
    };
    let root = path
        .parent()
        .ok_or_else(|| HostError::new("path_not_allowed", "Workspace has no parent directory."))?
        .to_path_buf();
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    let outline = build_outline(&root, path, matches!(kind, WorkspaceKind::Solution));
    Ok(Workspace {
        path: path.to_path_buf(),
        root,
        name,
        kind,
        outline,
    })
}

fn resolve_in_workspace(inner: &Inner, raw: &str) -> HostResult<(PathBuf, String)> {
    let workspace = inner.workspace.as_ref().ok_or_else(|| {
        HostError::new("invalid_state", "No workspace is open.")
    })?;
    if raw.trim().is_empty() {
        return Err(HostError::new(
            "path_not_allowed",
            "File path is empty.",
        ));
    }
    let requested = Path::new(raw);
    let candidate = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        workspace.root.join(requested)
    };
    let canonical = fs::canonicalize(&candidate)
        .map_err(|_| HostError::new("not_found", format!("File not found: {raw}")))?;
    if !canonical.starts_with(&workspace.root) {
        return Err(HostError::new(
            "outside_workspace",
            format!("Path is outside the workspace: {raw}"),
        ));
    }
    if !canonical.is_file() {
        return Err(HostError::new(
            "path_not_allowed",
            format!("Not a file: {raw}"),
        ));
    }
    let rel = canonical
        .strip_prefix(&workspace.root)
        .unwrap_or(&canonical)
        .to_string_lossy()
        .replace('\\', "/");
    Ok((canonical, rel))
}

fn content_hash(content: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in content.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64:{hash:016x}")
}
