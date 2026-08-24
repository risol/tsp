//! TCP listener + request dispatcher for TSP v2 PoC 1 slices 2..10b.
//!
//! See `tsp-v2-plan.md` sect.61 Phase 1 + sect.20-21. Responsibilities:
//!
//! 2. Bind to `0.0.0.0:<port>` (default 3000; override via `TSP_PORT`).
//! 3. Accept each connection on its own thread.
//! 4. Read a single request into a fixed-size buffer.
//! 5. `RouteTable::lookup` resolves the (path, method) to a route.
//! 6. Slice 5: re-prepare the matched route; real method set drives
//!    200 vs 405.
//! 7. Slice 6: spawn `bun.exe` via `jsc_bridge::execute` and return
//!    the rendered body.
//! 8. Slice 10b: thread the request through `PageRegistry` so a
//!    page that already built serves from `current.payload`
//!    without re-running `prepare + execute` on every request.
//!
//! Out of slice 10b (deferred to slice 10c):
//! - In-flight dedup: concurrent requests on a Building slot share
//!   the build future; for now the second request sees
//!   `BeginBuildError::NotBuildable(Building)` and falls back to
//!   LKG (or 503 if no LKG).
//! - Request pinning: a request that started on generation N
//!   finishes on N even if N+1 publishes mid-flight.
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::generation::{self, BeginBuildError, PageRef, PageRegistry, PageState};
use crate::jsc_bridge::BunRuntime;
use crate::pipeline;
use crate::router::{HttpMethod, MatchResult, RouteTable};

const DEFAULT_PORT: u16 = 3000;

#[derive(Debug)]
pub enum HostError {
    Bind(io::Error),
    Accept(io::Error),
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

static STOP: AtomicBool = AtomicBool::new(false);

/// Bind, accept, dispatch. `routes` and `registry` are
/// `&'static` because the slice-6 binary Box::leaks them; the
/// in-process bridge (slice 13) will swap the leak for
/// `Arc`-style ownership.
pub fn serve(
    host: &str,
    port: u16,
    routes: &'static RouteTable,
    registry: &'static PageRegistry,
    bun: &'static BunRuntime,
) -> Result<(), HostError> {
    let addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&addr).map_err(HostError::Bind)?;
    eprintln!(
        "TSPv2PoC1: listening on http://{addr} (slice 10b, {} route(s) loaded)",
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
            if let Err(e) = handle_connection(stream, routes, registry, bun) {
                eprintln!("TSPv2PoC1: {e}");
            }
        });
    }
    Ok(())
}

fn handle_connection(
    mut stream: TcpStream,
    routes: &RouteTable,
    registry: &PageRegistry,
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
            "TSP v2 PoC 1 slice 10b: malformed request line\n".to_string(),
        ),
        ParsedRequest::Known { method, path } => match routes.lookup(&path, method) {
            MatchResult::Found { route, method: req_method } => {
                let page_ref = PageRef {
                    route: route.path.clone(),
                    method: req_method,
                };
                render_for_route(route, req_method, &page_ref, registry, bun)
            }
            MatchResult::MethodNotAllowed { route, requested } => {
                // Slice-5 path: 405 with real Allow header from
                // the static method detector (no registry needed).
                let prepared = crate::page::prepare(route);
                let (allow, body) = render_405_body(route, requested, prepared);
                (
                    "HTTP/1.1 405 Method Not Allowed",
                    "text/plain; charset=utf-8",
                    Some(allow),
                    body,
                )
            }
            MatchResult::NotFound => (
                "HTTP/1.1 404 Not Found",
                "text/plain; charset=utf-8",
                None,
                format!(
                    "TSP v2 PoC 1 slice 10b: no route matches path={path} (table has {} route(s))\n",
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

/// Slice 10b dispatch. The page may be Unloaded, Dirty, Clean,
/// Building, or Failed. We try to read from the registry; on
/// Unloaded/Dirty we run a synchronous build; on Building we
/// fall back to LKG; on Failed we fall back to LKG too.
/// Slice 10c will add real in-flight dedup.
fn render_for_route(
    route: &crate::router::Route,
    requested: HttpMethod,
    page_ref: &PageRef,
    registry: &PageRegistry,
    bun: &BunRuntime,
) -> (&'static str, &'static str, Option<String>, String) {
    // The route exists (router matched) but the boot-time
    // method detector may not have registered this method
    // because the .tsp file does not export it. That is
    // semantically a 405, not a 500, so convert here before
    // the state machine sees the request.
    let snap = registry.snapshot(page_ref);
    if snap.is_none() {
        // Re-prepare to get the real method set for the
        // `Allow:` header (cheap for slice 10b; slice 10c
        // will cache the allow list per route).
        let allow = match crate::page::prepare(route) {
            Ok(p) => build_allow_header(&p.methods),
            Err(_) => String::new(),
        };
        let body = format!(
            "TSP v2 PoC 1 slice 10b: method {} not exported by {}\n",
            requested.as_str(),
            route.source.display()
        );
        return (
            "HTTP/1.1 405 Method Not Allowed",
            "text/plain; charset=utf-8",
            if allow.is_empty() { None } else { Some(allow) },
            body,
        );
    }
    let state = snap.as_ref().map(|s| s.state.clone()).unwrap_or(PageState::Unloaded);

    match state {
        PageState::Unloaded | PageState::Dirty => {
            // Build synchronously. The build pipeline (slice 6
            // bridge + slice 5 prepare) is what fills the
            // payload; slice 11+ will populate the dependency
            // list properly.
            match registry.begin_build(page_ref) {
                Ok(guard) => {
                    match pipeline::build(route, requested, bun) {
                        Ok(body) => {
                            let deps: Vec<crate::module_graph::ModuleId> = Vec::new();
                            guard.commit(deps, body.clone());
                            ("HTTP/1.1 200 OK", "text/html; charset=utf-8", None, body)
                        }
                        Err(e) => {
                            let msg = format!("{e}");
                            guard.fail(msg.clone());
                            eprintln!("TSPv2PoC1: build error on {}: {e}", route.source.display());
                            (
                                "HTTP/1.1 500 Internal Server Error",
                                "text/plain; charset=utf-8",
                                None,
                                format!("TSP v2 PoC 1 slice 10b: build error on {}\n  {e}\n", route.source.display()),
                            )
                        }
                    }
                }
                Err(BeginBuildError::NotBuildable(other_state)) => {
                    // The slot transitioned out of Unloaded /
                    // Dirty between snapshot and begin_build
                    // (another thread won the race, or the
                    // slot was never Unloaded/Dirty to begin
                    // with). Whatever the cause, the
                    // authoritative answer is whatever the
                    // slot's `current` is now. Fall back to
                    // current.
                    eprintln!(
                        "TSPv2PoC1: build race -- snapshot said Unloaded/Dirty, begin_build returned NotBuildable({other_state:?})"
                    );
                    serve_current_or_500(registry, page_ref)
                }
                Err(BeginBuildError::UnknownPage) => {
                    // Page was not registered. Should be
                    // caught at boot; treat as 500.
                    (
                        "HTTP/1.1 500 Internal Server Error",
                        "text/plain; charset=utf-8",
                        None,
                        "TSP v2 PoC 1 slice 10b: page not registered in PageRegistry\n".to_string(),
                    )
                }
            }
        }
        PageState::Clean => serve_current_or_500(registry, page_ref),
        PageState::Building => serve_lkg_or_503(registry, page_ref),
        PageState::Failed => serve_lkg_or_500(registry, page_ref, route),
    }
}

fn serve_current_or_500(
    registry: &PageRegistry,
    page_ref: &PageRef,
) -> (&'static str, &'static str, Option<String>, String) {
    match registry.read_current_payload(page_ref) {
        Some(body) => ("HTTP/1.1 200 OK", "text/html; charset=utf-8", None, body),
        None => (
            "HTTP/1.1 500 Internal Server Error",
            "text/plain; charset=utf-8",
            None,
            "TSP v2 PoC 1 slice 10b: Clean slot has no payload\n".to_string(),
        ),
    }
}

fn serve_lkg_or_503(
    registry: &PageRegistry,
    page_ref: &PageRef,
) -> (&'static str, &'static str, Option<String>, String) {
    match registry.read_lkg_payload(page_ref) {
        Some(body) => ("HTTP/1.1 200 OK", "text/html; charset=utf-8", None, body),
        None => (
            "HTTP/1.1 503 Service Unavailable",
            "text/plain; charset=utf-8",
            None,
            "TSP v2 PoC 1 slice 10b: concurrent build in flight, no LKG\n".to_string(),
        ),
    }
}

fn serve_lkg_or_500(
    registry: &PageRegistry,
    page_ref: &PageRef,
    route: &crate::router::Route,
) -> (&'static str, &'static str, Option<String>, String) {
    match registry.read_lkg_payload(page_ref) {
        Some(body) => ("HTTP/1.1 200 OK", "text/html; charset=utf-8", None, body),
        None => (
            "HTTP/1.1 500 Internal Server Error",
            "text/plain; charset=utf-8",
            None,
            format!(
                "TSP v2 PoC 1 slice 10b: page {} never built successfully\n",
                route.source.display()
            ),
        ),
    }
}

fn render_405_body(
    route: &crate::router::Route,
    requested: HttpMethod,
    prepared: Result<crate::page::PageSource, crate::page::PrepareError>,
) -> (String, String) {
    match prepared {
        Ok(page) => {
            let allow = build_allow_header(&page.methods);
            let body = format!(
                "TSP v2 PoC 1 slice 10b: method {} not exported by {}\n",
                requested.as_str(),
                route.source.display()
            );
            (allow, body)
        }
        Err(e) => (
            String::new(),
            format!("TSP v2 PoC 1 slice 10b: prepare error: {e}\n"),
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

// `_generation` re-exported to keep the import list honest for
// slice 10c's planned additions.
#[allow(dead_code)]
fn _generation_anchor(_g: &generation::GenerationId) {}