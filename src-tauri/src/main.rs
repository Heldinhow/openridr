#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

use openridr_host::{Host, HostError};
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, State, Url};

/// Event that tells the UI a path is waiting in `take_pending_open`.
const OPEN_EVENT: &str = "openridr-open";

/// Path handed over by Finder, the Dock or the command line until the UI takes it; the last one wins.
#[derive(Default)]
struct PendingOpen(Mutex<Option<String>>);

#[tauri::command]
fn host_call(method: String, params: Value, host: State<'_, Host>) -> Result<Value, HostError> {
    host.dispatch(&method, params)
}

#[tauri::command]
fn take_pending_open(pending: State<'_, PendingOpen>) -> Option<String> {
    pending.0.lock().unwrap().take()
}

/// First argument that is not a flag. macOS adds `-psn_...` when the app starts from Finder.
fn first_file_argument(args: impl IntoIterator<Item = String>) -> Option<String> {
    args.into_iter()
        .skip(1)
        .find(|arg| !arg.is_empty() && !arg.starts_with('-'))
}

/// Local path of a `file://` URL; other schemes are not files this app opens.
fn path_from_url(url: &Url) -> Option<String> {
    if url.scheme() != "file" {
        return None;
    }
    url.to_file_path()
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
}

fn offer_open(app: &AppHandle, path: String) {
    *app.state::<PendingOpen>().0.lock().unwrap() = Some(path);
    let _ = app.emit(OPEN_EVENT, ());
}

fn main() {
    tauri::Builder::default()
        .manage(Host::new())
        .manage(PendingOpen::default())
        .invoke_handler(tauri::generate_handler![host_call, take_pending_open])
        .setup(|app| {
            if let Some(path) = first_file_argument(std::env::args()) {
                offer_open(app.handle(), path);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("OpenRidr failed to start")
        .run(|app, event| {
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Opened { urls } = event {
                for path in urls.iter().filter_map(path_from_url) {
                    offer_open(app, path);
                }
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn argv_ignores_program_name_and_finder_flags() {
        assert_eq!(first_file_argument(args(&["OpenRidr"])), None);
        assert_eq!(first_file_argument(args(&["OpenRidr", "-psn_0_12345"])), None);
        assert_eq!(
            first_file_argument(args(&["OpenRidr", "-psn_0_1", "/tmp/My Apps/App.sln"])),
            Some("/tmp/My Apps/App.sln".to_string()),
        );
    }

    #[test]
    fn file_urls_become_local_paths_and_other_schemes_are_ignored() {
        let file = Url::parse("file:///tmp/My%20Apps/App.sln").unwrap();
        assert_eq!(path_from_url(&file).as_deref(), Some("/tmp/My Apps/App.sln"));

        let web = Url::parse("https://example.com/App.sln").unwrap();
        assert_eq!(path_from_url(&web), None);
    }
}
