//! Address guard shared by every host HTTP request: the destination may not be loopback, private
//! or link-local, and the proxy may.
//!
//! The guard sits in two places because `ureq` uses one resolver for whatever it is about to
//! connect to. Without a proxy that is the destination, so [`GuardedResolver`] refuses it there
//! and `ureq` connects to the addresses it just checked. With a proxy the resolver only ever sees
//! the proxy's `host:port`, so the destination is verified before the request starts
//! ([`require_public_target`]) and the resolver keeps refusing any other private address.

use std::io;
use std::net::{IpAddr, SocketAddr, ToSocketAddrs};

/// True for every address a hostile NuGet source must not reach.
pub(crate) fn private_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let octets = ip.octets();
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_broadcast()
                || ip.is_multicast()
                || octets[0] == 0
                || (octets[0] == 100 && (64..=127).contains(&octets[1]))
                || (octets[0] == 198 && (18..=19).contains(&octets[1]))
        }
        IpAddr::V6(ip) => {
            // Mapped/compatible IPv4 and the NAT64 well-known prefix embed an IPv4 address
            // that must be classified as IPv4, not as a public IPv6 literal.
            if let Some(v4) = ip.to_ipv4() {
                return private_ip(IpAddr::V4(v4));
            }
            let segments = ip.segments();
            if segments[..6] == [0x0064, 0xff9b, 0, 0, 0, 0] {
                let v4 = std::net::Ipv4Addr::new(
                    (segments[6] >> 8) as u8,
                    segments[6] as u8,
                    (segments[7] >> 8) as u8,
                    segments[7] as u8,
                );
                return private_ip(IpAddr::V4(v4));
            }
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                || (segments[0] & 0xfe00) == 0xfc00
                || (segments[0] & 0xffc0) == 0xfe80
        }
    }
}

/// Classifies a host name without asking DNS, so a private literal never reaches a resolver.
pub(crate) fn private_host(host: &str) -> bool {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if let Ok(ip) = host.parse::<IpAddr>() {
        return private_ip(ip);
    }
    if host == "localhost"
        || host.ends_with(".localhost")
        || host.ends_with(".local")
        || host == "0.0.0.0"
        || host == "::1"
        || host.starts_with("fe80:")
        || host.starts_with("fc")
        || host.starts_with("fd")
    {
        return true;
    }
    let octets: Vec<u8> = host
        .split('.')
        .filter_map(|part| part.parse::<u8>().ok())
        .collect();
    if octets.len() == 4 {
        return octets[0] == 0
            || octets[0] == 10
            || octets[0] == 127
            || (octets[0] == 192 && octets[1] == 168)
            || (octets[0] == 169 && octets[1] == 254)
            || (octets[0] == 172 && (16..=31).contains(&octets[1]))
            || (octets[0] == 100 && (64..=127).contains(&octets[1]));
    }
    false
}

/// The controlled-installer seam: a debug build with the probe flag may download from a loopback
/// server. Release builds compile the escape out.
#[cfg(debug_assertions)]
fn private_targets_allowed() -> bool {
    std::env::var("OPENRIDR_ENABLE_PROBE").as_deref() == Ok("1")
}

#[cfg(not(debug_assertions))]
fn private_targets_allowed() -> bool {
    false
}

/// Refuses a destination before any connection when a proxy would hide it from the resolver.
/// A name that cannot be resolved locally is refused too: the proxy could reach what the host
/// cannot verify.
pub(crate) fn require_public_target(host: &str, port: u16) -> Result<(), String> {
    if private_targets_allowed() {
        return Ok(());
    }
    if private_host(host) {
        return Err(format!(
            "The destination host {host} is loopback, private or link-local and is refused."
        ));
    }
    let netloc = format!("{}:{port}", bracketed(host));
    let addresses: Vec<SocketAddr> = netloc.to_socket_addrs().map_err(|error| {
        format!(
            "The destination host {host} could not be verified locally ({error}), so the request is refused instead of being sent through the proxy."
        )
    })?
    .collect();
    if addresses.is_empty() {
        return Err(format!(
            "The destination host {host} has no address, so the request is refused."
        ));
    }
    if addresses.iter().any(|address| private_ip(address.ip())) {
        return Err(format!(
            "The destination host {host} resolves to a loopback, private or link-local address and is refused."
        ));
    }
    Ok(())
}

fn bracketed(host: &str) -> String {
    if host.contains(':') && !host.starts_with('[') {
        format!("[{host}]")
    } else {
        host.to_string()
    }
}

/// Refuses a request whose resolved address is loopback/private/link-local, except when the
/// netloc being resolved is the proxy the operator configured.
pub(crate) struct GuardedResolver {
    /// The proxy's `host:port`, exactly as `ureq` formats it before resolving.
    pub proxy: Option<String>,
}

impl ureq::Resolver for GuardedResolver {
    fn resolve(&self, netloc: &str) -> io::Result<Vec<SocketAddr>> {
        let addresses: Vec<SocketAddr> = netloc.to_socket_addrs()?.collect();
        if addresses.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "the destination host has no address",
            ));
        }
        let is_proxy = self.proxy.as_deref() == Some(netloc);
        if !is_proxy
            && !private_targets_allowed()
            && addresses.iter().any(|address| private_ip(address.ip()))
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "the destination host resolves to a loopback, private or link-local address",
            ));
        }
        Ok(addresses)
    }
}

#[cfg(test)]
#[path = "netguard_test.rs"]
mod tests;
