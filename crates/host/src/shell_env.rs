//! The login shell's environment, for a macOS app that `launchd` started.
//!
//! A Mac opened from Finder, the Dock or Spotlight is started by `launchd`, which hands the app a
//! minimal environment: nothing from `~/.zshrc` or `~/.zprofile` reaches the process. Credentials
//! for private NuGet feeds (`VSS_NUGET_EXTERNAL_FEED_ENDPOINTS`, the token a `nuget.config`
//! references) and `PATH`/`DOTNET_ROOT` tweaks live exactly there, so `dotnet restore` fails inside
//! the app and works in a terminal. [`apply`] runs the user's login shell once, reads the
//! environment it produces and merges it into this process, which every child then inherits.
//!
//! The shell runs only on macOS and only for a `launchd` launch: from a terminal the environment is
//! already the user's. Nothing here prints a variable's value.

use std::ffi::OsString;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// How long the login shell gets before the app opens with the environment it already has.
const TIMEOUT: Duration = Duration::from_secs(5);

/// How often the wait looks at the child and at the pipe.
const POLL: Duration = Duration::from_millis(10);

/// The markers the script prints around the environment, so anything a `.zshrc` prints (a banner, a
/// warning) is discarded instead of being read as a variable.
const BEGIN: &str = "__OPENRIDR_SHELL_ENV_BEGIN__";
const END: &str = "__OPENRIDR_SHELL_ENV_END__";

/// Variables that describe the helper shell rather than the user's environment: the last command
/// (`_`), the nesting level (`SHLVL`) and the directories the helper ran in (`PWD`, `OLDPWD`).
const NOT_MERGED: &[&str] = &["_", "SHLVL", "PWD", "OLDPWD"];

/// The shell to run when `SHELL` names nothing.
const FALLBACK_SHELL: &str = "/bin/zsh";

/// Merges the login shell's environment into this process.
///
/// Call it once, from `main`, before anything spawns a child: a child inherits this process's
/// environment, so the merge reaches `dotnet`, the language server, the debug adapter and the
/// integrated terminal without touching any of them. A shell that fails or outlives [`TIMEOUT`]
/// leaves the environment as it was; the reason is logged, never a value.
pub fn apply() {
    if !should_resolve(cfg!(target_os = "macos"), parent_id()) {
        return;
    }
    let shell = shell_path();
    match resolve(&shell, TIMEOUT) {
        Ok(vars) => {
            let vars = merge(&vars);
            for (key, value) in &vars {
                std::env::set_var(key, value);
            }
            eprintln!(
                "openridr: shell environment: merged {} variable(s) from {}",
                vars.len(),
                shell.display()
            );
        }
        Err(failure) => {
            eprintln!("openridr: shell environment: {failure}; keeping the inherited environment")
        }
    }
}

/// The parent's pid: `1` when `launchd` started the app from Finder, the Dock or Spotlight, and the
/// shell's pid when it was started from a terminal.
fn parent_id() -> u32 {
    #[cfg(unix)]
    {
        std::os::unix::process::parent_id()
    }
    #[cfg(not(unix))]
    {
        0
    }
}

/// macOS only, and only for a `launchd` launch: from a terminal the environment is already the
/// user's, and Linux and Windows keep the behavior they had.
fn should_resolve(macos: bool, parent: u32) -> bool {
    macos && parent == 1
}

fn shell_path() -> PathBuf {
    shell_path_from(std::env::var_os("SHELL"))
}

fn shell_path_from(shell: Option<OsString>) -> PathBuf {
    match shell {
        Some(shell) if !shell.is_empty() => PathBuf::from(shell),
        _ => PathBuf::from(FALLBACK_SHELL),
    }
}

/// What the shell runs. `env -0` separates entries with NUL, so a value that contains a newline
/// survives; the markers keep the parser away from whatever the rc files printed.
fn script() -> String {
    format!("printf '%s' {BEGIN}; env -0; printf '%s' {END}")
}

fn resolve(shell: &Path, timeout: Duration) -> Result<Vec<(String, String)>, Failure> {
    let mut child = Command::new(shell)
        .arg("-ilc")
        .arg(script())
        // No tty and no stdin: an rc file that asks a question gets EOF instead of hanging the app,
        // and the shell's own complaints are not part of the environment.
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| Failure::Message(format!("cannot start {}: {error}", shell.display())))?;
    let output = capture(&mut child, timeout)?;
    let vars = parse(&output.bytes).ok_or_else(|| {
        Failure::Message(format!(
            "{} printed no environment ({}); a shell without `env -0` cannot be read",
            shell.display(),
            output.status
        ))
    })?;
    if vars.is_empty() {
        return Err(Failure::Message(format!(
            "{} reported an empty environment",
            shell.display()
        )));
    }
    Ok(vars)
}

/// What one shell run produced.
#[derive(Debug)]
struct ShellOutput {
    bytes: Vec<u8>,
    status: ExitStatus,
}

/// Reads the child's output while it runs, giving up after `timeout`.
///
/// The read happens on its own thread because a full pipe would block the wait; the channel is what
/// the wait polls, so a grandchild that keeps the pipe open (a daemon an rc file started) costs a
/// thread, never the app.
fn capture(child: &mut Child, timeout: Duration) -> Result<ShellOutput, Failure> {
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| Failure::Message("the shell has no standard output".to_string()))?;
    let (sender, receiver) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = stdout.read_to_end(&mut bytes);
        let _ = sender.send(bytes);
    });
    let deadline = Instant::now() + timeout;
    let mut status: Option<ExitStatus> = None;
    let mut bytes: Option<Vec<u8>> = None;
    loop {
        if status.is_none() {
            match child.try_wait() {
                Ok(exited) => status = exited,
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Failure::Message(format!(
                        "cannot wait for the shell: {error}"
                    )));
                }
            }
        }
        if bytes.is_none() {
            match receiver.try_recv() {
                Ok(read) => bytes = Some(read),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => bytes = Some(Vec::new()),
            }
        }
        if status.is_some() && bytes.is_some() {
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Failure::Timeout);
        }
        std::thread::sleep(POLL);
    }
    match (status, bytes) {
        (Some(status), Some(bytes)) => {
            let _ = reader.join();
            Ok(ShellOutput { bytes, status })
        }
        // The loop only leaves with both, so this is the timeout path in disguise.
        _ => Err(Failure::Timeout),
    }
}

/// Reads the `KEY=VALUE` entries the shell printed between the markers. `None` when the markers are
/// missing, which is how a shell that rejected `env -0` or died early shows up.
fn parse(bytes: &[u8]) -> Option<Vec<(String, String)>> {
    let start = find(bytes, BEGIN.as_bytes())? + BEGIN.len();
    let end = start + find(&bytes[start..], END.as_bytes())?;
    let mut vars = Vec::new();
    for entry in bytes[start..end].split(|byte| *byte == 0) {
        if entry.is_empty() {
            continue;
        }
        let Some(equals) = entry.iter().position(|byte| *byte == b'=') else {
            continue;
        };
        let (Ok(key), Ok(value)) = (
            std::str::from_utf8(&entry[..equals]),
            std::str::from_utf8(&entry[equals + 1..]),
        ) else {
            // A value that is not UTF-8 cannot be set without corrupting it, so the inherited one
            // stays.
            continue;
        };
        if key.is_empty() {
            continue;
        }
        vars.push((key.to_string(), value.to_string()));
    }
    Some(vars)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// What goes into this process: everything the shell reported except the variables that describe
/// the helper shell itself.
fn merge(vars: &[(String, String)]) -> Vec<(String, String)> {
    vars.iter()
        .filter(|(key, _)| !NOT_MERGED.contains(&key.as_str()))
        .cloned()
        .collect()
}

/// Why the shell's environment could not be read.
#[derive(Debug, PartialEq, Eq)]
enum Failure {
    /// The shell, or a process holding its output, outlived the timeout; it was killed.
    Timeout,
    /// The shell could not be started, waited on or read.
    Message(String),
}

impl std::fmt::Display for Failure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Failure::Timeout => write!(
                formatter,
                "the login shell did not finish in {}s",
                TIMEOUT.as_secs()
            ),
            Failure::Message(message) => formatter.write_str(message),
        }
    }
}

#[cfg(test)]
#[path = "shell_env_test.rs"]
mod tests;
