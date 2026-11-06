//! Git through the `git` executable found on PATH.
//!
//! Every invocation is an argv list with the repository root as cwd; nothing goes through a
//! shell. Reads return data states (`not_a_repository`, `git_missing`) instead of errors so a
//! surface can explain itself. Writes that would lose work refuse a dirty tree or an
//! in-progress merge/rebase/cherry-pick, and OpenRidr never auto-stashes. Credentials stay in
//! the user's git credential helper; URLs in captured output are masked.

use crate::{HostError, HostResult};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub const LOG_LIMIT: usize = 200;
pub const COMMIT_FILES_LIMIT: usize = 200;
pub const DIFF_LIMIT: usize = 240_000;
pub const OUTPUT_LIMIT: usize = 32_000;
/// Full-file sides are a read for the diff panel, never a guarantee: above these the read falls
/// back to the bounded patch, and the panel says why.
pub const FULL_FILE_BYTES: usize = 1_048_576;
pub const FULL_FILE_LINES: usize = 30_000;
const BINARY_PROBE: usize = 8_192;
/// stdout ceilings: the legacy patch keeps its 240 000-character contract (a UTF-8 character is
/// at most 4 bytes, so the byte ceiling can never cut a shorter contract), the `-U0` ranges only
/// need headers and get a generous bound, and stderr is drained but barely kept.
const PATCH_BYTES: usize = DIFF_LIMIT * 4;
const HUNK_BYTES: usize = 8 * 1024 * 1024;
const STDERR_BYTES: usize = 32_000;
/// `git.ignored` answers only for what the tree is showing: a batch of visible paths, bounded on
/// both the input (256 paths, 64 KiB of pathnames) and the output (256 KiB of porcelain records).
const IGNORED_PATHS: usize = 256;
const IGNORED_PATH_BYTES: usize = 64 * 1024;
const IGNORED_BYTES: usize = 256 * 1024;
/// Unique suffix for the private decoded-diff files (`pid` plus this counter).
static TEMP_SEQ: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFile {
    pub path: String,
    pub original_path: Option<String>,
    pub index: String,
    pub worktree: String,
    pub untracked: bool,
    pub staged: bool,
    pub conflicted: bool,
    pub label: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitStatus {
    pub state: &'static str,
    pub message: Option<String>,
    pub root: Option<String>,
    pub branch: Option<String>,
    pub detached: bool,
    pub initial: bool,
    pub upstream: Option<String>,
    pub upstream_gone: bool,
    pub ahead: u64,
    pub behind: u64,
    pub operation: Option<&'static str>,
    pub clean: bool,
    pub files: Vec<GitFile>,
    /// Configured remotes so Push can be disabled with a reason before any fetch.
    pub remotes: Vec<String>,
}

impl GitStatus {
    fn unavailable(state: &'static str, message: impl Into<String>) -> Self {
        Self {
            state,
            message: Some(message.into()),
            root: None,
            branch: None,
            detached: false,
            initial: false,
            upstream: None,
            upstream_gone: false,
            ahead: 0,
            behind: 0,
            operation: None,
            clean: false,
            files: Vec::new(),
            remotes: Vec::new(),
        }
    }
}

/// Which of the requested visible paths the repository's ignore rules match. `ignored` echoes the
/// caller's workspace-relative paths, never a resolved target.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitIgnored {
    pub state: &'static str,
    pub message: Option<String>,
    pub ignored: Vec<String>,
}

/// The `git` that PATH would run, reported read-only to Settings. No repository is opened.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub state: &'static str,
    pub path: Option<String>,
    pub version: Option<String>,
    pub message: Option<String>,
}

pub fn path() -> Option<String> {
    std::env::var_os("PATH")
        .map(|value| {
            std::env::split_paths(&value)
                .filter(|dir| !dir.as_os_str().is_empty())
                .map(|dir| dir.join(if cfg!(windows) { "git.exe" } else { "git" }))
                .find(|candidate| candidate.is_file())
                .map(|candidate| candidate.to_string_lossy().into_owned())
        })
        .flatten()
}

pub fn info() -> GitInfo {
    let path = path();
    let version = Command::new("git")
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string());
    match (path, version) {
        (Some(path), Some(version)) => GitInfo {
            state: "found",
            path: Some(path),
            version: Some(version),
            message: None,
        },
        _ => GitInfo {
            state: "missing",
            path: None,
            version: None,
            message: Some("git was not found on PATH.".to_string()),
        },
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Branch {
    pub name: String,
    pub upstream: Option<String>,
    pub ahead: u64,
    pub behind: u64,
    pub current: bool,
    pub remote: bool,
    pub gone: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranches {
    pub state: &'static str,
    pub message: Option<String>,
    pub current: Option<String>,
    pub local: Vec<Branch>,
    pub remote: Vec<Branch>,
    /// Configured remotes from `git remote`, so Push can be enabled/disabled honestly even
    /// before anything was fetched.
    pub remotes: Vec<String>,
}

impl GitBranches {
    fn unavailable(state: &'static str, message: impl Into<String>) -> Self {
        Self {
            state,
            message: Some(message.into()),
            current: None,
            local: Vec::new(),
            remote: Vec::new(),
            remotes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitFile {
    pub path: String,
    pub original_path: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub date: String,
    pub subject: String,
    pub parents: Vec<String>,
    pub files: Vec<CommitFile>,
    pub files_truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitLog {
    pub state: &'static str,
    pub message: Option<String>,
    pub limit: usize,
    pub truncated: bool,
    pub commits: Vec<GitCommit>,
}

impl GitLog {
    fn unavailable(state: &'static str, message: impl Into<String>) -> Self {
        Self {
            state,
            message: Some(message.into()),
            limit: LOG_LIMIT,
            truncated: false,
            commits: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDiff {
    pub state: &'static str,
    pub message: Option<String>,
    pub path: String,
    pub staged: bool,
    pub diff: String,
    pub truncated: bool,
}

impl GitDiff {
    fn unavailable(state: &'static str, message: impl Into<String>, path: &str, staged: bool) -> Self {
        Self {
            state,
            message: Some(message.into()),
            path: path.to_string(),
            staged,
            diff: String::new(),
            truncated: false,
        }
    }
}

/// One side of a full-file diff: the text is present only when the side exists and the host can
/// show it whole. `text: None` with `exists: true` is a fact for the fallback, never an empty file.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffSide {
    pub exists: bool,
    pub text: Option<String>,
}

/// A changed block as `git diff -U0` reports it: the base lines `oldStart..oldStart+oldCount`
/// pair with the current lines `newStart..newStart+newCount`. A count of zero at `start` means
/// the block is inserted after line `start` (0 = before the first line).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffCommit {
    pub hash: String,
    pub short_hash: String,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileDiff {
    pub state: &'static str,
    pub message: Option<String>,
    pub path: String,
    pub staged: bool,
    pub base: DiffSide,
    pub current: DiffSide,
    pub hunks: Vec<DiffHunk>,
    pub patch: String,
    pub truncated: bool,
    pub full: bool,
    pub full_reason: Option<&'static str>,
    pub commit: Option<DiffCommit>,
}

impl GitFileDiff {
    fn unavailable(state: &'static str, message: impl Into<String>, path: &str, staged: bool) -> Self {
        Self {
            state,
            message: Some(message.into()),
            path: path.to_string(),
            staged,
            base: DiffSide {
                exists: false,
                text: None,
            },
            current: DiffSide {
                exists: false,
                text: None,
            },
            hunks: Vec::new(),
            patch: String::new(),
            truncated: false,
            full: false,
            full_reason: None,
            commit: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Committed {
    pub hash: String,
    pub short_hash: String,
    pub summary: String,
    pub amend: bool,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub state: &'static str,
    pub message: Option<String>,
    pub output: String,
    pub status: GitStatus,
    pub commit: Option<Committed>,
    pub remote: Option<String>,
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u64,
}

struct Repo {
    top: PathBuf,
    git_dir: PathBuf,
}

#[derive(Debug)]
enum OpenError {
    NotRepository,
    Missing,
    Failed(String),
}

struct Output {
    code: Option<i32>,
    stdout: String,
    stderr: String,
    truncated: bool,
}

impl Output {
    fn ok(&self) -> bool {
        self.code == Some(0)
    }
}

/// Raw bounded capture: blob content must not go through a lossy text conversion before the
/// host decides whether it is text at all.
struct Captured {
    code: Option<i32>,
    stdout: Vec<u8>,
    stderr: String,
    truncated: bool,
}

impl Captured {
    fn text(self) -> Output {
        Output {
            code: self.code,
            stdout: String::from_utf8_lossy(&self.stdout).into_owned(),
            stderr: self.stderr,
            truncated: self.truncated,
        }
    }
}

enum SpawnError {
    Missing,
    Failed(String),
}

/// Raw stdout, for blob contents that must not go through lossy UTF-8 conversion before the
/// host decides whether they are text at all.
fn spawn_bytes(cwd: &Path, args: &[&str]) -> Result<(Option<i32>, Vec<u8>, String), SpawnError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        // A host without a terminal must fail instead of waiting on a username prompt. The
        // credential helper still answers, so the user's stored credentials keep working.
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => SpawnError::Missing,
            _ => SpawnError::Failed(error.to_string()),
        })?;
    Ok((
        output.status.code(),
        output.stdout,
        String::from_utf8_lossy(&output.stderr).into_owned(),
    ))
}

fn run(cwd: &Path, args: &[&str]) -> Result<Output, SpawnError> {
    let (code, stdout, stderr) = spawn_bytes(cwd, args)?;
    Ok(Output {
        code,
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr,
        truncated: false,
    })
}

/// A git run whose stdout never exceeds `limit` bytes. Once the ceiling is hit the child is
/// killed and `truncated` is set, so a runaway diff cannot become an unbounded allocation in the
/// host; stderr is drained on its own thread (bounded copy) so a chatty failure cannot deadlock.
fn spawn_bounded(cwd: &Path, args: &[&str], limit: usize) -> Result<Captured, SpawnError> {
    let mut child = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => SpawnError::Missing,
            _ => SpawnError::Failed(error.to_string()),
        })?;
    let mut stdout = child.stdout.take().expect("piped stdout");
    let stderr = child.stderr.take().expect("piped stderr");
    let drained = std::thread::spawn(move || {
        let mut kept = Vec::new();
        let mut buffer = [0u8; 4096];
        let mut reader = stderr;
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    if kept.len() < STDERR_BYTES {
                        let room = STDERR_BYTES - kept.len();
                        kept.extend_from_slice(&buffer[..count.min(room)]);
                    }
                }
            }
        }
        kept
    });
    let mut captured = Vec::new();
    let mut truncated = false;
    let mut buffer = [0u8; 8192];
    loop {
        match stdout.read(&mut buffer) {
            Ok(0) => break,
            Ok(count) => {
                if captured.len() + count > limit {
                    let room = limit.saturating_sub(captured.len());
                    captured.extend_from_slice(&buffer[..room]);
                    truncated = true;
                    let _ = child.kill();
                    break;
                }
                captured.extend_from_slice(&buffer[..count]);
            }
            Err(_) => break,
        }
    }
    let status = child.wait().ok();
    let stderr = drained.join().unwrap_or_default();
    Ok(Captured {
        code: status.and_then(|status| status.code()),
        stdout: captured,
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        truncated,
    })
}

fn spawn_message(error: SpawnError) -> String {
    match error {
        SpawnError::Missing => "git was not found on PATH.".to_string(),
        SpawnError::Failed(message) => mask_urls(&message),
    }
}

fn open(root: &Path) -> Result<Repo, OpenError> {
    let top = match run(root, &["rev-parse", "--show-toplevel"]) {
        Ok(output) if output.ok() => output.stdout,
        Ok(_) => return Err(OpenError::NotRepository),
        Err(SpawnError::Missing) => return Err(OpenError::Missing),
        Err(SpawnError::Failed(message)) => return Err(OpenError::Failed(mask_urls(&message))),
    };
    let top = top.trim();
    if top.is_empty() {
        return Err(OpenError::NotRepository);
    }
    let git_dir = match run(Path::new(top), &["rev-parse", "--absolute-git-dir"]) {
        Ok(output) if output.ok() => output.stdout.trim().to_string(),
        Ok(_) => return Err(OpenError::NotRepository),
        Err(SpawnError::Missing) => return Err(OpenError::Missing),
        Err(SpawnError::Failed(message)) => return Err(OpenError::Failed(mask_urls(&message))),
    };
    Ok(Repo {
        top: PathBuf::from(top),
        git_dir: PathBuf::from(git_dir),
    })
}

fn open_data_error(error: OpenError) -> (&'static str, String) {
    match error {
        OpenError::NotRepository => ("not_a_repository", "Not a git repository".to_string()),
        OpenError::Missing => ("git_missing", "git was not found on PATH.".to_string()),
        OpenError::Failed(message) => ("failed", message),
    }
}

fn require_repo(root: &Path) -> HostResult<Repo> {
    match open(root) {
        Ok(repo) => Ok(repo),
        Err(OpenError::NotRepository) => Err(HostError::new(
            "git_not_a_repository",
            "Not a git repository; no git command was useful here.",
        )),
        Err(OpenError::Missing) => Err(HostError::new(
            "git_not_found",
            "git was not found on PATH.",
        )),
        Err(OpenError::Failed(message)) => Err(HostError::new("git_failed", message)),
    }
}

fn cap(text: &str, limit: usize) -> (String, bool) {
    match text.char_indices().nth(limit) {
        Some((index, _)) => (format!("{}\n… truncated at {limit} characters", &text[..index]), true),
        None => (text.to_string(), false),
    }
}

/// Replaces `scheme://user:password@host` credentials before any output reaches a log or a
/// view. The app never stores credentials; this keeps captured git output from doing it too.
pub fn mask_urls(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(position) = rest.find("://") {
        let (head, tail) = rest.split_at(position + 3);
        out.push_str(head);
        let end = tail
            .find(|character: char| {
                character == '/'
                    || character.is_whitespace()
                    || character == '"'
                    || character == '\''
                    || character == ')'
            })
            .unwrap_or(tail.len());
        let authority = &tail[..end];
        match authority.rfind('@') {
            Some(at) => {
                out.push_str("***@");
                out.push_str(&authority[at + 1..]);
            }
            None => out.push_str(authority),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

fn tail(text: &str) -> String {
    let (capped, _) = cap(text.trim(), OUTPUT_LIMIT);
    capped
}

fn number_after(text: &str, label: &str) -> u64 {
    let Some(start) = text.find(label) else {
        return 0;
    };
    let digits: String = text[start + label.len()..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().unwrap_or(0)
}

#[derive(Default)]
struct Progress {
    branch: Option<String>,
    detached: bool,
    initial: bool,
    upstream: Option<String>,
    upstream_gone: bool,
    ahead: u64,
    behind: u64,
}

fn parse_header(line: &str) -> Progress {
    let mut progress = Progress::default();
    let text = line.trim_start_matches("## ").trim();
    if let Some(rest) = text.strip_prefix("No commits yet on ") {
        progress.initial = true;
        progress.branch = Some(rest.to_string());
        return progress;
    }
    if text.starts_with("HEAD (no branch)") {
        progress.detached = true;
        return progress;
    }
    let (branch, rest) = match text.split_once("...") {
        Some((branch, rest)) => (branch, Some(rest.trim())),
        None => (text, None),
    };
    progress.branch = Some(branch.trim().to_string());
    if let Some(rest) = rest {
        let (upstream, track) = match rest.split_once(" [") {
            Some((upstream, track)) => (upstream, Some(track)),
            None => (rest, None),
        };
        progress.upstream = Some(upstream.trim().to_string());
        if let Some(track) = track {
            progress.upstream_gone = track.starts_with("gone");
            progress.ahead = number_after(track, "ahead ");
            progress.behind = number_after(track, "behind ");
        }
    }
    progress
}

fn file_label(index: char, worktree: char) -> &'static str {
    if index == '?' && worktree == '?' {
        return "Untracked";
    }
    if matches!(
        (index, worktree),
        ('D', 'D') | ('A', 'U') | ('U', 'D') | ('U', 'A') | ('D', 'U') | ('A', 'A') | ('U', 'U')
    ) {
        return "Conflicted";
    }
    if index != ' ' {
        return if worktree == ' ' {
            "Staged"
        } else {
            "Staged + modified"
        };
    }
    match worktree {
        'M' => "Modified",
        'D' => "Deleted",
        'R' => "Renamed",
        'C' => "Copied",
        'T' => "Type changed",
        'A' => "Added",
        _ => "Changed",
    }
}

fn parse_status(text: &str) -> (Progress, Vec<GitFile>) {
    let records: Vec<&str> = text.split('\0').collect();
    let mut progress = Progress::default();
    let mut index = 0;
    if records.first().is_some_and(|record| record.starts_with("## ")) {
        progress = parse_header(records[0]);
        index = 1;
    }
    let mut files = Vec::new();
    while index < records.len() {
        let record = records[index];
        index += 1;
        if record.len() < 4 {
            continue;
        }
        let bytes = record.as_bytes();
        let x = bytes[0] as char;
        let y = bytes[1] as char;
        let path = record[3..].to_string();
        let mut original_path = None;
        if matches!(x, 'R' | 'C') || matches!(y, 'R' | 'C') {
            if index < records.len() {
                original_path = Some(records[index].to_string());
                index += 1;
            }
        }
        files.push(GitFile {
            path,
            original_path,
            index: x.to_string(),
            worktree: y.to_string(),
            untracked: x == '?' && y == '?',
            staged: x != ' ' && x != '?',
            conflicted: file_label(x, y) == "Conflicted",
            label: file_label(x, y),
        });
    }
    (progress, files)
}

fn in_progress(git_dir: &Path) -> Option<&'static str> {
    let markers: [(&str, &'static str); 5] = [
        ("rebase-merge", "rebase"),
        ("rebase-apply", "rebase"),
        ("MERGE_HEAD", "merge"),
        ("CHERRY_PICK_HEAD", "cherry-pick"),
        ("REVERT_HEAD", "revert"),
    ];
    markers
        .iter()
        .find(|(marker, _)| git_dir.join(marker).exists())
        .map(|(_, label)| *label)
}

fn read_status(repo: &Repo) -> Result<GitStatus, String> {
    let output = run(
        &repo.top,
        &["status", "--porcelain=v1", "-z", "--branch"],
    )
    .map_err(spawn_message)?;
    if !output.ok() {
        let stderr = mask_urls(output.stderr.trim());
        return Err(if stderr.is_empty() {
            "git status failed without output.".to_string()
        } else {
            stderr
        });
    }
    let (progress, files) = parse_status(&output.stdout);
    let remotes = run(&repo.top, &["remote"])
        .map(|remote| {
            remote
                .stdout
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    Ok(GitStatus {
        state: "ok",
        message: None,
        root: Some(repo.top.to_string_lossy().replace('\\', "/")),
        branch: progress.branch,
        detached: progress.detached,
        initial: progress.initial,
        upstream: progress.upstream,
        upstream_gone: progress.upstream_gone,
        ahead: progress.ahead,
        behind: progress.behind,
        operation: in_progress(&repo.git_dir),
        clean: files.is_empty(),
        files,
        remotes,
    })
}

pub fn status(root: &Path) -> GitStatus {
    match open(root) {
        Ok(repo) => match read_status(&repo) {
            Ok(status) => status,
            Err(message) => GitStatus::unavailable("failed", message),
        },
        Err(error) => {
            let (state, message) = open_data_error(error);
            GitStatus::unavailable(state, message)
        }
    }
}

/// Whether `path` (repository-relative) is ignored: the entry itself, or any ancestor directory
/// `git status --ignored=matching` reported (`build/` covers everything below it).
fn ignored_below(ignored: &HashSet<String>, path: &str) -> bool {
    let mut current = path;
    loop {
        if ignored.contains(current) {
            return true;
        }
        match current.rsplit_once('/') {
            Some((parent, _)) if !parent.is_empty() => current = parent,
            _ => return false,
        }
    }
}

/// Which of `paths` (workspace-relative, visible tree rows) the ignore rules match.
///
/// `git status --ignored` is the source instead of `check-ignore` because it never reports a
/// tracked file as ignored, and the pathspec keeps the answer bounded to what the tree is showing.
/// The command runs with a fixed argv (no shell), the child is killed at the output ceiling, and
/// an ignored directory answers for the files below it.
pub fn ignored(root: &Path, paths: &[String]) -> GitIgnored {
    let repo = match open(root) {
        Ok(repo) => repo,
        Err(error) => {
            let (state, message) = open_data_error(error);
            return GitIgnored {
                state,
                message: Some(message),
                ignored: Vec::new(),
            };
        }
    };
    // (workspace-relative as the caller sent it, repository-relative for the pathspec)
    // The workspace root is canonicalized first: `git rev-parse` answers with the physical path
    // (`/private/var/...` on macOS), and a textual prefix comparison would drop every row.
    let workspace = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let mut requested: Vec<(String, String)> = Vec::new();
    let mut bytes = 0usize;
    for raw in paths.iter().take(IGNORED_PATHS) {
        let absolute = workspace.join(raw);
        let Ok(relative) = absolute.strip_prefix(&repo.top) else {
            continue; // outside the repository: nothing to ignore-query
        };
        // `strip_prefix` is lexical, so `repo/../outside` still looks inside; a parent component
        // means the path escapes and must never become a pathspec.
        if relative
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
        {
            continue;
        }
        let relative = relative.to_string_lossy().replace('\\', "/");
        if relative.is_empty() {
            continue;
        }
        bytes += relative.len();
        if bytes > IGNORED_PATH_BYTES {
            break;
        }
        requested.push((raw.replace('\\', "/"), relative));
    }
    if requested.is_empty() {
        return GitIgnored {
            state: "ok",
            message: None,
            ignored: Vec::new(),
        };
    }
    let mut args: Vec<&str> = vec![
        "status",
        "--porcelain=v1",
        "-z",
        "--ignored=matching",
        "--",
    ];
    args.extend(requested.iter().map(|(_, relative)| relative.as_str()));
    let captured = match spawn_bounded(&repo.top, &args, IGNORED_BYTES) {
        Ok(captured) => captured,
        Err(error) => {
            return GitIgnored {
                state: "failed",
                message: Some(spawn_message(error)),
                ignored: Vec::new(),
            };
        }
    };
    // A killed child (truncated output) keeps the records already captured: a missing marker is
    // safe, an invented one is not. `git status` itself only fails on a fatal error (128/129).
    if matches!(captured.code, Some(128) | Some(129)) && !captured.truncated {
        let stderr = mask_urls(captured.stderr.trim());
        return GitIgnored {
            state: "failed",
            message: Some(if stderr.is_empty() {
                "git status failed without output.".to_string()
            } else {
                stderr
            }),
            ignored: Vec::new(),
        };
    }
    let text = String::from_utf8_lossy(&captured.stdout);
    let mut ignored_paths = HashSet::new();
    for record in text.split('\0') {
        if record.len() < 4 || !record.starts_with("!! ") {
            continue;
        }
        let path = record[3..].trim_end_matches('/').replace('\\', "/");
        if !path.is_empty() {
            ignored_paths.insert(path);
        }
    }
    let ignored = requested
        .into_iter()
        .filter(|(_, relative)| ignored_below(&ignored_paths, relative))
        .map(|(raw, _)| raw)
        .collect();
    GitIgnored {
        state: "ok",
        message: None,
        ignored,
    }
}

fn parse_track(track: &str) -> (u64, u64, bool) {
    (
        number_after(track, "ahead "),
        number_after(track, "behind "),
        track.trim().starts_with("gone"),
    )
}

fn parse_branches(text: &str) -> (Vec<Branch>, Vec<Branch>) {
    let mut local = Vec::new();
    let mut remote = Vec::new();
    for record in text.lines() {
        if record.is_empty() {
            continue;
        }
        let fields: Vec<&str> = record.split('\0').collect();
        if fields.len() < 6 {
            continue;
        }
        let full = fields[0];
        let name = fields[1];
        let current = fields[2] == "*";
        let upstream = if fields[3].is_empty() {
            None
        } else {
            Some(fields[3].to_string())
        };
        let (ahead, behind, gone) = parse_track(fields[4]);
        if !fields[5].is_empty() {
            continue;
        }
        let branch = Branch {
            name: name.to_string(),
            upstream,
            ahead,
            behind,
            current,
            remote: full.starts_with("refs/remotes/"),
            gone,
        };
        if branch.remote {
            remote.push(branch);
        } else {
            local.push(branch);
        }
    }
    local.sort_by(|a, b| b.current.cmp(&a.current).then_with(|| a.name.cmp(&b.name)));
    remote.sort_by(|a, b| a.name.cmp(&b.name));
    (local, remote)
}

pub fn branches(root: &Path) -> GitBranches {
    let repo = match open(root) {
        Ok(repo) => repo,
        Err(error) => {
            let (state, message) = open_data_error(error);
            return GitBranches::unavailable(state, message);
        }
    };
    let output = match run(
        &repo.top,
        &[
            "for-each-ref",
            "--format=%(refname)%00%(refname:short)%00%(HEAD)%00%(upstream:short)%00%(upstream:track)%00%(symref)%00",
            "refs/heads",
            "refs/remotes",
        ],
    ) {
        Ok(output) => output,
        Err(error) => return GitBranches::unavailable("failed", spawn_message(error)),
    };
    if !output.ok() {
        return GitBranches::unavailable("failed", mask_urls(output.stderr.trim()));
    }
    let (local, remote) = parse_branches(&output.stdout);
    let current = local
        .iter()
        .find(|branch| branch.current)
        .map(|branch| branch.name.clone());
    let remotes = run(&repo.top, &["remote"])
        .map(|output| {
            output
                .stdout
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    GitBranches {
        state: "ok",
        message: None,
        current,
        local,
        remote,
        remotes,
    }
}

fn parse_name_status(body: &str) -> (Vec<CommitFile>, bool) {
    let mut files = Vec::new();
    let mut truncated = false;
    let mut records = body.split('\0');
    while let Some(status) = records.next() {
        let status = status.trim_start_matches('\n');
        if status.is_empty() {
            continue;
        }
        let Some(first) = records.next() else {
            break;
        };
        if first.is_empty() {
            continue;
        }
        // `git log --name-status -z` prints the old path before the new one for R/C records,
        // unlike `git status -z`, which prints the new path first.
        let (path, original_path) = if status.starts_with('R') || status.starts_with('C') {
            let Some(second) = records.next() else {
                break;
            };
            (second.to_string(), Some(first.to_string()))
        } else {
            (first.to_string(), None)
        };
        if files.len() >= COMMIT_FILES_LIMIT {
            truncated = true;
            continue;
        }
        files.push(CommitFile {
            path: path.to_string(),
            original_path,
            status: status.to_string(),
        });
    }
    (files, truncated)
}

fn parse_log(text: &str) -> Vec<GitCommit> {
    let mut commits = Vec::new();
    for chunk in text.split('\x1e') {
        if chunk.is_empty() {
            continue;
        }
        let (header, body) = match chunk.split_once('\0') {
            Some((header, body)) => (header, body),
            None => (chunk, ""),
        };
        let fields: Vec<&str> = header.split('\x1f').collect();
        if fields.len() < 5 || fields[0].trim().is_empty() {
            continue;
        }
        let hash = fields[0].trim().to_string();
        let short_hash = hash.chars().take(7).collect();
        let parents: Vec<String> = fields[3]
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let (files, files_truncated) = parse_name_status(body);
        commits.push(GitCommit {
            hash,
            short_hash,
            author: fields[1].to_string(),
            date: fields[2].to_string(),
            subject: fields[4].to_string(),
            parents,
            files,
            files_truncated,
        });
    }
    commits
}

pub fn log(root: &Path) -> GitLog {
    let repo = match open(root) {
        Ok(repo) => repo,
        Err(error) => {
            let (state, message) = open_data_error(error);
            return GitLog::unavailable(state, message);
        }
    };
    let max = format!("--max-count={}", LOG_LIMIT + 1);
    let output = match run(
        &repo.top,
        &[
            "log",
            &max,
            "-z",
            "--name-status",
            "--format=%x1e%H%x1f%an%x1f%aI%x1f%P%x1f%s",
        ],
    ) {
        Ok(output) => output,
        Err(error) => return GitLog::unavailable("failed", spawn_message(error)),
    };
    if !output.ok() {
        let stderr = mask_urls(output.stderr.trim());
        if stderr.contains("does not have any commits") || stderr.contains("unknown revision") {
            return GitLog::unavailable("no_commits", "No commits yet.");
        }
        return GitLog::unavailable("failed", stderr);
    }
    let mut commits = parse_log(&output.stdout);
    let truncated = commits.len() > LOG_LIMIT;
    commits.truncate(LOG_LIMIT);
    GitLog {
        state: "ok",
        message: None,
        limit: LOG_LIMIT,
        truncated,
        commits,
    }
}

fn validate_path(path: &str) -> HostResult<()> {
    let parsed = Path::new(path);
    let escapes = parsed.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    });
    if path.trim().is_empty() || parsed.is_absolute() || escapes {
        return Err(HostError::new(
            "invalid_state",
            format!("Path must stay inside the repository: {path}"),
        ));
    }
    Ok(())
}

fn validate_branch(name: &str) -> HostResult<()> {
    if name.trim().is_empty() || name.starts_with('-') || name.chars().any(char::is_control) {
        return Err(HostError::new(
            "invalid_state",
            "A branch name must be non-empty and must not start with '-'.",
        ));
    }
    Ok(())
}

/// Defines what `git diff` compares. `unified` is the context the caller needs: `-U3` for the
/// patch a surface shows, `-U0` for the changed block ranges only.
fn diff_output(
    repo: &Repo,
    path: &str,
    staged: bool,
    tracked: bool,
    unified: &str,
) -> Result<Captured, SpawnError> {
    let limit = if unified == "-U0" { HUNK_BYTES } else { PATCH_BYTES };
    if staged {
        // HEAD -> index, tracked or not: a path absent from the index (an untracked path or a
        // staged deletion) is still compared to HEAD, and the worktree is never read here.
        let mut args = vec!["diff", "--no-color", "--no-ext-diff", unified, "--cached"];
        args.push("--");
        args.push(path);
        spawn_bounded(&repo.top, &args, limit)
    } else if tracked {
        spawn_bounded(
            &repo.top,
            &["diff", "--no-color", "--no-ext-diff", unified, "--", path],
            limit,
        )
    } else {
        // An untracked file has no index entry to diff against; /dev/null shows its content.
        spawn_bounded(
            &repo.top,
            &[
                "diff",
                "--no-color",
                "--no-ext-diff",
                unified,
                "--no-index",
                "--",
                "/dev/null",
                path,
            ],
            limit,
        )
    }
}

fn is_tracked(repo: &Repo, path: &str) -> Result<bool, (&'static str, String)> {
    let output = run(&repo.top, &["ls-files", "--error-unmatch", "--", path])
        .map_err(|error| (spawn_state(&error), spawn_message(error)))?;
    match output.code {
        Some(0) => Ok(true),
        // `--error-unmatch` exits 1 when the path simply has no index entry.
        Some(1) => Ok(false),
        // A corrupt or unreadable index must not turn into "untracked" (and then `--no-index`).
        _ => Err(("failed", mask_urls(output.stderr.trim()))),
    }
}

pub fn diff(root: &Path, path: &str, staged: bool) -> HostResult<GitDiff> {
    validate_path(path)?;
    let repo = match open(root) {
        Ok(repo) => repo,
        Err(error) => {
            let (state, message) = open_data_error(error);
            return Ok(GitDiff::unavailable(state, message, path, staged));
        }
    };
    let tracked = match is_tracked(&repo, path) {
        Ok(tracked) => tracked,
        Err((state, message)) => {
            return Ok(GitDiff::unavailable(state, message, path, staged))
        }
    };
    if !staged && worktree_confinement(&repo, path) == Confinement::Outside {
        return Ok(GitDiff::unavailable(
            "failed",
            outside_message(path),
            path,
            staged,
        ));
    }
    let output = match diff_output(&repo, path, staged, tracked, "-U3") {
        Ok(output) => output,
        Err(error) => return Ok(GitDiff::unavailable("failed", spawn_message(error), path, staged)),
    };
    // A cut-off capture is a successful read with less text, not a failed command.
    if !output.truncated && output.code != Some(0) && output.code != Some(1) {
        let stderr = mask_urls(output.stderr.trim());
        return Ok(GitDiff::unavailable("failed", stderr, path, staged));
    }
    let output = output.text();
    let text = mask_urls(&output.stdout);
    if text.trim().is_empty() {
        return Ok(GitDiff::unavailable(
            "empty",
            format!("No diff for {path}."),
            path,
            staged,
        ));
    }
    let (diff, capped) = cap(&text, DIFF_LIMIT);
    Ok(GitDiff {
        state: "ok",
        message: None,
        path: path.to_string(),
        staged,
        diff,
        truncated: capped || output.truncated,
    })
}

/// One side's bytes, keeping "does not exist" apart from "exists but cannot be read".
enum SideBytes {
    Missing,
    Bytes(Vec<u8>),
    /// The limit was hit while reading; the bytes were never collected.
    TooLarge,
    Unreadable,
}

/// `text: None` with `exists: true` is a fact the panel must explain, never an empty file.
fn read_side(bytes: SideBytes) -> (DiffSide, Option<&'static str>) {
    let missing = DiffSide {
        exists: false,
        text: None,
    };
    let bytes = match bytes {
        SideBytes::Missing => return (missing, None),
        SideBytes::TooLarge => {
            return (
                DiffSide {
                    exists: true,
                    text: None,
                },
                Some("too_large"),
            )
        }
        SideBytes::Unreadable => {
            return (
                DiffSide {
                    exists: true,
                    text: None,
                },
                Some("unreadable"),
            )
        }
        SideBytes::Bytes(bytes) => bytes,
    };
    let present = DiffSide {
        exists: true,
        text: None,
    };
    if bytes.len() > FULL_FILE_BYTES {
        return (present, Some("too_large"));
    }
    // UTF-16 text is full of NUL bytes and still something the editor shows; only a NUL without
    // a UTF-16 BOM is the Git-style "binary" verdict.
    let utf16 = bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]);
    if !utf16 && bytes.iter().take(BINARY_PROBE).any(|byte| *byte == 0) {
        return (present, Some("binary"));
    }
    match crate::fsutil::decode_text(&bytes) {
        Ok((text, _)) => {
            if text.lines().count() > FULL_FILE_LINES {
                return (present, Some("too_large"));
            }
            (
                DiffSide {
                    exists: true,
                    text: Some(text),
                },
                None,
            )
        }
        Err(_) => (present, Some("unsupported_encoding")),
    }
}

/// Whether the directory that holds `path` is really inside the repository. A lexical check
/// cannot see a symlinked directory, and both our reads and Git's worktree side would follow it
/// out of the workspace.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Confinement {
    Inside,
    Outside,
    /// The parent could not be resolved (usually it no longer exists); there is no target left
    /// to leak, so the caller may continue.
    Unresolved,
}

fn worktree_confinement(repo: &Repo, path: &str) -> Confinement {
    let absolute = repo.top.join(path);
    let Some(parent) = absolute.parent() else {
        return Confinement::Outside;
    };
    let (Ok(root), Ok(resolved)) = (fs::canonicalize(&repo.top), fs::canonicalize(parent)) else {
        return Confinement::Unresolved;
    };
    if resolved.starts_with(&root) {
        Confinement::Inside
    } else {
        Confinement::Outside
    }
}

fn read_bounded_file(path: &Path, limit: usize) -> SideBytes {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return SideBytes::Missing,
        Err(_) => return SideBytes::Unreadable,
    };
    let mut bytes = Vec::new();
    match file.take(limit as u64 + 1).read_to_end(&mut bytes) {
        Err(_) => SideBytes::Unreadable,
        Ok(_) if bytes.len() > limit => SideBytes::TooLarge,
        Ok(_) => SideBytes::Bytes(bytes),
    }
}

/// Reads one side the way Git sees it, never the target of a symlink: a link is its target path
/// (that is the blob Git stores), and a regular file is read with a hard byte ceiling.
fn read_worktree(repo: &Repo, path: &str) -> SideBytes {
    if worktree_confinement(repo, path) == Confinement::Outside {
        return SideBytes::Unreadable;
    }
    let absolute = repo.top.join(path);
    let metadata = match fs::symlink_metadata(&absolute) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return SideBytes::Missing,
        Err(_) => return SideBytes::Unreadable,
    };
    if metadata.file_type().is_symlink() {
        return match fs::read_link(&absolute) {
            Ok(target) => SideBytes::Bytes(target.to_string_lossy().into_owned().into_bytes()),
            Err(_) => SideBytes::Unreadable,
        };
    }
    if !metadata.is_file() {
        return SideBytes::Unreadable;
    }
    read_bounded_file(&absolute, FULL_FILE_BYTES)
}

/// Resolves `<spec>` to an object id, then checks its size before reading a single content byte.
/// Exit 1 from `rev-parse --verify --quiet` is the proved absence (added file, deletion, unborn
/// repository); anything else is a real read error and must not be reported as "no such file".
fn read_blob(repo: &Repo, spec: &str) -> Result<SideBytes, SpawnError> {
    let resolved = spawn_bounded(&repo.top, &["rev-parse", "--verify", "--quiet", spec], 4_096)?;
    if resolved.code == Some(1) {
        return Ok(SideBytes::Missing);
    }
    if resolved.code != Some(0) || resolved.truncated {
        return Ok(SideBytes::Unreadable);
    }
    let hash = String::from_utf8_lossy(&resolved.stdout).trim().to_string();
    if hash.is_empty() {
        return Ok(SideBytes::Missing);
    }
    let sized = spawn_bounded(&repo.top, &["cat-file", "-s", &hash], 64)?;
    if sized.code != Some(0) || sized.truncated {
        return Ok(SideBytes::Unreadable);
    }
    let size: usize = match String::from_utf8_lossy(&sized.stdout).trim().parse() {
        Ok(size) => size,
        Err(_) => return Ok(SideBytes::Unreadable),
    };
    if size > FULL_FILE_BYTES {
        return Ok(SideBytes::TooLarge);
    }
    let content = spawn_bounded(&repo.top, &["cat-file", "blob", &hash], size)?;
    if content.code != Some(0) || content.truncated {
        return Ok(SideBytes::Unreadable);
    }
    Ok(SideBytes::Bytes(content.stdout))
}

fn range(text: &str) -> Option<(u32, u32)> {
    match text.split_once(',') {
        Some((start, count)) => Some((start.parse().ok()?, count.parse().ok()?)),
        None => Some((text.parse().ok()?, 1)),
    }
}

/// Header-only parse of a `-U0` patch: `@@ -oldStart,oldCount +newStart,newCount @@`. The body is
/// not needed, because with zero context every hunk is exactly one changed block.
fn parse_hunks(patch: &str) -> Vec<DiffHunk> {
    let mut hunks = Vec::new();
    for line in patch.lines() {
        let Some(rest) = line.strip_prefix("@@ -") else {
            continue;
        };
        let Some((old, rest)) = rest.split_once(" +") else {
            continue;
        };
        let Some((new, _)) = rest.split_once(' ') else {
            continue;
        };
        let (Some((old_start, old_count)), Some((new_start, new_count))) =
            (range(old), range(new))
        else {
            continue;
        };
        hunks.push(DiffHunk {
            old_start,
            old_count,
            new_start,
            new_count,
        });
    }
    hunks
}

fn head_commit(repo: &Repo) -> Option<DiffCommit> {
    let output = run(&repo.top, &["log", "-1", "--format=%H\t%h\t%s"]).ok()?;
    if !output.ok() {
        return None;
    }
    let text = output.stdout.trim_end_matches('\n');
    let mut parts = text.splitn(3, '\t');
    let hash = parts.next()?.to_string();
    let short_hash = parts.next()?.to_string();
    let subject = parts.next()?.to_string();
    if hash.is_empty() {
        return None;
    }
    Some(DiffCommit {
        hash,
        short_hash,
        subject,
    })
}

fn outside_message(path: &str) -> String {
    format!("Path leaves the repository through a symbolic link: {path}")
}

fn spawn_state(error: &SpawnError) -> &'static str {
    match error {
        SpawnError::Missing => "git_missing",
        SpawnError::Failed(_) => "failed",
    }
}

/// A new file only the current user can read, never clobbering an existing path.
fn write_private(path: &Path, text: &str) -> std::io::Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    file.write_all(text.as_bytes())
}

/// Both decoded sides, removed on drop on every return path (a hard kill can still leave them;
/// the unique pid+sequence name and the 0600 mode keep them private and non-conflicting).
struct TempSides {
    base: PathBuf,
    current: PathBuf,
}

impl TempSides {
    fn new() -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or_default();
        let unique = format!(
            "openridr-diff-{}-{stamp}-{}",
            std::process::id(),
            TEMP_SEQ.fetch_add(1, Ordering::Relaxed)
        );
        let dir = std::env::temp_dir();
        Self {
            base: dir.join(format!("{unique}-base")),
            current: dir.join(format!("{unique}-current")),
        }
    }
}

impl Drop for TempSides {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.base);
        let _ = fs::remove_file(&self.current);
    }
}

/// `git diff` calls a UTF-16 change binary and emits no ranges, while the editor shows that file
/// as text. When both decoded sides exist and Git gave no ranges, Git produces them over the
/// decoded text in private temporary files; nothing of the file contents is returned.
fn decoded_hunks(base: &str, current: &str) -> Option<Vec<DiffHunk>> {
    let sides = TempSides::new();
    write_private(&sides.base, base).ok()?;
    write_private(&sides.current, current).ok()?;
    let dir = std::env::temp_dir();
    let (Some(base_arg), Some(current_arg)) = (sides.base.to_str(), sides.current.to_str()) else {
        return None;
    };
    let output = spawn_bounded(
        &dir,
        &[
            "diff",
            "--no-color",
            "--no-ext-diff",
            "-U0",
            "--no-index",
            "--",
            base_arg,
            current_arg,
        ],
        HUNK_BYTES,
    );
    let output = output.ok()?;
    if output.truncated || (output.code != Some(0) && output.code != Some(1)) {
        return None;
    }
    Some(parse_hunks(&String::from_utf8_lossy(&output.stdout)))
}

/// The read behind the diff panel: full base and current texts, the changed block ranges that
/// align them, the bounded patch as fallback, and the repository HEAD commit as context.
pub fn file_diff(root: &Path, path: &str, staged: bool) -> HostResult<GitFileDiff> {
    validate_path(path)?;
    let repo = match open(root) {
        Ok(repo) => repo,
        Err(error) => {
            let (state, message) = open_data_error(error);
            return Ok(GitFileDiff::unavailable(state, message, path, staged));
        }
    };
    let tracked = match is_tracked(&repo, path) {
        Ok(tracked) => tracked,
        Err((state, message)) => {
            return Ok(GitFileDiff::unavailable(state, message, path, staged))
        }
    };
    if !staged && worktree_confinement(&repo, path) == Confinement::Outside {
        return Ok(GitFileDiff::unavailable(
            "failed",
            outside_message(path),
            path,
            staged,
        ));
    }
    let patch_output = match diff_output(&repo, path, staged, tracked, "-U3") {
        Ok(output) => output,
        Err(error) => {
            return Ok(GitFileDiff::unavailable(
                "failed",
                spawn_message(error),
                path,
                staged,
            ))
        }
    };
    if !patch_output.truncated && patch_output.code != Some(0) && patch_output.code != Some(1) {
        let stderr = mask_urls(patch_output.stderr.trim());
        return Ok(GitFileDiff::unavailable("failed", stderr, path, staged));
    }
    let hunk_output = match diff_output(&repo, path, staged, tracked, "-U0") {
        Ok(output) => output,
        Err(error) => {
            return Ok(GitFileDiff::unavailable(
                "failed",
                spawn_message(error),
                path,
                staged,
            ))
        }
    };
    let mut hunks = Vec::new();
    if !hunk_output.truncated && (hunk_output.code == Some(0) || hunk_output.code == Some(1)) {
        hunks = parse_hunks(&String::from_utf8_lossy(&hunk_output.stdout));
    }

    // Unstaged compares the index with the worktree; staged compares HEAD with the index.
    let base_spec = if staged {
        format!("HEAD:{path}")
    } else {
        format!(":{path}")
    };
    let base_bytes = match read_blob(&repo, &base_spec) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Ok(GitFileDiff::unavailable(
                spawn_state(&error),
                spawn_message(error),
                path,
                staged,
            ))
        }
    };
    let current_bytes = if staged {
        match read_blob(&repo, &format!(":{path}")) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Ok(GitFileDiff::unavailable(
                    spawn_state(&error),
                    spawn_message(error),
                    path,
                    staged,
                ))
            }
        }
    } else {
        read_worktree(&repo, path)
    };
    let (base, base_reason) = read_side(base_bytes);
    let (current, current_reason) = read_side(current_bytes);
    let mut full_reason = base_reason.or(current_reason);
    let mut full = (!base.exists || base.text.is_some())
        && (!current.exists || current.text.is_some());

    // Git calls some text binary (UTF-16 with a BOM is the common case) and then emits no ranges.
    let text_differs = match (base.text.as_deref(), current.text.as_deref()) {
        (Some(base_text), Some(current_text)) => base_text != current_text,
        _ => false,
    };
    if full && hunks.is_empty() && text_differs {
        hunks = decoded_hunks(
            base.text.as_deref().expect("full implies text"),
            current.text.as_deref().expect("full implies text"),
        )
        .unwrap_or_default();
    }

    let text = mask_urls(&String::from_utf8_lossy(&patch_output.stdout));
    let commit = head_commit(&repo);
    if text.trim().is_empty() {
        // "No diff" would be a lie when a side exists and could not be read.
        let unreadable = base_reason == Some("unreadable") || current_reason == Some("unreadable");
        let (state, message) = if unreadable {
            (
                "failed",
                format!("The file could not be read for the diff: {path}."),
            )
        } else {
            ("empty", format!("No diff for {path}."))
        };
        return Ok(GitFileDiff {
            state,
            message: Some(message),
            path: path.to_string(),
            staged,
            base,
            current,
            hunks,
            patch: String::new(),
            truncated: false,
            full,
            full_reason,
            commit,
        });
    }
    if full && text_differs && hunks.is_empty() {
        // Something changed in the text, but the block ranges that align the two texts are
        // missing: showing the full file would be a guess.
        full = false;
        full_reason = Some("unaligned");
    }
    let (patch, truncated) = cap(&text, DIFF_LIMIT);
    Ok(GitFileDiff {
        state: "ok",
        message: None,
        path: path.to_string(),
        staged,
        base,
        current,
        hunks,
        patch,
        truncated,
        full,
        full_reason,
        commit,
    })
}

fn require_clean(status: &GitStatus, action: &str) -> HostResult<()> {
    if status.state != "ok" {
        return Err(HostError::new(
            "git_not_a_repository",
            "Not a git repository; no git command was run.",
        ));
    }
    if let Some(operation) = status.operation {
        return Err(HostError::new(
            "git_operation_in_progress",
            format!(
                "{action} refused: a {operation} is in progress. Finish or abort it first; OpenRidr does not auto-stash."
            ),
        ));
    }
    if !status.clean {
        return Err(HostError::new(
            "git_dirty_tree",
            format!(
                "{action} refused: the working tree has uncommitted changes. Commit or stash them; OpenRidr does not auto-stash."
            ),
        ));
    }
    Ok(())
}

fn operation(
    state: &'static str,
    message: Option<String>,
    output: &str,
    repo: &Repo,
    commit: Option<Committed>,
) -> Operation {
    let status = read_status(repo).unwrap_or_else(|message| GitStatus::unavailable("failed", message));
    let (capped, _) = cap(&mask_urls(output), OUTPUT_LIMIT);
    Operation {
        state,
        message,
        output: capped,
        status,
        commit,
        remote: None,
        branch: None,
        upstream: None,
        ahead: 0,
    }
}

fn reset_intent(repo: &Repo, paths: &[String]) {
    if paths.is_empty() {
        return;
    }
    let mut args = vec!["reset", "-q", "--"];
    args.extend(paths.iter().map(String::as_str));
    let _ = run(&repo.top, &args);
}

pub fn commit(root: &Path, paths: &[String], message: &str, amend: bool) -> HostResult<Operation> {
    if message.trim().is_empty() {
        return Err(HostError::new(
            "invalid_state",
            "A commit message is required.",
        ));
    }
    if paths.is_empty() {
        return Err(HostError::new(
            "invalid_state",
            "Select at least one file to commit.",
        ));
    }
    for path in paths {
        validate_path(path)?;
    }
    let repo = require_repo(root)?;
    let before = read_status(&repo).map_err(|message| HostError::new("git_failed", message))?;
    let untracked: Vec<String> = paths
        .iter()
        .filter(|path| {
            before
                .files
                .iter()
                .any(|file| &file.path == *path && file.untracked)
        })
        .cloned()
        .collect();
    if !untracked.is_empty() {
        // `git commit --only` refuses paths that are not known to the index. Intent-to-add
        // makes a new file committable without staging its content; it is rolled back below
        // only when the commit fails.
        let mut args = vec!["add", "--intent-to-add", "--"];
        args.extend(untracked.iter().map(String::as_str));
        let output = run(&repo.top, &args).map_err(|error| {
            HostError::new("git_failed", spawn_message(error))
        })?;
        if !output.ok() {
            return Err(HostError::new(
                "git_failed",
                format!("git add --intent-to-add failed: {}", tail(&mask_urls(&output.stderr))),
            ));
        }
    }
    let mut args = vec!["commit", "--only"];
    if amend {
        args.push("--amend");
    }
    args.extend(["-m", message, "--"]);
    args.extend(paths.iter().map(String::as_str));
    let output = match run(&repo.top, &args) {
        Ok(output) => output,
        Err(error) => {
            reset_intent(&repo, &untracked);
            return Err(HostError::new("git_failed", spawn_message(error)));
        }
    };
    let output_text = format!("{}{}", output.stdout, output.stderr);
    if !output.ok() {
        reset_intent(&repo, &untracked);
        let stderr = mask_urls(output.stderr.trim());
        return Ok(operation(
            "failed",
            Some(if stderr.is_empty() {
                "git commit failed without output.".to_string()
            } else {
                tail(&stderr)
            }),
            &output_text,
            &repo,
            None,
        ));
    }
    let summary_output = run(&repo.top, &["log", "-1", "--format=%H%x1f%s"])
        .map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
    let mut parts = summary_output.stdout.splitn(2, '\u{1f}');
    let hash = parts.next().unwrap_or("").trim().to_string();
    let summary = parts.next().unwrap_or("").trim().to_string();
    let short_hash = hash.chars().take(7).collect();
    Ok(operation(
        "ok",
        Some(format!("Committed {short_hash} {summary}")),
        &output_text,
        &repo,
        Some(Committed {
            hash,
            short_hash,
            summary,
            amend,
            paths: paths.to_vec(),
        }),
    ))
}

fn has_ref(repo: &Repo, reference: &str) -> bool {
    run(
        &repo.top,
        &["rev-parse", "--verify", "--quiet", reference],
    )
    .map(|output| output.ok())
    .unwrap_or(false)
}

pub fn branch_create(root: &Path, name: &str, checkout: bool) -> HostResult<Operation> {
    validate_branch(name)?;
    let repo = require_repo(root)?;
    let args = if checkout {
        vec!["checkout", "-b", name]
    } else {
        vec!["branch", name]
    };
    let output = run(&repo.top, &args).map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
    let text = format!("{}{}", output.stdout, output.stderr);
    if !output.ok() {
        let stderr = mask_urls(output.stderr.trim());
        return Ok(operation(
            "failed",
            Some(if stderr.is_empty() {
                "git branch failed without output.".to_string()
            } else {
                tail(&stderr)
            }),
            &text,
            &repo,
            None,
        ));
    }
    Ok(operation(
        "ok",
        Some(if checkout {
            format!("Created and checked out {name}.")
        } else {
            format!("Created {name}.")
        }),
        &text,
        &repo,
        None,
    ))
}

pub fn checkout(root: &Path, name: &str) -> HostResult<Operation> {
    validate_branch(name)?;
    let repo = require_repo(root)?;
    let status = read_status(&repo).map_err(|message| HostError::new("git_failed", message))?;
    require_clean(&status, "Checkout")?;
    let local_short = name.split_once('/').map(|(_, short)| short).unwrap_or(name);
    let args = if has_ref(&repo, &format!("refs/remotes/{name}"))
        && !has_ref(&repo, &format!("refs/heads/{local_short}"))
    {
        vec!["checkout", "--track", "-b", local_short, name]
    } else {
        vec!["checkout", name]
    };
    let output = run(&repo.top, &args).map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
    let text = format!("{}{}", output.stdout, output.stderr);
    if !output.ok() {
        let stderr = mask_urls(output.stderr.trim());
        return Ok(operation(
            "failed",
            Some(if stderr.is_empty() {
                "git checkout failed without output.".to_string()
            } else {
                tail(&stderr)
            }),
            &text,
            &repo,
            None,
        ));
    }
    let next = read_status(&repo).unwrap_or_else(|message| GitStatus::unavailable("failed", message));
    let message = match next.branch.as_deref() {
        Some(branch) => format!("Checked out {branch}."),
        None => "Checked out detached HEAD.".to_string(),
    };
    Ok(operation("ok", Some(message), &text, &repo, None))
}

pub fn pull(root: &Path) -> HostResult<Operation> {
    let repo = require_repo(root)?;
    let status = read_status(&repo).map_err(|message| HostError::new("git_failed", message))?;
    require_clean(&status, "Update")?;
    if status.upstream.is_none() {
        return Err(HostError::new(
            "git_no_upstream",
            "No upstream is configured; nothing to pull.",
        ));
    }
    let output = run(&repo.top, &["pull"]).map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
    let text = format!("{}{}", output.stdout, output.stderr);
    if !output.ok() {
        let stderr = mask_urls(output.stderr.trim());
        return Ok(operation(
            "failed",
            Some(if stderr.is_empty() {
                "git pull failed without output.".to_string()
            } else {
                tail(&stderr)
            }),
            &text,
            &repo,
            None,
        ));
    }
    Ok(operation(
        "ok",
        Some("Update completed.".to_string()),
        &text,
        &repo,
        None,
    ))
}

fn first_remote(repo: &Repo) -> Option<String> {
    let output = run(&repo.top, &["remote"]).ok()?;
    output
        .stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string)
}

pub fn push(root: &Path, params: &Value) -> HostResult<Operation> {
    if params.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err(HostError::new(
            "confirmation_required",
            "git.push requires confirm:true; no push was run.",
        ));
    }
    let repo = require_repo(root)?;
    let status = read_status(&repo).map_err(|message| HostError::new("git_failed", message))?;
    let Some(branch) = status.branch.clone() else {
        return Err(HostError::new(
            "invalid_state",
            "Cannot push a detached HEAD.",
        ));
    };
    let upstream_remote = status
        .upstream
        .as_ref()
        .and_then(|upstream| upstream.split_once('/').map(|(remote, _)| remote.to_string()));
    let remote = params
        .get("remote")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or(upstream_remote)
        .or_else(|| first_remote(&repo));
    let Some(remote) = remote else {
        return Err(HostError::new(
            "git_no_remote",
            "No remote is configured. Add one with git remote add; no push was run.",
        ));
    };
    let mut args = vec!["push".to_string()];
    if status.upstream.is_none() {
        args.extend([
            "--set-upstream".to_string(),
            remote.clone(),
            branch.clone(),
        ]);
    }
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = run(&repo.top, &borrowed).map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
    let text = format!("{}{}", output.stdout, output.stderr);
    let mut next = operation(
        if output.ok() { "ok" } else { "failed" },
        None,
        &text,
        &repo,
        None,
    );
    if output.ok() {
        next.message = Some(format!("Pushed {branch} to {remote}."));
    } else {
        let stderr = mask_urls(output.stderr.trim());
        next.message = Some(if stderr.is_empty() {
            "git push failed without output.".to_string()
        } else {
            tail(&stderr)
        });
    }
    next.remote = Some(remote);
    next.branch = Some(branch);
    next.upstream = next.status.upstream.clone();
    next.ahead = next.status.ahead;
    Ok(next)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;

    fn git(cwd: &Path, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(cwd)
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    fn init(name: &str) -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join(name);
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.name", "OpenRidr Test"]);
        git(&root, &["config", "user.email", "test@openridr.invalid"]);
        (temp, root)
    }

    fn commit_all(root: &Path, message: &str) {
        git(root, &["add", "."]);
        git(root, &["commit", "-qm", message]);
    }

    fn repo_of(root: &Path) -> Repo {
        open(root).expect("the temporary repository opens")
    }

    #[test]
    fn info_reports_the_git_that_path_would_run() {
        let info = info();
        assert_eq!(info.state, "found");
        assert!(info.path.unwrap().ends_with("git"));
        assert!(info.version.unwrap().contains("git version"));
    }

    #[test]
    fn status_reports_branch_upstream_files_and_missing_repo() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "initial");
        git(&root, &["checkout", "-qb", "feature"]);
        git(&root, &["branch", "--set-upstream-to=main", "feature"]);
        fs::write(root.join("feature.txt"), "feature\n").unwrap();
        commit_all(&root, "feature work");
        fs::write(root.join("a.txt"), "one\ntwo\n").unwrap();
        fs::write(root.join("new.txt"), "new\n").unwrap();
        git(&root, &["add", "new.txt"]);
        git(&root, &["mv", "a.txt", "renamed.txt"]);
        fs::write(root.join("dirty.txt"), "x\n").unwrap();

        let current = status(&root);
        assert_eq!(current.state, "ok");
        assert_eq!(current.branch.as_deref(), Some("feature"));
        assert_eq!(current.upstream.as_deref(), Some("main"));
        assert_eq!(current.ahead, 1);
        assert!(!current.detached);
        assert!(!current.clean);
        let renamed = current
            .files
            .iter()
            .find(|file| file.path == "renamed.txt")
            .expect("rename is listed");
        assert_eq!(renamed.original_path.as_deref(), Some("a.txt"));
        assert!(renamed.staged);
        let staged = current
            .files
            .iter()
            .find(|file| file.path == "new.txt")
            .unwrap();
        assert_eq!(staged.label, "Staged");
        assert!(current.files.iter().any(|file| file.path == "dirty.txt"
            && file.untracked
            && file.label == "Untracked"));

        let plain = tempfile::tempdir().unwrap();
        let missing = status(plain.path());
        assert_eq!(missing.state, "not_a_repository");
        assert_eq!(missing.message.as_deref(), Some("Not a git repository"));
    }

    #[test]
    fn ignored_matches_patterns_and_never_a_tracked_file() {
        let (_temp, root) = init("repo");
        fs::write(root.join(".gitignore"), "*.log\nbuild/\n").unwrap();
        fs::create_dir_all(root.join("build")).unwrap();
        fs::write(root.join("build/app.log"), "x\n").unwrap();
        fs::write(root.join("other.log"), "y\n").unwrap();
        fs::write(root.join("tracked.log"), "z\n").unwrap();
        fs::write(root.join("App.cs"), "class App {}\n").unwrap();
        git(&root, &["add", ".gitignore", "App.cs"]);
        git(&root, &["add", "-f", "tracked.log"]);
        git(&root, &["commit", "-qm", "initial"]);

        let answer = ignored(
            &root,
            &[
                "other.log".to_string(),
                "build/app.log".to_string(),
                "App.cs".to_string(),
                "tracked.log".to_string(),
                "../outside.log".to_string(),
            ],
        );
        assert_eq!(answer.state, "ok", "{:?}", answer.message);
        // The ignored directory answers for the file below it; the tracked file never appears.
        assert_eq!(
            answer.ignored,
            vec!["other.log".to_string(), "build/app.log".to_string()]
        );
    }

    #[test]
    fn ignored_maps_a_workspace_inside_the_repository() {
        let (_temp, root) = init("repo");
        fs::create_dir_all(root.join("apps/web")).unwrap();
        fs::write(root.join(".gitignore"), "*.log\n").unwrap();
        fs::write(root.join("apps/web/a.log"), "x\n").unwrap();
        fs::write(root.join("apps/web/App.cs"), "class App {}\n").unwrap();
        git(&root, &["add", ".gitignore", "apps/web/App.cs"]);
        git(&root, &["commit", "-qm", "initial"]);

        let answer = ignored(
            &root.join("apps/web"),
            &["a.log".to_string(), "App.cs".to_string()],
        );
        assert_eq!(answer.state, "ok");
        assert_eq!(answer.ignored, vec!["a.log".to_string()]);
    }

    #[test]
    fn ignored_handles_spaces_and_newlines_in_paths() {
        let (_temp, root) = init("repo");
        fs::write(root.join(".gitignore"), "*.log\n").unwrap();
        let spaced = "my file.log";
        let newline = "line\nbreak.log";
        fs::write(root.join(spaced), "x\n").unwrap();
        fs::write(root.join(newline), "x\n").unwrap();
        git(&root, &["add", ".gitignore"]);
        git(&root, &["commit", "-qm", "initial"]);

        let answer = ignored(
            &root,
            &[spaced.to_string(), newline.to_string(), "missing.log".to_string()],
        );
        assert_eq!(answer.state, "ok");
        assert_eq!(answer.ignored, vec![spaced.to_string(), newline.to_string()]);
    }

    #[test]
    fn ignored_reports_a_missing_repository_as_data() {
        let temp = tempfile::tempdir().unwrap();
        let answer = ignored(temp.path(), &["a.log".to_string()]);
        assert_eq!(answer.state, "not_a_repository");
        assert!(answer.ignored.is_empty());
    }

    #[test]
    fn ignored_stops_at_the_batch_cap() {
        let (_temp, root) = init("repo");
        fs::write(root.join(".gitignore"), "*.log\n").unwrap();
        fs::write(root.join("z.log"), "x\n").unwrap();
        git(&root, &["add", ".gitignore"]);
        git(&root, &["commit", "-qm", "initial"]);
        let mut paths: Vec<String> = (0..256).map(|index| format!("missing{index}.cs")).collect();
        paths.push("z.log".to_string());
        let answer = ignored(&root, &paths);
        assert_eq!(answer.state, "ok");
        assert!(answer.ignored.is_empty(), "the 257th path is never queried");
    }

    #[test]
    fn status_reports_detached_head_and_in_progress_operation() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "initial");
        let head = git(&root, &["rev-parse", "HEAD"]);
        git(&root, &["checkout", "-q", head.trim()]);
        let detached = status(&root);
        assert!(detached.detached);
        assert_eq!(detached.branch, None);

        fs::write(root.join(".git/MERGE_HEAD"), "0000\n").unwrap();
        let merging = status(&root);
        assert_eq!(merging.operation, Some("merge"));
    }

    #[test]
    fn log_parses_messages_parents_and_changed_files() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "first ridge");
        fs::write(root.join("a.txt"), "one\ntwo\n").unwrap();
        git(&root, &["mv", "a.txt", "b.txt"]);
        commit_all(&root, "second ridge");

        let log = log(&root);
        assert_eq!(log.state, "ok");
        assert_eq!(log.limit, LOG_LIMIT);
        assert_eq!(log.commits.len(), 2);
        assert_eq!(log.commits[0].subject, "second ridge");
        assert_eq!(log.commits[0].parents.len(), 1);
        assert_eq!(log.commits[1].parents.len(), 0);
        assert_eq!(log.commits[0].author, "OpenRidr Test");
        let rename = log.commits[0]
            .files
            .iter()
            .find(|file| file.status.starts_with('R'))
            .expect("rename is listed");
        assert_eq!(rename.path, "b.txt");
        assert_eq!(rename.original_path.as_deref(), Some("a.txt"));
        assert!(!log.commits[0].short_hash.is_empty());
    }

    #[test]
    fn log_reports_an_empty_repository_without_an_error() {
        let (_temp, root) = init("repo");
        let empty = log(&root);
        assert_eq!(empty.state, "no_commits");
        assert!(empty.commits.is_empty());
    }

    #[test]
    fn diff_shows_worktree_staged_and_untracked_content() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "initial");
        fs::write(root.join("a.txt"), "one\ntwo\n").unwrap();
        fs::write(root.join("new.txt"), "brand new\n").unwrap();

        let worktree = diff(&root, "a.txt", false).unwrap();
        assert_eq!(worktree.state, "ok");
        assert!(worktree.diff.contains("+two"));
        let staged = diff(&root, "a.txt", true).unwrap();
        assert_eq!(staged.state, "empty");

        let untracked = diff(&root, "new.txt", false).unwrap();
        assert_eq!(untracked.state, "ok");
        assert!(untracked.diff.contains("+brand new"));

        let outside = diff(&root, "../escape.txt", false).unwrap_err();
        assert_eq!(outside.code, "invalid_state");
    }

    #[test]
    fn hunk_headers_parse_single_line_and_zero_count_ranges() {
        let hunks = parse_hunks("@@ -1 +1 @@\n@@ -0,0 +1,3 @@\n@@ -5,2 +4,0 @@\n");
        assert_eq!(
            hunks,
            vec![
                DiffHunk {
                    old_start: 1,
                    old_count: 1,
                    new_start: 1,
                    new_count: 1
                },
                DiffHunk {
                    old_start: 0,
                    old_count: 0,
                    new_start: 1,
                    new_count: 3
                },
                DiffHunk {
                    old_start: 5,
                    old_count: 2,
                    new_start: 4,
                    new_count: 0
                },
            ]
        );
    }

    #[test]
    fn file_diff_returns_full_sides_hunks_and_the_head_commit() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\ntwo\nthree\nfour\n").unwrap();
        commit_all(&root, "initial");
        fs::write(root.join("a.txt"), "one\nTWO\nthree\nfour\n").unwrap();
        let head = git(&root, &["rev-parse", "HEAD"]).trim().to_string();
        let short = git(&root, &["rev-parse", "--short", "HEAD"]).trim().to_string();

        let answer = file_diff(&root, "a.txt", false).unwrap();
        assert_eq!(answer.state, "ok");
        assert!(answer.full, "{:?}", answer.full_reason);
        assert_eq!(answer.base.text.as_deref(), Some("one\ntwo\nthree\nfour\n"));
        assert_eq!(answer.current.text.as_deref(), Some("one\nTWO\nthree\nfour\n"));
        assert_eq!(
            answer.hunks,
            vec![DiffHunk {
                old_start: 2,
                old_count: 1,
                new_start: 2,
                new_count: 1
            }]
        );
        let commit = answer.commit.expect("HEAD is reported");
        assert_eq!(commit.hash, head);
        assert_eq!(commit.short_hash, short);
        assert_eq!(commit.subject, "initial");
        assert!(answer.patch.contains("+TWO"));
    }

    #[test]
    fn file_diff_uses_the_index_as_the_unstaged_base_and_head_for_staged() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "initial");
        fs::write(root.join("a.txt"), "staged\n").unwrap();
        git(&root, &["add", "a.txt"]);
        fs::write(root.join("a.txt"), "worktree\n").unwrap();

        let unstaged = file_diff(&root, "a.txt", false).unwrap();
        assert_eq!(unstaged.base.text.as_deref(), Some("staged\n"));
        assert_eq!(unstaged.current.text.as_deref(), Some("worktree\n"));

        let staged = file_diff(&root, "a.txt", true).unwrap();
        assert_eq!(staged.base.text.as_deref(), Some("one\n"));
        assert_eq!(staged.current.text.as_deref(), Some("staged\n"));
        assert!(staged.commit.is_some());
    }

    #[test]
    fn file_diff_marks_added_and_deleted_sides() {
        let (_temp, root) = init("repo");
        fs::write(root.join("gone.txt"), "a\nb\n").unwrap();
        commit_all(&root, "initial");
        fs::remove_file(root.join("gone.txt")).unwrap();
        fs::write(root.join("new.txt"), "brand new\n").unwrap();

        let added = file_diff(&root, "new.txt", false).unwrap();
        assert_eq!(added.state, "ok");
        assert!(!added.base.exists);
        assert_eq!(added.base.text, None);
        assert_eq!(added.current.text.as_deref(), Some("brand new\n"));
        assert!(added.full);
        assert_eq!(added.hunks.len(), 1);

        let deleted = file_diff(&root, "gone.txt", false).unwrap();
        assert!(!deleted.current.exists);
        assert_eq!(deleted.current.text, None);
        assert_eq!(deleted.base.text.as_deref(), Some("a\nb\n"));
        assert!(deleted.full);
    }

    #[test]
    fn file_diff_falls_back_for_binary_and_oversized_sides() {
        let (_temp, root) = init("repo");
        fs::write(root.join("bin.dat"), b"ab\0cd").unwrap();
        let binary = file_diff(&root, "bin.dat", false).unwrap();
        assert_eq!(binary.state, "ok");
        assert!(!binary.base.exists);
        assert!(binary.current.exists);
        assert_eq!(binary.current.text, None);
        assert!(!binary.full);
        assert_eq!(binary.full_reason, Some("binary"));
        assert!(!binary.patch.is_empty());

        let big = "x".repeat(FULL_FILE_BYTES + 1);
        fs::write(root.join("big.txt"), &big).unwrap();
        let oversized = file_diff(&root, "big.txt", false).unwrap();
        assert!(!oversized.full);
        assert_eq!(oversized.full_reason, Some("too_large"));
    }

    #[test]
    fn file_diff_decodes_utf16_and_has_no_commit_before_the_first_one() {
        let (_temp, root) = init("repo");
        let mut bytes = vec![0xFF, 0xFE];
        for unit in "one\ntwo\n".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        fs::write(root.join("utf16.txt"), bytes).unwrap();
        let answer = file_diff(&root, "utf16.txt", false).unwrap();
        assert_eq!(answer.state, "ok");
        assert_eq!(answer.current.text.as_deref(), Some("one\ntwo\n"));
        assert!(answer.commit.is_none());
        assert!(!answer.base.exists);

        let outside = file_diff(&root, "../escape.txt", false).unwrap_err();
        assert_eq!(outside.code, "invalid_state");
    }

    #[test]
    fn hunk_ranges_are_read_from_the_decoded_text_for_utf16() {
        for little in [true, false] {
            let (_temp, root) = init(if little { "le" } else { "be" });
            let encode = |text: &str| {
                let mut bytes = if little {
                    vec![0xFF, 0xFE]
                } else {
                    vec![0xFE, 0xFF]
                };
                for unit in text.encode_utf16() {
                    let pair = if little {
                        unit.to_le_bytes()
                    } else {
                        unit.to_be_bytes()
                    };
                    bytes.extend_from_slice(&pair);
                }
                bytes
            };
            fs::write(root.join("u.txt"), encode("one\ntwo\nthree\n")).unwrap();
            commit_all(&root, "initial");
            fs::write(root.join("u.txt"), encode("one\nTWO\nthree\n")).unwrap();

            let answer = file_diff(&root, "u.txt", false).unwrap();
            assert_eq!(answer.state, "ok");
            assert!(answer.full, "{:?}", answer.full_reason);
            assert_eq!(answer.base.text.as_deref(), Some("one\ntwo\nthree\n"));
            assert_eq!(answer.current.text.as_deref(), Some("one\nTWO\nthree\n"));
            let expected = vec![DiffHunk {
                old_start: 2,
                old_count: 1,
                new_start: 2,
                new_count: 1,
            }];
            assert_eq!(answer.hunks, expected);

            git(&root, &["add", "u.txt"]);
            let staged = file_diff(&root, "u.txt", true).unwrap();
            assert!(staged.full, "{:?}", staged.full_reason);
            assert_eq!(staged.base.text.as_deref(), Some("one\ntwo\nthree\n"));
            assert_eq!(staged.current.text.as_deref(), Some("one\nTWO\nthree\n"));
            assert_eq!(staged.hunks, expected);
        }
    }

    #[test]
    fn blob_reads_tell_absence_apart_from_a_read_error() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "initial");

        assert!(matches!(
            read_blob(&repo_of(&root), "HEAD:nope"),
            Ok(SideBytes::Missing)
        ));
        assert!(matches!(
            read_blob(&repo_of(&root), ":nope"),
            Ok(SideBytes::Missing)
        ));
        assert!(matches!(
            read_blob(&repo_of(&root), "HEAD:a.txt"),
            Ok(SideBytes::Bytes(_))
        ));

        let blob = git(&root, &["rev-parse", "HEAD:a.txt"]).trim().to_string();
        let object = root.join(".git/objects").join(&blob[..2]).join(&blob[2..]);
        let mut permissions = fs::metadata(&object).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(&object, permissions).unwrap();
        fs::write(&object, b"corrupt").unwrap();

        // The path still resolves through the tree; only the content read fails.
        assert!(matches!(
            read_blob(&repo_of(&root), "HEAD:a.txt"),
            Ok(SideBytes::Unreadable)
        ));
        let answer = file_diff(&root, "a.txt", false).unwrap();
        assert_eq!(answer.state, "failed", "{:?}", answer.message);
        assert!(answer.message.unwrap_or_default().contains("could not be read"));
        assert!(answer.base.exists, "a read error must not look like a deletion");
        assert_eq!(answer.base.text, None);
        assert_eq!(answer.full_reason, Some("unreadable"));
        assert!(answer.patch.is_empty());
        assert!(!answer.full);
    }

    #[test]
    #[cfg(unix)]
    fn file_diff_refuses_symlinks_that_leave_the_repository() {
        let (_temp, root) = init("repo");
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret.txt"), "outside secret\n").unwrap();
        fs::write(root.join("keep.txt"), "keep\n").unwrap();
        commit_all(&root, "initial");

        // A symlinked directory would take our read and Git's worktree side out of the repo.
        std::os::unix::fs::symlink(outside.path(), root.join("outdir")).unwrap();
        let escaped = file_diff(&root, "outdir/secret.txt", false).unwrap();
        assert_eq!(escaped.state, "failed");
        assert!(escaped.message.unwrap_or_default().contains("symbolic link"));
        assert!(escaped.patch.is_empty());
        assert_eq!(escaped.base.text, None);
        assert_eq!(escaped.current.text, None);
        let legacy = diff(&root, "outdir/secret.txt", false).unwrap();
        assert_eq!(legacy.state, "failed");
        assert!(!legacy.diff.contains("outside secret"));

        // A tracked file whose directory was swapped for a symlink is refused before Git reads it.
        fs::create_dir_all(root.join("realdir")).unwrap();
        fs::write(root.join("realdir/a.txt"), "inside\n").unwrap();
        commit_all(&root, "add tracked");
        fs::rename(root.join("realdir"), outside.path().join("swapped")).unwrap();
        std::os::unix::fs::symlink(outside.path().join("swapped"), root.join("realdir")).unwrap();
        fs::write(
            outside.path().join("swapped/a.txt"),
            "EXTERNAL SECRET\n",
        )
        .unwrap();
        let tracked = file_diff(&root, "realdir/a.txt", false).unwrap();
        assert_eq!(tracked.state, "failed");
        assert!(!tracked.patch.contains("EXTERNAL SECRET"));

        // A symlink file is a link object, exactly like Git stores it: the target path, not its
        // content, and never the content of a file outside the workspace.
        std::os::unix::fs::symlink(
            outside.path().join("secret.txt"),
            root.join("link.txt"),
        )
        .unwrap();
        let link = file_diff(&root, "link.txt", false).unwrap();
        assert_eq!(link.state, "ok");
        assert!(!link.patch.contains("outside secret"));
        assert_eq!(
            link.current.text.as_deref(),
            Some(outside.path().join("secret.txt").to_str().unwrap())
        );
    }

    #[test]
    fn patch_capture_is_bounded_and_the_ranges_still_align_the_full_texts() {
        let (_temp, root) = init("repo");
        let original: String = (0..6_000)
            .map(|index| format!("line {index:04} {}\n", "x".repeat(86)))
            .collect();
        fs::write(root.join("big.txt"), &original).unwrap();
        commit_all(&root, "initial");
        let rewritten: String = (0..6_000)
            .map(|index| format!("changed {index:04} {}\n", "y".repeat(83)))
            .collect();
        fs::write(root.join("big.txt"), &rewritten).unwrap();

        let patch = diff(&root, "big.txt", false).unwrap();
        assert_eq!(patch.state, "ok");
        assert!(patch.truncated, "the patch must say it was cut");
        assert!(patch.diff.chars().count() <= DIFF_LIMIT + 80);

        let answer = file_diff(&root, "big.txt", false).unwrap();
        assert!(answer.full, "{:?}", answer.full_reason);
        assert_eq!(answer.hunks.len(), 1);
        assert!(answer.truncated);
        assert!(answer.patch.chars().count() <= DIFF_LIMIT + 80);
    }

    #[test]
    fn full_file_sides_stop_at_the_limit_without_collecting_the_file() {
        let (_temp, root) = init("repo");
        let huge = "z".repeat(FULL_FILE_BYTES + 4_096);
        fs::write(root.join("huge.txt"), &huge).unwrap();
        let answer = file_diff(&root, "huge.txt", false).unwrap();
        assert_eq!(answer.state, "ok");
        assert!(answer.current.exists);
        assert_eq!(answer.current.text, None);
        assert!(!answer.full);
        assert_eq!(answer.full_reason, Some("too_large"));
        assert!(answer.truncated, "the bounded patch still says it was cut");
        assert!(!answer.patch.is_empty(), "the fallback stays usable");
        assert!(answer.patch.starts_with("diff --git"));
        assert!(answer.patch.chars().count() <= DIFF_LIMIT + 80);

        // A committed side is rejected from the blob size, before any content byte is read.
        fs::write(root.join("committed.txt"), &huge).unwrap();
        commit_all(&root, "big");
        fs::write(root.join("committed.txt"), format!("{huge}b")).unwrap();
        let committed = file_diff(&root, "committed.txt", false).unwrap();
        assert_eq!(committed.full_reason, Some("too_large"));
        assert_eq!(committed.base.text, None);
        assert_eq!(committed.current.text, None);
    }

    #[cfg(unix)]
    fn decoded_temporaries() -> Vec<String> {
        let prefix = format!("openridr-diff-{}-", std::process::id());
        fs::read_dir(std::env::temp_dir())
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .filter(|name| name.starts_with(&prefix))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    #[cfg(unix)]
    fn decoded_diff_temporaries_are_private_and_removed() {
        let before = decoded_temporaries();
        let hunks = decoded_hunks("a\nb\n", "a\nB\n").expect("both texts diff");
        assert_eq!(
            hunks,
            vec![DiffHunk {
                old_start: 2,
                old_count: 1,
                new_start: 2,
                new_count: 1
            }]
        );
        assert_eq!(decoded_temporaries(), before, "no temporary file survives");

        let path = std::env::temp_dir().join(format!(
            "openridr-diff-private-check-{}",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        write_private(&path, "secret\n").unwrap();
        let mode = std::os::unix::fs::PermissionsExt::mode(&fs::metadata(&path).unwrap().permissions());
        let _ = fs::remove_file(&path);
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    #[cfg(unix)]
    fn both_reads_refuse_a_worktree_reached_through_a_symlink_in_every_staged_state() {
        let (_temp, root) = init("repo");
        let outside = tempfile::tempdir().unwrap();
        let marker = "R2-SECRET-MARKER-48";
        fs::write(outside.path().join("secret.txt"), format!("{marker}\n")).unwrap();
        fs::write(root.join("keep.txt"), "keep\n").unwrap();
        commit_all(&root, "initial");
        std::os::unix::fs::symlink(outside.path(), root.join("outdir")).unwrap();

        for staged in [false, true] {
            let legacy = diff(&root, "outdir/secret.txt", staged).unwrap();
            let full = file_diff(&root, "outdir/secret.txt", staged).unwrap();
            assert!(!legacy.diff.contains(marker), "staged={staged}");
            assert!(!full.patch.contains(marker), "staged={staged}");
            assert!(
                !full.base.text.as_deref().unwrap_or_default().contains(marker),
                "staged={staged}"
            );
            assert!(
                !full.current.text.as_deref().unwrap_or_default().contains(marker),
                "staged={staged}"
            );
            assert!(
                !full.message.clone().unwrap_or_default().contains(marker),
                "staged={staged}"
            );
            if staged {
                // HEAD -> index never reads the worktree: an untracked path simply has no diff.
                assert_eq!(legacy.state, "empty", "staged={staged}");
                assert_eq!(full.state, "empty", "staged={staged}");
            } else {
                assert_eq!(legacy.state, "failed", "staged={staged}");
                assert_eq!(full.state, "failed", "staged={staged}");
                assert!(
                    full.message.unwrap_or_default().contains("symbolic link"),
                    "staged={staged}"
                );
            }
        }
    }

    #[test]
    fn staged_reads_keep_head_to_index_for_tracked_and_deleted_paths() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        fs::write(root.join("gone.txt"), "bye\n").unwrap();
        commit_all(&root, "initial");
        fs::write(root.join("a.txt"), "ONE\n").unwrap();
        git(&root, &["add", "a.txt"]);
        git(&root, &["rm", "-q", "gone.txt"]);

        let tracked = file_diff(&root, "a.txt", true).unwrap();
        assert_eq!(tracked.state, "ok");
        assert_eq!(tracked.base.text.as_deref(), Some("one\n"));
        assert_eq!(tracked.current.text.as_deref(), Some("ONE\n"));
        assert_eq!(
            tracked.hunks,
            vec![DiffHunk {
                old_start: 1,
                old_count: 1,
                new_start: 1,
                new_count: 1
            }]
        );
        let legacy = diff(&root, "a.txt", true).unwrap();
        assert_eq!(legacy.state, "ok");
        assert!(legacy.diff.contains("+ONE"));

        // A staged deletion has no index entry; it must still be HEAD -> index, not --no-index.
        let deleted = file_diff(&root, "gone.txt", true).unwrap();
        assert_eq!(deleted.state, "ok", "{:?}", deleted.message);
        assert_eq!(deleted.base.text.as_deref(), Some("bye\n"));
        assert!(!deleted.current.exists);
        assert_eq!(deleted.current.text, None);
        assert_eq!(
            deleted.hunks,
            vec![DiffHunk {
                old_start: 1,
                old_count: 1,
                new_start: 0,
                new_count: 0
            }]
        );
        let legacy_deleted = diff(&root, "gone.txt", true).unwrap();
        assert_eq!(legacy_deleted.state, "ok");
        assert!(legacy_deleted.diff.contains("-bye"));
    }

    #[test]
    fn corrupt_index_and_tree_reads_are_errors_not_absences() {
        // A corrupt tree keeps `HEAD:a.txt` unresolvable: that is a read error, not an addition.
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "initial");
        let tree = git(&root, &["rev-parse", "HEAD^{tree}"]).trim().to_string();
        let object = root.join(".git/objects").join(&tree[..2]).join(&tree[2..]);
        let mut permissions = fs::metadata(&object).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(&object, permissions).unwrap();
        fs::write(&object, b"corrupt").unwrap();
        assert!(matches!(
            read_blob(&repo_of(&root), "HEAD:a.txt"),
            Ok(SideBytes::Unreadable)
        ));
        // Only the staged read needs the HEAD tree, and it must fail instead of inventing a side.
        let broken_tree = file_diff(&root, "a.txt", true).unwrap();
        assert_eq!(broken_tree.state, "failed");
        assert!(!broken_tree.message.clone().unwrap_or_default().is_empty());

        // A corrupt index must not become "untracked" (and then a --no-index read).
        let (_temp2, root2) = init("repo2");
        fs::write(root2.join("a.txt"), "one\n").unwrap();
        commit_all(&root2, "initial");
        let index = root2.join(".git/index");
        let mut permissions = fs::metadata(&index).unwrap().permissions();
        permissions.set_readonly(false);
        fs::set_permissions(&index, permissions).unwrap();
        fs::write(&index, b"garbage").unwrap();
        assert!(matches!(
            read_blob(&repo_of(&root2), ":a.txt"),
            Ok(SideBytes::Unreadable)
        ));
        assert!(is_tracked(&repo_of(&root2), "a.txt").is_err());
        let broken_index = file_diff(&root2, "a.txt", false).unwrap();
        assert_eq!(broken_index.state, "failed");
        assert!(broken_index
            .message
            .unwrap_or_default()
            .contains("index"));
    }

    #[test]
    fn commit_only_keeps_staged_paths_and_handles_amend_and_untracked() {
        let (_temp, root) = init("repo");
        fs::write(root.join("staged.txt"), "base\n").unwrap();
        fs::write(root.join("picked.txt"), "base\n").unwrap();
        commit_all(&root, "initial");

        fs::write(root.join("staged.txt"), "base\nstaged\n").unwrap();
        git(&root, &["add", "staged.txt"]);
        fs::write(root.join("picked.txt"), "base\npicked\n").unwrap();
        fs::write(root.join("new.txt"), "new file\n").unwrap();

        let result = commit(
            &root,
            &["picked.txt".to_string(), "new.txt".to_string()],
            "selective",
            false,
        )
        .unwrap();
        assert_eq!(result.state, "ok", "{:?}", result.message);
        let hash = result.commit.as_ref().unwrap().hash.clone();
        let changed = git(&root, &["show", "--name-only", "--format=", &hash]);
        let paths: Vec<&str> = changed.lines().filter(|line| !line.is_empty()).collect();
        assert_eq!(paths, vec!["new.txt", "picked.txt"]);
        let staged = git(&root, &["diff", "--cached", "--name-only"]);
        assert_eq!(staged.trim(), "staged.txt");
        assert_eq!(git(&root, &["status", "--porcelain"]).trim(), "M  staged.txt");

        fs::write(root.join("picked.txt"), "base\npicked\nagain\n").unwrap();
        let amended = commit(&root, &["picked.txt".to_string()], "amended subject", true).unwrap();
        assert_eq!(amended.state, "ok", "{:?}", amended.message);
        assert_eq!(
            git(&root, &["log", "-1", "--format=%s"]).trim(),
            "amended subject"
        );
        assert_eq!(git(&root, &["rev-list", "--count", "HEAD"]).trim(), "2");
        assert_eq!(git(&root, &["diff", "--cached", "--name-only"]).trim(), "staged.txt");
        let new_tracked = git(&root, &["ls-files", "new.txt"]);
        assert_eq!(new_tracked.trim(), "new.txt");
    }

    #[test]
    fn checkout_and_pull_refuse_dirty_trees_and_in_progress_operations() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "initial");
        git(&root, &["checkout", "-qb", "other"]);
        git(&root, &["checkout", "-q", "main"]);

        fs::write(root.join("a.txt"), "dirty\n").unwrap();
        let dirty = checkout(&root, "other").unwrap_err();
        assert_eq!(dirty.code, "git_dirty_tree");
        assert!(dirty.message.contains("does not auto-stash"));
        assert_eq!(status(&root).branch.as_deref(), Some("main"));

        let dirty_pull = pull(&root).unwrap_err();
        assert_eq!(dirty_pull.code, "git_dirty_tree");

        git(&root, &["checkout", "--", "a.txt"]);
        fs::write(root.join(".git/MERGE_HEAD"), "0000\n").unwrap();
        let merging = checkout(&root, "other").unwrap_err();
        assert_eq!(merging.code, "git_operation_in_progress");
        assert!(merging.message.contains("merge"));
        let merging_pull = pull(&root).unwrap_err();
        assert_eq!(merging_pull.code, "git_operation_in_progress");
        fs::remove_file(root.join(".git/MERGE_HEAD")).unwrap();

        let created = branch_create(&root, "third", true).unwrap();
        assert_eq!(created.state, "ok");
        assert_eq!(created.status.branch.as_deref(), Some("third"));
    }

    #[test]
    fn checkout_of_a_remote_branch_creates_a_local_tracking_branch() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "initial");
        let remote = tempfile::tempdir().unwrap();
        git(remote.path(), &["init", "-q", "-b", "main", "--bare"]);
        git(
            &root,
            &[
                "remote",
                "add",
                "origin",
                remote.path().to_str().unwrap(),
            ],
        );
        git(&root, &["push", "-q", "-u", "origin", "main"]);
        git(&root, &["checkout", "-qb", "feature"]);
        fs::write(root.join("a.txt"), "two\n").unwrap();
        commit_all(&root, "feature work");
        git(&root, &["push", "-q", "-u", "origin", "feature"]);
        git(&root, &["checkout", "-q", "main"]);
        git(&root, &["branch", "-q", "-D", "feature"]);
        git(&root, &["fetch", "-q", "origin"]);

        let switched = checkout(&root, "origin/feature").unwrap();
        assert_eq!(switched.state, "ok", "{:?}", switched.message);
        assert_eq!(switched.status.branch.as_deref(), Some("feature"));
        assert_eq!(switched.status.upstream.as_deref(), Some("origin/feature"));
    }

    #[test]
    fn push_requires_confirmation_never_forces_and_sets_upstream() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "initial");
        let remote = tempfile::tempdir().unwrap();
        let bare = remote.path().join("origin.git");
        fs::create_dir_all(&bare).unwrap();
        git(&bare, &["init", "-q", "-b", "main", "--bare"]);

        let missing = push(&root, &serde_json::json!({})).unwrap_err();
        assert_eq!(missing.code, "confirmation_required");
        let no_remote = push(&root, &serde_json::json!({ "confirm": true })).unwrap_err();
        assert_eq!(no_remote.code, "git_no_remote");

        git(&root, &["remote", "add", "origin", bare.to_str().unwrap()]);
        let pushed = push(&root, &serde_json::json!({ "confirm": true })).unwrap();
        assert_eq!(pushed.state, "ok", "{:?}", pushed.message);
        assert_eq!(pushed.remote.as_deref(), Some("origin"));
        assert_eq!(pushed.status.upstream.as_deref(), Some("origin/main"));
        let count = git(&bare, &["rev-list", "--count", "main"]);
        assert_eq!(count.trim(), "1");
    }

    #[test]
    fn branch_listing_marks_current_local_and_remote_branches() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "initial");
        git(&root, &["checkout", "-qb", "feature"]);

        let listed = branches(&root);
        assert_eq!(listed.state, "ok");
        assert_eq!(listed.current.as_deref(), Some("feature"));
        assert_eq!(listed.local[0].name, "feature");
        assert!(listed.local[0].current);
        let plain = tempfile::tempdir().unwrap();
        assert_eq!(branches(plain.path()).state, "not_a_repository");
    }

    #[test]
    fn masked_output_hides_credentials() {
        let text = "fatal: unable to access 'https://user:secret-token@example.com/repo.git/': denied";
        let masked = mask_urls(text);
        assert!(!masked.contains("secret-token"), "{masked}");
        assert!(!masked.contains("user:"), "{masked}");
        assert!(masked.contains("https://***@example.com/repo.git/"), "{masked}");
        assert_eq!(mask_urls("plain text"), "plain text");
    }
}
