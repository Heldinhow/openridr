#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use openridr_host::{Host, HostError};
use serde_json::Value;
use tauri::menu::{Menu, MenuItemBuilder};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, Url, WindowEvent};

/// Event that tells the UI a path is waiting in `take_pending_open`.
const OPEN_EVENT: &str = "openridr-open";
/// Event that tells the UI the user asked to quit; the UI calls `quit_app` after its decision.
const QUIT_EVENT: &str = "openridr-quit";
const QUIT_MENU_ID: &str = "quit";
/// macOS resolves a menu key equivalent before the webview sees the key, so a native item that
/// owns a keymap shortcut makes the advertised shortcut dead. `Close Window` (⌘W) swallowed
/// `editor.closeTab`, and the AppKit injects `Close All` (⌥⌘W) next to a `performClose:` item,
/// which swallowed `git.update` and quit the app (issue #111). Neither half can be fixed on the
/// keymap side without losing Rider parity, so the close item is ours: no accelerator, and it
/// closes the window through `CloseRequested` so the UI can still ask about unsaved buffers.
const CLOSE_WINDOW_MENU_ID: &str = "window.close";
/// Set once the UI approved the quit, so the `ExitRequested` it causes is let through.
static QUITTING: AtomicBool = AtomicBool::new(false);

/// Serializes host dispatches: the UI fires `lsp.didOpen`, `lsp.didChange` and `session.write`
/// without awaiting them, so the host must receive them in the order they were sent.
static HOST_CALL: OnceLock<tauri::async_runtime::Mutex<()>> = OnceLock::new();

/// Path handed over by Finder, the Dock or the command line until the UI takes it; the last one wins.
#[derive(Default)]
struct PendingOpen(Mutex<Option<String>>);

/// A quit request that arrived before the UI listener was ready; the first taker handles it.
#[derive(Default)]
struct PendingQuit(AtomicBool);

#[tauri::command]
async fn host_call(
    method: String,
    params: Value,
    host: State<'_, Arc<Host>>,
) -> Result<Value, HostError> {
    let host = Arc::clone(&host);
    // Tokio's Mutex is FIFO, so this keeps the old synchronous ordering without running the
    // dispatch on the main thread.
    let lock = HOST_CALL.get_or_init(|| tauri::async_runtime::Mutex::new(()));
    let _guard = lock.lock().await;
    tauri::async_runtime::spawn_blocking(move || host.dispatch(&method, params))
        .await
        .map_err(|error| HostError::new("internal_error", format!("Host task failed: {error}")))?
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
fn quit_app(app: AppHandle, host: State<'_, Arc<Host>>) {
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
    let host = app.state::<Arc<Host>>();
    let _ = host.dispatch("workspace.close", Value::Null);
}

/// The window close the native item used to perform. `close()` emits `CloseRequested`, which the
/// UI intercepts to ask about unsaved buffers; `destroy()` would skip that decision.
fn close_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.close();
    }
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
/// NSApplication and never reaches the webview that owns the unsaved buffers. The close item is
/// replaced for the same kind of reason: its accelerator belongs to the keymap (see
/// `CLOSE_WINDOW_MENU_ID`).
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

    // The File submenu holds only the predefined Close Window item; replace it so the accelerator
    // table above stops owning ⌘W. AppKit derives the `Close All` item it injects into this same
    // submenu from that item: with no `performClose:` item left, neither the item nor its ⌥⌘W
    // accelerator exists any more, which is what returns both keys to the keymap.
    let submenus = menu.items()?;
    let submenu = |title: &str| {
        submenus
            .iter()
            .filter_map(|item| item.as_submenu())
            .find(|submenu| submenu.text().is_ok_and(|text| text == title))
            .cloned()
            .unwrap_or_else(|| panic!("the default macOS menu has a {title} submenu"))
    };
    let file_menu = submenu("File");
    let close = MenuItemBuilder::with_id(CLOSE_WINDOW_MENU_ID, "Close Window").build(app)?;
    let close_position = |menu: &tauri::menu::Submenu<tauri::Wry>| -> Option<usize> {
        menu.items().ok()?.iter().position(|item| {
            item.as_predefined_menuitem()
                .is_some_and(|item| item.text().is_ok_and(|text| text == "Close Window"))
        })
    };
    let file_position = close_position(&file_menu)
        .expect("the default File submenu holds the predefined close item");
    file_menu.remove_at(file_position)?;
    file_menu.insert(&close, file_position)?;

    // The Window submenu carried a second predefined close item with the same ⌘W; the single
    // accelerator-free item in File is enough for a one-window app.
    let window_menu = submenu("Window");
    if let Some(duplicate) = close_position(&window_menu) {
        window_menu.remove_at(duplicate)?;
    }
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
            } else if event.id() == CLOSE_WINDOW_MENU_ID {
                close_main_window(app);
            }
        })
        .manage(Arc::new(Host::new()))
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
        assert_eq!(
            first_file_argument(args(&["OpenRidr", "-psn_0_12345"])),
            None
        );
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
        assert_eq!(
            path_from_url(&file).as_deref(),
            Some("/tmp/My Apps/App.sln")
        );

        let web = Url::parse("https://example.com/App.sln").unwrap();
        assert_eq!(path_from_url(&web), None);
    }
}
