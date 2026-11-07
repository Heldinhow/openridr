//! The macOS system proxy, read from System Settings without elevation.
//!
//! A Mac opened from Finder or the Dock never sees `HTTPS_PROXY` in its environment: the proxy
//! lives in System Settings, and `scutil --proxy` is the read-only way to read it. This module turns
//! that dictionary into the shape the route decision already knows ([`SystemProxy`]), so the HTTP
//! module only has to try one more source after the variables.

use std::path::{Path, PathBuf};

/// What the system settings say, in the shape the route decision needs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SystemProxy {
    /// `host:port` for an https destination: `HTTPSProxy`, falling back to `HTTPProxy`.
    pub https: Option<String>,
    /// `host:port` for an http destination.
    pub http: Option<String>,
    /// The macOS `ExceptionsList`, read with the same semantics as `NO_PROXY`.
    pub exceptions: Vec<String>,
    /// A setting this client cannot use, named for the diagnostics.
    pub unsupported: Option<&'static str>,
}

/// Reads the system proxy. `None` when the source is off, the platform has no `scutil` or the
/// command fails: the route then goes direct, exactly as before this source existed.
pub(crate) fn read() -> Option<SystemProxy> {
    let text = match source() {
        Source::Disabled => return None,
        Source::Scutil => run(Path::new(SCUTIL))?,
        Source::Program(program) => run(&program)?,
    };
    Some(parse(&text))
}

/// `scutil` prints the SystemConfiguration proxy dictionary; the read needs no elevation.
const SCUTIL: &str = "/usr/sbin/scutil";

fn run(program: &Path) -> Option<String> {
    let output = std::process::Command::new(program)
        .arg("--proxy")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

/// Where the proxy text comes from.
enum Source {
    Scutil,
    /// A program that prints the same dictionary. Development only, see [`source_for`].
    Program(PathBuf),
    Disabled,
}

/// `OPENRIDR_SYSTEM_PROXY=off` turns the source off; a program path replaces `scutil`. Replacing it
/// is a development seam, gated like the others: a release build accepts only `off` and ignores any
/// other value.
fn source() -> Source {
    source_for(
        std::env::var("OPENRIDR_SYSTEM_PROXY").ok().as_deref(),
        cfg!(debug_assertions) || crate::netguard::probe_enabled(),
    )
}

fn source_for(value: Option<&str>, development: bool) -> Source {
    match value.map(str::trim) {
        Some("off") => Source::Disabled,
        Some(program) if development && !program.is_empty() => {
            Source::Program(PathBuf::from(program))
        }
        _ => Source::Scutil,
    }
}

/// Interprets the `scutil --proxy` dictionary. Keys are the SystemConfiguration proxy properties
/// with the `kSCPropNetProxies` prefix stripped, which is what `scutil` prints.
pub(crate) fn parse(text: &str) -> SystemProxy {
    let mut values: Vec<(&str, &str)> = Vec::new();
    let mut exceptions: Vec<String> = Vec::new();
    let mut in_exceptions = false;
    for line in text.lines() {
        let line = line.trim();
        if in_exceptions {
            match line {
                "}" => in_exceptions = false,
                _ => {
                    if let Some((_, value)) = line.split_once(':') {
                        exceptions.push(value.trim().to_string());
                    }
                }
            }
            continue;
        }
        if let Some(key) = line.strip_suffix("<array> {") {
            in_exceptions = key.split(':').next().unwrap_or_default().trim() == "ExceptionsList";
            continue;
        }
        if let Some((key, value)) = line.split_once(':') {
            values.push((key.trim(), value.trim()));
        }
    }
    let enabled = |key: &str| {
        values
            .iter()
            .any(|(name, value)| *name == key && *value == "1")
    };
    let text_of = |key: &str| {
        values
            .iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| *value)
            .filter(|value| !value.is_empty())
    };
    let proxy = |host_key: &str, port_key: &str, default_port: u16| {
        text_of(host_key).map(|host| {
            let port = text_of(port_key)
                .and_then(|port| port.parse::<u16>().ok())
                .unwrap_or(default_port);
            // Bracketed, so an IPv6 setting reaches the route decision as one address instead of
            // failing as a malformed `host:port`.
            match host.contains(':') {
                true => format!("[{host}]:{port}"),
                false => format!("{host}:{port}"),
            }
        })
    };
    // A PAC file decides per URL and this client does not read PAC, so with it on the manual proxy
    // is not the setting the Mac is actually using.
    if enabled("ProxyAutoConfigEnable") || enabled("ProxyAutoDiscoveryEnable") {
        return SystemProxy {
            exceptions,
            unsupported: Some("automatic proxy configuration (PAC/WPAD)"),
            ..SystemProxy::default()
        };
    }
    let http = if enabled("HTTPEnable") {
        proxy("HTTPProxy", "HTTPPort", 80)
    } else {
        None
    };
    // CFNetwork uses the HTTP proxy for an https URL when no HTTPS proxy is configured, which is
    // the usual corporate setup (only "Web Proxy (HTTP)" filled in).
    let https = if enabled("HTTPSEnable") {
        proxy("HTTPSProxy", "HTTPSPort", 443)
    } else {
        None
    }
    .or_else(|| http.clone());
    let unsupported = if https.is_none() && http.is_none() && enabled("SOCKSEnable") {
        Some("a SOCKS proxy")
    } else {
        None
    };
    SystemProxy {
        https,
        http,
        exceptions,
        unsupported,
    }
}

#[cfg(test)]
#[path = "system_proxy_test.rs"]
mod tests;
