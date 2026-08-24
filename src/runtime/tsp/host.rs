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
/// for (e.g. `TSP3002` JSX transform, `TSP3012`
/// subprocess failure) use `format_error_body_raw`
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
        out.push_str("}}");
        out
    }
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
    const ALPH: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
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
                let headers = parse_envelope_headers(obj.get("headers"));
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

fn resolve_max_body_bytes() -> usize {
    match std::env::var("TSP_MAX_BODY_BYTES") {
        Ok(s) => s.parse::<usize>().unwrap_or(DEFAULT_MAX_BODY_BYTES),
        Err(_) => DEFAULT_MAX_BODY_BYTES,
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
    // Slice 16d: read the full request (header block up to
    // CRLFCRLF + exactly Content-Length body bytes). Body
    // over the configured limit (TSP_MAX_BODY_BYTES,
    // default 1 MiB) is rejected with 413 before the page
    // sees it (spec sect.14.2).
    let max_body = resolve_max_body_bytes();
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
            stream.write_all(head.as_bytes()).map_err(HostError::Connection)?;
            stream.write_all(body_text.as_bytes()).map_err(HostError::Connection)?;
            let _ = stream.shutdown(Shutdown::Both);
            return Ok(());
        }
    };
    let parsed = parse_request(&head);

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
        ParsedRequest::Known { method, path, query, headers } => {
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
            let matched = routes.lookup(&path, method);
            let params: std::collections::HashMap<String, String> = match &matched {
                MatchResult::Found { route, .. } | MatchResult::FoundHeadOverGet { route } => {
                    route.params.clone()
                }
                _ => std::collections::HashMap::new(),
            };
            let ctx = Context {
                method,
                path: path.clone(),
                query,
                params,
                body,
                headers,
            };
            match matched {
            MatchResult::Found { route, method: req_method } => {
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
                    || !ctx.params.is_empty();
                let (_status_line, _ct, allow_header, body) = if per_request {
                    render_per_request(&route, req_method, bun, &ctx)
                } else {
                    render_for_route(&route, req_method, &page_ref, registry, bun, &ctx)
                };
                // Slice 16b: bun emits a `__TSP_OUT_V1__` envelope
                // with the page's return value classified as
                // either HtmlNode (string) or Web Response.
                // parse_envelope unpacks it and surfaces the
                // correct status / content-type / body / headers.
                let outcome = parse_envelope(&body);
                // The envelope is the source of truth for the
                // page's actual status when the page ran. When
                // the envelope is absent (Legacy), the body is
                // an error page produced by the host (405 / 500)
                // and the host's own status line must win --
                // otherwise a method rejection would be served
                // as 200 OK.
                let use_envelope = outcome.kind != EnvelopeKind::Legacy;
                (
                    if use_envelope {
                        outcome.status_line
                    } else {
                        _status_line
                    },
                    if use_envelope {
                        outcome.content_type
                    } else {
                        _ct.to_string()
                    },
                    allow_header,
                    outcome.body,
                    outcome.headers,
                )
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
                format_error_body(
                    TspError::NoRouteMatches,
                    &format!(
                        "TSP v2 PoC 1 slice 10b: no route matches path={path} (table has {} route(s))\n",
                        routes.len()
                    ),
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
/// 405 (method not exported) and 500 (build failure) still
/// shape correctly.
fn render_per_request(
    route: &crate::router::Route,
    requested: HttpMethod,
    bun: &BunRuntime,
    ctx: &Context,
) -> (&'static str, &'static str, Option<String>, String) {
    // Mirror render_for_route's 405 conversion: the page must
    // export the requested method. We use the static detector
    // (page::prepare) rather than the registry because we are
    // deliberately outside the registry path.
    match crate::page::prepare(route) {
        Ok(page) if page.methods.contains(&requested) => {
            match pipeline::build(route, requested, bun, &ctx.to_json()) {
                Ok(body) => ("HTTP/1.1 200 OK", "text/html; charset=utf-8", None, body),
                Err(e) => {
                    let msg = format!("{e}");
                    eprintln!(
                        "TSPv2PoC1: build error on {}: {e}",
                        route.source.display()
                    );
                    // Build-time errors carry their own
                    // `BuildError::code()` (TSP3001 for
                    // prepare, TSP3002 / TSP3010-3014 for
                    // the bridge). The bridge code path
                    // supplies a more specific description
                    // (JscError::describe); the prepare
                    // path falls back to the host enum.
                    let (code, desc) = match &e {
                        pipeline::BuildError::Prepare(_) => {
                            (TspError::PagePrepareError.code(),
                             TspError::PagePrepareError.describe())
                        }
                        pipeline::BuildError::Jsc(j) => (j.code(), j.describe()),
                    };
                    (
                        "HTTP/1.1 500 Internal Server Error",
                        "text/plain; charset=utf-8",
                        None,
                        format_error_body_raw(
                            code,
                            desc,
                            &format!(
                                "TSP v2 PoC 1 slice 16d: build error on {}\n  {msg}\n",
                                route.source.display()
                            ),
                        ),
                    )
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
fn read_request(
    stream: &mut TcpStream,
    max_body: usize,
) -> Result<ReadOutcome, HostError> {
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

    #[test]
    fn context_to_json_basic() {
        let ctx = Context {
            method: HttpMethod::Get,
            path: "/".to_string(),
            query: "".to_string(),
            params: std::collections::HashMap::new(),
            body: Vec::new(),
            headers: Vec::new(),
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
    fn context_to_json_with_query() {
        let ctx = Context {
            method: HttpMethod::Get,
            path: "/search".to_string(),
            query: "q=hello&page=2".to_string(),
            params: std::collections::HashMap::new(),
            body: Vec::new(),
            headers: Vec::new(),
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
                ("content-type".to_string(), "application/x-www-form-urlencoded".to_string()),
                ("x-trace".to_string(), "abc".to_string()),
            ],
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
            ParsedRequest::Known { method, path, query, headers } => {
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
        client.write_all(b"POST / HTTP/1.1\r\nContent-Length: 6\r\n\r\nab").expect("write part 1");
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
    fn format_error_body_typed_form() {
        // Body shape contract: first line is
        // `[TSP-NNNN] <description>`, the detail line
        // follows, the detail keeps its trailing newline
        // when present.
        let body = format_error_body(
            TspError::NoRouteMatches,
            "TSP v2 PoC 1 slice 10b: no route matches path=/ (table has 0 route(s))\n",
        );
        assert!(body.starts_with("[TSP2003] no route matches\n"), "got: {body:?}");
        assert!(body.contains("slice 10b: no route matches"), "got: {body:?}");
    }

    #[test]
    fn format_error_body_raw_passes_arbitrary_code() {
        // The raw variant is used by the build pipeline
        // when the failure came from the JSC bridge --
        // it owns the code (`TSP3002` / `TSP3012` etc.)
        // and a description, neither of which the host's
        // own enum has a variant for.
        let body = format_error_body_raw(
            "TSP3012",
            "bun subprocess exited non-zero",
            "bun exited 1: error page\n",
        );
        assert!(body.starts_with("[TSP3012] bun subprocess exited non-zero\n"),
                "got: {body:?}");
        assert!(body.contains("bun exited 1: error page\n"), "got: {body:?}");
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
        let json = r#"{"type":"response","status":201,"headers":{"x-comma":"a,b,c"},"body":"created"}"#;
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
        assert!(out.headers.contains(&("x-ok".to_string(), "yes".to_string())));
    }
}
