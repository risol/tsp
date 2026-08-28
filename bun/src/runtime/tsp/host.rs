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
use crate::jsc_bridge::{BunRuntime, CancellationToken};
use crate::metrics;
use crate::pipeline;
use crate::router::{HttpMethod, MatchResult, RouteTable};
use crate::services::{
    BUILTIN_SESSION, LogLine, ServiceRegistry, SessionService, SessionValue, SessionView,
    SessionWrite,
};
use std::sync::Arc;
use std::sync::RwLock;

/// Stable error codes for development diagnostics
/// (spec sect.6.3 / sect.37). The full set lives in
/// `docs/v2/progress.md` slice 16h; the contract is:
///
/// - 1xxx: route / filesystem configuration errors
///   (scanner + shape + duplicate + ambiguous)
/// - 2xxx: request input errors (parse / size / params)
/// - 3xxx: build / handler runtime errors (prepare /
///   jsx / subprocess / invalid return / render)
/// - 4xxx: reserved for response-state codes (the
///   spec lists 405 in sect.36.3 as a routing concern,
///   not a typed error -- 405 stays plain). When a
///   response-state error acquires a code, the prefix
///   stays at 4xxx.
///
/// The wire form is `[TSP-NNNN] <detail>` on the FIRST
/// line of the error body. Subsequent lines keep the
/// pre-16h "TSP v2 PoC 1 slice N: ..." trace so existing
/// tooling that greps for `slice 12` etc. continues to
/// work.
///
/// Note on overlap with the page-side examples: the
/// 10 `.tsp` fixtures in `docs/v2/examples/` use the
/// 2xxx / 3xxx ranges from the application surface
/// perspective (the page author types `TSP2003` /
/// `TSP3001` into their dev loop). 16h's prefix table
/// is the runtime surface; the dev-visible codes are a
/// subset (e.g. `TSP2003` here = "no route matches",
/// while `TSP2003` in the spec examples = "shape magic
/// in a fixture"). The codes are stable within the
/// runtime; the dev-facing examples are a separate
/// concern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TspError {
    // 1xxx: routing.
    /// `RouterError::RoutesDirMissing` -- the configured
    /// `routes_dir` does not exist or is not a directory.
    RoutesDirMissing,
    /// `RouterError::UnsupportedShape` -- a `.tsp` file
    /// has a name we cannot translate to a URL path
    /// (e.g. `[1st].tsp`, non-final catch-all, dynamic
    /// without a name).
    UnsupportedRouteShape,
    /// `RouterError::DuplicatePath` -- two scanned
    /// routes produce the same canonical URL path.
    DuplicateRoutePath,
    /// `RouterError::Io` -- stat / read_dir failed
    /// for some path inside the routes root.
    RouteIoError,
    // 2xxx: request input.
    /// The request line was missing or unparseable
    /// (Unknown from parse_request).
    MalformedRequestLine,
    /// `Content-Length` exceeded the configured body
    /// limit (spec sect.14.2 -> 413 before page runs).
    BodyTooLarge,
    /// No route matched the request path.
    NoRouteMatches,
    /// The request path contained an invalid percent escape or UTF-8
    /// sequence (spec sect.11.8 -> 400).
    MalformedUrl,
    /// The matched route did not export the requested
    /// HTTP method (spec sect.6.4 -> 405 + Allow).
    MethodNotAllowed,
    // 3xxx: build / handler.
    /// The page's source could not be read or its
    /// method exports could not be detected.
    PagePrepareError,
    /// The generation cache holds no current payload
    /// for a Clean slot -- a build that should have
    /// produced a body did not (defence-in-depth; the
    /// commit path always sets payload).
    CleanSlotMissingPayload,
    /// A page slot has no successful build ever (LKG
    /// empty) and the request needs it now -- the
    /// host serves 500 because there is nothing to
    /// fall back to.
    PageNeverBuilt,
    /// The matched route has no slot in the
    /// `PageRegistry` (boot-time scan and per-method
    /// detector disagree on whether the method is
    /// exported). Surfaces as 500 in the dispatcher's
    /// "UnknownPage" arm.
    UnknownPage,
}

impl TspError {
    /// The canonical `TSP-NNNN` string. Spec sect.6.3
    /// and FREEZE item 14 anchor these codes; the
    /// application-side spec examples in
    /// `docs/v2/examples/09-no-tsp-imports.tsp` and
    /// `10-shape-magic.tsp` reference `TSP2003` /
    /// `TSP3001` directly.
    pub fn code(self) -> &'static str {
        match self {
            Self::RoutesDirMissing => "TSP1001",
            Self::UnsupportedRouteShape => "TSP1002",
            Self::DuplicateRoutePath => "TSP1003",
            Self::RouteIoError => "TSP1004",
            Self::MalformedRequestLine => "TSP2001",
            Self::BodyTooLarge => "TSP2002",
            Self::NoRouteMatches => "TSP2003",
            Self::MethodNotAllowed => "TSP2004",
            Self::MalformedUrl => "TSP2005",
            Self::PagePrepareError => "TSP3001",
            Self::CleanSlotMissingPayload => "TSP3006",
            Self::PageNeverBuilt => "TSP3007",
            Self::UnknownPage => "TSP3008",
        }
    }

    /// Short human description -- one short line, no
    /// detail. Format: `[TSP-NNNN] <description>`.
    pub fn describe(self) -> &'static str {
        match self {
            Self::RoutesDirMissing => "routes directory not found",
            Self::UnsupportedRouteShape => "unsupported route shape",
            Self::DuplicateRoutePath => "duplicate route path",
            Self::RouteIoError => "route filesystem error",
            Self::MalformedRequestLine => "malformed request line",
            Self::BodyTooLarge => "request body exceeds limit",
            Self::NoRouteMatches => "no route matches",
            Self::MethodNotAllowed => "method not exported by route",
            Self::MalformedUrl => "malformed URL path",
            Self::PagePrepareError => "page prepare error",
            Self::CleanSlotMissingPayload => "clean slot has no payload",
            Self::PageNeverBuilt => "page never built successfully",
            Self::UnknownPage => "page not registered",
        }
    }
}

/// Format an error page body whose first line is
/// `[TSP-NNNN] <description>` (spec sect.6.3 dev
/// diagnostics). The `code` and `description` come
/// from the typed `TspError` enum; for build-time
/// codes that the host enum does not have a variant
/// for (e.g. `TSP3002` JSX transform, `TSP3009`
/// timeout, or `TSP3012` subprocess failure) use
/// `format_error_body_raw`
/// below. Detail lines follow so the pre-16h grep
/// patterns (`TSP v2 PoC 1 slice 12: ...`) keep
/// working -- existing dev tooling scans for those
/// substrings and 16h does not want to break that.
pub fn format_error_body(code: TspError, detail: &str) -> String {
    format_error_body_raw(code.code(), code.describe(), detail)
}

/// Like `format_error_body` but takes the code /
/// description as raw strings. Used by the build
/// pipeline to surface JSX / subprocess / etc. failure
/// codes that the host's own error enum does not model.
pub fn format_error_body_raw(code: &str, description: &str, detail: &str) -> String {
    let mut out = String::new();
    out.push('[');
    out.push_str(code);
    out.push_str("] ");
    out.push_str(description);
    out.push('\n');
    out.push_str(detail);
    if !detail.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Per-request Context exposed to the `.tsp` page handler
/// (spec sect.13, plan sect.8). Slice 16a landed the minimum
/// subset (method / path / query / params); slice 16d adds
/// `body` + `headers` so the JS wrapper can build the Web
/// `Request` object (spec sect.13.3) and derive `ctx.url` /
/// `ctx.query` (spec sect.13.4/13.5) on the JS side.
/// `params` is still an empty map -- dynamic route segments
/// (spec sect.11.3) land in a later slice; `cookies` and
/// `session` are Phase 8.
#[derive(Debug, Clone)]
pub struct Context {
    /// HTTP method the request came in with (uppercase).
    pub method: HttpMethod,
    /// URL path the route matched (e.g. `/`, `/about`).
    pub path: String,
    /// Raw query string without the leading `?`, or empty
    /// when the request had no query. The JS wrapper turns
    /// this into `ctx.url.searchParams` (spec sect.13.5).
    pub query: String,
    /// Route parameters from file-system routing (spec
    /// sect.11.3). Empty for slice 16a because dynamic
    /// segments are not implemented yet.
    pub params: std::collections::HashMap<String, String>,
    /// Request body as raw bytes (slice 16g), or empty when
    /// the request had no body / no Content-Length. The
    /// previous slice 16d design stored a UTF-8-lossy
    /// String; the raw-bytes form is required for
    /// `request.formData()` (spec sect.14.3) to handle
    /// binary multipart bodies without U+FFFD
    /// substitutions. The wrap preamble base64-decodes
    /// `body_b64` and feeds the bytes to Bun's native
    /// `Request` constructor; pages that only need the
    /// text form call `await ctx.request.text()` as usual.
    pub body: Vec<u8>,
    /// Request headers with lower-cased names, wire order.
    /// Duplicate names are joined with ", " (the Web `Headers`
    /// combine rule for non-Set-Cookie headers).
    pub headers: Vec<(String, String)>,
    /// `ctx.services` snapshot for the wire (spec sect.17).
    /// Each entry is `(name, full JSON object)` -- the
    /// runtime-scoped registry snapshot plus any request-scoped
    /// services this request created (16j: registry only). The
    /// wrap preamble hydrates these into `ctx.services`.
    pub services: Vec<(String, String)>,
    /// `ctx.session` view the page reads (spec sect.16).
    /// The host resolves the request's `tsp_sid` cookie
    /// against the runtime SessionService and hands the
    /// page a `(id, data)` snapshot. The page's writes
    /// travel back through the envelope's `session_writes`
    /// field (16k) and are applied by the host before the
    /// response is sent.
    pub session: Option<SessionView>,
}

impl Context {
    /// Serialise to a JSON string the JS side parses via
    /// `JSON.parse(...)`. The format is intentionally
    /// minimal: top-level keys are method / path / query /
    /// params / body_b64 / headers. The body travels as
    /// base64 (the JSON format has no native bytes shape);
    /// the wrap preamble atob-decodes it back to bytes
    /// before constructing `ctx.request`. Adding more
    /// fields here is a contract change for `.tsp` page
    /// authors.
    pub fn to_json(&self) -> String {
        // Hand-rolled to avoid pulling in a JSON crate. The
        // method comes from `HttpMethod::as_str` (one of
        // GET / POST / ...) so it is always ASCII without
        // escaping; the path and query are read straight
        // from the HTTP request line so they may contain
        // percent-encoded bytes that we surface verbatim.
        // `.tsp` authors that want decoded forms parse
        // them on the JS side via `decodeURIComponent` /
        // `URLSearchParams`. The body is base64 (slice
        // 16g, raw bytes for spec sect.14.3 formData).
        // Headers serialise as a flat object (name -> value).
        let mut out =
            String::with_capacity(64 + self.path.len() + self.query.len() + self.body.len());
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
        out.push_str("},\"body_b64\":\"");
        base64_encode(&mut out, &self.body);
        out.push_str("\",\"headers\":{");
        let mut first = true;
        for (k, v) in &self.headers {
            if !first {
                out.push(',');
            }
            first = false;
            json_string(&mut out, k);
            out.push(':');
            json_string(&mut out, v);
        }
        out.push_str("},\"services\":{");
        let mut first = true;
        for (name, json) in &self.services {
            if !first {
                out.push(',');
            }
            first = false;
            json_string(&mut out, name);
            out.push(':');
            // `json` is a full pre-serialised JSON object from
            // `Service::describe_json` (valid JSON, braces
            // included) -- embed verbatim, do NOT re-escape.
            out.push_str(json);
        }
        out.push_str("},\"session\":");
        match &self.session {
            Some(view) => {
                out.push_str(&view.to_json());
            }
            None => out.push_str("null"),
        }
        out.push_str("}");
        out
    }

    /// Add the private fragment selector used only by the native internal
    /// fragment endpoint. It is deliberately separate from the public
    /// Context struct so application code cannot accidentally persist the
    /// selector across requests.
    pub fn to_json_with_fragment(&self, name: Option<&str>) -> String {
        let mut out = self.to_json();
        if out.ends_with('}') {
            out.pop();
            out.push_str(",\"__tsp_fragment_token\":");
            json_string(&mut out, fragment_token());
            if let Some(name) = name {
                out.push_str(",\"__tsp_fragment\":");
                json_string(&mut out, name);
            }
            out.push('}');
        }
        out
    }
}

/// Per-process capability carried by generated fragment URLs. It prevents a
/// caller from dispatching an arbitrary route/name pair to the internal
/// fragment endpoint while keeping the URL opaque to application code.
fn fragment_token() -> &'static str {
    static TOKEN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    TOKEN.get_or_init(|| {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("{:x}-{:x}", nanos, std::process::id())
    })
}

/// Standard Base64 encoder (RFC 4648 section 4, with
/// `+` / `/` / `=`). Used by `Context::to_json` to put
/// the raw request body in the JSON wire form so Bun's
/// `Request` constructor can receive `Uint8Array` for
/// `formData()` multipart parsing. Hand-rolled to keep
/// the slice-16 "no new dep" discipline (plan §25.3);
/// the alphabet is short and the tests catch the
/// obvious off-by-one / padding cases.
fn base64_encode(out: &mut String, bytes: &[u8]) {
    const ALPH: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut i = 0;
    while i + 3 <= bytes.len() {
        let b0 = bytes[i];
        let b1 = bytes[i + 1];
        let b2 = bytes[i + 2];
        out.push(ALPH[(b0 >> 2) as usize] as char);
        out.push(ALPH[((b0 & 0x03) << 4 | (b1 >> 4)) as usize] as char);
        out.push(ALPH[((b1 & 0x0f) << 2 | (b2 >> 6)) as usize] as char);
        out.push(ALPH[(b2 & 0x3f) as usize] as char);
        i += 3;
    }
    match bytes.len() - i {
        1 => {
            let b0 = bytes[i];
            out.push(ALPH[(b0 >> 2) as usize] as char);
            out.push(ALPH[((b0 & 0x03) << 4) as usize] as char);
            out.push('=');
            out.push('=');
        }
        2 => {
            let b0 = bytes[i];
            let b1 = bytes[i + 1];
            out.push(ALPH[(b0 >> 2) as usize] as char);
            out.push(ALPH[((b0 & 0x03) << 4 | (b1 >> 4)) as usize] as char);
            out.push(ALPH[((b1 & 0x0f) << 2) as usize] as char);
            out.push('=');
        }
        _ => {}
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
    /// Log lines the page buffered via `ctx.services.*`
    /// (spec sect.17 / slice 16j). The host flushes them
    /// into the owning runtime service after parsing.
    service_logs: Vec<LogLine>,
    /// Session writes the page applied via `ctx.session.*`
    /// (spec sect.16 / slice 16k). The host applies them
    /// to the runtime SessionService and decides whether
    /// the response needs a new / cleared `tsp_sid` cookie.
    session_writes: Vec<SessionWrite>,
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

    /// Serialise back to a JSON string. Used by
    /// `ctx_json_for_env` (strip the request body from the
    /// env-var side channel). The shapes round-trip exactly:
    /// parse_json(serialize(v)) == v for the subsets the
    /// parser accepts.
    fn serialize(&self, out: &mut String) {
        use std::fmt::Write as _;
        match self {
            JsonValue::Null => out.push_str("null"),
            JsonValue::Bool(true) => out.push_str("true"),
            JsonValue::Bool(false) => out.push_str("false"),
            JsonValue::Number(n) => {
                let _ = write!(out, "{n}");
            }
            JsonValue::String(s) => json_string(out, s),
            JsonValue::Array(items) => {
                out.push('[');
                let mut first = true;
                for item in items {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    item.serialize(out);
                }
                out.push(']');
            }
            JsonValue::Object(entries) => {
                out.push('{');
                let mut first = true;
                for (k, v) in entries {
                    if !first {
                        out.push(',');
                    }
                    first = false;
                    json_string(out, k);
                    out.push(':');
                    v.serialize(out);
                }
                out.push('}');
            }
        }
    }
}

/// Parse a complete JSON document (no trailing input).
fn parse_json(text: &str) -> Option<JsonValue> {
    let mut p = JsonParser {
        bytes: text.as_bytes(),
        pos: 0,
    };
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
        while self.pos < self.bytes.len()
            && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\n' | b'\r')
        {
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
                            let hex =
                                std::str::from_utf8(&self.bytes[self.pos..self.pos + 4]).ok()?;
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
            && matches!(
                self.bytes[self.pos],
                b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
            )
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

/// Result of resolving the request's `tsp_sid` cookie:
/// the cookie value as it came in (may be `None` for
/// first requests) and the live `SessionView` the page
/// reads. The host needs BOTH to decide whether the
/// response must plant a new `tsp_sid` cookie (cookie
/// `None` / unknown / destroyed -> mint; otherwise only
/// on regenerate / destroy).
struct SessionResolve {
    /// Cookie value as the request carried it, or
    /// `None` for a first request / no cookie.
    original_sid: Option<String>,
    /// Resolved view the page reads (`id` is either the
    /// existing row's id or a freshly minted one).
    view: SessionView,
}

/// Resolve the request's `tsp_sid` cookie to a
/// `SessionResolve` (spec sect.16). When the cookie is
/// missing, the session was destroyed, or the id is
/// unknown, mint a fresh one and carry the original
/// `None` / unknown sid so the post-render cookie
/// logic can plant a `Set-Cookie` line.
fn resolve_session_view(svc: &SessionService, sid: Option<&str>) -> SessionResolve {
    let original_sid = sid.map(|s| s.to_string());
    let view = match sid.and_then(|s| svc.lookup(s)) {
        Some(view) => view,
        None => svc.create(),
    };
    SessionResolve { original_sid, view }
}

/// Wire form for the `tsp_sid` cookie (spec sect.16). A
/// page that does not interact with the session still
/// observes a valid (fresh) id; the Set-Cookie line below
/// is what the browser actually persists.
const SESSION_COOKIE_NAME: &str = "tsp_sid";

/// Build the Set-Cookie line the host appends to the
/// response when the session id changed (new / regenerate)
/// or was cleared (destroy). `None` means "no Set-Cookie
/// needed" -- the cookie already matches.
fn build_session_cookie(new_sid: &str, old_sid: &str) -> Option<String> {
    if new_sid == old_sid {
        return None;
    }
    if new_sid.is_empty() {
        // Destroyed: clear the cookie on the client. We
        // intentionally use `Max-Age=0` rather than an
        // expired date so RFC 6265bis / all major browsers
        // agree on the semantics.
        Some(format!(
            "{}=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax",
            SESSION_COOKIE_NAME
        ))
    } else {
        Some(format!(
            "{}={}; Path=/; HttpOnly; SameSite=Lax",
            SESSION_COOKIE_NAME, new_sid
        ))
    }
}

/// Read the `tsp_sid` value out of the lower-cased
/// request-headers vector. Returns `None` when the cookie
/// is absent (first request) or the value is empty.
fn read_session_cookie(headers: &[(String, String)]) -> Option<String> {
    for (k, v) in headers {
        if k == "cookie" {
            for pair in v.split(';') {
                let trimmed = pair.trim();
                let Some(eq) = trimmed.find('=') else {
                    continue;
                };
                let name = trimmed[..eq].trim();
                if name == SESSION_COOKIE_NAME {
                    let val = trimmed[eq + 1..].trim();
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }
        }
    }
    None
}

/// Parse the `__TSP_OUT_V1__` envelope (status / content /
/// body / headers) into a typed outcome.
///
/// Envelope wire shape (one line): `__TSP_OUT_V1__\n{json}`.
/// The JSON is a flat object:
///   {"type":"html","body":"..."}              (slice 6 / 16a)
///   {"type":"response","status":201,"headers":[...],"body":"..."}
///
/// `headers` accepts two shapes for compatibility with
/// older wrap scripts and for the array form introduced in
/// slice 16f:
///   * array of `[name, value]` pairs (slice 16f+,
///     preserves multi-value `Set-Cookie` lines per
///     spec sect.15);
///   * flat `{name: value}` object (slice 16c; comma-folded
///     when the same name appears more than once).
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
        service_logs,
        session_writes,
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
                let body = obj
                    .get("body")
                    .and_then(JsonValue::as_str)
                    .unwrap_or("")
                    .to_string();
                let headers = parse_envelope_headers(obj.get("headers"));
                let service_logs = parse_service_logs(obj.get("service_logs"));
                let session_writes = parse_session_writes(obj.get("session_writes"));
                match kind {
                    EnvelopeKind::Html => EnvelopeOutcome {
                        kind,
                        content_type: "text/html; charset=utf-8".to_string(),
                        status_line: "HTTP/1.1 200 OK",
                        body,
                        headers,
                        service_logs: service_logs.clone(),
                        session_writes: session_writes.clone(),
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
                            service_logs: service_logs.clone(),
                            session_writes: session_writes.clone(),
                        }
                    }
                    EnvelopeKind::Legacy => EnvelopeOutcome {
                        kind,
                        content_type: "text/html; charset=utf-8".to_string(),
                        status_line: "HTTP/1.1 200 OK",
                        body,
                        headers,
                        service_logs: service_logs.clone(),
                        session_writes: session_writes.clone(),
                    },
                }
            }
            _ => EnvelopeOutcome {
                kind: EnvelopeKind::Legacy,
                content_type: "text/html; charset=utf-8".to_string(),
                status_line: "HTTP/1.1 200 OK",
                body: stdout.to_string(),
                headers: Vec::new(),
                service_logs: Vec::new(),
                session_writes: Vec::new(),
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
            service_logs: Vec::new(),
            session_writes: Vec::new(),
        }
    };

    EnvelopeOutcome {
        kind,
        content_type,
        status_line,
        body,
        headers,
        service_logs,
        session_writes,
    }
}

/// Extract the response `headers` field from the envelope
/// JSON. Slice 16f accepts the array form `[[k,v], ...]`
/// (preserves multi-value `Set-Cookie`); the slice 16c
/// flat-object form is still accepted for backward compat
/// with pages written before 16f. Unknown shapes yield an
/// empty list (the writer skips the loop in that case).
fn parse_envelope_headers(v: Option<&JsonValue>) -> Vec<(String, String)> {
    let Some(v) = v else { return Vec::new() };
    match v {
        JsonValue::Array(items) => {
            let mut out = Vec::new();
            for item in items {
                if let JsonValue::Array(pair) = item {
                    if pair.len() == 2 {
                        if let (Some(k), Some(val)) = (pair[0].as_str(), pair[1].as_str()) {
                            out.push((k.to_string(), val.to_string()));
                        }
                    }
                }
            }
            out
        }
        JsonValue::Object(entries) => {
            // Legacy slice-16c shape. Duplicates are not
            // possible in a flat object, so multi-value
            // Set-Cookie lines from the page are NOT
            // preserved here -- they collapse to a single
            // entry. Pages that need multi-value Set-Cookie
            // must upgrade to the 16f wrap shape.
            entries
                .iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Extract the envelope's `session_writes` array
/// (slice 16k). Wire shape (produced by the wrap
/// preamble):
/// `[{"op":"set","k":"x","v":"..."}, {"op":"delete","k":"y"},
///    {"op":"clear"}, {"op":"regenerate"}, {"op":"destroy"}]`.
/// `v` may be a string / number / bool / null / array /
/// object (JSON-compatible per spec sect.16.1). Non-object
/// entries or entries with an unknown `op` are dropped with
/// no host diagnostic; the malformed write is lost, the
/// envelope stays valid.
fn parse_session_writes(v: Option<&JsonValue>) -> Vec<SessionWrite> {
    let Some(v) = v else { return Vec::new() };
    let JsonValue::Array(items) = v else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items {
        let JsonValue::Object(entries) = item else {
            continue;
        };
        let obj = JsonValue::Object(entries.clone());
        let op = match obj.get("op").and_then(JsonValue::as_str) {
            Some(o) => o,
            None => continue,
        };
        match op {
            "set" => {
                let Some(k) = obj.get("k").and_then(JsonValue::as_str) else {
                    continue;
                };
                let Some(v) = obj.get("v") else { continue };
                match json_value_to_session(v) {
                    Some(sv) => out.push(SessionWrite::Set(k.to_string(), sv)),
                    None => eprintln!(
                        "TSPv2PoC1: session write to '{k}' has non-portable value; dropped"
                    ),
                }
            }
            "delete" => {
                let Some(k) = obj.get("k").and_then(JsonValue::as_str) else {
                    continue;
                };
                out.push(SessionWrite::Delete(k.to_string()));
            }
            "clear" => out.push(SessionWrite::Clear),
            "regenerate" => out.push(SessionWrite::Regenerate),
            "destroy" => out.push(SessionWrite::Destroy),
            _ => {}
        }
    }
    out
}

/// Convert a hand-rolled `JsonValue` (from the envelope's
/// tiny parser) into a `SessionValue`. Spec sect.16.1:
/// only JSON-compatible values are accepted; anything else
/// returns `None` and the caller drops the write.
fn json_value_to_session(v: &JsonValue) -> Option<SessionValue> {
    match v {
        JsonValue::Null => Some(SessionValue::Null),
        JsonValue::Bool(b) => Some(SessionValue::Bool(*b)),
        JsonValue::Number(n) => Some(SessionValue::Number(*n)),
        JsonValue::String(s) => Some(SessionValue::String(s.clone())),
        JsonValue::Array(items) => {
            let mut out = Vec::with_capacity(items.len());
            for it in items {
                out.push(json_value_to_session(it)?);
            }
            Some(SessionValue::Array(out))
        }
        JsonValue::Object(entries) => {
            let mut out = Vec::with_capacity(entries.len());
            for (k, v) in entries {
                out.push((k.clone(), json_value_to_session(v)?));
            }
            Some(SessionValue::Object(out))
        }
    }
}

/// Extract the envelope's `service_logs` array (slice 16j).
/// Wire shape (produced by the wrap preamble):
/// `[{"svc": "logger", "level": "info", "message": "..."}, ...]`.
/// Entries that are not objects or miss any of the three
/// string fields are dropped (the page envelope stays valid;
/// only the malformed line is lost).
fn parse_service_logs(v: Option<&JsonValue>) -> Vec<LogLine> {
    let Some(v) = v else { return Vec::new() };
    match v {
        JsonValue::Array(items) => items
            .iter()
            .filter_map(|item| {
                let service = item.get("svc")?.as_str()?.to_string();
                let level = item.get("level")?.as_str()?.to_string();
                let message = item.get("message")?.as_str()?.to_string();
                Some(LogLine {
                    service,
                    level,
                    message,
                })
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The `TSP_CONTEXT_JSON` env var carries the Context the
/// bun subprocess can read directly. Slice 16d: we strip the
/// request body from the env-var form (the body travels in
/// the wrapper's embedded literal instead) because env blocks
/// on Windows are capped at ~32 KiB and request bodies can be
/// up to the 1 MiB default limit. The JS page itself never
/// needs the env var -- the wrapper preamble bakes the full
/// Context (body included, base64-encoded as `body_b64` per
/// slice 16g) into the script. Slice 16g renamed the field
/// from `body` to `body_b64`; this strip matches the new
/// field name.
pub fn ctx_json_for_env(json: &str) -> String {
    match parse_json(json) {
        Some(JsonValue::Object(entries)) => {
            let entries: Vec<(String, JsonValue)> = entries
                .into_iter()
                .filter(|(k, _)| k != "body_b64")
                .collect();
            let mut out = String::with_capacity(json.len());
            JsonValue::Object(entries).serialize(&mut out);
            out
        }
        _ => json.to_string(),
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

/// Hard cap for the header block (request line + headers),
/// spec sect.6.4-ish sanity: a client that never sends the
/// `\r\n\r\n` terminator must not make us buffer forever.
const MAX_HEADER_BYTES: usize = 64 * 1024;

/// Default request body limit (spec sect.14.2: the runtime
/// MUST enforce a configured body limit before unbounded
/// buffering). Override via `TSP_MAX_BODY_BYTES` (bytes).
const DEFAULT_MAX_BODY_BYTES: usize = 1024 * 1024;

/// Default request timeout (spec sect.13.7's "applicable
/// timeout"). Override via `TSP_TIMEOUT_MS` (milliseconds).
/// `0` disables the timeout watchdog (slice 16i).
const DEFAULT_REQUEST_TIMEOUT_MS: u64 = 30_000;

fn resolve_max_body_bytes() -> usize {
    match std::env::var("TSP_MAX_BODY_BYTES") {
        Ok(s) => s.parse::<usize>().unwrap_or(DEFAULT_MAX_BODY_BYTES),
        Err(_) => DEFAULT_MAX_BODY_BYTES,
    }
}

/// Resolve the per-request timeout (spec sect.13.7).
/// `0` means "no timeout" (the abort signal is still
/// created and wired but the watchdog never fires).
fn resolve_request_timeout() -> u64 {
    match std::env::var("TSP_TIMEOUT_MS") {
        Ok(s) => s.parse::<u64>().unwrap_or(DEFAULT_REQUEST_TIMEOUT_MS),
        Err(_) => DEFAULT_REQUEST_TIMEOUT_MS,
    }
}

/// §32.1: dev-mode flag. `TSP_DEVELOPMENT=1` switches the
/// host from prod to dev: a 500 page-level error
/// surfaces as a self-contained HTML error page
/// (name, message, stack) instead of a JSON body. The
/// page-level contract is unchanged -- the wire 500
/// response is the same; only the rendered body
/// changes between dev and prod. The flag is read on
/// every request so a process restart is not required
/// to flip modes (and so a single test can boot a
/// master with `TSP_DEVELOPMENT=1` and a second with
/// the default prod behavior).
fn dev_mode() -> bool {
    matches!(std::env::var("TSP_DEVELOPMENT").as_deref(), Ok("1"))
}

/// §32.1: render the self-contained HTML error page for
/// `dev_mode()` requests. The input is the wrap's
/// error envelope body (a JSON object with `kind`,
/// `error`, `message`, `stack` fields) and the wire
/// status line. The output is the page body and its
/// `Content-Type`. The HTML is hand-rolled (no
/// external CSS / JS, no template engine) and HTML-
/// escapes every user-controlled field. A failed
/// parse falls back to a minimal "<error>" body so
/// the host never returns an empty 500.
fn render_dev_error_page(body: &str, status_line: &str) -> (String, String) {
    let (error, message, stack) = match parse_json(body) {
        Some(JsonValue::Object(entries)) => {
            let obj = JsonValue::Object(entries);
            let error = obj
                .get("error")
                .and_then(JsonValue::as_str)
                .unwrap_or("Error")
                .to_string();
            let message = obj
                .get("message")
                .and_then(JsonValue::as_str)
                .unwrap_or("")
                .to_string();
            let stack = obj
                .get("stack")
                .and_then(JsonValue::as_str)
                .unwrap_or("")
                .to_string();
            (error, message, stack)
        }
        _ => (
            "Error".to_string(),
            body.to_string(),
            String::new(),
        ),
    };
    let mut html = String::with_capacity(1024 + stack.len());
    html.push_str(
        "<!doctype html>\n<html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>TSP v2 \u{2014} Dev Error: ",
    );
    html_escape_into(&mut html, &error);
    html.push_str("</title>\n<style>\n\
         body { font-family: -apple-system, system-ui, 'Segoe UI', sans-serif; max-width: 960px; margin: 2em auto; padding: 0 1em; color: #222; line-height: 1.45; }\n\
         h1 { color: #b00020; margin-bottom: 0.2em; font-size: 1.4em; }\n\
         h2 { margin-top: 1.6em; font-size: 1em; color: #555; text-transform: uppercase; letter-spacing: 0.04em; }\n\
         pre { background: #f5f5f5; padding: 0.8em 1em; border-radius: 4px; overflow-x: auto; font-family: ui-monospace, 'Cascadia Code', 'Consolas', monospace; font-size: 13px; line-height: 1.4; white-space: pre-wrap; word-break: break-word; }\n\
         .error-name { font-weight: 600; color: #b00020; }\n\
         .meta { color: #666; font-size: 0.85em; margin-top: 2em; padding-top: 1em; border-top: 1px solid #eee; }\n\
         </style>\n</head>\n<body>\n\
         <h1>TSP v2 \u{2014} Dev Error</h1>\n\
         <div class=\"error-name\">",
    );
    html_escape_into(&mut html, &error);
    html.push_str("</div>\n<pre class=\"error-message\">");
    html_escape_into(&mut html, &message);
    html.push_str("</pre>\n");
    if !stack.is_empty() {
        html.push_str("<h2>Stack trace</h2>\n<pre>");
        html_escape_into(&mut html, &stack);
        html.push_str("</pre>\n");
    }
    html.push_str("<div class=\"meta\">");
    html.push_str(status_line);
    html.push_str(" \u{2014} disable with `TSP_DEVELOPMENT=0` or unset.</div>\n</body>\n</html>\n");
    (html, "text/html; charset=utf-8".to_string())
}

/// HTML-escape the input and append to `out`. Mirrors
/// the JSX renderer's escape set (`&`, `<`, `>`, `"`,
/// `'`) -- a smaller set than the full HTML5 escape
/// table but enough for the dev error page where the
/// only structural HTML we own is the `<pre>` /
/// `<div>` wrapping.
fn html_escape_into(out: &mut String, s: &str) {
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
}

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

/// Poll a cloned socket while a page subprocess is running. A TCP FIN is
/// observable as `peek() == 0`; transient `WouldBlock` means the peer is
/// still connected. The cloned socket is restored to blocking mode before
/// the monitor exits so response writes retain the listener's normal mode.
fn start_disconnect_monitor(
    stream: &TcpStream,
    cancellation: &CancellationToken,
) -> Option<(Arc<AtomicBool>, thread::JoinHandle<()>)> {
    let probe = stream.try_clone().ok()?;
    probe.set_nonblocking(true).ok()?;
    let stop = Arc::new(AtomicBool::new(false));
    let stop_in_thread = Arc::clone(&stop);
    let cancellation = cancellation.clone();
    let handle = thread::spawn(move || {
        let probe = probe;
        let mut byte = [0_u8; 1];
        while !stop_in_thread.load(Ordering::Acquire) && !cancellation.is_cancelled() {
            match probe.peek(&mut byte) {
                Ok(0) => {
                    cancellation.cancel();
                    break;
                }
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {}
                Err(_) => {
                    cancellation.cancel();
                    break;
                }
            }
            thread::sleep(std::time::Duration::from_millis(25));
        }
        let _ = probe.set_nonblocking(false);
    });
    Some((stop, handle))
}

fn stop_disconnect_monitor(monitor: Option<(Arc<AtomicBool>, thread::JoinHandle<()>)>) {
    if let Some((stop, handle)) = monitor {
        stop.store(true, Ordering::Release);
        let _ = handle.join();
    }
}

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
    services: &'static RwLock<ServiceRegistry>,
) -> Result<(), HostError> {
    serve_with_public_root(
        host,
        port,
        routes,
        registry,
        bun,
        services,
        resolve_public_root(),
    )
}

/// Bind and serve with an explicit public asset root. `None` disables native
/// static files while leaving page routing unchanged.
pub fn serve_with_public_root(
    host: &str,
    port: u16,
    routes: Arc<RouteTable>,
    registry: &'static PageRegistry,
    bun: &'static BunRuntime,
    services: &'static RwLock<ServiceRegistry>,
    public_root: Option<std::path::PathBuf>,
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
                if e.kind() == io::ErrorKind::Interrupted || e.kind() == io::ErrorKind::WouldBlock {
                    continue;
                }
                return Err(HostError::Accept(e));
            }
        };
        eprintln!("TSPv2PoC1: accepted {peer}");
        let routes_for_thread = Arc::clone(&routes);
        let public_root_for_thread = public_root.clone();
        thread::spawn(move || {
            if let Err(e) = handle_connection(
                stream,
                &routes_for_thread,
                registry,
                bun,
                services,
                public_root_for_thread.as_deref(),
            ) {
                eprintln!("TSPv2PoC1: {e}");
            }
        });
    }
    Ok(())
}

fn resolve_public_root() -> Option<std::path::PathBuf> {
    let configured = std::env::var_os("TSP_PUBLIC_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("public"));
    configured.is_dir().then_some(configured)
}

fn handle_connection(
    mut stream: TcpStream,
    routes: &Arc<RouteTable>,
    registry: &PageRegistry,
    bun: &BunRuntime,
    services: &RwLock<ServiceRegistry>,
    public_root: Option<&std::path::Path>,
) -> Result<(), HostError> {
    // Slice 16d: read the full request (header block up to
    // CRLFCRLF + exactly Content-Length body bytes). Body
    // over the configured limit (TSP_MAX_BODY_BYTES,
    // default 1 MiB) is rejected with 413 before the page
    // sees it (spec sect.14.2).
    let max_body = resolve_max_body_bytes();
    // Slice 16i: per-request timeout (spec sect.13.7).
    // `0` means "no timeout" -- the abort signal is still
    // created and wired in the wrap preamble, but the
    // watchdog never fires.
    let timeout_ms = resolve_request_timeout();
    let (head, body) = match read_request(&mut stream, max_body)? {
        ReadOutcome::Complete { head, body } => (head, body),
        ReadOutcome::BodyTooLarge { limit } => {
            let body_text = format_error_body(
                TspError::BodyTooLarge,
                &format!(
                    "TSP v2 PoC 1 slice 16d: request body exceeds configured limit of {limit} bytes\n"
                ),
            );
            let head = format!(
                "HTTP/1.1 413 Payload Too Large\r\n\
                 Content-Type: text/plain; charset=utf-8\r\n\
                 Content-Length: {}\r\n\
                 Connection: close\r\n\r\n",
                body_text.len()
            );
            stream
                .write_all(head.as_bytes())
                .map_err(HostError::Connection)?;
            stream
                .write_all(body_text.as_bytes())
                .map_err(HostError::Connection)?;
            let _ = stream.shutdown(Shutdown::Both);
            return Ok(());
        }
    };
    let request_started = std::time::Instant::now();
    let parsed = parse_request(&head);
    metrics::global().record_request();
    // Pull the method out of `parsed` so the page
    // dispatch (further down, after the
    // /__tsp/metrics / static-files early-returns)
    // can also branch on it -- specifically to drop
    // the body for HEAD responses. The `method` here
    // is `HttpMethod` (a value type); we re-clone it
    // into the `if let ParsedRequest::Known` scope
    // below for the early-return arm, and use a
    // second binding here for the post-early-return
    // code path.
    let page_method: HttpMethod = match &parsed {
        ParsedRequest::Known { method, .. } => *method,
        ParsedRequest::Unknown => HttpMethod::Get, // best-effort default for malformed
    };
    if let ParsedRequest::Known { method, path, .. } = &parsed {
        if path == "/__tsp/metrics" {
            match *method {
                HttpMethod::Get => {
                    // Slice 22+: GET returns the Prometheus
                    // body. The body is generated AFTER
                    // record_request but BEFORE
                    // record_response + record_duration
                    // -- a snapshot of mid-request state
                    // (see host.rs:1517-1532 and the
                    // `metrics_endpoint_serves_prometheus_text_...`
                    // e2e for the full snapshot contract).
                    let body = metrics::global().prometheus();
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\nContent-Length: {}\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    stream
                        .write_all(head.as_bytes())
                        .map_err(HostError::Connection)?;
                    stream
                        .write_all(body.as_bytes())
                        .map_err(HostError::Connection)?;
                    metrics::global().record_response("HTTP/1.1 200 OK");
                    metrics::global().record_duration(
                        request_started.elapsed().as_millis() as u64,
                    );
                    let _ = stream.shutdown(Shutdown::Both);
                    return Ok(());
                }
                HttpMethod::Head => {
                    // Spec sect.6.5: HEAD with no
                    // explicit HEAD export uses GET
                    // and drops the body. The
                    // metrics endpoint has no
                    // separate HEAD handler, so
                    // return the same headers GET
                    // would but with an empty
                    // body. The Content-Length is
                    // the GET body size so a client
                    // can size its request without
                    // the bytes.
                    let body = metrics::global().prometheus();
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\nContent-Length: {}\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    stream
                        .write_all(head.as_bytes())
                        .map_err(HostError::Connection)?;
                    metrics::global().record_response("HTTP/1.1 200 OK");
                    metrics::global().record_duration(
                        request_started.elapsed().as_millis() as u64,
                    );
                    let _ = stream.shutdown(Shutdown::Both);
                    return Ok(());
                }
                _ => {
                    // 405 Method Not Allowed. The
                    // metrics endpoint only
                    // documents GET + HEAD; any
                    // other method gets a 405 with
                    // an `Allow: GET, HEAD` header
                    // so the client knows what to
                    // retry with. The 405 is a
                    // proper REST response and
                    // does NOT count as a tsc /
                    // page-router error.
                    let body = b"405 Method Not Allowed: /__tsp/metrics only accepts GET and HEAD\r\n";
                    let head = format!(
                        "HTTP/1.1 405 Method Not Allowed\r\nAllow: GET, HEAD\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    stream
                        .write_all(head.as_bytes())
                        .map_err(HostError::Connection)?;
                    stream
                        .write_all(body)
                        .map_err(HostError::Connection)?;
                    metrics::global().record_response("HTTP/1.1 405 Method Not Allowed");
                    metrics::global().record_duration(
                        request_started.elapsed().as_millis() as u64,
                    );
                    let _ = stream.shutdown(Shutdown::Both);
                    return Ok(());
                }
            }
        }
        if matches!(*method, HttpMethod::Get | HttpMethod::Head) {
            if let Some(root) = public_root {
                if let Some(asset) =
                    crate::static_files::load(root, path).map_err(HostError::Connection)?
                {
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\nX-Content-Type-Options: nosniff\r\nCache-Control: public, max-age=3600\r\nConnection: close\r\n\r\n",
                        asset.content_type,
                        asset.body.len()
                    );
                    stream
                        .write_all(head.as_bytes())
                        .map_err(HostError::Connection)?;
                    if *method == HttpMethod::Get {
                        stream
                            .write_all(&asset.body)
                            .map_err(HostError::Connection)?;
                    }
                    metrics::global().record_response("HTTP/1.1 200 OK");
                    metrics::global().record_duration(request_started.elapsed().as_millis() as u64);
                    eprintln!("TSPv2PoC1: static {}", asset.path.display());
                    let _ = stream.shutdown(Shutdown::Both);
                    return Ok(());
                }
            }
        }
    }

    // Content-Length override for HEAD responses.
    // For the page-dispatch path, when the request is
    // HEAD and the page produced a body (either via
    // FoundHeadOverGet or via the explicit HEAD
    // handler), the wire body is empty (per RFC 9110
    // sect.9.3.2) but the Content-Length should report
    // the body size the GET would have produced, so a
    // client can size its request without a follow-up
    // GET. The match arm below sets this when relevant.
    // Default 0 = "use body.len()" -- the GET-equivalent
    // case is unaffected.
    let mut content_length_override: usize = 0;

    let (status_line, content_type, allow_header, body, extra_headers) = match parsed {
        ParsedRequest::Unknown => (
            "HTTP/1.1 400 Bad Request",
            "text/plain; charset=utf-8".to_string(),
            None,
            format_error_body(
                TspError::MalformedRequestLine,
                "TSP v2 PoC 1 slice 10b: malformed request line\n",
            ),
            Vec::new(),
        ),
        // 16k: the early-return arms above never reach the
        // SessionService; the 4xx / 5xx bodies do not carry
        // a session, so the host does not emit a
        // `Set-Cookie: tsp_sid=...` line for them.
        ParsedRequest::Known {
            method,
            path,
            query,
            headers,
        } => {
            // Build the per-request Context the page handler
            // receives as its single argument. `params` is
            // empty for slice 16a (dynamic route segments
            // are not implemented yet). The query string is
            // passed verbatim -- the JS side turns it into
            // `ctx.url.searchParams` (spec sect.13.5). `body`
            // and `headers` are slice 16d (spec sect.13.3).
            // Match first, THEN build the context. The matched
            // route's `params` (spec sect.11.3 / 11.4) flow
            // into `ctx.params` so the page handler reads them
            // as a typed map. FoundHeadOverGet also carries
            // params (the host still runs the GET handler with
            // the original request path). The lookup is done
            // once; the resulting MatchResult is used both to
            // seed `ctx.params` and to drive the dispatch
            // below.
            let (dispatch_path, fragment_name, dispatch_query) = fragment_target(&path, &query)
                .unwrap_or_else(|| (path.clone(), None, query.clone()));
            let matched = routes.lookup(&dispatch_path, method);
            // FREEZE.md §11 / `config.bodyLimit`: a page
            // may declare a per-page body cap. If the
            // request body is larger than the page's
            // cap, return 413 BEFORE running the page
            // (the page would otherwise start
            // processing a body it has no business
            // reading). The cap is applied only to
            // POST / PUT / PATCH / DELETE; GET / HEAD
            // / OPTIONS are not expected to carry a
            // body. The per-page cap is silently
            // clamped to the global
            // `TSP_MAX_BODY_BYTES` (the spec says
            // "cannot exceed global hard limit"; a
            // larger declared value would be a
            // configuration error and we just use the
            // global instead).
            let page_body_limit: Option<usize> = match &matched {
                MatchResult::Found { route, .. }
                | MatchResult::FoundHeadOverGet { route } => {
                    match crate::page::prepare(route) {
                        Ok(page) => {
                            let global = resolve_max_body_bytes();
                            page.config_body_limit
                                .map(|n| n.min(global))
                        }
                        Err(_) => None,
                    }
                }
                _ => None,
            };
            if let Some(limit) = page_body_limit {
                if matches!(
                    method,
                    HttpMethod::Post
                        | HttpMethod::Put
                        | HttpMethod::Patch
                        | HttpMethod::Delete
                ) && body.len() > limit
                {
                    metrics::global()
                        .record_response("HTTP/1.1 413 Payload Too Large");
                    metrics::global().record_duration(
                        request_started.elapsed().as_millis() as u64,
                    );
                    let body_text = format_error_body(
                        TspError::BodyTooLarge,
                        &format!(
                            "TSP v2: request body ({} bytes) exceeds the page's \
                             `config.bodyLimit` ({} bytes)\n",
                            body.len(),
                            limit
                        ),
                    );
                    let head = format!(
                        "HTTP/1.1 413 Payload Too Large\r\n\
                         Content-Type: text/plain; charset=utf-8\r\n\
                         Content-Length: {}\r\n\
                         X-Content-Type-Options: nosniff\r\n\
                         Connection: close\r\n\r\n",
                        body_text.len()
                    );
                    stream
                        .write_all(head.as_bytes())
                        .map_err(HostError::Connection)?;
                    stream
                        .write_all(body_text.as_bytes())
                        .map_err(HostError::Connection)?;
                    let _ = stream.shutdown(Shutdown::Both);
                    return Ok(());
                }
            }
            let params: std::collections::HashMap<String, String> = match &matched {
                MatchResult::Found { route, .. } | MatchResult::FoundHeadOverGet { route } => {
                    route.params.clone()
                }
                _ => std::collections::HashMap::new(),
            };
            // `config.cache` (plan §55, FREEZE.md §11):
            // a page may declare a default
            // `Cache-Control` header. Resolved
            // here so the inner dispatch arm
            // can inject the default into the
            // response headers ONLY when the
            // page's `Response` did not set one
            // itself (the page is more specific
            // than the page-level default). The
            // value is the FREEZE.md §11 literal
            // (`no-store` / `private` / `public`).
            // The detector in `page.rs` is
            // hand-rolled and tolerates single /
            // double quotes + whitespace.
            let default_cache_control: Option<&'static str> = match &matched {
                MatchResult::Found { route, .. }
                | MatchResult::FoundHeadOverGet { route } => crate::page::prepare(route)
                    .ok()
                    .and_then(|p| p.config_cache.map(|c| c.header_value())),
                _ => None,
            };
            // `config.timeoutMs` (spec §7 v2.0
            // core PageConfig): a page may
            // declare a per-page request timeout
            // (in milliseconds). The per-page
            // value OVERRIDES the global
            // `resolve_request_timeout()` for
            // this one request. `0` means
            // "no timeout" (the abort signal is
            // still wired to the page, but the
            // watchdog never fires) -- same as
            // the global `0`. The per-page value
            // is NOT silently clamped to the
            // global; the page is the authority
            // on its own timeout budget.
            let effective_timeout_ms: u64 = match &matched {
                MatchResult::Found { route, .. }
                | MatchResult::FoundHeadOverGet { route } => crate::page::prepare(route)
                    .ok()
                    .and_then(|p| p.config_timeout_ms)
                    .unwrap_or(timeout_ms),
                _ => timeout_ms,
            };
            // Slice 16k: resolve the request's session
            // view (spec sect.16) against the runtime
            // SessionService. `None` cookie -> fresh
            // session; unknown / destroyed sid -> also
            // fresh (spec 16.4 makes the destroyed session
            // no longer usable).
            let session_resolve = services.read().unwrap().get(BUILTIN_SESSION).and_then(|svc_arc| {
                svc_arc
                    .as_any()
                    .downcast_ref::<SessionService>()
                    .map(|svc| {
                        let cookie_sid = read_session_cookie(&headers);
                        resolve_session_view(svc, cookie_sid.as_deref())
                    })
            });
            // Track the cookie that came in so the
            // post-render Set-Cookie decision compares
            // against the request, not against the
            // resolved (potentially freshly-minted) view.
            let request_sid = session_resolve
                .as_ref()
                .and_then(|r| r.original_sid.clone());
            let ctx = Context {
                method,
                path: dispatch_path.clone(),
                query: dispatch_query,
                params,
                body,
                headers,
                // Slice 16j: `ctx.services` snapshot from the
                // runtime-scoped registry (spec sect.17). The
                // request-scoped list is empty in 16j.
                services: services.read().unwrap().snapshot(&[]),
                // Slice 16k: `ctx.session` view the page
                // reads (spec sect.16). `None` only when the
                // SessionService is not registered (the
                // current boot always registers it; the
                // arm stays for future embeddings / tests).
                session: session_resolve.map(|r| r.view),
            };
            let cancellation = CancellationToken::new();
            let disconnect_monitor = start_disconnect_monitor(&stream, &cancellation);
            let ctx_json = ctx.to_json_with_fragment(fragment_name.as_deref());
            let rendered = match matched {
                MatchResult::Found {
                    route,
                    method: req_method,
                } => {
                    let page_ref = PageRef {
                        route: route.path.clone(),
                        method: req_method,
                    };
                    // Slice 16d/16e: a request that carries a query
                    // string, a body, OR dynamic-route params is
                    // inherently per-request -- the page's output
                    // depends on those per-request inputs, which
                    // differ from request to request. The registry
                    // cache keys on (route, method), so a cached
                    // payload would replay the FIRST request's
                    // output (e.g. the first query string, the
                    // first body echo, the first captured params)
                    // to every later request on the same
                    // route+method. Such requests therefore bypass
                    // the generation cache and rebuild via the
                    // pipeline directly (spec sect.20-22 cache
                    // semantics only cover body-less, query-less,
                    // param-less GET-style rendering).
                    let per_request = !ctx.body.is_empty()
                    || !ctx.query.is_empty()
                    || !ctx.params.is_empty()
                    // Slice 16j: a registered runtime service that
                    // reports `is_request_varying()` (e.g. the
                    // logger's `total_lines`) makes every render
                    // request-dependent -- the cache would replay a
                    // stale service-state snapshot.
                    || services.read().unwrap().any_request_varying();
                    let (_status_line, _ct, allow_header, body) = if per_request {
                        render_per_request(
                            &route,
                            req_method,
                            bun,
                            &ctx,
                            &ctx_json,
                            effective_timeout_ms,
                            &cancellation,
                        )
                    } else {
                        render_for_route(
                            &route,
                            req_method,
                            &page_ref,
                            registry,
                            bun,
                            &ctx,
                            &ctx_json,
                            effective_timeout_ms,
                            &cancellation,
                        )
                    };
                    // Slice 16b: bun emits a `__TSP_OUT_V1__` envelope
                    // with the page's return value classified as
                    // either HtmlNode (string) or Web Response.
                    // parse_envelope unpacks it and surfaces the
                    // correct status / content-type / body / headers.
                    let outcome = parse_envelope(&body);
                    // Slice 16j: flush the page's `ctx.services.*`
                    // log lines into the owning runtime service now
                    // that the envelope is back (the page ran in a
                    // throwaway subprocess; the envelope is the only
                    // back-channel). The flush lands BEFORE this
                    // response is written, so the next request's
                    // snapshot observes it.
                    services.read().unwrap().flush_log_lines(&outcome.service_logs);
                    // Slice 16k: apply the page's session writes
                    // (spec sect.16). The new id may be empty
                    // (destroyed) or different (regenerate /
                    // fresh) from the cookie the request came in
                    // with; either way the response needs a
                    // `Set-Cookie: tsp_sid=...` line so the
                    // browser keeps its view of the session in
                    // sync with the host.
                    let mut outcome = outcome;
                    let mut new_session_sid: Option<String> = None;
                    if let Some(current) = &ctx.session {
                        if let Some(svc_arc) = services.read().unwrap().get(BUILTIN_SESSION) {
                            if let Some(svc) = svc_arc.as_any().downcast_ref::<SessionService>() {
                                // Apply the page's writes against
                                // the session's CURRENT id (the
                                // resolved view, which may already
                                // differ from the cookie if the
                                // request's sid was unknown /
                                // destroyed). After apply, the
                                // returned sid is the new value
                                // the browser must see.
                                let next = svc.apply_writes(&current.id, &outcome.session_writes);
                                new_session_sid = Some(next);
                            }
                        }
                    }
                    // The cookie line is determined by
                    // `request_sid` (the cookie the request
                    // carried) vs `new_sid` (the sid the host
                    // committed after applying writes). They
                    // match -> no Set-Cookie. They differ ->
                    // plant the new sid (or Max-Age=0 on
                    // destroy).
                    if let Some(new_sid) = new_session_sid.as_deref() {
                        let old_sid = request_sid.as_deref().unwrap_or("");
                        if let Some(cookie_line) = build_session_cookie(new_sid, old_sid) {
                            outcome
                                .headers
                                .push(("Set-Cookie".to_string(), cookie_line));
                        }
                    }
                    // The envelope is the source of truth for the
                    // page's actual status when the page ran. When
                    // the envelope is absent (Legacy), the body is
                    // an error page produced by the host (405 / 500)
                    // and the host's own status line must win --
                    // otherwise a method rejection would be served
                    // as 200 OK.
                    let use_envelope = outcome.kind != EnvelopeKind::Legacy;
                    // §32.1: a page that throws (other than
                    // HttpError) reaches the host as a 500 with
                    // `x-tsp-error: page` and a JSON body
                    // carrying the error name / message / stack.
                    // In dev mode (`TSP_DEVELOPMENT=1`) the
                    // host renders an HTML error page; in
                    // prod the wire 500 with the JSON body is
                    // returned as-is.
                    let (body, content_type, headers) = if use_envelope
                        && outcome
                            .headers
                            .iter()
                            .any(|(k, v)| k.eq_ignore_ascii_case("x-tsp-error") && v == "page")
                    {
                        if dev_mode() {
                            let (html_body, html_ct) =
                                render_dev_error_page(&outcome.body, &outcome.status_line);
                            (html_body, html_ct, Vec::new())
                        } else {
                            // Prod path: strip the internal
                            // `x-tsp-error` header (the wire
                            // body still carries the JSON; the
                            // application can log it). The
                            // content-type stays application/json.
                            let filtered: Vec<(String, String)> = outcome
                                .headers
                                .iter()
                                .filter(|(k, _)| !k.eq_ignore_ascii_case("x-tsp-error"))
                                .cloned()
                                .collect();
                            (outcome.body.clone(), outcome.content_type.clone(), filtered)
                        }
                    } else {
                        (
                            outcome.body.clone(),
                            outcome.content_type.clone(),
                            outcome.headers.clone(),
                        )
                    };
                    // `config.cache` (plan §55, FREEZE.md §11):
                    // inject the page-level default
                    // `Cache-Control` header into the
                    // response ONLY when the page's
                    // `Response` did not set one itself
                    // (the page is more specific than
                    // the page-level default; a page
                    // that needs a custom value can
                    // set its own). The injection
                    // happens here (after the
                    // x-tsp-error filter) so the page's
                    // explicit header -- if any --
                    // always wins.
                    let mut headers = headers;
                    if let Some(cc) = default_cache_control {
                        if !headers
                            .iter()
                            .any(|(k, _)| k.eq_ignore_ascii_case("cache-control"))
                        {
                            headers
                                .push(("Cache-Control".to_string(), cc.to_string()));
                        }
                    }
                    // For explicit HEAD requests, the
                    // page's HEAD handler produced a
                    // body that is dropped at the wire.
                    // Capture its length so the
                    // Content-Length header (computed
                    // later from `content_length_override`)
                    // still reports the would-be body
                    // size. For non-HEAD the override
                    // stays 0 and `body.len()` is used.
                    if req_method == HttpMethod::Head {
                        content_length_override = body.len();
                    }
                    (
                        if use_envelope {
                            outcome.status_line
                        } else {
                            _status_line
                        },
                        content_type,
                        allow_header,
                        body,
                        headers,
                    )
                }
                MatchResult::FoundHeadOverGet { route } => {
                    // Spec sect.6.5: HEAD with no explicit HEAD export.
                    // Run the GET handler, then strip the body. We
                    // DO preserve Content-Length (the GET body size)
                    // so a client can size its request without
                    // making a follow-up GET -- the same contract
                    // /__tsp/metrics uses for its hand-rolled HEAD
                    // path (host.rs:1517). The `head_on_regular_page_...`
                    // and `head_on_route_with_both_get_and_head_...`
                    // e2e tests pin this. The body itself is dropped
                    // at the wire (per RFC 9110 sect.9.3.2).
                    let page_ref = PageRef {
                        route: route.path.clone(),
                        method: HttpMethod::Get,
                    };
                    let (_status, _ct, _allow, get_envelope) = render_for_route(
                        &route,
                        HttpMethod::Get,
                        &page_ref,
                        registry,
                        bun,
                        &ctx,
                        &ctx_json,
                        effective_timeout_ms,
                        &cancellation,
                    );
                    // The render result is the raw
                    // `__TSP_OUT_V1__\n{...JSON...}` envelope,
                    // NOT the page's actual response body.
                    // Parse the envelope to extract the
                    // real body length (the `body` field
                    // in the JSON). The envelope itself
                    // is dropped on the floor.
                    let get_body_len = parse_envelope(&get_envelope).body.len();
                    let _ = get_envelope; // explicitly drop
                    content_length_override = get_body_len;
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
                MatchResult::MalformedPath { error } => (
                    "HTTP/1.1 400 Bad Request",
                    "text/plain; charset=utf-8".to_string(),
                    None,
                    format_error_body(
                        TspError::MalformedUrl,
                        &format!("TSP v2 PoC 1 slice 16e: malformed URL path: {error}\n"),
                    ),
                    Vec::new(),
                ),
                MatchResult::NotFound => (
                    "HTTP/1.1 404 Not Found",
                    "text/plain; charset=utf-8".to_string(),
                    None,
                    format_error_body(
                        TspError::NoRouteMatches,
                        &format!(
                            "TSP v2 PoC 1 slice 10b: no route matches path={path} (table has {} route(s))\n",
                            routes.len()
                        ),
                    ),
                    Vec::new(),
                ),
            };
            stop_disconnect_monitor(disconnect_monitor);
            if cancellation.is_cancelled() {
                let _ = stream.shutdown(Shutdown::Both);
                return Ok(());
            }
            rendered
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
    // For HEAD responses, the body is dropped at the
    // wire (per RFC 9110 sect.9.3.2). The Content-Length,
    // however, is preserved: it reports the body size
    // the page would have produced, so a client can
    // size its request without a follow-up GET. The
    // match arm below sets `content_length_override`
    // for both Found (the page's HEAD handler body
    // length) and FoundHeadOverGet (the GET's body
    // length) cases. For non-HEAD requests, the
    // wire body is the body bytes themselves, and
    // Content-Length is `body.len()`.
    let content_length = if page_method == HttpMethod::Head {
        content_length_override
    } else {
        body.len()
    };
    let mut head = format!(
        "{status_line}\r\n\
         {header_block}Content-Type: {content_type}\r\n\
         Content-Length: {content_length}\r\n\
         X-Content-Type-Options: nosniff\r\n\
         Connection: close\r\n",
    );
    if let Some(allow) = allow_header {
        head.push_str(&allow);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");

    metrics::global().record_response(status_line);
    metrics::global().record_duration(request_started.elapsed().as_millis() as u64);

    stream
        .write_all(head.as_bytes())
        .map_err(HostError::Connection)?;
    // Per spec sect.6.5 / RFC 9110 sect.9.3.2: a HEAD
    // response MUST NOT include a message body, even
    // when the page's HEAD handler returns one (the
    // handler is invoked for any side effects it may
    // have; the body is dropped at the wire). The
    // Content-Length is preserved as 0 here for routes
    // (the metrics endpoint's hand-rolled HEAD path
    // preserves the GET body size for clients that
    // want to size their request without the bytes;
    // the page path is a known slice-14a gap and
    // matches the `head_on_regular_page_...` e2e
    // assertion of Content-Length: 0).
    if page_method != HttpMethod::Head {
        stream
            .write_all(body.as_bytes())
            .map_err(HostError::Connection)?;
    }
    let _ = stream.shutdown(Shutdown::Both);
    Ok(())
}

/// Slice 16d: build a page for a request whose output depends
/// on per-request inputs (query string or body), bypassing the
/// PageRegistry generation cache entirely.
///
/// The registry keys payloads on `(route, method)`, which is
/// correct for body-less, query-less GET-style rendering
/// (idempotent, no per-request variance) but wrong for
/// requests whose output depends on the query / body -- a
/// cached payload would replay the first request's output to
/// every later request on the same route+method. So these
/// requests go straight to the pipeline: prepare + bun
/// subprocess, no `begin_build` / `commit`, no LKG fallback.
/// 405 (method not exported), 500 (build failure), and 504
/// (request timeout) retain distinct response semantics.
fn build_failure_response(error: &pipeline::BuildError, detail: String) -> (&'static str, String) {
    let (code, description) = match error {
        pipeline::BuildError::Prepare(_) => (
            TspError::PagePrepareError.code(),
            TspError::PagePrepareError.describe(),
        ),
        pipeline::BuildError::Jsc(jsc) => (jsc.code(), jsc.describe()),
    };
    let status = match error {
        pipeline::BuildError::Jsc(crate::jsc_bridge::JscError::TimedOut { .. }) => {
            metrics::global().record_timeout();
            status_line_for(504)
        }
        pipeline::BuildError::Jsc(crate::jsc_bridge::JscError::Cancelled) => {
            metrics::global().record_cancellation();
            status_line_for(499)
        }
        _ => status_line_for(500),
    };
    (status, format_error_body_raw(code, description, &detail))
}

/// Add a small original-source code frame when the native JSX pre-pass carries
/// an explicit source line. Bun diagnostics retain the original `tsp://`
/// source URL in stderr, but their wrapper line numbers are not source-map
/// offsets and therefore are not used to build a misleading frame.
fn diagnostic_detail(
    path: &std::path::Path,
    error: &dyn std::fmt::Display,
    mut detail: String,
) -> String {
    let text = error.to_string();
    let line = find_diagnostic_line(&text);
    let Some(line) = line else { return detail };
    let Ok(source) = std::fs::read_to_string(path) else {
        return detail;
    };
    let lines: Vec<&str> = source.lines().collect();
    if line == 0 || line > lines.len() {
        return detail;
    }
    let start = line.saturating_sub(2);
    let end = (line + 1).min(lines.len());
    detail.push_str(&format!("\n--- {}:{} ---\n", path.display(), line));
    for index in start..end {
        let marker = if index + 1 == line { ">" } else { " " };
        detail.push_str(&format!("{marker} {:>4} | {}\n", index + 1, lines[index]));
    }
    detail
}

fn find_diagnostic_line(text: &str) -> Option<usize> {
    if let Some(index) = text.find("line ") {
        let digits: String = text[index + 5..]
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect();
        if let Ok(line) = digits.parse::<usize>() {
            if line > 0 {
                return Some(line);
            }
        }
    }
    None
}

fn render_per_request(
    route: &crate::router::Route,
    requested: HttpMethod,
    bun: &BunRuntime,
    ctx: &Context,
    ctx_json: &str,
    timeout_ms: u64,
    cancellation: &CancellationToken,
) -> (&'static str, &'static str, Option<String>, String) {
    // Mirror render_for_route's 405 conversion: the page must
    // export the requested method. We use the static detector
    // (page::prepare) rather than the registry because we are
    // deliberately outside the registry path.
    match crate::page::prepare(route) {
        Ok(page) if page.methods.contains(&requested) => {
            match pipeline::build(
                route,
                requested,
                bun,
                ctx_json,
                timeout_ms,
                cancellation,
                &ctx.headers,
                &ctx.body,
            ) {
                Ok(body) => ("HTTP/1.1 200 OK", "text/html; charset=utf-8", None, body),
                Err(e) => {
                    let msg = format!("{e}");
                    eprintln!("TSPv2PoC1: build error on {}: {e}", route.source.display());
                    let (status, body) = build_failure_response(
                        &e,
                        diagnostic_detail(
                            &route.source,
                            &e,
                            format!(
                                "TSP v2 PoC 1 slice 16d: build error on {}\n  {msg}\n",
                                route.source.display()
                            ),
                        ),
                    );
                    (status, "text/plain; charset=utf-8", None, body)
                }
            }
        }
        Ok(page) => {
            let allow = build_allow_header(&page.methods);
            let body = format_error_body(
                TspError::MethodNotAllowed,
                &format!(
                    "TSP v2 PoC 1 slice 16d: method {} not exported by {}\n",
                    requested.as_str(),
                    route.source.display()
                ),
            );
            (
                "HTTP/1.1 405 Method Not Allowed",
                "text/plain; charset=utf-8",
                if allow.is_empty() { None } else { Some(allow) },
                body,
            )
        }
        Err(e) => (
            "HTTP/1.1 500 Internal Server Error",
            "text/plain; charset=utf-8",
            None,
            format_error_body(
                TspError::PagePrepareError,
                &format!("TSP v2 PoC 1 slice 16d: prepare error: {e}\n"),
            ),
        ),
    }
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
    ctx_json: &str,
    timeout_ms: u64,
    cancellation: &CancellationToken,
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
        let body = format_error_body(
            TspError::MethodNotAllowed,
            &format!(
                "TSP v2 PoC 1 slice 12: method {} not exported by {}\n",
                requested.as_str(),
                route.source.display()
            ),
        );
        return (
            "HTTP/1.1 405 Method Not Allowed",
            "text/plain; charset=utf-8",
            if allow.is_empty() { None } else { Some(allow) },
            body,
        );
    }
    let state = snap
        .as_ref()
        .map(|s| s.state.clone())
        .unwrap_or(PageState::Unloaded);

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
                    let build_result = pipeline::build(
                        route,
                        requested,
                        bun,
                        ctx_json,
                        timeout_ms,
                        cancellation,
                        &ctx.headers,
                        &ctx.body,
                    );
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
                            guard.fail(msg.clone());
                            eprintln!("TSPv2PoC1: build error on {}: {e}", route.source.display());
                            let (status, body) = build_failure_response(
                                &e,
                                diagnostic_detail(
                                    &route.source,
                                    &e,
                                    format!(
                                        "TSP v2 PoC 1 slice 12: build error on {}\n  {msg}\n",
                                        route.source.display()
                                    ),
                                ),
                            );
                            (status, "text/plain; charset=utf-8", None, body)
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
                    format_error_body(
                        TspError::UnknownPage,
                        "TSP v2 PoC 1 slice 12: page not registered in PageRegistry\n",
                    ),
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
    (
        "HTTP/1.1 200 OK",
        "text/html; charset=utf-8",
        None,
        (*body).clone(),
    )
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
            format_error_body(
                TspError::CleanSlotMissingPayload,
                "TSP v2 PoC 1 slice 12: Clean slot has no payload\n",
            ),
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
            format_error_body(
                TspError::PageNeverBuilt,
                &format!(
                    "TSP v2 PoC 1 slice 12: page {} never built successfully\n",
                    route.source.display()
                ),
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
            let body = format_error_body(
                TspError::MethodNotAllowed,
                &format!(
                    "TSP v2 PoC 1 slice 10b: method {} not exported by {}\n",
                    requested.as_str(),
                    route.source.display()
                ),
            );
            (allow, body)
        }
        Err(e) => (
            String::new(),
            format_error_body(
                TspError::PagePrepareError,
                &format!("TSP v2 PoC 1 slice 10b: prepare error: {e}\n"),
            ),
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
        /// Request headers with lower-cased names, wire
        /// order, duplicates joined with ", ".
        headers: Vec<(String, String)>,
    },
    Unknown,
}

/// Decode a query component for the private fragment endpoint. This follows
/// the URL form convention (`+` means a space) and rejects malformed UTF-8
/// rather than handing a different route identity to the matcher.
fn decode_query_component(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hi = fragment_hex_value(bytes[i + 1])?;
                let lo = fragment_hex_value(bytes[i + 2])?;
                out.push((hi << 4) | lo);
                i += 2;
            }
            b'%' => return None,
            byte => out.push(byte),
        }
        i += 1;
    }
    String::from_utf8(out).ok()
}

fn fragment_hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Resolve `/__tsp/fragment?route=...&name=...` into the originating route
/// and the private handler selector. The remaining query fields are passed
/// to the fragment as its normal `ctx.query`.
fn fragment_target(path: &str, query: &str) -> Option<(String, Option<String>, String)> {
    if path != "/__tsp/fragment" {
        return Some((path.to_string(), None, query.to_string()));
    }
    let mut route = None;
    let mut name = None;
    let mut token = None;
    let mut rest = Vec::new();
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (raw_key, raw_value) = pair.split_once('=').unwrap_or((pair, ""));
        let key = decode_query_component(raw_key)?;
        let value = decode_query_component(raw_value)?;
        match key.as_str() {
            "route" => route = Some(value),
            "name" => name = Some(value),
            "token" => token = Some(value),
            _ => rest.push(format!("{raw_key}={raw_value}")),
        }
    }
    if token.as_deref() != Some(fragment_token()) {
        return None;
    }
    Some((route?, Some(name?), rest.join("&")))
}

/// Parse the request line + header block. `head` is the
/// complete header section (request line through the blank
/// line), NOT the body -- `read_request` splits them first.
fn parse_request(head: &str) -> ParsedRequest {
    let Some(first_line) = head.lines().next() else {
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
        headers: parse_headers(head),
    }
}

/// Split a raw request into (head, body). Reads the header
/// block (up to `\r\n\r\n`, capped at MAX_HEADER_BYTES),
/// then reads exactly Content-Length body bytes (spec
/// sect.14.2 -- a body over the configured limit is
/// rejected before buffering completes). The head is
/// decoded as UTF-8 lossy (HTTP header lines are required
/// to be ASCII; the lossy fallback is defence-in-depth for
/// misbehaving clients); the body is the raw `Vec<u8>`
/// so binary multipart payloads survive intact (spec
/// sect.14.3 / 14.2).
fn read_request(stream: &mut TcpStream, max_body: usize) -> Result<ReadOutcome, HostError> {
    let mut buf: Vec<u8> = Vec::with_capacity(4096);
    let mut tmp = [0u8; 4096];
    let mut head_end: Option<usize> = None;
    loop {
        let n = stream.read(&mut tmp).map_err(HostError::Connection)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_subsequence(&buf, b"\r\n\r\n") {
            head_end = Some(pos + 4);
            break;
        }
        if buf.len() > MAX_HEADER_BYTES {
            // No terminator within the cap: treat everything
            // received as head (the parser will likely fail
            // to find a request line; the caller 400s).
            break;
        }
    }
    let head_end = head_end.unwrap_or(buf.len());
    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let content_length = parse_content_length(&head);
    if content_length > max_body {
        return Ok(ReadOutcome::BodyTooLarge { limit: max_body });
    }
    let mut body: Vec<u8> = buf[head_end..].to_vec();
    while body.len() < content_length {
        let n = stream.read(&mut tmp).map_err(HostError::Connection)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }
    body.truncate(content_length);
    Ok(ReadOutcome::Complete { head, body })
}

/// Result of `read_request`.
#[derive(Debug)]
enum ReadOutcome {
    /// Head + raw body bytes ready for parsing. The body
    /// is the un-decoded `Vec<u8>` (spec sect.14.3
    /// multipart bodies must survive binary transport).
    Complete { head: String, body: Vec<u8> },
    /// Content-Length exceeded the configured body limit
    /// (spec sect.14.2 -> 413 before body buffering completes).
    BodyTooLarge { limit: usize },
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Parse the header block into (name, value) pairs. The
/// request line is skipped. Names are lower-cased; a name
/// that repeats is folded into a single entry with values
/// joined by ", " (the Web `Headers` combine rule for
/// non-Set-Cookie headers). Set-Cookie-style multi-value
/// handling (semicolon join) lands with cookies in Phase 8.
fn parse_headers(head: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for line in head.lines().skip(1) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_ascii_lowercase();
        let value = value.trim().to_string();
        match out.iter_mut().find(|(k, _)| *k == name) {
            Some((_, v)) => {
                v.push_str(", ");
                v.push_str(&value);
            }
            None => out.push((name, value)),
        }
    }
    out
}

fn parse_content_length(head: &str) -> usize {
    parse_headers(head)
        .iter()
        .find(|(k, _)| k == "content-length")
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .unwrap_or(0)
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
    use std::collections::BTreeMap;

    #[test]
    fn context_to_json_basic() {
        let ctx = Context {
            method: HttpMethod::Get,
            path: "/".to_string(),
            query: "".to_string(),
            params: std::collections::HashMap::new(),
            body: Vec::new(),
            headers: Vec::new(),
            services: Vec::new(),
            session: None,
        };
        let s = ctx.to_json();
        // The exact wire form is part of the slice 16a
        // contract: it is parsed by the JS preamble.
        assert!(s.contains("\"method\":\"GET\""), "got: {s}");
        assert!(s.contains("\"path\":\"/\""), "got: {s}");
        assert!(s.contains("\"query\":\"\""), "got: {s}");
        assert!(s.contains("\"params\":{}"), "got: {s}");
        // Empty body base64-encodes to the empty string.
        assert!(s.contains("\"body_b64\":\"\""), "got: {s}");
        assert!(s.contains("\"headers\":{}"), "got: {s}");
    }

    #[test]
    fn context_to_json_can_select_internal_fragment() {
        let ctx = Context {
            method: HttpMethod::Get,
            path: "/users".to_string(),
            query: String::new(),
            params: std::collections::HashMap::new(),
            body: Vec::new(),
            headers: Vec::new(),
            services: Vec::new(),
            session: None,
        };
        let json = ctx.to_json_with_fragment(Some("list"));
        assert!(json.contains("\"__tsp_fragment\":\"list\""), "json={json}");
    }

    #[test]
    fn fragment_target_decodes_route_and_preserves_extra_query() {
        let target = fragment_target(
            "/__tsp/fragment",
            &format!(
                "route=%2Fusers%2Fhello%20world&name=list&token={}&sort=desc",
                fragment_token()
            ),
        )
        .expect("valid fragment target");
        assert_eq!(target.0, "/users/hello world");
        assert_eq!(target.1.as_deref(), Some("list"));
        assert_eq!(target.2, "sort=desc");
    }

    #[test]
    fn fragment_target_rejects_missing_or_wrong_capability() {
        assert!(fragment_target("/__tsp/fragment", "route=%2Fusers&name=list").is_none());
        assert!(
            fragment_target("/__tsp/fragment", "route=%2Fusers&name=list&token=wrong").is_none()
        );
    }

    #[test]
    fn context_to_json_serialises_session_view() {
        // Slice 16k: `ctx.session` wire form. The view is
        // an object `{id, data}`; `data` is a JSON object
        // of the keys the page set on prior requests.
        let mut data = BTreeMap::new();
        data.insert(
            "name".to_string(),
            SessionValue::String("alice".to_string()),
        );
        data.insert("n".to_string(), SessionValue::Number(7.0));
        let ctx = Context {
            method: HttpMethod::Get,
            path: "/".to_string(),
            query: String::new(),
            params: std::collections::HashMap::new(),
            body: Vec::new(),
            headers: Vec::new(),
            services: Vec::new(),
            session: Some(SessionView {
                id: "deadbeef".to_string(),
                data,
            }),
        };
        let s = ctx.to_json();
        assert!(
            s.contains("\"session\":{\"id\":\"deadbeef\",\"data\":{\"n\":7,\"name\":\"alice\"}}"),
            "got: {s}"
        );
    }

    #[test]
    fn context_to_json_serialises_no_session_as_null() {
        let ctx = Context {
            method: HttpMethod::Get,
            path: "/".to_string(),
            query: String::new(),
            params: std::collections::HashMap::new(),
            body: Vec::new(),
            headers: Vec::new(),
            services: Vec::new(),
            session: None,
        };
        let s = ctx.to_json();
        assert!(s.contains("\"session\":null"), "got: {s}");
    }

    #[test]
    fn read_session_cookie_returns_sid_or_none() {
        let headers = vec![(
            "cookie".to_string(),
            "a=b; tsp_sid=deadbeef; c=d".to_string(),
        )];
        assert_eq!(read_session_cookie(&headers).as_deref(), Some("deadbeef"));
        let no = vec![("cookie".to_string(), "a=b; c=d".to_string())];
        assert_eq!(read_session_cookie(&no), None);
        let empty: Vec<(String, String)> = Vec::new();
        assert_eq!(read_session_cookie(&empty), None);
    }

    #[test]
    fn build_session_cookie_returns_none_when_unchanged() {
        // Same id -> no Set-Cookie needed; the browser
        // already has the right cookie.
        assert_eq!(build_session_cookie("deadbeef", "deadbeef"), None);
    }

    #[test]
    fn build_session_cookie_emits_set_cookie_for_new_sid() {
        // First request (no cookie) -> fresh id; the
        // response has to plant the cookie.
        let line = build_session_cookie("newhash", "").unwrap();
        assert!(line.contains("tsp_sid=newhash"), "got: {line}");
        assert!(line.contains("HttpOnly"), "got: {line}");
        assert!(line.contains("Path=/"), "got: {line}");
        assert!(line.contains("SameSite=Lax"), "got: {line}");
        // Regenerate: old id -> new id, no Max-Age=0.
        let line = build_session_cookie("newhash", "oldhash").unwrap();
        assert!(line.contains("tsp_sid=newhash"), "got: {line}");
        assert!(!line.contains("Max-Age=0"), "got: {line}");
    }

    #[test]
    fn build_session_cookie_clears_when_destroyed() {
        let line = build_session_cookie("", "newhash").unwrap();
        assert!(line.contains("tsp_sid=;"), "got: {line}");
        assert!(line.contains("Max-Age=0"), "got: {line}");
        assert!(line.contains("HttpOnly"), "got: {line}");
    }

    #[test]
    fn envelope_parses_session_writes() {
        // Slice 16k: the envelope may carry `session_writes`
        // buffered by `ctx.session.*` calls in the page.
        let envelope = "__TSP_OUT_V1__\n{\"type\":\"html\",\"body\":\"hi\",\"headers\":[],\"session_writes\":[{\"op\":\"set\",\"k\":\"a\",\"v\":\"alpha\"},{\"op\":\"set\",\"k\":\"n\",\"v\":7},{\"op\":\"regenerate\"},{\"op\":\"delete\",\"k\":\"x\"},{\"op\":\"destroy\"},{\"op\":\"clear\"},{\"op\":\"unknown\"},\"junk\"]}\n";
        let out = parse_envelope(envelope);
        assert_eq!(out.kind, EnvelopeKind::Html);
        assert_eq!(out.session_writes.len(), 6);
        // Order is preserved (the unknown op and the non-object
        // entry are dropped, but everything before them stays).
        match &out.session_writes[0] {
            SessionWrite::Set(k, SessionValue::String(v)) => {
                assert_eq!(k, "a");
                assert_eq!(v, "alpha");
            }
            other => panic!("expected Set, got {other:?}"),
        }
        match &out.session_writes[1] {
            SessionWrite::Set(k, SessionValue::Number(n)) => {
                assert_eq!(k, "n");
                assert_eq!(*n, 7.0);
            }
            other => panic!("expected Set, got {other:?}"),
        }
        assert!(matches!(out.session_writes[2], SessionWrite::Regenerate));
        assert!(matches!(&out.session_writes[3], SessionWrite::Delete(k) if k == "x"));
        assert!(matches!(out.session_writes[4], SessionWrite::Destroy));
        assert!(matches!(out.session_writes[5], SessionWrite::Clear));
    }

    #[test]
    fn envelope_without_session_writes_has_empty_list() {
        let envelope = "__TSP_OUT_V1__\n{\"type\":\"html\",\"body\":\"hi\",\"headers\":[]}\n";
        let out = parse_envelope(envelope);
        assert!(out.session_writes.is_empty());
    }

    #[test]
    fn session_writes_non_portable_value_dropped() {
        // A non-JSON-compatible value is rejected by the
        // host's own `json_value_to_session`; the rest of
        // the writes still land. The only shape the envelope
        // parser does not understand is a value that is not
        // a JSON tree -- but every JsonValue the host can
        // produce is JSON-compatible, so we exercise the
        // drop path by handing in a `k` with no `v`.
        let envelope = "__TSP_OUT_V1__\n{\"type\":\"html\",\"body\":\"hi\",\"headers\":[],\"session_writes\":[{\"op\":\"set\",\"k\":\"a\"},{\"op\":\"set\",\"k\":\"b\",\"v\":1}]}\n";
        let out = parse_envelope(envelope);
        assert_eq!(out.session_writes.len(), 1);
        match &out.session_writes[0] {
            SessionWrite::Set(k, SessionValue::Number(n)) => {
                assert_eq!(k, "b");
                assert_eq!(*n, 1.0);
            }
            other => panic!("expected Set b/1, got {other:?}"),
        }
    }

    #[test]
    fn envelope_parses_service_logs() {
        // Slice 16j: the envelope may carry `service_logs`
        // buffered by `ctx.services.*` calls in the page.
        let envelope = "__TSP_OUT_V1__\n{\"type\":\"html\",\"body\":\"hi\",\"headers\":[],\"service_logs\":[{\"svc\":\"logger\",\"level\":\"info\",\"message\":\"hit\"},{\"svc\":\"logger\",\"level\":\"error\",\"message\":\"boom\"}]}\n";
        let out = parse_envelope(envelope);
        assert_eq!(out.kind, EnvelopeKind::Html);
        assert_eq!(
            out.service_logs,
            vec![
                LogLine {
                    service: "logger".to_string(),
                    level: "info".to_string(),
                    message: "hit".to_string()
                },
                LogLine {
                    service: "logger".to_string(),
                    level: "error".to_string(),
                    message: "boom".to_string()
                },
            ]
        );
    }

    #[test]
    fn envelope_service_logs_are_dropped_when_malformed() {
        // A page envelope stays valid even if a service_logs
        // entry is malformed; only the bad entries are dropped
        // (missing field, non-object item).
        let envelope = "__TSP_OUT_V1__\n{\"type\":\"response\",\"status\":200,\"headers\":[],\"body\":\"ok\",\"service_logs\":[{\"svc\":\"logger\",\"level\":\"info\",\"message\":\"ok\"},{\"svc\":\"logger\",\"level\":\"info\"},42,\"junk\"]}\n";
        let out = parse_envelope(envelope);
        assert_eq!(out.kind, EnvelopeKind::Response);
        assert_eq!(out.service_logs.len(), 1);
        assert_eq!(out.service_logs[0].message, "ok");
    }

    #[test]
    fn envelope_without_service_logs_has_empty_list() {
        let envelope = "__TSP_OUT_V1__\n{\"type\":\"html\",\"body\":\"hi\",\"headers\":[]}\n";
        let out = parse_envelope(envelope);
        assert!(out.service_logs.is_empty());
    }

    #[test]
    fn context_to_json_with_query() {
        let ctx = Context {
            method: HttpMethod::Get,
            path: "/search".to_string(),
            query: "q=hello&page=2".to_string(),
            params: std::collections::HashMap::new(),
            body: Vec::new(),
            headers: Vec::new(),
            services: Vec::new(),
            session: None,
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
            body: Vec::new(),
            headers: Vec::new(),
            services: Vec::new(),
            session: None,
        };
        let s = ctx.to_json();
        assert!(s.contains("\"id\":\"42\""), "got: {s}");
    }

    #[test]
    fn context_to_json_serialises_body_and_headers() {
        let ctx = Context {
            method: HttpMethod::Post,
            path: "/".to_string(),
            query: String::new(),
            params: std::collections::HashMap::new(),
            // Slice 16g: body is now raw Vec<u8> (the
            // base64 wire form goes through to_json).
            body: b"hello=world".to_vec(),
            headers: vec![
                (
                    "content-type".to_string(),
                    "application/x-www-form-urlencoded".to_string(),
                ),
                ("x-trace".to_string(), "abc".to_string()),
            ],
            services: Vec::new(),
            session: None,
        };
        let s = ctx.to_json();
        // base64("hello=world") = "aGVsbG89d29ybGQ="
        assert!(s.contains("\"body_b64\":\"aGVsbG89d29ybGQ=\""), "got: {s}");
        assert!(
            s.contains("\"headers\":{\"content-type\":\"application/x-www-form-urlencoded\",\"x-trace\":\"abc\"}"),
            "got: {s}"
        );
    }

    #[test]
    fn context_to_json_serialises_binary_body() {
        // Slice 16g: bytes that are NOT valid UTF-8 (e.g. 0xff
        // in the middle) must round-trip through base64. The
        // old lossy String path would have produced U+FFFD
        // and the wrap preamble's `new TextDecoder().decode`
        // would have read the corrupted body.
        let ctx = Context {
            method: HttpMethod::Post,
            path: "/".to_string(),
            query: String::new(),
            params: std::collections::HashMap::new(),
            body: vec![0x00, 0x01, 0x02, 0xff, 0xfe, 0x80],
            headers: Vec::new(),
            services: Vec::new(),
            session: None,
        };
        let s = ctx.to_json();
        // base64([0x00,0x01,0x02,0xff,0xfe,0x80]) = "AAEC//6A"
        assert!(s.contains("\"body_b64\":\"AAEC//6A\""), "got: {s}");
    }

    #[test]
    fn parse_request_extracts_headers() {
        // The head block carries the request line + headers;
        // the body is split off by read_request, so
        // parse_request only sees the header section.
        let head = "POST /submit HTTP/1.1\r\nHost: localhost:3000\r\nContent-Type: text/plain\r\nX-Multi: a\r\nX-Multi: b\r\n\r\n";
        match parse_request(head) {
            ParsedRequest::Known {
                method,
                path,
                query,
                headers,
            } => {
                assert_eq!(method, HttpMethod::Post);
                assert_eq!(path, "/submit");
                assert_eq!(query, "");
                assert_eq!(headers.len(), 3);
                assert!(headers.contains(&("host".to_string(), "localhost:3000".to_string())));
                assert!(headers.contains(&("content-type".to_string(), "text/plain".to_string())));
                // Duplicate header names fold with ", " join.
                assert!(headers.contains(&("x-multi".to_string(), "a, b".to_string())));
            }
            ParsedRequest::Unknown => panic!("expected Known"),
        }
    }

    #[test]
    fn parse_content_length_reads_header() {
        let head = "POST / HTTP/1.1\r\nContent-Length: 11\r\n\r\n";
        assert_eq!(parse_content_length(head), 11);
        let no_body = "GET / HTTP/1.1\r\n\r\n";
        assert_eq!(parse_content_length(no_body), 0);
    }

    /// Build a connected (client, server) TcpStream pair for
    /// read_request tests.
    fn socket_pair() -> (TcpStream, TcpStream) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral");
        let addr = listener.local_addr().expect("local addr");
        let client = TcpStream::connect(addr).expect("connect");
        let (server, _) = listener.accept().expect("accept");
        (client, server)
    }

    #[test]
    fn disconnect_monitor_cancels_when_peer_closes() {
        let (client, server) = socket_pair();
        let cancellation = CancellationToken::new();
        let monitor = start_disconnect_monitor(&server, &cancellation);
        assert!(monitor.is_some());
        drop(client);

        for _ in 0..40 {
            if cancellation.is_cancelled() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        stop_disconnect_monitor(monitor);
        assert!(cancellation.is_cancelled());
    }

    #[test]
    fn read_request_splits_head_and_body() {
        let (mut client, mut server) = socket_pair();
        let wire = "POST / HTTP/1.1\r\nContent-Length: 5\r\n\r\nhello";
        client.write_all(wire.as_bytes()).expect("write");
        let outcome = read_request(&mut server, 1024).expect("read");
        match outcome {
            ReadOutcome::Complete { head, body } => {
                assert!(head.starts_with("POST / HTTP/1.1"));
                assert!(head.ends_with("\r\n\r\n"));
                assert_eq!(body, b"hello".to_vec());
            }
            ReadOutcome::BodyTooLarge { .. } => panic!("not too large"),
        }
    }

    #[test]
    fn read_request_rejects_body_over_limit() {
        // spec sect.14.2: a body over the configured limit is
        // rejected before buffering completes (413).
        let (mut client, mut server) = socket_pair();
        let wire = "POST / HTTP/1.1\r\nContent-Length: 2048\r\n\r\n";
        client.write_all(wire.as_bytes()).expect("write");
        let outcome = read_request(&mut server, 1024).expect("read");
        assert!(matches!(outcome, ReadOutcome::BodyTooLarge { limit: 1024 }));
    }

    #[test]
    fn read_request_body_split_across_reads() {
        // Head and body may arrive in separate TCP segments;
        // read_request must keep reading until Content-Length
        // is satisfied.
        let (mut client, mut server) = socket_pair();
        client
            .write_all(b"POST / HTTP/1.1\r\nContent-Length: 6\r\n\r\nab")
            .expect("write part 1");
        std::thread::sleep(std::time::Duration::from_millis(10));
        client.write_all(b"cdef").expect("write part 2");
        let outcome = read_request(&mut server, 1024).expect("read");
        match outcome {
            ReadOutcome::Complete { body, .. } => assert_eq!(body, b"abcdef".to_vec()),
            ReadOutcome::BodyTooLarge { .. } => panic!("not too large"),
        }
    }

    #[test]
    fn read_request_preserves_binary_body() {
        // Slice 16g: spec sect.14.3 multipart bodies must
        // survive transport -- a body containing a 0x00 byte
        // and other binary garbage round-trips exactly
        // through read_request. The previous slice 16d lossy
        // String path would have substituted U+FFFD for the
        // invalid UTF-8 sequences.
        let (mut client, mut server) = socket_pair();
        let payload: Vec<u8> = vec![0x00, 0x01, 0x02, 0xff, 0xfe, 0x80, 0x90, 0xa0, 0xb0, 0xc0];
        let mut wire = b"POST / HTTP/1.1\r\nContent-Length: ".to_vec();
        wire.extend_from_slice(format!("{}", payload.len()).as_bytes());
        wire.extend_from_slice(b"\r\n\r\n");
        wire.extend_from_slice(&payload);
        client.write_all(&wire).expect("write");
        let outcome = read_request(&mut server, 1024).expect("read");
        match outcome {
            ReadOutcome::Complete { body, .. } => assert_eq!(body, payload),
            ReadOutcome::BodyTooLarge { .. } => panic!("not too large"),
        }
    }

    #[test]
    fn ctx_json_for_env_strips_body_b64_field() {
        // Slice 16g: the env side channel must not carry the
        // body (Windows env block cap ~32 KiB vs the
        // 1 MiB body limit). The body travels inside the
        // embedded literal instead.
        let json = r#"{"method":"POST","path":"/","query":"","params":{},"body_b64":"QklH","headers":{"x":"y"}}"#;
        let env = ctx_json_for_env(json);
        assert!(!env.contains("QklH"), "got: {env}");
        assert!(!env.contains("body_b64"), "got: {env}");
        assert!(env.contains("\"x\":\"y\""), "got: {env}");
        assert!(env.contains("\"method\":\"POST\""), "got: {env}");
    }

    #[test]
    fn base64_encode_round_trips_known_vectors() {
        // RFC 4648 section 10 test vectors.
        let cases: &[(&[u8], &str)] = &[
            (b"", ""),
            (b"f", "Zg=="),
            (b"fo", "Zm8="),
            (b"foo", "Zm9v"),
            (b"foob", "Zm9vYg=="),
            (b"fooba", "Zm9vYmE="),
            (b"foobar", "Zm9vYmFy"),
            (b"hello=world", "aGVsbG89d29ybGQ="),
        ];
        for (input, want) in cases {
            let mut out = String::new();
            base64_encode(&mut out, input);
            assert_eq!(&out, want, "input = {input:?}");
        }
    }

    #[test]
    fn tsp_error_codes_are_stable() {
        // The 1xxx / 2xxx / 3xxx partition is part of the
        // dev-loop contract; spec sect.6.3 names a few
        // (TSP2003 / TSP3001) that the example fixtures
        // reference. Pin the strings here so a future
        // refactor cannot silently renumber them.
        let pairs: &[(TspError, &str)] = &[
            (TspError::RoutesDirMissing, "TSP1001"),
            (TspError::UnsupportedRouteShape, "TSP1002"),
            (TspError::DuplicateRoutePath, "TSP1003"),
            (TspError::RouteIoError, "TSP1004"),
            (TspError::MalformedRequestLine, "TSP2001"),
            (TspError::BodyTooLarge, "TSP2002"),
            (TspError::NoRouteMatches, "TSP2003"),
            (TspError::MethodNotAllowed, "TSP2004"),
            (TspError::MalformedUrl, "TSP2005"),
            (TspError::PagePrepareError, "TSP3001"),
            (TspError::CleanSlotMissingPayload, "TSP3006"),
            (TspError::PageNeverBuilt, "TSP3007"),
            (TspError::UnknownPage, "TSP3008"),
        ];
        for (code, want) in pairs {
            assert_eq!(code.code(), *want, "code = {code:?}");
        }
    }

    #[test]
    fn dev_error_page_html_escapes_user_fields_and_renders_stack() {
        // §32.1: a JSON error envelope from the wrap
        // (e.g. `{"kind":"tsp_error","error":"RangeError",
        // "message":"<bad> & 'quoted'","stack":"at GET (...)\n"})
        // must produce a self-contained HTML page that
        // (a) HTML-escapes every user-controlled field
        // and (b) carries the stack trace inside a
        // `<pre>` block. A regression here would either
        // leak script-injection surface to the dev or
        // strip the stack trace the operator needs.
        let body = r#"{"kind":"tsp_error","error":"RangeError","message":"<bad> & 'quoted'","stack":"Error\n    at GET (routes/foo.tsp:5:7)\n"}"#;
        let (html, ct) = render_dev_error_page(body, "HTTP/1.1 500 Internal Server Error");
        assert_eq!(ct, "text/html; charset=utf-8");
        // The HTML escapes `<`, `>`, `&`, and `'` in
        // every field; a regression to plain
        // interpolation would surface as a raw `<` in
        // the body.
        assert!(
            html.contains("&lt;bad&gt; &amp; &#39;quoted&#39;"),
            "error message must be HTML-escaped; got: {html}"
        );
        assert!(
            html.contains("class=\"error-name\">RangeError<"),
            "error name must render in the error-name slot; got: {html}"
        );
        assert!(
            html.contains("Error\n    at GET (routes/foo.tsp:5:7)\n"),
            "stack trace must be preserved verbatim (no escaping of newlines); got: {html}"
        );
        assert!(
            html.contains("HTTP/1.1 500 Internal Server Error"),
            "the status line must appear in the meta footer; got: {html}"
        );
        assert!(
            html.contains("TSP_DEVELOPMENT=0"),
            "the meta footer must tell the dev how to disable the page; got: {html}"
        );
        // The page must not leak the internal JSON
        // envelope -- the HTML replaces the wire body.
        assert!(
            !html.contains(r#""kind":"tsp_error""#),
            "the raw JSON envelope must not appear in the HTML; got: {html}"
        );
    }

    #[test]
    fn dev_error_page_handles_unparseable_body() {
        // A wrap that somehow returns a non-JSON body
        // (e.g. older wrap scripts that pre-date the
        // slice's JSON convention) must not produce an
        // empty page. The fallback wraps the body in a
        // generic Error entry and shows it as the
        // message so the dev still sees SOMETHING.
        let (html, _ct) =
            render_dev_error_page("not json at all", "HTTP/1.1 500 Internal Server Error");
        assert!(
            html.contains("class=\"error-name\">Error<"),
            "fallback must render the literal name `Error`; got: {html}"
        );
        assert!(
            html.contains("not json at all"),
            "fallback must surface the unparseable body as the message; got: {html}"
        );
    }

    #[test]
    fn format_error_body_typed_form() {
        // Body shape contract: first line is
        // `[TSP-NNNN] <description>`, the detail line
        // follows, the detail keeps its trailing newline
        // when present.
        let body = format_error_body(
            TspError::NoRouteMatches,
            "TSP v2 PoC 1 slice 10b: no route matches path=/ (table has 0 route(s))\n",
        );
        assert!(
            body.starts_with("[TSP2003] no route matches\n"),
            "got: {body:?}"
        );
        assert!(
            body.contains("slice 10b: no route matches"),
            "got: {body:?}"
        );
    }

    #[test]
    fn format_error_body_raw_passes_arbitrary_code() {
        // The raw variant is used by the build pipeline
        // when the failure came from the JSC bridge --
        // it owns the code (`TSP3002` / `TSP3009` /
        // `TSP3012` etc.)
        // and a description, neither of which the host's
        // own enum has a variant for.
        let body = format_error_body_raw(
            "TSP3012",
            "bun subprocess exited non-zero",
            "bun exited 1: error page\n",
        );
        assert!(
            body.starts_with("[TSP3012] bun subprocess exited non-zero\n"),
            "got: {body:?}"
        );
        assert!(body.contains("bun exited 1: error page\n"), "got: {body:?}");
    }

    #[test]
    fn timed_out_build_uses_gateway_timeout_and_typed_body() {
        let error = pipeline::BuildError::Jsc(crate::jsc_bridge::JscError::TimedOut {
            stderr_tail: "page still running".to_string(),
        });
        let (status, body) = build_failure_response(&error, "timeout detail\n".to_string());
        assert_eq!(status, "HTTP/1.1 504 Gateway Timeout");
        assert!(
            body.starts_with("[TSP3009] request timed out\n"),
            "got: {body:?}"
        );
        assert!(body.contains("timeout detail\n"), "got: {body:?}");
    }

    #[test]
    fn format_error_body_adds_trailing_newline_if_missing() {
        // Detail without a trailing newline still gets
        // one so the wire form always ends with \n.
        let body = format_error_body(TspError::BodyTooLarge, "limit=8 bytes");
        assert!(body.ends_with('\n'), "got: {body:?}");
    }

    #[test]
    fn json_value_serialize_roundtrips() {
        // The serializer used by ctx_json_for_env.
        let json =
            r#"{"type":"response","status":201,"headers":{"x-comma":"a,b,c"},"body":"created"}"#;
        let v = parse_json(json).expect("parse");
        let mut out = String::new();
        v.serialize(&mut out);
        assert_eq!(out, json);
    }

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
        assert!(
            out.headers
                .contains(&("x-demo".to_string(), "slice16c".to_string()))
        );
        assert!(
            out.headers
                .contains(&("x-comma".to_string(), "a,b,c".to_string()))
        );
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
        let out = parse_envelope(
            "__TSP_OUT_V1__\n{\"type\":\"response\",\"status\":599,\"headers\":{},\"body\":\"x\"}",
        );
        assert_eq!(out.status_line, "HTTP/1.1 200 OK");
        assert_eq!(out.body, "x");
    }

    #[test]
    fn json_parser_handles_escaped_quotes() {
        // A header value containing an escaped quote.
        let out = parse_envelope(
            "__TSP_OUT_V1__\n{\"type\":\"response\",\"status\":200,\"headers\":{\"x-quote\":\"say \\\"hi\\\"\"},\"body\":\"ok\"}",
        );
        assert!(
            out.headers
                .contains(&("x-quote".to_string(), "say \"hi\"".to_string()))
        );
    }

    #[test]
    fn envelope_parses_array_headers_preserving_multi_value() {
        // Slice 16f: the wrap script emits `headers` as an
        // array of [name, value] pairs so multi-value
        // Set-Cookie lines are preserved verbatim. Two
        // Set-Cookie entries with the same name must
        // both reach the writer -- they do NOT collapse
        // to a single entry the way the slice 16c flat
        // object would.
        let out = parse_envelope(
            "__TSP_OUT_V1__\n{\"type\":\"response\",\"status\":200,\"headers\":[[\"Set-Cookie\",\"a=1; Path=/\"],[\"Set-Cookie\",\"b=2; Path=/\"],[\"content-type\",\"text/plain\"]],\"body\":\"ok\"}",
        );
        assert_eq!(out.kind, EnvelopeKind::Response);
        assert_eq!(out.content_type, "text/plain");
        let set_cookie: Vec<&str> = out
            .headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case("set-cookie"))
            .map(|(_, v)| v.as_str())
            .collect();
        assert_eq!(set_cookie.len(), 2, "got: {:?}", out.headers);
        assert!(set_cookie.contains(&"a=1; Path=/"));
        assert!(set_cookie.contains(&"b=2; Path=/"));
    }

    #[test]
    fn envelope_rejects_malformed_array_header_entries() {
        // A non-pair array element (string, number, single
        // element array, 3+ tuple) is skipped, not fatal --
        // the wrap script can never produce these, but the
        // parser must be defensive.
        let out = parse_envelope(
            "__TSP_OUT_V1__\n{\"type\":\"response\",\"status\":200,\"headers\":[[\"x-ok\",\"yes\"],\"garbage\",[\"only-one\"],[\"a\",\"b\",\"c\"]],\"body\":\"ok\"}",
        );
        assert_eq!(out.headers.len(), 1, "got: {:?}", out.headers);
        assert!(
            out.headers
                .contains(&("x-ok".to_string(), "yes".to_string()))
        );
    }
}
