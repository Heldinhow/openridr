//! Proxy-routing tests. The route decision runs against an injected environment so no test writes
//! to the process environment, and the end-to-end cases talk to a local test proxy only.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

use super::*;

/// An HTTP proxy that records the request line and answers `502`, so a test can prove the request
/// went to the proxy instead of the destination without reaching anything external.
struct TestProxy {
    port: u16,
    hits: Arc<Mutex<Vec<String>>>,
    heads: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl TestProxy {
    fn start() -> Self {
        Self::with_response("HTTP/1.1 502 Bad Gateway")
    }

    fn with_response(status_line: &'static str) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let hits = Arc::new(Mutex::new(Vec::new()));
        let heads = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread_hits = Arc::clone(&hits);
        let thread_heads = Arc::clone(&heads);
        let thread_stop = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming() {
                if thread_stop.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(mut stream) = stream else { continue };
                let mut buffer = [0u8; 4096];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]).to_string();
                let first = request
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .trim_end()
                    .to_string();
                thread_hits.lock().unwrap().push(first);
                thread_heads.lock().unwrap().push(request.clone());
                let response = format!("{status_line}\r\nContent-Length: 0\r\n\r\n");
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });
        Self {
            port,
            hits,
            heads,
            stop,
            handle: Some(handle),
        }
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    fn hits(&self) -> Vec<String> {
        self.hits.lock().unwrap().clone()
    }

    fn heads(&self) -> Vec<String> {
        self.heads.lock().unwrap().clone()
    }
}

impl Drop for TestProxy {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect(("127.0.0.1", self.port));
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn env_of(pairs: Vec<(&'static str, String)>) -> impl Fn(&str) -> Option<String> {
    move |name: &str| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| value.clone())
    }
}

fn route(url: &str, pairs: Vec<(&'static str, String)>) -> Route {
    plan(url, &env_of(pairs), None)
        .expect("the plan must be valid")
        .1
}

fn pairs_of(items: &[(&'static str, &str)]) -> Vec<(&'static str, String)> {
    items
        .iter()
        .map(|(key, value)| (*key, (*value).to_string()))
        .collect()
}

#[test]
fn proxy_variables_are_read_in_order_with_lowercase_first() {
    let lowercase = route(
        "https://api.nuget.org/v3/index.json",
        pairs_of(&[
            ("https_proxy", "http://proxy.corp:3128"),
            ("HTTPS_PROXY", "http://other.corp:8080"),
        ]),
    );
    assert_eq!(lowercase.described().as_deref(), Some("proxy.corp:3128"));

    let fallback = route(
        "https://api.nuget.org/v3/index.json",
        pairs_of(&[("HTTP_PROXY", "http://proxy.corp:8080")]),
    );
    assert_eq!(fallback.described().as_deref(), Some("proxy.corp:8080"));

    let all_proxy = route(
        "https://api.nuget.org/v3/index.json",
        pairs_of(&[("ALL_PROXY", "http://all.corp:9090")]),
    );
    assert_eq!(all_proxy.described().as_deref(), Some("all.corp:9090"));

    let direct = route("https://api.nuget.org/v3/index.json", pairs_of(&[]));
    assert!(matches!(direct, Route::Direct));

    let blank = route(
        "https://api.nuget.org/v3/index.json",
        pairs_of(&[("HTTPS_PROXY", "  ")]),
    );
    assert!(matches!(blank, Route::Direct));
}

#[test]
fn no_proxy_forms_send_the_destination_direct() {
    let cases: &[(&str, &str, bool)] = &[
        ("*", "api.nuget.org", true),
        ("api.nuget.org", "api.nuget.org", true),
        ("corp.example", "feed.corp.example", true),
        (".corp.example", "feed.corp.example", true),
        ("*.corp.example", "feed.corp.example", true),
        ("CORP.EXAMPLE", "feed.corp.example", true),
        (
            "other.example, feed.corp.example",
            "feed.corp.example",
            true,
        ),
        ("corp.example", "evilcorp.example", false),
        ("feed.corp.example:8080", "feed.corp.example", false),
        ("feed.corp.example:443", "feed.corp.example", true),
        ("", "feed.corp.example", false),
    ];
    for (list, host, expected) in cases {
        assert_eq!(
            no_proxy_matches(list, host, 443),
            *expected,
            "NO_PROXY={list} host={host}"
        );
    }
    let bypassed = route(
        "https://feed.corp.example/v3/index.json",
        pairs_of(&[
            ("HTTPS_PROXY", "http://proxy.corp:3128"),
            ("NO_PROXY", "corp.example"),
        ]),
    );
    assert!(
        matches!(bypassed, Route::Direct),
        "a NO_PROXY hit must win over the proxy variable"
    );
    let lowercase = route(
        "https://feed.corp.example/v3/index.json",
        pairs_of(&[
            ("HTTPS_PROXY", "http://proxy.corp:3128"),
            ("no_proxy", "*.corp.example"),
        ]),
    );
    assert!(matches!(lowercase, Route::Direct));
}

#[test]
fn a_bare_host_and_port_is_read_as_an_http_proxy() {
    let route = route(
        "https://api.nuget.org/v3/index.json",
        pairs_of(&[("HTTPS_PROXY", "proxy.corp:3128")]),
    );
    assert_eq!(route.described().as_deref(), Some("proxy.corp:3128"));
}

#[test]
fn an_unusable_proxy_value_fails_closed_without_leaking_credentials() {
    for (value, expected) in [
        ("socks5://user:s3cret@proxy.corp:1080", "socks5"),
        ("socks4://proxy.corp:1080", "socks4"),
        ("https://proxy.corp:3128", "https"),
    ] {
        let error = plan(
            "https://api.nuget.org/v3/index.json",
            &env_of(vec![("HTTPS_PROXY", value.to_string())]),
            None,
        )
        .unwrap_err();
        assert!(error.contains("HTTPS_PROXY"), "{error}");
        assert!(error.contains(expected), "{error}");
        assert!(!error.contains("s3cret"), "{error}");
        assert!(!error.contains("user"), "{error}");
    }
}

#[test]
fn a_malformed_proxy_value_fails_closed() {
    let error = plan(
        "https://api.nuget.org/v3/index.json",
        &env_of(vec![("HTTPS_PROXY", "http://".to_string())]),
        None,
    )
    .unwrap_err();
    assert!(error.contains("HTTPS_PROXY"), "{error}");
}

#[test]
fn destinations_are_parsed_with_default_ports_and_bracketed_ipv6() {
    assert_eq!(
        destination("https://api.nuget.org/v3/index.json").unwrap(),
        Destination {
            scheme: "https".to_string(),
            host: "api.nuget.org".to_string(),
            port: 443
        }
    );
    assert_eq!(
        destination("https://api.nuget.org:8443/v3/index.json").unwrap(),
        Destination {
            scheme: "https".to_string(),
            host: "api.nuget.org".to_string(),
            port: 8443
        }
    );
    assert_eq!(
        destination("https://[::1]:9/v3/index.json").unwrap().host,
        "::1"
    );
    assert!(destination("https://user:pass@api.nuget.org/v3/index.json").is_err());
    assert!(destination("ftp://api.nuget.org/").is_err());
}

#[test]
fn a_proxy_receives_the_connect_instead_of_the_destination() {
    let proxy = TestProxy::start();
    let url = "https://1.1.1.1/v3/index.json";
    let agent = agent_for_with(url, &env_of(vec![("HTTPS_PROXY", proxy.url())]), None)
        .expect("a local proxy is usable")
        .timeout(std::time::Duration::from_secs(10))
        .build();
    let error = agent.get(url).call().unwrap_err();
    assert!(
        format!("{error:?}").to_lowercase().contains("proxy"),
        "the failure must be reported as a proxy failure: {error:?}"
    );
    let hits = proxy.hits();
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(
        hits[0].starts_with("CONNECT 1.1.1.1:443"),
        "the proxy must be asked to reach the destination: {hits:?}"
    );
}

#[test]
fn a_proxied_private_destination_never_reaches_the_proxy() {
    if std::env::var("OPENRIDR_ENABLE_PROBE").as_deref() == Ok("1") {
        eprintln!("skipped: OPENRIDR_ENABLE_PROBE=1 opens the controlled-installer seam");
        return;
    }
    let proxy = TestProxy::start();
    let error = agent_for_with(
        "https://127.0.0.1:9/v3/index.json",
        &env_of(vec![("HTTPS_PROXY", proxy.url())]),
        None,
    )
    .unwrap_err();
    assert!(error.contains("127.0.0.1"), "{error}");
    assert!(error.to_lowercase().contains("refused"), "{error}");
    assert!(
        proxy.hits().is_empty(),
        "the guard must run before any connection: {:?}",
        proxy.hits()
    );
}

#[test]
fn a_public_destination_without_a_proxy_builds_direct() {
    let (destination, route) = plan(
        "https://api.nuget.org/v3/index.json",
        &env_of(Vec::new()),
        None,
    )
    .expect("a public destination with no proxy is a valid plan");
    assert!(matches!(route, Route::Direct));
    assert_eq!(destination.host, "api.nuget.org");
    let agent = agent_for_with(
        "https://api.nuget.org/v3/index.json",
        &env_of(Vec::new()),
        None,
    )
    .expect("a direct route builds");
    drop(agent);
}

#[test]
fn proxy_credentials_reach_the_client_but_never_the_label() {
    let proxy = TestProxy::start();
    let value = format!(
        "http://openridr:s3cret@{}/",
        proxy.url().trim_start_matches("http://")
    );
    let (destination, route) = plan(
        "https://1.1.1.1/v3/index.json",
        &env_of(vec![("HTTPS_PROXY", value)]),
        None,
    )
    .expect("an authenticated http proxy is usable");
    assert_eq!(destination.port, 443);
    let label = route.described().expect("a proxied route");
    assert_eq!(label, format!("127.0.0.1:{}", proxy.port));
    assert!(
        !label.contains("s3cret") && !label.contains("openridr"),
        "{label}"
    );

    let agent = agent_for_with(
        "https://1.1.1.1/v3/index.json",
        &env_of(vec![(
            "HTTPS_PROXY",
            format!("http://openridr:s3cret@127.0.0.1:{}", proxy.port),
        )]),
        None,
    )
    .expect("the client builds")
    .timeout(std::time::Duration::from_secs(10))
    .build();
    let _ = agent.get("https://1.1.1.1/v3/index.json").call();
    let heads = proxy.heads();
    assert_eq!(heads.len(), 1, "{heads:?}");
    // `openridr:s3cret` in base64: the proxy has to receive it, otherwise an authenticated proxy
    // could never work.
    let head = &heads[0];
    let authorization = head
        .lines()
        .find(|line| line.to_lowercase().starts_with("proxy-authorization:"))
        .expect("the CONNECT must carry the proxy credentials");
    assert!(
        authorization.ends_with("basic b3BlbnJpZHI6czNjcmV0"),
        "unexpected authorization header: {authorization}"
    );

    // The scheme-less form is what many corporate instructions show: the credentials must survive
    // it too, and the label must still be clean.
    let (_, route) = plan(
        "https://1.1.1.1/v3/index.json",
        &env_of(vec![(
            "HTTPS_PROXY",
            format!("openridr:s3cret@127.0.0.1:{}", proxy.port),
        )]),
        None,
    )
    .expect("a scheme-less proxy with credentials is usable");
    assert_eq!(
        route.described().as_deref(),
        Some(format!("127.0.0.1:{}", proxy.port).as_str())
    );
    let agent = agent_for_with(
        "https://1.1.1.1/v3/index.json",
        &env_of(vec![(
            "HTTPS_PROXY",
            format!("openridr:s3cret@127.0.0.1:{}", proxy.port),
        )]),
        None,
    )
    .expect("the client builds")
    .timeout(std::time::Duration::from_secs(10))
    .build();
    let _ = agent.get("https://1.1.1.1/v3/index.json").call();
    let heads = proxy.heads();
    let last = heads.last().expect("the second request reached the proxy");
    let authorization = last
        .lines()
        .find(|line| line.to_lowercase().starts_with("proxy-authorization:"))
        .expect("the scheme-less proxy must authenticate too");
    assert!(
        authorization.ends_with("basic b3BlbnJpZHI6czNjcmV0"),
        "unexpected authorization header: {authorization}"
    );
}

#[test]
fn relative_redirects_resolve_against_the_request_url() {
    let cases = [
        (
            "https://api.nuget.org/v3/package.nupkg",
            "package2.nupkg",
            "https://api.nuget.org/v3/package2.nupkg",
        ),
        (
            "https://api.nuget.org/v3/package.nupkg",
            "/other/file.nupkg",
            "https://api.nuget.org/other/file.nupkg",
        ),
        (
            "https://api.nuget.org/v3/a?x=1",
            "?y=2",
            "https://api.nuget.org/v3/a?y=2",
        ),
        (
            "https://api.nuget.org/v3/a",
            "//cdn.example/file",
            "https://cdn.example/file",
        ),
        (
            "https://api.nuget.org/v3/deep/a",
            "../b",
            "https://api.nuget.org/v3/b",
        ),
        ("https://api.nuget.org/a", "b", "https://api.nuget.org/b"),
        (
            "https://api.nuget.org/v3/a",
            "b#fragment",
            "https://api.nuget.org/v3/b",
        ),
    ];
    for (base, location, expected) in cases {
        assert_eq!(
            resolve_location(base, location),
            expected,
            "base={base} location={location}"
        );
    }
}

#[test]
fn an_impossible_no_proxy_port_does_not_match_every_port() {
    assert!(
        !no_proxy_matches("corp.example:99999", "corp.example", 443),
        "a port that cannot exist must not turn into 'any port'"
    );
    assert!(
        !no_proxy_matches("corp.example:99999", "corp.example", 65535),
        "the entry is broken, not a wildcard"
    );
    assert!(
        no_proxy_matches("corp.example:443", "corp.example", 443),
        "a valid port keeps working"
    );
}

#[test]
fn a_proxy_answer_of_407_is_explained_as_credentials() {
    let proxy = TestProxy::with_response("HTTP/1.1 407 Proxy Authentication Required");
    let url = "https://1.1.1.1/v3/index.json";
    let agent = agent_for_with(url, &env_of(vec![("HTTPS_PROXY", proxy.url())]), None)
        .expect("the client builds")
        .timeout(std::time::Duration::from_secs(10))
        .build();
    let error = agent.get(url).call().unwrap_err();
    let message = describe_via(
        url,
        &format!("{error:?}"),
        Some(&format!("127.0.0.1:{}", proxy.port)),
        None,
    );
    assert!(message.contains("407"), "{message}");
    assert!(message.contains("HTTPS_PROXY"), "{message}");
    assert!(
        message.contains(&format!("127.0.0.1:{}", proxy.port)),
        "{message}"
    );
}

#[test]
fn an_empty_no_proxy_does_not_shadow_the_uppercase_value() {
    let route = route(
        "https://feed.corp.example/v3/index.json",
        pairs_of(&[
            ("HTTPS_PROXY", "http://proxy.corp:3128"),
            ("no_proxy", ""),
            ("NO_PROXY", "corp.example"),
        ]),
    );
    assert!(
        matches!(route, Route::Direct),
        "an empty lowercase value means unset, so NO_PROXY must still be read"
    );
}

#[test]
fn a_redirect_to_plain_http_is_refused() {
    assert_eq!(
        resolve_location("https://api.nuget.org/v3/a", "/v3/b"),
        "https://api.nuget.org/v3/b"
    );
    let error = redirected("https://api.nuget.org/v3/a", "http://127.0.0.1/v3/b").unwrap_err();
    assert!(error.contains("only HTTPS destinations"), "{error}");
    assert!(error.contains("Nothing was downloaded"), "{error}");
    assert!(redirected("https://api.nuget.org/v3/a", "/v3/b").is_ok());
}

/// A macOS setting in the shape the reader produces. `unsupported` belongs to the diagnostics
/// ticket, so it stays empty here.
fn setting(
    https: Option<&str>,
    http: Option<&str>,
    exceptions: &[&str],
) -> crate::system_proxy::SystemProxy {
    crate::system_proxy::SystemProxy {
        https: https.map(str::to_string),
        http: http.map(str::to_string),
        exceptions: exceptions
            .iter()
            .map(|entry| (*entry).to_string())
            .collect(),
        unsupported: None,
    }
}

fn route_with_setting(url: &str, setting: &crate::system_proxy::SystemProxy) -> Route {
    plan(url, &env_of(Vec::new()), Some(setting))
        .expect("the plan must be valid")
        .1
}

#[test]
fn a_proxy_from_the_system_settings_is_used_when_the_environment_is_empty() {
    let setting = setting(Some("proxy.corp:3128"), Some("plain.corp:8080"), &[]);
    assert_eq!(
        route_with_setting("https://api.nuget.org/v3/index.json", &setting)
            .described()
            .as_deref(),
        Some("proxy.corp:3128 (macOS system proxy)")
    );
    assert_eq!(
        route_with_setting("http://api.nuget.org/v3/index.json", &setting)
            .described()
            .as_deref(),
        Some("plain.corp:8080 (macOS system proxy)"),
        "an http destination uses the http setting"
    );
}

#[test]
fn an_environment_proxy_wins_over_the_system_setting() {
    let setting = setting(Some("system.corp:3128"), None, &[]);
    let (_, route) = plan(
        "https://api.nuget.org/v3/index.json",
        &env_of(pairs_of(&[("HTTPS_PROXY", "http://env.corp:8080")])),
        Some(&setting),
    )
    .expect("the plan must be valid");
    assert_eq!(route.described().as_deref(), Some("env.corp:8080"));
}

/// The system source is the last item of #51's ordered list, so even `HTTP_PROXY` — the last resort
/// among the variables — wins over the Mac's `HTTPSProxy`. An environment that names a proxy is the
/// operator's decision for this app.
#[test]
fn an_environment_http_proxy_also_wins_over_the_system_setting() {
    let setting = setting(Some("system.corp:3128"), None, &[]);
    let (_, route) = plan(
        "https://api.nuget.org/v3/index.json",
        &env_of(pairs_of(&[("HTTP_PROXY", "http://env.corp:8080")])),
        Some(&setting),
    )
    .expect("the plan must be valid");
    assert_eq!(route.described().as_deref(), Some("env.corp:8080"));
}

#[test]
fn the_system_exceptions_send_the_destination_direct() {
    let setting = setting(Some("proxy.corp:3128"), None, &["*.local", ".corp.example"]);
    assert!(
        matches!(
            route_with_setting("https://feed.corp.example/v3/index.json", &setting),
            Route::Direct
        ),
        "an exception is a direct route"
    );
    assert_eq!(
        route_with_setting("https://api.nuget.org/v3/index.json", &setting)
            .described()
            .as_deref(),
        Some("proxy.corp:3128 (macOS system proxy)")
    );
}

#[test]
fn an_environment_no_proxy_wins_over_the_system_exceptions() {
    let setting = setting(Some("proxy.corp:3128"), None, &["corp.example"]);
    let (_, route) = plan(
        "https://feed.corp.example/v3/index.json",
        &env_of(pairs_of(&[("NO_PROXY", "other.example")])),
        Some(&setting),
    )
    .expect("the plan must be valid");
    assert_eq!(
        route.described().as_deref(),
        Some("proxy.corp:3128 (macOS system proxy)"),
        "the environment list is the only one read"
    );
}

#[test]
fn a_private_destination_never_reaches_a_system_proxy() {
    if std::env::var("OPENRIDR_ENABLE_PROBE").as_deref() == Ok("1") {
        eprintln!("skipped: OPENRIDR_ENABLE_PROBE=1 opens the controlled-installer seam");
        return;
    }
    let proxy = TestProxy::start();
    let setting = setting(Some(&format!("127.0.0.1:{}", proxy.port)), None, &[]);
    let error = agent_for_with(
        "https://127.0.0.1:9/v3/index.json",
        &env_of(Vec::new()),
        Some(&setting),
    )
    .unwrap_err();
    assert!(error.contains("127.0.0.1"), "{error}");
    assert!(error.to_lowercase().contains("refused"), "{error}");
    assert!(
        proxy.hits().is_empty(),
        "the guard must run before any connection: {:?}",
        proxy.hits()
    );
}

#[test]
fn a_system_proxy_receives_the_connect_instead_of_the_destination() {
    let proxy = TestProxy::start();
    let setting = setting(Some(&format!("127.0.0.1:{}", proxy.port)), None, &[]);
    let url = "https://1.1.1.1/v3/index.json";
    let agent = agent_for_with(url, &env_of(Vec::new()), Some(&setting))
        .expect("a local proxy is usable")
        .timeout(std::time::Duration::from_secs(10))
        .build();
    let error = agent.get(url).call().unwrap_err();
    assert!(
        format!("{error:?}").to_lowercase().contains("proxy"),
        "the failure must be reported as a proxy failure: {error:?}"
    );
    let hits = proxy.hits();
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(
        hits[0].starts_with("CONNECT 1.1.1.1:443"),
        "the proxy must be asked to reach the destination: {hits:?}"
    );
}

/// A message that names the system settings tells the person which knob to turn; a proxy from the
/// environment needs no origin, because the variable is the knob.
#[test]
fn the_system_proxy_label_names_its_origin() {
    let setting = setting(Some("proxy.corp:3128"), None, &[]);
    let (_, from_system) = plan(
        "https://api.nuget.org/v3/index.json",
        &env_of(Vec::new()),
        Some(&setting),
    )
    .expect("the plan must be valid");
    assert_eq!(
        from_system.described().as_deref(),
        Some("proxy.corp:3128 (macOS system proxy)")
    );
    let (_, from_env) = plan(
        "https://api.nuget.org/v3/index.json",
        &env_of(pairs_of(&[("HTTPS_PROXY", "http://env.corp:8080")])),
        Some(&setting),
    )
    .expect("the plan must be valid");
    assert_eq!(from_env.described().as_deref(), Some("env.corp:8080"));
}

#[test]
fn the_download_log_names_the_system_proxy_and_its_origin() {
    let setting = setting(Some("proxy.corp:3128"), None, &[]);
    let url = "https://github.com/o/r/asset.zip";
    let label = proxy_label_with(url, &env_of(Vec::new()), Some(&setting));
    assert_eq!(
        log_line(url, label.as_deref()),
        "GET https://github.com/o/r/asset.zip via proxy proxy.corp:3128 (macOS system proxy)"
    );
}

/// A Mac set to SOCKS has no proxy this client can use, so the failure has to say why instead of
/// leaving the person to guess that the NuGet feed is down.
#[test]
fn a_failure_with_socks_names_the_setting_and_the_way_out() {
    let message = describe_via(
        "https://api.nuget.org/v3/index.json",
        "Transport(ConnectionFailed)",
        None,
        Some("a SOCKS proxy"),
    );
    assert!(message.contains("a SOCKS proxy"), "{message}");
    assert!(message.contains("does not support"), "{message}");
    assert!(message.contains("HTTPS_PROXY"), "{message}");
}

#[test]
fn a_failure_with_a_pac_file_names_the_setting() {
    let message = describe_via(
        "https://api.nuget.org/v3/index.json",
        "Transport(ConnectionFailed)",
        None,
        Some("automatic proxy configuration (PAC/WPAD)"),
    );
    assert!(
        message.contains("automatic proxy configuration (PAC/WPAD)"),
        "{message}"
    );
    assert!(message.contains("HTTPS_PROXY"), "{message}");
}

/// A value from the system settings that the client cannot use names the settings, not a variable
/// the person never set.
#[test]
fn an_unusable_system_proxy_names_the_system_settings() {
    let setting = setting(Some("[::1]:8080"), None, &[]);
    let error = plan(
        "https://api.nuget.org/v3/index.json",
        &env_of(Vec::new()),
        Some(&setting),
    )
    .unwrap_err();
    assert!(error.contains("macOS system proxy"), "{error}");
    assert!(error.contains("IPv6"), "{error}");
    assert!(!error.contains("HTTPS_PROXY"), "{error}");
}
