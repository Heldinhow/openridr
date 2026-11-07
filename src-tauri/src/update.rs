//! In-app updates: check the signed manifest, download and install the artifact, restart.
//!
//! The updater replaces the `.app` bundle, so it only exists in a bundled build; `tauri dev` and
//! the dev HTTP bridge get `unsupported` instead of a check that could never install.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::{Update, UpdaterExt};

/// Event the UI listens to for the download bar; the payload is `{ downloaded, total }`.
const PROGRESS_EVENT: &str = "openridr-update-progress";
/// A connection that never opens must fail in seconds. There is deliberately no total timeout: the
/// plugin applies the same one to the artifact download, and a ~70 MB transfer on a slow link is
/// not an error.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// The bar is redrawn at most this often; the stream hands the body over in small chunks.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(200);

/// The update the user was told about, so `update_install` installs exactly that one.
#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<Update>>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    /// `available`, `current` (already the newest) or `unsupported` (not a bundled build).
    state: &'static str,
    version: Option<String>,
    current: Option<String>,
    notes: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    downloaded: u64,
    total: Option<u64>,
}

impl CheckResult {
    fn available(update: &Update) -> Self {
        Self {
            state: "available",
            version: Some(update.version.clone()),
            current: Some(update.current_version.clone()),
            notes: update.body.clone(),
        }
    }

    fn current(version: String) -> Self {
        Self {
            state: "current",
            version: None,
            current: Some(version),
            notes: None,
        }
    }

    fn unsupported() -> Self {
        Self {
            state: "unsupported",
            version: None,
            current: None,
            notes: None,
        }
    }
}

/// The updater replaces the `.app` bundle, so a binary that is not inside one has nothing to
/// update: `tauri dev` and the dev HTTP bridge run `target/debug/openridr`.
fn updatable_executable(executable: &Path) -> bool {
    let Some(macos) = executable.parent() else {
        return false;
    };
    let Some(contents) = macos.parent() else {
        return false;
    };
    let Some(bundle) = contents.parent() else {
        return false;
    };
    macos.file_name().and_then(|name| name.to_str()) == Some("MacOS")
        && contents.file_name().and_then(|name| name.to_str()) == Some("Contents")
        && bundle.extension().and_then(|extension| extension.to_str()) == Some("app")
}

/// The UI shows these sentences verbatim, so each one names the phase and keeps the cause; the
/// plugin's own errors already say what the specific failure was.
fn check_error(cause: &str) -> String {
    format!("Could not check for updates: {cause}")
}

fn install_error(cause: &str) -> String {
    format!("Could not install the update: {cause}")
}

fn should_emit(last: Instant, now: Instant) -> bool {
    now.duration_since(last) >= PROGRESS_INTERVAL
}

/// Progress for one install: the two download callbacks share it, so it cannot be a local counter.
struct ProgressSink {
    app: AppHandle,
    downloaded: AtomicU64,
    last: Mutex<Instant>,
}

impl ProgressSink {
    fn new(app: AppHandle) -> Self {
        Self {
            app,
            downloaded: AtomicU64::new(0),
            last: Mutex::new(Instant::now()),
        }
    }

    fn chunk(&self, chunk: usize, total: Option<u64>) {
        let downloaded = self.downloaded.fetch_add(chunk as u64, Ordering::Relaxed) + chunk as u64;
        let mut last = self.last.lock().unwrap();
        let now = Instant::now();
        if should_emit(*last, now) {
            *last = now;
            let _ = self
                .app
                .emit(PROGRESS_EVENT, Progress { downloaded, total });
        }
    }

    /// The last event always goes out, so the bar reaches its end before the install starts.
    fn finish(&self) {
        let downloaded = self.downloaded.load(Ordering::Relaxed);
        let _ = self.app.emit(
            PROGRESS_EVENT,
            Progress {
                downloaded,
                total: Some(downloaded),
            },
        );
    }
}

#[tauri::command]
pub async fn update_check(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> Result<CheckResult, String> {
    let executable = std::env::current_exe().map_err(|error| check_error(&error.to_string()))?;
    if !updatable_executable(&executable) {
        return Ok(CheckResult::unsupported());
    }
    let updater = app
        .updater_builder()
        .configure_client(|builder| builder.connect_timeout(CONNECT_TIMEOUT))
        .build()
        .map_err(|error| check_error(&error.to_string()))?;
    let update = updater.check().await.map_err(|error| {
        let message = check_error(&error.to_string());
        // The startup check is silent in the UI, so this line is the only trace it leaves.
        eprintln!("update: {message}");
        message
    })?;
    let result = match &update {
        Some(update) => CheckResult::available(update),
        None => CheckResult::current(app.package_info().version.to_string()),
    };
    *pending.0.lock().unwrap() = update;
    Ok(result)
}

#[tauri::command]
pub async fn update_install(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
    version: String,
) -> Result<(), String> {
    let Some(update) = pending.0.lock().unwrap().take() else {
        return Err(install_error("there is no update to install; check again"));
    };
    // The user confirmed one version: a check that finished in between must not install another.
    if update.version != version {
        return Err(install_error(&format!(
            "{version} is no longer the version to install; check again"
        )));
    }
    let sink = ProgressSink::new(app);
    update
        .download_and_install(|chunk, total| sink.chunk(chunk, total), || sink.finish())
        .await
        .map_err(|error| install_error(&error.to_string()))
}

/// End of the update flow: the UI already saved or discarded, so the app may restart now. The
/// host owns the LSP and debugger process groups, and `restart` leaves through LaunchServices
/// without the exit event that would close them.
#[tauri::command]
pub fn restart_app(app: AppHandle) {
    crate::close_host_workspace(&app);
    crate::approve_exit();
    app.restart();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_bundled_executable_can_update_itself() {
        assert!(updatable_executable(Path::new(
            "/Applications/OpenRidr.app/Contents/MacOS/openridr"
        )));
        assert!(updatable_executable(Path::new(
            "/Users/x/Applications-test/OpenRidr.app/Contents/MacOS/openridr"
        )));
        assert!(
            !updatable_executable(Path::new("/repo/src-tauri/target/debug/openridr")),
            "tauri dev runs a bare binary"
        );
        assert!(
            !updatable_executable(Path::new("/Applications/OpenRidr/Contents/MacOS/openridr")),
            "a directory without the .app extension is not a bundle"
        );
        assert!(
            !updatable_executable(Path::new(
                "/Applications/OpenRidr.app/Contents/Resources/openridr"
            )),
            "only the MacOS directory holds the executable"
        );
        assert!(
            !updatable_executable(Path::new("/Applications/OpenRidr.app")),
            "the bundle itself is not an executable inside one"
        );
        assert!(!updatable_executable(Path::new("/")), "no parents");
    }

    #[test]
    fn each_phase_names_itself_and_keeps_the_cause() {
        assert_eq!(
            check_error("network is down"),
            "Could not check for updates: network is down"
        );
        assert_eq!(
            install_error("Signature verification failed"),
            "Could not install the update: Signature verification failed"
        );
    }

    #[test]
    fn progress_is_throttled_and_the_boundary_emits() {
        let start = Instant::now();
        assert!(!should_emit(start, start));
        assert!(!should_emit(
            start,
            start + PROGRESS_INTERVAL - Duration::from_millis(1)
        ));
        assert!(should_emit(start, start + PROGRESS_INTERVAL));
    }
}
