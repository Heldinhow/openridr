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
use std::ffi::{OsStr, OsString};
use std::sync::{Mutex, MutexGuard, Once};

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
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Serializes the tests that write process environment, and removes the ambient values first.
/// Poisoning is ignored: one failed test must not fail every test that runs after it.
pub fn env_guard() -> MutexGuard<'static, ()> {
    let guard = ENV_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    purge_locked();
    guard
}

/// A host that cannot be configured by the shell that ran the test.
pub fn host() -> Host {
    purge_if_pending();
    Host::new()
}

/// Removes the ambient values once. Called by every helper, so it runs before the first test can
/// observe them, and never removes a value a test set through [`EnvVar`].
pub fn purge_ambient() {
    let _guard = ENV_LOCK
        .lock()
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
