//! Shared HTTP diagnostics for the host's direct `ureq` requests.

#[cfg(test)]
#[path = "http_tls_trust_test.rs"]
mod tls_tests;

pub(crate) fn agent_builder() -> ureq::AgentBuilder {
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

pub(crate) fn describe_error(url: &str, error: &ureq::Error) -> String {
    describe_details(url, &format!("{} {error:?}", error))
}

fn describe_details(url: &str, detail: &str) -> String {
    let diagnostic = detail.to_ascii_lowercase();
    if diagnostic.contains("unknownissuer")
        || diagnostic.contains("unknown issuer")
        || diagnostic.contains("certificate")
    {
        format!(
            "The network certificate for {url} is not trusted, possibly because a corporate proxy or TLS inspection is in use. Ask your network administrator to trust its CA in the system Keychain, then retry. Details: {detail}"
        )
    } else {
        format!("{url}: {detail}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certificate_errors_explain_network_trust_and_keep_the_url() {
        let message = describe_details(
            "https://api.nuget.org/v3/index.json",
            "invalid peer certificate: UnknownIssuer",
        );
        assert!(message.contains("https://api.nuget.org/v3/index.json"));
        assert!(message.contains("network certificate"));
        assert!(message.contains("system Keychain"));
    }
}
