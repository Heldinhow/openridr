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
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl TestProxy {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let hits = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let thread_hits = Arc::clone(&hits);
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
                let _ = stream.write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n");
                let _ = stream.flush();
            }
        });
        Self {
            port,
            hits,
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
    plan(url, &env_of(pairs)).expect("the plan must be valid").1
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
    assert_eq!(lowercase.label(), Some("proxy.corp:3128"));

    let fallback = route(
        "https://api.nuget.org/v3/index.json",
        pairs_of(&[("HTTP_PROXY", "http://proxy.corp:8080")]),
    );
    assert_eq!(fallback.label(), Some("proxy.corp:8080"));

    let all_proxy = route(
        "https://api.nuget.org/v3/index.json",
        pairs_of(&[("ALL_PROXY", "http://all.corp:9090")]),
    );
    assert_eq!(all_proxy.label(), Some("all.corp:9090"));

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
    assert_eq!(route.label(), Some("proxy.corp:3128"));
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
    let agent = agent_for_with(url, &env_of(vec![("HTTPS_PROXY", proxy.url())]))
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
    let (destination, route) = plan("https://api.nuget.org/v3/index.json", &env_of(Vec::new()))
        .expect("a public destination with no proxy is a valid plan");
    assert!(matches!(route, Route::Direct));
    assert_eq!(destination.host, "api.nuget.org");
    let agent = agent_for_with("https://api.nuget.org/v3/index.json", &env_of(Vec::new()))
        .expect("a direct route builds");
    drop(agent);
}
