//! The one place the host builds an `ureq` client.
//!
//! Every direct request the host makes (NuGet Discover, the netcoredbg and csharp-ls fallback
//! downloads) goes through [`agent_for`], which
//!
//! 1. decides the route for that destination from the environment (`HTTPS_PROXY`/`HTTP_PROXY`/
//!    `ALL_PROXY`, lowercase first, honouring `NO_PROXY`),
//! 2. verifies the destination host before a proxy could hide it from the resolver, and
//! 3. installs a resolver that refuses a loopback/private/link-local destination while still
//!    allowing the proxy the operator configured.
//!
//! `proxy-from-env` stays off in `ureq`: the route is decided here, because `ureq` resolves the
//! *proxy* address with the resolver it is given, so letting it decide would silently skip the
//! destination guard (the reason it was turned off in #38).

#[cfg(test)]
#[path = "http_tls_trust_test.rs"]
mod tls_tests;

#[cfg(test)]
#[path = "http_proxy_test.rs"]
mod proxy_tests;

/// Builds the agent every host request starts from. The route is decided by [`agent_for`], which
/// is the only public way in; nothing here picks a proxy up from the environment on its own.
fn agent_builder() -> ureq::AgentBuilder {
    agent_builder_with_certs(rustls_native_certs::load_native_certs().unwrap_or_default())
}

fn agent_builder_with_certs(
    certs: Vec<rustls::pki_types::CertificateDer<'static>>,
) -> ureq::AgentBuilder {
    let mut roots = rustls::RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };
    roots.add_parsable_certificates(certs);
    let tls = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    ureq::AgentBuilder::new()
        .try_proxy_from_env(false)
        .tls_config(std::sync::Arc::new(tls))
}

/// The client for one destination in this process's environment. The caller sets timeouts,
/// redirect policy and limits; the proxy and the address guard are not the caller's business.
pub(crate) fn agent_for(url: &str) -> Result<ureq::AgentBuilder, String> {
    agent_for_with(url, &process_env)
}

/// The log line for one download request: the URL, plus the proxy when one is in use. Credentials
/// never reach this text because [`proxy_label`] only keeps `host:port`.
pub(crate) fn log_line(url: &str, proxy: Option<&str>) -> String {
    match proxy {
        Some(proxy) => format!("GET {url} via proxy {proxy}"),
        None => format!("GET {url}"),
    }
}

/// The proxy label (`host:port`, never credentials) used for one URL, for logs and diagnostics.
pub(crate) fn proxy_label(url: &str) -> Option<String> {
    match plan(url, &process_env) {
        Ok((_, route)) => route.label().map(str::to_string),
        Err(_) => None,
    }
}

fn process_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// One destination URL, as the route decision needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Destination {
    scheme: String,
    host: String,
    port: u16,
}

#[derive(Debug, Clone)]
struct ProxyTarget {
    /// What `ureq` receives: the raw value, name for a `host:port` without scheme.
    url: String,
    /// `host:port`, which is both the safe label and the netloc `ureq` resolves.
    netloc: String,
}

impl ProxyTarget {
    fn proxy(&self) -> ureq::Proxy {
        match ureq::Proxy::new(&self.url) {
            Ok(proxy) => proxy,
            // Unreachable: `proxy_target` parses the same value before returning it.
            Err(_) => ureq::Proxy::new(format!("http://{}", self.netloc))
                .expect("a parsed http proxy stays valid"),
        }
    }
}

#[derive(Debug, Clone)]
enum Route {
    Direct,
    Proxied(ProxyTarget),
}

impl Route {
    fn label(&self) -> Option<&str> {
        match self {
            Route::Direct => None,
            Route::Proxied(target) => Some(&target.netloc),
        }
    }
}

fn plan(url: &str, env: &dyn Fn(&str) -> Option<String>) -> Result<(Destination, Route), String> {
    let destination = destination(url)?;
    let route = route_for(&destination, env)?;
    Ok((destination, route))
}

fn agent_for_with(
    url: &str,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<ureq::AgentBuilder, String> {
    let (destination, route) = plan(url, env)?;
    match route {
        Route::Direct => {
            Ok(agent_builder().resolver(crate::netguard::GuardedResolver { proxy: None }))
        }
        Route::Proxied(target) => {
            // `ureq` hands the resolver the proxy's netloc, never the destination: the guard has
            // to run here, before any connection is made.
            crate::netguard::require_public_target(&destination.host, destination.port)?;
            Ok(agent_builder()
                .proxy(target.proxy())
                .resolver(crate::netguard::GuardedResolver {
                    proxy: Some(target.netloc.clone()),
                }))
        }
    }
}

fn destination(url: &str) -> Result<Destination, String> {
    let (scheme, rest) = url
        .split_once("://")
        .ok_or_else(|| format!("{url} is not an absolute URL."))?;
    let scheme = scheme.to_ascii_lowercase();
    if scheme != "https" && scheme != "http" {
        return Err(format!(
            "{url}: the host only makes HTTP and HTTPS requests, so {scheme} is refused."
        ));
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.contains('@') {
        return Err(format!("{url}: a URL with credentials is refused."));
    }
    let default_port = if scheme == "https" { 443 } else { 80 };
    let (host, port) = split_host_port(authority, default_port)
        .ok_or_else(|| format!("{url}: the host part of the URL is invalid."))?;
    Ok(Destination { scheme, host, port })
}

/// Splits `host[:port]`, accepting `[ipv6]` and `[ipv6]:port`, and returning the host without
/// brackets so it can be classified and re-bracketed for resolution.
fn split_host_port(text: &str, default_port: u16) -> Option<(String, u16)> {
    if let Some(rest) = text.strip_prefix('[') {
        let (host, after) = rest.split_once(']')?;
        if host.is_empty() {
            return None;
        }
        return match after.strip_prefix(':') {
            Some(port) => port
                .parse::<u16>()
                .ok()
                .map(|port| (host.to_string(), port)),
            None if after.is_empty() => Some((host.to_string(), default_port)),
            None => None,
        };
    }
    let (host, port) = match text.rsplit_once(':') {
        Some((host, port)) if !host.contains(':') => (host, Some(port)),
        Some(_) => return None,
        None => (text, None),
    };
    if host.is_empty() {
        return None;
    }
    match port {
        Some(port) => port
            .parse::<u16>()
            .ok()
            .map(|port| (host.to_string(), port)),
        None => Some((host.to_string(), default_port)),
    }
}

/// Proxy variables in the order this client reads them. Lowercase wins (the `curl` convention,
/// which keeps a CGI-style `HTTP_PROXY` from silently taking over); `HTTP_PROXY` is the last
/// resort for an HTTPS destination because most corporate instructions only set that one.
const HTTPS_PROXY_VARS: &[&str] = &[
    "https_proxy",
    "HTTPS_PROXY",
    "all_proxy",
    "ALL_PROXY",
    "http_proxy",
    "HTTP_PROXY",
];
const HTTP_PROXY_VARS: &[&str] = &["http_proxy", "HTTP_PROXY", "all_proxy", "ALL_PROXY"];

fn route_for(
    destination: &Destination,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<Route, String> {
    let no_proxy = env("no_proxy")
        .filter(|value| !value.trim().is_empty())
        .or_else(|| env("NO_PROXY").filter(|value| !value.trim().is_empty()));
    if let Some(list) = no_proxy {
        if no_proxy_matches(&list, &destination.host, destination.port) {
            return Ok(Route::Direct);
        }
    }
    let variables = if destination.scheme == "https" {
        HTTPS_PROXY_VARS
    } else {
        HTTP_PROXY_VARS
    };
    for variable in variables {
        let value = env(variable).filter(|value| !value.trim().is_empty());
        if let Some(value) = value {
            return Ok(Route::Proxied(proxy_target(variable, &value)?));
        }
    }
    Ok(Route::Direct)
}

/// Parses one proxy value. A value this client cannot use is an error, never a silent direct
/// connection: falling back would break the network policy the proxy exists to enforce.
fn proxy_target(variable: &str, value: &str) -> Result<ProxyTarget, String> {
    let value = value.trim();
    let (scheme, rest) = match value.split_once("://") {
        Some((scheme, rest)) => (scheme.to_ascii_lowercase(), rest),
        None => ("http".to_string(), value),
    };
    if scheme != "http" {
        return Err(format!(
            "{variable} points at a {scheme} proxy; this client only supports http:// proxies. The request is refused instead of going direct."
        ));
    }
    let host_port = match rest.rsplit_once('@') {
        Some((_, host_port)) => host_port,
        None => rest,
    };
    let host_port = host_port.trim_end_matches('/');
    let (host, port) = split_host_port(host_port, 80).ok_or_else(|| {
        format!("{variable} is not a valid proxy URL; expected http://host:port.")
    })?;
    if host.contains(':') {
        return Err(format!(
            "{variable} points at an IPv6 proxy, which this client cannot reach; use the proxy host name or its IPv4 address."
        ));
    }
    let netloc = format!("{host}:{port}");
    // The client needs the credentials; the label never leaves this function with them. Without a
    // scheme `ureq` reads `user:pass@host:port` itself, so the raw value is what it gets.
    let url = if value.contains("://") {
        format!("{scheme}://{rest}")
    } else {
        value.to_string()
    };
    if ureq::Proxy::new(&url).is_err() {
        return Err(format!(
            "{variable}: the proxy {netloc} could not be parsed; expected http://host:port."
        ));
    }
    Ok(ProxyTarget { url, netloc })
}

/// How many redirects one download may follow before it is refused.
const MAX_DOWNLOAD_REDIRECTS: usize = 5;

/// Resolves a `Location` header against the URL that produced it, following the reference forms
/// HTTP actually sends: absolute, scheme-relative (`//host/path`), absolute path, query-only and
/// path-relative (`file.nupkg`, `../x`). A fragment is dropped: it is never sent.
pub(crate) fn resolve_location(base: &str, location: &str) -> String {
    let location = location.split('#').next().unwrap_or_default();
    if location.contains("://") {
        return location.to_string();
    }
    let (scheme, rest) = base.split_once("://").unwrap_or(("https", base));
    let (authority, path_and_query) = match rest.find('/') {
        Some(index) => (&rest[..index], &rest[index..]),
        None => (rest, "/"),
    };
    if let Some(network) = location.strip_prefix("//") {
        return format!("{scheme}://{network}");
    }
    let path = path_and_query.split('?').next().unwrap_or(path_and_query);
    if let Some(query) = location.strip_prefix('?') {
        return format!("{scheme}://{authority}{path}?{query}");
    }
    if location.starts_with('/') {
        return format!("{scheme}://{authority}{}", normalize_path(location));
    }
    let directory = match path.rfind('/') {
        Some(index) => &path[..=index],
        None => "/",
    };
    let joined = normalize_path(&format!("{directory}{location}"));
    format!("{scheme}://{authority}{joined}")
}

/// Removes `.` and `..` segments from an absolute path, keeping the leading slash.
fn normalize_path(path: &str) -> String {
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop();
            }
            other => segments.push(other),
        }
    }
    let trailing = path.ends_with('/') && !segments.is_empty();
    let mut normalized = format!("/{}", segments.join("/"));
    if trailing {
        normalized.push('/');
    }
    normalized
}

/// A redirect target must stay on HTTPS: a proxy or a compromised host cannot downgrade the
/// download to plain HTTP.
fn redirected(url: &str, location: &str) -> Result<String, String> {
    let next = resolve_location(url, location);
    if !next.starts_with("https://") {
        return Err(format!(
            "Refused to follow a redirect from {url} to {next}: only HTTPS destinations are followed. Nothing was downloaded."
        ));
    }
    Ok(next)
}

/// A download is bounded by inactivity, not by a total deadline: a slow link that keeps receiving
/// bytes finishes, and a connection that stops sending fails after this long without data. A total
/// deadline is what made the csharp-ls download fail on a slow network (#161), and `ureq` gives a
/// total timeout precedence over this one, so the two cannot be combined.
pub(crate) const DOWNLOAD_STALL: std::time::Duration = std::time::Duration::from_secs(60);

/// A connection attempt must not hang setup either. The same bound the netcoredbg download uses.
///
/// The one wait this does not bound is name resolution: `ureq` cannot interrupt it, and the total
/// deadline it replaced could not either (it is checked between operations, after the resolver
/// returns). The system resolver keeps its own timeouts, so a hung lookup is a pre-existing limit of
/// every host request, not something this download introduced.
pub(crate) const DOWNLOAD_CONNECT: std::time::Duration = std::time::Duration::from_secs(15);

/// The client every download starts from: the route and the destination guard of [`agent_for`], no
/// redirects of its own (each hop is a new destination) and the inactivity policy in one place —
/// [`DOWNLOAD_CONNECT`] for a connection that never opens, `stall` for a socket that stops sending
/// or a write that blocks because the peer stopped reading. There is deliberately no total deadline.
pub(crate) fn download_agent(
    url: &str,
    stall: std::time::Duration,
) -> Result<ureq::AgentBuilder, String> {
    download_agent_with(url, stall, DOWNLOAD_CONNECT, &process_env)
}

/// `connect` is a parameter only so a test can prove that a connection that never opens is not
/// reported as a stall; the product always passes [`DOWNLOAD_CONNECT`].
fn download_agent_with(
    url: &str,
    stall: std::time::Duration,
    connect: std::time::Duration,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<ureq::AgentBuilder, String> {
    Ok(agent_for_with(url, env)?
        .redirects(0)
        .timeout_connect(connect)
        .timeout_read(stall)
        .timeout_write(stall))
}

/// Downloads with up to [`MAX_DOWNLOAD_REDIRECTS`] hops, rebuilding the client for every one: a
/// redirect is a new destination, so the route and the destination guard have to run again. With a
/// proxy the resolver never sees the destination, so reusing one client would skip the guard.
pub(crate) fn fetch(url: &str, stall: std::time::Duration) -> Result<ureq::Response, String> {
    fetch_with(url, stall, DOWNLOAD_CONNECT, &process_env)
}

/// `connect` is a parameter only so a test can prove that a connection that never opens is not
/// reported as a stall; the product always passes [`DOWNLOAD_CONNECT`].
fn fetch_with(
    url: &str,
    stall: std::time::Duration,
    connect: std::time::Duration,
    env: &dyn Fn(&str) -> Option<String>,
) -> Result<ureq::Response, String> {
    let mut url = url.to_string();
    for _ in 0..=MAX_DOWNLOAD_REDIRECTS {
        let agent = download_agent_with(&url, stall, connect, env)?.build();
        let location = match agent.get(&url).call() {
            Ok(response) if (300..400).contains(&response.status()) => {
                let status = response.status();
                match response.header("location") {
                    Some(location) => location.to_string(),
                    None => {
                        return Err(format!(
                            "The server answered {status} for {url} without a Location; nothing was downloaded."
                        ))
                    }
                }
            }
            Ok(response) => return Ok(response),
            Err(ureq::Error::Status(code, response)) if (300..400).contains(&code) => {
                match response.header("location") {
                    Some(location) => location.to_string(),
                    None => {
                        return Err(format!(
                            "The server answered {code} for {url} without a Location; nothing was downloaded."
                        ))
                    }
                }
            }
            Err(error) => {
                return Err(if timed_out(&error) {
                    stalled(&url, stall)
                } else {
                    describe_error(&url, &error)
                })
            }
        };
        url = redirected(&url, &location)?;
    }
    Err(format!(
        "More than {MAX_DOWNLOAD_REDIRECTS} redirects from the download URL; the download was refused."
    ))
}

/// The app's words for a download that stopped receiving data. The response head and the body use
/// the same sentence: for the person reading it, they are the same failure.
pub(crate) fn stalled(url: &str, stall: std::time::Duration) -> String {
    format!(
        "{url}: the connection stalled — no data arrived for {} seconds. Nothing was installed or replaced; check the network and retry the setup.",
        stall.as_secs()
    )
}

/// True when a body read gave up because the socket stopped answering. `ureq` normalises the
/// platform's `WouldBlock` to `TimedOut`, so this is the one kind to look for.
pub(crate) fn is_stall(error: &std::io::Error) -> bool {
    error.kind() == std::io::ErrorKind::TimedOut
}

/// True when the transport failed on a read timeout. The `ureq` error carries the `io::Error` in
/// its source chain; the message text is never inspected. A connect failure is not a stall — the
/// socket never opened, so no data was ever expected — and `ureq` reports it as
/// `ConnectionFailed`, which is why the kind is checked before the chain.
pub(crate) fn timed_out(error: &ureq::Error) -> bool {
    if error.kind() != ureq::ErrorKind::Io {
        return false;
    }
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        if cause.downcast_ref::<std::io::Error>().is_some_and(is_stall) {
            return true;
        }
        source = cause.source();
    }
    false
}

/// True when `NO_PROXY` says this destination goes direct.
fn no_proxy_matches(list: &str, host: &str, port: u16) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    list.split([',', ' ', '\t', '\n'])
        .map(|entry| entry.trim().to_ascii_lowercase())
        .filter(|entry| !entry.is_empty())
        .any(|entry| {
            if entry == "*" {
                return true;
            }
            // `*.corp.example` is read as the same suffix as `.corp.example`.
            let entry = entry.strip_prefix('*').unwrap_or(&entry);
            let (name, wanted_port) = match entry.rsplit_once(':') {
                Some((name, port_text))
                    if !name.contains(':')
                        && !port_text.is_empty()
                        && port_text
                            .chars()
                            .all(|character| character.is_ascii_digit()) =>
                {
                    match port_text.parse::<u16>() {
                        Ok(port) => (name, Some(port)),
                        // A port that cannot exist is a broken entry, not "any port": matching it
                        // would send the destination direct when the operator named a port.
                        Err(_) => return false,
                    }
                }
                _ => (entry, None),
            };
            if let Some(wanted) = wanted_port {
                if wanted != port {
                    return false;
                }
            }
            let name = name.trim_end_matches('.').trim_start_matches('.');
            if name.is_empty() {
                return false;
            }
            host == name || host.ends_with(&format!(".{name}"))
        })
}

pub(crate) fn describe_error(url: &str, error: &ureq::Error) -> String {
    let detail = format!("{} {error:?}", error);
    describe_via(url, &detail, proxy_label(url).as_deref())
}

fn describe_via(url: &str, detail: &str, configured: Option<&str>) -> String {
    let diagnostic = detail.to_ascii_lowercase();
    let proxy = configured.unwrap_or("the configured proxy");
    // `ureq` renders a 407 on the CONNECT as `ProxyUnauthorized` / "Provided proxy credentials are
    // incorrect"; a plain `Status(407, ..)` only appears for a proxy answering in absolute form.
    let unauthorized = [
        "proxy unauthorized",
        "proxyunauthorized",
        "proxy credentials",
        "status(407",
    ]
    .iter()
    .any(|needle| diagnostic.contains(needle));
    if unauthorized {
        return format!(
            "The proxy at {proxy} rejected the request for {url} (HTTP 407: authentication required). Set the credentials in HTTPS_PROXY and retry, or ask your network administrator. Details: {detail}"
        );
    }
    let proxy_refused = [
        "proxy connect",
        "proxyconnect",
        "proxy protocol",
        "proxyprotocol",
    ]
    .iter()
    .any(|needle| diagnostic.contains(needle));
    if proxy_refused {
        return format!("The proxy at {proxy} refused the connection to {url}. Details: {detail}");
    }
    if diagnostic.contains("unknownissuer")
        || diagnostic.contains("unknown issuer")
        || diagnostic.contains("certificate")
    {
        return format!(
            "The network certificate for {url} is not trusted, possibly because a corporate proxy or TLS inspection is in use. Ask your network administrator to trust its CA in the system Keychain, then retry. Details: {detail}"
        );
    }
    match configured {
        Some(proxy) => format!("{url}: {detail} (via proxy {proxy})"),
        None => format!("{url}: {detail}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_errors_explain_network_trust_and_keep_the_url() {
        let message = describe_via(
            "https://api.nuget.org/v3/index.json",
            "invalid peer certificate: UnknownIssuer",
            None,
        );
        assert!(message.contains("https://api.nuget.org/v3/index.json"));
        assert!(message.contains("network certificate"));
        assert!(message.contains("system Keychain"));
    }

    #[test]
    fn the_download_log_names_the_proxy_only_when_there_is_one() {
        assert_eq!(
            log_line("https://github.com/o/r/asset.zip", Some("proxy.corp:3128")),
            "GET https://github.com/o/r/asset.zip via proxy proxy.corp:3128"
        );
        assert_eq!(
            log_line("https://github.com/o/r/asset.zip", None),
            "GET https://github.com/o/r/asset.zip"
        );
    }

    #[test]
    fn proxy_errors_name_the_proxy_and_never_the_credentials() {
        let unauthorized = describe_via(
            "https://api.nuget.org/v3/index.json",
            "Transport(Status(407, Response))",
            Some("proxy.corp:3128"),
        );
        assert!(unauthorized.contains("proxy.corp:3128"));
        assert!(unauthorized.contains("407"));
        let refused = describe_via(
            "https://api.nuget.org/v3/index.json",
            "Transport(ProxyConnect)",
            Some("proxy.corp:3128"),
        );
        assert!(refused.contains("proxy.corp:3128"));
        for message in [unauthorized, refused] {
            assert!(!message.contains("s3cret"), "{message}");
        }
    }

    /// A local server answering one download in `chunks` pieces, `delay` apart, used as the proxy
    /// for a public literal destination: the address guard allows the configured proxy, so the test
    /// needs no DNS and no external network. `declared` is the `Content-Length` it promises, which a
    /// stalling server promises and does not keep.
    fn serve_download(
        declared: usize,
        chunks: usize,
        size: usize,
        delay: std::time::Duration,
    ) -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut request = [0u8; 4096];
            let _ = std::io::Read::read(&mut stream, &mut request);
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {declared}\r\nConnection: close\r\n\r\n"
            );
            if std::io::Write::write_all(&mut stream, head.as_bytes()).is_err() {
                return;
            }
            for _ in 0..chunks {
                if std::io::Write::write_all(&mut stream, &vec![7u8; size]).is_err() {
                    return;
                }
                let _ = std::io::Write::flush(&mut stream);
                std::thread::sleep(delay);
            }
        });
        format!("http://{address}")
    }

    fn proxy_env(proxy: &str) -> impl Fn(&str) -> Option<String> {
        let proxy = proxy.to_string();
        move |name: &str| (name == "http_proxy").then(|| proxy.clone())
    }

    /// A server that accepts the connection and never answers, like a proxy that took the request
    /// and died.
    fn silent_server() -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut request = [0u8; 4096];
            let _ = std::io::Read::read(&mut stream, &mut request);
            std::thread::sleep(std::time::Duration::from_secs(3));
        });
        format!("http://{address}")
    }

    const PACKAGE_URL: &str = "http://1.1.1.1/csharp-ls.0.28.0.nupkg";

    /// The bug in #161: a total deadline killed a download that was still receiving bytes. Twelve
    /// chunks 200 ms apart take ~2.4 s, well past the 1 s window below, and every byte must arrive.
    /// The 5x gap between the send interval and the window keeps the test off the scheduler's back.
    #[test]
    fn a_download_that_keeps_receiving_bytes_finishes_however_long_it_takes() {
        let proxy = serve_download(12 * 4096, 12, 4096, std::time::Duration::from_millis(200));
        let response = fetch_with(
            PACKAGE_URL,
            std::time::Duration::from_secs(1),
            DOWNLOAD_CONNECT,
            &proxy_env(&proxy),
        )
        .expect("the proxy answers");
        let mut bytes = Vec::new();
        std::io::Read::read_to_end(&mut response.into_reader(), &mut bytes)
            .expect("the whole body arrives");
        assert_eq!(bytes.len(), 12 * 4096);
    }

    /// The other half of #161: a connection that really stops has to fail, and the person has to
    /// read the app's sentence instead of the transport's.
    #[test]
    fn a_download_that_stops_receiving_bytes_is_reported_as_a_stall() {
        // Three bytes of a promised thousand, then silence: the head arrives, the body stops.
        let proxy = serve_download(1000, 1, 3, std::time::Duration::from_secs(3));
        let response = fetch_with(
            PACKAGE_URL,
            std::time::Duration::from_secs(1),
            DOWNLOAD_CONNECT,
            &proxy_env(&proxy),
        )
        .expect("the head arrives");
        let mut bytes = Vec::new();
        let error = std::io::Read::read_to_end(&mut response.into_reader(), &mut bytes)
            .expect_err("the body stops");
        assert!(is_stall(&error), "a read timeout is a stall: {error:?}");
        let message = stalled(PACKAGE_URL, DOWNLOAD_STALL);
        assert!(message.contains(PACKAGE_URL), "{message}");
        assert!(message.contains("stalled"), "{message}");
        assert!(message.contains("60 seconds"), "{message}");
        assert!(
            message.contains("Nothing was installed or replaced"),
            "{message}"
        );
        assert!(message.contains("retry the setup"), "{message}");
    }

    #[test]
    fn a_server_that_never_answers_is_reported_as_a_stall() {
        let proxy = silent_server();
        let error = fetch_with(
            PACKAGE_URL,
            std::time::Duration::from_secs(1),
            DOWNLOAD_CONNECT,
            &proxy_env(&proxy),
        )
        .expect_err("nothing arrives");
        assert!(error.contains("stalled"), "{error}");
        assert!(error.contains("retry the setup"), "{error}");
        assert!(
            !error.contains("Network Error"),
            "the raw transport text must not reach the user: {error}"
        );
    }

    /// A connection that never opens is a different failure from a transfer that stopped: the
    /// sentence must not claim that data stopped arriving when no data was ever expected. TEST-NET-1
    /// answers nothing, so the 300 ms connect bound is what ends the attempt; an environment that
    /// refuses it outright produces the same `ConnectionFailed` kind.
    #[test]
    fn a_connection_that_never_opens_is_not_reported_as_a_stall() {
        let error = fetch_with(
            "http://192.0.2.1/csharp-ls.0.28.0.nupkg",
            DOWNLOAD_STALL,
            std::time::Duration::from_millis(300),
            &|_: &str| None,
        )
        .expect_err("the connection never opens");
        assert!(
            !error.contains("stalled"),
            "a connect failure is not a stall: {error}"
        );
        assert!(!error.contains("no data arrived"), "{error}");
    }
}
