//! Native file chooser for the Linux shell, through the XDG desktop portal.
//!
//! The portal answers `FileChooser.OpenFile` with a `Response` signal emitted **unicast** to the
//! connection that made the call, and it refuses `Request.Close` from any other connection
//! (`AccessDenied: Portal operation not allowed: Unmatched caller`). A one-shot `gdbus call`
//! therefore loses the answer when it exits, which is why the shell links `zbus` and keeps this
//! connection alive until the dialog answers.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use openridr_host::{HostError, HostResult};
use serde_json::{json, Value};
use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::{OwnedObjectPath, OwnedValue, Value as ZValue};

const PORTAL: &str = "org.freedesktop.portal.Desktop";
const PORTAL_PATH: &str = "/org/freedesktop/portal/desktop";
const FILE_CHOOSER: &str = "org.freedesktop.portal.FileChooser";
const REQUEST: &str = "org.freedesktop.portal.Request";
const TITLE: &str = "Open solution or project";

/// `workspace.pick` on Linux: the portal's file chooser, filtered to solutions and projects.
/// `{ "path": null }` means the user cancelled; `pick_unavailable` means there is no portal (or no
/// file-chooser backend), which is the welcome screen's cue to focus the typed path instead.
pub fn pick_workspace() -> HostResult<Value> {
    let connection = Connection::session().map_err(unavailable)?;
    let token = next_token();
    let expected = request_path(&connection, &token);
    let request =
        Proxy::new(&connection, PORTAL, expected.as_str(), REQUEST).map_err(unavailable)?;
    // Subscribe before the call: the portal may answer before `OpenFile` returns.
    let mut response = request.receive_signal("Response").map_err(unavailable)?;

    let chooser =
        Proxy::new(&connection, PORTAL, PORTAL_PATH, FILE_CHOOSER).map_err(unavailable)?;
    let handle: OwnedObjectPath = chooser
        .call("OpenFile", &("", TITLE, options(&token)))
        .map_err(open_error)?;
    // The portal honours `handle_token`, so the reply is the path the signal is already
    // subscribed to. A different handle means the answer path is unknown: subscribing now could
    // miss a response that was already emitted, so fail instead of waiting forever.
    accepted_handle(&expected, handle.as_str())?;

    let message = response
        .next()
        .ok_or_else(|| failed("the portal closed the request without answering"))?;
    let body = message.body();
    let (code, mut results): (u32, HashMap<String, OwnedValue>) =
        body.deserialize().map_err(|error| failed(&error))?;
    match code {
        // 0 = success, 1 = cancelled, 2 = other error (portal spec).
        0 => {
            let path = results
                .remove("uris")
                .and_then(|uris| Vec::<String>::try_from(uris).ok())
                .and_then(|uris| uris.first().and_then(|uri| path_from_uri(uri)))
                .ok_or_else(|| failed("the portal returned no file"))?;
            Ok(json!({ "path": path }))
        }
        1 => Ok(json!({ "path": null })),
        _ => Err(failed("the portal reported an error")),
    }
}

/// `OpenFile` options: one entry per filter group, `(0 = glob, pattern)` per pattern.
fn options<'a>(token: &'a str) -> HashMap<&'a str, ZValue<'a>> {
    let filters: Vec<(&str, Vec<(u32, &str)>)> = vec![
        (
            "Solutions and projects",
            vec![(0, "*.sln"), (0, "*.slnx"), (0, "*.csproj")],
        ),
        ("All files", vec![(0, "*")]),
    ];
    let mut options = HashMap::new();
    options.insert("handle_token", ZValue::from(token));
    options.insert("modal", ZValue::from(true));
    options.insert("multiple", ZValue::from(false));
    options.insert("filters", ZValue::new(filters));
    options
}

/// The request object the portal will emit `Response` from: its unique name without the leading
/// `:` and with `.` replaced by `_`, plus the token we asked for.
fn request_path(connection: &Connection, token: &str) -> String {
    let sender = connection
        .unique_name()
        .map(|name| name.as_str().to_string())
        .unwrap_or_default();
    request_path_for(&sender, token)
}

fn request_path_for(sender: &str, token: &str) -> String {
    let sender = sender.trim_start_matches(':').replace('.', "_");
    format!("/org/freedesktop/portal/desktop/request/{sender}/{token}")
}

/// A token is a D-Bus path element: alphanumerics and underscores only.
fn next_token() -> String {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    format!(
        "openridr_{}_{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

/// The portal is expected to answer on the request path derived from `handle_token`; anything
/// else leaves the picker without a path to wait on.
fn accepted_handle(expected: &str, handle: &str) -> HostResult<()> {
    if handle == expected {
        Ok(())
    } else {
        Err(failed(format!(
            "the portal returned request {handle} instead of {expected}"
        )))
    }
}

/// The portal returns `file://` URIs; other schemes are not files this app opens.
fn path_from_uri(uri: &str) -> Option<String> {
    let url = tauri::Url::parse(uri).ok()?;
    if url.scheme() != "file" {
        return None;
    }
    url.to_file_path()
        .ok()
        .map(|path| path.to_string_lossy().into_owned())
}

/// No session bus, no portal, or no file-chooser backend: the typed path is the way out.
fn unavailable(error: impl std::fmt::Display) -> HostError {
    eprintln!("openridr: file chooser unavailable: {error}");
    HostError::new(
        "pick_unavailable",
        "No native file chooser is available here; type the path instead.",
    )
}

/// The portal exists but the request failed: still a notice on the welcome screen, not a banner.
fn failed(error: impl std::fmt::Display) -> HostError {
    eprintln!("openridr: file chooser failed: {error}");
    HostError::new(
        "pick_failed",
        "The file chooser failed; type the path instead.",
    )
}

/// A missing portal service or interface is "unavailable", not a broken dialog.
fn open_error(error: zbus::Error) -> HostError {
    match &error {
        zbus::Error::MethodError(name, _, _)
            if matches!(
                name.as_str(),
                "org.freedesktop.DBus.Error.ServiceUnknown"
                    | "org.freedesktop.DBus.Error.NameHasNoOwner"
                    | "org.freedesktop.DBus.Error.UnknownMethod"
                    | "org.freedesktop.DBus.Error.UnknownInterface"
            ) =>
        {
            unavailable(error)
        }
        _ => failed(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_path_is_the_sender_without_the_colon_and_the_token() {
        assert_eq!(
            request_path_for(":1.42", "openridr_7_0"),
            "/org/freedesktop/portal/desktop/request/1_42/openridr_7_0"
        );
        assert_eq!(
            request_path_for(":1.7", "openridr_7_0"),
            "/org/freedesktop/portal/desktop/request/1_7/openridr_7_0"
        );
    }

    #[test]
    fn a_handle_that_is_not_the_token_path_is_refused() {
        let expected = request_path_for(":1.42", "openridr_7_0");
        assert!(accepted_handle(&expected, &expected).is_ok());
        let error = accepted_handle(
            &expected,
            "/org/freedesktop/portal/desktop/request/1_42/other",
        )
        .expect_err("a different handle has no answer path");
        assert_eq!(error.code, "pick_failed");
    }

    #[test]
    fn tokens_are_unique_path_elements() {
        let first = next_token();
        let second = next_token();
        assert_ne!(first, second);
        assert!(first.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
    }

    #[test]
    fn file_uris_become_local_paths_and_other_schemes_are_ignored() {
        assert_eq!(
            path_from_uri("file:///tmp/My%20Apps/OrderDesk.sln").as_deref(),
            Some("/tmp/My Apps/OrderDesk.sln")
        );
        assert_eq!(path_from_uri("https://example.com/App.sln"), None);
        assert_eq!(path_from_uri("not a uri"), None);
    }

    #[test]
    fn the_filters_cover_solutions_projects_and_every_file() {
        let options = options("openridr_7_0");
        assert_eq!(options["handle_token"], ZValue::from("openridr_7_0"));
        assert_eq!(options["multiple"], ZValue::from(false));
        assert_eq!(options["modal"], ZValue::from(true));
        assert_eq!(
            options["filters"].value_signature().to_string(),
            "a(sa(us))",
            "the portal only accepts the documented filter shape"
        );
        let filters: Vec<(String, Vec<(u32, String)>)> = options["filters"]
            .try_clone()
            .expect("filters clone")
            .downcast()
            .expect("filters are a(sa(us))");
        assert_eq!(filters[0].0, "Solutions and projects");
        assert_eq!(
            filters[0]
                .1
                .iter()
                .map(|(_, pattern)| pattern.as_str())
                .collect::<Vec<_>>(),
            ["*.sln", "*.slnx", "*.csproj"]
        );
        assert_eq!(filters[1].0, "All files");
        assert_eq!(filters[1].1, vec![(0u32, "*".to_string())]);
    }
}
