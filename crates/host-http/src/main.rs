use std::sync::{Arc, Mutex};

use openridr_host::{Host, HostError};
use serde_json::{json, Value};
use tiny_http::{Header, Method, Request, Response, Server};

const ALLOWED_ORIGINS: [&str; 2] = ["http://localhost:1420", "http://127.0.0.1:1420"];

fn main() {
    let port = parse_port();
    let address = format!("127.0.0.1:{port}");
    let server = Server::http(&address).unwrap_or_else(|error| {
        eprintln!("openridr-host-http: cannot bind {address}: {error}");
        std::process::exit(1);
    });
    let host = Arc::new(Host::new());
    // Mirrors the shell's host-call lock: one dispatch at a time, so the order the UI sends is the
    // order the host sees. A request per thread is what lets `git.cancel` be answered while a push
    // is still running — with the old one-at-a-time loop it queued behind the very call it stops.
    let gate = Arc::new(Mutex::new(()));
    println!("openridr-host-http listening on http://{address}");
    println!("dev bridge only: no authentication, never ship this binary");

    while let Ok(request) = server.recv() {
        let host = Arc::clone(&host);
        let gate = Arc::clone(&gate);
        std::thread::spawn(move || handle(request, &host, &gate));
    }
}

fn handle(mut request: Request, host: &Host, gate: &Mutex<()>) {
    let origin = request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Origin"))
        .map(|header| header.value.as_str().to_string());
    let method = request.method().clone();
    let url = request.url().to_string();
    eprintln!("{method} {url}");

    if method == Method::Options {
        respond(request.respond(with_cors(
            Response::from_string("").with_status_code(204),
            origin.as_deref(),
        )));
        return;
    }

    if method == Method::Get && url == "/health" {
        let body = json!({
            "ok": true,
            "service": "openridr-host-http",
            "version": env!("CARGO_PKG_VERSION"),
            "pid": std::process::id(),
        });
        respond(
            request.respond(with_cors(
                Response::from_string(body.to_string())
                    .with_status_code(200)
                    .with_header(json_header()),
                origin.as_deref(),
            )),
        );
        return;
    }

    if method == Method::Post && url == "/call" {
        let mut body = String::new();
        if let Err(error) = request.as_reader().read_to_string(&mut body) {
            respond(
                request.respond(with_cors(
                    Response::from_string(error_envelope(HostError::new(
                        "invalid_state",
                        format!("Could not read request body: {error}"),
                    )))
                    .with_header(json_header()),
                    origin.as_deref(),
                )),
            );
            return;
        }
        let envelope = match serde_json::from_str::<Value>(&body) {
            Ok(payload) => dispatch(host, &payload, gate),
            Err(error) => error_envelope(HostError::new(
                "invalid_state",
                format!("Request body is not JSON: {error}"),
            )),
        };
        respond(request.respond(with_cors(
            Response::from_string(envelope).with_header(json_header()),
            origin.as_deref(),
        )));
        return;
    }

    respond(
        request.respond(with_cors(
            Response::from_string(
                json!({ "ok": false, "error": { "code": "not_found", "message": "Not found" } })
                    .to_string(),
            )
            .with_status_code(404)
            .with_header(json_header()),
            origin.as_deref(),
        )),
    );
}

fn parse_port() -> u16 {
    let mut args = std::env::args().skip(1);
    let mut port = 4317;
    while let Some(arg) = args.next() {
        if arg == "--port" {
            if let Some(value) = args.next() {
                port = value.parse().unwrap_or(port);
            }
        }
    }
    port
}

fn dispatch(host: &Host, payload: &Value, gate: &Mutex<()>) -> String {
    let Some(method) = payload.get("method").and_then(Value::as_str) else {
        return error_envelope(HostError::new(
            "invalid_state",
            "Request must contain a string `method`.",
        ));
    };
    let params = payload.get("params").cloned().unwrap_or(Value::Null);
    // The same exception the shell applies, from the same list: a cancel must not queue behind the
    // operation it stops. A poisoned gate keeps serving instead of taking the bridge down.
    let _guard = if openridr_host::bypasses_host_lock(method) {
        None
    } else {
        Some(
            gate.lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    };
    match host.dispatch(method, params) {
        Ok(data) => json!({ "ok": true, "data": data }).to_string(),
        Err(error) => error_envelope(error),
    }
}

fn error_envelope(error: HostError) -> String {
    json!({ "ok": false, "error": error }).to_string()
}

fn json_header() -> Header {
    Header::from_bytes("Content-Type", "application/json").unwrap()
}

fn with_cors(
    mut response: Response<std::io::Cursor<Vec<u8>>>,
    origin: Option<&str>,
) -> Response<std::io::Cursor<Vec<u8>>> {
    if let Some(origin) = origin.filter(|origin| ALLOWED_ORIGINS.contains(origin)) {
        for (name, value) in [
            ("Access-Control-Allow-Origin", origin),
            ("Vary", "Origin"),
            ("Access-Control-Allow-Methods", "GET, POST, OPTIONS"),
            ("Access-Control-Allow-Headers", "content-type"),
        ] {
            response.add_header(Header::from_bytes(name.as_bytes(), value.as_bytes()).unwrap());
        }
    }
    response
}

fn respond(result: Result<(), std::io::Error>) {
    if let Err(error) = result {
        eprintln!("openridr-host-http: failed to respond: {error}");
    }
}
