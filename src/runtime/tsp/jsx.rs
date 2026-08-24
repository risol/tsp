//! Minimal TSX -> JS pre-processor for TSP v2 PoC 1 slice 6.
//!
//! See `tsp-v2-plan.md` sect.11 (TSP JSX runtime) and sect.3.1
//! (`.tsp` is standard TSX). The full JSX -> HtmlNode pipeline (the
//! spec's `tsp:jsx-runtime` + HtmlNode ABI) lands in slice 7+; for
//! slice 6 we only need to land a "the host can turn a `.tsp` into
//! a runnable JS string" path that is honest about its scope. The
//! transform below:
//!
//! - replaces single-line `<tag>text</tag>` JSX with a string literal
//!   `"<tag>text</tag>"` so the slice-1 fixture
//!   `<h1>Hello from TSP v2</h1>` becomes the runnable
//!   `return "<h1>Hello from TSP v2</h1>";`;
//! - strips the `export ` keyword from `export function` declarations
//!   so the page module evaluates under `bun -e` (no module wrapper);
//! - leaves everything else alone and surfaces "unsupported shape"
//!   on anything it cannot pattern-match (nested JSX, components,
//!   attributes, fragments, multi-line JSX). The real TSX parser is
//!   slice 7.
//!
//! Two transforms we explicitly do *not* do here:
//! - TypeScript type annotation stripping (`: Context`, `<T>`, `as X`).
//!   The fixture has none and PoC 1 does not need them; the slice-7
//!   `bun_js_parser` pass handles them properly.
//! - JSX attribute handling (`class`, `onClick`, etc.). Same reason.
//!
//! These are documented in `tsp-v2-plan.md` sect.10.4 ("prohibit shape
//! magic") -- the host's job is to refuse, not to silently mis-render.
use std::fmt::Write as _;

#[derive(Debug)]
pub enum JsxError {
    /// The source had a JSX shape the slice-6 pre-processor cannot
    /// translate to plain JS. Surfacing this to the operator is
    /// better than producing a half-correct transform.
    UnsupportedShape { line: usize, reason: &'static str },
}

impl std::fmt::Display for JsxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedShape { line, reason } => {
                write!(f, "unsupported JSX shape at line {line}: {reason}")
            }
        }
    }
}

impl std::error::Error for JsxError {}

/// Transform a `.tsp` source string into a runnable `.js` string for
/// `bun -e` / `bun run tempfile`. See module docs for scope.
pub fn tsx_to_js(source: &str) -> Result<String, JsxError> {
    // 1. JSX single-line `<tag>text</tag>` -> string literal
    //    `"<tag>text</tag>"`. Anything richer (nested / attributes /
    //    multi-line) is `UnsupportedShape` so the operator gets a
    //    useful diagnostic instead of a half-correct transform.
    let mut out = String::with_capacity(source.len());
    for (idx, line) in source.lines().enumerate() {
        if let Some(stripped) = try_strip_inline_jsx(line) {
            out.push_str(&stripped);
        } else if has_jsx(line) {
            return Err(JsxError::UnsupportedShape {
                line: idx + 1,
                reason: "slice-6 only handles single-line <tag>text</tag> JSX; see plan sect.11 for the full JSX ABI",
            });
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }

    // 2. Strip `export ` from `export function NAME(` lines so the
    //    resulting JS evaluates under `bun -e` (no module wrapper).
    let mut out2 = String::with_capacity(out.len());
    for line in out.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("export function ") {
            let indent_len = line.len() - trimmed.len();
            for _ in 0..indent_len {
                out2.push(' ');
            }
            out2.push_str("function ");
            out2.push_str(rest);
        } else {
            out2.push_str(line);
        }
        out2.push('\n');
    }

    Ok(out2)
}

/// If `line` contains a single-line `<tag>text</tag>` JSX expression,
/// return a copy of the line with the JSX replaced by a string literal.
/// Returns `None` if the line has no JSX or has a shape we don't handle
/// (nested tags, attributes, fragments, multi-line JSX).
fn try_strip_inline_jsx(line: &str) -> Option<String> {
    // The slice-6 shape is exactly: one opening tag `<NAME>` (NAME is
    // `[A-Za-z][A-Za-z0-9]*`, no attributes), text content that does
    // NOT itself contain `<`, and a matching `</NAME>`, all on one
    // line. Anything else returns `None` so the caller can produce a
    // clean `UnsupportedShape` error rather than silently mis-render.
    let open = find_opening_tag(line)?;
    let name = &line[open.tag_start + 1..open.name_end];
    let content_start = open.name_end + 1;
    let close_tag = format!("</{name}>");
    let close_rel = line[content_start..].find(&close_tag)?;
    let content_end = content_start + close_rel;
    let content = &line[content_start..content_end];
    // Nested JSX guard: a content that starts with `<` means there is
    // another JSX element inside, which is not slice-6 shape.
    if content.trim_start().starts_with('<') {
        return None;
    }
    let after_close = content_end + close_tag.len();

    let mut out = String::with_capacity(line.len() + 4);
    out.push_str(&line[..open.tag_start]);
    out.push('"');
    let _ = write!(out, "<{name}>");
    out.push_str(content);
    let _ = write!(out, "</{name}>");
    out.push('"');
    out.push_str(&line[after_close..]);
    Some(out)
}

/// Position of the first `<NAME>` opening tag in `line`.
struct OpeningTag {
    tag_start: usize,
    /// Index of the byte AFTER the tag name (i.e. the `>` of `<NAME>`).
    name_end: usize,
}

fn find_opening_tag(line: &str) -> Option<OpeningTag> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'<' {
            i += 1;
            continue;
        }
        // Must be followed by an alphabetic character to count as a
        // tag (excludes `</`, `<!--`, `<=`, `<<`, etc.).
        let next = *bytes.get(i + 1)?;
        if !next.is_ascii_alphabetic() {
            i += 1;
            continue;
        }
        let name_len = parse_tag_name(&bytes[i + 1..])?;
        let name_end = i + 1 + name_len;
        if bytes.get(name_end) != Some(&b'>') {
            i += 1;
            continue;
        }
        return Some(OpeningTag {
            tag_start: i,
            name_end,
        });
    }
    None
}

/// True if `line` contains a JSX-shaped `<` followed by an alphabetic
/// character. Used by `tsx_to_js` to distinguish "no JSX" (pass
/// through) from "JSX we cannot handle" (return `UnsupportedShape`).
fn has_jsx(line: &str) -> bool {
    let bytes = line.as_bytes();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'<' && bytes[i + 1].is_ascii_alphabetic() {
            return true;
        }
    }
    false
}

/// Parse `[A-Za-z][A-Za-z0-9]*` at the start of `bytes`. Returns the
/// length of the matched name, or `None` if the first char is not
/// alphabetic.
fn parse_tag_name(bytes: &[u8]) -> Option<usize> {
    let first = *bytes.first()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }
    let mut len = 1;
    while len < bytes.len() && bytes[len].is_ascii_alphanumeric() {
        len += 1;
    }
    Some(len)
}

/// Wrap a transformed module body in a small "evaluate GET and print
/// the result" preamble so `bun run tempfile.js` emits the HTTP
/// response body on stdout. Slice 6 uses `console.log`; the slice-7
/// in-process JSC bridge replaces this with a direct return value.
///
/// `ctx_json` is the JSON-serialised `Context` the host built for
/// the request (spec sect.13). If `Some`, the preamble reads
/// `process.env.TSP_CONTEXT_JSON` (the env var the host sets before
/// spawning bun) and binds the parsed object to `__tspContext`,
/// which is then passed to the exported method as its single
/// argument. If `None`, the method is called with no argument so
/// the legacy zero-arg fixture keeps working.
/// Wrap a transformed module body in a preamble that
/// invokes the page handler, inspects the return value, and
/// emits a `__TSP_OUT_V1__` envelope to stdout. The host
/// parses the envelope to pick the right HTTP response
/// shape:
///
/// - `instanceof Response` -> `{type: 'response', status,
///   headers, body}` (status / headers / body verbatim from
///   the page's Response object).
/// - `typeof string` -> `{type: 'html', body}` (the string
///   is the rendered HTML; the host emits 200 text/html).
/// - anything else (number / boolean / object / null /
///   undefined) -> bun throws, the host sees a non-zero
///   exit and serves a 500. This matches spec sect.6.3
///   (invalid return values).
///
/// `ctx_json` is the JSON-serialised `Context` (spec
/// sect.13). If `Some`, the preamble parses it and passes
/// the resulting object as the page handler's only
/// argument. If `None`, the handler is called with no
/// argument so legacy zero-arg fixtures keep working.
pub fn wrap_for_bun_cli(
    transformed: &str,
    method: &str,
    ctx_json: Option<&str>,
) -> String {
    let mut out = String::with_capacity(transformed.len() + 1024);
    out.push_str("// Generated by TSP v2 PoC 1 slice 16b (jsx.rs)\n");
    out.push_str("// Transformed .tsp -> runnable .js for `bun run tempfile`.\n");
    out.push_str("// Do not edit by hand; the host regenerates this on every request.\n");
    out.push_str("const __tspConsoleLog = globalThis.console ? globalThis.console.log : (...a) => print(...a);\n");
    if let Some(json) = ctx_json {
        // Embed the Context JSON as a literal so the
        // page does not have to read process.env (and
        // so a context-shape mismatch breaks the
        // build rather than silently mis-rendering).
        //
        // `{json:?}` (Rust Debug for str) emits a valid JS
        // string literal: it escapes `"` / `\` / control
        // chars exactly like JS needs. Do NOT pre-escape the
        // JSON with a manual `replace` on top -- the Debug
        // formatter would then double-escape backslashes
        // (body text containing `\"` broke `JSON.parse`
        // before slice 16d fixed this).
        let _ = write!(
            out,
            "const __tspContext = JSON.parse({json:?});\n",
            json = json
        );
        // Slice 16d: build the Web-standard request-facing
        // context from the wire Context. `ctx.url` is a real
        // `URL` (spec sect.13.4), `ctx.query` is
        // `ctx.url.searchParams` (spec sect.13.5),
        // `ctx.request` is a real Web `Request` (spec
        // sect.13.3 -- text() / json() / formData() all
        // come from Bun's native Request), and `ctx.signal`
        // is a live AbortSignal (spec sect.13.7). The host
        // does not abort it yet (no timeout / disconnect
        // detection), but pages can register listeners.
        // The request body is attached only for methods that
        // carry one (a GET/HEAD Request with a body is a
        // Web API TypeError). The page's own `content-length`
        // header is dropped -- the host computes the real
        // length from the lossy body it read.
        out.push_str(
            "const __tspReqHost = (__tspContext.headers && __tspContext.headers.host) ? __tspContext.headers.host : 'localhost';\n\
             const __tspReqUrl = new URL('http://' + __tspReqHost + __tspContext.path + (__tspContext.query ? '?' + __tspContext.query : ''));\n\
             __tspContext.url = __tspReqUrl;\n\
             __tspContext.query = __tspReqUrl.searchParams;\n\
             const __tspReqHeaders = new Headers(__tspContext.headers || {});\n\
             // Slice 16g: the body is raw bytes (base64 over\n\
             // the wire -- JSON has no native bytes shape)\n\
             // so binary multipart reaches Bun's native\n\
             // `Request` constructor intact, ready for\n\
             // `await ctx.request.formData()` (spec\n\
             // sect.14.3). We atob the string to a\n\
             // Uint8Array; for non-body methods (GET/HEAD)\n\
             // the field is empty and we skip the\n\
             // attachment.\n\
             const __tspReqInit = { method: __tspContext.method, headers: __tspReqHeaders };\n\
             if (__tspContext.method !== 'GET' && __tspContext.method !== 'HEAD' && __tspContext.body_b64) {\n\
             \x20 // Wrap the decoded bytes in a Blob (with the\n\
             \x20 // proper MIME type from the request header) so\n\
             \x20 // Bun's `Request.formData()` can stream-parse\n\
             \x20 // multipart -- a bare Uint8Array body has no\n\
             \x20 // \"duplex half\" association and Bun's multipart\n\
             \x20 // parser hangs on file parts without it.\n\
             \x20 const __tspBodyBytes__ = Uint8Array.from(atob(__tspContext.body_b64), __c__ => __c__.charCodeAt(0));\n\
             \x20 const __tspBodyMime__ = __tspReqHeaders.get('content-type') || 'application/octet-stream';\n\
             \x20 __tspReqInit.body = new Blob([__tspBodyBytes__], { type: __tspBodyMime__ });\n\
             \x20 __tspReqInit.duplex = 'half';\n\
             }\n\
             __tspContext.request = new Request(__tspReqUrl, __tspReqInit);\n\
             __tspContext.signal = new AbortController().signal;\n\
             // Slice 16f: ctx.cookies (spec sect.15). Parse the\n\
             // request's Cookie header (form: `a=b; c=d`) into a\n\
             // tiny read-only map, then expose read methods. The\n\
             // `__tspCookieWrites` buffer collects Set-Cookie\n\
             // lines the page emits via `ctx.cookies.set` / `.delete`;\n\
             // the async IIFE below merges them into the response\n\
             // header array.\n\
             const __tspCookieMap__ = new Map();\n\
             const __tspCookieRaw__ = __tspReqHeaders.get('cookie') || '';\n\
             for (const __pair__ of __tspCookieRaw__.split(';')) {\n\
             \x20 const __trimmed__ = __pair__.trim();\n\
             \x20 if (!__trimmed__) continue;\n\
             \x20 const __eq__ = __trimmed__.indexOf('=');\n\
             \x20 if (__eq__ < 0) continue;\n\
             \x20 const __ckName__ = __trimmed__.slice(0, __eq__).trim();\n\
             \x20 const __ckValue__ = __trimmed__.slice(__eq__ + 1).trim();\n\
             \x20 if (__ckName__) __tspCookieMap__.set(__ckName__, __ckValue__);\n\
             }\n\
             const __tspCookieWrites = [];\n\
             function __tspFormatCookie__(__name__, __value__, __options__) {\n\
             \x20 let __line__ = __name__ + '=' + __value__;\n\
             \x20 const __opts__ = __options__ || {};\n\
             \x20 if (__opts__.path) __line__ += '; Path=' + __opts__.path;\n\
             \x20 if (typeof __opts__.maxAge === 'number') __line__ += '; Max-Age=' + __opts__.maxAge;\n\
             \x20 if (__opts__.domain) __line__ += '; Domain=' + __opts__.domain;\n\
             \x20 if (__opts__.httpOnly) __line__ += '; HttpOnly';\n\
             \x20 if (__opts__.secure) __line__ += '; Secure';\n\
             \x20 if (__opts__.sameSite) __line__ += '; SameSite=' + __opts__.sameSite;\n\
             \x20 return __line__;\n\
             }\n\
             __tspContext.cookies = {\n\
             \x20 get(__n__) { return __tspCookieMap__.has(__n__) ? __tspCookieMap__.get(__n__) : undefined; },\n\
             \x20 has(__n__) { return __tspCookieMap__.has(__n__); },\n\
             \x20 set(__n__, __v__, __o__) { __tspCookieWrites.push(__tspFormatCookie__(__n__, __v__, __o__)); },\n\
             \x20 delete(__n__, __o__) { const __optDel__ = Object.assign({maxAge: 0}, __o__ || {}); __tspCookieWrites.push(__tspFormatCookie__(__n__, '', __optDel__)); },\n\
             };\n"
        );
    }
    // Existence check + invocation + envelope emission.
    // The user's transformed source (with `function METHOD(...)`
    // declarations) is appended LAST so the call below sees
    // the declarations in scope. Function declarations are
    // hoisted in JS so the order is for code-clarity only,
    // but we still keep the call-after pattern because the
    // async envelope needs the function reference to exist.
    let _ = write!(
        out,
        "if (typeof {method} !== 'function') {{\n  throw new Error('page does not export function {method}()');\n}}\n"
    );
    let call = if ctx_json.is_some() {
        format!("const __tspResultPromise__ = {method}(__tspContext);\n")
    } else {
        format!("const __tspResultPromise__ = {method}();\n")
    };
    out.push_str(&call);
    // Slice 16f: the wrap preamble now (a) builds
    // `ctx.cookies` with read methods and a write-buffer
    // (`__tspCookieWrites`), and (b) emits the response
    // envelope with `headers` as an ARRAY of `[name, value]`
    // pairs (preserving multi-value `Set-Cookie` lines
    // per spec sect.15: "preserve all valid cookie header
    // lines rather than comma-joining them"). The host's
    // `parse_envelope` accepts the array shape and the
    // writer emits one wire line per entry, so a Response
    // that calls `ctx.cookies.set('a', 'v1')` followed by
    // `ctx.cookies.set('b', 'v2')` surfaces as two
    // `Set-Cookie:` wire lines, not one comma-joined line.
    out.push_str(
        "(async () => {\n         \x20let __tspBody__, __tspStatus__, __tspHeaders__, __tspType__;\n         \x20const __tspResult__ = await __tspResultPromise__;\n         \x20__tspHeaders__ = [];\n         \x20if (__tspResult__ instanceof Response) {\n         \x20\x20__tspType__ = 'response';\n         \x20\x20__tspStatus__ = __tspResult__.status;\n         \x20\x20for (const [__k__, __v__] of __tspResult__.headers) __tspHeaders__.push([__k__, __v__]);\n         \x20\x20__tspBody__ = await __tspResult__.text();\n         \x20} else if (typeof __tspResult__ === 'string') {\n         \x20\x20__tspType__ = 'html';\n         \x20\x20__tspStatus__ = 200;\n         \x20\x20__tspBody__ = __tspResult__;\n         \x20} else {\n         \x20\x20throw new Error('page returned invalid value (expected string or Response, got ' + (typeof __tspResult__) + ')');\n         \x20}\n         \x20// Merge runtime cookie writes into the outgoing headers\n         \x20// (spec sect.15: cookie writes MUST be reflected even when\n         \x20// the handler returns an HtmlNode). Each write becomes a\n         \x20// separate Set-Cookie line so multiple cookies on one\n         \x20// request don't collapse via the response's flatten loop.\n         \x20if (Array.isArray(__tspCookieWrites)) {\n         \x20\x20for (const __cookieLine__ of __tspCookieWrites) {\n         \x20\x20\x20__tspHeaders__.push(['Set-Cookie', __cookieLine__]);\n         \x20\x20}\n         \x20}\n         \x20const __tspEnvelope__ = JSON.stringify({type: __tspType__, status: __tspStatus__, headers: __tspHeaders__, body: __tspBody__});\n         \x20__tspConsoleLog('__TSP_OUT_V1__' + '\\n' + __tspEnvelope__);\n         })().catch((e) => { console.error(String(e && e.stack || e)); process.exit(1); });\n"
    );
    out.push_str(transformed);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_export_keyword() {
        let src = "export function GET() { return 'x'; }\n";
        let out = tsx_to_js(src).unwrap();
        assert!(out.contains("function GET"));
        assert!(!out.contains("export function GET"));
    }

    #[test]
    fn inline_jsx_to_string_literal() {
        let src = "export function GET() { return <h1>Hello</h1>; }\n";
        let out = tsx_to_js(src).unwrap();
        assert!(out.contains("\"<h1>Hello</h1>\""), "got: {out}");
    }

    #[test]
    fn slice_6_fixture_full_transform() {
        let src = "// comment\nexport function GET() {\n  return <h1>Hello from TSP v2</h1>;\n}\n";
        let out = tsx_to_js(src).unwrap();
        assert!(!out.contains("export function"));
        assert!(out.contains("function GET"));
        assert!(
            out.contains("\"<h1>Hello from TSP v2</h1>\""),
            "got: {out}"
        );
    }

    #[test]
    fn nested_jsx_is_unsupported() {
        // Slice 6 only handles flat single-line `<tag>text</tag>`.
        // A nested form must error rather than silently mis-render.
        let src = "export function GET() { return <div><h1>Hi</h1></div>; }\n";
        let err = tsx_to_js(src).unwrap_err();
        assert!(matches!(err, JsxError::UnsupportedShape { .. }));
    }

    #[test]
    fn no_jsx_passes_through() {
        let src = "const x = 42;\n";
        let out = tsx_to_js(src).unwrap();
        assert!(out.contains("const x = 42;"));
    }

    #[test]
    fn wrap_emits_call_to_method_no_ctx() {
        // No Context -> call the method with no argument.
        let body = "function GET() { return 'ok'; }\n";
        let wrapped = wrap_for_bun_cli(body, "GET", None);
        assert!(wrapped.contains("const __tspResultPromise__ = GET();"));
        assert!(wrapped.contains("__TSP_OUT_V1__"));
        // And the ctx-json is NOT in the output.
        assert!(!wrapped.contains("__tspContext"));
    }

    #[test]
    fn wrap_emits_call_to_method_with_ctx() {
        // With a Context JSON, the preamble parses it and
        // passes the result as the method's only argument.
        let body = "function GET(ctx) { return ctx.method; }\n";
        let json = r#"{"method":"GET"}"#;
        let wrapped = wrap_for_bun_cli(body, "GET", Some(json));
        assert!(wrapped.contains("const __tspContext = JSON.parse("));
        assert!(wrapped.contains("const __tspResultPromise__ = GET(__tspContext);"));
        assert!(wrapped.contains("__TSP_OUT_V1__"));
    }

    #[test]
    fn wrap_builds_request_url_query_signal() {
        // Slice 16d: the preamble decorates the context with
        // the Web-standard request surface -- a real URL, a
        // URLSearchParams query, a Web Request, and a live
        // AbortSignal. All four must appear in the wrapped
        // output when a ctx_json is provided.
        let body = "function POST(ctx) { return ctx.request.text(); }\n";
        let json = r#"{"method":"POST","path":"/","query":"a=1","headers":{"host":"localhost:9000"},"body_b64":"aGk="}"#;
        let wrapped = wrap_for_bun_cli(body, "POST", Some(json));
        assert!(wrapped.contains("new URL("), "got: {wrapped}");
        assert!(wrapped.contains("searchParams"), "got: {wrapped}");
        assert!(wrapped.contains("new Request("), "got: {wrapped}");
        assert!(wrapped.contains("new AbortController().signal"), "got: {wrapped}");
        // The request body is passed to Bun's native Request
        // for body-bearing methods (POST). Slice 16g
        // changed the wire form: base64 over the JSON
        // field `body_b64`; the wrap preamble atob-decodes
        // it to a Uint8Array so binary multipart survives.
        assert!(wrapped.contains("atob(__tspContext.body_b64)"), "got: {wrapped}");
        assert!(wrapped.contains("Uint8Array.from"), "got: {wrapped}");
    }

    #[test]
    fn wrap_get_request_has_no_body() {
        // GET/HEAD Requests must not carry a body (Web API
        // TypeError), so the init object only gains `.body`
        // for other methods.
        let body = "function GET(ctx) { return 'x'; }\n";
        let json = r#"{"method":"GET","path":"/","query":"","headers":{},"body_b64":""}"#;
        let wrapped = wrap_for_bun_cli(body, "GET", Some(json));
        // The GET branch skips attaching a body; verify the
        // method-check guard is present in the preamble.
        assert!(wrapped.contains("__tspContext.method !== 'GET'"), "got: {wrapped}");
    }

    #[test]
    fn wrap_builds_cookies_with_read_and_write_methods() {
        // Slice 16f: the preamble parses the request's Cookie
        // header into ctx.cookies (get/has) and exposes set
        // /delete that push lines into a writes buffer the
        // async IIFE merges into the response.
        let body = "function GET(ctx) { ctx.cookies.set('sid', 'abc'); return ''; }\n";
        let json = r#"{"method":"GET","path":"/","query":"","headers":{"cookie":"a=1; sid=old; c=3"},"body":""}"#;
        let wrapped = wrap_for_bun_cli(body, "GET", Some(json));
        // Cookie parsing: split on ';' and read from headers.
        assert!(wrapped.contains("__tspCookieMap__"), "got: {wrapped}");
        assert!(wrapped.contains("__tspCookieWrites"), "got: {wrapped}");
        // Read API: get/has.
        assert!(wrapped.contains("get(__n__)"), "got: {wrapped}");
        assert!(wrapped.contains("has(__n__)"), "got: {wrapped}");
        // Write API: set/delete push formatted lines.
        assert!(wrapped.contains("__tspFormatCookie__"), "got: {wrapped}");
        // Async IIFE merges writes into the response header
        // array (Set-Cookie entries).
        assert!(wrapped.contains("['Set-Cookie', __cookieLine__]"), "got: {wrapped}");
        // Header wire shape: array of [k, v] pairs (16f),
        // not the slice 16c flat object.
        assert!(wrapped.contains("__tspHeaders__.push([__k__, __v__])"), "got: {wrapped}");
    }

    #[test]
    fn wrap_envelope_inspects_response_and_string() {
        // The envelope contains both an instanceof Response
        // branch and a typeof string branch. A page that
        // returns either one produces a well-formed envelope;
        // anything else throws at run time.
        let body = "function GET() { return new Response(\'hi\', { status: 201 }); }\n";
        let wrapped = wrap_for_bun_cli(body, "GET", None);
        assert!(wrapped.contains("instanceof Response"));
        assert!(wrapped.contains("typeof __tspResult__ === \'string\'"));
        assert!(wrapped.contains("page returned invalid value"));
    }
}
