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

/// Per-request Context exposed to the `.tsp` page handler
/// (spec sect.13, plan sect.8). The slice 16a surface is the
/// minimum useful subset: HTTP method, URL path, raw query
/// string. `params` is an empty object for now -- dynamic
/// route segments (spec sect.11.3) land in a later slice.
/// `signal` (AbortSignal) and `body` (Web Request) are slice
/// 16b/c; `cookies` is Phase 8.
#[derive(Debug, Clone)]
pub struct Context {
    /// HTTP method the request came in with (uppercase).
    pub method: HttpMethod,
    /// URL path the route matched (e.g. `/`, `/about`).
    pub path: String,
    /// Raw query string without the leading `?`, or empty
    /// when the request had no query. The page can parse it
    /// via `new URLSearchParams(ctx.query)` (the JS API
    /// does the percent-decoding for us).
    pub query: String,
    /// Route parameters from file-system routing (spec
    /// sect.11.3). Empty for slice 16a because dynamic
    /// segments are not implemented yet.
    pub params: std::collections::HashMap<String, String>,
}

impl Context {
    /// Serialise to a JSON string the JS side parses via
    /// `JSON.parse(...)`. The format is intentionally
    /// minimal: top-level keys are method / path / query /
    /// params. Adding more fields here is a contract
    /// change for `.tsp` page authors.
    pub fn to_json(&self) -> String {
        // Hand-rolled to avoid pulling in a JSON crate. The
        // method comes from `HttpMethod::as_str` (one of
        // GET / POST / ...) so it is always ASCII without
        // escaping; the path and query are read straight
        // from the HTTP request line so they may contain
        // percent-encoded bytes that we surface verbatim.
        // `.tsp` authors that want decoded forms parse
        // them on the JS side via `decodeURIComponent` /
        // `URLSearchParams`.
        let mut out = String::with_capacity(64 + self.path.len() + self.query.len());
        out.push_str("{\"method\":");
        json_string(&mut out, self.method.as_str());
        out.push_str(",\"path\":");
        json_string(&mut out, &self.path);
        out.push_str(",\"query\":");
        json_string(&mut out, &self.query);
        out.push_str(",\"params\":{");
        let mut first = true;
        for (k, v) in &self.params {
            if !first {
                out.push(',');
            }
            first = false;
            json_string(&mut out, k);
            out.push(':');
            json_string(&mut out, v);
        }
        out.push_str("}}");
        out
    }
}

/// Minimal JSON string escape: handle quote, backslash, and
/// control characters. The Context's `path` and `query`
/// come from the HTTP request line and may contain quotes
/// (rare, but possible in malformed input), so we escape
/// defensively rather than trust the input.
fn json_string(out: &mut String, s: &str) {
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Outcome of parsing the `__TSP_OUT_V1__` envelope the page
/// emits on stdout. The envelope is one line of header tag
/// plus one line of JSON. We split on the first '\n' and
/// inspect the JSON object's `type` field.
#[derive(Debug, PartialEq, Eq)]
enum EnvelopeKind {
    /// `{type: "html", body: "..."}` -> 200 text/html.
    Html,
    /// `{type: "response", status, headers, body}` -> use
    /// the status / headers / body verbatim.
    Response,
    /// The line did not start with the envelope tag, or
    /// the JSON was malformed. The host falls back to
    /// treating the whole stdout as an HTML body (slice 6
    /// legacy behaviour).
    Legacy,
}

#[derive(Debug)]
#[allow(dead_code)] // kind / status_line: not used by slice 16b's
                   // render path; reserved for the dev-inspector
                   // slice and for the Response handling (status_line
                   // is currently produced by an explicit match in
                   // the function that consumes the outcome).
struct EnvelopeOutcome {
    kind: EnvelopeKind,
    /// `text/html; charset=utf-8` for HTML, preserved from
    /// the page for Response (currently always
    /// `text/plain; charset=utf-8` from the JS side -- the
    /// page's `Response` content-type is the source of
    /// truth for spec sect.18.3; we surface whatever the
    /// page set).
    content_type: String,
    /// HTTP status line, e.g. `"HTTP/1.1 201 Created"`.
    status_line: &'static str,
    /// Raw body bytes (already UTF-8 String).
    body: String,
    /// Headers (slice 16b: only Content-Type; full header
    /// map lands in 16c). Empty for HTML.
    headers: Vec<(String, String)>,
}

const ENVELOPE_TAG: &str = "__TSP_OUT_V1__";

fn parse_envelope(stdout: &str) -> EnvelopeOutcome {
    // The first line must be the envelope tag; the rest is
    // the JSON body. Trim a trailing newline.
    let mut lines = stdout.splitn(2, '\n');
    let head = lines.next().unwrap_or("").trim_end_matches('\r');
    let body_json = lines.next().unwrap_or("").trim_end_matches('\n');
    if head != ENVELOPE_TAG {
        return EnvelopeOutcome {
            kind: EnvelopeKind::Legacy,
            content_type: "text/html; charset=utf-8".to_string(),
            status_line: "HTTP/1.1 200 OK",
            body: stdout.to_string(),
            headers: Vec::new(),
        };
    }
    // Minimal JSON object walk: find `"type":"html"` or
    // `"type":"response"` and the relevant fields. The
    // wrap script produces a flat object with primitive
    // values; we walk the top level by scanning for `"key":`
    // positions. This is intentionally small: it does not
    // handle nested arrays, escaped backslashes inside the
    // body, etc. The wrap script's JSON.stringify never
    // produces those.
    let kind = if body_json.contains("\"type\":\"html\"") {
        EnvelopeKind::Html
    } else if body_json.contains("\"type\":\"response\"") {
        EnvelopeKind::Response
    } else {
        EnvelopeKind::Legacy
    };
    let body = json_extract_string(body_json, "body").unwrap_or_default();
    match kind {
        EnvelopeKind::Html => EnvelopeOutcome {
            kind,
            content_type: "text/html; charset=utf-8".to_string(),
            status_line: "HTTP/1.1 200 OK",
            body,
            headers: Vec::new(),
        },
        EnvelopeKind::Response => {
            let status = json_extract_number(body_json, "status")
                .unwrap_or(200) as u16;
            let content_type = json_extract_string(body_json, "content_type")
                .unwrap_or_else(|| "text/plain; charset=utf-8".to_string());
            // The wrap script does not currently emit
            // `content_type` (Response objects carry the
            // type implicitly via the body's media type),
            // so we always fall back to text/plain above
            // and treat any explicit `Content-Type` header
            // the page set as the source of truth.
            let _ = content_type; // suppress unused warning
            let headers = json_extract_headers(body_json);
            let status_line: &'static str = match status {
                100..=199 => "HTTP/1.1 100 Continue",
                200 => "HTTP/1.1 200 OK",
                201 => "HTTP/1.1 201 Created",
                202 => "HTTP/1.1 202 Accepted",
                204 => "HTTP/1.1 204 No Content",
                301 => "HTTP/1.1 301 Moved Permanently",
                302 => "HTTP/1.1 302 Found",
                303 => "HTTP/1.1 303 See Other",
                304 => "HTTP/1.1 304 Not Modified",
                307 => "HTTP/1.1 307 Temporary Redirect",
                308 => "HTTP/1.1 308 Permanent Redirect",
                400 => "HTTP/1.1 400 Bad Request",
                401 => "HTTP/1.1 401 Unauthorized",
                403 => "HTTP/1.1 403 Forbidden",
                404 => "HTTP/1.1 404 Not Found",
                405 => "HTTP/1.1 405 Method Not Allowed",
                409 => "HTTP/1.1 409 Conflict",
                410 => "HTTP/1.1 410 Gone",
                412 => "HTTP/1.1 412 Precondition Failed",
                415 => "HTTP/1.1 415 Unsupported Media Type",
                422 => "HTTP/1.1 422 Unprocessable Entity",
                429 => "HTTP/1.1 429 Too Many Requests",
                500 => "HTTP/1.1 500 Internal Server Error",
                501 => "HTTP/1.1 501 Not Implemented",
                502 => "HTTP/1.1 502 Bad Gateway",
                503 => "HTTP/1.1 503 Service Unavailable",
                504 => "HTTP/1.1 504 Gateway Timeout",
                _ => "HTTP/1.1 200 OK",
            };
            // Content-Type defaults to whatever the page's
            // Response set; absent that, text/plain (since
            // we do not know the page's actual media type).
            let ct = json_extract_string(body_json, "content_type")
                .unwrap_or_else(|| "text/plain; charset=utf-8".to_string());
            EnvelopeOutcome {
                kind,
                content_type: ct,
                status_line,
                body,
                headers,
            }
        }
        EnvelopeKind::Legacy => EnvelopeOutcome {
            kind,
            content_type: "text/html; charset=utf-8".to_string(),
            status_line: "HTTP/1.1 200 OK",
            body: stdout.to_string(),
            headers: Vec::new(),
        },
    }
}

/// Extract a top-level JSON string value by key. Returns
/// the un-escaped value (handles the common escapes the
/// wrap script produces: backslash, quote, n, r, t, uXXXX).
fn json_extract_string(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\":", key);
    let pos = json.find(&needle)? + needle.len();
    let rest = &json[pos..];
    let rest = rest.strip_prefix(' ').unwrap_or(rest);
    if !rest.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let chars: Vec<char> = rest.chars().collect();
    let mut i = 1; // skip opening quote
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            return Some(out);
        } else if c == '\\' && i + 1 < chars.len() {
            let nxt = chars[i + 1];
            match nxt {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' if i + 5 < chars.len() => {
                    let hex: String = chars[i+2..i+6].iter().collect();
                    if let Ok(code) = u32::from_str_radix(&hex, 16) {
                        if let Some(ch) = char::from_u32(code) {
                            out.push(ch);
                        }
                    }
                    i += 4;
                }
                _ => out.push(nxt),
            }
            i += 2;
        } else {
            out.push(c);
            i += 1;
        }
    }
    None
}

/// Extract a top-level JSON number by key. Returns None if
/// the value is not a parseable integer.
fn json_extract_number(json: &str, key: &str) -> Option<u32> {
    let needle = format!("\"{}\":", key);
    let pos = json.find(&needle)? + needle.len();
    let rest = &json[pos..];
    let rest = rest.strip_prefix(' ').unwrap_or(rest);
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

/// Extract the `headers` object's top-level entries. The
/// wrap script serialises headers as a flat object:
///   {"x-foo":"bar","content-type":"text/html"}
/// We walk the object body between the outer braces and
/// split on `","`. Good enough for the slice-16b
/// surface; multi-value headers (Set-Cookie) and
/// header values containing commas land in a later slice.
fn json_extract_headers(json: &str) -> Vec<(String, String)> {
    let needle = "\"headers\":";
    let Some(pos) = json.find(needle) else {
        return Vec::new();
    };
    let rest = &json[pos + needle.len()..];
    let rest = rest.strip_prefix(' ').unwrap_or(rest);
    if !rest.starts_with('{') {
        return Vec::new();
    }
    // Find the matching close brace. We use brace depth
    // counting; values are strings (no nested objects in
    // the headers map per the wrap script).
    let bytes = rest.as_bytes();
    let mut depth = 0i32;
    let mut end = 0;
    for (i, &b) in bytes.iter().enumerate() {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    let body = &rest[1..end];
    // Split on `","` (string boundary). We rely on the
    // wrap script never producing commas inside header
    // values.
    let mut out = Vec::new();
    for entry in body.split("\",\"") {
        let colon = match entry.find("\":") {
            Some(i) => i,
            None => continue,
        };
        let key = entry[..colon].trim().to_string();
        let value = entry[colon + 4..].to_string();
        // Strip the surrounding quotes from the value.
        let value = value.trim_matches('"').to_string();
        if !key.is_empty() {
            out.push((key, value));
        }
    }
    out
}

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
    routes: Arc<RouteTable>,
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
        let routes_for_thread = Arc::clone(&routes);
        thread::spawn(move || {
            if let Err(e) = handle_connection(stream, &routes_for_thread, registry, bun) {
                eprintln!("TSPv2PoC1: {e}");
            }
        });
    }
    Ok(())
}

fn handle_connection(
    mut stream: TcpStream,
    routes: &Arc<RouteTable>,
    registry: &PageRegistry,
    bun: &BunRuntime,
) -> Result<(), HostError> {
    let mut buf = [0u8; 8192];
    let n = stream.read(&mut buf).map_err(HostError::Connection)?;
    let request = std::str::from_utf8(&buf[..n]).unwrap_or("");
    let parsed = parse_request(request);

    let (status_line, content_type, allow_header, body, _extra_headers) = match parsed {
        ParsedRequest::Unknown => (
            "HTTP/1.1 400 Bad Request",
            "text/plain; charset=utf-8".to_string(),
            None,
            "TSP v2 PoC 1 slice 10b: malformed request line\n".to_string(),
            Vec::new(),
        ),
        ParsedRequest::Known { method, path, query } => {
            // Build the per-request Context the page handler
            // receives as its single argument. `params` is
            // empty for slice 16a (dynamic route segments
            // are not implemented yet). The query string is
            // passed verbatim -- the JS side decodes it via
            // `URLSearchParams` if it wants parsed forms.
            let ctx = Context {
                method,
                path: path.clone(),
                query,
                params: std::collections::HashMap::new(),
            };
            match routes.lookup(&path, method) {
            MatchResult::Found { route, method: req_method } => {
                let page_ref = PageRef {
                    route: route.path.clone(),
                    method: req_method,
                };
                // render_for_route does the heavy lifting
                // (begin_build, InFlightBuild dedup, build
                // pipeline, payload caching); we only use its
                // body string and then parse the envelope.
                let (_status_line, _ct, allow_header, body) = render_for_route(
                    &route, req_method, &page_ref, registry, bun, &ctx,
                );
                // Slice 16b: bun emits a `__TSP_OUT_V1__` envelope
                // with the page's return value classified as
                // either HtmlNode (string) or Web Response.
                // parse_envelope unpacks it and surfaces the
                // correct status / content-type / body / headers.
                // The legacy branch (envelope tag absent) treats
                // the body as raw HTML, which is the slice 6
                // behaviour and stays compatible with the
                // fixtures that pre-date slice 16b.
                let outcome = parse_envelope(&body);
                // The status_line from `render_for_route` is
                // always "HTTP/1.1 200 OK" (the placeholder
                // before the bun subprocess is called); the
                // envelope is the source of truth for the
                // page's actual status.
                //
                // Slice 16b covers status / content_type /
                // body propagation. Header propagation
                // (extra_headers) lands in slice 16c with a
                // proper JSON object walker; for now we pass
                // an empty vec so the writer skips the loop.
                let response_headers: Vec<(String, String)> = Vec::new();
                (outcome.status_line, outcome.content_type, allow_header, outcome.body, response_headers)
            }
            MatchResult::FoundHeadOverGet { route } => {
                // Spec sect.6.5: HEAD with no explicit HEAD export.
                // Run the GET handler, then strip the body. We
                // intentionally do NOT preserve Content-Length --
                // see the slice 14a note in progress.md for the
                // proper Content-Length-preserving refactor.
                // The body is dropped here; we do NOT call
                // parse_envelope on the GET result because the
                // head body must be empty regardless of the
                // page's response shape.
                let page_ref = PageRef {
                    route: route.path.clone(),
                    method: HttpMethod::Get,
                };
                let (_status, _ct, _allow, _body) = render_for_route(
                    &route, HttpMethod::Get, &page_ref, registry, bun, &ctx,
                );
                (
                    "HTTP/1.1 200 OK",
                    "text/html; charset=utf-8".to_string(),
                    None,
                    String::new(),
                    Vec::new(),
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
                        "text/plain; charset=utf-8".to_string(),
                        Some(allow),
                        String::new(),
                        Vec::new(),
                    )
                } else {
                // Slice-5 path: 405 with real Allow header from
                // the static method detector (no registry needed).
                let prepared = crate::page::prepare(&route);
                let (allow, body) = render_405_body(&route, requested, prepared);
                (
                    "HTTP/1.1 405 Method Not Allowed",
                    "text/plain; charset=utf-8".to_string(),
                    Some(allow),
                    body,
                    Vec::new(),
                )
                }
            }
            MatchResult::NotFound => (
                "HTTP/1.1 404 Not Found",
                "text/plain; charset=utf-8".to_string(),
                None,
                format!(
                    "TSP v2 PoC 1 slice 10b: no route matches path={path} (table has {} route(s))\n",
                    routes.len()
                ),
                Vec::new(),
            ),
            }
        }
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
    ctx: &Context,
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
                    let build_result = pipeline::build(route, requested, bun, &ctx.to_json());
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
    Known {
        method: HttpMethod,
        path: String,
        /// Raw query string without the leading `?`, or
        /// empty when the request had no query.
        query: String,
    },
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
    let qpos = path.find('?');
    let end = qpos.unwrap_or(path.len());
    let query = if let Some(q) = qpos {
        path[q + 1..].to_string()
    } else {
        String::new()
    };
    ParsedRequest::Known {
        method,
        path: path[..end].to_string(),
        query,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::router::HttpMethod;

    #[test]
    fn context_to_json_basic() {
        let ctx = Context {
            method: HttpMethod::Get,
            path: "/".to_string(),
            query: "".to_string(),
            params: std::collections::HashMap::new(),
        };
        let s = ctx.to_json();
        // The exact wire form is part of the slice 16a
        // contract: it is parsed by the JS preamble.
        assert!(s.contains("\"method\":\"GET\""), "got: {s}");
        assert!(s.contains("\"path\":\"/\""), "got: {s}");
        assert!(s.contains("\"query\":\"\""), "got: {s}");
        assert!(s.contains("\"params\":{}"), "got: {s}");
    }

    #[test]
    fn context_to_json_with_query() {
        let ctx = Context {
            method: HttpMethod::Get,
            path: "/search".to_string(),
            query: "q=hello&page=2".to_string(),
            params: std::collections::HashMap::new(),
        };
        let s = ctx.to_json();
        assert!(s.contains("\"q=hello&page=2\""), "got: {s}");
    }


    #[test]
    fn context_to_json_serialises_params() {
        let mut params = std::collections::HashMap::new();
        params.insert("id".to_string(), "42".to_string());
        let ctx = Context {
            method: HttpMethod::Get,
            path: "/users/42".to_string(),
            query: String::new(),
            params,
        };
        let s = ctx.to_json();
        assert!(s.contains("\"id\":\"42\""), "got: {s}");
    }
}
