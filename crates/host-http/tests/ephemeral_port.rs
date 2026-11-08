//! `--port 0` asks the kernel for a free port. The bridge has to print the port it actually got,
//! because the E2E helper reads it from stdout instead of choosing one (issue #246).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};

const LISTENING: &str = "openridr-host-http listening on http://127.0.0.1:";

fn start(port: &str) -> (Child, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_openridr-host-http"))
        .args(["--port", port])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn openridr-host-http");
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("stdout"))
        .read_line(&mut line)
        .expect("read the listening line");
    (child, line.trim().to_string())
}

fn health(port: u16) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to the bridge");
    write!(
        stream,
        "GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )
    .expect("write the request");
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("read the response");
    response
}

#[test]
fn port_zero_prints_the_port_the_kernel_granted() {
    let (mut child, line) = start("0");
    let printed = line
        .strip_prefix(LISTENING)
        .unwrap_or_else(|| panic!("unexpected listening line: {line}"));
    let port: u16 = printed
        .parse()
        .unwrap_or_else(|_| panic!("not a port: {printed}"));
    // Before the fix the line repeats the requested port, and 0 is not a port anyone can reach.
    assert_ne!(port, 0, "the bridge printed the requested port: {line}");
    let response = health(port);
    assert!(
        response.contains("\"service\":\"openridr-host-http\""),
        "{response}"
    );
    // The pid proves the port belongs to this child, which is what the E2E helper checks.
    assert!(
        response.contains(&format!("\"pid\":{}", child.id())),
        "expected pid {} in {response}",
        child.id()
    );
    child.kill().expect("kill the bridge");
    child.wait().expect("reap the bridge");
}

#[test]
fn a_fixed_port_still_prints_that_port() {
    // Port 0 is the new path; the fixed one keeps working for `bun run host:http`.
    let free = std::net::TcpListener::bind("127.0.0.1:0").expect("pick a free port");
    let port = free.local_addr().expect("local addr").port();
    drop(free);
    let (mut child, line) = start(&port.to_string());
    assert_eq!(line, format!("{LISTENING}{port}"));
    child.kill().expect("kill the bridge");
    child.wait().expect("reap the bridge");
}
