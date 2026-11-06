use portable_pty::ChildKiller;
use serde::Serialize;
use serde_json::{json, Value};
use std::io;
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Where the PTY waiter thread publishes the exit it observed. `None` means the shell has not
/// exited; `Some(None)` means it exited without a code the host could read.
pub type ExitSlot = Arc<Mutex<Option<Option<i32>>>>;

const GRACE: Duration = Duration::from_secs(2);
const AFTER_KILL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Termination {
    pub requested: String,
    pub observed: String,
    pub exit: Option<i32>,
    pub killed_after_grace: bool,
}

/// What the registry observed for one tracked process, without inventing a clean exit.
#[derive(Debug, Clone)]
pub struct ProcessStatus {
    pub state: &'static str,
    pub exit: Option<i32>,
    pub signal: Option<i32>,
    pub termination: Option<Termination>,
    pub awaiting_decision: bool,
}

/// A tracked child whose output the caller streams. The child stays in the registry, so
/// termination/awaiting-decision semantics are the same as every other process.
pub struct PipedChild {
    pub id: u64,
    pub pid: u32,
    pub stdin: Option<ChildStdin>,
    pub stdout: Option<ChildStdout>,
    pub stderr: Option<ChildStderr>,
}

struct Entry {
    id: u64,
    kind: String,
    pid: u32,
    workspace_id: u64,
    child: Option<Child>,
    /// A PTY shell is waited on by its own thread; the registry reads that result here.
    external_exit: Option<ExitSlot>,
    killer: Option<Box<dyn ChildKiller + Send + Sync>>,
    exited: bool,
    exit: Option<i32>,
    signal: Option<i32>,
    awaiting_decision: bool,
    kept: bool,
    termination: Option<Termination>,
    /// Unix waits without reaping until the whole group has been terminated; this keeps the
    /// pid/pgid reserved so the cleanup cannot hit a recycled group.
    reaped: bool,
}

impl Entry {
    fn view(&self) -> Value {
        json!({
            "id": self.id,
            "kind": self.kind,
            "pid": self.pid,
            "workspaceId": self.workspace_id,
            "state": if self.exited { "exited" } else { "running" },
            "exit": self.exit,
            "signal": self.signal,
            "awaitingDecision": self.awaiting_decision,
            "kept": self.kept,
            "termination": self.termination,
        })
    }

    fn refresh(&mut self) {
        if self.exited {
            return;
        }
        if self.child.is_some() {
            #[cfg(unix)]
            {
                // WNOWAIT observes the exit without reaping the leader. The pid (and therefore
                // the pgid) stays reserved until `finish_group` kills every descendant and
                // `reap` releases it, so a recycled pid can never be mistaken for our group.
                let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
                let observed = unsafe {
                    libc::waitid(
                        libc::P_PID,
                        self.pid as libc::id_t,
                        &mut info,
                        libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                    )
                };
                if observed == 0 && info.si_pid == self.pid as libc::pid_t {
                    self.exited = true;
                    match info.si_code {
                        libc::CLD_EXITED => self.exit = Some(info.si_status),
                        libc::CLD_KILLED | libc::CLD_DUMPED => self.signal = Some(info.si_status),
                        _ => {}
                    }
                    self.awaiting_decision = false;
                    self.finish_group();
                    self.reap();
                }
                return;
            }
            #[cfg(not(unix))]
            {
                if let Some(child) = self.child.as_mut() {
                    if let Ok(Some(status)) = child.try_wait() {
                        self.exited = true;
                        self.exit = status.code();
                        self.signal = exit_signal(&status);
                        // Once the host has observed the exit there is no termination decision left.
                        self.awaiting_decision = false;
                    }
                }
                return;
            }
        }
        if let Some(slot) = &self.external_exit {
            if let Some(code) = *slot.lock().unwrap() {
                self.exited = true;
                self.exit = code;
                self.awaiting_decision = false;
            }
        }
    }

    fn running(&self) -> bool {
        !self.exited
    }

    /// The leader is a zombie here, so its pid (and pgid) cannot be reused yet. Terminate any
    /// grandchildren, escalate to KILL if the group survives, then release the leader.
    #[cfg(unix)]
    fn finish_group(&mut self) {
        // ponytail: the leader is still a zombie here, so the kernel cannot reuse its pid/pgid;
        // this closes the practical race. A future design that reaps before group cleanup would
        // need an ownership token, because pgid reuse after reap is theoretically possible.
        let pgid = self.pid;
        if pgid == 0 || self.killer.is_some() {
            return;
        }
        let group = |signal: libc::c_int| unsafe { libc::kill(-(pgid as i32), signal) };
        if group(0) != 0 {
            return;
        }
        let _ = group(libc::SIGTERM);
        let deadline = Instant::now() + Duration::from_millis(300);
        while Instant::now() < deadline && group(0) == 0 {
            std::thread::sleep(Duration::from_millis(10));
        }
        if group(0) == 0 {
            let _ = group(libc::SIGKILL);
        }
    }

    #[cfg(not(unix))]
    fn finish_group(&mut self) {}

    fn reap(&mut self) {
        if self.reaped {
            return;
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.try_wait();
        }
        self.reaped = true;
    }
}

pub struct Registry {
    entries: Vec<Entry>,
    next_id: u64,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }

    pub fn spawn(
        &mut self,
        kind: &str,
        workspace_id: u64,
        mut command: Command,
    ) -> io::Result<Value> {
        command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let child = command.spawn()?;
        let id = self.register(kind, workspace_id, child);
        Ok(self
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .expect("just registered")
            .view())
    }

    /// Same tracked process, but stdin/stdout/stderr are piped to the caller. Restore
    /// ignores stdin; the LSP client writes protocol frames there.
    pub fn spawn_piped(
        &mut self,
        kind: &str,
        workspace_id: u64,
        mut command: Command,
    ) -> io::Result<PipedChild> {
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        let mut child = command.spawn()?;
        let pid = child.id();
        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let id = self.register(kind, workspace_id, child);
        Ok(PipedChild {
            id,
            pid,
            stdin,
            stdout,
            stderr,
        })
    }

    fn register(&mut self, kind: &str, workspace_id: u64, child: Child) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.entries.push(Entry {
            id,
            kind: kind.to_string(),
            pid: child.id(),
            workspace_id,
            child: Some(child),
            external_exit: None,
            killer: None,
            exited: false,
            exit: None,
            signal: None,
            awaiting_decision: false,
            kept: false,
            termination: None,
            reaped: false,
        });
        id
    }

    /// Registers the shell of a PTY. The caller owns the blocking wait and publishes the
    /// observed exit into the returned slot; termination/awaiting-decision stay the same as
    /// every other tracked process.
    pub fn register_pty(
        &mut self,
        kind: &str,
        workspace_id: u64,
        pid: u32,
        killer: Box<dyn ChildKiller + Send + Sync>,
    ) -> (u64, ExitSlot) {
        let id = self.next_id;
        self.next_id += 1;
        let slot = ExitSlot::default();
        self.entries.push(Entry {
            id,
            kind: kind.to_string(),
            pid,
            workspace_id,
            child: None,
            external_exit: Some(slot.clone()),
            killer: Some(killer),
            exited: false,
            exit: None,
            signal: None,
            awaiting_decision: false,
            kept: false,
            termination: None,
            reaped: false,
        });
        (id, slot)
    }

    pub fn status(&mut self, id: u64) -> Option<ProcessStatus> {
        self.refresh();
        let entry = self.entries.iter().find(|entry| entry.id == id)?;
        Some(ProcessStatus {
            state: if entry.exited { "exited" } else { "running" },
            exit: entry.exit,
            signal: entry.signal,
            termination: entry.termination.clone(),
            awaiting_decision: entry.awaiting_decision,
        })
    }

    pub fn views(&mut self) -> Vec<Value> {
        self.refresh();
        self.entries.iter().map(Entry::view).collect()
    }

    pub fn refresh(&mut self) {
        for entry in &mut self.entries {
            entry.refresh();
        }
    }

    /// A workspace boundary (revoke, close, switch) leaves live processes visible until the
    /// user makes an explicit decision. A prior "keep" decision is not silently reset.
    pub fn mark_awaiting(&mut self) {
        self.refresh();
        for entry in &mut self.entries {
            if entry.running() && !entry.kept {
                entry.awaiting_decision = true;
            }
        }
    }

    pub fn keep(&mut self, id: u64) -> Option<Value> {
        self.refresh();
        let entry = self.entries.iter_mut().find(|entry| entry.id == id)?;
        entry.awaiting_decision = false;
        entry.kept = true;
        Some(entry.view())
    }

    /// Real termination request: TERM, wait up to ~2 s, then KILL if still alive. The
    /// returned state is what the host observed, never an assumption of cleanup.
    pub fn terminate(&mut self, id: u64) -> Option<Value> {
        self.refresh();
        let index = self.entries.iter().position(|entry| entry.id == id)?;
        if self.entries[index].running() {
            let requested = self.signal(index, false);
            let mut killed_after_grace = false;
            let mut forced = None;
            self.wait_until_exited(index, GRACE);
            if self.entries[index].running() {
                killed_after_grace = true;
                forced = Some(self.signal(index, true));
                self.wait_until_exited(index, AFTER_KILL);
            }
            let entry = &mut self.entries[index];
            entry.termination = Some(Termination {
                requested: match forced {
                    Some(forced) => format!("{requested} then {forced}"),
                    None => requested.to_string(),
                },
                observed: if entry.exited {
                    "exited".to_string()
                } else {
                    "still running".to_string()
                },
                exit: entry.exit,
                killed_after_grace,
            });
        }
        Some(self.entries[index].view())
    }

    /// Same real termination as `terminate`, reported through the typed status used by
    /// callers that must distinguish requested/observed/exit without parsing JSON.
    pub fn terminate_status(&mut self, id: u64) -> Option<ProcessStatus> {
        self.terminate(id)?;
        self.status(id)
    }

    pub fn stop_remaining_group(&mut self, id: u64) {
        #[cfg(unix)]
        if let Some(entry) = self.entries.iter().find(|entry| entry.id == id) {
            // A reaped leader's pgid can be recycled by a new process group. Only signal while
            // the leader still exists; otherwise refuse to guess which group is ours.
            if process_alive(entry.pid) {
                unsafe {
                    libc::kill(-(entry.pid as i32), libc::SIGTERM);
                }
            }
        }
        #[cfg(not(unix))]
        let _ = id;
    }

    fn wait_until_exited(&mut self, index: usize, limit: Duration) {
        let deadline = Instant::now() + limit;
        while Instant::now() < deadline {
            self.entries[index].refresh();
            if self.entries[index].exited {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[cfg(unix)]
    fn signal(&mut self, index: usize, hard: bool) -> &'static str {
        let entry = &mut self.entries[index];
        let pid = entry.pid;
        // Each child is spawned in its own process group, so signal the whole group.
        if !kill_group(pid, hard) {
            // Fallback for a child that is no longer a group leader.
            if let Some(killer) = entry.killer.as_mut() {
                let _ = killer.kill();
            }
        }
        if hard {
            "SIGKILL"
        } else {
            "SIGTERM"
        }
    }

    /// Windows has no process groups here, so the real tree is terminated by `taskkill /T`.
    /// Without `/F` it asks; with `/F` it forces. If taskkill cannot run, the registry still
    /// falls back to the child killer and says so instead of claiming a taskkill it did not do.
    #[cfg(windows)]
    fn signal(&mut self, index: usize, hard: bool) -> &'static str {
        let entry = &mut self.entries[index];
        if taskkill(entry.pid, hard) {
            return if hard {
                "taskkill /T /F"
            } else {
                "taskkill /T"
            };
        }
        if let Some(killer) = entry.killer.as_mut() {
            let _ = killer.kill();
        } else if let Some(child) = entry.child.as_mut() {
            let _ = child.kill();
        }
        "child kill (taskkill failed)"
    }

    #[cfg(not(any(unix, windows)))]
    fn signal(&mut self, index: usize, _hard: bool) -> &'static str {
        let entry = &mut self.entries[index];
        if let Some(killer) = entry.killer.as_mut() {
            let _ = killer.kill();
        } else if let Some(child) = entry.child.as_mut() {
            let _ = child.kill();
        }
        "child kill"
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(windows)]
fn taskkill(pid: u32, hard: bool) -> bool {
    let mut command = Command::new("taskkill");
    command.arg("/PID").arg(pid.to_string()).arg("/T");
    if hard {
        command.arg("/F");
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    matches!(command.status(), Ok(status) if status.success())
}

/// The signal that killed the child, if any. A normal exit reports `code()` and no signal.
#[cfg(unix)]
#[cfg(not(unix))]
fn exit_signal(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}

/// Portable signal names for numbers 1–15. These five numbers do not share a meaning between
/// macOS and Linux, so the table is chosen by OS instead of guessing one numbering.
#[cfg(target_os = "macos")]
const SIGNALS: &[(i32, &str)] = &[
    (1, "SIGHUP"),
    (2, "SIGINT"),
    (3, "SIGQUIT"),
    (4, "SIGILL"),
    (5, "SIGTRAP"),
    (6, "SIGABRT"),
    (7, "SIGEMT"),
    (8, "SIGFPE"),
    (9, "SIGKILL"),
    (10, "SIGBUS"),
    (11, "SIGSEGV"),
    (12, "SIGSYS"),
    (13, "SIGPIPE"),
    (14, "SIGALRM"),
    (15, "SIGTERM"),
];

/// Linux numbering; Windows never reports a POSIX signal, so this table is unreachable there.
#[cfg(not(target_os = "macos"))]
const SIGNALS: &[(i32, &str)] = &[
    (1, "SIGHUP"),
    (2, "SIGINT"),
    (3, "SIGQUIT"),
    (4, "SIGILL"),
    (5, "SIGTRAP"),
    (6, "SIGABRT"),
    (7, "SIGBUS"),
    (8, "SIGFPE"),
    (9, "SIGKILL"),
    (10, "SIGUSR1"),
    (11, "SIGSEGV"),
    (12, "SIGUSR2"),
    (13, "SIGPIPE"),
    (14, "SIGALRM"),
    (15, "SIGTERM"),
];

/// Short name of a POSIX signal; an unknown number stays `SIG<n>` instead of inventing a name.
pub fn signal_name(signal: i32) -> String {
    SIGNALS
        .iter()
        .find(|(number, _)| *number == signal)
        .map(|(_, name)| (*name).to_string())
        .unwrap_or_else(|| format!("SIG{signal}"))
}

/// Signals a process group that this host created. Returns false when the signal could not
/// be delivered (already gone or not permitted).
#[cfg(unix)]
pub(crate) fn kill_group(pid: u32, hard: bool) -> bool {
    if !process_alive(pid) {
        // Never signal a negative pid after its leader was reaped: that pid may be reused.
        return false;
    }
    let signal = if hard { libc::SIGKILL } else { libc::SIGTERM };
    let group = unsafe { libc::kill(-(pid as i32), signal) };
    if group == 0 {
        return true;
    }
    unsafe { libc::kill(pid as i32, signal) == 0 }
}

/// Whether the process still exists. Used only for the short kill grace of a failed LSP.
#[cfg(unix)]
pub(crate) fn process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 }
}

#[cfg(not(unix))]
pub(crate) fn kill_group(_pid: u32, _hard: bool) -> bool {
    // The registry owns child handles on Windows; a reader thread cannot signal them.
    false
}

#[cfg(not(unix))]
pub(crate) fn process_alive(_pid: u32) -> bool {
    false
}

/// Controlled children used only by the dev/test probe. No PTY is built in this ticket.
pub fn probe_command(kind: &str) -> io::Result<Command> {
    #[cfg(unix)]
    {
        let mut command = match kind {
            "sleep" => {
                let mut command = Command::new("/bin/sleep");
                command.arg("60");
                command
            }
            "ignore-term" => {
                let mut command = Command::new("/bin/sh");
                command
                    .arg("-c")
                    .arg("trap '' TERM; while true; do sleep 1; done");
                command
            }
            "quick" => {
                let mut command = Command::new("/bin/sh");
                command.arg("-c").arg("exit 3");
                command
            }
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Unknown probe kind: {other}"),
                ))
            }
        };
        command.env_clear();
        command.env("PATH", "/usr/bin:/bin");
        Ok(command)
    }
    #[cfg(not(unix))]
    {
        let _ = kind;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "processes.probe is only implemented on Unix in this slice.",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn a_leader_that_exits_does_not_leave_its_grandchild_alive() {
        let temp = tempfile::tempdir().unwrap();
        let marker = temp.path().join("grandchild-pid");
        let mut command = Command::new("/bin/sh");
        command
            .arg("-c")
            .arg("sleep 60 & echo $! > \"$1\"")
            .arg("sh")
            .arg(&marker);
        let mut registry = Registry::new();
        let process = registry.spawn("lsp:csharp-ls", 1, command).unwrap();
        let id = process["id"].as_u64().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !marker.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        let grandchild: u32 = std::fs::read_to_string(&marker)
            .unwrap()
            .trim()
            .parse()
            .unwrap();
        // Observing the leader exit runs the group cleanup, even though the leader is gone.
        let exited = loop {
            let status = registry.status(id).unwrap();
            if status.state == "exited" {
                break status;
            }
            assert!(Instant::now() < deadline, "leader did not exit");
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(exited.exit, Some(0));
        let deadline = Instant::now() + Duration::from_secs(3);
        while process_alive(grandchild) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(!process_alive(grandchild), "grandchild survived the leader");
    }

    #[test]
    fn a_signal_number_is_reported_by_its_name() {
        assert_eq!(signal_name(6), "SIGABRT");
        assert_eq!(signal_name(11), "SIGSEGV");
        assert_eq!(signal_name(999), "SIG999");
    }

    // SIGEMT/SIGBUS/SIGUSR1/SIGUSR2/SIGSYS numbers differ between macOS and Linux.
    #[cfg(target_os = "macos")]
    #[test]
    fn macos_signal_numbers_use_the_macos_names() {
        assert_eq!(signal_name(7), "SIGEMT");
        assert_eq!(signal_name(10), "SIGBUS");
        assert_eq!(signal_name(12), "SIGSYS");
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    #[test]
    fn linux_signal_numbers_use_the_linux_names() {
        assert_eq!(signal_name(7), "SIGBUS");
        assert_eq!(signal_name(10), "SIGUSR1");
        assert_eq!(signal_name(12), "SIGUSR2");
    }
}
