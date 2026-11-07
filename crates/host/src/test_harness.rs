//! Test-only harness for the lib unit tests.
//!
//! The lib test binary is shared by every unit test in this crate, so a unit test must not write
//! the process environment while others run. This module owns the only writes:
//!
//! - [`prepare`] sets the one shared environment once (ambient `OPENRIDR_*` removed, app home
//!   pointed at a temporary directory);
//! - [`EnvVar`] scopes a value for a test that owns its process;
//! - [`run_isolated_leaf`] runs one `#[ignore]`d leaf in its own process with a controlled
//!   environment, which is how a unit test that needs different values runs.

#![cfg(test)]

use std::ffi::{OsStr, OsString};
use std::sync::{Condvar, Mutex, Once, OnceLock};

/// Every `OPENRIDR_*` variable the host crate reads. Keep in sync with `tests/support/mod.rs`.
const AMBIENT: &[&str] = &[
    "OPENRIDR_BUNDLED_CSHARP_LS",
    "OPENRIDR_BUNDLED_DOTNET_ROOT",
    "OPENRIDR_BUNDLED_NETCOREDBG",
    "OPENRIDR_BUNDLED_REQUIRED",
    "OPENRIDR_DEBUG_ASSET_SHA256",
    "OPENRIDR_DEBUG_ASSET_URL",
    "OPENRIDR_DEBUG_PLATFORM",
    "OPENRIDR_ENABLE_PROBE",
    "OPENRIDR_FORCE_HOSTROOT",
    "OPENRIDR_HOME",
    "OPENRIDR_SESSION_STORE",
    "OPENRIDR_SYSTEM_PROXY",
    "OPENRIDR_TOOLS_MUXER",
    "OPENRIDR_TOOLS_PACKAGE_SHA512",
    "OPENRIDR_TOOLS_PACKAGE_URL",
    "OPENRIDR_TOOLS_PROBE_DOWNLOAD_DELAY_MS",
    "OPENRIDR_TRUST_STORE",
];

static PURGE: Once = Once::new();
/// Same cap as the integration harness: isolated leaves do real work, so the binary must not start
/// all of them at once.
static SLOTS: Mutex<usize> = Mutex::new(0);
static SLOT_READY: Condvar = Condvar::new();
const PARALLEL_CHILDREN: usize = 1;

struct Slot;

impl Drop for Slot {
    fn drop(&mut self) {
        let mut running = SLOTS.lock().unwrap_or_else(|poison| poison.into_inner());
        *running -= 1;
        SLOT_READY.notify_one();
    }
}

fn slot() -> Slot {
    let mut running = SLOTS.lock().unwrap_or_else(|poison| poison.into_inner());
    while *running >= PARALLEL_CHILDREN {
        running = SLOT_READY
            .wait(running)
            .unwrap_or_else(|poison| poison.into_inner());
    }
    *running += 1;
    Slot
}
static HOME: OnceLock<tempfile::TempDir> = OnceLock::new();

/// The one environment the lib test binary shares. Set once, before any test writes anything.
pub fn prepare() {
    PURGE.call_once(|| {
        for key in AMBIENT {
            std::env::remove_var(key);
        }
        let home = tempfile::tempdir().expect("create the process test app home");
        std::env::set_var("OPENRIDR_HOME", home.path());
        // The macOS proxy source reads the machine's network settings; a test must not.
        std::env::set_var("OPENRIDR_SYSTEM_PROXY", "off");
        let _ = HOME.set(home);
    });
}

/// The environment every isolated child gets: nothing inherited except what a test binary needs to
/// find its tools and temporary directory.
pub fn controlled_command() -> std::process::Command {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command.env_clear();
    // Windows cannot start a socket without `SystemRoot` (WSAStartup loads its providers from
    // there) and its temp directory comes from `TEMP`, not `TMPDIR`.
    let keys: &[&str] = if cfg!(windows) {
        &[
            "PATH",
            "HOME",
            "TMPDIR",
            "LANG",
            "LC_ALL",
            "SystemRoot",
            "SystemDrive",
            "WINDIR",
            "APPDATA",
            "LOCALAPPDATA",
            "USERPROFILE",
            "TEMP",
            "TMP",
            "PATHEXT",
            "COMSPEC",
        ]
    } else {
        &["PATH", "HOME", "TMPDIR", "LANG", "LC_ALL"]
    };
    for key in keys {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
}

/// Runs one isolated leaf of this binary in its own process.
pub fn run_isolated_leaf(leaf: &str) {
    let _slot = slot();
    let done = controlled_command()
        .args(["--test-threads=1", "--ignored", "--exact", leaf])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&done.stdout);
    let stderr = String::from_utf8_lossy(&done.stderr);
    assert!(
        done.status.success(),
        "the isolated leaf `{leaf}` failed in its own process:\n{stdout}{stderr}"
    );
    // The filter is a substring, so `1 passed` is what proves only this leaf ran. A module-qualified
    // unit test matches by its bare name.
    assert!(
        stdout.contains("1 passed"),
        "the isolated process did not run exactly `{leaf}`:\n{stdout}{stderr}"
    );
}

/// Runs `run` with `OPENRIDR_HOME` pointed at a fresh temporary directory and the trust store off.
/// Only for a leaf that runs in its own process.
pub fn with_home(run: impl FnOnce(&std::path::Path)) {
    let home = tempfile::tempdir().unwrap();
    let _home = EnvVar::set("OPENRIDR_HOME", home.path());
    let _store = EnvVar::remove("OPENRIDR_TRUST_STORE");
    run(home.path());
}

/// Sets or removes one variable for the duration of the binding, restoring the previous value on
/// drop. Only for a leaf that runs in its own process.
pub struct EnvVar {
    key: &'static str,
    old: Option<OsString>,
}

impl EnvVar {
    pub fn set(key: &'static str, value: impl AsRef<OsStr>) -> Self {
        prepare();
        let old = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, old }
    }

    pub fn remove(key: &'static str) -> Self {
        prepare();
        let old = std::env::var_os(key);
        std::env::remove_var(key);
        Self { key, old }
    }
}

impl Drop for EnvVar {
    fn drop(&mut self) {
        match &self.old {
            Some(value) => std::env::set_var(self.key, value),
            None => std::env::remove_var(self.key),
        }
    }
}
