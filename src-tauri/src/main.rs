#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use openridr_host::{Host, HostError};
use serde_json::Value;
use tauri::menu::{Menu, MenuItemBuilder};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, Url, WindowEvent};

/// Event that tells the UI a path is waiting in `take_pending_open`.
const OPEN_EVENT: &str = "openridr-open";
/// Event that tells the UI the user asked to quit; the UI calls `quit_app` after its decision.
const QUIT_EVENT: &str = "openridr-quit";
const QUIT_MENU_ID: &str = "quit";
/// Set once the UI approved the quit, so the `ExitRequested` it causes is let through.
static QUITTING: AtomicBool = AtomicBool::new(false);

/// Path handed over by Finder, the Dock or the command line until the UI takes it; the last one wins.
#[derive(Default)]
struct PendingOpen(Mutex<Option<String>>);

/// A quit request that arrived before the UI listener was ready; the first taker handles it.
#[derive(Default)]
struct PendingQuit(AtomicBool);

#[tauri::command]
fn host_call(method: String, params: Value, host: State<'_, Host>) -> Result<Value, HostError> {
    host.dispatch(&method, params)
}

#[tauri::command]
fn take_pending_open(pending: State<'_, PendingOpen>) -> Option<String> {
    pending.0.lock().unwrap().take()
}

#[tauri::command]
fn take_pending_quit(pending: State<'_, PendingQuit>) -> bool {
    pending.0.swap(false, Ordering::SeqCst)
}

/// End of the quit flow: the UI already saved or discarded, so nothing may prevent the exit.
#[tauri::command]
fn quit_app(app: AppHandle, host: State<'_, Host>) {
    let _ = host.dispatch("workspace.close", Value::Null);
    QUITTING.store(true, Ordering::SeqCst);
    app.exit(0);
}

/// First argument that is not a flag. macOS adds `-psn_...` when the app starts from Finder.
fn first_file_argument(args: impl IntoIterator<Item = String>) -> Option<String> {
    args.into_iter()
        .skip(1)
        .find(|arg| !arg.is_empty() && !arg.starts_with('-'))
}

/// Path a launch asks to open, resolved against that launch's directory: a forwarded argv is
/// read by another process, so its relative path no longer resolves against the right cwd.
fn launch_path(args: impl IntoIterator<Item = String>, cwd: Option<&Path>) -> Option<String> {
    let raw = first_file_argument(args)?;
    let path = Path::new(&raw);
    if path.is_absolute() {
        return Some(raw);
    }
    match cwd {
        Some(cwd) => Some(cwd.join(path).to_string_lossy().into_owned()),
        None => Some(raw),
    }
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

/// Every exit path closes the host workspace first: analysis and debugger process groups are
/// owned by the host, not by the webview that may already be gone.
fn close_host_workspace(app: &AppHandle) {
    let host = app.state::<Host>();
    let _ = host.dispatch("workspace.close", Value::Null);
}

fn request_quit(app: &AppHandle) {
    if QUITTING.load(Ordering::SeqCst) {
        app.exit(0);
        return;
    }
    app.state::<PendingQuit>().0.store(true, Ordering::SeqCst);
    let _ = app.emit(QUIT_EVENT, ());
}

/// Tauri's default menu, with Quit replaced: the predefined item calls `terminate:` on
/// NSApplication and never reaches the webview that owns the unsaved buffers.
#[cfg(target_os = "macos")]
fn default_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::default(app)?;
    // On macOS the app submenu is first and the predefined Quit item is its last entry.
    let app_menu = menu
        .items()?
        .into_iter()
        .next()
        .and_then(|item| item.as_submenu().cloned())
        .expect("the default macOS menu starts with the app submenu");
    let position = app_menu.items()?.len().saturating_sub(1);
    let quit = MenuItemBuilder::with_id(QUIT_MENU_ID, "Quit OpenRidr")
        .accelerator("CmdOrCtrl+Q")
        .build(app)?;
    app_menu.remove_at(position)?;
    app_menu.insert(&quit, position)?;
    Ok(menu)
}

#[cfg(not(target_os = "macos"))]
fn default_menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    Menu::default(app)
}

fn main() {
    tauri::Builder::default()
        // Must stay the first plugin: it has to exit a second process before any other setup runs.
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.set_focus();
            }
            if let Some(path) = launch_path(argv, Some(Path::new(&cwd))) {
                offer_open(app, path);
            }
        }))
        .menu(default_menu)
        .on_menu_event(|app, event| {
            if event.id() == QUIT_MENU_ID {
                request_quit(app);
            }
        })
        .manage(Host::new())
        .manage(PendingOpen::default())
        .manage(PendingQuit::default())
        .invoke_handler(tauri::generate_handler![
            host_call,
            take_pending_open,
            take_pending_quit,
            quit_app
        ])
        .setup(|app| {
            let cwd = std::env::current_dir().ok();
            if let Some(path) = launch_path(std::env::args(), cwd.as_deref()) {
                offer_open(app.handle(), path);
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("OpenRidr failed to start")
        .run(|app, event| match event {
            RunEvent::ExitRequested { api, .. } => {
                // Ask only while the webview can still show the dialog; with no window left
                // the app would stay alive with no way to close it.
                if !QUITTING.load(Ordering::SeqCst) && app.get_webview_window("main").is_some() {
                    api.prevent_exit();
                    request_quit(app);
                } else {
                    close_host_workspace(app);
                }
            }
            RunEvent::WindowEvent {
                event: WindowEvent::Destroyed,
                label,
                ..
            } if label == "main" => {
                // Closing the last window must stop LSP/netcoredbg instead of leaving orphans.
                close_host_workspace(app);
            }
            RunEvent::Exit => close_host_workspace(app),
            #[cfg(target_os = "macos")]
            RunEvent::Opened { urls } => {
                for path in urls.iter().filter_map(path_from_url) {
                    offer_open(app, path);
                }
            }
            _ => {}
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
    fn a_forwarded_relative_path_resolves_against_the_second_launch_directory() {
        let cwd = Path::new("/tmp/My Apps");
        assert_eq!(
            launch_path(args(&["OpenRidr", "-psn_0_1", "App.sln"]), Some(cwd)).as_deref(),
            Some("/tmp/My Apps/App.sln"),
        );
        assert_eq!(
            launch_path(args(&["OpenRidr", "/tmp/Other.sln"]), Some(cwd)).as_deref(),
            Some("/tmp/Other.sln"),
        );
        assert_eq!(
            launch_path(args(&["OpenRidr"]), Some(cwd)),
            None,
            "a launch with only flags forwards no path",
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
