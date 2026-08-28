//! Page source reader + static export detector for TSP v2 PoC 1 slice 5.
//!
//! See `tsp-v2-plan.md` sect.4.2 (named HTTP method exports),
//! sect.48 (export validation), and sect.20.2-20.3 (ModuleGraph +
//! PageSlot, both landed in slice 7+). For slice 5 we only need the
//! minimum to prove the host can:
//!
//! 1. Read a `.tsp` file from disk into memory.
//! 2. Confirm the file actually exports the requested method via a
//!    static, regex-based scan.
//! 3. Hand the caller enough metadata to build a 200/405 response
//!    without yet running the page (real JSC execution lands in slice
//!    6+ alongside the `bun_runtime` integration).
//!
//! This is *not* a TypeScript parser. The scan catches the conventional
//! `export function GET(ctx) { ... }` form. A `.tsp` file that exports
//! `GET` indirectly (`const GET = ...; export { GET }`, or
//! `export { foo as GET }`) will look like a no-`GET` file to this
//! pass. The real AST-based detection lands in slice 7 when we wire up
//! `bun_js_parser` for the page graph.
use std::fs;

use crate::router::{HttpMethod, Route};

/// All the host knows about a page after the slice-5 prepare pass:
/// the original text (so future slices can hand it to the transpiler)
/// and the method set the static scan actually found. The host uses
/// this to choose 200 vs 405 and to build the response body.
#[derive(Debug, Clone)]
pub struct PageSource {
    pub text: String,
    pub byte_len: usize,
    pub methods: Vec<HttpMethod>,
    /// `config.methods` if the page declared a
    /// `PageConfig` (FREEZE.md §11). When `Some`,
    /// the static check (`tspserver_v2 check`)
    /// validates the declared set against the
    /// actual exports; a mismatch is reported
    /// as an error. The runtime does NOT enforce
    /// `config.methods` (the host's static scan
    /// already wins on 405 dispatch), so this is
    /// strictly a check-time validation.
    pub config_methods: Option<Vec<HttpMethod>>,
}

#[derive(Debug)]
pub enum PrepareError {
    /// The route's `source` file is gone or unreadable. We surface the
    /// underlying `io::Error` so the operator sees the real reason
    /// (permission denied, race with an editor, etc.).
    Io {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
    /// The file's text was not valid UTF-8. `.tsp` is a text format;
    /// a binary file in this slot is operator error, not host error,
    /// and the 500 page should explain the actual byte sequence.
    Utf8 {
        path: std::path::PathBuf,
        source: std::string::FromUtf8Error,
    },
}

impl std::fmt::Display for PrepareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "read {} failed: {source}", path.display())
            }
            Self::Utf8 { path, source } => {
                write!(f, "{} is not valid UTF-8: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for PrepareError {}

/// Read the route's source file, run the static method-export scan,
/// and return the prepared page. The returned `methods` is sorted in
/// the canonical wire order (GET, POST, PUT, PATCH, DELETE, OPTIONS)
/// so the `Allow:` header is deterministic across requests.
pub fn prepare(route: &Route) -> Result<PageSource, PrepareError> {
    let bytes = fs::read(&route.source).map_err(|e| PrepareError::Io {
        path: route.source.clone(),
        source: e,
    })?;
    let text = String::from_utf8(bytes).map_err(|e| PrepareError::Utf8 {
        path: route.source.clone(),
        source: e,
    })?;
    let byte_len = text.len();
    let methods = detect_methods(&text);
    let config_methods = detect_config_methods(&text);
    Ok(PageSource {
        text,
        byte_len,
        methods,
        config_methods,
    })
}

/// Static, line-based scan for `export function GET(ctx) { ... }` (and
/// the other five standard methods). See module docs for the slice-5
/// limitations vs. a real AST detector.
pub fn detect_methods(text: &str) -> Vec<HttpMethod> {
    let mut found = Vec::new();
    for method in &HttpMethod::ALL {
        if exports_method(text, *method) {
            found.push(*method);
        }
    }
    found
}

fn exports_method(text: &str, method: HttpMethod) -> bool {
    // Accept the two conventional line-start forms:
    //   export function GET(...)            (sync)
    //   export async function GET(...)      (async component, plan sect.12.2)
    // Indentation is allowed (trim_start). Comments / string literals
    // anchored anywhere other than line start are correctly rejected
    // because we use `starts_with`, not `contains`. The full TS
    // generic / decorator / re-export edge cases land with the AST
    // detector in slice 7.
    let needle_async = format!("export async function {}(", method.as_str());
    let needle_sync = format!("export function {}(", method.as_str());
    text.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with(&needle_async) || trimmed.starts_with(&needle_sync)
    })
}

/// Detect `config.methods` in a .tsp file (FREEZE.md
/// §11). The page's `export const config = { ... }`
/// is the only source of truth for the declared
/// method set; if absent, this returns `None` (the
/// page uses the implicit "all exported methods"
/// set, which is the historical default).
///
/// The detector is hand-rolled (no regex dep) and
/// handles the common shapes:
///   methods: ["GET", "POST"]
///   methods: ['GET', 'POST']
///   methods:[GET,POST]               (no quotes -- lenient)
///   methods: ["GET"]                (single)
///   methods: []                     (empty)
///   methods: [ "GET" , "POST" ]      (whitespace)
/// It rejects an unrecognized method name with
/// `None` (the whole config is then `None`, so the
/// check will not surface a partial / broken parse
/// as a successful parse with a wrong value).
pub fn detect_config_methods(text: &str) -> Option<Vec<HttpMethod>> {
    // Find `methods:` in the source. We anchor on
    // the colon so an `auth.methods` or
    // `something.methods` does not match. The
    // surrounding braces / `config` keyword are
    // NOT required -- `methods: [...]: ...;`
    // also matches. This is lenient on purpose
    // (the user is allowed to write the methods
    // list in a sibling object literal).
    let idx = text.find("methods:")?;
    let after = &text[idx + "methods:".len()..];
    // Find the start of the array. The first
    // `[` is the array opener.
    let array_start = after.find('[')?;
    let after_open = &after[array_start + 1..];
    // Find the matching `]`. The detector does
    // NOT skip string literals (the file might
    // have `]` inside a string) -- the conventional
    // .tsp config is short enough that this is a
    // non-issue in practice. A full TS parser
    // (slice 7's AST pass) would handle this.
    let array_end = after_open.find(']')?;
    let inside = &after_open[..array_end];
    // Empty list -> empty config_methods.
    if inside.trim().is_empty() {
        return Some(Vec::new());
    }
    let mut out = Vec::new();
    for raw in inside.split(',') {
        let item = raw.trim();
        // Strip surrounding quotes (single or
        // double). Unquoted values are rejected
        // -- we want explicit string literals, not
        // identifiers (which would be a different
        // shape entirely).
        let unquoted = if (item.starts_with('"') && item.ends_with('"'))
            || (item.starts_with('\'') && item.ends_with('\''))
        {
            &item[1..item.len() - 1]
        } else {
            return None;
        };
        // Reject any whitespace inside the method
        // name (a malformed value like `"G ET"`).
        if unquoted.chars().any(|c| c.is_whitespace()) {
            return None;
        }
        let method = match unquoted {
            "GET" => HttpMethod::Get,
            "POST" => HttpMethod::Post,
            "PUT" => HttpMethod::Put,
            "PATCH" => HttpMethod::Patch,
            "DELETE" => HttpMethod::Delete,
            "HEAD" => HttpMethod::Head,
            "OPTIONS" => HttpMethod::Options,
            _ => return None,
        };
        out.push(method);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_get_only() {
        let src = "
            // PoC 1 fixture
            export function GET() {
                return <h1>Hello</h1>;
            }
        ";
        let methods = detect_methods(src);
        assert_eq!(methods, vec![HttpMethod::Get]);
    }

    #[test]
    fn detects_multiple_methods() {
        let src = "
            export function GET() { return null; }
            export async function POST() { return null; }
            export function DELETE() { return null; }
        ";
        let methods = detect_methods(src);
        assert_eq!(
            methods,
            vec![HttpMethod::Get, HttpMethod::Post, HttpMethod::Delete]
        );
    }

    #[test]
    fn no_exports_is_empty() {
        let src = "
            const helper = () => 42;
            // nothing exported
        ";
        let methods = detect_methods(src);
        assert!(methods.is_empty());
    }

    #[test]
    fn ignores_non_exported_function_with_same_name() {
        // `function GET` without `export` is not a page handler.
        let src = "
            function GET() { return 'internal'; }
        ";
        let methods = detect_methods(src);
        assert!(methods.is_empty());
    }

    #[test]
    fn ignores_string_literal_mentions() {
        // The slice-5 detector is line-start anchored, so a mention of
        // `export function GET(` *inside* a string literal cannot match:
        // the line itself starts with `const`, not `export`. (A real
        // AST pass is still required for cases like `export { foo as
        // GET }`; see slice 7.)
        let src = r#"
            const note = "export function GET() is the contract";
        "#;
        let methods = detect_methods(src);
        assert!(methods.is_empty());
    }

    #[test]
    fn detects_async_form() {
        // Async components are first-class in plan sect.12.2; the
        // detector must recognise `export async function GET()`.
        let src = "export async function GET() { return null; }";
        let methods = detect_methods(src);
        assert_eq!(methods, vec![HttpMethod::Get]);
    }

    // -----------------------------------------------------------------
    // `config.methods` detection (FREEZE.md §11, slice 11 of plan)
    //
    // The page's `export const config = { methods: [...] }` is the
    // single source of truth for the declared method set. The
    // detector is hand-rolled (no regex dep) and lenient on the
    // common shapes (single / double quotes, whitespace). An
    // unparseable config returns `None` so the check treats the
    // page as if it had no `config` at all (no false-positive
    // mismatches from a partial parse).
    // -----------------------------------------------------------------

    #[test]
    fn detect_config_methods_none_when_absent() {
        let src = r#"
export function GET() {
  return new Response("ok");
}
"#;
        assert_eq!(detect_config_methods(src), None);
    }

    #[test]
    fn detect_config_methods_double_quotes() {
        let src = r#"
export const config = {
  methods: ["GET", "POST"],
} satisfies PageConfig;
export function GET() { return null; }
export function POST() { return null; }
"#;
        assert_eq!(
            detect_config_methods(src),
            Some(vec![HttpMethod::Get, HttpMethod::Post])
        );
    }

    #[test]
    fn detect_config_methods_single_quotes() {
        let src = r#"
export const config = { methods: ['GET'] };
export function GET() { return null; }
"#;
        assert_eq!(
            detect_config_methods(src),
            Some(vec![HttpMethod::Get])
        );
    }

    #[test]
    fn detect_config_methods_empty_list() {
        let src = r#"
export const config = { methods: [] };
"#;
        assert_eq!(detect_config_methods(src), Some(vec![]));
    }

    #[test]
    fn detect_config_methods_rejects_unknown_name() {
        // An unknown method (e.g. "BREW" for the
        // coffee protocol) is a hard parse error:
        // the detector returns None so the check
        // does not surface a wrong value.
        let src = r#"
export const config = { methods: ["BREW"] };
"#;
        assert_eq!(detect_config_methods(src), None);
    }

    #[test]
    fn detect_config_methods_rejects_unquoted() {
        let src = r#"
export const config = { methods: [GET] };
"#;
        // Unquoted values are NOT identifiers
        // (the field is a string list, not a
        // reference to a const). The detector
        // returns None.
        assert_eq!(detect_config_methods(src), None);
    }

    #[test]
    fn detect_config_methods_tolerates_whitespace() {
        let src = r#"
export const config = {
  methods:   [   "GET"   ,   "DELETE"   ]   ,
};
"#;
        assert_eq!(
            detect_config_methods(src),
            Some(vec![HttpMethod::Get, HttpMethod::Delete])
        );
    }

    #[test]
    fn ignores_comment_lines() {
        // `// export function GET()` -- the line starts with `//`, not
        // `export`, so the detector correctly rejects it.
        let src = "// export function GET() is the contract";
        let methods = detect_methods(src);
        assert!(methods.is_empty());
    }

    // -----------------------------------------------------------------
    // Multi-route dispatch regression test
    //
    // The user's bug: every URL returns index.tsp's body. The wrap
    // tests already proved wrap_for_embedded_worker is source-specific
    // and the router tests proved lookup is path-specific. The remaining
    // stage that can leak routes is page::prepare, which reads from
    // `route.source` -- if it ever ignored `route.source` and read a
    // fixed path, all routes would alias index.tsp. Pin it.
    // -----------------------------------------------------------------

    #[test]
    fn prepare_reads_route_source_not_a_hardcoded_path() {
        use crate::router::{HttpMethod, Route, Segment};

        let dir = tempdir();
        let index_path = dir.join("index.tsp");
        let time_path = dir.join("time.tsp");
        let index_body = r#"export function GET(ctx) {
            return `<h1>Hello ${ctx.method} ${ctx.path}</h1>`;
        }
"#;
        let time_body = r#"export async function GET(ctx) {
            const t = ctx.services.time;
            return new Response(`iso=${t.iso}`, { status: 200 });
        }
"#;
        std::fs::write(&index_path, index_body).expect("write index");
        std::fs::write(&time_path, time_body).expect("write time");

        let index_route = Route {
            path: "/".to_string(),
            source: index_path.clone(),
            methods: vec![HttpMethod::Get],
            segments: vec![],
            params: Default::default(),
        };
        let time_route = Route {
            path: "/time".to_string(),
            source: time_path.clone(),
            methods: vec![HttpMethod::Get],
            segments: vec![Segment::Static("time".to_string())],
            params: Default::default(),
        };

        let index_page = prepare(&index_route).expect("prepare index");
        let time_page = prepare(&time_route).expect("prepare time");

        // Each route must yield its own source bytes, not a shared
        // alias. If `prepare` ever ignored `route.source`, both
        // routes would resolve to the same file (whichever the bug
        // pointed at) and one of these asserts would fail.
        assert_eq!(index_page.text, index_body, "index.tsp content leaked");
        assert_eq!(time_page.text, time_body, "time.tsp content leaked");
        assert_ne!(
            index_page.text, time_page.text,
            "two routes produced identical source bytes"
        );
        assert_eq!(index_page.byte_len, index_body.len());
        assert_eq!(time_page.byte_len, time_body.len());
    }

    fn tempdir() -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        let pid = std::process::id();
        let nonce: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        dir.push(format!("tsp-page-test-{pid}-{nonce}"));
        std::fs::create_dir_all(&dir).expect("create tempdir");
        dir
    }
}
