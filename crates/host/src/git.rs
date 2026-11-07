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
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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
/// Every network Git command is bounded in time. Every host dispatch runs behind one lock in the
/// shell, so a command waiting on an unreachable remote freezes the whole app, not just the panel
/// that started it. The bound is the freeze ceiling, not a transfer budget: a push uploads objects
/// and waits for the server's hooks, so it gets twice the download budget. A command that needs
/// longer is killed and retried, never left holding the lock.
const FETCH_DEADLINE: Duration = Duration::from_secs(60);
const PUSH_DEADLINE: Duration = Duration::from_secs(120);
/// How often the bounded fetch checks whether its child finished. Short enough to answer soon
/// after a kill, long enough to keep a healthy fetch off the CPU.
const FETCH_POLL: Duration = Duration::from_millis(20);
/// How long the bounded fetch waits for its pipe readers after the child is gone. A reader that is
/// still blocked after this is holding a pipe nothing here can close; the call returns anyway.
const DRAIN_GRACE: Duration = Duration::from_secs(2);
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
    std::env::var_os("PATH").and_then(|value| {
        std::env::split_paths(&value)
            .filter(|dir| !dir.as_os_str().is_empty())
            .map(|dir| dir.join(if cfg!(windows) { "git.exe" } else { "git" }))
            .find(|candidate| candidate.is_file())
            .map(|candidate| candidate.to_string_lossy().into_owned())
    })
}

pub fn info() -> GitInfo {
    let path = path();
    let version = Command::new("git")
        .arg("--version")
        // Off for the optional stat-cache lock only: the host must never hold `.git/index.lock`
        // against the user's own git command. Real writes still take it.
        .env("GIT_OPTIONAL_LOCKS", "0")
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
    fn unavailable(
        state: &'static str,
        message: impl Into<String>,
        path: &str,
        staged: bool,
    ) -> Self {
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
    fn unavailable(
        state: &'static str,
        message: impl Into<String>,
        path: &str,
        staged: bool,
    ) -> Self {
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

/// The diff of one file of a commit. Both sides are blobs of the commit and its first parent, so
/// nothing here reads the worktree: an uncommitted edit cannot change the answer.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitCommitDiff {
    pub state: &'static str,
    pub message: Option<String>,
    pub path: String,
    pub original_path: Option<String>,
    /// The commit whose file is shown; `None` only when the read itself failed.
    pub commit: Option<DiffCommit>,
    /// The first parent, or `None` for a root commit.
    pub parent: Option<DiffCommit>,
    pub base: DiffSide,
    pub current: DiffSide,
    pub hunks: Vec<DiffHunk>,
    pub patch: String,
    pub truncated: bool,
    pub full: bool,
    pub full_reason: Option<&'static str>,
}

impl GitCommitDiff {
    fn unavailable(
        state: &'static str,
        message: impl Into<String>,
        path: &str,
        original_path: Option<&str>,
    ) -> Self {
        Self {
            state,
            message: Some(message.into()),
            path: path.to_string(),
            original_path: original_path.map(str::to_string),
            commit: None,
            parent: None,
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
    /// Why the run stopped early. Internal: the published view carries the state instead.
    #[serde(skip)]
    pub stop: Stop,
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
        // Off for the optional stat-cache lock only: the host must never hold `.git/index.lock`
        // against the user's own git command. Real writes still take it.
        .env("GIT_OPTIONAL_LOCKS", "0")
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
        .env("GIT_OPTIONAL_LOCKS", "0")
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

/// The network Git operation in flight, published in the snapshot like `build`/`run`/`tests`.
/// One record per host instance, never a process-wide one: a `static` would let a test's cancel
/// (the host tests run in parallel) or another app instance kill an operation it never started.
///
/// Every transition happens under one mutex, and the id is issued **here**: a cancel can only name
/// an operation the host started, and the end of the network step is atomic with it, so a cancel
/// either lands before (and the operation reports itself cancelled) or after (and the host answers
/// that there was nothing to stop).
#[derive(Default)]
pub struct Net {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    seq: u64,
    running: Option<Running>,
    last: Option<NetView>,
}

struct Running {
    id: String,
    kind: Kind,
    cancelled: bool,
    /// The child ended: from here on a cancel has nothing to stop, even before `end` publishes the
    /// result. This is what makes the local merge of an Update uncancellable (#207).
    finished: bool,
}

/// Which network operation a record belongs to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Push,
    Update,
    Fetch,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Push => "push",
            Kind::Update => "update",
            Kind::Fetch => "fetch",
        }
    }
}

/// The published state of the network operation: what the snapshot carries and the UI renders.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetView {
    pub id: String,
    pub kind: &'static str,
    pub state: &'static str,
    pub message: Option<String>,
}

impl Net {
    pub fn idle_view() -> NetView {
        NetView {
            id: String::new(),
            kind: "",
            state: "idle",
            message: None,
        }
    }

    /// A panic elsewhere must not turn every later operation into a panic: the state is a few
    /// fields, so a poisoned lock is still readable.
    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The operation in flight, else the last one that ended, else idle.
    pub fn view(&self) -> NetView {
        let state = self.lock();
        match &state.running {
            Some(running) => NetView {
                id: running.id.clone(),
                kind: running.kind.as_str(),
                state: "running",
                message: None,
            },
            None => state.last.clone().unwrap_or_else(Self::idle_view),
        }
    }

    /// Registers one operation before any work runs and returns the id the host issued. One
    /// network operation per host, like a build: a second start is refused, naming the one in
    /// flight.
    pub fn begin(&self, kind: Kind) -> Result<String, String> {
        let mut state = self.lock();
        if let Some(running) = &state.running {
            return Err(format!(
                "A {} is already running ({}). Cancel it before starting another.",
                running.kind.as_str(),
                running.id
            ));
        }
        state.seq += 1;
        let id = format!("git:{}", state.seq);
        state.running = Some(Running {
            id: id.clone(),
            kind,
            cancelled: false,
            finished: false,
        });
        Ok(id)
    }

    /// Whether a cancel landed on **that** operation. Read by the wait loop, which owns the child:
    /// nothing here signals a pid it does not own.
    pub fn cancelled(&self, id: &str) -> bool {
        self.lock()
            .running
            .as_ref()
            .is_some_and(|running| running.id == id && running.cancelled)
    }

    /// Ends the network step: returns whether a cancel landed on it and closes the window, under
    /// the same lock as `cancel`. Without that, a cancel could be accepted in the gap and the
    /// operation would carry on.
    pub fn finish_network(&self, id: &str) -> bool {
        let mut state = self.lock();
        match state.running.as_mut() {
            Some(running) if running.id == id && !running.finished => {
                running.finished = true;
                running.cancelled
            }
            _ => false,
        }
    }

    /// Stops the operation that carries this id, and only it: nothing in flight, an id the host
    /// never issued, or a network step that already ended all answer `false`.
    pub fn cancel(&self, id: &str) -> bool {
        let mut state = self.lock();
        match state.running.as_mut() {
            Some(running) if running.id == id && !running.finished => {
                running.cancelled = true;
                true
            }
            _ => false,
        }
    }

    /// Publishes the result of the operation: `cancelled` wins over the operation's own state, and
    /// an error becomes a `failed` view with its message. Keeps the id and kind the host issued.
    pub fn end(&self, id: &str, result: HostResult<Operation>) {
        let mut state = self.lock();
        let running = match state.running.as_ref() {
            Some(running) if running.id == id => Some((running.kind, running.cancelled)),
            _ => None,
        };
        if running.is_some() {
            state.running = None;
        }
        // A cancel that was accepted is what the operation reports, even when it then failed on its
        // own (a dirty tree, no remote): `git.cancel` answering `true` must never end as `ok` or as
        // a failure the user reads as "it ran anyway". The message stays the operation's own, so the
        // reason it did not run is still the one shown.
        let cancelled = running.is_some_and(|(_, cancelled)| cancelled);
        let (state_name, message) = match result {
            Ok(operation) => (
                if cancelled || operation.stop == Stop::Cancelled {
                    "cancelled"
                } else {
                    operation.state
                },
                operation.message,
            ),
            Err(error) => (
                if cancelled { "cancelled" } else { "failed" },
                Some(error.message),
            ),
        };
        state.last = Some(NetView {
            id: id.to_string(),
            kind: running.map(|(kind, _)| kind.as_str()).unwrap_or(""),
            state: state_name,
            message,
        });
    }
}

/// Why a bounded run stopped early. `No` means the child finished on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    No,
    Deadline,
    Cancelled,
}

/// A `git` run that is bounded in time: at `deadline` or when `net` is cancelled the child is
/// killed and the reason says which. Both pipes are drained on their own threads, so neither a
/// chatty failure nor a child waiting to write can deadlock the wait — and a killed child can still
/// be reaped, so no zombie outlives it.
fn run_with_deadline(
    cwd: &Path,
    args: &[&str],
    deadline: Duration,
    net: &Net,
    id: &str,
) -> Result<(Output, Stop), SpawnError> {
    // A cancel that arrived before the dispatch must cost nothing: no child, no connection.
    if net.cancelled(id) {
        return Ok((
            Output {
                code: None,
                stdout: String::new(),
                stderr: String::new(),
                truncated: false,
            },
            Stop::Cancelled,
        ));
    }
    let mut command = Command::new("git");
    command
        .args(args)
        .current_dir(cwd)
        .env("GIT_TERMINAL_PROMPT", "0")
        // Same as the other spawns: never hold the optional index lock for a read.
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // Its own process group, so the deadline can reach what git spawned: `git-remote-http`,
        // ssh and a credential helper all inherit the pipes, and killing only the leader would
        // leave a reader waiting on a pipe a grandchild still holds — the deadline would not
        // return at all, which is the freeze this bound exists to prevent.
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|error| match error.kind() {
        std::io::ErrorKind::NotFound => SpawnError::Missing,
        _ => SpawnError::Failed(error.to_string()),
    })?;
    let pid = child.id();
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    // The readers report through channels instead of being joined: a pipe held by something the
    // kill could not reach must cost one bounded wait, never the whole host call.
    let (out_tx, out_rx) = mpsc::channel();
    let (err_tx, err_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = out_tx.send(drain_bounded(&mut stdout));
    });
    std::thread::spawn(move || {
        let _ = err_tx.send(drain_bounded(&mut stderr));
    });
    let began = Instant::now();
    let mut stop = Stop::No;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) => {
                // The cancel is checked first: when both land in the same tick the user's click is
                // the reason the message names.
                if net.cancelled(id) {
                    stop = Stop::Cancelled;
                    kill_tree(&mut child, pid);
                    break child.wait().ok();
                }
                if began.elapsed() >= deadline {
                    stop = Stop::Deadline;
                    kill_tree(&mut child, pid);
                    break child.wait().ok();
                }
                std::thread::sleep(FETCH_POLL);
            }
            Err(_) => {
                kill_tree(&mut child, pid);
                break child.wait().ok();
            }
        }
    };
    // Before the drain: the child is gone, so the network step is over even if reading its pipes
    // still takes a moment. `finish_network` is atomic with `cancel`, so a cancel that lands here
    // makes the operation report itself cancelled instead of being accepted and then ignored.
    // Always: the window closes when the child ends, whatever stopped it. Without this a cancel
    // that arrives while the pipes drain would be accepted for a child that is already dead.
    let cancelled = net.finish_network(id);
    if stop == Stop::No && cancelled {
        stop = Stop::Cancelled;
    }
    let stdout = out_rx.recv_timeout(DRAIN_GRACE).unwrap_or_default();
    let stderr = err_rx.recv_timeout(DRAIN_GRACE).unwrap_or_default();
    Ok((
        Output {
            code: status.and_then(|status| status.code()),
            stdout: String::from_utf8_lossy(&stdout.0).into_owned(),
            stderr: String::from_utf8_lossy(&stderr.0).into_owned(),
            truncated: stdout.1 || stderr.1,
        },
        stop,
    ))
}

/// Kills the git child and everything it spawned. The leader is not reaped yet, so its pid (and
/// therefore the group id) cannot be reused while the signal is in flight.
fn kill_tree(child: &mut std::process::Child, pid: u32) {
    #[cfg(unix)]
    {
        let _ = crate::process::kill_group(pid, true);
    }
    #[cfg(not(unix))]
    let _ = pid;
    let _ = child.kill();
}

/// Reads a pipe to its end and keeps the **last** `STDERR_BYTES`: what a failure says is at the end
/// of its output, so a truncated buffer must lose the beginning, not the message. A child that
/// never closes its pipe cannot grow the host either.
fn drain_bounded<R: Read>(reader: &mut R) -> (Vec<u8>, bool) {
    let mut kept: Vec<u8> = Vec::new();
    let mut dropped = false;
    let mut buffer = [0u8; 4096];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(count) => {
                kept.extend_from_slice(&buffer[..count]);
                if kept.len() > STDERR_BYTES * 2 {
                    let cut = kept.len() - STDERR_BYTES;
                    kept.drain(..cut);
                    dropped = true;
                }
            }
        }
    }
    if kept.len() > STDERR_BYTES {
        let cut = kept.len() - STDERR_BYTES;
        kept.drain(..cut);
        dropped = true;
    }
    (kept, dropped)
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
        Some((index, _)) => (
            format!("{}\n… truncated at {limit} characters", &text[..index]),
            true,
        ),
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
    if records
        .first()
        .is_some_and(|record| record.starts_with("## "))
    {
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
        if (matches!(x, 'R' | 'C') || matches!(y, 'R' | 'C')) && index < records.len() {
            original_path = Some(records[index].to_string());
            index += 1;
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
    let output =
        run(&repo.top, &["status", "--porcelain=v1", "-z", "--branch"]).map_err(spawn_message)?;
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
        // A pathname that starts with `:` is pathspec magic, and `*`, `?`, `[` are wildcards;
        // the tree sends literal names, so the global flag keeps the pathspec literal.
        "--literal-pathspecs",
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
    // A killed child (truncated output) keeps the complete records already captured: a missing
    // marker is safe, an invented one is not. Any other non-zero exit is a real failure.
    if !captured.truncated && captured.code != Some(0) {
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
    let ignored_paths = ignored_records(&captured.stdout);
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

/// Ignored paths from `git status --porcelain -z`. Only complete NUL-terminated records count: a
/// capture killed at the output ceiling can end in the middle of a name, and a partial name must
/// never become a directory that marks unrelated rows as ignored.
fn ignored_records(bytes: &[u8]) -> HashSet<String> {
    let mut records: Vec<&[u8]> = bytes.split(|byte| *byte == 0).collect();
    if !bytes.is_empty() && !bytes.ends_with(&[0]) {
        records.pop();
    }
    let mut ignored = HashSet::new();
    for record in records {
        if record.len() < 4 || !record.starts_with(b"!! ") {
            continue;
        }
        let path = String::from_utf8_lossy(&record[3..]);
        let path = path.trim_end_matches('/').replace('\\', "/");
        if !path.is_empty() {
            ignored.insert(path);
        }
    }
    ignored
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
        if fields.len() < 6 || fields[0].trim().is_empty() {
            continue;
        }
        let hash = fields[0].trim().to_string();
        // Git's own abbreviation, so the log row and the diff panel never disagree on it.
        let short_hash = fields[1].trim().to_string();
        let parents: Vec<String> = fields[4].split_whitespace().map(str::to_string).collect();
        let (files, files_truncated) = parse_name_status(body);
        commits.push(GitCommit {
            hash,
            short_hash,
            author: fields[2].to_string(),
            date: fields[3].to_string(),
            subject: fields[5].to_string(),
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
            "--format=%x1e%H%x1f%h%x1f%an%x1f%aI%x1f%P%x1f%s",
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
    let limit = if unified == "-U0" {
        HUNK_BYTES
    } else {
        PATCH_BYTES
    };
    if staged {
        // HEAD -> index, tracked or not: a path absent from the index (an untracked path or a
        // staged deletion) is still compared to HEAD, and the worktree is never read here.
        let mut args = vec![
            "--literal-pathspecs",
            "diff",
            "--no-color",
            "--no-ext-diff",
            unified,
            "--cached",
        ];
        args.push("--");
        args.push(path);
        spawn_bounded(&repo.top, &args, limit)
    } else if tracked {
        spawn_bounded(
            &repo.top,
            &[
                "--literal-pathspecs",
                "diff",
                "--no-color",
                "--no-ext-diff",
                unified,
                "--",
                path,
            ],
            limit,
        )
    } else {
        // An untracked file has no index entry to diff against; /dev/null shows its content.
        spawn_bounded(
            &repo.top,
            &[
                "--literal-pathspecs",
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
    let output = run(
        &repo.top,
        &[
            "--literal-pathspecs",
            "ls-files",
            "--error-unmatch",
            "--",
            path,
        ],
    )
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
        Err((state, message)) => return Ok(GitDiff::unavailable(state, message, path, staged)),
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
        Err(error) => {
            return Ok(GitDiff::unavailable(
                "failed",
                spawn_message(error),
                path,
                staged,
            ))
        }
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
    let resolved = spawn_bounded(
        &repo.top,
        &["rev-parse", "--verify", "--quiet", spec],
        4_096,
    )?;
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
        let (Some((old_start, old_count)), Some((new_start, new_count))) = (range(old), range(new))
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

fn commit_ref(repo: &Repo, rev: &str) -> Option<DiffCommit> {
    // A subject is user data: bounded, so a huge message cannot be collected whole.
    let output = spawn_bounded(
        &repo.top,
        &["log", "-1", "--format=%H\t%h\t%s", rev],
        OUTPUT_LIMIT,
    )
    .ok()?;
    if output.code != Some(0) || output.truncated {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let text = stdout.trim_end_matches('\n');
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

fn head_commit(repo: &Repo) -> Option<DiffCommit> {
    commit_ref(repo, "HEAD")
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

/// What both diff reads return: the two sides, the ranges that align them, the bounded patch as
/// fallback, and why the full file cannot be shown.
struct Assembled {
    state: &'static str,
    message: Option<String>,
    base: DiffSide,
    current: DiffSide,
    hunks: Vec<DiffHunk>,
    patch: String,
    truncated: bool,
    full: bool,
    full_reason: Option<&'static str>,
}

/// The tail shared by the worktree diff and the commit diff: only how the two sides were resolved
/// differs between the two reads, so everything after that lives here once.
fn assemble(
    path: &str,
    base_bytes: SideBytes,
    current_bytes: SideBytes,
    patch_output: Captured,
    hunk_output: Captured,
) -> Assembled {
    let mut hunks = Vec::new();
    if !hunk_output.truncated && (hunk_output.code == Some(0) || hunk_output.code == Some(1)) {
        hunks = parse_hunks(&String::from_utf8_lossy(&hunk_output.stdout));
    }
    let (base, base_reason) = read_side(base_bytes);
    let (current, current_reason) = read_side(current_bytes);
    let mut full_reason = base_reason.or(current_reason);
    let mut full =
        (!base.exists || base.text.is_some()) && (!current.exists || current.text.is_some());

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
        return Assembled {
            state,
            message: Some(message),
            base,
            current,
            hunks,
            patch: String::new(),
            truncated: false,
            full,
            full_reason,
        };
    }
    if full && text_differs && hunks.is_empty() {
        // Something changed in the text, but the block ranges that align the two texts are
        // missing: showing the full file would be a guess.
        full = false;
        full_reason = Some("unaligned");
    }
    let (patch, truncated) = cap(&text, DIFF_LIMIT);
    Assembled {
        state: "ok",
        message: None,
        base,
        current,
        hunks,
        patch,
        truncated,
        full,
        full_reason,
    }
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
        Err((state, message)) => return Ok(GitFileDiff::unavailable(state, message, path, staged)),
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
    let commit = head_commit(&repo);
    let assembled = assemble(path, base_bytes, current_bytes, patch_output, hunk_output);
    Ok(GitFileDiff {
        state: assembled.state,
        message: assembled.message,
        path: path.to_string(),
        staged,
        base: assembled.base,
        current: assembled.current,
        hunks: assembled.hunks,
        patch: assembled.patch,
        truncated: assembled.truncated,
        full: assembled.full,
        full_reason: assembled.full_reason,
        commit,
    })
}

fn validate_commit(hash: &str) -> HostResult<()> {
    // A hash is hex: anything else would reach `git rev-parse` as an option or a revision syntax.
    let valid = (4..=64).contains(&hash.len()) && hash.chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err(HostError::new(
            "invalid_state",
            format!("Not a commit hash: {hash}"),
        ));
    }
    Ok(())
}

/// The empty tree of this repository's hash algorithm, so a root commit has a base to diff against.
fn empty_tree(repo: &Repo) -> Result<String, SpawnError> {
    let output = spawn_bounded(&repo.top, &["hash-object", "-t", "tree", "--stdin"], 128)?;
    if output.code != Some(0) || output.truncated {
        return Err(SpawnError::Failed(
            "the empty tree could not be resolved".to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The two commits a commit diff compares. The pathspec names both sides of a rename: with only the
/// new path Git reports an add and a delete instead of the rename with its changes.
struct CommitRange {
    base: String,
    head: String,
    paths: Vec<String>,
}

fn commit_diff_output(
    repo: &Repo,
    range: &CommitRange,
    unified: &str,
) -> Result<Captured, SpawnError> {
    let limit = if unified == "-U0" {
        HUNK_BYTES
    } else {
        PATCH_BYTES
    };
    let mut args = vec![
        // A path is a name, not a pattern: `a[1].txt` must not also match `a1.txt`.
        "--literal-pathspecs",
        "diff",
        "--no-color",
        "--no-ext-diff",
        unified,
        range.base.as_str(),
        range.head.as_str(),
        "--",
    ];
    args.extend(range.paths.iter().map(String::as_str));
    spawn_bounded(&repo.top, &args, limit)
}

/// The read behind a commit's diff tab: both sides are blobs of the commit and its first parent.
pub fn commit_diff(
    root: &Path,
    hash: &str,
    path: &str,
    original_path: Option<&str>,
) -> HostResult<GitCommitDiff> {
    validate_commit(hash)?;
    validate_path(path)?;
    if let Some(original) = original_path {
        validate_path(original)?;
    }
    let repo = match open(root) {
        Ok(repo) => repo,
        Err(error) => {
            let (state, message) = open_data_error(error);
            return Ok(GitCommitDiff::unavailable(
                state,
                message,
                path,
                original_path,
            ));
        }
    };
    let spec = format!("{hash}^{{commit}}");
    let resolved = match run(&repo.top, &["rev-parse", "--verify", "--quiet", &spec]) {
        Ok(output) => output,
        Err(error) => {
            return Ok(GitCommitDiff::unavailable(
                spawn_state(&error),
                spawn_message(error),
                path,
                original_path,
            ))
        }
    };
    let full = resolved.stdout.trim().to_string();
    if resolved.code != Some(0) || full.is_empty() {
        // A hash the log showed but that no longer exists (amend, rebase) is not a diff.
        return Ok(GitCommitDiff::unavailable(
            "failed",
            format!("Commit not found: {hash}."),
            path,
            original_path,
        ));
    }
    let parent_spec = format!("{full}^");
    let parent_hash = match run(
        &repo.top,
        &["rev-parse", "--verify", "--quiet", &parent_spec],
    ) {
        Ok(output) if output.code == Some(0) => {
            let text = output.stdout.trim().to_string();
            if text.is_empty() {
                None
            } else {
                Some(text)
            }
        }
        // Exit 1 is a root commit: it has no parent, which is not an error.
        Ok(output) if output.code == Some(1) => None,
        Ok(output) => {
            return Ok(GitCommitDiff::unavailable(
                "failed",
                mask_urls(output.stderr.trim()),
                path,
                original_path,
            ))
        }
        Err(error) => {
            return Ok(GitCommitDiff::unavailable(
                spawn_state(&error),
                spawn_message(error),
                path,
                original_path,
            ))
        }
    };
    let base_path = original_path.unwrap_or(path);
    let base_bytes = match parent_hash.as_deref() {
        Some(parent) => match read_blob(&repo, &format!("{parent}:{base_path}")) {
            Ok(bytes) => bytes,
            Err(error) => {
                return Ok(GitCommitDiff::unavailable(
                    spawn_state(&error),
                    spawn_message(error),
                    path,
                    original_path,
                ))
            }
        },
        // A root commit has no parent to compare with: the empty tree has no entries, so the base
        // is absent and the panel says "No previous file", exactly like an added file.
        None => SideBytes::Missing,
    };
    let current_bytes = match read_blob(&repo, &format!("{full}:{path}")) {
        Ok(bytes) => bytes,
        Err(error) => {
            return Ok(GitCommitDiff::unavailable(
                spawn_state(&error),
                spawn_message(error),
                path,
                original_path,
            ))
        }
    };
    let base_rev = match parent_hash.as_deref() {
        Some(parent) => parent.to_string(),
        None => match empty_tree(&repo) {
            Ok(tree) => tree,
            Err(error) => {
                return Ok(GitCommitDiff::unavailable(
                    spawn_state(&error),
                    spawn_message(error),
                    path,
                    original_path,
                ))
            }
        },
    };
    let mut paths = vec![path.to_string()];
    if base_path != path {
        paths.push(base_path.to_string());
    }
    let range = CommitRange {
        base: base_rev,
        head: full.clone(),
        paths,
    };
    let patch_output = match commit_diff_output(&repo, &range, "-U3") {
        Ok(output) => output,
        Err(error) => {
            return Ok(GitCommitDiff::unavailable(
                spawn_state(&error),
                spawn_message(error),
                path,
                original_path,
            ))
        }
    };
    if !patch_output.truncated && patch_output.code != Some(0) && patch_output.code != Some(1) {
        return Ok(GitCommitDiff::unavailable(
            "failed",
            mask_urls(patch_output.stderr.trim()),
            path,
            original_path,
        ));
    }
    let hunk_output = match commit_diff_output(&repo, &range, "-U0") {
        Ok(output) => output,
        Err(error) => {
            return Ok(GitCommitDiff::unavailable(
                spawn_state(&error),
                spawn_message(error),
                path,
                original_path,
            ))
        }
    };
    let commit = commit_ref(&repo, &full);
    let parent = parent_hash
        .as_deref()
        .and_then(|parent| commit_ref(&repo, parent));
    let assembled = assemble(path, base_bytes, current_bytes, patch_output, hunk_output);
    Ok(GitCommitDiff {
        state: assembled.state,
        message: assembled.message,
        path: path.to_string(),
        original_path: original_path.map(str::to_string),
        commit,
        parent,
        base: assembled.base,
        current: assembled.current,
        hunks: assembled.hunks,
        patch: assembled.patch,
        truncated: assembled.truncated,
        full: assembled.full,
        full_reason: assembled.full_reason,
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
    let status =
        read_status(repo).unwrap_or_else(|message| GitStatus::unavailable("failed", message));
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
        stop: Stop::No,
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
    // Git refuses a partial commit during a merge (`cannot do a partial commit during a merge`),
    // so the app refuses first, in its own words and pointing at the way out (#114).
    if let Some(operation) = before.operation {
        return Err(HostError::new(
            "git_operation_in_progress",
            match operation {
                "merge" => "Commit refused: a merge is in progress. Finish or abort it from the branch menu; a partial commit is not possible during a merge.".to_string(),
                other => format!(
                    "Commit refused: a {other} is in progress. Finish or abort it in a terminal; a partial commit is not possible during a {other}."
                ),
            },
        ));
    }
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
        let output = run(&repo.top, &args)
            .map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
        if !output.ok() {
            return Err(HostError::new(
                "git_failed",
                format!(
                    "git add --intent-to-add failed: {}",
                    tail(&mask_urls(&output.stderr))
                ),
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
    run(&repo.top, &["rev-parse", "--verify", "--quiet", reference])
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
    let output = run(&repo.top, &args)
        .map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
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
    let output = run(&repo.top, &args)
        .map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
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
    let next =
        read_status(&repo).unwrap_or_else(|message| GitStatus::unavailable("failed", message));
    let message = match next.branch.as_deref() {
        Some(branch) => format!("Checked out {branch}."),
        None => "Checked out detached HEAD.".to_string(),
    };
    Ok(operation("ok", Some(message), &text, &repo, None))
}

/// Brings the remote's refs into the local copy: `git fetch --quiet --prune <remote>`, on the
/// remote the upstream tracks (the first configured remote when there is no upstream).
///
/// Fetch moves only refs, so a dirty tree, a merge in progress or a detached HEAD are not reasons
/// to refuse: the worktree, the index, the local branches and `MERGE_HEAD` are all left alone. The
/// deadline is the other half of the contract: a hung network command would hold the lock every
/// host call waits behind, so fetch, push and the fetch of an update are all bounded.
pub fn fetch(root: &Path, net: &Net, id: &str) -> HostResult<Operation> {
    fetch_with(root, FETCH_DEADLINE, net, id)
}

fn fetch_with(root: &Path, deadline: Duration, net: &Net, id: &str) -> HostResult<Operation> {
    let repo = require_repo(root)?;
    let status = read_status(&repo).map_err(|message| HostError::new("git_failed", message))?;
    let Some(remote) = fetch_remote(&repo, &status) else {
        return Err(HostError::new(
            "git_no_remote",
            "No remote is configured. Add one with git remote add; nothing was fetched.",
        ));
    };

    // The refspec is explicit on purpose. Without it the fetch uses `remote.<name>.fetch`, and
    // `--prune` deletes whatever those refspecs point at — a repository that maps the remote into
    // `refs/heads/*` or `refs/mirror/*` would lose local refs to a button that claims to touch only
    // remote-tracking refs. Naming the destination makes the blast radius exactly
    // `refs/remotes/<remote>/*`. Tags still follow the fetched commits, as with a plain fetch.
    let refspec = format!("+refs/heads/*:refs/remotes/{remote}/*");
    let fetched = run_with_deadline(
        &repo.top,
        // `--refmap=` (empty) drops `remote.<name>.fetch` from the ref mapping entirely. Without
        // it Git still applies the configured refspec as an extra refmap, so a repository that
        // mirrors the remote into `refs/mirror/*` — or into `refs/heads/*` — would have those refs
        // created or updated by a button that promises to touch only remote-tracking refs.
        &[
            "fetch",
            "--quiet",
            "--prune",
            "--refmap=",
            &remote,
            &refspec,
        ],
        deadline,
        net,
        id,
    )
    .map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
    let (output, stop) = fetched;
    let text = format!("{}{}", output.stdout, output.stderr);
    let mut next = operation("ok", None, &text, &repo, None);
    next.remote = Some(remote.clone());
    next.stop = stop;
    if stop != Stop::No {
        next.state = "failed";
        // A kill can land after some refs were already updated, so this cannot promise that
        // nothing changed. It promises what a fetch can never change: the branch, the index and
        // the working tree.
        next.message = Some(match stop {
            Stop::Cancelled => "Fetch cancelled: your branch, index and working tree were not touched; refs already received stay.".to_string(),
            _ => format!(
                "Fetch stopped: {remote} did not answer within {} seconds. Your branch, index and working tree were not touched; refs already received stay.",
                deadline.as_secs()
            ),
        });
        return Ok(next);
    }
    if !output.ok() {
        next.state = "failed";
        let stderr = mask_urls(output.stderr.trim());
        next.message = Some(fetch_failure(&remote, &stderr));
        return Ok(next);
    }
    next.message = Some(format!(
        "Fetched {remote}: {}.",
        tracking_line(&next.status)
    ));
    Ok(next)
}

/// Which remote the fetch talks to: the one the current branch is configured to track
/// (`branch.<name>.remote`), else the first configured remote. The config is the source, not the
/// upstream short name: a remote name may contain `/`, and a branch can track a local branch
/// (`.`), which is not a remote at all.
fn fetch_remote(repo: &Repo, status: &GitStatus) -> Option<String> {
    let configured = remotes(repo);
    let tracked = status.branch.as_ref().and_then(|branch| {
        let key = format!("branch.{branch}.remote");
        let output = run(&repo.top, &["config", "--get", &key]).ok()?;
        if !output.ok() {
            return None;
        }
        let value = output.stdout.trim().to_string();
        if value.is_empty() || value == "." {
            return None;
        }
        Some(value)
    });
    tracked
        .filter(|remote| configured.iter().any(|name| name == remote))
        .or_else(|| configured.first().cloned())
}

/// The refreshed tracking state in one clause: what the fetch just changed and what the counters
/// now read. The message never claims the counter is fresh — it says what this fetch saw.
fn tracking_line(status: &GitStatus) -> String {
    let branch = status.branch.clone().unwrap_or_else(|| "HEAD".to_string());
    match status.upstream.as_deref() {
        Some(upstream) => format!(
            "{branch} is {} ahead, {} behind {upstream}",
            status.ahead, status.behind
        ),
        None => format!(
            "{branch} is {} ahead, {} behind",
            status.ahead, status.behind
        ),
    }
}

/// Every failed fetch either names a class the app knows or keeps Git's own tail. The credential
/// class is the one the app must own: the host runs without a terminal and with interactive
/// prompting disabled, so Git's advice ("terminal prompts disabled") describes a situation the
/// user cannot see from inside the app.
fn fetch_failure(remote: &str, stderr: &str) -> String {
    let lower = stderr.to_lowercase();
    if lower.contains("could not read username")
        || lower.contains("could not read password")
        || lower.contains("terminal prompts disabled")
        || lower.contains("authentication failed")
        || lower.contains("permission denied (publickey)")
        || lower.contains("invalid username or password")
    {
        return format!(
            "Fetch needs credentials for {remote}. OpenRidr cannot ask for them: the credential helper has to answer, or fetch from a terminal. Nothing was changed."
        );
    }
    if stderr.is_empty() {
        "git fetch failed without output.".to_string()
    } else {
        tail(stderr)
    }
}

/// The update policy the dialog states: fetch, then merge. `--ff` is passed explicitly so the
/// repository's `pull.rebase`/`pull.ff`/`merge.ff` cannot change what the dialog promised, and
/// `--no-edit` keeps git's merge message without an editor. Never `--no-ff` (a fast-forward stays
/// one) and never a rebase: it rewrites local commits and leaves a rebase the app cannot finish or
/// abort (it only finishes or aborts a merge).
///
/// Fetch and merge are separate commands on purpose: the fetch moves only refs, so a timeout or a
/// network failure stops before the merge runs, and the merge itself is local and unbounded.
const MERGE_FLAGS: [&str; 2] = ["--ff", "--no-edit"];
const CONFLICT_PATH_LIMIT: usize = 20;

pub fn pull(root: &Path, net: &Net, id: &str) -> HostResult<Operation> {
    pull_with(root, FETCH_DEADLINE, net, id)
}

/// The deadline covers the fetch only. The merge is local, so it cannot wait on a remote, and
/// killing it mid-way would leave a half-applied merge — a worse state than the freeze the bound
/// prevents. Because the fetch comes first, an update that runs out of time never starts a merge.
fn pull_with(root: &Path, deadline: Duration, net: &Net, id: &str) -> HostResult<Operation> {
    let repo = require_repo(root)?;
    let status = read_status(&repo).map_err(|message| HostError::new("git_failed", message))?;
    require_clean(&status, "Update")?;
    if status.upstream.is_none() {
        return Err(HostError::new(
            "git_no_upstream",
            "No upstream is configured; nothing to pull.",
        ));
    }

    let (fetched, stop) = run_with_deadline(&repo.top, &["fetch", "--quiet"], deadline, net, id)
        .map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
    let mut text = format!("{}{}", fetched.stdout, fetched.stderr);
    if stop != Stop::No {
        let remote = fetch_remote(&repo, &status).unwrap_or_else(|| "the remote".to_string());
        // Nothing was merged: the merge only runs after a fetch that succeeded. Refs already
        // received stay, because a kill can land after Git updated some of them.
        let message = match stop {
            Stop::Cancelled => "Update cancelled: nothing was merged; your branch, index and working tree were not touched, and refs already received stay.".to_string(),
            _ => format!(
                "Update stopped: {remote} did not answer within {} seconds. Nothing was merged; your branch, index and working tree were not touched, and refs already received stay.",
                deadline.as_secs()
            ),
        };
        let mut next = operation("failed", Some(message), &text, &repo, None);
        next.stop = stop;
        return Ok(next);
    }
    if !fetched.ok() {
        let stderr = mask_urls(fetched.stderr.trim());
        return Ok(operation(
            "failed",
            Some(update_failure(&repo, &stderr)),
            &text,
            &repo,
            None,
        ));
    }

    // The merge is local and not cancellable: the slot was released when the fetch child ended, so a
    // cancel from here on reports that there is nothing to stop instead of promising a kill that
    // will never come. Killing a merge mid-way would leave a half-applied merge (#136 decision).
    let resolved = run(&repo.top, &["rev-parse", "@{upstream}"])
        .map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
    if !resolved.ok() || resolved.stdout.trim().is_empty() {
        text.push_str(&resolved.stdout);
        text.push_str(&resolved.stderr);
        let stderr = mask_urls(resolved.stderr.trim());
        return Ok(operation(
            "failed",
            Some(update_failure(&repo, &stderr)),
            &text,
            &repo,
            None,
        ));
    }

    let mut args = vec!["merge"];
    args.extend(MERGE_FLAGS);
    args.push("@{upstream}");
    let merged = run(&repo.top, &args)
        .map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
    text.push_str(&merged.stdout);
    text.push_str(&merged.stderr);
    if !merged.ok() {
        let stderr = mask_urls(merged.stderr.trim());
        return Ok(operation(
            "failed",
            Some(update_failure(&repo, &stderr)),
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

/// Every failed update ends in the app's own words. A merge that conflicts is **left in progress**:
/// the branch menu finishes or aborts it (#114), so the update never undoes the user's chance to
/// resolve it. Git's advice about configuration or shell commands is never dumped; the raw output
/// stays in `Operation.output` for anything the app does not know.
fn update_failure(repo: &Repo, stderr: &str) -> String {
    let status = read_status(repo).ok();
    if let Some(status) = status.as_ref() {
        if let Some(operation) = status.operation {
            if operation == "merge" {
                return format!(
                    "Update stopped: {}. The merge is still in progress: resolve the conflicted files in the editor, then finish or abort it from the branch menu.",
                    conflict_subject(status)
                );
            }
            return format!(
                "Update stopped: a {operation} is in progress. Finish or abort it in a terminal before updating again."
            );
        }
    }
    if missing_identity(stderr) {
        return "Update needs a Git identity for this repository: set user.name and user.email, then try again; a merge commit has no author without it. Nothing was changed.".to_string();
    }
    let lower = stderr.to_lowercase();
    if lower.contains("not possible to fast-forward") || lower.contains("non-fast-forward") {
        return "Update stopped: git refused a non-fast-forward update. OpenRidr reconciles a diverged branch with a merge commit. Nothing was changed.".to_string();
    }
    if stderr.is_empty() {
        "git pull failed without output.".to_string()
    } else {
        tail(stderr)
    }
}

/// The conflicted paths in one clause: how many, and up to `CONFLICT_PATH_LIMIT` of them.
fn conflict_subject(status: &GitStatus) -> String {
    let mut paths: Vec<String> = status
        .files
        .iter()
        .filter(|file| file.conflicted)
        .map(|file| file.path.clone())
        .collect();
    paths.sort_unstable();
    paths.dedup();
    match paths.len() {
        0 => "the merge conflicted".to_string(),
        n => format!(
            "{n} file{} changed on both sides ({})",
            if n == 1 { "" } else { "s" },
            list_paths(&paths)
        ),
    }
}

/// Aborts the merge in progress: `git merge --abort` puts the branch, index and working tree back
/// where they were before the merge. Only a merge is offered here; a rebase, cherry-pick or revert
/// in progress is refused and left to a terminal (#114 scope).
pub fn merge_abort(root: &Path) -> HostResult<Operation> {
    let repo = require_repo(root)?;
    let status = read_status(&repo).map_err(|message| HostError::new("git_failed", message))?;
    match status.operation {
        Some("merge") => {}
        Some(operation) => {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "Abort refused: a {operation} is in progress, and OpenRidr only aborts a merge. Finish or abort it in a terminal."
                ),
            ))
        }
        None => {
            return Err(HostError::new(
                "invalid_state",
                "Abort refused: no merge is in progress; nothing was aborted.",
            ))
        }
    }
    let output = run(&repo.top, &["merge", "--abort"])
        .map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
    let text = format!("{}{}", output.stdout, output.stderr);
    if !output.ok() {
        let stderr = mask_urls(output.stderr.trim());
        return Ok(operation(
            "failed",
            Some(if stderr.is_empty() {
                "git merge --abort failed without output; the merge is still in progress."
                    .to_string()
            } else {
                format!(
                    "Abort failed; the merge is still in progress. {}",
                    tail(&stderr)
                )
            }),
            &text,
            &repo,
            None,
        ));
    }
    Ok(operation(
        "ok",
        Some("Merge aborted; the merge is undone and your commits stay.".to_string()),
        &text,
        &repo,
        None,
    ))
}

/// How much of `git diff --check` output is kept: one line per problem, and the marker lines are
/// what the guard reads. A cut-off capture refuses the call instead of guessing: the marker could
/// be past the ceiling. One MiB is far more than a real resolution produces, so the refusal is a
/// safety net, not a routine path.
const CHECK_BYTES: usize = 1024 * 1024;

/// Finishes the merge in progress: stages the paths that were conflicted (the user resolved them in
/// the editor) and commits with the message Git already prepared (`--no-edit`). A path that still
/// holds a conflict marker refuses the whole call, so a click cannot commit `<<<<<<<`.
pub fn merge_continue(root: &Path) -> HostResult<Operation> {
    merge_continue_with(root, CHECK_BYTES)
}

fn merge_continue_with(root: &Path, check_limit: usize) -> HostResult<Operation> {
    let repo = require_repo(root)?;
    let status = read_status(&repo).map_err(|message| HostError::new("git_failed", message))?;
    match status.operation {
        Some("merge") => {}
        Some(operation) => {
            return Err(HostError::new(
                "invalid_state",
                format!(
                    "Complete merge refused: a {operation} is in progress, and OpenRidr only completes a merge. Finish or abort it in a terminal."
                ),
            ))
        }
        None => {
            return Err(HostError::new(
                "invalid_state",
                "Complete merge refused: no merge is in progress; nothing was committed.",
            ))
        }
    }
    let markers = marker_paths(&repo, check_limit)?;
    if !markers.is_empty() {
        return Err(HostError::new("invalid_state", marker_message(&markers)));
    }
    let conflicted: Vec<String> = status
        .files
        .iter()
        .filter(|file| file.conflicted)
        .map(|file| file.path.clone())
        .collect();
    let mut text = String::new();
    if !conflicted.is_empty() {
        // `-u` also stages a resolution that deleted the file, and the pathspec keeps unrelated
        // edits out of the merge commit. Nothing is staged when the list is empty: a bare
        // `git add -u` would sweep the whole tree into the merge. `--literal-pathspecs` keeps a
        // name like `a*.txt` from matching `a-other.txt`.
        let mut args = vec!["--literal-pathspecs", "add", "-u", "--"];
        args.extend(conflicted.iter().map(String::as_str));
        let added = run(&repo.top, &args)
            .map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
        text.push_str(&added.stdout);
        text.push_str(&added.stderr);
        if !added.ok() {
            let stderr = mask_urls(added.stderr.trim());
            return Err(HostError::new(
                "git_failed",
                if stderr.is_empty() {
                    "git add failed without output.".to_string()
                } else {
                    tail(&stderr)
                },
            ));
        }
    }
    let unmerged = run(&repo.top, &["ls-files", "-u"])
        .map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
    text.push_str(&unmerged.stdout);
    text.push_str(&unmerged.stderr);
    let still = unmerged_paths(&unmerged.stdout);
    if !unmerged.ok() || !still.is_empty() {
        return Err(HostError::new(
            "invalid_state",
            if still.is_empty() {
                "Complete merge refused: git could not list the unmerged paths.".to_string()
            } else {
                format!(
                    "Complete merge refused: {} still unmerged ({}). Resolve them in the editor first.",
                    still.len(),
                    list_paths(&still)
                )
            },
        ));
    }
    let committed = run(&repo.top, &["commit", "--no-edit"])
        .map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
    text.push_str(&committed.stdout);
    text.push_str(&committed.stderr);
    if !committed.ok() {
        let stderr = mask_urls(committed.stderr.trim());
        let message = if missing_identity(&stderr) {
            "Complete merge needs a Git identity for this repository: set user.name and user.email, then try again; a merge commit has no author without it. The merge is still in progress.".to_string()
        } else if stderr.is_empty() {
            "git commit failed without output; the merge is still in progress.".to_string()
        } else {
            format!(
                "Complete merge failed; the merge is still in progress. {}",
                tail(&stderr)
            )
        };
        return Ok(operation("failed", Some(message), &text, &repo, None));
    }
    Ok(operation(
        "ok",
        Some("Merge completed.".to_string()),
        &text,
        &repo,
        None,
    ))
}

/// Paths with a leftover conflict marker in the content the merge commit would carry. Both the
/// worktree (`git diff --check HEAD`) and the index (`--cached HEAD`) are checked: the commit uses
/// the index, so markers staged and then removed from the worktree are invisible to the first
/// check, and unmerged paths are invisible to the second. A cut-off capture refuses the call
/// instead of proving nothing.
fn marker_paths(repo: &Repo, limit: usize) -> HostResult<Vec<String>> {
    let mut paths: Vec<String> = Vec::new();
    for args in [
        &["diff", "--check", "--no-ext-diff", "HEAD"][..],
        &["diff", "--check", "--no-ext-diff", "--cached", "HEAD"][..],
    ] {
        let checked = spawn_bounded(&repo.top, args, limit)
            .map_err(|error| HostError::new("git_failed", spawn_message(error)))?
            .text();
        // A cut-off capture is checked first: the kill that bounds it also changes the exit code.
        if checked.truncated {
            return Err(HostError::new(
                "invalid_state",
                "Complete merge refused: the conflict-marker check was cut off before the end of the diff, so OpenRidr cannot prove there are no markers left. Commit the merge from a terminal.",
            ));
        }
        if checked.code != Some(0) && checked.code != Some(2) {
            let stderr = mask_urls(checked.stderr.trim());
            return Err(HostError::new(
                "git_failed",
                if stderr.is_empty() {
                    "git diff --check failed without output.".to_string()
                } else {
                    tail(&stderr)
                },
            ));
        }
        paths.extend(conflict_marker_paths(&checked.stdout));
    }
    paths.sort_unstable();
    paths.dedup();
    Ok(paths)
}

/// Paths with a leftover conflict marker in a `git diff --check` report (`<path>:<line>: leftover
/// conflict marker`). The path is split from the right, so a name containing `:` survives.
fn conflict_marker_paths(text: &str) -> Vec<String> {
    let mut paths: Vec<String> = text
        .lines()
        .filter_map(|line| line.strip_suffix(": leftover conflict marker"))
        .filter_map(|head| head.rsplit_once(':'))
        .map(|(path, _)| path.to_string())
        .collect();
    paths.sort_unstable();
    paths.dedup();
    paths
}

/// Paths from `git ls-files -u` (`<mode> <object> <stage>\t<path>`), deduplicated.
fn unmerged_paths(text: &str) -> Vec<String> {
    let mut paths: Vec<String> = text
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(_, path)| path.to_string())
        .collect();
    paths.sort_unstable();
    paths.dedup();
    paths
}

/// Up to `CONFLICT_PATH_LIMIT` paths, comma separated, with `, …` when more were found.
fn list_paths(paths: &[String]) -> String {
    let head: Vec<&str> = paths
        .iter()
        .take(CONFLICT_PATH_LIMIT)
        .map(String::as_str)
        .collect();
    if paths.len() > CONFLICT_PATH_LIMIT {
        format!("{}, …", head.join(", "))
    } else {
        head.join(", ")
    }
}

fn marker_message(paths: &[String]) -> String {
    match paths.len() {
        1 => format!(
            "Complete merge refused: 1 file still contains conflict markers ({}). Resolve it in the editor first.",
            paths[0]
        ),
        n => format!(
            "Complete merge refused: {n} files still contain conflict markers ({}). Resolve them in the editor first.",
            list_paths(paths)
        ),
    }
}

/// Git's own words for "there is no author to write the commit with". The app says where to set it
/// instead of dumping the hint.
fn missing_identity(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    lower.contains("identity unknown")
        || lower.contains("please tell me who you are")
        || lower.contains("unable to auto-detect email address")
        || lower.contains("empty ident")
}

fn remotes(repo: &Repo) -> Vec<String> {
    run(&repo.top, &["remote"])
        .map(|output| {
            output
                .stdout
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn first_remote(repo: &Repo) -> Option<String> {
    remotes(repo).into_iter().next()
}

/// Pushes the current branch. Bounded like every network command: a killed push may still have
/// reached the remote, so the message says what to check instead of claiming the commit is there.
pub fn push(root: &Path, params: &Value, net: &Net, id: &str) -> HostResult<Operation> {
    push_with(root, params, PUSH_DEADLINE, net, id)
}

fn push_with(
    root: &Path,
    params: &Value,
    deadline: Duration,
    net: &Net,
    id: &str,
) -> HostResult<Operation> {
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
    let upstream_remote = status.upstream.as_ref().and_then(|upstream| {
        upstream
            .split_once('/')
            .map(|(remote, _)| remote.to_string())
    });
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
        args.extend(["--set-upstream".to_string(), remote.clone(), branch.clone()]);
    }
    let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
    let (output, stop) = run_with_deadline(&repo.top, &borrowed, deadline, net, id)
        .map_err(|error| HostError::new("git_failed", spawn_message(error)))?;
    let text = format!("{}{}", output.stdout, output.stderr);
    let mut next = operation(
        if stop == Stop::No && output.ok() {
            "ok"
        } else {
            "failed"
        },
        None,
        &text,
        &repo,
        None,
    );
    next.stop = stop;
    if stop == Stop::Cancelled {
        // The same promise as the deadline: the kill can land after the remote accepted the refs,
        // and a `pre-push` hook may already have run, so neither the outcome nor the local state is
        // claimed. The remote is not named here either (see the deadline message).
        next.message = Some(
            "Push cancelled: the push may already have reached the remote. Check the branch there before pushing again."
                .to_string(),
        );
    } else if stop == Stop::Deadline {
        // The message promises neither the outcome nor the local state: the kill can land after the
        // remote accepted the refs, and a `pre-push` hook may already have run. It also does not
        // name a remote: `git push` with no repository argument resolves the destination from
        // `branch.<name>.pushRemote`/`remote.pushDefault`, which the app does not read.
        next.message = Some(format!(
            "Push stopped: the remote did not answer within {} seconds. The push may already have reached it: check the branch there before pushing again.",
            deadline.as_secs()
        ));
    } else if output.ok() {
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
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;

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

    /// Some fixtures make a git object or the index read-only on purpose. Restoring write access is
    /// platform-specific: on Unix, `set_readonly(false)` sets *every* write bit (`0o222`), which would
    /// also make group and others able to write, so the owner bit is added to the mode instead. Clippy
    /// rejects the `set_readonly(false)` form for exactly that reason (`permissions_set_readonly_false`).
    fn make_writable(path: &Path) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(path).unwrap().permissions();
            permissions.set_mode(permissions.mode() | 0o600);
            fs::set_permissions(path, permissions).unwrap();
        }
        #[cfg(not(unix))]
        {
            let mut permissions = fs::metadata(path).unwrap().permissions();
            permissions.set_readonly(false);
            fs::set_permissions(path, permissions).unwrap();
        }
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
        assert!(current
            .files
            .iter()
            .any(|file| file.path == "dirty.txt" && file.untracked && file.label == "Untracked"));

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
            &[
                spaced.to_string(),
                newline.to_string(),
                "missing.log".to_string(),
            ],
        );
        assert_eq!(answer.state, "ok");
        assert_eq!(
            answer.ignored,
            vec![spaced.to_string(), newline.to_string()]
        );
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
    fn ignored_treats_pathspecs_as_literal_names() {
        let (_temp, root) = init("repo");
        fs::write(root.join(".gitignore"), "*.log\n").unwrap();
        // Pathspec magic and wildcards would match something else (or nothing) without the
        // literal flag; the tree sends real names, so the answer must be about these files.
        let names = ["a[1].log", "q?.log", "star*.log", ":magic.log"];
        for name in names {
            fs::write(root.join(name), "x\n").unwrap();
        }
        git(&root, &["add", ".gitignore"]);
        git(&root, &["commit", "-qm", "initial"]);
        let answer = ignored(&root, &names.map(str::to_string));
        assert_eq!(answer.state, "ok", "{:?}", answer.message);
        assert_eq!(answer.ignored, names.map(str::to_string));
    }

    #[test]
    fn ignored_records_ignore_an_incomplete_trailing_name() {
        let records = ignored_records(b"!! build/\0!! partial");
        assert!(records.contains("build"));
        assert!(
            !records.contains("partial"),
            "a truncated name is never a record"
        );
        assert_eq!(records.len(), 1);
        assert!(ignored_records(b"").is_empty());
        assert!(ignored_records(b"\0").is_empty());
    }

    #[test]
    fn ignored_answers_change_when_the_rules_change_for_the_same_paths() {
        let (_temp, root) = init("repo");
        fs::write(root.join(".gitignore"), "\n").unwrap();
        fs::write(root.join("notes.log"), "x\n").unwrap();
        git(&root, &["add", ".gitignore"]);
        git(&root, &["commit", "-qm", "initial"]);
        let paths = ["notes.log".to_string()];
        assert!(ignored(&root, &paths).ignored.is_empty());
        // A checkout/commit can change the rules without changing the list of visible paths: the
        // read is not cached, so the same request answers differently.
        fs::write(root.join(".gitignore"), "*.log\n").unwrap();
        assert_eq!(ignored(&root, &paths).ignored, paths);
    }

    #[test]
    fn ignored_reports_a_failing_git_as_an_error() {
        let (_temp, root) = init("repo");
        fs::write(root.join(".gitignore"), "*.log\n").unwrap();
        fs::write(root.join("a.log"), "x\n").unwrap();
        git(&root, &["add", ".gitignore"]);
        git(&root, &["commit", "-qm", "initial"]);
        // A corrupt index makes `git status` fail; the read must say so instead of answering
        // "nothing is ignored".
        fs::write(root.join(".git/index"), b"not an index").unwrap();
        let answer = ignored(&root, &["a.log".to_string()]);
        assert_eq!(answer.state, "failed");
        assert!(!answer.message.as_deref().unwrap_or_default().is_empty());
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
        // Git's abbreviation, not a seven-character slice: the log row and the diff panel agree.
        assert_eq!(
            log.commits[0].short_hash,
            git(&root, &["rev-parse", "--short", "HEAD"]).trim()
        );
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
        let short = git(&root, &["rev-parse", "--short", "HEAD"])
            .trim()
            .to_string();

        let answer = file_diff(&root, "a.txt", false).unwrap();
        assert_eq!(answer.state, "ok");
        assert!(answer.full, "{:?}", answer.full_reason);
        assert_eq!(answer.base.text.as_deref(), Some("one\ntwo\nthree\nfour\n"));
        assert_eq!(
            answer.current.text.as_deref(),
            Some("one\nTWO\nthree\nfour\n")
        );
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
    fn commit_diff_returns_both_blobs_hunks_and_the_two_commits() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\ntwo\nthree\nfour\n").unwrap();
        commit_all(&root, "initial");
        fs::write(root.join("a.txt"), "one\nTWO\nthree\nfour\n").unwrap();
        commit_all(&root, "second");
        let head = git(&root, &["rev-parse", "HEAD"]).trim().to_string();
        let parent_hash = git(&root, &["rev-parse", "HEAD^"]).trim().to_string();
        // A commit's diff is the commit's blobs: an uncommitted edit must not change it.
        fs::write(root.join("a.txt"), "worktree only\n").unwrap();

        let answer = commit_diff(&root, &head, "a.txt", None).unwrap();
        assert_eq!(answer.state, "ok");
        assert!(answer.full, "{:?}", answer.full_reason);
        assert_eq!(answer.base.text.as_deref(), Some("one\ntwo\nthree\nfour\n"));
        assert_eq!(
            answer.current.text.as_deref(),
            Some("one\nTWO\nthree\nfour\n")
        );
        assert_eq!(
            answer.hunks,
            vec![DiffHunk {
                old_start: 2,
                old_count: 1,
                new_start: 2,
                new_count: 1
            }]
        );
        let commit = answer.commit.expect("the commit is reported");
        assert_eq!(commit.hash, head);
        assert_eq!(
            commit.short_hash,
            git(&root, &["rev-parse", "--short", "HEAD"]).trim()
        );
        assert_eq!(commit.subject, "second");
        let parent = answer.parent.expect("the first parent is reported");
        assert_eq!(parent.hash, parent_hash);
        assert_eq!(parent.subject, "initial");
        assert!(answer.patch.contains("+TWO"));
    }

    #[test]
    fn commit_diff_marks_added_and_deleted_files_of_the_commit() {
        let (_temp, root) = init("repo");
        fs::write(root.join("gone.txt"), "a\nb\n").unwrap();
        commit_all(&root, "initial");
        fs::remove_file(root.join("gone.txt")).unwrap();
        fs::write(root.join("new.txt"), "brand new\n").unwrap();
        commit_all(&root, "remove and add");
        let head = git(&root, &["rev-parse", "HEAD"]).trim().to_string();

        let added = commit_diff(&root, &head, "new.txt", None).unwrap();
        assert_eq!(added.state, "ok");
        assert!(!added.base.exists);
        assert_eq!(added.base.text, None);
        assert_eq!(added.current.text.as_deref(), Some("brand new\n"));
        assert!(added.full);
        assert_eq!(added.hunks.len(), 1);

        let deleted = commit_diff(&root, &head, "gone.txt", None).unwrap();
        assert!(!deleted.current.exists);
        assert_eq!(deleted.current.text, None);
        assert_eq!(deleted.base.text.as_deref(), Some("a\nb\n"));
        assert!(deleted.full);
    }

    #[test]
    fn commit_diff_uses_the_original_path_as_the_base_of_a_rename() {
        let (_temp, root) = init("repo");
        let original: String = (1..=20).map(|n| format!("line {n}\n")).collect();
        fs::write(root.join("old.txt"), &original).unwrap();
        commit_all(&root, "initial");
        git(&root, &["mv", "old.txt", "new.txt"]);
        let edited = original.replace("line 10\n", "line TEN\n");
        fs::write(root.join("new.txt"), &edited).unwrap();
        commit_all(&root, "rename and edit");
        let head = git(&root, &["rev-parse", "HEAD"]).trim().to_string();
        // The log reports the rename source; the base comes from it, not from the new path.
        assert!(git(&root, &["show", "--name-status", "--format=", "HEAD"]).contains("R0"));

        let answer = commit_diff(&root, &head, "new.txt", Some("old.txt")).unwrap();
        assert_eq!(answer.original_path.as_deref(), Some("old.txt"));
        assert_eq!(answer.base.text.as_deref(), Some(original.as_str()));
        assert_eq!(answer.current.text.as_deref(), Some(edited.as_str()));
        assert_eq!(
            answer.hunks,
            vec![DiffHunk {
                old_start: 10,
                old_count: 1,
                new_start: 10,
                new_count: 1
            }]
        );

        // Without the rename source the base is the new path in the parent, which does not exist.
        let without = commit_diff(&root, &head, "new.txt", None).unwrap();
        assert!(!without.base.exists);
        assert_eq!(without.current.text.as_deref(), Some(edited.as_str()));
    }

    #[test]
    fn paths_are_literal_names_not_patterns() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a[1].txt"), "one\n").unwrap();
        fs::write(root.join("a1.txt"), "other\n").unwrap();
        commit_all(&root, "initial");
        fs::write(root.join("a[1].txt"), "two\n").unwrap();
        fs::write(root.join("a1.txt"), "other2\n").unwrap();
        commit_all(&root, "both");
        let head = git(&root, &["rev-parse", "HEAD"]).trim().to_string();

        // `a[1].txt` is a character class to Git's pathspec parser: it must not also match `a1.txt`.
        let commit = commit_diff(&root, &head, "a[1].txt", None).unwrap();
        assert_eq!(commit.hunks.len(), 1, "{}", commit.patch);
        assert!(!commit.patch.contains("a1.txt"), "{}", commit.patch);

        fs::write(root.join("a[1].txt"), "three\n").unwrap();
        fs::write(root.join("a1.txt"), "other3\n").unwrap();
        let worktree = file_diff(&root, "a[1].txt", false).unwrap();
        assert_eq!(worktree.hunks.len(), 1, "{}", worktree.patch);
        assert!(!worktree.patch.contains("a1.txt"), "{}", worktree.patch);

        // A literal name whose pattern matches nothing must still be tracked, not "untracked".
        fs::write(root.join("x[1].txt"), "only\n").unwrap();
        git(&root, &["add", "x[1].txt"]);
        git(&root, &["commit", "-qm", "literal only"]);
        fs::write(root.join("x[1].txt"), "only2\n").unwrap();
        let literal = file_diff(&root, "x[1].txt", false).unwrap();
        assert!(literal.base.exists, "a literal name is not untracked");
        assert_eq!(literal.base.text.as_deref(), Some("only\n"));
        assert_eq!(literal.current.text.as_deref(), Some("only2\n"));
    }

    #[test]
    fn log_keeps_a_tab_in_the_subject_and_git_folds_the_rest_into_it() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        git(&root, &["add", "."]);
        git(&root, &["commit", "-qm", "subject with\ttab\nsecond line"]);
        let log = log(&root);
        // Git folds the message's line breaks into the subject; the tab must survive the split.
        assert_eq!(log.commits[0].subject, "subject with\ttab second line");
        assert_eq!(
            log.commits[0].short_hash,
            git(&root, &["rev-parse", "--short", "HEAD"]).trim()
        );
    }

    #[test]
    fn commit_diff_of_a_root_commit_has_no_parent_and_no_base() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\ntwo\n").unwrap();
        commit_all(&root, "root");
        let head = git(&root, &["rev-parse", "HEAD"]).trim().to_string();

        let answer = commit_diff(&root, &head, "a.txt", None).unwrap();
        assert_eq!(answer.state, "ok");
        assert!(answer.parent.is_none());
        assert!(!answer.base.exists);
        assert_eq!(answer.current.text.as_deref(), Some("one\ntwo\n"));
        assert!(answer.full);
        assert_eq!(answer.hunks.len(), 1);
        assert!(answer.patch.contains("+one"));
    }

    #[test]
    fn commit_diff_falls_back_for_binary_and_rejects_unknown_commits() {
        let (_temp, root) = init("repo");
        fs::write(root.join("bin.dat"), b"ab\0cd").unwrap();
        commit_all(&root, "binary");
        let head = git(&root, &["rev-parse", "HEAD"]).trim().to_string();

        let binary = commit_diff(&root, &head, "bin.dat", None).unwrap();
        assert_eq!(binary.state, "ok");
        assert!(!binary.full);
        assert_eq!(binary.full_reason, Some("binary"));
        assert!(!binary.patch.is_empty());

        // The blob is refused by size before any content is read, like the worktree read.
        fs::write(root.join("big.txt"), "x".repeat(FULL_FILE_BYTES + 1)).unwrap();
        commit_all(&root, "big");
        let big_head = git(&root, &["rev-parse", "HEAD"]).trim().to_string();
        let oversized = commit_diff(&root, &big_head, "big.txt", None).unwrap();
        assert_eq!(oversized.state, "ok");
        assert!(!oversized.full);
        assert_eq!(oversized.full_reason, Some("too_large"));

        let unknown = commit_diff(
            &root,
            "0123456789abcdef0123456789abcdef01234567",
            "bin.dat",
            None,
        )
        .unwrap();
        assert_eq!(unknown.state, "failed");
        assert!(unknown.message.unwrap().contains("not found"));

        let shape = commit_diff(&root, "--help", "bin.dat", None).unwrap_err();
        assert_eq!(shape.code, "invalid_state");
        let outside = commit_diff(&root, &head, "../escape.txt", None).unwrap_err();
        assert_eq!(outside.code, "invalid_state");
        let outside_original =
            commit_diff(&root, &head, "bin.dat", Some("../escape.txt")).unwrap_err();
        assert_eq!(outside_original.code, "invalid_state");
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
        make_writable(&object);
        fs::write(&object, b"corrupt").unwrap();

        // The path still resolves through the tree; only the content read fails.
        assert!(matches!(
            read_blob(&repo_of(&root), "HEAD:a.txt"),
            Ok(SideBytes::Unreadable)
        ));
        let answer = file_diff(&root, "a.txt", false).unwrap();
        assert_eq!(answer.state, "failed", "{:?}", answer.message);
        assert!(answer
            .message
            .unwrap_or_default()
            .contains("could not be read"));
        assert!(
            answer.base.exists,
            "a read error must not look like a deletion"
        );
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
        assert!(escaped
            .message
            .unwrap_or_default()
            .contains("symbolic link"));
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
        fs::write(outside.path().join("swapped/a.txt"), "EXTERNAL SECRET\n").unwrap();
        let tracked = file_diff(&root, "realdir/a.txt", false).unwrap();
        assert_eq!(tracked.state, "failed");
        assert!(!tracked.patch.contains("EXTERNAL SECRET"));

        // A symlink file is a link object, exactly like Git stores it: the target path, not its
        // content, and never the content of a file outside the workspace.
        std::os::unix::fs::symlink(outside.path().join("secret.txt"), root.join("link.txt"))
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
        let mode =
            std::os::unix::fs::PermissionsExt::mode(&fs::metadata(&path).unwrap().permissions());
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
                !full
                    .base
                    .text
                    .as_deref()
                    .unwrap_or_default()
                    .contains(marker),
                "staged={staged}"
            );
            assert!(
                !full
                    .current
                    .text
                    .as_deref()
                    .unwrap_or_default()
                    .contains(marker),
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
        make_writable(&object);
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
        make_writable(&index);
        fs::write(&index, b"garbage").unwrap();
        assert!(matches!(
            read_blob(&repo_of(&root2), ":a.txt"),
            Ok(SideBytes::Unreadable)
        ));
        assert!(is_tracked(&repo_of(&root2), "a.txt").is_err());
        let broken_index = file_diff(&root2, "a.txt", false).unwrap();
        assert_eq!(broken_index.state, "failed");
        assert!(broken_index.message.unwrap_or_default().contains("index"));
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
        assert_eq!(
            git(&root, &["status", "--porcelain"]).trim(),
            "M  staged.txt"
        );

        fs::write(root.join("picked.txt"), "base\npicked\nagain\n").unwrap();
        let amended = commit(&root, &["picked.txt".to_string()], "amended subject", true).unwrap();
        assert_eq!(amended.state, "ok", "{:?}", amended.message);
        assert_eq!(
            git(&root, &["log", "-1", "--format=%s"]).trim(),
            "amended subject"
        );
        assert_eq!(git(&root, &["rev-list", "--count", "HEAD"]).trim(), "2");
        assert_eq!(
            git(&root, &["diff", "--cached", "--name-only"]).trim(),
            "staged.txt"
        );
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

        let dirty_pull = pull(&root, &Net::default(), "").unwrap_err();
        assert_eq!(dirty_pull.code, "git_dirty_tree");

        git(&root, &["checkout", "--", "a.txt"]);
        fs::write(root.join(".git/MERGE_HEAD"), "0000\n").unwrap();
        let merging = checkout(&root, "other").unwrap_err();
        assert_eq!(merging.code, "git_operation_in_progress");
        assert!(merging.message.contains("merge"));
        let merging_pull = pull(&root, &Net::default(), "").unwrap_err();
        assert_eq!(merging_pull.code, "git_operation_in_progress");
        fs::remove_file(root.join(".git/MERGE_HEAD")).unwrap();

        let created = branch_create(&root, "third", true).unwrap();
        assert_eq!(created.state, "ok");
        assert_eq!(created.status.branch.as_deref(), Some("third"));
    }

    /// A bare origin plus the clone the app would open, with identity only inside the throwaway
    /// repository (never `--global`).
    fn remote_repo(name: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let bare = temp.path().join("origin.git");
        git(
            temp.path(),
            &["init", "-q", "--bare", "-b", "main", "origin.git"],
        );
        let user = temp.path().join(name);
        git(
            temp.path(),
            &[
                "clone",
                "-q",
                bare.to_str().unwrap(),
                user.to_str().unwrap(),
            ],
        );
        git(&user, &["config", "user.name", "OpenRidr Test"]);
        git(&user, &["config", "user.email", "test@openridr.invalid"]);
        // A machine-wide signing requirement must not decide whether a test can merge.
        git(&user, &["config", "commit.gpgsign", "false"]);
        fs::write(user.join("a.txt"), "one\n").unwrap();
        fs::write(user.join("b.txt"), "one\n").unwrap();
        commit_all(&user, "initial");
        git(&user, &["push", "-q", "-u", "origin", "main"]);
        (temp, user, bare)
    }

    /// The counterpart: a second clone driven only by git, the way a colleague publishes work.
    fn peer_clone(temp: &Path, bare: &Path) -> PathBuf {
        let peer = temp.join("peer");
        git(
            temp,
            &[
                "clone",
                "-q",
                bare.to_str().unwrap(),
                peer.to_str().unwrap(),
            ],
        );
        git(&peer, &["config", "user.name", "OpenRidr Peer"]);
        git(&peer, &["config", "user.email", "peer@openridr.invalid"]);
        git(&peer, &["config", "commit.gpgsign", "false"]);
        peer
    }

    fn publish(peer: &Path, path: &str, content: &str, message: &str) {
        fs::write(peer.join(path), content).unwrap();
        commit_all(peer, message);
        git(peer, &["push", "-q", "origin", "main"]);
    }

    fn parent_count(root: &Path) -> usize {
        git(root, &["rev-list", "--parents", "-n", "1", "HEAD"])
            .split_whitespace()
            .count()
            - 1
    }

    /// A repository mid-merge: both sides changed `a.txt`, so the merge conflicts on purpose and
    /// its exit status is expected to be non-zero.
    fn conflicted_merge(name: &str) -> (tempfile::TempDir, PathBuf) {
        let (temp, root) = init(name);
        fs::write(root.join("a.txt"), "one\n").unwrap();
        fs::write(root.join("b.txt"), "one\n").unwrap();
        commit_all(&root, "initial");
        git(&root, &["checkout", "-qb", "other"]);
        fs::write(root.join("a.txt"), "other\n").unwrap();
        commit_all(&root, "other");
        git(&root, &["checkout", "-q", "main"]);
        fs::write(root.join("a.txt"), "main\n").unwrap();
        commit_all(&root, "main");
        let _ = Command::new("git")
            .args(["merge", "other"])
            .current_dir(&root)
            .output()
            .expect("git runs");
        assert!(
            root.join(".git/MERGE_HEAD").exists(),
            "the merge is in progress"
        );
        (temp, root)
    }

    #[test]
    fn merge_abort_restores_the_state_before_the_merge() {
        let (_temp, root) = conflicted_merge("repo");
        let head = git(&root, &["rev-parse", "HEAD"]).trim().to_string();
        let count = git(&root, &["rev-list", "--count", "HEAD"])
            .trim()
            .to_string();
        // A half-finished resolution must not survive the abort.
        fs::write(root.join("a.txt"), "resolved by hand\n").unwrap();

        let result = merge_abort(&root).unwrap();

        assert_eq!(result.state, "ok", "{:?}", result.message);
        assert!(!root.join(".git/MERGE_HEAD").exists());
        assert_eq!(git(&root, &["rev-parse", "HEAD"]).trim(), head);
        assert_eq!(git(&root, &["rev-list", "--count", "HEAD"]).trim(), count);
        assert_eq!(git(&root, &["status", "--porcelain"]).trim(), "");
        assert_eq!(fs::read_to_string(root.join("a.txt")).unwrap(), "main\n");
        assert!(git(&root, &["ls-files", "-u"]).trim().is_empty());
        assert_eq!(result.status.operation, None);
        assert!(
            result.message.unwrap_or_default().contains("Merge aborted"),
            "the message says what happened"
        );
    }

    #[test]
    fn merge_continue_stages_the_resolution_and_commits_the_merge() {
        let (_temp, root) = conflicted_merge("repo");
        let head = git(&root, &["rev-parse", "HEAD"]).trim().to_string();
        fs::write(root.join("a.txt"), "one\nresolved\n").unwrap();
        // An unrelated edit must not be swept into the merge commit.
        fs::write(root.join("b.txt"), "one\nstray\n").unwrap();

        let result = merge_continue(&root).unwrap();

        assert_eq!(result.state, "ok", "{:?}", result.message);
        assert!(!root.join(".git/MERGE_HEAD").exists());
        assert_eq!(parent_count(&root), 2, "a merge commit has two parents");
        assert!(
            git(&root, &["rev-list", "--parents", "-n", "1", "HEAD"]).contains(&head),
            "the pre-merge commit is a parent"
        );
        assert_eq!(
            fs::read_to_string(root.join("a.txt")).unwrap(),
            "one\nresolved\n"
        );
        assert_eq!(
            git(&root, &["show", "HEAD:b.txt"]),
            "one\n",
            "the stray edit stays out of the merge commit"
        );
        assert_eq!(
            fs::read_to_string(root.join("b.txt")).unwrap(),
            "one\nstray\n"
        );
        assert_eq!(git(&root, &["status", "--porcelain"]).trim(), "M b.txt");
        assert!(git(&root, &["ls-files", "-u"]).trim().is_empty());
        assert_eq!(result.status.operation, None);
    }

    #[test]
    fn merge_continue_refuses_while_a_conflict_marker_remains() {
        let (_temp, root) = conflicted_merge("repo");
        let head = git(&root, &["rev-parse", "HEAD"]).trim().to_string();
        // The file still holds the markers git wrote.
        let error = merge_continue(&root).unwrap_err();

        assert_eq!(error.code, "invalid_state");
        assert!(error.message.contains("a.txt"), "{}", error.message);
        assert!(
            error
                .message
                .contains("1 file still contains conflict markers"),
            "{}",
            error.message
        );
        assert!(root.join(".git/MERGE_HEAD").exists(), "the merge stays");
        assert_eq!(git(&root, &["rev-parse", "HEAD"]).trim(), head);
        assert!(
            !git(&root, &["ls-files", "-u"]).trim().is_empty(),
            "nothing was staged"
        );
    }

    #[test]
    fn merge_continue_refuses_markers_staged_and_removed_from_the_worktree() {
        let (_temp, root) = conflicted_merge("repo");
        // Stage the file with its markers, then resolve only in the worktree: the commit would
        // carry the staged markers, so the index check has to refuse too.
        git(&root, &["add", "-u", "--", "a.txt"]);
        fs::write(root.join("a.txt"), "one\nresolved\n").unwrap();
        let head = git(&root, &["rev-parse", "HEAD"]).trim().to_string();

        let error = merge_continue(&root).unwrap_err();

        assert_eq!(error.code, "invalid_state");
        assert!(error.message.contains("a.txt"), "{}", error.message);
        assert!(root.join(".git/MERGE_HEAD").exists(), "the merge stays");
        assert_eq!(git(&root, &["rev-parse", "HEAD"]).trim(), head);
    }

    #[test]
    fn merge_continue_stages_the_conflicted_name_literally() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a*.txt"), "base\n").unwrap();
        fs::write(root.join("a-other.txt"), "base\n").unwrap();
        commit_all(&root, "initial");
        git(&root, &["checkout", "-qb", "other"]);
        fs::write(root.join("a*.txt"), "other\n").unwrap();
        commit_all(&root, "other");
        git(&root, &["checkout", "-q", "main"]);
        fs::write(root.join("a*.txt"), "main\n").unwrap();
        commit_all(&root, "main");
        let _ = Command::new("git")
            .args(["merge", "other"])
            .current_dir(&root)
            .output()
            .expect("git runs");
        assert!(root.join(".git/MERGE_HEAD").exists());
        fs::write(root.join("a*.txt"), "resolved\n").unwrap();
        // A tracked file the glob would match if the name were a pathspec.
        fs::write(root.join("a-other.txt"), "stray\n").unwrap();

        let result = merge_continue(&root).unwrap();

        assert_eq!(result.state, "ok", "{:?}", result.message);
        assert_eq!(
            git(&root, &["show", "HEAD:a-other.txt"]),
            "base\n",
            "the glob match stays out of the merge commit"
        );
        assert_eq!(
            git(&root, &["status", "--porcelain"]).trim(),
            "M a-other.txt"
        );
    }

    #[test]
    fn merge_continue_refuses_when_the_marker_check_is_cut_off() {
        let (_temp, root) = conflicted_merge("repo");
        let head = git(&root, &["rev-parse", "HEAD"]).trim().to_string();

        // A ceiling this small always truncates: the call must refuse instead of proving nothing.
        let error = merge_continue_with(&root, 8).unwrap_err();

        assert_eq!(error.code, "invalid_state");
        assert!(error.message.contains("cut off"), "{}", error.message);
        assert!(root.join(".git/MERGE_HEAD").exists(), "the merge stays");
        assert_eq!(git(&root, &["rev-parse", "HEAD"]).trim(), head);
    }

    #[test]
    fn merge_continue_refuses_without_a_merge() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "initial");

        let error = merge_continue(&root).unwrap_err();

        assert_eq!(error.code, "invalid_state");
        assert!(
            error.message.contains("no merge is in progress"),
            "{}",
            error.message
        );
    }

    #[test]
    fn commit_is_refused_while_a_merge_is_in_progress() {
        let (_temp, root) = conflicted_merge("repo");
        let head = git(&root, &["rev-parse", "HEAD"]).trim().to_string();

        let error = commit(&root, &["b.txt".to_string()], "stray", false).unwrap_err();

        assert_eq!(error.code, "git_operation_in_progress");
        assert!(error.message.contains("branch menu"), "{}", error.message);
        assert_eq!(
            git(&root, &["rev-parse", "HEAD"]).trim(),
            head,
            "no commit was created"
        );
    }

    #[test]
    fn merge_abort_refuses_without_a_merge_and_for_another_operation() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "initial");

        let nothing = merge_abort(&root).unwrap_err();
        assert_eq!(nothing.code, "invalid_state");
        assert!(nothing.message.contains("no merge is in progress"));

        // The app only aborts a merge; another operation is named and left alone.
        fs::create_dir_all(root.join(".git/rebase-merge")).unwrap();
        let rebase = merge_abort(&root).unwrap_err();
        assert_eq!(rebase.code, "invalid_state");
        assert!(
            rebase.message.contains("only aborts a merge"),
            "{}",
            rebase.message
        );
        assert!(root.join(".git/rebase-merge").exists());
    }

    #[test]
    fn update_fast_forwards_without_a_merge_commit() {
        let (temp, user, bare) = remote_repo("user");
        let peer = peer_clone(temp.path(), &bare);
        publish(&peer, "b.txt", "one\npeer\n", "peer change");
        let before = git(&user, &["rev-parse", "HEAD"]).trim().to_string();

        let result = pull(&user, &Net::default(), "").unwrap();

        assert_eq!(result.state, "ok", "{:?}", result.message);
        assert_ne!(git(&user, &["rev-parse", "HEAD"]).trim(), before);
        assert_eq!(
            parent_count(&user),
            1,
            "a fast-forward adds no merge commit"
        );
        assert_eq!(
            git(&user, &["log", "-1", "--format=%s"]).trim(),
            "peer change"
        );
        assert_eq!(result.status.ahead, 0);
        assert_eq!(result.status.behind, 0);
        assert_eq!(result.status.operation, None);
    }

    #[test]
    fn update_merges_a_diverged_branch_preserving_both_sides() {
        let (temp, user, bare) = remote_repo("user");
        let peer = peer_clone(temp.path(), &bare);
        publish(&peer, "b.txt", "one\npeer\n", "peer change");
        fs::write(user.join("a.txt"), "one\nlocal\n").unwrap();
        commit_all(&user, "local change");
        let local = git(&user, &["rev-parse", "HEAD"]).trim().to_string();

        let result = pull(&user, &Net::default(), "").unwrap();

        assert_eq!(result.state, "ok", "{:?}", result.message);
        let parents = git(&user, &["rev-list", "--parents", "-n", "1", "HEAD"]);
        assert_eq!(parent_count(&user), 2, "a merge commit has two parents");
        assert!(parents.contains(&local), "the local commit is a parent");
        assert_eq!(
            fs::read_to_string(user.join("a.txt")).unwrap(),
            "one\nlocal\n"
        );
        assert_eq!(
            fs::read_to_string(user.join("b.txt")).unwrap(),
            "one\npeer\n"
        );
        assert_eq!(git(&user, &["status", "--porcelain"]).trim(), "");
        // The merge commit itself is not pushed, so the branch now leads its upstream by the
        // local commit plus the merge — and is no longer behind.
        assert_eq!(result.status.ahead, 2);
        assert_eq!(result.status.behind, 0);
        assert_eq!(result.status.operation, None);
    }

    #[test]
    fn update_ignores_the_repository_pull_and_merge_settings() {
        let (temp, user, bare) = remote_repo("user");
        let peer = peer_clone(temp.path(), &bare);
        // The dialog states the policy, so the repository's preferences cannot silently win.
        git(&user, &["config", "pull.rebase", "true"]);
        git(&user, &["config", "pull.ff", "only"]);
        git(&user, &["config", "merge.ff", "only"]);
        publish(&peer, "b.txt", "one\npeer\n", "peer change");
        fs::write(user.join("a.txt"), "one\nlocal\n").unwrap();
        commit_all(&user, "local change");

        let result = pull(&user, &Net::default(), "").unwrap();

        assert_eq!(result.state, "ok", "{:?}", result.message);
        assert_eq!(parent_count(&user), 2, "a merge commit, not a rebase");
        assert_eq!(result.status.operation, None);
    }

    #[test]
    fn update_leaves_a_conflicting_merge_for_the_user_to_resolve() {
        let (temp, user, bare) = remote_repo("user");
        let peer = peer_clone(temp.path(), &bare);
        publish(&peer, "a.txt", "one\npeer\n", "peer change");
        fs::write(user.join("a.txt"), "one\nlocal\n").unwrap();
        commit_all(&user, "local change");
        let head_before = git(&user, &["rev-parse", "HEAD"]).trim().to_string();
        let count_before = git(&user, &["rev-list", "--count", "HEAD"])
            .trim()
            .to_string();

        let result = pull(&user, &Net::default(), "").unwrap();

        assert_eq!(result.state, "failed");
        let message = result.message.clone().unwrap_or_default();
        assert!(
            message.contains("1 file changed on both sides (a.txt)"),
            "{message}"
        );
        assert!(message.contains("still in progress"), "{message}");
        assert!(message.contains("branch menu"), "{message}");
        assert!(!message.contains("hint:"), "no git advice leaks: {message}");
        assert!(!message.contains("aborted"), "{message}");
        let lower = message.to_lowercase();
        assert!(!lower.contains("pull.rebase"), "{message}");
        assert!(!lower.contains("pull.ff"), "{message}");
        // Nothing was undone: the merge is there for the branch menu to finish or abort, both
        // commits are intact, and the local bytes are untouched until the user resolves.
        assert!(user.join(".git/MERGE_HEAD").exists());
        assert_eq!(git(&user, &["rev-parse", "HEAD"]).trim(), head_before);
        assert_eq!(
            git(&user, &["rev-list", "--count", "HEAD"]).trim(),
            count_before
        );
        assert_eq!(result.status.operation, Some("merge"));
        assert!(result
            .status
            .files
            .iter()
            .any(|file| file.path == "a.txt" && file.conflicted));
        assert!(result.output.contains("CONFLICT"), "{}", result.output);

        // The branch menu's Complete merge finishes exactly this state.
        fs::write(user.join("a.txt"), "one\nresolved\n").unwrap();
        let completed = merge_continue(&user).unwrap();
        assert_eq!(completed.state, "ok", "{:?}", completed.message);
        assert!(!user.join(".git/MERGE_HEAD").exists());
        assert_eq!(parent_count(&user), 2, "a merge commit has two parents");
        assert_eq!(git(&user, &["status", "--porcelain"]).trim(), "");
        assert_eq!(
            fs::read_to_string(user.join("a.txt")).unwrap(),
            "one\nresolved\n"
        );
    }

    #[test]
    fn update_abort_undoes_the_conflicting_merge_when_asked() {
        let (temp, user, bare) = remote_repo("user");
        let peer = peer_clone(temp.path(), &bare);
        publish(&peer, "a.txt", "one\npeer\n", "peer change");
        fs::write(user.join("a.txt"), "one\nlocal\n").unwrap();
        commit_all(&user, "local change");
        let head_before = git(&user, &["rev-parse", "HEAD"]).trim().to_string();

        let failed = pull(&user, &Net::default(), "").unwrap();
        assert_eq!(failed.state, "failed");
        let aborted = merge_abort(&user).unwrap();

        assert_eq!(aborted.state, "ok", "{:?}", aborted.message);
        assert!(!user.join(".git/MERGE_HEAD").exists());
        assert_eq!(git(&user, &["rev-parse", "HEAD"]).trim(), head_before);
        assert_eq!(git(&user, &["status", "--porcelain"]).trim(), "");
        assert_eq!(
            fs::read_to_string(user.join("a.txt")).unwrap(),
            "one\nlocal\n"
        );
        assert_eq!(aborted.status.operation, None);
    }

    #[test]
    fn update_reports_a_missing_identity_in_the_apps_own_words() {
        let (temp, user, bare) = remote_repo("user");
        let peer = peer_clone(temp.path(), &bare);
        publish(&peer, "b.txt", "one\npeer\n", "peer change");
        fs::write(user.join("a.txt"), "one\nlocal\n").unwrap();
        commit_all(&user, "local change");
        // No guessing from the user or host name, and no local identity: the merge commit cannot
        // be authored whatever the machine's global config says.
        git(&user, &["config", "user.useConfigOnly", "true"]);
        git(&user, &["config", "user.name", ""]);
        git(&user, &["config", "user.email", ""]);
        let head_before = git(&user, &["rev-parse", "HEAD"]).trim().to_string();

        let result = pull(&user, &Net::default(), "").unwrap();

        assert_eq!(result.state, "failed");
        let message = result.message.clone().unwrap_or_default();
        assert!(message.contains("needs a Git identity"), "{message}");
        assert!(message.contains("user.name and user.email"), "{message}");
        assert!(!message.contains("Please tell me who you are"), "{message}");
        assert_eq!(git(&user, &["rev-parse", "HEAD"]).trim(), head_before);
        assert_eq!(git(&user, &["status", "--porcelain"]).trim(), "");
        assert!(!user.join(".git/MERGE_HEAD").exists());
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
            &["remote", "add", "origin", remote.path().to_str().unwrap()],
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

        let missing = push(&root, &serde_json::json!({}), &Net::default(), "").unwrap_err();
        assert_eq!(missing.code, "confirmation_required");
        let no_remote = push(
            &root,
            &serde_json::json!({ "confirm": true }),
            &Net::default(),
            "",
        )
        .unwrap_err();
        assert_eq!(no_remote.code, "git_no_remote");

        git(&root, &["remote", "add", "origin", bare.to_str().unwrap()]);
        let pushed = push(
            &root,
            &serde_json::json!({ "confirm": true }),
            &Net::default(),
            "",
        )
        .unwrap();
        assert_eq!(pushed.state, "ok", "{:?}", pushed.message);
        assert_eq!(pushed.remote.as_deref(), Some("origin"));
        assert_eq!(pushed.status.upstream.as_deref(), Some("origin/main"));
        let count = git(&bare, &["rev-list", "--count", "main"]);
        assert_eq!(count.trim(), "1");
    }

    #[test]
    fn fetch_moves_the_behind_counter_and_the_remote_branch_list() {
        let (temp, user, bare) = remote_repo("user");
        let peer = peer_clone(temp.path(), &bare);
        publish(&peer, "b.txt", "one\npeer\n", "peer change");
        git(&peer, &["checkout", "-qb", "published", "origin/main"]);
        // A branch only the counterpart knows about, published from a second commit.
        fs::write(peer.join("c.txt"), "peer only\n").unwrap();
        commit_all(&peer, "peer branch");
        git(&peer, &["push", "-q", "origin", "published"]);

        // Before any fetch the local tracking refs are stale: this is the #113 symptom.
        let stale = status(&user);
        assert_eq!(stale.behind, 0);
        assert!(branches(&user)
            .remote
            .iter()
            .all(|branch| branch.name != "origin/published"));
        let head = git(&user, &["rev-parse", "HEAD"]).trim().to_string();
        let worktree_before = fs::read_to_string(user.join("b.txt")).unwrap();

        let result = fetch(&user, &Net::default(), "").unwrap();

        assert_eq!(result.state, "ok", "{:?}", result.message);
        assert_eq!(result.remote.as_deref(), Some("origin"));
        assert_eq!(result.status.behind, 1, "the counter the panel shows");
        assert_eq!(result.status.ahead, 0);
        assert_eq!(
            git(&user, &["rev-parse", "HEAD"]).trim(),
            head,
            "fetch never moves HEAD"
        );
        assert_eq!(
            fs::read_to_string(user.join("b.txt")).unwrap(),
            worktree_before,
            "fetch never writes the worktree"
        );
        assert_eq!(git(&user, &["status", "--porcelain"]).trim(), "");
        let listed = branches(&user);
        assert!(listed
            .remote
            .iter()
            .any(|branch| branch.name == "origin/published"));
        let message = result.message.unwrap_or_default();
        assert!(message.contains("Fetched origin"), "{message}");
        assert!(message.contains("1 behind"), "{message}");
    }

    #[test]
    fn fetch_prunes_a_branch_the_counterpart_deleted_and_keeps_local_work() {
        let (temp, user, bare) = remote_repo("user");
        let peer = peer_clone(temp.path(), &bare);
        git(&peer, &["checkout", "-qb", "doomed"]);
        fs::write(peer.join("doomed.txt"), "x\n").unwrap();
        commit_all(&peer, "doomed");
        git(&peer, &["push", "-q", "origin", "doomed"]);
        fetch(&user, &Net::default(), "").unwrap();
        assert!(branches(&user)
            .remote
            .iter()
            .any(|branch| branch.name == "origin/doomed"));
        // The user keeps a local branch with the same short name: prune must not touch it.
        git(&user, &["branch", "doomed", "origin/doomed"]);
        let local_before = git(&user, &["rev-parse", "doomed"]).trim().to_string();

        git(&peer, &["push", "-q", "origin", "--delete", "doomed"]);
        let result = fetch(&user, &Net::default(), "").unwrap();

        assert_eq!(result.state, "ok", "{:?}", result.message);
        assert!(branches(&user)
            .remote
            .iter()
            .all(|branch| branch.name != "origin/doomed"));
        assert_eq!(
            git(&user, &["rev-parse", "doomed"]).trim(),
            local_before,
            "prune only removes refs/remotes/*"
        );
    }

    #[test]
    fn fetch_is_allowed_with_a_dirty_tree_and_around_an_in_progress_merge() {
        let (temp, user, bare) = remote_repo("user");
        let peer = peer_clone(temp.path(), &bare);
        publish(&peer, "a.txt", "one\npeer\n", "peer change");
        fs::write(user.join("a.txt"), "one\nlocal\n").unwrap();
        commit_all(&user, "local change");
        // The app's own action first, so the merge below has something to conflict with.
        fetch(&user, &Net::default(), "").unwrap();
        // A real conflict, started outside the app: MERGE_HEAD exists and the tree is dirty.
        let _ = Command::new("git")
            .args(["merge", "origin/main"])
            .current_dir(&user)
            .output()
            .expect("git runs");
        assert!(user.join(".git/MERGE_HEAD").exists());
        let dirty = fs::read_to_string(user.join("a.txt")).unwrap();

        let result = fetch(&user, &Net::default(), "").unwrap();

        assert_eq!(result.state, "ok", "{:?}", result.message);
        assert!(user.join(".git/MERGE_HEAD").exists());
        assert_eq!(fs::read_to_string(user.join("a.txt")).unwrap(), dirty);
        assert!(result.status.operation.is_some(), "still mid-merge");
    }

    #[test]
    fn fetch_without_a_remote_is_refused_with_the_apps_reason() {
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "initial");

        let refused = fetch(&root, &Net::default(), "").unwrap_err();

        assert_eq!(refused.code, "git_no_remote");
        assert!(
            refused.message.contains("No remote is configured"),
            "{}",
            refused.message
        );
    }

    #[test]
    fn fetch_reports_a_credential_failure_in_the_apps_own_words() {
        let message = fetch_failure(
            "origin",
            "fatal: could not read Username for 'https://example.com': terminal prompts disabled",
        );
        assert!(
            message.contains("needs credentials for origin"),
            "{message}"
        );
        assert!(message.contains("credential helper"), "{message}");
        assert!(message.contains("Nothing was changed"), "{message}");
        assert!(!message.contains("terminal prompts"), "{message}");
        // Anything the app does not know keeps Git's own tail instead of a guess.
        let other = fetch_failure("origin", "fatal: unable to access: connection reset");
        assert!(other.contains("connection reset"), "{other}");
    }

    #[test]
    fn a_fetch_that_outruns_its_deadline_is_killed_and_changes_nothing() {
        let (temp, user, bare) = remote_repo("user");
        let peer = peer_clone(temp.path(), &bare);
        publish(&peer, "b.txt", "one\npeer\n", "peer change");
        let before = git(&user, &["for-each-ref", "refs/remotes"]);
        let began = Instant::now();

        // A deadline that is already spent: the child cannot finish, so it is killed.
        let result = fetch_with(&user, Duration::ZERO, &Net::default(), "").unwrap();

        assert_eq!(result.state, "failed");
        let message = result.message.unwrap_or_default();
        assert!(message.contains("did not answer within"), "{message}");
        // The promise is about what a fetch can never touch, not about refs a kill may have caught
        // mid-update.
        assert!(
            message.contains("branch, index and working tree were not touched"),
            "{message}"
        );
        assert!(
            began.elapsed() < Duration::from_secs(10),
            "the kill must not wait for the remote"
        );
        assert_eq!(
            git(&user, &["for-each-ref", "refs/remotes"]),
            before,
            "no tracking ref moved"
        );
        assert_eq!(status(&user).behind, 0);
    }

    /// A remote that accepts the connection and never answers: the deadline has to reach the
    /// helper git spawned (`git-remote-http`), which is what holds the pipes. Killing only the
    /// leader would leave the reader blocked and the host call would never return.
    #[test]
    fn a_fetch_against_a_remote_that_never_answers_returns_at_the_deadline() {
        let (_temp, user, _bare) = remote_repo("user");
        let (port, _accepted) = silent_remote();
        git(
            &user,
            &[
                "remote",
                "set-url",
                "origin",
                &format!("http://127.0.0.1:{port}/repo.git"),
            ],
        );
        let before = git(&user, &["for-each-ref", "refs/remotes"]);
        let began = Instant::now();

        let result = fetch_with(&user, Duration::from_secs(1), &Net::default(), "").unwrap();

        assert_eq!(result.state, "failed");
        let message = result.message.unwrap_or_default();
        assert!(
            message.contains("did not answer within 1 seconds"),
            "{message}"
        );
        assert!(
            began.elapsed() < Duration::from_secs(15),
            "the deadline must return even with a hung helper: {:?}",
            began.elapsed()
        );
        assert_eq!(git(&user, &["for-each-ref", "refs/remotes"]), before);
        // git reaches an http remote through `git remote-http` → `git-remote-http`, and those
        // inherit the pipes. They must die with the deadline: a survivor would hold the pipe the
        // reader is waiting on, which is the freeze the bound exists to prevent.
        if let Some(survivors) = descendants_of(&port.to_string()) {
            assert!(
                survivors.is_empty(),
                "the helper survived the deadline: {survivors:?}"
            );
        }
    }

    /// A remote that accepts the connection and holds it open without ever answering, on a port the
    /// caller can point a clone at, plus a flag set once the connection was accepted: the tests that
    /// kill the child wait for it, so the kill has a live connection to reach. The accepting thread
    /// is left detached: joining it would put its sleep on the test's clock.
    fn silent_remote() -> (u16, Arc<AtomicBool>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let accepted = Arc::new(AtomicBool::new(false));
        let signal = Arc::clone(&accepted);
        std::thread::spawn(move || {
            if let Ok((stream, _)) = listener.accept() {
                signal.store(true, Ordering::SeqCst);
                std::thread::sleep(Duration::from_secs(10));
                drop(stream);
            }
        });
        (port, accepted)
    }

    /// A push is the one network command whose kill can land after the remote accepted the refs, so
    /// the message may not claim success — and may not claim the upstream was configured either,
    /// because `--set-upstream` writes nothing when the push never completes.
    #[test]
    fn a_push_against_a_remote_that_never_answers_returns_at_the_deadline() {
        let (_temp, user, bare) = remote_repo("user");
        let (port, _accepted) = silent_remote();
        git(
            &user,
            &[
                "remote",
                "set-url",
                "origin",
                &format!("http://127.0.0.1:{port}/repo.git"),
            ],
        );
        fs::write(user.join("a.txt"), "one\ntwo\n").unwrap();
        commit_all(&user, "local change");
        let head = git(&user, &["rev-parse", "HEAD"]).trim().to_string();
        let published = git(&bare, &["rev-parse", "main"]).trim().to_string();
        let began = Instant::now();

        let result = push_with(
            &user,
            &serde_json::json!({ "confirm": true }),
            Duration::from_secs(1),
            &Net::default(),
            "",
        )
        .unwrap();

        assert_eq!(result.state, "failed");
        let message = result.message.unwrap_or_default();
        assert!(
            message.contains("did not answer within 1 seconds"),
            "{message}"
        );
        assert!(!message.contains("Pushed"), "{message}");
        assert!(message.contains("may already have reached it"), "{message}");
        // Neither the local state nor the destination remote is promised: a `pre-push` hook can
        // change the tree, and `git push` may resolve a remote the app never read.
        assert!(!message.contains("not touched"), "{message}");
        assert!(!message.contains("origin"), "{message}");
        assert!(
            began.elapsed() < Duration::from_secs(15),
            "the deadline must return even with a hung helper: {:?}",
            began.elapsed()
        );
        assert_eq!(
            git(&bare, &["rev-parse", "main"]).trim(),
            published,
            "the remote must not have the local commit"
        );
        assert_eq!(git(&user, &["rev-parse", "HEAD"]).trim(), head);
        if let Some(survivors) = descendants_of(&port.to_string()) {
            assert!(
                survivors.is_empty(),
                "the helper survived the deadline: {survivors:?}"
            );
        }
    }

    /// The update's deadline covers the fetch, which runs before the merge: a killed update cannot
    /// leave a merge behind, so the message says nothing was merged instead of pointing at
    /// `git merge --abort`.
    #[test]
    fn an_update_against_a_remote_that_never_answers_returns_at_the_deadline() {
        let (_temp, user, _bare) = remote_repo("user");
        let (port, _accepted) = silent_remote();
        git(
            &user,
            &[
                "remote",
                "set-url",
                "origin",
                &format!("http://127.0.0.1:{port}/repo.git"),
            ],
        );
        let head = git(&user, &["rev-parse", "HEAD"]).trim().to_string();
        let before = git(&user, &["for-each-ref", "refs/remotes"]);
        let began = Instant::now();

        let result = pull_with(&user, Duration::from_secs(1), &Net::default(), "").unwrap();

        assert_eq!(result.state, "failed");
        let message = result.message.unwrap_or_default();
        assert!(
            message.contains("did not answer within 1 seconds"),
            "{message}"
        );
        assert!(message.contains("Nothing was merged"), "{message}");
        assert!(!message.contains("merge --abort"), "{message}");
        assert!(
            began.elapsed() < Duration::from_secs(15),
            "the deadline must return even with a hung helper: {:?}",
            began.elapsed()
        );
        assert_eq!(git(&user, &["rev-parse", "HEAD"]).trim(), head);
        assert!(!user.join(".git/MERGE_HEAD").exists());
        assert!(result.status.operation.is_none());
        assert_eq!(git(&user, &["for-each-ref", "refs/remotes"]), before);
        if let Some(survivors) = descendants_of(&port.to_string()) {
            assert!(
                survivors.is_empty(),
                "the helper survived the deadline: {survivors:?}"
            );
        }
    }

    /// Runs `operation` on its own thread and cancels it once `silent_remote` accepted the
    /// connection — cancelling before the connection exists would pass the "no descendant
    /// survived" assertion without the kill ever having a target.
    ///
    /// The deadline is far longer than the bound the test asserts, so returning early can only be
    /// the cancel — the same trick the deadline tests use with a spent deadline.
    fn cancel_in_flight<F>(
        net: &Net,
        id: &str,
        accepted: &AtomicBool,
        operation: F,
    ) -> (HostResult<Operation>, Duration)
    where
        F: FnOnce() -> HostResult<Operation> + Send,
        F::Output: Send,
    {
        let began = Instant::now();
        std::thread::scope(|scope| {
            let running = scope.spawn(operation);
            while !accepted.load(Ordering::SeqCst) {
                assert!(
                    began.elapsed() < Duration::from_secs(10),
                    "the operation never reached the remote"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(
                net.cancel(id),
                "the cancel must find the operation in flight"
            );
            let result = running.join().expect("the operation thread returns");
            (result, began.elapsed())
        })
    }

    /// A push is the one operation whose kill can land after the remote accepted the refs, so the
    /// cancelled message keeps the deadline's promises: no success, no claim about the local state.
    #[test]
    fn a_push_stopped_by_the_cancel_returns_before_the_deadline() {
        let (_temp, user, bare) = remote_repo("user");
        let (port, accepted) = silent_remote();
        git(
            &user,
            &[
                "remote",
                "set-url",
                "origin",
                &format!("http://127.0.0.1:{port}/repo.git"),
            ],
        );
        fs::write(user.join("a.txt"), "one\ntwo\n").unwrap();
        commit_all(&user, "local change");
        let head = git(&user, &["rev-parse", "HEAD"]).trim().to_string();
        let tree = git(&user, &["status", "--porcelain"]);
        let published = git(&bare, &["rev-parse", "main"]).trim().to_string();
        let net = Net::default();
        let id = net.begin(Kind::Push).expect("the host starts one push");

        let (result, elapsed) = cancel_in_flight(&net, &id, &accepted, || {
            push_with(
                &user,
                &serde_json::json!({ "confirm": true }),
                Duration::from_secs(120),
                &net,
                &id,
            )
        });

        let result = result.unwrap();
        assert_eq!(result.state, "failed");
        assert_eq!(result.stop, Stop::Cancelled);
        let message = result.message.clone().unwrap_or_default();
        assert!(message.contains("Push cancelled"), "{message}");
        assert!(
            message.contains("may already have reached the remote"),
            "{message}"
        );
        assert!(!message.contains("Pushed"), "{message}");
        assert!(
            elapsed < Duration::from_secs(5),
            "the cancel must return well before the 120 s deadline: {elapsed:?}"
        );
        assert_eq!(
            git(&bare, &["rev-parse", "main"]).trim(),
            published,
            "the remote must not have the local commit"
        );
        assert_eq!(git(&user, &["rev-parse", "HEAD"]).trim(), head);
        assert_eq!(git(&user, &["status", "--porcelain"]), tree);
        if let Some(survivors) = descendants_of(&port.to_string()) {
            assert!(
                survivors.is_empty(),
                "the helper survived the cancel: {survivors:?}"
            );
        }
        // The published view is the host's: the cancel wins over the operation's own state.
        net.end(&id, Ok(result));
        let view = net.view();
        assert_eq!(view.state, "cancelled");
        assert_eq!(view.id, id);
        assert_eq!(view.kind, "push");
        assert!(
            view.message.unwrap_or_default().contains("Push cancelled"),
            "the view carries the app's message"
        );
    }

    /// The fetch keeps its own promise: it can never change the branch, the index or the working
    /// tree, and refs already received stay. The id is the host's, so the cancel names exactly the
    /// operation it was issued for.
    #[test]
    fn a_fetch_stopped_by_the_cancel_returns_before_the_deadline() {
        let (_temp, user, _bare) = remote_repo("user");
        let (port, accepted) = silent_remote();
        git(
            &user,
            &[
                "remote",
                "set-url",
                "origin",
                &format!("http://127.0.0.1:{port}/repo.git"),
            ],
        );
        let head = git(&user, &["rev-parse", "HEAD"]).trim().to_string();
        let branch = git(&user, &["rev-parse", "--abbrev-ref", "HEAD"])
            .trim()
            .to_string();
        let tree = git(&user, &["status", "--porcelain"]);
        let before = git(&user, &["for-each-ref", "refs/remotes"]);
        let net = Net::default();
        let id = net.begin(Kind::Fetch).expect("the host starts one fetch");

        let (result, elapsed) = cancel_in_flight(&net, &id, &accepted, || {
            fetch_with(&user, Duration::from_secs(60), &net, &id)
        });

        let result = result.unwrap();
        assert_eq!(result.state, "failed");
        assert_eq!(result.stop, Stop::Cancelled);
        let message = result.message.clone().unwrap_or_default();
        assert!(message.contains("Fetch cancelled"), "{message}");
        assert!(!message.contains("did not answer within"), "{message}");
        assert!(
            elapsed < Duration::from_secs(5),
            "the cancel must return well before the 60 s deadline: {elapsed:?}"
        );
        assert_eq!(git(&user, &["for-each-ref", "refs/remotes"]), before);
        assert_eq!(git(&user, &["rev-parse", "HEAD"]).trim(), head);
        assert_eq!(
            git(&user, &["rev-parse", "--abbrev-ref", "HEAD"]).trim(),
            branch
        );
        assert_eq!(git(&user, &["status", "--porcelain"]), tree);
        if let Some(survivors) = descendants_of(&port.to_string()) {
            assert!(
                survivors.is_empty(),
                "the helper survived the cancel: {survivors:?}"
            );
        }
        // The published view is the host's: the cancel wins over the operation's own state.
        net.end(&id, Ok(result));
        let view = net.view();
        assert_eq!(view.state, "cancelled");
        assert_eq!(view.id, id);
        assert_eq!(view.kind, "fetch");
        assert!(
            view.message.unwrap_or_default().contains("Fetch cancelled"),
            "the view carries the app's message"
        );
    }

    /// The update's cancel covers the fetch, which runs before the merge: a cancelled update cannot
    /// leave a merge behind, so the message says nothing was merged instead of pointing at
    /// `git merge --abort`.
    #[test]
    fn an_update_stopped_by_the_cancel_returns_before_the_deadline() {
        let (_temp, user, _bare) = remote_repo("user");
        let (port, accepted) = silent_remote();
        git(
            &user,
            &[
                "remote",
                "set-url",
                "origin",
                &format!("http://127.0.0.1:{port}/repo.git"),
            ],
        );
        let head = git(&user, &["rev-parse", "HEAD"]).trim().to_string();
        let tree = git(&user, &["status", "--porcelain"]);
        let before = git(&user, &["for-each-ref", "refs/remotes"]);
        let net = Net::default();
        let id = net.begin(Kind::Update).expect("the host starts one update");

        let (result, elapsed) = cancel_in_flight(&net, &id, &accepted, || {
            pull_with(&user, Duration::from_secs(60), &net, &id)
        });

        let result = result.unwrap();
        assert_eq!(result.state, "failed");
        assert_eq!(result.stop, Stop::Cancelled);
        let message = result.message.unwrap_or_default();
        assert!(message.contains("Update cancelled"), "{message}");
        assert!(message.contains("nothing was merged"), "{message}");
        assert!(!message.contains("merge --abort"), "{message}");
        assert!(
            elapsed < Duration::from_secs(5),
            "the cancel must return well before the 60 s deadline: {elapsed:?}"
        );
        assert_eq!(git(&user, &["rev-parse", "HEAD"]).trim(), head);
        assert_eq!(git(&user, &["status", "--porcelain"]), tree);
        assert!(!user.join(".git/MERGE_HEAD").exists());
        assert!(result.status.operation.is_none());
        assert_eq!(git(&user, &["for-each-ref", "refs/remotes"]), before);
        if let Some(survivors) = descendants_of(&port.to_string()) {
            assert!(
                survivors.is_empty(),
                "the helper survived the cancel: {survivors:?}"
            );
        }
    }

    /// The local merge of an update is not cancellable: once the fetch child ended, a cancel finds
    /// nothing to stop and the update carries on to the merge (#207).
    #[test]
    fn a_cancel_after_the_network_step_reports_nothing_to_stop() {
        let net = Net::default();
        let id = net.begin(Kind::Update).unwrap();
        // The fetch child ended: no cancel landed, and the window is closed either way.
        assert!(!net.finish_network(&id));
        assert!(
            !net.cancel(&id),
            "the local merge of an update is not cancellable"
        );
    }

    /// One network operation per host, and the id is the host's: a second start is refused, naming
    /// the one in flight, and a cancel can only name an operation that exists.
    #[test]
    fn the_host_issues_the_id_and_refuses_a_second_operation() {
        let net = Net::default();
        assert_eq!(net.view().state, "idle");
        assert!(!net.cancel("git:1"), "an idle host has nothing to cancel");

        let id = net.begin(Kind::Push).expect("the first start is accepted");
        assert_eq!(id, "git:1");
        assert_eq!(net.view().state, "running");
        assert_eq!(net.view().id, id);
        assert_eq!(net.view().kind, "push");
        let refused = net
            .begin(Kind::Push)
            .expect_err("a second start is refused");
        assert!(
            refused.contains("A push is already running (git:1)"),
            "{refused}"
        );

        assert!(
            !net.cancel("git:999"),
            "a cancel names an operation the host issued"
        );
        assert!(net.cancel(&id), "the cancel reaches the operation it names");
        assert!(net.cancelled(&id));
        assert!(
            !net.cancelled("git:999"),
            "another id is not that operation"
        );
    }

    /// The end of the network step and a cancel are one transition: a cancel that lands before it is
    /// reported to the operation, and one that lands after finds nothing to stop. Both under the
    /// same lock, so there is no gap where a cancel is accepted and then ignored.
    #[test]
    fn the_end_of_the_network_step_is_atomic_with_the_cancel() {
        let net = Net::default();
        let id = net.begin(Kind::Push).unwrap();
        assert!(net.cancel(&id));
        assert!(
            net.finish_network(&id),
            "a cancel that landed before the end is reported to the operation"
        );
        assert!(
            !net.cancel(&id),
            "after the end there is nothing to stop: the local merge of an update is not cancellable"
        );
        assert!(
            !net.finish_network(&id),
            "the window closes once, so a second end cannot report a cancel"
        );
        // The cancel was accepted, so it is what the operation reports — even when the operation
        // then failed on its own. The message stays the operation's own, so the reason it did not
        // run is still visible; `git.cancel` answering `true` never ends as `ok` or as a plain
        // failure the user reads as "it ran anyway".
        net.end(&id, Err(HostError::new("git_failed", "boom")));
        let view = net.view();
        assert_eq!(view.state, "cancelled");
        assert_eq!(view.message.as_deref(), Some("boom"));
        assert_eq!(view.id, id);
        assert_eq!(view.kind, "push");
        assert!(!net.cancel(&id), "the ended operation is not cancellable");
        assert!(
            net.begin(Kind::Push).is_ok(),
            "the next operation starts once the last one ended"
        );
    }

    /// A cancel that names an operation that already ended must not touch the one running now: the
    /// id is what separates them.
    #[test]
    fn a_cancel_for_a_finished_operation_does_not_stop_the_next_one() {
        let (_temp, user, _bare) = remote_repo("user");
        let net = Net::default();
        let first = net.begin(Kind::Push).unwrap();
        net.end(&first, Err(HostError::new("git_failed", "no remote")));
        assert!(
            !net.cancel(&first),
            "a cancel for a finished operation stops nothing"
        );

        let (port, accepted) = silent_remote();
        git(
            &user,
            &[
                "remote",
                "set-url",
                "origin",
                &format!("http://127.0.0.1:{port}/repo.git"),
            ],
        );
        let second = net.begin(Kind::Push).unwrap();
        std::thread::scope(|scope| {
            let running = scope.spawn(|| {
                push_with(
                    &user,
                    &serde_json::json!({ "confirm": true }),
                    Duration::from_secs(60),
                    &net,
                    &second,
                )
            });
            while !accepted.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(5));
            }
            assert!(
                !net.cancel(&first),
                "the finished id must not reach the new operation"
            );
            std::thread::sleep(Duration::from_millis(100));
            assert!(
                !running.is_finished(),
                "the stale cancel must not have stopped the new operation"
            );
            assert!(
                net.cancel(&second),
                "the new operation is cancelled by its own id"
            );
            let result = running.join().expect("the push thread returns").unwrap();
            assert_eq!(result.state, "failed");
            assert!(
                result
                    .message
                    .unwrap_or_default()
                    .contains("Push cancelled"),
                "the new operation reports its own cancel"
            );
        });
    }

    /// Processes whose command line mentions `needle`, or `None` when the platform has no `pgrep`
    /// (the assertion is then skipped rather than guessed).
    fn descendants_of(needle: &str) -> Option<Vec<String>> {
        let output = Command::new("pgrep").args(["-fl", needle]).output().ok()?;
        Some(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::to_string)
                .collect(),
        )
    }

    /// A remote URL that carries credentials must not leak them into the message or the operation
    /// output, which is what a view or a log would show.
    #[test]
    fn a_failing_fetch_never_echoes_credentials_from_the_remote_url() {
        let (_temp, user, _bare) = remote_repo("user");
        // Port 1 refuses immediately: the failure is real, the network is not.
        git(
            &user,
            &[
                "remote",
                "set-url",
                "origin",
                "https://user:secret-token@127.0.0.1:1/repo.git",
            ],
        );

        let result = fetch_with(&user, Duration::from_secs(20), &Net::default(), "").unwrap();

        assert_eq!(result.state, "failed");
        let message = result.message.unwrap_or_default();
        assert!(!message.contains("secret-token"), "{message}");
        assert!(!result.output.contains("secret-token"), "{}", result.output);
        // Measured here: for a connection failure git itself already prints the URL without the
        // userinfo, so the assertion above would pass even without masking. The masking the app
        // owns is proved where the operation output is built, with a stderr that does carry it.
        let masked = operation(
            "failed",
            None,
            "fatal: unable to access 'https://user:secret-token@example.com/x.git/': denied",
            &repo_of(&user),
            None,
        );
        assert!(!masked.output.contains("secret-token"), "{}", masked.output);
        assert!(
            masked.output.contains("***@example.com"),
            "{}",
            masked.output
        );
        assert!(!fetch_failure(
            "origin",
            &mask_urls(
                "fatal: could not read Username for 'https://user:secret-token@example.com'"
            )
        )
        .contains("secret-token"));
    }

    /// `--prune` deletes whatever the configured refspecs point at. A repository that maps the
    /// remote somewhere else must not lose those refs to the Fetch button.
    #[test]
    fn fetch_prune_cannot_reach_refs_outside_the_remote_tracking_namespace() {
        let (temp, user, bare) = remote_repo("user");
        let peer = peer_clone(temp.path(), &bare);
        git(&peer, &["checkout", "-qb", "doomed"]);
        fs::write(peer.join("doomed.txt"), "x\n").unwrap();
        commit_all(&peer, "doomed");
        git(&peer, &["push", "-q", "origin", "doomed"]);
        // A hostile (or just unusual) refspec: the remote mirrors into refs/mirror/*. The app's
        // fetch must neither create those refs nor update them — Git applies the configured
        // refspec as an extra refmap unless the command drops it.
        git(
            &user,
            &[
                "config",
                "remote.origin.fetch",
                "+refs/heads/*:refs/mirror/*",
            ],
        );
        assert_eq!(git(&user, &["for-each-ref", "refs/mirror"]).trim(), "");
        let first = fetch(&user, &Net::default(), "").unwrap();
        assert_eq!(first.state, "ok", "{:?}", first.message);
        assert_eq!(
            git(&user, &["for-each-ref", "refs/mirror"]).trim(),
            "",
            "the app's fetch must not create refs outside refs/remotes/*"
        );

        // The same for a ref that already exists there: it must keep its old value.
        git(&user, &["fetch", "-q", "origin"]);
        let mirrored = git(&user, &["for-each-ref", "refs/mirror"]);
        assert!(mirrored.contains("refs/mirror/doomed"), "{mirrored}");
        publish(&peer, "b.txt", "one\npeer\n", "peer change");

        git(&peer, &["push", "-q", "origin", "--delete", "doomed"]);
        let result = fetch(&user, &Net::default(), "").unwrap();

        assert_eq!(result.state, "ok", "{:?}", result.message);
        assert_eq!(
            git(&user, &["for-each-ref", "refs/mirror"]),
            mirrored,
            "the app's fetch must not prune or update refs outside refs/remotes/*"
        );
        assert!(branches(&user)
            .remote
            .iter()
            .all(|branch| branch.name != "origin/doomed"));
    }

    /// A remote name may contain `/`, and a branch may track a local branch instead of a remote.
    /// The remote comes from `branch.<name>.remote`, not from splitting the upstream short name.
    #[test]
    fn fetch_resolves_the_remote_from_the_branch_configuration() {
        let (temp, user, bare) = remote_repo("user");
        let peer = peer_clone(temp.path(), &bare);
        publish(&peer, "b.txt", "one\npeer\n", "peer change");
        let slashed = temp.path().join("team-origin.git");
        git(
            temp.path(),
            &["init", "-q", "--bare", "-b", "main", "team-origin.git"],
        );
        git(
            &user,
            &["remote", "add", "team/origin", slashed.to_str().unwrap()],
        );
        git(&user, &["config", "branch.main.remote", "team/origin"]);

        let resolved = fetch_remote(&repo_of(&user), &status(&user));

        assert_eq!(resolved.as_deref(), Some("team/origin"));
        // A local upstream (`.`) is not a remote: the first configured one is used instead.
        git(&user, &["config", "branch.main.remote", "."]);
        assert_eq!(
            fetch_remote(&repo_of(&user), &status(&user)).as_deref(),
            Some("origin")
        );
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

    /// `git status` degrades silently when it cannot take the index lock (measured with a held
    /// `.git/index.lock`, a read-only index and a read-only `.git`: exit 0 every time), so the
    /// child's environment is the only deterministic observation of what the host asked for. A
    /// shim first on `PATH` records `GIT_OPTIONAL_LOCKS` and execs the real git.
    #[cfg(unix)]
    #[test]
    #[ignore = "isolated leaf: it rewrites PATH for its own process"]
    fn git_runs_do_not_take_the_optional_index_lock_isolated() {
        use crate::test_harness::EnvVar;
        use std::os::unix::fs::PermissionsExt;

        let real = path().expect("git is on PATH");
        let (_temp, root) = init("repo");
        fs::write(root.join("a.txt"), "one\n").unwrap();
        commit_all(&root, "initial");

        let shim_dir = tempfile::tempdir().unwrap();
        let record = shim_dir.path().join("optional-locks.txt");
        let shim = shim_dir.path().join("git");
        fs::write(
            &shim,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"${{GIT_OPTIONAL_LOCKS-}}\" >> '{}'\nexec '{}' \"$@\"\n",
                record.display(),
                real
            ),
        )
        .unwrap();
        fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).unwrap();

        let old_path = std::env::var("PATH").unwrap_or_default();
        let _path = EnvVar::set("PATH", format!("{}:{old_path}", shim_dir.path().display()));

        assert_eq!(status(&root).state, "ok");
        assert_eq!(info().state, "found");

        let recorded = fs::read_to_string(&record).unwrap();
        let values: Vec<&str> = recorded.lines().collect();
        assert!(!values.is_empty(), "the shim git never ran");
        assert!(
            values.iter().all(|value| *value == "0"),
            "GIT_OPTIONAL_LOCKS reached git as {values:?}"
        );
    }

    #[test]
    fn masked_output_hides_credentials() {
        let text =
            "fatal: unable to access 'https://user:secret-token@example.com/repo.git/': denied";
        let masked = mask_urls(text);
        assert!(!masked.contains("secret-token"), "{masked}");
        assert!(!masked.contains("user:"), "{masked}");
        assert!(
            masked.contains("https://***@example.com/repo.git/"),
            "{masked}"
        );
        assert_eq!(mask_urls("plain text"), "plain text");
    }
}

#[cfg(unix)]
#[test]
fn git_runs_do_not_take_the_optional_index_lock() {
    crate::test_harness::run_isolated_leaf(
        "git::tests::git_runs_do_not_take_the_optional_index_lock_isolated",
    );
}
