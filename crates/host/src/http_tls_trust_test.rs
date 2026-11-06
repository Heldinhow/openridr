use std::fs;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct TlsServer(Child);

impl Drop for TlsServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn https_client_uses_explicit_system_ca_and_still_rejects_unknown_ca() {
    let temp = tempfile::tempdir().unwrap();
    let ca = temp.path().join("test-ca.pem");
    let ca_key = temp.path().join("test-ca.key");
    let key = temp.path().join("server.key");
    let cert = temp.path().join("server.pem");
    let extensions = temp.path().join("server.ext");
    fs::write(
        &extensions,
        "subjectAltName=DNS:localhost\nextendedKeyUsage=serverAuth\n",
    )
    .unwrap();

    let generated = Command::new("openssl")
        .args(["req", "-x509", "-newkey", "rsa:2048", "-nodes", "-keyout"])
        .arg(&ca_key)
        .args(["-out"])
        .arg(&ca)
        .args(["-days", "1", "-subj", "/CN=OpenRidr Test CA"])
        .output()
        .expect("openssl must be installed for the local TLS trust test");
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );

    let csr = temp.path().join("server.csr");
    let generated = Command::new("openssl")
        .args(["req", "-new", "-newkey", "rsa:2048", "-nodes", "-keyout"])
        .arg(&key)
        .args(["-out"])
        .arg(&csr)
        .args(["-subj", "/CN=localhost"])
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let generated = Command::new("openssl")
        .args(["x509", "-req", "-in"])
        .arg(&csr)
        .args(["-CA"])
        .arg(&ca)
        .args(["-CAkey"])
        .arg(&ca_key)
        .args(["-CAcreateserial", "-out"])
        .arg(&cert)
        .args(["-days", "1", "-extfile"])
        .arg(&extensions)
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );

    let probe = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    let server = Command::new("openssl")
        .args(["s_server", "-quiet", "-www", "-accept"])
        .arg(port.to_string())
        .args(["-cert"])
        .arg(&cert)
        .args(["-key"])
        .arg(&key)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let _server = TlsServer(server);
    let url = format!("https://localhost:{port}/");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "local OpenSSL TLS server did not start"
        );
        thread::sleep(Duration::from_millis(20));
    }

    let rejected = super::agent_builder_with_certs(vec![])
        .build()
        .get(&url)
        .call();
    assert!(
        format!("{rejected:?}").contains("UnknownIssuer"),
        "unexpected result: {rejected:?}"
    );

    let ca_der = Command::new("openssl")
        .args(["x509", "-outform", "DER", "-in"])
        .arg(&ca)
        .output()
        .unwrap();
    assert!(ca_der.status.success());
    let accepted = super::agent_builder_with_certs(vec![ca_der.stdout.into()])
        .build()
        .get(&url)
        .call();
    assert!(
        accepted.is_ok(),
        "the configured system CA should be trusted: {accepted:?}"
    );
}
