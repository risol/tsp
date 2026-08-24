//! TCP listener + minimal HTTP/1.1 responder for TSP v2 PoC 1 slices 2 + 3.
//!
//! See `tsp-v2-plan.md` sect.61 Phase 1. Responsibilities in slice 3:
//!
//! 1. Bind to `0.0.0.0:<port>` (default 3000; override via `TSP_PORT`).
//! 2. Accept each connection on its own thread.
//! 3. Read a single request into a fixed-size buffer.
//! 4. Look up `(path, method)` in the [`RouteTable`] the caller hands in.
//!    - `Found`               -> 200 with the slice-3 marker body
//!    - `MethodNotAllowed`    -> 405 with an `Allow:` header
//!    - `NotFound`            -> 404 with the slice-2 marker body
//! 5. Close the connection.
//!
//! No router code lives here -- the table is built once at boot by
//! [`crate::router::RouteTable::scan`] and treated as read-only for the
//! rest of the process lifetime. Reload, dynamic-segment, and
//! catch-all support land in later slices alongside the
//! Context/Request/Response bridge.
//!
//! Production HTTP lives behind `bun_uws` (plan sect.25.3) and arrives
//! when the HTTP path needs async / multi-worker / uWS-grade
//! throughput. Keeping slice 2-3 stdlib-only means the bootstrap stays
//! auditable line-by-line.
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::router::{HttpMethod, MatchResult, RouteTable};

/// Default TCP port for the PoC listener.
const DEFAULT_PORT: u16 = 3000;

/// Hand-rolled error type. We deliberately avoid pulling `thiserror` or
/// any other error crate for slice 2 -- one variant per failure mode and
/// `Display` is enough.
#[derive(Debug)]
pub enum HostError {
    /// `TcpListener::bind` failed (port in use, permission denied, etc.).
    Bind(io::Error),
    /// A per-connection `accept` returned an error other than a closed
    /// listener (interrupted system call, EMFILE, etc.).
    Accept(io::Error),
    /// A connection handler failed. Logged with `eprintln!` and the loop
    /// continues -- a single bad client must not take the server down.
    Connection(io::Error),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bind(e) => write!(f, "bind failed: {e}"),
            Self::Accept(e) => write!(f, "accept failed: {e}"),
            Self::Connection(e) => write!(f, "connection handler failed: {e}"),
        }
    }
}

impl std::error::Error for HostError {}

/// Global "stop the accept loop" flag, flipped to `true` by
/// [`serve_until_stopped`] when it observes a non-recoverable accept
/// error or when the process receives SIGINT (the binary's signal
/// handler is the only writer besides the accept loop itself).
static STOP: AtomicBool = AtomicBool::new(false);

/// Bind to `host:port`, accept connections forever, hand each one to a
/// fresh thread running [`handle_connection`]. Returns
/// [`HostError::Bind`] if the listener cannot be created; any error on
/// the accept loop is forwarded as [`HostError::Accept`].
///
/// `routes` is borrowed for the entire lifetime of the server. Slice 3
/// does not mutate it; later slices may swap the backing store for a
/// generation-aware radix tree but the `&'static` style here stays.
pub fn serve(host: &str, port: u16, routes: &'static RouteTable) -> Result<(), HostError> {
    let addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&addr).map_err(HostError::Bind)?;
    eprintln!(
        "TSPv2PoC1: listening on http://{addr} (slice 3, {} route(s) loaded)",
        routes.len()
    );

    while !STOP.load(Ordering::Acquire) {
        let (stream, peer) = match listener.accept() {
            Ok(pair) => pair,
            Err(e) => {
                if e.kind() == io::ErrorKind::Interrupted
                    || e.kind() == io::ErrorKind::WouldBlock
                {
                    continue;
                }
                return Err(HostError::Accept(e));
            }
        };
        eprintln!("TSPv2PoC1: accepted {peer}");
        thread::spawn(move || {
            if let Err(e) = handle_connection(stream, routes) {
                eprintln!("TSPv2PoC1: {e}");
            }
        });
    }
    Ok(())
}

/// Read one request, classify it against `routes`, write the response,
/// close the connection. Status + body are the slice-3 markers so a
/// `curl` immediately shows whether the table loaded and which branch
/// fired.
fn handle_connection(mut stream: TcpStream, routes: &RouteTable) -> Result<(), HostError> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf).map_err(HostError::Connection)?;
    let request = std::str::from_utf8(&buf[..n]).unwrap_or("");
    let parsed = parse_request(request);
    let (status_line, allow_header, body) = match parsed {
        ParsedRequest::Unknown => (
            "HTTP/1.1 400 Bad Request",
            None,
            "TSP v2 PoC 1 slice 3: malformed request line\n".to_string(),
        ),
        ParsedRequest::Known { method, path } => match routes.lookup(&path, method) {
            MatchResult::Found { route, method } => (
                "HTTP/1.1 200 OK",
                None,
                format!(
                    "TSP v2 PoC 1 slice 3: route matched (path={}, method={}, source={}); JSC not wired yet\n",
                    path,
                    method.as_str(),
                    route.source.display()
                ),
            ),
            MatchResult::MethodNotAllowed { route, requested } => {
                let allow = build_allow_header(&route.methods);
                (
                    "HTTP/1.1 405 Method Not Allowed",
                    Some(allow),
                    format!(
                        "TSP v2 PoC 1 slice 3: method {} not exported by {}\n",
                        requested.as_str(),
                        route.source.display()
                    ),
                )
            }
            MatchResult::NotFound => (
                "HTTP/1.1 404 Not Found",
                None,
                format!(
                    "TSP v2 PoC 1 slice 3: no route matches path={} (table has {} route(s)); JSC not wired yet\n",
                    path,
                    routes.len()
                ),
            ),
        },
    };

    let mut head = format!(
        "{status_line}\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n",
        body.len()
    );
    if let Some(allow) = allow_header {
        head.push_str(&allow);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");

    stream
        .write_all(head.as_bytes())
        .map_err(HostError::Connection)?;
    stream
        .write_all(body.as_bytes())
        .map_err(HostError::Connection)?;
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}

#[derive(Debug)]
enum ParsedRequest {
    Known { method: HttpMethod, path: String },
    Unknown,
}

/// Pull `METHOD` and path out of a request line. Path is owned so the
/// caller can hand it to the `RouteTable` without lifetime gymnastics.
/// Query strings are stripped (plan sect.6 covers the routing model;
/// query lives in `ctx.query`, which slice 7+ will populate).
fn parse_request(request: &str) -> ParsedRequest {
    let Some(first_line) = request.lines().next() else {
        return ParsedRequest::Unknown;
    };
    let mut parts = first_line.split_whitespace();
    let Some(method_str) = parts.next() else {
        return ParsedRequest::Unknown;
    };
    let Some(path) = parts.next() else {
        return ParsedRequest::Unknown;
    };
    let Some(method) = HttpMethod::from_request_line(method_str) else {
        return ParsedRequest::Unknown;
    };
    let end = path.find('?').unwrap_or(path.len());
    ParsedRequest::Known {
        method,
        path: path[..end].to_string(),
    }
}

fn build_allow_header(methods: &[HttpMethod]) -> String {
    let joined: Vec<&'static str> = methods.iter().map(|m| m.as_str()).collect();
    format!("Allow: {}", joined.join(", "))
}

/// Resolve the listen port for slice 2. `TSP_PORT` env var wins, falling
/// back to [`DEFAULT_PORT`]. Parse failure is a hard error so a typo
/// in the env var does not silently bind to the default.
pub fn resolve_port() -> Result<u16, HostError> {
    match std::env::var("TSP_PORT") {
        Err(_) => Ok(DEFAULT_PORT),
        Ok(s) => s.parse::<u16>().map_err(|_| {
            HostError::Bind(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("TSP_PORT is not a u16: {s:?}"),
            ))
        }),
    }
}