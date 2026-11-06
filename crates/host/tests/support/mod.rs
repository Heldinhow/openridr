//! Harness shared by the host boundary test binaries.
//!
//! A test decides the environment of the host it creates. Ambient `OPENRIDR_*` values from the
//! shell that ran `cargo test` would otherwise change the result per machine: an exported
//! `OPENRIDR_BUNDLED_NETCOREDBG` alone makes the controlled installer tests see a bundled adapter.
//! The first harness call in a binary removes those values; a helper that needs one sets it and
//! restores it on drop. Nothing here touches the real app home: the host already resolves a
//! per-process temporary home for test binaries, and every test that wants another one passes an
//! explicit `OPENRIDR_HOME`.

#![allow(dead_code)]

use openridr_host::Host;
use std::cell::Cell;
use std::ffi::{OsStr, OsString};
use std::ops::Deref;
use std::sync::{Once, RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Every `OPENRIDR_*` variable the host crate reads. A new one belongs here or the suite starts
/// depending on whichever shell ran it.
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
    "OPENRIDR_TOOLS_MUXER",
    "OPENRIDR_TOOLS_PACKAGE_SHA512",
    "OPENRIDR_TOOLS_PACKAGE_URL",
    "OPENRIDR_TOOLS_PROBE_DOWNLOAD_DELAY_MS",
    "OPENRIDR_TRUST_STORE",
];

static PURGE: Once = Once::new();
static PROBE: Once = Once::new();
/// Readers (a host being created or used by a test) share it; a test that writes the process
/// environment needs it exclusively. Without the read side, a host could read `OPENRIDR_HOME`
/// from another test's scope. Poisoning is ignored: one failed test must not fail the rest.
static ENV_LOCK: RwLock<()> = RwLock::new(());

thread_local! {
    static WRITES_ENV: Cell<bool> = const { Cell::new(false) };
    static READS_ENV: Cell<bool> = const { Cell::new(false) };
}

/// Holds the environment exclusively for as long as the binding lives. Every test that writes a
/// process variable takes this first, so no host can be created or used under a half-swapped
/// environment. Removes the ambient values before yielding.
pub struct EnvGuard {
    _guard: RwLockWriteGuard<'static, ()>,
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        WRITES_ENV.with(|flag| flag.set(false));
    }
}

pub fn env_guard() -> EnvGuard {
    let guard = ENV_LOCK
        .write()
        .unwrap_or_else(|poison| poison.into_inner());
    purge_locked();
    WRITES_ENV.with(|flag| flag.set(true));
    EnvGuard { _guard: guard }
}

/// The development-only probe switch is one decision for the whole binary, not a per-test scope.
/// A scoped binding restores "unset" on drop, which is how a test that relied on the probe saw
/// `processes.probe` fail unless the variable was exported before `cargo test`.
pub fn enable_probe() {
    purge_if_pending();
    PROBE.call_once(|| std::env::set_var("OPENRIDR_ENABLE_PROBE", "1"));
}

/// A host that cannot be configured by the shell that ran the test.
///
/// The returned host holds the shared side of the environment lock until it is dropped, so no test
/// can be swapping a variable while this host is created *or used*. A test that already holds the
/// exclusive side (it writes the environment itself) reuses that permission: taking the shared
/// side there would deadlock.
pub fn host() -> TestHost {
    purge_if_pending();
    if WRITES_ENV.with(Cell::get) {
        return TestHost {
            host: Host::new(),
            guard: None,
        };
    }
    // One shared guard is enough per thread; a second `host()` in the same test would take a
    // second read lock that a waiting writer could deadlock against.
    if READS_ENV.with(Cell::get) {
        return TestHost {
            host: Host::new(),
            guard: None,
        };
    }
    let guard = ENV_LOCK
        .read()
        .unwrap_or_else(|poison| poison.into_inner());
    READS_ENV.with(|flag| flag.set(true));
    TestHost {
        host: Host::new(),
        guard: Some(guard),
    }
}

/// A host from [`host`] plus the shared environment lock it holds for the test's lifetime.
/// Dereferences to the host, so `host.dispatch(...)` and `&host` helpers keep working.
pub struct TestHost {
    host: Host,
    guard: Option<RwLockReadGuard<'static, ()>>,
}

impl Deref for TestHost {
    type Target = Host;

    fn deref(&self) -> &Host {
        &self.host
    }
}

impl Drop for TestHost {
    fn drop(&mut self) {
        if self.guard.is_some() {
            READS_ENV.with(|flag| flag.set(false));
        }
    }
}

/// Runs one leaf test of this same binary in a fresh process, with `envs` set on top of the current
/// environment. The purge runs once per binary, so only a new process can prove it.
pub fn assert_leaf_passes_in_a_fresh_process(test: &str, envs: &[(&str, &OsStr)]) {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command.args(["--test-threads=1", "--exact", test]);
    for (key, value) in envs {
        command.env(key, value);
    }
    let done = command.output().unwrap();
    let stdout = String::from_utf8_lossy(&done.stdout);
    let stderr = String::from_utf8_lossy(&done.stderr);
    assert!(
        done.status.success(),
        "`{test}` failed in a fresh process with {envs:?}:\n{stdout}{stderr}"
    );
    // A filter that matches nothing reports `0 passed`, which would make this guard vacuous.
    assert!(
        stdout.contains("1 passed"),
        "the fresh process did not run exactly `{test}`:\n{stdout}{stderr}"
    );
}

/// Removes the ambient values once. Called by every helper, so it runs before the first test can
/// observe them, and never removes a value a test set through [`EnvVar`].
pub fn purge_ambient() {
    let _guard = ENV_LOCK
        .write()
        .unwrap_or_else(|poison| poison.into_inner());
    purge_locked();
}

fn purge_if_pending() {
    // Taking the lock while a test already holds `env_guard` would deadlock, and that test has
    // already purged. `is_completed` makes this cheap on every later call.
    if !PURGE.is_completed() {
        purge_ambient();
    }
}

fn purge_locked() {
    PURGE.call_once(|| {
        for key in AMBIENT {
            std::env::remove_var(key);
        }
    });
}

/// Sets or removes one variable for the duration of the binding, restoring the previous value on
/// drop. Always purges the ambient values first, so an inherited value is never mistaken for the
/// test's own.
pub struct EnvVar {
    key: &'static str,
    old: Option<OsString>,
}

impl EnvVar {
    pub fn set(key: &'static str, value: impl AsRef<OsStr>) -> Self {
        purge_if_pending();
        let old = std::env::var_os(key);
        std::env::set_var(key, value);
        Self { key, old }
    }

    pub fn remove(key: &'static str) -> Self {
        purge_if_pending();
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
