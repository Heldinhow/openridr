//! One real PTY shell for the trusted workspace.
//!
//! The shell is discovered (never assumed) and started with an explicit cwd; input is raw
//! bytes written to the PTY master, so Ctrl+C (0x03) reaches the foreground process group.
//! Output is drained on its own thread into a bounded buffer that survives exit/close. The
//! shell child lives in the shared registry, so termination and workspace boundaries report
//! what was observed instead of claiming a clean exit.

use crate::error::{HostError, HostResult};
use crate::process::{ProcessStatus, Termination};
use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::collections::VecDeque;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_COLS: u16 = 400;
pub const MAX_ROWS: u16 = 200;
pub const DEFAULT_COLS: u16 = 80;
pub const DEFAULT_ROWS: u16 = 24;
const MIN_COLS: u16 = 20;
const MIN_ROWS: u16 = 5;

const MAX_LINES: usize = 4_000;
const MAX_CHARS: usize = 120_000;
/// Raw PTY bytes kept for `terminal.read`; the UI emulates the screen, so it needs the bytes as sent.
const MAX_RAW_BYTES: usize = 1 << 20;

/// Picks the shell that is actually available. `$SHELL`/`COMSPEC` counts only when it is an
/// absolute, executable file; otherwise the first usable platform default wins.
pub fn discover_shell() -> HostResult<PathBuf> {
    let from_env = env_shell();
    let candidates = platform_candidates();
    pick_shell(from_env.as_deref(), &candidates, is_usable).ok_or_else(|| {
        HostError::new(
            "shell_missing",
            "No usable shell was found; the terminal was not started. Point SHELL at an absolute executable path.",
        )
    })
}

fn env_shell() -> Option<String> {
    #[cfg(windows)]
    {
        std::env::var("COMSPEC").ok()
    }
    #[cfg(not(windows))]
    {
        std::env::var("SHELL").ok()
    }
}

fn pick_shell(
    from_env: Option<&str>,
    candidates: &[PathBuf],
    usable: impl Fn(&Path) -> bool,
) -> Option<PathBuf> {
    if let Some(shell) = from_env {
        let path = Path::new(shell);
        if path.is_absolute() && usable(path) {
            return Some(path.to_path_buf());
        }
    }
    candidates.iter().find(|path| usable(path)).cloned()
}

#[cfg(unix)]
fn platform_candidates() -> Vec<PathBuf> {
    ["/bin/zsh", "/bin/bash", "/bin/sh"]
        .iter()
        .map(PathBuf::from)
        .collect()
}

#[cfg(windows)]
fn platform_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(root) = std::env::var("SystemRoot") {
        let root = PathBuf::from(root);
        candidates.push(root.join("System32/WindowsPowerShell/v1.0/powershell.exe"));
        candidates.push(root.join("System32/cmd.exe"));
    }
    candidates.push(PathBuf::from("cmd.exe"));
    candidates
}

#[cfg(unix)]
fn is_usable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_usable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|meta| meta.is_file())
        .unwrap_or(false)
}

/// Bounded tail of the shell output. `seq` advances per chunk so a poller can tell that new
/// output arrived without diffing the whole text.
#[derive(Default)]
struct Output {
    lines: VecDeque<String>,
    chars: usize,
    pending: String,
    dropped: bool,
    seq: u64,
    raw: VecDeque<u8>,
    /// Absolute offset of `raw[0]`; bytes before it were dropped to respect `MAX_RAW_BYTES`.
    raw_base: u64,
}

/// Raw bytes from `from` (absolute offset) to `end`. `truncated` means the caller asked for
/// bytes that were already dropped, so the chunk starts at the oldest byte still kept.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RawChunk {
    pub from: u64,
    pub end: u64,
    pub truncated: bool,
    pub bytes: Vec<u8>,
}

impl Output {
    fn push(&mut self, bytes: &[u8]) {
        self.seq += 1;
        self.raw.extend(bytes);
        if self.raw.len() > MAX_RAW_BYTES {
            let excess = self.raw.len() - MAX_RAW_BYTES;
            self.raw.drain(..excess);
            self.raw_base += excess as u64;
        }
        let raw = String::from_utf8_lossy(bytes);
        let text = strip_ansi(&raw).replace("\r\n", "\n");
        self.pending.push_str(&text);
        while let Some(index) = self.pending.find('\n') {
            let line: String = self.pending.drain(..=index).collect();
            self.push_line(line.trim_end_matches('\n').to_string());
        }
        // shortcut: an unterminated progress line is capped in place, not line-split; upgrade
        // if a real full-screen TUI must be reproduced instead of plain shell output.
        if self.pending.chars().count() > MAX_CHARS {
            let tail: String = self
                .pending
                .chars()
                .rev()
                .take(MAX_CHARS)
                .collect::<Vec<char>>()
                .into_iter()
                .rev()
                .collect();
            self.pending = tail;
            self.dropped = true;
        }
    }

    fn push_line(&mut self, line: String) {
        self.chars += line.chars().count() + 1;
        self.lines.push_back(line);
        while self.lines.len() > MAX_LINES || self.chars > MAX_CHARS {
            let Some(front) = self.lines.pop_front() else {
                break;
            };
            self.chars = self.chars.saturating_sub(front.chars().count() + 1);
            self.dropped = true;
        }
    }

    fn read_raw(&self, since: u64) -> RawChunk {
        let end = self.raw_base + self.raw.len() as u64;
        let from = since.clamp(self.raw_base, end);
        let skip = (from - self.raw_base) as usize;
        RawChunk {
            from,
            end,
            truncated: since < self.raw_base,
            bytes: self.raw.iter().skip(skip).copied().collect(),
        }
    }

    fn text(&self) -> String {
        let mut text = String::new();
        if self.dropped {
            text.push_str("… log truncated: earlier output was not kept …\n");
        }
        for line in &self.lines {
            text.push_str(line);
            text.push('\n');
        }
        text.push_str(&self.pending);
        text.trim_end().to_string()
    }
}

/// Removes terminal control sequences (colors, cursor moves, paste markers) because this view
/// shows plain text, not a screen emulator. Printable shell text is kept exactly.
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\u{1b}' {
            match chars.next() {
                Some('[') => {
                    for next in chars.by_ref() {
                        if ('@'..='~').contains(&next) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    for next in chars.by_ref() {
                        if next == '\u{7}' {
                            break;
                        }
                        if next == '\u{1b}' {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                Some('(') | Some(')') => {
                    chars.next();
                }
                Some(_) | None => {}
            }
            continue;
        }
        if character == '\n' || character == '\r' || character == '\t' || !character.is_control() {
            out.push(character);
        }
    }
    out
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalView {
    pub state: &'static str,
    pub shell: String,
    pub command: Vec<String>,
    pub cwd: String,
    pub cols: u16,
    pub rows: u16,
    pub log: String,
    pub log_truncated: bool,
    pub output_seq: u64,
    pub exit: Option<i32>,
    pub termination: Option<Termination>,
    pub process_id: Option<u64>,
    pub awaiting_decision: bool,
    pub kept: bool,
    pub revision: u64,
    pub workspace_id: Option<u64>,
    pub context_id: Option<u64>,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub stale_reason: Option<String>,
    pub sealed: bool,
}

pub struct Start {
    pub revision: u64,
    pub shell: PathBuf,
    pub cwd: PathBuf,
    pub cols: u16,
    pub rows: u16,
    pub workspace_id: u64,
    pub context_id: u64,
}

impl Clone for Start {
    fn clone(&self) -> Self {
        Self {
            revision: self.revision,
            shell: self.shell.clone(),
            cwd: self.cwd.clone(),
            cols: self.cols,
            rows: self.rows,
            workspace_id: self.workspace_id,
            context_id: self.context_id,
        }
    }
}

pub struct Attachment {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    pid: u32,
}

/// A spawned shell and the handles the host must split between the registry and the drain
/// threads. Nothing here is inferred: `pid` is the child the PTY actually started.
pub struct Session {
    pub terminal: Terminal,
    pub pid: u32,
    pub reader: Box<dyn Read + Send>,
    pub killer: Box<dyn ChildKiller + Send + Sync>,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}

/// Opens the PTY and spawns the discovered shell in it. On Unix the child becomes a session
/// leader with the PTY as its controlling terminal, so 0x03 reaches its foreground group.
pub fn spawn(start: Start) -> HostResult<Session> {
    let pair = portable_pty::native_pty_system()
        .openpty(PtySize {
            rows: start.rows,
            cols: start.cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| {
            HostError::new("operation_failed", format!("Could not open a PTY: {error}"))
        })?;
    let child = pair
        .slave
        .spawn_command(command(&start.shell, &start.cwd))
        .map_err(|error| {
            HostError::new(
                "operation_failed",
                format!(
                    "Could not start {} in {}: {error}",
                    start.shell.to_string_lossy(),
                    start.cwd.to_string_lossy()
                ),
            )
        })?;
    drop(pair.slave);
    let pid = child.process_id().ok_or_else(|| {
        HostError::new(
            "operation_failed",
            "The shell started without an observable process id.",
        )
    })?;
    let reader = pair.master.try_clone_reader().map_err(|error| {
        HostError::new(
            "operation_failed",
            format!("Could not read the PTY: {error}"),
        )
    })?;
    let writer = pair.master.take_writer().map_err(|error| {
        HostError::new(
            "operation_failed",
            format!("Could not write the PTY: {error}"),
        )
    })?;
    let killer = child.clone_killer();
    let terminal = Terminal::begin(
        start,
        Attachment {
            master: pair.master,
            writer,
            pid,
        },
    );
    Ok(Session {
        terminal,
        pid,
        reader,
        killer,
        child,
    })
}

struct Data {
    revision: u64,
    state: &'static str,
    shell: String,
    command: Vec<String>,
    cwd: String,
    cols: u16,
    rows: u16,
    output: Output,
    exit: Option<i32>,
    termination: Option<Termination>,
    process_id: Option<u64>,
    awaiting_decision: bool,
    kept: bool,
    workspace_id: u64,
    context_id: u64,
    started_at: Option<u64>,
    finished_at: Option<u64>,
    stale_reason: Option<String>,
    sealed: bool,
    attachment: Option<Attachment>,
}

pub struct Terminal {
    data: Mutex<Data>,
}

impl Terminal {
    pub fn idle() -> Self {
        Self {
            data: Mutex::new(Data {
                revision: 0,
                state: "idle",
                shell: String::new(),
                command: Vec::new(),
                cwd: String::new(),
                cols: DEFAULT_COLS,
                rows: DEFAULT_ROWS,
                output: Output::default(),
                exit: None,
                termination: None,
                process_id: None,
                awaiting_decision: false,
                kept: false,
                workspace_id: 0,
                context_id: 0,
                started_at: None,
                finished_at: None,
                stale_reason: None,
                sealed: false,
                attachment: None,
            }),
        }
    }

    pub fn begin(start: Start, attachment: Attachment) -> Self {
        let shell = start.shell.to_string_lossy().to_string();
        Self {
            data: Mutex::new(Data {
                revision: start.revision,
                state: "running",
                command: vec![shell.clone()],
                shell,
                cwd: start.cwd.to_string_lossy().to_string(),
                cols: start.cols,
                rows: start.rows,
                output: Output::default(),
                exit: None,
                termination: None,
                process_id: None,
                awaiting_decision: false,
                kept: false,
                workspace_id: start.workspace_id,
                context_id: start.context_id,
                started_at: Some(now_ms()),
                finished_at: None,
                stale_reason: None,
                sealed: false,
                attachment: Some(attachment),
            }),
        }
    }

    pub fn failed(start: Start, message: String) -> Self {
        let shell = start.shell.to_string_lossy().to_string();
        let mut output = Output::default();
        output.push_line(message);
        Self {
            data: Mutex::new(Data {
                revision: start.revision,
                state: "failed",
                command: vec![shell.clone()],
                shell,
                cwd: start.cwd.to_string_lossy().to_string(),
                cols: start.cols,
                rows: start.rows,
                output,
                exit: None,
                termination: None,
                process_id: None,
                awaiting_decision: false,
                kept: false,
                workspace_id: start.workspace_id,
                context_id: start.context_id,
                started_at: Some(now_ms()),
                finished_at: Some(now_ms()),
                stale_reason: None,
                sealed: false,
                attachment: None,
            }),
        }
    }

    /// Raw PTY bytes after the absolute offset `since`, exactly as the shell wrote them, with the
    /// revision that owns those offsets.
    pub fn read(&self, since: u64) -> (u64, RawChunk) {
        let data = self.data.lock().unwrap();
        (data.revision, data.output.read_raw(since))
    }

    pub fn view(&self) -> TerminalView {
        let data = self.data.lock().unwrap();
        TerminalView {
            state: data.state,
            shell: data.shell.clone(),
            command: data.command.clone(),
            cwd: data.cwd.clone(),
            cols: data.cols,
            rows: data.rows,
            log: data.output.text(),
            log_truncated: data.output.dropped,
            output_seq: data.output.seq,
            exit: data.exit,
            termination: data.termination.clone(),
            process_id: data.process_id,
            awaiting_decision: data.awaiting_decision,
            kept: data.kept,
            revision: data.revision,
            workspace_id: (data.revision > 0).then_some(data.workspace_id),
            context_id: (data.revision > 0).then_some(data.context_id),
            started_at: data.started_at,
            finished_at: data.finished_at,
            stale_reason: data.stale_reason.clone(),
            sealed: data.sealed,
        }
    }

    pub fn note_process(&self, id: u64) {
        let mut data = self.data.lock().unwrap();
        if data.state == "running" && !data.sealed {
            data.process_id = Some(id);
        }
    }

    pub fn running_process_id(&self) -> Option<u64> {
        let data = self.data.lock().unwrap();
        if data.state == "running" && !data.sealed {
            data.process_id
        } else {
            None
        }
    }

    /// Drains output even after a seal so the child can never block on a full PTY buffer; a
    /// sealed record simply ignores the bytes.
    pub fn append(&self, bytes: &[u8]) {
        let mut data = self.data.lock().unwrap();
        if data.sealed {
            return;
        }
        data.output.push(bytes);
    }

    /// Raw bytes, exactly as typed. 0x03 is a real interrupt for the foreground process.
    pub fn write(&self, text: &str) -> HostResult<()> {
        let mut data = self.data.lock().unwrap();
        if data.state != "running" || data.sealed {
            return Err(HostError::new(
                "invalid_state",
                "The shell is not running; no input was sent.",
            ));
        }
        let Some(attachment) = data.attachment.as_mut() else {
            return Err(HostError::new(
                "invalid_state",
                "The shell is not attached to a PTY; no input was sent.",
            ));
        };
        attachment
            .writer
            .write_all(text.as_bytes())
            .and_then(|_| attachment.writer.flush())
            .map_err(|error| {
                HostError::new(
                    "operation_failed",
                    format!("Could not write to the shell: {error}"),
                )
            })
    }

    pub fn resize(&self, cols: u16, rows: u16) -> HostResult<()> {
        let mut data = self.data.lock().unwrap();
        if data.state != "running" || data.sealed {
            return Err(HostError::new(
                "invalid_state",
                "The shell is not running; the PTY was not resized.",
            ));
        }
        let Some(attachment) = data.attachment.as_ref() else {
            return Err(HostError::new(
                "invalid_state",
                "The shell is not attached to a PTY; the PTY was not resized.",
            ));
        };
        attachment
            .master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| {
                HostError::new(
                    "operation_failed",
                    format!("Could not resize the PTY: {error}"),
                )
            })?;
        data.cols = cols;
        data.rows = rows;
        Ok(())
    }

    /// Best effort before the group is signalled: a foreground job (job control) may live in
    /// its own process group, which the shell's group signal would not reach.
    pub fn signal_foreground(&self) {
        #[cfg(unix)]
        {
            let data = self.data.lock().unwrap();
            let Some(attachment) = data.attachment.as_ref() else {
                return;
            };
            if let Some(group) = attachment.master.process_group_leader() {
                if group > 0 && group != attachment.pid as i32 {
                    unsafe {
                        libc::kill(-group, libc::SIGTERM);
                    }
                }
            }
        }
    }

    pub fn complete(&self, status: &ProcessStatus) {
        let mut data = self.data.lock().unwrap();
        if data.state != "running" || data.sealed {
            return;
        }
        data.exit = status.exit;
        if let Some(termination) = &status.termination {
            data.termination = Some(termination.clone());
        }
        data.awaiting_decision = false;
        // A host-requested termination is "stopped"; a shell that ended by itself is "exited".
        data.state = if data.termination.is_some() {
            "stopped"
        } else {
            "exited"
        };
        data.finished_at = Some(now_ms());
    }

    pub fn observe_decision(&self, awaiting: bool) {
        let mut data = self.data.lock().unwrap();
        if data.state == "running" && !data.sealed {
            data.awaiting_decision = awaiting;
        }
    }

    pub fn keep(&self, process_id: u64) {
        let mut data = self.data.lock().unwrap();
        if data.process_id == Some(process_id) {
            data.kept = true;
            data.awaiting_decision = false;
        }
    }

    /// A workspace boundary seals the record: late output or a late exit cannot attach to the
    /// new workspace. The termination shown is the one the registry observed.
    pub fn seal(&self, reason: &str, status: Option<&ProcessStatus>) {
        let mut data = self.data.lock().unwrap();
        if data.revision == 0 {
            return;
        }
        data.sealed = true;
        data.stale_reason = Some(reason.to_string());
        if data.state != "running" {
            return;
        }
        if let Some(status) = status {
            data.exit = status.exit;
            data.termination = status.termination.clone();
            if status.state == "exited" {
                data.state = "stopped";
                data.awaiting_decision = false;
                data.finished_at = Some(now_ms());
            } else {
                data.awaiting_decision = true;
            }
        }
    }
}

/// Reads the PTY until EOF. The shell exit itself is observed by the waiter, not inferred here.
pub fn pump(mut reader: impl Read, terminal: Arc<Terminal>) {
    let mut buffer = [0u8; 8_192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => terminal.append(&buffer[..read]),
            Err(_) => break,
        }
    }
}

/// CommandBuilder for the discovered shell with the workspace root as cwd and a real TERM.
pub fn command(shell: &Path, cwd: &Path) -> CommandBuilder {
    let mut builder = CommandBuilder::new(shell.as_os_str());
    builder.cwd(cwd);
    if term_needs_override(std::env::var_os("TERM").as_deref()) {
        builder.env("TERM", "xterm-256color");
    }
    builder
}

/// The emulator is xterm.js, so a missing, empty or `dumb` TERM inherited from the launcher is replaced.
fn term_needs_override(term: Option<&std::ffi::OsStr>) -> bool {
    term.map_or(true, |value| value.is_empty() || value == "dumb")
}

pub fn clamp_size(cols: Option<u64>, rows: Option<u64>) -> (u16, u16) {
    let cols = cols
        .unwrap_or(DEFAULT_COLS as u64)
        .clamp(MIN_COLS as u64, MAX_COLS as u64);
    let rows = rows
        .unwrap_or(DEFAULT_ROWS as u64)
        .clamp(MIN_ROWS as u64, MAX_ROWS as u64);
    (cols as u16, rows as u16)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dumb_or_missing_term_is_replaced_but_a_real_one_is_kept() {
        use std::ffi::OsStr;
        assert!(term_needs_override(None));
        assert!(term_needs_override(Some(OsStr::new(""))));
        assert!(term_needs_override(Some(OsStr::new("dumb"))));
        assert!(!term_needs_override(Some(OsStr::new("xterm-256color"))));
        assert!(!term_needs_override(Some(OsStr::new("screen"))));
    }

    fn existing(names: &[&'static str]) -> impl Fn(&Path) -> bool {
        let names: Vec<&'static str> = names.to_vec();
        move |path: &Path| names.iter().any(|name| path == Path::new(name))
    }

    #[test]
    fn shell_discovery_prefers_an_absolute_executable_env_value() {
        let candidates = vec![PathBuf::from("/bin/zsh"), PathBuf::from("/bin/sh")];
        assert_eq!(
            pick_shell(
                Some("/opt/custom/fish"),
                &candidates,
                existing(&["/opt/custom/fish"])
            ),
            Some(PathBuf::from("/opt/custom/fish"))
        );
    }

    #[test]
    fn shell_discovery_ignores_a_relative_or_missing_env_shell() {
        let candidates = vec![PathBuf::from("/bin/zsh"), PathBuf::from("/bin/sh")];
        let usable = existing(&["/bin/sh", "bash"]);
        assert_eq!(
            pick_shell(Some("zsh"), &candidates, &usable),
            Some(PathBuf::from("/bin/sh"))
        );
        assert_eq!(
            pick_shell(Some("/opt/missing"), &candidates, &usable),
            Some(PathBuf::from("/bin/sh"))
        );
        assert_eq!(
            pick_shell(None, &[PathBuf::from("/bin/bash")], &usable),
            None
        );
    }

    #[test]
    fn output_keeps_the_tail_and_a_monotonic_sequence() {
        let mut output = Output::default();
        output.push(b"one\ntwo\r\n");
        assert_eq!(output.seq, 1);
        assert_eq!(output.text(), "one\ntwo");
        output.push(b"progress-before-CR\rprogress-after-CR\n");
        assert_eq!(
            output.text(),
            "one\ntwo\nprogress-before-CR\rprogress-after-CR"
        );
        assert_eq!(output.seq, 2);
        output.push(b"partial ");
        assert_eq!(output.seq, 3);
        assert_eq!(
            output.text(),
            "one\ntwo\nprogress-before-CR\rprogress-after-CR\npartial"
        );
        for index in 0..(MAX_LINES + 10) {
            output.push(format!("line {index}\n").as_bytes());
        }
        let text = output.text();
        assert!(output.dropped);
        assert!(text.starts_with("… log truncated"));
        assert!(text.ends_with(&format!("line {}", MAX_LINES + 9)));
    }

    #[test]
    fn control_sequences_are_not_part_of_the_visible_output() {
        assert_eq!(
            strip_ansi("\u{1b}[1mhi\u{1b}[0m \u{1b}]0;title\u{7}there\n"),
            "hi there\n"
        );
        assert_eq!(strip_ansi("plain text"), "plain text");
    }

    #[test]
    fn sizes_are_clamped_and_defaults_are_explicit() {
        assert_eq!(clamp_size(None, None), (DEFAULT_COLS, DEFAULT_ROWS));
        assert_eq!(clamp_size(Some(0), Some(0)), (MIN_COLS, MIN_ROWS));
        assert_eq!(clamp_size(Some(99_999), Some(99_999)), (MAX_COLS, MAX_ROWS));
        assert_eq!(clamp_size(Some(120), Some(40)), (120, 40));
    }

    #[test]
    fn an_idle_session_spawns_nothing_and_hides_identity() {
        let view = Terminal::idle().view();
        assert_eq!(view.state, "idle");
        assert_eq!(view.revision, 0);
        assert_eq!(view.process_id, None);
        assert!(view.shell.is_empty());
    }

    fn running() -> Terminal {
        Terminal {
            data: Mutex::new(Data {
                revision: 1,
                state: "running",
                shell: "/bin/sh".to_string(),
                command: vec!["/bin/sh".to_string()],
                cwd: "/work".to_string(),
                cols: 80,
                rows: 24,
                output: Output::default(),
                exit: None,
                termination: None,
                process_id: Some(7),
                awaiting_decision: false,
                kept: false,
                workspace_id: 1,
                context_id: 2,
                started_at: Some(1),
                finished_at: None,
                stale_reason: None,
                sealed: false,
                attachment: None,
            }),
        }
    }

    fn status(
        state: &'static str,
        exit: Option<i32>,
        termination: Option<Termination>,
        awaiting_decision: bool,
    ) -> ProcessStatus {
        ProcessStatus {
            state,
            exit,
            signal: None,
            termination,
            awaiting_decision,
        }
    }

    #[test]
    fn exit_without_a_code_stays_unknown_a_stop_is_reported_and_a_seal_drops_late_output() {
        let exited = running();
        exited.append(b"before close\n");
        exited.complete(&status("exited", None, None, false));
        let view = exited.view();
        assert_eq!(view.state, "exited");
        assert_eq!(view.exit, None);
        assert!(view.log.contains("before close"));
        assert!(view.finished_at.is_some());

        let stopped = running();
        let termination = Termination {
            requested: "SIGTERM".to_string(),
            observed: "exited".to_string(),
            exit: Some(143),
            killed_after_grace: false,
        };
        stopped.complete(&status("exited", Some(143), Some(termination), false));
        let view = stopped.view();
        assert_eq!(view.state, "stopped");
        assert_eq!(view.exit, Some(143));
        assert_eq!(view.termination.unwrap().observed, "exited");
    }

    #[test]
    fn a_sealed_record_keeps_its_log_and_never_takes_a_late_line() {
        let terminal = running();
        terminal.append(b"kept\n");
        let still_running = Termination {
            requested: "SIGTERM".to_string(),
            observed: "still running".to_string(),
            exit: None,
            killed_after_grace: true,
        };
        terminal.seal(
            "Trust was revoked.",
            Some(&status("running", None, Some(still_running), true)),
        );
        let view = terminal.view();
        assert!(view.sealed);
        assert_eq!(view.stale_reason.as_deref(), Some("Trust was revoked."));
        assert!(view.awaiting_decision);
        assert!(view.log.contains("kept"));
        terminal.append(b"late\n");
        let later = terminal.view();
        assert!(!later.log.contains("late"));
        assert_eq!(later.finished_at, None);
    }

    #[test]
    fn raw_read_returns_the_bytes_as_written_from_an_offset() {
        let mut output = Output::default();
        output.push(b"\x1b[31mred\x1b[0m\r\n");
        let all = output.read_raw(0);
        assert_eq!(all.bytes, b"\x1b[31mred\x1b[0m\r\n".to_vec());
        assert_eq!((all.from, all.end, all.truncated), (0, 14, false));

        output.push(b"ls");
        let next = output.read_raw(all.end);
        assert_eq!(next.bytes, b"ls".to_vec());
        assert_eq!((next.from, next.end), (14, 16));
        assert!(output.read_raw(16).bytes.is_empty());
    }

    #[test]
    fn a_full_buffer_drops_the_oldest_bytes_and_says_so() {
        let mut output = Output::default();
        output.push(&vec![b'a'; MAX_RAW_BYTES]);
        output.push(b"tail");
        let chunk = output.read_raw(0);
        assert!(chunk.truncated);
        assert_eq!((chunk.from, chunk.end), (4, (MAX_RAW_BYTES + 4) as u64));
        assert_eq!(chunk.bytes.len(), MAX_RAW_BYTES);
        assert!(chunk.bytes.ends_with(b"tail"));
    }

    #[test]
    fn raw_bytes_survive_past_the_line_cut_and_ignore_offsets_ahead_of_the_end() {
        let mut output = Output::default();
        for _ in 0..(MAX_LINES + 5) {
            output.push(b"line\n");
        }
        let chunk = output.read_raw(0);
        assert_eq!(chunk.bytes.len(), (MAX_LINES + 5) * 5);
        assert!(!chunk.truncated);
        let ahead = output.read_raw(10_000_000);
        assert_eq!((ahead.from, ahead.end), (ahead.end, ahead.end));
        assert!(ahead.bytes.is_empty());
    }
}
