//! TCP listener + minimal HTTP/1.1 responder for TSP v2 PoC 1 slices 2..6.
//!
//! See `tsp-v2-plan.md` sect.61 Phase 1. Responsibilities across slices:
//!
//! 2. Bind to `0.0.0.0:<port>` (default 3000; override via `TSP_PORT`).
//! 3. Accept each connection on its own thread.
//! 4. Read a single request into a fixed-size buffer.
//! 5. Slice 3: look up `(path, method)` in the [`RouteTable`].
//! 6. Slice 5: re-prepare the matched route on every request; use the
//!    real method set to pick 200 vs 405.
//! 7. Slice 6: on the 200 path, call [`jsc_bridge::execute`] which
//!    spawns the vendored `bun.exe` to evaluate the page.
//! 8. Close the connection.
//!
//! Production HTTP lives behind `bun_uws` (plan sect.25.3) and arrives
//! when the HTTP path needs async / multi-worker / uWS-grade
//! throughput. Keeping slice 2-6 stdlib-only means the bootstrap stays
//! auditable line-by-line.
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::jsc_bridge::{self, BunRuntime};
use crate::page;
use crate::router::{HttpMethod, MatchResult, RouteTable};

/// Default TCP port for the PoC listener.
const DEFAULT_PORT: u16 = 3000;

/// Hand-rolled error type. We deliberately avoid pulling `thiserror` or
/// any other error crate for slice 2 -- one variant per failure mode
/// and `Display` is enough.
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

/// Global "stop the accept loop" flag.
static STOP: AtomicBool = AtomicBool::new(false);

/// Bind to `host:port`, accept connections forever, hand each one to a
/// fresh thread running [`handle_connection`]. Slice 6: `bun` is the
/// handle the host uses to evaluate matched pages; it is borrowed for
/// the entire process lifetime.
pub fn serve(
    host: &str,
    port: u16,
    routes: &'static RouteTable,
    bun: &'static BunRuntime,
) -> Result<(), HostError> {
    let addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&addr).map_err(HostError::Bind)?;
    eprintln!(
        "TSPv2PoC1: listening on http://{addr} (slice 6, {} route(s) loaded, bun={})",
        routes.len(),
        bun.bin.display()
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
            if let Err(e) = handle_connection(stream, routes, bun) {
                eprintln!("TSPv2PoC1: {e}");
            }
        });
    }
    Ok(())
}

fn handle_connection(
    mut stream: TcpStream,
    routes: &RouteTable,
    bun: &BunRuntime,
) -> Result<(), HostError> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf).map_err(HostError::Connection)?;
    let request = std::str::from_utf8(&buf[..n]).unwrap_or("");
    let parsed = parse_request(request);

    let (status_line, content_type, allow_header, body) = match parsed {
        ParsedRequest::Unknown => (
            "HTTP/1.1 400 Bad Request",
            "text/plain; charset=utf-8",
            None,
            "TSP v2 PoC 1 slice 6: malformed request line\n".to_string(),
        ),
        ParsedRequest::Known { method, path } => match routes.lookup(&path, method) {
            MatchResult::Found { route, method: req_method } => {
                let prepared = page::prepare(route);
                render_for_route(route, req_method, prepared, bun)
            }
            MatchResult::MethodNotAllowed { route, requested } => {
                let prepared = page::prepare(route);
                let (allow, body) = render_405_body(route, requested, prepared);
                ("HTTP/1.1 405 Method Not Allowed", "text/plain; charset=utf-8", Some(allow), body)
            }
            MatchResult::NotFound => (
                "HTTP/1.1 404 Not Found",
                "text/plain; charset=utf-8",
                None,
                format!(
                    "TSP v2 PoC 1 slice 6: no route matches path={path} (table has {} route(s))\n",
                    routes.len()
                ),
            ),
        },
    };

    let mut head = format!(
        "{status_line}\r\n\
         Content-Type: {content_type}\r\n\
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

fn render_for_route(
    route: &crate::router::Route,
    requested: HttpMethod,
    prepared: Result<page::PageSource, page::PrepareError>,
    bun: &BunRuntime,
) -> (&'static str, &'static str, Option<String>, String) {
    match prepared {
        Ok(page) if page.methods.contains(&requested) => {
            // Slice 6: the body is the JSC-executed page output. On
            // any JscError, fall back to a 500 that names the source
            // and the error so the operator can fix the page.
            match jsc_bridge::execute(bun, &page.text, requested) {
                Ok(rendered) => (
                    "HTTP/1.1 200 OK",
                    "text/html; charset=utf-8",
                    None,
                    rendered,
                ),
                Err(e) => {
                    eprintln!("TSPv2PoC1: jsc error on {}: {e}", route.source.display());
                    (
                        "HTTP/1.1 500 Internal Server Error",
                        "text/plain; charset=utf-8",
                        None,
                        format!(
                            "TSP v2 PoC 1 slice 6: jsc error on {}\n  {e}\n",
                            route.source.display()
                        ),
                    )
                }
            }
        }
        Ok(page) => {
            let allow = build_allow_header(&page.methods);
            let body = format!(
                "TSP v2 PoC 1 slice 6: method {} not exported by {}\n",
                requested.as_str(),
                route.source.display()
            );
            (
                "HTTP/1.1 405 Method Not Allowed",
                "text/plain; charset=utf-8",
                Some(allow),
                body,
            )
        }
        Err(e) => (
            "HTTP/1.1 500 Internal Server Error",
            "text/plain; charset=utf-8",
            None,
            format!("TSP v2 PoC 1 slice 6: prepare error: {e}\n"),
        ),
    }
}

fn render_405_body(
    route: &crate::router::Route,
    requested: HttpMethod,
    prepared: Result<page::PageSource, page::PrepareError>,
) -> (String, String) {
    match prepared {
        Ok(page) => {
            let allow = build_allow_header(&page.methods);
            let body = format!(
                "TSP v2 PoC 1 slice 6: method {} not exported by {}\n",
                requested.as_str(),
                route.source.display()
            );
            (allow, body)
        }
        Err(e) => (
            String::new(),
            format!("TSP v2 PoC 1 slice 6: prepare error: {e}\n"),
        ),
    }
}

#[derive(Debug)]
enum ParsedRequest {
    Known { method: HttpMethod, path: String },
    Unknown,
}

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