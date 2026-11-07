//! The `scutil --proxy` dictionary, interpreted without touching the machine's network settings.
//!
//! The samples are the shapes `scutil` prints: the first one is the real output of a Mac with no
//! proxy configured, the others follow the SystemConfiguration key names.

use super::*;

/// The real `scutil --proxy` of a Mac with no proxy configured.
const NO_PROXY: &str = "<dictionary> {
  ExceptionsList : <array> {
    0 : *.local
    1 : 169.254/16
  }
  FTPPassive : 1
}
";

const BOTH_PROXIES: &str = "<dictionary> {
  ExceptionsList : <array> {
    0 : *.local
    1 : 169.254/16
    2 : .corp.example
  }
  ExcludeSimpleHostnames : 1
  HTTPEnable : 1
  HTTPPort : 8080
  HTTPProxy : proxy.corp.example
  HTTPSEnable : 1
  HTTPSPort : 8443
  HTTPSProxy : secure.corp.example
  FTPPassive : 1
}
";

#[test]
fn a_mac_without_a_proxy_yields_no_route() {
    let parsed = parse(NO_PROXY);
    assert_eq!(parsed.https, None);
    assert_eq!(parsed.http, None);
    assert_eq!(parsed.unsupported, None);
    assert_eq!(parsed.exceptions, ["*.local", "169.254/16"]);
}

#[test]
fn the_https_and_http_proxies_are_read_with_their_ports() {
    let parsed = parse(BOTH_PROXIES);
    assert_eq!(parsed.https.as_deref(), Some("secure.corp.example:8443"));
    assert_eq!(parsed.http.as_deref(), Some("proxy.corp.example:8080"));
    assert_eq!(parsed.unsupported, None);
    assert_eq!(
        parsed.exceptions,
        ["*.local", "169.254/16", ".corp.example"],
        "the exceptions list is read in order"
    );
}

/// Only "Web Proxy (HTTP)" filled in is the usual corporate setup: CFNetwork uses it for an https
/// URL too, so the host must not fall back to a direct connection.
#[test]
fn an_http_only_setting_also_covers_https() {
    let parsed = parse(
        "<dictionary> {
  HTTPEnable : 1
  HTTPProxy : proxy.corp.example
}
",
    );
    assert_eq!(parsed.http.as_deref(), Some("proxy.corp.example:80"));
    assert_eq!(parsed.https.as_deref(), Some("proxy.corp.example:80"));
}

#[test]
fn a_missing_port_uses_the_scheme_default() {
    let parsed = parse(
        "<dictionary> {
  HTTPSEnable : 1
  HTTPSProxy : secure.corp.example
}
",
    );
    assert_eq!(parsed.https.as_deref(), Some("secure.corp.example:443"));
}

#[test]
fn a_disabled_proxy_is_not_a_route() {
    let parsed = parse(
        "<dictionary> {
  HTTPEnable : 0
  HTTPPort : 8080
  HTTPProxy : proxy.corp.example
}
",
    );
    assert_eq!(parsed.http, None);
    assert_eq!(parsed.https, None);
}

#[test]
fn socks_alone_is_named_but_not_used() {
    let parsed = parse(
        "<dictionary> {
  SOCKSEnable : 1
  SOCKSPort : 1080
  SOCKSProxy : 127.0.0.1
}
",
    );
    assert_eq!(parsed.http, None);
    assert_eq!(parsed.https, None);
    assert_eq!(parsed.unsupported, Some("a SOCKS proxy"));
}

/// A PAC file decides per URL, so the manual proxy beside it is not the setting the Mac is using.
#[test]
fn a_pac_file_wins_over_the_manual_proxy_and_is_named() {
    let parsed = parse(
        "<dictionary> {
  HTTPEnable : 1
  HTTPPort : 8080
  HTTPProxy : stale.corp.example
  ProxyAutoConfigEnable : 1
  ProxyAutoConfigURLString : http://wpad.corp.example/proxy.pac
}
",
    );
    assert_eq!(parsed.http, None);
    assert_eq!(parsed.https, None);
    assert_eq!(
        parsed.unsupported,
        Some("automatic proxy configuration (PAC/WPAD)")
    );
}

#[test]
fn a_usable_proxy_is_not_reported_as_unsupported() {
    let parsed = parse(
        "<dictionary> {
  HTTPSEnable : 1
  HTTPSProxy : secure.corp.example
  SOCKSEnable : 1
  SOCKSPort : 1080
  SOCKSProxy : 127.0.0.1
}
",
    );
    assert_eq!(parsed.https.as_deref(), Some("secure.corp.example:443"));
    assert_eq!(parsed.unsupported, None);
}

#[test]
fn an_empty_dictionary_yields_nothing() {
    let parsed = parse("<dictionary> {\n  FTPPassive : 1\n}\n");
    assert_eq!(parsed, SystemProxy::default());
}

#[test]
fn garbage_is_read_as_no_setting() {
    let parsed = parse("scutil: no such key\n\n");
    assert_eq!(parsed, SystemProxy::default());
}

#[test]
fn the_seam_turns_the_source_off_in_every_build() {
    assert!(matches!(source_for(Some("off"), true), Source::Disabled));
    assert!(matches!(source_for(Some("off"), false), Source::Disabled));
    assert!(matches!(source_for(Some(" off "), true), Source::Disabled));
}

/// The program form is a development seam: a release build ignores it and reads the real setting,
/// so a variable cannot point the host at another program in a shipped app.
#[test]
fn only_a_development_build_may_replace_the_reader() {
    assert!(matches!(
        source_for(Some("/tmp/fake-scutil"), true),
        Source::Program(program) if program == Path::new("/tmp/fake-scutil")
    ));
    assert!(matches!(
        source_for(Some("/tmp/fake-scutil"), false),
        Source::Scutil
    ));
    assert!(matches!(source_for(Some(""), true), Source::Scutil));
    assert!(matches!(source_for(None, true), Source::Scutil));
}

#[cfg(unix)]
#[test]
fn a_program_that_prints_the_dictionary_is_read_like_the_real_one() {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let program = directory.path().join("fake-scutil");
    let mut file = std::fs::File::create(&program).unwrap();
    writeln!(file, "#!/bin/sh").unwrap();
    for line in BOTH_PROXIES.lines() {
        writeln!(file, "echo '{line}'").unwrap();
    }
    drop(file);
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();

    let text = run(&program).expect("the program prints the dictionary");
    let parsed = parse(&text);
    assert_eq!(parsed.https.as_deref(), Some("secure.corp.example:8443"));
    assert_eq!(parsed.exceptions.len(), 3);
}

#[cfg(unix)]
#[test]
fn a_program_that_fails_is_read_as_no_setting() {
    let directory = tempfile::tempdir().unwrap();
    let program = directory.path().join("broken-scutil");
    std::fs::write(&program, "#!/bin/sh\nexit 3\n").unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(run(&program), None);
    assert_eq!(run(&directory.path().join("missing")), None);
}

/// The real command on this machine: the point is that its output is understood, not that a proxy
/// is configured (a machine without one is the normal case for a test runner).
#[cfg(target_os = "macos")]
#[test]
fn the_real_scutil_output_is_understood() {
    let Some(text) = run(Path::new(SCUTIL)) else {
        eprintln!("not verified: scutil did not answer on this machine");
        return;
    };
    let parsed = parse(&text);
    for proxy in [parsed.https.as_deref(), parsed.http.as_deref()]
        .into_iter()
        .flatten()
    {
        assert!(proxy.contains(':'), "{proxy}");
        assert!(!proxy.contains(char::is_whitespace), "{proxy}");
    }
}

/// An IPv6 setting has to reach the route decision as one address: unbracketed it would fail as a
/// malformed `host:port` and the message would not say "IPv6".
#[test]
fn an_ipv6_setting_is_bracketed() {
    let parsed = parse(
        "<dictionary> {
  HTTPSEnable : 1
  HTTPSPort : 8080
  HTTPSProxy : ::1
}
",
    );
    assert_eq!(parsed.https.as_deref(), Some("[::1]:8080"));
}
