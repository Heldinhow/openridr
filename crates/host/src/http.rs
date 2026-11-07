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
    // The client needs the credentials; the label never leaves this function with them.
    let url = if value.contains("://") {
        format!("{scheme}://{rest}")
    } else {
        host_port.to_string()
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

/// Resolves a `Location` header against the URL that produced it.
pub(crate) fn resolve_location(base: &str, location: &str) -> String {
    if location.contains("://") {
        return location.to_string();
    }
    let (scheme, rest) = base.split_once("://").unwrap_or(("https", base));
    let authority = rest.split('/').next().unwrap_or_default();
    format!("{scheme}://{authority}{location}")
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

/// Downloads with up to [`MAX_DOWNLOAD_REDIRECTS`] hops, rebuilding the client for every one: a
/// redirect is a new destination, so the route and the destination guard have to run again. With a
/// proxy the resolver never sees the destination, so reusing one client would skip the guard.
pub(crate) fn fetch(url: &str, timeout: std::time::Duration) -> Result<ureq::Response, String> {
    let mut url = url.to_string();
    for _ in 0..=MAX_DOWNLOAD_REDIRECTS {
        let agent = agent_for(&url)?.redirects(0).timeout(timeout).build();
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
            Err(error) => return Err(describe_error(&url, &error)),
        };
        url = redirected(&url, &location)?;
    }
    Err(format!(
        "More than {MAX_DOWNLOAD_REDIRECTS} redirects from the download URL; the download was refused."
    ))
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
                    (name, port_text.parse::<u16>().ok())
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
}
