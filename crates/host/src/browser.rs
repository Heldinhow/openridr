//! Opening a launch profile's URL in the default browser.
//!
//! The profile decides (`launchBrowser`/`launchUrl`); the host composes the URL, validates it,
//! waits (bounded) for the app to accept connections and then spawns the platform opener with the
//! URL as a single argument. Nothing here goes through a shell, and a failure to open never fails
//! the launch: the caller writes the returned line into the Run or Debug log.

use std::net::{TcpStream, ToSocketAddrs};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const READY_TIMEOUT: Duration = Duration::from_secs(15);
const RETRY: Duration = Duration::from_millis(200);
const CONNECT_TIMEOUT: Duration = Duration::from_millis(500);

/// What a launch should do about the browser, decided from the profile alone.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Launch {
    /// The profile does not ask for a browser.
    #[default]
    None,
    /// Open this URL once the app accepts connections.
    Open(String),
    /// The profile asks, but this URL cannot be opened; the reason belongs in the log.
    Skip(String),
}

/// A validated `http(s)` URL: what to open and where to wait for the app.
#[derive(Debug, PartialEq)]
struct Target {
    scheme: String,
    authority: String,
    host: String,
    port: u16,
}

/// The URL a profile asks for: the first entry of the effective `ASPNETCORE_URLS` combined with
/// `launchUrl` (an absolute one wins, a relative one is appended). Anything that is not an
/// `http(s)` URL with a host — and a port the app actually picks — is refused with its reason.
pub fn plan(launch_browser: bool, launch_url: Option<&str>, urls: Option<&str>) -> Launch {
    if !launch_browser {
        return Launch::None;
    }
    let base = urls.and_then(|value| {
        value
            .split(';')
            .map(str::trim)
            .find(|part| !part.is_empty())
    });
    let launch_url = launch_url.map(str::trim).filter(|value| !value.is_empty());
    let candidate =
        match (launch_url, base) {
            (Some(url), _) if scheme(url).is_some() => url.to_string(),
            (Some(url), Some(base)) => match parse(base) {
                Ok(target) if url.starts_with('/') => {
                    format!("{}://{}{url}", target.scheme, target.authority)
                }
                _ => format!(
                    "{}/{}",
                    base.trim_end_matches('/'),
                    url.trim_start_matches('/')
                ),
            },
            (Some(_), None) => return Launch::Skip(
                "launchUrl is relative and the profile has no applicationUrl or ASPNETCORE_URLS."
                    .to_string(),
            ),
            (None, Some(base)) => base.to_string(),
            (None, None) => return Launch::Skip(
                "launchBrowser is set but the profile has no applicationUrl or ASPNETCORE_URLS."
                    .to_string(),
            ),
        };
    match parse(&candidate) {
        Ok(_) => Launch::Open(candidate),
        Err(reason) => Launch::Skip(reason),
    }
}

/// Waits for the app to accept connections and opens the URL. Returns the line for the log.
pub fn open_when_ready(url: &str, alive: impl Fn() -> bool + Send + 'static) -> String {
    let Ok(target) = parse(url) else {
        return format!("Launch browser skipped: {url} is not an http(s) URL.");
    };
    if !wait_ready(&target, &alive, READY_TIMEOUT) {
        return format!(
            "Launch browser skipped: {url} did not accept connections within {} s.",
            READY_TIMEOUT.as_secs()
        );
    }
    match open(url) {
        Ok(()) => format!("Opened {url} in the default browser."),
        Err(reason) => format!("Launch browser skipped: {reason}"),
    }
}

/// The platform opener with the URL as its single argument: never a shell, never a command line
/// string. `None` means this platform has no opener wired (Windows is not supported yet).
fn command(url: &str) -> Option<Command> {
    let program = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "linux") {
        "xdg-open"
    } else {
        return None;
    };
    let mut command = Command::new(program);
    command.arg(url).stdout(Stdio::null()).stderr(Stdio::null());
    Some(command)
}

fn open(url: &str) -> Result<(), String> {
    let Some(mut command) = command(url) else {
        return Err("opening a browser is not supported on this platform.".to_string());
    };
    let mut child = command
        .spawn()
        .map_err(|error| format!("the system opener could not start ({error})."))?;
    // The opener exits on its own; waiting in a thread reaps it instead of leaving a zombie.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

fn wait_ready(target: &Target, alive: &dyn Fn() -> bool, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if connects(target) {
            return true;
        }
        if !alive() || Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(RETRY);
    }
}

fn connects(target: &Target) -> bool {
    let Ok(addresses) = (target.host.as_str(), target.port).to_socket_addrs() else {
        return false;
    };
    addresses
        .into_iter()
        .any(|address| TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).is_ok())
}

/// The scheme of an absolute URL (`http:`, `file:`, `javascript:`), or `None` for a relative one.
fn scheme(url: &str) -> Option<&str> {
    let (head, _) = url.split_once(':')?;
    let mut chars = head.chars();
    if !chars.next()?.is_ascii_alphabetic() {
        return None;
    }
    if head
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.'))
    {
        Some(head)
    } else {
        None
    }
}

fn parse(url: &str) -> Result<Target, String> {
    let scheme = scheme(url).ok_or_else(|| format!("{url} is not an absolute URL."))?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(format!(
            "{url} is not an http(s) URL; only http and https are opened."
        ));
    }
    let rest = url
        .split_once("://")
        .map(|(_, rest)| rest)
        .ok_or_else(|| format!("{url} is not an absolute URL."))?;
    let authority = rest
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .to_string();
    if authority.is_empty() {
        return Err(format!("{url} has no host."));
    }
    if authority.contains('@') {
        return Err(format!(
            "{url} carries credentials; only a plain host is opened."
        ));
    }
    // A Kestrel wildcard binds every interface; it is not an address a browser can visit.
    if matches!(authority.as_str(), "*" | "+")
        || authority.starts_with("*:")
        || authority.starts_with("+:")
    {
        return Err(format!(
            "{url} binds a wildcard address, which is not a URL to visit; use localhost."
        ));
    }
    let (host, port) = split_authority(&authority, &scheme)
        .ok_or_else(|| format!("{url} has no valid host and port."))?;
    if host.is_empty() {
        return Err(format!("{url} has no host."));
    }
    if port == 0 {
        return Err(format!(
            "{url} uses port 0, so the app picks a port the host cannot know; nothing was opened."
        ));
    }
    Ok(Target {
        scheme,
        authority,
        host,
        port,
    })
}

fn split_authority(authority: &str, scheme: &str) -> Option<(String, u16)> {
    let default = if scheme == "https" { 443 } else { 80 };
    // An IPv6 literal keeps its brackets and colons; only a port after `]` is a port.
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, tail) = rest.split_once(']')?;
        let port = match tail.strip_prefix(':') {
            Some(port) => port.parse().ok()?,
            None if tail.is_empty() => default,
            None => return None,
        };
        return Some((host.to_string(), port));
    }
    match authority.rsplit_once(':') {
        Some((host, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => {
            Some((host.to_string(), port.parse().ok()?))
        }
        Some(_) => None,
        None => Some((authority.to_string(), default)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_profile_without_launch_browser_never_opens() {
        assert_eq!(
            plan(false, Some("/page"), Some("http://localhost:5083")),
            Launch::None
        );
    }

    #[test]
    fn the_url_is_the_first_entry_of_the_effective_urls() {
        assert_eq!(
            plan(
                true,
                None,
                Some("https://localhost:7259;http://localhost:5083")
            ),
            Launch::Open("https://localhost:7259".to_string())
        );
        assert_eq!(
            plan(true, None, Some(" ; http://localhost:5083 ")),
            Launch::Open("http://localhost:5083".to_string())
        );
    }

    #[test]
    fn launch_url_is_appended_or_wins_when_absolute() {
        assert_eq!(
            plan(true, Some("/swagger"), Some("http://localhost:5083")),
            Launch::Open("http://localhost:5083/swagger".to_string())
        );
        assert_eq!(
            plan(true, Some("swagger"), Some("http://localhost:5083/")),
            Launch::Open("http://localhost:5083/swagger".to_string())
        );
        assert_eq!(
            plan(
                true,
                Some("https://example.com/x"),
                Some("http://localhost:5083")
            ),
            Launch::Open("https://example.com/x".to_string())
        );
    }

    #[test]
    fn a_profile_without_a_url_skips_with_its_reason() {
        assert!(
            matches!(plan(true, None, None), Launch::Skip(reason) if reason.contains("no applicationUrl"))
        );
        assert!(
            matches!(plan(true, Some("/page"), None), Launch::Skip(reason) if reason.contains("relative"))
        );
    }

    #[test]
    fn only_http_urls_with_a_host_and_a_real_port_are_opened() {
        let base = Some("http://localhost:5083");
        for (launch_url, expected) in [
            ("file:///etc/passwd", "not an http(s) URL"),
            ("javascript:alert(1)", "not an http(s) URL"),
            ("data:text/html,x", "not an http(s) URL"),
            ("http://:5083/x", "no host"),
            ("http://user:pass@localhost:5083", "credentials"),
            ("http://localhost:0", "port 0"),
            ("http://localhost:notaport", "no valid host"),
            ("http://*:5090", "wildcard"),
            ("http://+:5090", "wildcard"),
        ] {
            match plan(true, Some(launch_url), base) {
                Launch::Skip(reason) => assert!(
                    reason.contains(expected),
                    "{launch_url}: {reason} does not mention {expected}"
                ),
                other => panic!("{launch_url} must not open: {other:?}"),
            }
        }
        assert!(
            matches!(plan(true, None, Some("ftp://localhost:5083")), Launch::Skip(reason) if reason.contains("not an http(s)"))
        );
    }

    #[test]
    fn the_opener_takes_the_url_as_its_only_argument() {
        let Some(command) = command("http://localhost:5083/page?q=1&x=2") else {
            return; // Windows: no opener is wired, declared out of scope.
        };
        let program = command.get_program().to_string_lossy().to_string();
        assert!(program == "open" || program == "xdg-open", "{program}");
        let args: Vec<String> = command
            .get_args()
            .map(|argument| argument.to_string_lossy().to_string())
            .collect();
        assert_eq!(args, vec!["http://localhost:5083/page?q=1&x=2"]);
    }

    #[test]
    fn the_wait_returns_when_the_address_accepts_connections() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let target = parse(&format!("http://127.0.0.1:{port}/")).unwrap();
        assert!(wait_ready(&target, &|| true, Duration::from_secs(2)));
    }

    #[test]
    fn the_wait_gives_up_on_a_closed_port() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let target = parse(&format!("http://127.0.0.1:{port}/")).unwrap();
        assert!(!wait_ready(&target, &|| true, Duration::from_millis(300)));
    }

    #[test]
    fn the_wait_stops_when_the_launch_is_gone() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        let target = parse(&format!("http://127.0.0.1:{port}/")).unwrap();
        let started = Instant::now();
        assert!(!wait_ready(&target, &|| false, Duration::from_secs(5)));
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "{:?}",
            started.elapsed()
        );
    }

    #[test]
    fn a_url_that_cannot_be_opened_is_reported_instead_of_opened() {
        let line = open_when_ready("file:///etc/passwd", || true);
        assert!(line.starts_with("Launch browser skipped:"), "{line}");
        assert!(line.contains("http(s)"), "{line}");
    }
}
