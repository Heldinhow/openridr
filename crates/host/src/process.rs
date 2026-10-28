use serde::Serialize;
use serde_json::{json, Value};
use std::io;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const GRACE: Duration = Duration::from_secs(2);
const AFTER_KILL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Termination {
    requested: String,
    observed: String,
    exit: Option<i32>,
    killed_after_grace: bool,
}

struct Entry {
    id: u64,
    kind: String,
    pid: u32,
    workspace_id: u64,
    child: Option<Child>,
    exited: bool,
    exit: Option<i32>,
    awaiting_decision: bool,
    kept: bool,
    termination: Option<Termination>,
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
            "awaitingDecision": self.awaiting_decision,
            "kept": self.kept,
            "termination": self.termination,
        })
    }

    fn refresh(&mut self) {
        if self.exited {
            return;
        }
        let Some(child) = self.child.as_mut() else {
            return;
        };
        if let Ok(Some(status)) = child.try_wait() {
            self.exited = true;
            self.exit = status.code();
            // Once the host has observed the exit there is no termination decision left.
            self.awaiting_decision = false;
        }
    }

    fn running(&self) -> bool {
        !self.exited
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
        let id = self.next_id;
        self.next_id += 1;
        let entry = Entry {
            id,
            kind: kind.to_string(),
            pid: child.id(),
            workspace_id,
            child: Some(child),
            exited: false,
            exit: None,
            awaiting_decision: false,
            kept: false,
            termination: None,
        };
        let view = entry.view();
        self.entries.push(entry);
        Ok(view)
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
            self.signal(index, false);
            let mut killed_after_grace = false;
            self.wait_until_exited(index, GRACE);
            if self.entries[index].running() {
                killed_after_grace = true;
                self.signal(index, true);
                self.wait_until_exited(index, AFTER_KILL);
            }
            let entry = &mut self.entries[index];
            entry.termination = Some(Termination {
                requested: if killed_after_grace {
                    "SIGTERM then SIGKILL".to_string()
                } else {
                    "SIGTERM".to_string()
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
    fn signal(&mut self, index: usize, hard: bool) {
        let signal = if hard { libc::SIGKILL } else { libc::SIGTERM };
        let pid = self.entries[index].pid as i32;
        // Each child is spawned in its own process group, so signal the whole group.
        let group = unsafe { libc::kill(-pid, signal) };
        if group != 0 {
            unsafe {
                libc::kill(pid, signal);
            }
        }
    }

    #[cfg(not(unix))]
    fn signal(&mut self, index: usize, _hard: bool) {
        // Windows has no graceful SIGTERM equivalent here: TerminateProcess is reported as
        // an observed kill and remains a documented limitation for this slice.
        if let Some(child) = self.entries[index].child.as_mut() {
            let _ = child.kill();
        }
    }
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
