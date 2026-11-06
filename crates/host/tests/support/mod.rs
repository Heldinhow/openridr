//! Harness shared by the host boundary test binaries.
//!
//! A test decides the environment of the host it creates, but the test *process* is shared: no
//! test may write `OPENRIDR_*` there. [`prepare`] sets the one shared environment (temporary app
//! home, probe on, ambient values removed) before any test can observe it, and every test that
//! needs different values runs its body in its own process ([`run_isolated_leaf`]).

#![allow(dead_code)]

use openridr_host::Host;
use std::ffi::{OsStr, OsString};
use std::sync::{Once, OnceLock};

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
static HOME: OnceLock<tempfile::TempDir> = OnceLock::new();

/// The one environment every test in a binary shares. It is set once, before the first test can
/// observe it, and never written again: no test is allowed to change the environment of the
/// process it shares with other tests. A test that needs different values runs its body in its
/// own process (see [`run_isolated_leaf`]).
///
/// - removes the ambient `OPENRIDR_*` values of whoever ran `cargo test`;
/// - points the app home at a temporary directory for this process;
/// - turns the development-only probe on for every test.
pub fn prepare() {
    PURGE.call_once(|| {
        for key in AMBIENT {
            std::env::remove_var(key);
        }
        let home = tempfile::tempdir().expect("create the process test app home");
        std::env::set_var("OPENRIDR_HOME", home.path());
        std::env::set_var("OPENRIDR_ENABLE_PROBE", "1");
        let _ = HOME.set(home);
    });
}

/// A host that cannot be configured by the shell that ran the test.
pub fn host() -> Host {
    prepare();
    Host::new()
}

/// The environment an isolated child gets: nothing inherited except what a test binary needs to
/// find its tools and its temporary directory. A hostile shell cannot reach the child.
pub fn controlled_command() -> std::process::Command {
    let mut command = std::process::Command::new(std::env::current_exe().unwrap());
    command.env_clear();
    for key in ["PATH", "HOME", "TMPDIR", "LANG", "LC_ALL"] {
        if let Some(value) = std::env::var_os(key) {
            command.env(key, value);
        }
    }
    command
}

/// Runs one isolated leaf of this binary in its own process. The leaf is marked `#[ignore]`
/// because it writes `OPENRIDR_*` for its host; only this fresh process ever sees those values.
pub fn run_isolated_leaf(leaf: &str) {
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

/// Runs one plain leaf of this same binary in a fresh process, with `envs` set on top of the
/// current environment. For tests that prove the shared environment is not inherited.
pub fn assert_leaf_passes_in_a_fresh_process(test: &str, envs: &[(&str, &OsStr)]) {
    let mut command = controlled_command();
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
    assert!(
        stdout.contains("1 passed"),
        "the fresh process did not run exactly `{test}`:\n{stdout}{stderr}"
    );
}

/// Sets or removes one variable for the duration of the binding, restoring the previous value on
/// drop. Only for a leaf that runs in its own process: the shared test process never writes the
/// environment (the `env_writes_are_confined_to_isolated_leaves` guard checks the sources).
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
