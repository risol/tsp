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
}

#[derive(Debug)]
pub enum PrepareError {
    /// The route's `source` file is gone or unreadable. We surface the
    /// underlying `io::Error` so the operator sees the real reason
    /// (permission denied, race with an editor, etc.).
    Io { path: std::path::PathBuf, source: std::io::Error },
    /// The file's text was not valid UTF-8. `.tsp` is a text format;
    /// a binary file in this slot is operator error, not host error,
    /// and the 500 page should explain the actual byte sequence.
    Utf8 { path: std::path::PathBuf, source: std::string::FromUtf8Error },
}

impl std::fmt::Display for PrepareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(f, "read {} failed: {source}", path.display())
            }
            Self::Utf8 { path, source } => write!(
                f,
                "{} is not valid UTF-8: {source}",
                path.display()
            ),
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
    Ok(PageSource {
        text,
        byte_len,
        methods,
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

    #[test]
    fn ignores_comment_lines() {
        // `// export function GET()` -- the line starts with `//`, not
        // `export`, so the detector correctly rejects it.
        let src = "// export function GET() is the contract";
        let methods = detect_methods(src);
        assert!(methods.is_empty());
    }
}