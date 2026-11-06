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
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};

pub const LOG_LIMIT: usize = 200;
pub const COMMIT_FILES_LIMIT: usize = 200;
pub const DIFF_LIMIT: usize = 240_000;
pub const OUTPUT_LIMIT: usize = 32_000;
/// Full-file sides are a read for the diff panel, never a guarantee: above these the read falls
/// back to the bounded patch, and the panel says why.
pub const FULL_FILE_BYTES: usize = 1_048_576;
pub const FULL_FILE_LINES: usize = 30_000;
const BINARY_PROBE: usize = 8_192;

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

enum OpenError {
    NotRepository,
    Missing,
    Failed(String),
}

struct Output {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Output {
    fn ok(&self) -> bool {
        self.code == Some(0)
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
) -> Result<Output, SpawnError> {
    if tracked {
        let mut args = vec!["diff", "--no-color", "--no-ext-diff", unified];
        if staged {
            args.push("--cached");
        }
        args.push("--");
        args.push(path);
        run(&repo.top, &args)
    } else {
        // An untracked file has no index entry to diff against; /dev/null shows its content.
        run(
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
        )
    }
}

fn is_tracked(repo: &Repo, path: &str) -> Result<bool, SpawnError> {
    Ok(run(&repo.top, &["ls-files", "--error-unmatch", "--", path])?.ok())
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
        Err(error) => return Ok(GitDiff::unavailable("failed", spawn_message(error), path, staged)),
    };
    let output = match diff_output(&repo, path, staged, tracked, "-U3") {
        Ok(output) => output,
        Err(error) => return Ok(GitDiff::unavailable("failed", spawn_message(error), path, staged)),
    };
    if output.code != Some(0) && output.code != Some(1) {
        let stderr = mask_urls(output.stderr.trim());
        return Ok(GitDiff::unavailable("failed", stderr, path, staged));
    }
    let text = mask_urls(&output.stdout);
    if text.trim().is_empty() {
        return Ok(GitDiff::unavailable(
            "empty",
            format!("No diff for {path}."),
            path,
            staged,
        ));
    }
    let (diff, truncated) = cap(&text, DIFF_LIMIT);
    Ok(GitDiff {
        state: "ok",
        message: None,
        path: path.to_string(),
        staged,
        diff,
        truncated,
    })
}

/// One side's bytes, keeping "does not exist" apart from "exists but cannot be read".
enum SideBytes {
    Missing,
    Bytes(Vec<u8>),
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

fn read_blob(repo: &Repo, spec: &str) -> SideBytes {
    match spawn_bytes(&repo.top, &["cat-file", "blob", spec]) {
        Ok((Some(0), stdout, _)) => SideBytes::Bytes(stdout),
        // A failed `cat-file` is "this revision does not have that path" for every caller here:
        // added files, deletions and a repository without commits all land in this arm.
        _ => SideBytes::Missing,
    }
}

fn read_worktree(repo: &Repo, path: &str) -> SideBytes {
    match std::fs::read(repo.top.join(path)) {
        Ok(bytes) => SideBytes::Bytes(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => SideBytes::Missing,
        Err(_) => SideBytes::Unreadable,
    }
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
        Err(error) => {
            return Ok(GitFileDiff::unavailable(
                "failed",
                spawn_message(error),
                path,
                staged,
            ))
        }
    };
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
    if patch_output.code != Some(0) && patch_output.code != Some(1) {
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
    if hunk_output.code == Some(0) || hunk_output.code == Some(1) {
        hunks = parse_hunks(&hunk_output.stdout);
    }

    // Unstaged compares the index with the worktree; staged compares HEAD with the index.
    let base_spec = if staged {
        format!("HEAD:{path}")
    } else {
        format!(":{path}")
    };
    let (base, base_reason) = read_side(read_blob(&repo, &base_spec));
    let (current, current_reason) = if staged {
        read_side(read_blob(&repo, &format!(":{path}")))
    } else {
        read_side(read_worktree(&repo, path))
    };
    let mut full_reason = base_reason.or(current_reason);
    let mut full = (!base.exists || base.text.is_some())
        && (!current.exists || current.text.is_some());

    let text = mask_urls(&patch_output.stdout);
    let commit = head_commit(&repo);
    if text.trim().is_empty() {
        return Ok(GitFileDiff {
            state: "empty",
            message: Some(format!("No diff for {path}.")),
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
    if full && hunks.is_empty() {
        // The patch says something changed, but the block ranges that align the two texts are
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
