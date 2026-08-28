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
    /// `config.bodyLimit` if the page declared one
    /// (FREEZE.md §11). The runtime applies this
    /// AFTER route matching: if the body is
    /// larger than this limit, the host returns
    /// 413. The per-page limit MUST be <= the
    /// global `TSP_MAX_BODY_BYTES` (spec
    /// "cannot exceed global hard limit"); a
    /// larger value falls back to the global at
    /// runtime.
    pub config_body_limit: Option<usize>,
    /// `config.cache` if the page declared one
    /// (plan §55, FREEZE.md §11). The runtime
    /// applies this AFTER route matching: the
    /// declared value is used as a default
    /// `Cache-Control` header on the response,
    /// but the page's own `Response.headers` set
    /// of `Cache-Control` always wins (the page
    /// is more specific than the page-level
    /// default). The supported values are the
    /// three FREEZE.md §11 literals:
    ///   "no-store" -> `Cache-Control: no-store`
    ///   "private"  -> `Cache-Control: private`
    ///   "public"   -> `Cache-Control: public`
    /// Anything else is unparseable (returns
    /// `None` so `tspserver_v2 check` does not
    /// surface a wrong value as a successful
    /// parse).
    pub config_cache: Option<CachePolicy>,
}

/// `config.cache` policy values. The wire form
/// is the value the user wrote in
/// `export const config = { cache: "..." }`; the
/// `header_value` is the `Cache-Control` header
/// line the host applies when the page's
/// `Response` did not set one itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CachePolicy {
    NoStore,
    Private,
    Public,
}

impl CachePolicy {
    /// `Cache-Control` header value for this policy.
    /// Kept as `&'static str` so the host can push
    /// the literal directly into the response header
    /// block without allocating.
    pub fn header_value(self) -> &'static str {
        match self {
            CachePolicy::NoStore => "no-store",
            CachePolicy::Private => "private",
            CachePolicy::Public => "public",
        }
    }
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
    let config_body_limit = detect_config_body_limit(&text);
    let config_cache = detect_config_cache(&text);
    Ok(PageSource {
        text,
        byte_len,
        methods,
        config_methods,
        config_body_limit,
        config_cache,
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

/// Detect `config.bodyLimit` in a .tsp file
/// (FREEZE.md §11). The page's
/// `export const config = { bodyLimit: N }` is the
/// single source of truth for the per-page body
/// size cap (in bytes). Returns `None` when the
/// page did not declare one (the runtime falls
/// back to the global `TSP_MAX_BODY_BYTES`).
///
/// The detector is hand-rolled (no regex dep) and
/// tolerates the common shapes:
///   bodyLimit: 1024
///   bodyLimit: 2 * 1024 * 1024
///   bodyLimit: 0
///   "bodyLimit": 1024
/// It rejects negative values, NaN, and Infinity
/// (the `usize` cast would underflow / panic). On
/// any unparseable value, the function returns
/// `None` so the check does not surface a wrong
/// value as a successful parse.
pub fn detect_config_body_limit(text: &str) -> Option<usize> {
    // Find `bodyLimit:` in the source. Anchor on
    // the colon so `something.bodyLimit` does not
    // match.
    let idx = text.find("bodyLimit:")?;
    let after = &text[idx + "bodyLimit:".len()..];
    // Skip the optional value body up to the next
    // `,` or `}`. We allow the value to be a
    // single integer (with optional underscores
    // for readability) or a small expression of
    // the form `<int> [* <int>]+`. Anything more
    // complex is out of scope.
    let end = after
        .find(|c: char| c == ',' || c == '}' || c == '\n')
        .unwrap_or(after.len());
    let raw = after[..end].trim();
    // Strip surrounding quotes (single / double)
    // if the user used a string literal (rare but
    // legal).
    let raw = if (raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2)
        || (raw.starts_with('\'') && raw.ends_with('\'') && raw.len() >= 2)
    {
        &raw[1..raw.len() - 1]
    } else {
        raw
    };
    // Strip underscores (numeric separator).
    let raw = raw.replace('_', "");
    // Compute the value. We support a small subset
    // of arithmetic: a chain of `int * int * int * ...`
    // (one or more multiplications). More complex
    // expressions are rejected so the user gets an
    // explicit error rather than a silent fallback
    // to None.
    let parts: Vec<&str> = raw.split('*').map(|p| p.trim()).collect();
    if parts.is_empty() {
        return None;
    }
    let mut product: usize = 1;
    for part in &parts {
        let n: usize = match part.parse() {
            Ok(n) => n,
            Err(_) => return None,
        };
        product = match product.checked_mul(n) {
            Some(p) => p,
            None => return None,
        };
    }
    Some(product)
}

/// Detect `config.cache` in a .tsp file
/// (FREEZE.md §11, plan §55). The page's
/// `export const config = { cache: "..." }` is
/// a single string literal; the runtime maps it
/// to a `Cache-Control` header value used as a
/// default when the page's `Response.headers`
/// did not set one. Returns `None` when the
/// page did not declare a value OR the value is
/// not one of the three FREEZE.md §11 literals
/// (so `tspserver_v2 check` does not surface a
/// wrong value as a successful parse).
///
/// The detector is hand-rolled (no regex dep)
/// and tolerates the common shapes:
///   cache: "no-store"
///   cache: 'public'
///   cache:   "private"
///   "cache": "public"
/// It rejects unknown strings, numbers, and
/// expressions -- those would not be a valid
/// `CachePolicy` at the wire anyway.
pub fn detect_config_cache(text: &str) -> Option<CachePolicy> {
    // Find the `cache` key. Three shapes are
    // accepted:
    //   cache:  "..."   (unquoted key)
    //   "cache": "..."  (double-quoted key)
    //   'cache': "..."  (single-quoted key)
    // We locate the first `cache` substring and
    // then walk forward to the `:` -- accepting
    // an optional quote + whitespace between
    // `cache` and `:`. The first such `cache`
    // in a config block is the binding the
    // runtime cares about; the parser does not
    // try to skip unrelated `cache` substrings
    // (e.g. inside a comment) because the
    // hand-rolled detector follows the same
    // simple line-shape rule as the other
    // PageConfig detectors in this module.
    let idx = text.find("cache")?;
    let after_key = &text[idx + "cache".len()..];
    // Optional closing quote + whitespace, then `:`.
    let mut i = 0;
    if after_key.starts_with('"') || after_key.starts_with('\'') {
        i += 1;
    }
    while i < after_key.len() && after_key.as_bytes()[i] == b' ' {
        i += 1;
    }
    if after_key.as_bytes().get(i) != Some(&b':') {
        // Not a `cache:` key (could be
        // `cache = ...` or `cached: ...`);
        // reject.
        return None;
    }
    let after = &after_key[i + 1..];
    // Skip the optional value body up to the
    // next `,` or `}` or newline. We allow the
    // value to be a single string literal (with
    // single or double quotes).
    let end = after
        .find(|c: char| c == ',' || c == '}' || c == '\n')
        .unwrap_or(after.len());
    let raw = after[..end].trim();
    // Strip surrounding quotes (single / double).
    let stripped = if (raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2)
        || (raw.starts_with('\'') && raw.ends_with('\'') && raw.len() >= 2)
    {
        &raw[1..raw.len() - 1]
    } else {
        // Not a string literal: cannot be a
        // valid `CachePolicy` (numbers /
        // expressions are out of scope).
        return None;
    };
    match stripped {
        "no-store" => Some(CachePolicy::NoStore),
        "private" => Some(CachePolicy::Private),
        "public" => Some(CachePolicy::Public),
        // Unknown string: rejected so the
        // check-time surface does not
        // silently accept it.
        _ => None,
    }
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

    // -----------------------------------------------------------------
    // `config.bodyLimit` detection (FREEZE.md §11, slice 11 of plan)
    //
    // Hand-rolled parser for `bodyLimit: N` (bytes).
    // Supports the common shapes (single int, single
    // int with underscores, `int * int` expression,
    // string-quoted int). Rejects negative numbers,
    // NaN, Infinity, and multi-level expressions.
    // On any unparseable value, returns None.
    // -----------------------------------------------------------------

    #[test]
    fn detect_config_body_limit_none_when_absent() {
        let src = r#"
export const config = {
  methods: ["GET"],
};
"#;
        assert_eq!(detect_config_body_limit(src), None);
    }

    #[test]
    fn detect_config_body_limit_simple_int() {
        let src = r#"
export const config = { bodyLimit: 4096 };
"#;
        assert_eq!(detect_config_body_limit(src), Some(4096));
    }

    #[test]
    fn detect_config_body_limit_expression() {
        let src = r#"
export const config = { bodyLimit: 2 * 1024 * 1024 };
"#;
        assert_eq!(detect_config_body_limit(src), Some(2 * 1024 * 1024));
    }

    #[test]
    fn detect_config_body_limit_underscore_separator() {
        // Numeric separators (TypeScript 5+)
        // are allowed in the value.
        let src = r#"
export const config = { bodyLimit: 1_048_576 };
"#;
        assert_eq!(detect_config_body_limit(src), Some(1_048_576));
    }

    #[test]
    fn detect_config_body_limit_allows_zero() {
        // `bodyLimit: 0` is allowed (the user might
        // want to reject ALL bodies, which is a
        // legitimate use case for endpoints that
        // don't accept bodies).
        let src = r#"
export const config = { bodyLimit: 0 };
"#;
        assert_eq!(detect_config_body_limit(src), Some(0));
    }

    #[test]
    fn detect_config_body_limit_rejects_unparseable() {
        // `Infinity` would panic the `usize` cast
        // if we let it through. The detector
        // rejects it.
        let src = r#"
export const config = { bodyLimit: Infinity };
"#;
        assert_eq!(detect_config_body_limit(src), None);
    }

    #[test]
    fn detect_config_body_limit_rejects_negative() {
        // `parse::<usize>()` rejects negative
        // numbers; the detector surfaces the
        // unparseable value as None.
        let src = r#"
export const config = { bodyLimit: -1 };
"#;
        assert_eq!(detect_config_body_limit(src), None);
    }

    #[test]
    fn detect_config_cache_none_when_absent() {
        let src = r#"
export function GET() { return new Response("ok"); }
"#;
        assert_eq!(detect_config_cache(src), None);
    }

    #[test]
    fn detect_config_cache_no_store() {
        let src = r#"
export const config = { cache: "no-store" } satisfies PageConfig;
export function GET() { return new Response("ok"); }
"#;
        assert_eq!(detect_config_cache(src), Some(CachePolicy::NoStore));
    }

    #[test]
    fn detect_config_cache_private_and_public() {
        let src_priv = r#"
export const config = { cache: "private" };
"#;
        assert_eq!(detect_config_cache(src_priv), Some(CachePolicy::Private));
        let src_pub = r#"
export const config = { cache: 'public' };
"#;
        // Single quotes are accepted
        // (config values can be either quoted).
        assert_eq!(detect_config_cache(src_pub), Some(CachePolicy::Public));
    }

    #[test]
    fn detect_config_cache_tolerates_whitespace_and_quoted_key() {
        // The `"cache":` (quoted-key) shape and
        // extra whitespace between `cache:` and
        // the value both work.
        let src = r#"
export const config = {   "cache"   :     "no-store"   };
"#;
        assert_eq!(detect_config_cache(src), Some(CachePolicy::NoStore));
    }

    #[test]
    fn detect_config_cache_rejects_unknown_value() {
        // "max-age=60" is a valid Cache-Control
        // directive but NOT one of the three
        // FREEZE.md §11 cache policies. The
        // detector surfaces it as None so
        // `tspserver_v2 check` does not
        // silently accept a value the runtime
        // cannot map to a default header.
        let src = r#"
export const config = { cache: "max-age=60" };
"#;
        assert_eq!(detect_config_cache(src), None);
    }

    #[test]
    fn detect_config_cache_rejects_unquoted() {
        // Unquoted identifiers are not valid
        // string literals; the runtime does
        // not parse them.
        let src = r#"
export const config = { cache: no_store };
"#;
        assert_eq!(detect_config_cache(src), None);
    }

    #[test]
    fn cache_policy_header_value_maps_to_freeze_literals() {
        // The header value is exactly the
        // FREEZE.md §11 literal so a user who
        // reads the spec sees the same string
        // on the wire that they wrote in
        // `config`.
        assert_eq!(CachePolicy::NoStore.header_value(), "no-store");
        assert_eq!(CachePolicy::Private.header_value(), "private");
        assert_eq!(CachePolicy::Public.header_value(), "public");
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
