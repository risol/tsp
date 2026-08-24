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

/// Outcome of parsing the `__TSP_OUT_V1__` envelope the page
/// emits on stdout.
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
#[allow(dead_code)] // kind / status_line: used by the dev-
                   // inspector slice; parse_envelope produces
                   // them for the request path.
struct EnvelopeOutcome {
    kind: EnvelopeKind,
    content_type: String,
    status_line: &'static str,
    body: String,
    headers: Vec<(String, String)>,
}

const ENVELOPE_TAG: &str = "__TSP_OUT_V1__";

/// Minimal JSON string escape: handle quote, backslash, and
/// control characters. The Context's `path` and `query`
/// come from the HTTP request line and may contain quotes
/// (rare, but possible in malformed input), so we escape
/// defensively rather than trust the input. This is the
/// serializer counterpart of the parser below.
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

/// A tiny JSON value tree. This is the host-side parser for
/// the `__TSP_OUT_V1__` envelope and the page's headers
/// object. It is intentionally minimal (no serde dep stays
/// in the slice-16 cost envelope) but correct for the
/// shapes the wrap script produces: flat objects, strings,
/// numbers, booleans, null, and arrays of strings.
///
/// Accepted grammar (subset of RFC 8259):
///   json = value
///   value = object | array | string | number | true | false | null
/// Strings can contain escaped quote / backslash / n r t uXXXX.
#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    /// Look up a top-level object field by key (only valid
    /// on Object).
    fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            JsonValue::Object(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Treat the value as a JSON string and return its text.
    fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Treat the value as a JSON number and return it
    /// rounded to u16 (for the HTTP status code).
    fn as_u16(&self) -> Option<u16> {
        match self {
            JsonValue::Number(n) => Some(n.round() as u16),
            _ => None,
        }
    }
}

/// Parse a complete JSON document (no trailing input).
fn parse_json(text: &str) -> Option<JsonValue> {
    let mut p = JsonParser { bytes: text.as_bytes(), pos: 0 };
    let v = p.parse_value()?;
    p.skip_ws();
    if p.pos == p.bytes.len() {
        Some(v)
    } else {
        None
    }
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\n' | b'\r') {
            self.pos += 1;
        }
    }

    fn peek(&self) -> u8 {
        *self.bytes.get(self.pos).unwrap_or(&0)
    }

    fn parse_value(&mut self) -> Option<JsonValue> {
        self.skip_ws();
        match self.peek() {
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'"' => self.parse_string().map(JsonValue::String),
            b't' => self.parse_keyword("true").map(|_| JsonValue::Bool(true)),
            b'f' => self.parse_keyword("false").map(|_| JsonValue::Bool(false)),
            b'n' => self.parse_keyword("null").map(|_| JsonValue::Null),
            b'-' | b'0'..=b'9' => self.parse_number(),
            _ => None,
        }
    }

    fn parse_keyword(&mut self, kw: &str) -> Option<()> {
        if self.bytes[self.pos..].starts_with(kw.as_bytes()) {
            self.pos += kw.len();
            Some(())
        } else {
            None
        }
    }

    fn parse_string(&mut self) -> Option<String> {
        // Assumes the caller is AT the opening quote.
        if self.peek() != b'"' {
            return None;
        }
        self.pos += 1;
        let mut out = String::new();
        loop {
            let ch = *self.bytes.get(self.pos)?;
            match ch {
                b'"' => {
                    self.pos += 1;
                    return Some(out);
                }
                b'\\' => {
                    self.pos += 1;
                    let esc = *self.bytes.get(self.pos)?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            if self.pos + 4 > self.bytes.len() {
                                return None;
                            }
                            let hex = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4]).ok()?;
                            let code = u32::from_str_radix(hex, 16).ok()?;
                            out.push(char::from_u32(code)?);
                            self.pos += 4;
                        }
                        _ => return None,
                    }
                }
                _ => {
                    // UTF-8 decode a single char (multi-byte safe).
                    let rest = &self.bytes[self.pos..];
                    let s = std::str::from_utf8(rest).ok()?;
                    let c = s.chars().next()?;
                    out.push(c);
                    self.pos += c.len_utf8();
                }
            }
        }
    }

    fn parse_number(&mut self) -> Option<JsonValue> {
        let start = self.pos;
        while self.pos < self.bytes.len()
            && matches!(self.bytes[self.pos], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
        {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).ok()?;
        text.parse::<f64>().ok().map(JsonValue::Number)
    }

    fn parse_array(&mut self) -> Option<JsonValue> {
        // Assumes the caller is AT the opening '['.
        self.pos += 1;
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                b']' => {
                    self.pos += 1;
                    return Some(JsonValue::Array(items));
                }
                b',' => {
                    self.pos += 1;
                }
                _ => {
                    let v = self.parse_value()?;
                    items.push(v);
                }
            }
        }
    }

    fn parse_object(&mut self) -> Option<JsonValue> {
        // Assumes the caller is AT the opening '{'.
        self.pos += 1;
        let mut entries = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                b'}' => {
                    self.pos += 1;
                    return Some(JsonValue::Object(entries));
                }
                b',' => {
                    self.pos += 1;
                }
                b'"' => {
                    let key = self.parse_string()?;
                    self.skip_ws();
                    if self.peek() != b':' {
                        return None;
                    }
                    self.pos += 1;
                    let v = self.parse_value()?;
                    entries.push((key, v));
                }
                _ => return None,
            }
        }
    }
}

/// Parse the `__TSP_OUT_V1__` envelope (status / content /
/// body / headers) into a typed outcome.
///
/// Envelope wire shape (one line): `__TSP_OUT_V1__\n{json}`.
/// The JSON is a flat object:
///   {"type":"html","body":"..."}              (slice 6 / 16a)
///   {"type":"response","status":201,"headers":{"x-foo":"bar"},"body":"..."}
fn parse_envelope(stdout: &str) -> EnvelopeOutcome {
    // The first line must be the envelope tag; the rest is
    // the JSON body.
    let mut lines = stdout.splitn(2, '\n');
    let head = lines.next().unwrap_or("").trim_end_matches('\r');
    let body_json = lines.next().unwrap_or("").trim_end_matches('\n');

    let EnvelopeOutcome {
        kind,
        content_type,
        status_line,
        body,
        headers,
    } = if head == ENVELOPE_TAG {
        match parse_json(body_json) {
            Some(JsonValue::Object(entries)) => {
                let obj = JsonValue::Object(entries);
                let ty = obj.get("type").and_then(JsonValue::as_str).unwrap_or("");
                let kind = match ty {
                    "html" => EnvelopeKind::Html,
                    "response" => EnvelopeKind::Response,
                    _ => EnvelopeKind::Legacy,
                };
                let body = obj.get("body").and_then(JsonValue::as_str).unwrap_or("").to_string();
                let mut headers: Vec<(String, String)> = Vec::new();
                if let Some(JsonValue::Object(hs)) = obj.get("headers") {
                    for (k, v) in hs {
                        if let Some(s) = v.as_str() {
                            headers.push((k.clone(), s.to_string()));
                        }
                    }
                }
                match kind {
                    EnvelopeKind::Html => EnvelopeOutcome {
                        kind,
                        content_type: "text/html; charset=utf-8".to_string(),
                        status_line: "HTTP/1.1 200 OK",
                        body,
                        headers,
                    },
                    EnvelopeKind::Response => {
                        let status = obj.get("status").and_then(JsonValue::as_u16).unwrap_or(200);
                        let status_line = status_line_for(status);
                        let ct = headers
                            .iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                            .map(|(_, v)| v.clone())
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
                        body,
                        headers,
                    },
                }
            }
            _ => EnvelopeOutcome {
                kind: EnvelopeKind::Legacy,
                content_type: "text/html; charset=utf-8".to_string(),
                status_line: "HTTP/1.1 200 OK",
                body: stdout.to_string(),
                headers: Vec::new(),
            },
        }
    } else {
        // Legacy: no envelope tag -> raw HTML body.
        EnvelopeOutcome {
            kind: EnvelopeKind::Legacy,
            content_type: "text/html; charset=utf-8".to_string(),
            status_line: "HTTP/1.1 200 OK",
            body: stdout.to_string(),
            headers: Vec::new(),
        }
    };

    EnvelopeOutcome {
        kind,
        content_type,
        status_line,
        body,
        headers,
    }
}

/// Map an HTTP status code to its canonical reason-phrase
/// (the wire form the host emits). Unknown codes fall back
/// to "HTTP/1.1 200 OK" -- accepting a page that returns a
/// code we do not document is better than failing the
/// whole request; the dev-inspector slice can surface the
/// unmapped code.
fn status_line_for(status: u16) -> &'static str {
    match status {
        100 => "HTTP/1.1 100 Continue",
        101 => "HTTP/1.1 101 Switching Protocols",
        200 => "HTTP/1.1 200 OK",
        201 => "HTTP/1.1 201 Created",
        202 => "HTTP/1.1 202 Accepted",
        203 => "HTTP/1.1 203 Non-Authoritative Information",
        204 => "HTTP/1.1 204 No Content",
        205 => "HTTP/1.1 205 Reset Content",
        206 => "HTTP/1.1 206 Partial Content",
        300 => "HTTP/1.1 300 Multiple Choices",
        301 => "HTTP/1.1 301 Moved Permanently",
        302 => "HTTP/1.1 302 Found",
        303 => "HTTP/1.1 303 See Other",
        304 => "HTTP/1.1 304 Not Modified",
        307 => "HTTP/1.1 307 Temporary Redirect",
        308 => "HTTP/1.1 308 Permanent Redirect",
        400 => "HTTP/1.1 400 Bad Request",
        401 => "HTTP/1.1 401 Unauthorized",
        402 => "HTTP/1.1 402 Payment Required",
        403 => "HTTP/1.1 403 Forbidden",
        404 => "HTTP/1.1 404 Not Found",
        405 => "HTTP/1.1 405 Method Not Allowed",
        406 => "HTTP/1.1 406 Not Acceptable",
        408 => "HTTP/1.1 408 Request Timeout",
        409 => "HTTP/1.1 409 Conflict",
        410 => "HTTP/1.1 410 Gone",
        411 => "HTTP/1.1 411 Length Required",
        412 => "HTTP/1.1 412 Precondition Failed",
        413 => "HTTP/1.1 413 Payload Too Large",
        414 => "HTTP/1.1 414 URI Too Long",
        415 => "HTTP/1.1 415 Unsupported Media Type",
        416 => "HTTP/1.1 416 Range Not Satisfiable",
        417 => "HTTP/1.1 417 Expectation Failed",
        418 => "HTTP/1.1 418 I'm a teapot",
        422 => "HTTP/1.1 422 Unprocessable Entity",
        425 => "HTTP/1.1 425 Too Early",
        426 => "HTTP/1.1 426 Upgrade Required",
        429 => "HTTP/1.1 429 Too Many Requests",
        431 => "HTTP/1.1 431 Request Header Fields Too Large",
        451 => "HTTP/1.1 451 Unavailable For Legal Reasons",
        500 => "HTTP/1.1 500 Internal Server Error",
        501 => "HTTP/1.1 501 Not Implemented",
        502 => "HTTP/1.1 502 Bad Gateway",
        503 => "HTTP/1.1 503 Service Unavailable",
        504 => "HTTP/1.1 504 Gateway Timeout",
        505 => "HTTP/1.1 505 HTTP Version Not Supported",
        _ => "HTTP/1.1 200 OK",
    }
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

    let (status_line, content_type, allow_header, body, extra_headers) = match parsed {
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
                (outcome.status_line, outcome.content_type, allow_header, outcome.body, outcome.headers)
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

    // Slice 16c: emit the page's extra headers before
    // Content-Type. Content-Type / Content-Length are
    // computed by the host (from the envelope and the body
    // length respectively), so the page's own copies are
    // skipped.
    let mut header_block = String::new();
    for (k, v) in &extra_headers {
        if k.eq_ignore_ascii_case("content-type") {
            continue;
        }
        if k.eq_ignore_ascii_case("content-length") {
            continue;
        }
        header_block.push_str(k);
        header_block.push_str(": ");
        header_block.push_str(v);
        header_block.push_str("\r\n");
    }
    let mut head = format!(
        "{status_line}\r\n\
         {header_block}Content-Type: {content_type}\r\n\
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

#[cfg(test)]
    

    #[test]
    fn envelope_parses_html() {
        let out = parse_envelope("__TSP_OUT_V1__\n{\"type\":\"html\",\"body\":\"<h1>Hi</h1>\"}");
        assert_eq!(out.kind, EnvelopeKind::Html);
        assert_eq!(out.body, "<h1>Hi</h1>");
        assert_eq!(out.status_line, "HTTP/1.1 200 OK");
        assert_eq!(out.content_type, "text/html; charset=utf-8");
        assert!(out.headers.is_empty());
    }

    #[test]
    fn envelope_parses_response_with_headers() {
        let out = parse_envelope(
            "__TSP_OUT_V1__\n{\"type\":\"response\",\"status\":201,\"headers\":{\"x-demo\":\"slice16c\",\"x-comma\":\"a,b,c\",\"content-type\":\"application/json\"},\"body\":\"created\"}",
        );
        assert_eq!(out.kind, EnvelopeKind::Response);
        assert_eq!(out.status_line, "HTTP/1.1 201 Created");
        assert_eq!(out.content_type, "application/json");
        assert_eq!(out.body, "created");
        // Headers: content-type is derived into content_type,
        // both extras are propagated.
        assert_eq!(out.headers.len(), 3);
        assert!(out.headers.contains(&("x-demo".to_string(), "slice16c".to_string())));
        assert!(out.headers.contains(&("x-comma".to_string(), "a,b,c".to_string())));
    }

    #[test]
    fn envelope_legacy_when_no_tag() {
        // A raw body (no envelope tag) is treated as HTML.
        let out = parse_envelope("<h1>raw</h1>");
        assert_eq!(out.kind, EnvelopeKind::Legacy);
        assert_eq!(out.body, "<h1>raw</h1>");
        assert_eq!(out.status_line, "HTTP/1.1 200 OK");
    }

    #[test]
    fn envelope_unknown_status_falls_back_200() {
        let out = parse_envelope("__TSP_OUT_V1__\n{\"type\":\"response\",\"status\":599,\"headers\":{},\"body\":\"x\"}");
        assert_eq!(out.status_line, "HTTP/1.1 200 OK");
        assert_eq!(out.body, "x");
    }

    #[test]
    fn json_parser_handles_escaped_quotes() {
        // A header value containing an escaped quote.
        let out = parse_envelope(
            "__TSP_OUT_V1__\n{\"type\":\"response\",\"status\":200,\"headers\":{\"x-quote\":\"say \\\"hi\\\"\"},\"body\":\"ok\"}",
        );
        assert!(out.headers.contains(&("x-quote".to_string(), "say \"hi\"".to_string())));
    }
}
