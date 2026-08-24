//! TCP listener + request dispatcher for TSP v2 PoC 1 slices 2..12.
//!
//! See `tsp-v2-plan.md` sect.61 Phase 1 + sect.20-22. Responsibilities:
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
//! 9. Slice 12: in-flight dedup (plan sect.22.4) and request
//!    pinning (plan sect.21.3) on the `Arc<String>` payload.
//!    Concurrent requests on a Building slot share one build
//!    future; a request that pinned a body serves that exact
//!    body even if a later commit overwrites `current` mid-flight.
//!
//! Out of slice 12 (deferred to slice 13+):
//! - In-process JSC bridge (plan sect.25.3): replace the
//!   `bun.exe` subprocess path with `bun_runtime`.
//! - New-route pickup without restart (the watcher rebuilds
//!   the module graph on add/remove in slice 12+).
use std::io::{self, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crate::generation::{
    self, BeginBuildError, BuildOutcome, InFlightState, PageRef, PageRegistry, PageState,
};
use crate::jsc_bridge::BunRuntime;
use crate::pipeline;
use crate::router::{HttpMethod, MatchResult, RouteTable};
use std::sync::Arc;

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
        "TSPv2PoC1: listening on http://{addr} (slice 12, {} route(s) loaded)",
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
            MatchResult::FoundHeadOverGet { route } => {
                // Spec sect.6.5: HEAD with no explicit HEAD export.
                // Run the GET handler, then strip the body. We
                // intentionally do NOT preserve Content-Length --
                // see the slice 14a note in progress.md for the
                // proper Content-Length-preserving refactor.
                let page_ref = PageRef {
                    route: route.path.clone(),
                    method: HttpMethod::Get,
                };
                let (_status, _ct, _allow, _body) = render_for_route(
                    route, HttpMethod::Get, &page_ref, registry, bun,
                );
                (
                    "HTTP/1.1 200 OK",
                    "text/html; charset=utf-8",
                    None,
                    String::new(),
                )
            }
            MatchResult::MethodNotAllowed { route, requested } => {
                // Spec sect.6.6: OPTIONS with no explicit OPTIONS
                // export -> automatic 204 with Allow. Only applies
                // when the route exports other methods (so the
                // Allow list is non-empty -- a route that exports
                // only OPTIONS still 405s).
                if requested == HttpMethod::Options
                    && route.methods.iter().any(|m| *m != HttpMethod::Options)
                {
                    let allow = build_allow_header(&route.methods);
                    (
                        "HTTP/1.1 204 No Content",
                        "text/plain; charset=utf-8",
                        Some(allow),
                        String::new(),
                    )
                } else {
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

/// Slice 12 dispatch. The page may be Unloaded, Clean, Dirty,
/// Building, or Failed. The request hot path uses the new
/// pinned-payload + in-flight dedup primitives:
///
/// - Unloaded/Dirty/Failed: we win the `begin_build` race,
///   run the pipeline, `commit` the result, pin the
///   `Arc<String>`, and serve it.
/// - Building: a sibling request is already building. We
///   join the shared in-flight future (plan sect.22.4),
///   wait on the condvar, and either serve the committed
///   payload or fall back to LKG on failure.
/// - Clean: pin the current `Arc<String>` and serve. The
///   pin (plan sect.21.3) means a later commit cannot
///   retroactively change the body this request observes
///   -- mid-flight reloads do not corrupt in-progress
///   responses.
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
        let allow = match crate::page::prepare(route) {
            Ok(p) => build_allow_header(&p.methods),
            Err(_) => String::new(),
        };
        let body = format!(
            "TSP v2 PoC 1 slice 12: method {} not exported by {}
",
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
        PageState::Unloaded | PageState::Dirty | PageState::Failed => {
            // Try to win the build race. If we lose (another
            // thread transitioned us to Building between
            // snapshot and begin_build), fall through to the
            // Building branch on the same request -- the
            // shared future means we still share one build.
            match registry.begin_build(page_ref) {
                Ok(guard) => {
                    // We own the build. Run pipeline,
                    // commit, pin the payload, serve.
                    let build_result = pipeline::build(route, requested, bun);
                    match build_result {
                        Ok(body) => {
                            let deps: Vec<crate::module_graph::ModuleId> = Vec::new();
                            // commit() consumes guard; payload
                            // becomes an Arc<String> on the
                            // new current generation.
                            guard.commit(deps, body);
                            // Re-read the pinned Arc from the
                            // current generation we just
                            // published. This is the
                            // request-pinning primitive.
                            let pinned = registry
                                .read_current_arc(page_ref)
                                .expect("post-commit current exists");
                            serve_arc(200, "OK", pinned)
                        }
                        Err(e) => {
                            let msg = format!("{e}");
                            guard.fail(msg);
                            eprintln!(
                                "TSPv2PoC1: build error on {}: {e}",
                                route.source.display()
                            );
                            (
                                "HTTP/1.1 500 Internal Server Error",
                                "text/plain; charset=utf-8",
                                None,
                                format!(
                                    "TSP v2 PoC 1 slice 12: build error on {}
  {e}
",
                                    route.source.display()
                                ),
                            )
                        }
                    }
                }
                Err(BeginBuildError::NotBuildable(PageState::Building)) => {
                    // Lost the race. Re-dispatch into the
                    // Building branch on this same request.
                    handle_building(route, page_ref, registry)
                }
                Err(BeginBuildError::NotBuildable(other)) => {
                    // Slot transitioned Clean between our
                    // snapshot and begin_build (watcher race
                    // + us re-snapshot). Serve current.
                    eprintln!(
                        "TSPv2PoC1: build race -- snapshot said Unloaded/Dirty, begin_build returned NotBuildable({other:?})"
                    );
                    serve_current_pinned_or_500(registry, page_ref)
                }
                Err(BeginBuildError::UnknownPage) => (
                    "HTTP/1.1 500 Internal Server Error",
                    "text/plain; charset=utf-8",
                    None,
                    "TSP v2 PoC 1 slice 12: page not registered in PageRegistry
".to_string(),
                ),
            }
        }
        PageState::Building => handle_building(route, page_ref, registry),
        PageState::Clean => serve_current_pinned_or_500(registry, page_ref),
        // PageState::Failed is handled in the
        // Unloaded|Dirty|Failed arm above (a Failed slot
        // can be rebuilt by begin_build; the failed-then-
        // succeed transition promotes the new build).
    }
}

/// Wait on the shared in-flight build (plan sect.22.4). If
/// the outcome is Ok, serve the pinned body. If the outcome
/// is Failed or Abandoned, fall back to LKG.
fn handle_building(
    route: &crate::router::Route,
    page_ref: &PageRef,
    registry: &PageRegistry,
) -> (&'static str, &'static str, Option<String>, String) {
    let Some(shared) = registry.join_in_flight(page_ref) else {
        // State is Building but the in-flight handle is gone
        // (shouldn't happen with the current state machine,
        // but defend against it by serving LKG).
        return serve_lkg_pinned_or_500(registry, page_ref, route);
    };
    let guard = shared.state.lock().expect("in-flight lock poisoned");
    match shared.wait(guard) {
        InFlightState::Done(BuildOutcome::Ok(arc)) => {
            // Pin the Arc<String> we just received. The
            // request will see this body even if a later
            // commit overwrites current while we are
            // writing the response.
            serve_arc(200, "OK", arc)
        }
        InFlightState::Done(BuildOutcome::Failed(msg)) => {
            eprintln!(
                "TSPv2PoC1: in-flight build for {} failed: {msg}",
                route.source.display()
            );
            serve_lkg_pinned_or_500(registry, page_ref, route)
        }
        InFlightState::Abandoned => {
            eprintln!(
                "TSPv2PoC1: in-flight build for {} abandoned (guard dropped)",
                route.source.display()
            );
            serve_lkg_pinned_or_500(registry, page_ref, route)
        }
        InFlightState::Running => unreachable!("wait() must observe a terminal state"),
    }
}

fn serve_arc(
    _code: u16,
    _reason: &'static str,
    body: Arc<String>,
) -> (&'static str, &'static str, Option<String>, String) {
    ("HTTP/1.1 200 OK", "text/html; charset=utf-8", None, (*body).clone())
}

fn serve_current_pinned_or_500(
    registry: &PageRegistry,
    page_ref: &PageRef,
) -> (&'static str, &'static str, Option<String>, String) {
    match registry.read_current_arc(page_ref) {
        Some(arc) => serve_arc(200, "OK", arc),
        None => (
            "HTTP/1.1 500 Internal Server Error",
            "text/plain; charset=utf-8",
            None,
            "TSP v2 PoC 1 slice 12: Clean slot has no payload
".to_string(),
        ),
    }
}

fn serve_lkg_pinned_or_500(
    registry: &PageRegistry,
    page_ref: &PageRef,
    route: &crate::router::Route,
) -> (&'static str, &'static str, Option<String>, String) {
    match registry.read_lkg_arc(page_ref) {
        Some(arc) => serve_arc(200, "OK", arc),
        None => (
            "HTTP/1.1 500 Internal Server Error",
            "text/plain; charset=utf-8",
            None,
            format!(
                "TSP v2 PoC 1 slice 12: page {} never built successfully
",
                route.source.display()
            ),
        ),
    }
}

// Slice 12 rewrites the legacy helpers:
//   - serve_current_or_500  ->  serve_current_pinned_or_500
//   - serve_lkg_or_503      ->  removed (Building no longer 503s;
//     we share the in-flight build via `handle_building` and
//     fall back to LKG on failure)
//   - serve_lkg_or_500      ->  serve_lkg_pinned_or_500
// All three new helpers live in `render_for_route`'s block
// above. The hot path uses Arc<String> exclusively so a
// mid-flight commit cannot change the body an in-progress
// request observes (plan sect.21.3 request pinning).


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