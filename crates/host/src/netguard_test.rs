//! Address-guard tests. Everything here uses literal addresses, so nothing resolves a public name
//! and the suite stays offline.

use std::net::IpAddr;

use super::*;
use ureq::Resolver;

/// The guard's private-address escape is the controlled-installer seam; a shell that exports it
/// would make every "refused" assertion below meaningless, so say so instead of failing obscurely.
fn probe_leaks_into_this_run() -> bool {
    std::env::var("OPENRIDR_ENABLE_PROBE").as_deref() == Ok("1")
}

#[test]
fn private_addresses_are_classified_for_both_families() {
    for address in [
        "127.0.0.1",
        "10.1.2.3",
        "172.16.0.1",
        "172.31.255.255",
        "192.168.0.1",
        "169.254.169.254",
        "0.0.0.0",
        "100.64.0.1",
        "198.18.0.1",
        "224.0.0.1",
        "::1",
        "::",
        "fc00::1",
        "fd12::1",
        "fe80::1",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "64:ff9b::7f00:1",
        "64:ff9b::10.0.0.1",
    ] {
        assert!(
            private_ip(address.parse::<IpAddr>().unwrap()),
            "{address} must be private"
        );
    }
    for address in ["1.1.1.1", "93.184.216.34", "8.8.8.8", "2606:4700::1111"] {
        assert!(
            !private_ip(address.parse::<IpAddr>().unwrap()),
            "{address} must be public"
        );
    }
}

#[test]
fn private_hosts_are_refused_without_asking_dns() {
    for host in [
        "localhost",
        "api.localhost",
        "server.local",
        "127.0.0.1",
        "10.0.0.2",
        "192.168.1.2",
        "169.254.1.2",
        "0.0.0.0",
        "100.64.0.1",
        "::1",
        "fe80::1",
        "fc00::1",
    ] {
        assert!(private_host(host), "{host} must be private");
    }
    for host in ["api.nuget.org", "github.com", "1.1.1.1"] {
        assert!(!private_host(host), "{host} must be public");
    }
}

#[test]
fn the_resolver_refuses_a_private_destination_and_allows_the_configured_proxy() {
    if probe_leaks_into_this_run() {
        eprintln!("skipped: OPENRIDR_ENABLE_PROBE=1 opens the controlled-installer seam");
        return;
    }
    let direct = GuardedResolver { proxy: None };
    for netloc in [
        "127.0.0.1:443",
        "10.0.0.1:443",
        "169.254.169.254:80",
        "[::1]:443",
    ] {
        assert!(direct.resolve(netloc).is_err(), "{netloc} must be refused");
    }

    let proxied = GuardedResolver {
        proxy: Some("127.0.0.1:3128".to_string()),
    };
    assert!(
        proxied.resolve("127.0.0.1:3128").is_ok(),
        "the operator's proxy may live on loopback"
    );
    assert!(
        proxied.resolve("127.0.0.1:3129").is_err(),
        "every other loopback netloc is still refused"
    );
}

#[test]
fn a_proxied_destination_is_verified_before_the_guard_is_bypassed() {
    if probe_leaks_into_this_run() {
        eprintln!("skipped: OPENRIDR_ENABLE_PROBE=1 opens the controlled-installer seam");
        return;
    }
    for host in ["127.0.0.1", "10.0.0.1", "localhost", "api.internal.local"] {
        let error = require_public_target(host, 443).unwrap_err();
        assert!(error.contains(host) && error.contains("refused"), "{error}");
    }
    assert!(
        require_public_target("1.1.1.1", 443).is_ok(),
        "a public literal needs no DNS lookup"
    );
}
