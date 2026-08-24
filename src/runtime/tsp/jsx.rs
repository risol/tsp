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
pub fn wrap_for_bun_cli(
    transformed: &str,
    method: &str,
    ctx_json: Option<&str>,
) -> String {
    let mut out = String::with_capacity(transformed.len() + 512);
    out.push_str("// Generated by TSP v2 PoC 1 slice 16a (jsx.rs)\n");
    out.push_str("// Transformed .tsp -> runnable .js for `bun run tempfile`.\n");
    out.push_str("// Do not edit by hand; the host regenerates this on every request.\n");
    out.push_str("const __tsp_console_log__ = globalThis.console ? globalThis.console.log : (...a) => print(...a);\n");
    if let Some(json) = ctx_json {
        // Inject the Context literal inline so the page does not
        // depend on bun's process.env being readable. The host
        // sets the same env var for any code that wants it
        // (process.env.TSP_CONTEXT_JSON); the page just uses the
        // const for convenience. Embedding the JSON as a
        // literal keeps the contract honest -- if the host
        // changes Context shape, the literal embedded here
        // changes too, and a misaligned const fails the
        // build instead of silently mis-rendering.
        let escaped = json.replace('\\', "\\\\").replace('`', "\\`");
        let _ = write!(
            out,
            "const __tspContext = JSON.parse({json:?});\n",
            json = escaped
        );
        let _ = write!(
            out,
            "if (typeof {method} !== 'function') {{\n  throw new Error('page does not export function {method}()');\n}}\n__tsp_console_log__({method}(__tspContext));\n"
        );
    } else {
        let _ = write!(
            out,
            "if (typeof {method} !== 'function') {{\n  throw new Error('page does not export function {method}()');\n}}\n__tsp_console_log__({method}());\n"
        );
    }
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
        assert!(wrapped.contains("__tsp_console_log__(GET());"));
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
        assert!(wrapped.contains(r#"\"method\":\"GET\""#));
        assert!(wrapped.contains("__tsp_console_log__(GET(__tspContext));"));
    }
}
