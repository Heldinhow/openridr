use std::sync::mpsc;
use std::sync::Arc;

use openridr_host::{Host, HostError};
use serde_json::{json, Value};
use tiny_http::{Header, Method, Request, Response, Server};

const ALLOWED_ORIGINS: [&str; 2] = ["http://localhost:1420", "http://127.0.0.1:1420"];

/// A request waiting for its turn, with the body already read: the accept loop needs the method to
/// decide whether the request may overtake the queue.
type Job = (Request, Option<String>);

fn main() {
    let port = parse_port();
    let address = format!("127.0.0.1:{port}");
    let server = Server::http(&address).unwrap_or_else(|error| {
        eprintln!("openridr-host-http: cannot bind {address}: {error}");
        std::process::exit(1);
    });
    let host = Arc::new(Host::new());
    // One worker keeps the send order the app shell guarantees: the queue is FIFO, so the order the
    // UI sent is the order the host sees. A cancel never enters it — it is answered on the accept
    // loop, because it has to overtake the operation it stops. Which methods may overtake is the
    // host's list, the same one the Tauri shell uses.
    let (queue, work) = mpsc::channel::<Job>();
    let worker = Arc::clone(&host);
    std::thread::spawn(move || {
        for (request, body) in work {
            handle(request, &worker, body);
        }
    });
    println!("openridr-host-http listening on http://{address}");
    println!("dev bridge only: no authentication, never ship this binary");

    while let Ok(mut request) = server.recv() {
        let is_call = *request.method() == Method::Post && request.url() == "/call";
        if !is_call {
            let _ = queue.send((request, None));
            continue;
        }
        let mut body = String::new();
        if let Err(error) = request.as_reader().read_to_string(&mut body) {
            let origin = origin_of(&request);
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
            continue;
        }
        if overtakes_the_queue(&body) {
            handle(request, &host, Some(body));
        } else {
            let _ = queue.send((request, Some(body)));
        }
    }
}

/// Whether this call may skip the queue, from the host's own list.
fn overtakes_the_queue(body: &str) -> bool {
    serde_json::from_str::<Value>(body)
        .ok()
        .and_then(|payload| {
            payload
                .get("method")
                .and_then(Value::as_str)
                .map(openridr_host::bypasses_host_lock)
        })
        .unwrap_or(false)
}

fn origin_of(request: &Request) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|header| header.field.equiv("Origin"))
        .map(|header| header.value.as_str().to_string())
}

fn handle(request: Request, host: &Host, body: Option<String>) {
    let origin = origin_of(&request);
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
        let envelope = match serde_json::from_str::<Value>(&body.unwrap_or_default()) {
            Ok(payload) => dispatch(host, &payload),
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

fn dispatch(host: &Host, payload: &Value) -> String {
    let Some(method) = payload.get("method").and_then(Value::as_str) else {
        return error_envelope(HostError::new(
            "invalid_state",
            "Request must contain a string `method`.",
        ));
    };
    let params = payload.get("params").cloned().unwrap_or(Value::Null);
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
