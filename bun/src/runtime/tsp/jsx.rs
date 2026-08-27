//! Minimal TSX -> JS pre-processor for TSP v2 PoC 1 slice 6.
//!
//! See `tsp-v2-plan.md` sect.11 (TSP JSX runtime) and sect.3.1
//! (`.tsp` is standard TSX). The full JSX -> HtmlNode pipeline (the
//! spec's `tsp:jsx-runtime` + HtmlNode ABI) lands in slice 7+; for
//! slice 6 we only need to land a "the host can turn a `.tsp` into
//! a runnable JS string" path that is honest about its scope. The
//! transform below:
//!
//! - rewrites the narrow named `tsp:server` compatibility imports into
//!   bindings supplied by the generated wrapper;
//! - leaves TypeScript/TSX parsing to Bun's subprocess transpiler. The
//!   generated file uses a `.tsx` suffix, and the wrapper provides the
//!   small React-compatible element factory needed by Bun's classic JSX
//!   lowering. The wrapper then renders the element tree to HTML.
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

/// Transform a `.tsp` source string into a runnable TSX module body for
/// `bun run tempfile`. Bun performs the actual TypeScript/TSX lowering.
pub fn tsx_to_js(source: &str) -> Result<String, JsxError> {
    let source = rewrite_tsp_server_imports(source)?;
    Ok(rewrite_fragment_exports(&source))
}

/// Transform relative local imports into a form the synthetic `bun:main`
/// wrap can load.
///
/// **Why not keep the `import` statement?** The wrap preamble (`jsx::wrap_for_bun_cli`)
/// prepends a non-trivial prelude (nanoid, zod, bcrypt, `require("bun").SQL`,
/// `__tspServer`, ctx hydration, fragment registry, etc.) before the page
/// body. The page's `import { x } from "./y"` therefore lands **mid-file**
/// in the generated module. Bun's synthetic `bun:main` entry evaluates the
/// file as a single module, and a mid-file `import` is invalid in both CJS
/// and ESM contexts. Empirically (slice 17d) the wrap loads successfully
/// but the IIFE that calls the page handler never resolves, and the worker
/// poll loop times out with "embedded TSP wrapper did not produce a response".
///
/// **The fix**: rewrite `import { a, b as c } from "./y"` into
/// `const { a, b: c } = require("file:///...y?tsp_generation=N");` and
/// `import("./y")` into `require("file:///...y?tsp_generation=N")`. The
/// page keeps its `import` syntax (no page-side changes); the rewriter
/// turns it into a CommonJS `require` so the page's relative import lands
/// at a position in the module where a `const` / expression is legal.
///
/// `.tsp` is intentionally rejected as an import target; route modules are
/// entry points, not reusable library modules.
pub fn rewrite_local_imports(
    source: &str,
    importer_dir: &std::path::Path,
) -> Result<String, JsxError> {
    rewrite_local_imports_at_generation(source, importer_dir, None)
}

/// Rewrite local imports with a generation query. Bun's module cache keys
/// file URLs, so a new query makes changed dependencies load into the
/// persistent worker without restarting the master or creating a Bun child.
pub fn rewrite_local_imports_for_generation(
    source: &str,
    importer_dir: &std::path::Path,
    generation: u64,
) -> Result<String, JsxError> {
    rewrite_local_imports_at_generation(source, importer_dir, Some(generation))
}

fn rewrite_local_imports_at_generation(
    source: &str,
    importer_dir: &std::path::Path,
    generation: Option<u64>,
) -> Result<String, JsxError> {
    let mut out = String::with_capacity(source.len());
    for (line_no, line) in source.lines().enumerate() {
        let mut rewritten = line.to_string();
        for quote in ['"', '\''] {
            let marker = format!("from {quote}");
            rewritten = rewrite_import_marker(
                &rewritten,
                &marker,
                quote,
                importer_dir,
                line_no + 1,
                generation,
            )?;
            let marker = format!("import{quote}");
            rewritten = rewrite_import_marker(
                &rewritten,
                &marker,
                quote,
                importer_dir,
                line_no + 1,
                generation,
            )?;
            rewritten =
                rewrite_dynamic_import(&rewritten, quote, importer_dir, line_no + 1, generation)?;
        }
        out.push_str(&rewritten);
        out.push('\n');
    }
    Ok(out)
}

/// Rewrite a dynamic `import("./y")` expression into a synchronous
/// `require("file:///...y?tsp_generation=N")`. Dynamic imports are
/// expressions, so they can appear mid-file in any module mode; the
/// `require` substitution keeps the call site and its surrounding
/// parentheses / arguments intact.
fn rewrite_dynamic_import(
    line: &str,
    quote: char,
    importer_dir: &std::path::Path,
    line_no: usize,
    generation: Option<u64>,
) -> Result<String, JsxError> {
    let marker = format!("import({quote}");
    let Some(start) = line.find(&marker) else {
        return Ok(line.to_string());
    };
    let spec_start = start + marker.len();
    let Some(end_rel) = line[spec_start..].find(quote) else {
        return Ok(line.to_string());
    };
    let end = spec_start + end_rel;
    let specifier = &line[spec_start..end];
    if !specifier.starts_with('.') {
        return Ok(line.to_string());
    }
    if specifier.ends_with(".tsp") {
        return Err(JsxError::UnsupportedShape {
            line: line_no,
            reason: "route .tsp modules cannot be imported; move shared code to .ts/.tsx",
        });
    }
    let Some(path) = resolve_local_module(importer_dir, specifier) else {
        return Err(JsxError::UnsupportedShape {
            line: line_no,
            reason: "local dynamic import could not be resolved",
        });
    };
    let url = file_url(&path, generation);
    // `import("...")` -> `require("file://...")`. We keep the same
    // span (everything up to and including the closing `)` of the call).
    let mut result = String::with_capacity(line.len() + 8 + url.len());
    result.push_str(&line[..start]);
    result.push_str("require(\"");
    result.push_str(&url);
    result.push_str("\")");
    // The original `import(...)` is closed by the `)` that comes after
    // the specifier; we've already emitted that via the url-quote-quote
    // suffix. We must skip the `)` in the original line -- so drop
    // `end` (specifier close quote) AND the character right after
    // (which is the `)`).
    let after = if end < line.len() { &line[end + 1..] } else { "" };
    // The `after` slice starts with the character following the spec's
    // closing quote. For `import("...")`, that's `)`. We want to keep
    // it (the `require("...")` is the call, no extra `)` needed).
    result.push_str(after);
    Ok(result)
}

/// Rewrite a static `import { a, b as c } from "./y"` declaration into a
/// `const { a, b: c } = require("file:///...y?...")`. The named-binding
/// shape is the only one the v2 rewriter accepts (plan §16.1:
/// `import { x }` is the supported named-import form). The transformation
/// preserves `as` renames (`a as b` -> `a: b` in the destructure).
fn rewrite_import_marker(
    line: &str,
    marker: &str,
    quote: char,
    importer_dir: &std::path::Path,
    line_no: usize,
    generation: Option<u64>,
) -> Result<String, JsxError> {
    let Some(start) = line.find(marker) else {
        return Ok(line.to_string());
    };
    let spec_start = start + marker.len();
    let Some(end_rel) = line[spec_start..].find(quote) else {
        return Ok(line.to_string());
    };
    let end = spec_start + end_rel;
    let specifier = &line[spec_start..end];
    if !specifier.starts_with('.') {
        return Ok(line.to_string());
    }
    if specifier.ends_with(".tsp") {
        return Err(JsxError::UnsupportedShape {
            line: line_no,
            reason: "route .tsp modules cannot be imported; move shared code to .ts/.tsx",
        });
    }
    let Some(path) = resolve_local_module(importer_dir, specifier) else {
        return Err(JsxError::UnsupportedShape {
            line: line_no,
            reason: "local import could not be resolved",
        });
    };
    // Locate the named-binding list. The marker is `from "..."`, so the
    // `{ ... }` block must appear earlier on the same line. We find
    // the rightmost `import` keyword at-or-before `start`, then scan
    // forward for the matching `}`.
    let Some(import_kw) = line[..start].rfind("import") else {
        return Err(JsxError::UnsupportedShape {
            line: line_no,
            reason: "local import: cannot locate `import` keyword",
        });
    };
    let Some(open_rel) = line[import_kw..].find('{') else {
        return Err(JsxError::UnsupportedShape {
            line: line_no,
            reason: "local import: expected named-binding list (`{ ... }`)",
        });
    };
    let open_abs = import_kw + open_rel;
    let mut close = None;
    let mut depth = 1usize;
    for (i, ch) in line[open_abs + 1..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(open_abs + 1 + i);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(close) = close else {
        return Err(JsxError::UnsupportedShape {
            line: line_no,
            reason: "local import: unterminated named-binding list",
        });
    };
    let url = file_url(&path, generation);
    let bindings = &line[open_abs + 1..close];
    // Transform the named-binding list to a destructure pattern:
    //   `a`              -> `a`
    //   `a as b`         -> `a: b`   (ES6 destructure rename)
    //   `type X`         -> dropped (type-only, erased at runtime)
    //   trailing commas, surrounding whitespace are normalized.
    let mut destructure = String::with_capacity(bindings.len());
    for raw in bindings.split(',') {
        let item = raw.trim();
        if item.is_empty() {
            continue;
        }
        if item.starts_with("type ") {
            // type-only imports are erased at runtime; drop them.
            continue;
        }
        let mut parts = item.splitn(2, " as ");
        let imported = parts.next().unwrap_or_default().trim();
        let local = parts.next().map(str::trim).unwrap_or(imported);
        if !destructure.is_empty() {
            destructure.push_str(", ");
        }
        if imported == local {
            destructure.push_str(imported);
        } else {
            destructure.push_str(imported);
            destructure.push_str(": ");
            destructure.push_str(local);
        }
    }
    // `import { a, b as c, type D } from "./y";`
    //   ->
    // `const { a, b: c } = require("file:///...y?...");`
    let mut replacement = String::with_capacity(line.len() + 24 + url.len());
    replacement.push_str(&line[..import_kw]);
    replacement.push_str("const { ");
    replacement.push_str(&destructure);
    replacement.push_str(" } = require(\"");
    replacement.push_str(&url);
    replacement.push_str("\");");
    Ok(replacement)
}

fn resolve_local_module(
    importer_dir: &std::path::Path,
    specifier: &str,
) -> Option<std::path::PathBuf> {
    let base = importer_dir.join(specifier);
    if base.extension().and_then(|e| e.to_str()) == Some("tsp") {
        return None;
    }
    let mut candidates = vec![base.clone()];
    if base.extension().is_none() {
        for extension in ["ts", "tsx", "js", "jsx", "json"] {
            candidates.push(base.with_extension(extension));
        }
        for extension in ["ts", "tsx", "js", "jsx", "json"] {
            candidates.push(base.join(format!("index.{extension}")));
        }
    }
    candidates.into_iter().find(|path| path.is_file())
}

fn file_url(path: &std::path::Path, generation: Option<u64>) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut value = canonical.to_string_lossy().replace('\\', "/");
    // Windows `canonicalize` returns the device path form
    // `\\?\C:\...`; the leading `\\?\` is a Win32-only marker
    // that web/file-URL consumers don't understand. Strip it so
    // the URL stays a normal `file:///C:/...` (the extra `/` we
    // would otherwise add in front would produce `file:////?/C:/`
    // which no URL parser round-trips).
    if let Some(rest) = value.strip_prefix("//?/") {
        value = rest.to_string();
    }
    value = value
        .replace('%', "%25")
        .replace(' ', "%20")
        .replace('#', "#");
    let suffix = generation
        .map(|generation| format!("?tsp_generation={generation}"))
        .unwrap_or_default();
    if value.starts_with('/') {
        format!("file://{value}{suffix}")
    } else {
        format!("file:///{value}{suffix}")
    }
}

/// Give fragment handlers a runtime-visible name while keeping normal page
/// exports as ESM exports. The wrapper uses the registry when a request asks
/// for an internal fragment render.
fn rewrite_fragment_exports(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix("export const ") {
            if let Some(name_end) = rest.find(" = fragment(") {
                let name = &rest[..name_end];
                let indent = &line[..line.len() - trimmed.len()];
                out.push_str(indent);
                out.push_str("const ");
                out.push_str(name);
                out.push_str(" = fragment(\"");
                out.push_str(name);
                out.push_str("\", ");
                out.push_str(&rest[name_end + " = fragment(".len()..]);
                out.push('\n');
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Rewrite the stable named `tsp:server` import surface into bindings supplied
/// by the generated wrapper. The subprocess bridge does not run a module
/// loader for virtual `tsp:*` specifiers, so this is the narrow compatibility
/// seam until the full Bun module-loader bridge lands.
fn rewrite_tsp_server_imports(source: &str) -> Result<String, JsxError> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.starts_with("import") {
            let mut statement = lines[i].to_string();
            let mut end = i;
            let has_tsp_source = |text: &str| {
                text.contains("from \"tsp:server\"")
                    || text.contains("from 'tsp:server'")
                    || text.contains("from \"tsp:html\"")
                    || text.contains("from 'tsp:html'")
            };
            while !statement.contains(';')
                && !has_tsp_source(&statement)
                && !statement.contains(" from ")
                && !statement.contains("from ")
                && end + 1 < lines.len()
            {
                end += 1;
                statement.push_str(lines[end].trim());
            }
            if !has_tsp_source(&statement) {
                out.push_str(lines[i]);
                out.push('\n');
                i += 1;
                continue;
            }
            let is_html_source = statement.contains("tsp:html");
            let Some(open) = statement.find('{') else {
                return Err(JsxError::UnsupportedShape {
                    line: end + 1,
                    reason: "tsp:server currently supports named imports only",
                });
            };
            let Some(close) = statement.rfind('}') else {
                return Err(JsxError::UnsupportedShape {
                    line: end + 1,
                    reason: "malformed tsp:server import",
                });
            };
            let mut bindings = Vec::new();
            for raw in statement[open + 1..close].split(',') {
                let item = raw.trim();
                if item.is_empty() || item.starts_with("type ") {
                    continue;
                }
                let mut parts = item.splitn(2, " as ");
                let imported = parts.next().unwrap_or_default().trim();
                let local = parts.next().unwrap_or(imported).trim();
                let allowed = if is_html_source {
                    imported == "raw"
                } else {
                    matches!(
                        imported,
                        "json"
                            | "redirect"
                            | "text"
                            | "html"
                            | "notFound"
                            | "HttpError"
                            | "fragment"
                            | "nanoid"
                            | "customAlphabet"
                            | "customRandom"
                            | "random"
                            | "zod"
                            | "password"
                            | "sql"
                    )
                };
                if !allowed || imported.is_empty() || local.is_empty() {
                    return Err(JsxError::UnsupportedShape {
                        line: end + 1,
                        reason: "unsupported tsp:server named import",
                    });
                }
                if imported == local {
                    bindings.push(imported.to_string());
                } else {
                    bindings.push(format!("{imported}: {local}"));
                }
            }
            if !bindings.is_empty() {
                out.push_str("const { ");
                out.push_str(&bindings.join(", "));
                out.push_str(" } = __tspServer;\n");
            }
            i = end;
        } else {
            out.push_str(lines[i]);
            out.push('\n');
        }
        i += 1;
    }
    Ok(out)
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
/// Hardcoded copy of nanoid 5.1.6's `url-alphabet` constant
/// (defined in `node_modules/nanoid/url-alphabet/index.js`).
/// The page module's wrap preamble runs in a temp file with no
/// on-disk access to `node_modules/`, so the relative import
/// `from './url-alphabet/index.js'` would fail to resolve. We
/// inline the constant string instead and strip the import in
/// `nanoid_prelude` below.
const NANOID_URL_ALPHABET: &str =
    "useandom-26T198340PX75pxJACKVERYMINDBUSHWOLF_GQZbfghjklqvwyzrict";

/// The raw nanoid 5.1.6 source. Embedded into the binary at
/// compile time via `include_str!` so the wrap preamble can
/// ship a working ID generator without an `import` step. The
/// path is relative to this file (`bun/src/runtime/tsp/jsx.rs`):
/// four `..` segments walk back to the workspace root, then into
/// `node_modules/nanoid/`. If nanoid is moved out of that path
/// the build will fail with a clear `include_str!` error.
const NANOID_RAW_SOURCE: &str =
    include_str!("../../../../node_modules/nanoid/index.js");

/// Pre-bundled zod 4.4.3 (CJS, single file) compiled by
/// `bun build node_modules/zod/index.js --format=cjs --bundle --target=bun --minify`
/// at `bun/src/runtime/tsp/vendor/zod-4.4.3.cjs`. The path is
/// relative to this file (`bun/src/runtime/tsp/jsx.rs`): `vendor/`
/// lives next to the module that embeds it. The bundle keeps the
/// whole library self-contained (no `require` calls back into
/// `node_modules/zod/...` at runtime), so the wrap preamble can
/// run from a temp file with no module resolver. If the bundle is
/// regenerated (e.g. to upgrade zod), commit the new file in
/// place -- the `include_str!` hash will refresh the next
/// `cargo build`.
///
/// zod 4 is API-compatible with zod 3 for the slice 17b usage
/// (`zod.object({...})`, `zod.coerce.number().int().min(...).max(...)`,
/// `zod.string().email()`, `safeParse({success, data | error})`),
/// so no page-side changes are required when moving from 3.x to
/// 4.x. The vendor file shrunk from 138 KB to 63 KB (~55%
/// reduction) because zod 4 redesigned ZodObject generics to
/// avoid TS instantiation explosions and dropped legacy
/// compatibility shims.
///
/// Regeneration command (run from workspace root; `--target=bun`
/// is required because the bundle uses bun's CJS bridge for
/// builtin modules like `node:url`):
///
/// ```text
/// bun build node_modules/zod/index.js \
///     --format=cjs --bundle --target=bun --minify \
///     --outfile bun/src/runtime/tsp/vendor/zod-4.4.3.cjs
/// ```
///
/// Re-pin the version in `bun/package.json#zod` and `bun.lock` in
/// the same commit so the next person can re-derive the bundle
/// from the locked source.
const ZOD_RAW_SOURCE: &str = include_str!("vendor/zod-4.4.3.cjs");

// Slice 17d: the `mysql` namespace in `__tspServer` is **not** a
// pre-bundled npm library. Bun 1.3+ ships a unified `bun:sql`
// API (`Bun.SQL` / `require("bun").SQL`) that supports
// PostgreSQL, MySQL/MariaDB, and SQLite through native Rust
// drivers -- zero npm dependencies, zero prelude bytes, zero
// per-request parse cost. v2 already requires bun as the host
// runtime, so adopting the builtin keeps the single-binary
// distribution contract and avoids embedding the ~795 KB
// minified mysql2 bundle. The cost is that `sql` resolves to
// bun's pool-backed connection factory rather than a per-page
// `mysql.createConnection()` call: the page sees a fresh logical
// connection per request, but TCP-level reuse is handled by
// bun's internal pool (per-worker process, not master-held),
// which is the same trade-off PHP-FPM `pconnect` makes.

/// Build the prelude that inlines nanoid 5.1.6's runtime as
/// top-level function declarations in the page module's
/// scope. The page reaches them via `import { nanoid } from
/// "tsp:server" and `wrap_for_bun_cli` then re-exports them on
/// the frozen `__tspServer` object that the rewriter
/// destructures against.
///
/// This deliberately does NOT publish the functions on
/// `globalThis` (plan §16.4: framework API must be explicitly
/// imported or accessed via Context, not exposed on the
/// global). The names are scoped to the wrap's module; the
/// page sees them only as imports of `tsp:server`.
///
/// Three transformations are applied to the raw source:
/// 1. The `node:crypto` import is replaced with `globalThis.crypto`
///    (same webcrypto instance in Bun; avoids a module graph
///    edge the synthetic entry doesn't need).
/// 2. The relative `./url-alphabet/index.js` import and the
///    re-export are stripped; the alphabet is injected as a
///    local `const` from `NANOID_URL_ALPHABET`.
/// 3. The four `export function` declarations are reduced to
///    plain `function` so the prelude runs as top-level script
///    (the synthetic entry evaluates the wrap as a module, but
///    the `export` keyword on a per-request prelude is noise and
///    would shadow the page module's own exports).
fn nanoid_prelude() -> String {
    let mut src = NANOID_RAW_SOURCE.to_string();
    src = src.replace(
        "import { webcrypto as crypto } from 'node:crypto'",
        "const crypto = globalThis.crypto;",
    );
    src = src.replace(
        "import { urlAlphabet as scopedUrlAlphabet } from './url-alphabet/index.js'",
        &format!("const scopedUrlAlphabet = {:?};", NANOID_URL_ALPHABET),
    );
    src = src.replace(
        "export { urlAlphabet } from './url-alphabet/index.js'\n",
        "",
    );
    // All four public functions get a `__tspNanoid*` prefix so the
    // module scope keeps NO bare `nanoid`/`random`/etc. names. If we
    // left them bare, the page-side rewriter (
    // `import { nanoid } from "tsp:server"` -> `const { nanoid } =
    // __tspServer;`) would collide with the prelude's `function
    // nanoid(...)` declaration in the same module scope ("already
    // declared" SyntaxError). The `__tspServer` freeze exports them
    // under their PUBLIC names, so pages still destructure cleanly.
    src = src.replace("export function random(", "function __tspNanoidRandom(");
    src = src.replace(
        "export function customRandom(",
        "function __tspNanoidCustomRandom(",
    );
    src = src.replace(
        "export function customAlphabet(",
        "function __tspNanoidCustomAlphabet(",
    );
    src = src.replace("export function nanoid(", "function __tspNanoid(");
    // Internal cross-references inside the inlined source (they use
    // the bare names in the original module).
    src = src.replace(
        "return customRandom(alphabet, size, random)",
        "return __tspNanoidCustomRandom(alphabet, size, __tspNanoidRandom)",
    );
    format!(
        "// === Inlined nanoid runtime (compiled from {} bytes of source, see bun/package.json#nanoid) ===\n\
         {}\n\
         // === End nanoid runtime (functions are in module scope; tsp:server re-exports them) ===\n",
        NANOID_RAW_SOURCE.len(),
        src,
    )
}

/// Build the prelude that inlines zod 3.25.76 (pre-bundled CJS)
/// into the wrap. The page reaches the zod namespace via
/// `import { zod } from "tsp:server"`; the rewriter emits
/// `const { zod } = __tspServer;` and the page uses it as
/// `zod.object({...})` / `zod.string()` / `zod.coerce.number()`.
///
/// The bundle is plain CJS that mutates `module.exports.z` at the
/// top level. To keep every top-level `var` (`z`, `__commonJS`,
/// `__importStar`, the `require_*` lazy factories, etc.) inside
/// the wrap's local scope -- and out of the page module -- the
/// body is wrapped in an immediately-invoked function that
/// provides fresh `module` and `exports` locals and returns
/// `module.exports.z` (the zod namespace object). The IIFE's
/// return value is bound to a single const so only one
/// identifier (`__tspZodNs__`) escapes into the page module
/// scope, and the `__tspServer` freeze then re-exposes it under
/// its public `zod` name.
///
/// This deliberately does NOT publish the namespace on
/// `globalThis` (plan §16.4: framework API must be explicitly
/// imported or accessed via Context, not leaked globally).
fn zod_prelude() -> String {
    format!(
        "// === Inlined zod runtime ({} bytes of bundled CJS; regenerate via `bun build node_modules/zod/index.js --format=cjs --bundle --target=bun --minify --outfile bun/src/runtime/tsp/vendor/zod-4.4.3.cjs`) ===\n\
         const __tspZodNs__ = (function() {{\n\
         \x20 var module = {{ exports: {{}} }};\n\
         \x20 var exports = module.exports;\n\
         \x20 {}\n\
         \x20 return module.exports.z;\n\
         }})();\n\
         // === End zod runtime ===\n",
        ZOD_RAW_SOURCE.len(),
        ZOD_RAW_SOURCE
    )
}

/// Build the prelude that surfaces bun's native `Bun.password`
/// to the page as `__tspServer.password`. The page reaches the
/// password API via
/// `import { password } from "tsp:server"`; the rewriter emits
/// `const { password } = __tspServer;` and the page uses it as
/// `password.hashSync("hello", { algorithm: "bcrypt", cost: 4 })`
/// or `password.verifySync("hello", hash)` or
/// `password.hashSync("hello", { algorithm: "argon2id" })`.
///
/// The page-side API is **not** wrapped to look like v1's
/// `bcrypt.hashSync(pw, salt)` shape; the previous wrap of
/// bcryptjs (slice 17c first attempt) was abandoned in favour
/// of the direct `Bun.password` surface once we confirmed the
/// builtin supports bcrypt / argon2id / scrypt with native
/// performance. Pages write the bun-native API directly; the
/// only added value of the namespace is satisfying plan §16.4
/// (no `globalThis.Bun` leakage) and giving the rewriter a
/// single point to gate which builtin surfaces the page can
/// reach (`password` is whitelisted; `Bun` itself is not).
///
/// Cost notes: `Bun.password` is a native Rust implementation
/// (no JS-side parse per request, no per-call allocations from
/// a pre-bundled npm library). The page handler's first call
/// triggers algorithm-key + salt generation inside bun; once
/// warm the per-hash latency is well under 1 ms for bcrypt at
/// cost=4. Production deployments should prefer `argon2id`
/// (the bun default) over `bcrypt` per OWASP 2024+ guidance;
/// bcrypt remains here for legacy hash format interop
/// (verifying existing user passwords stored as `$2b$...`).
fn password_prelude() -> String {
    // Forward Bun.password through the frozen `__tspServer`
    // object. No `require`, no `include_str!` -- bun's
    // `Bun.password` is already in the host binary, and the
    // synthetic `bun:main` evaluation context exposes it via
    // the global `Bun` object. The rewriter hands the page
    // `__tspServer.password` which IS the same `Bun.password`
    // instance, so any future `Bun.password.*` API surface
    // (e.g. `Bun.password.options`) is automatically available
    // to the page without a v2 release.
    "const __tspPasswordNs__ = Bun.password;\n".to_string()
}

/// `ctx_json` is the JSON-serialised `Context` (spec
/// sect.13). If `Some`, the preamble parses it and passes
/// the resulting object as the page handler's only
/// argument. If `None`, the handler is called with no
/// argument so legacy zero-arg fixtures keep working.
pub fn wrap_for_bun_cli(transformed: &str, method: &str, ctx_json: Option<&str>) -> String {
    let mut out = String::with_capacity(
        transformed.len()
            + 1024
            + nanoid_prelude().len()
            + zod_prelude().len()
            + password_prelude().len(),
    );
    // Compile nanoid 5.1.6 into the wrap preamble so pages can call
    // `nanoid()` / `customAlphabet()` / `random()` / `customRandom()`
    // directly, without an `import` step (the synthetic entry has no
    // module resolver for arbitrary npm packages). See
    // `nanoid_prelude` for the build pipeline.
    out.push_str(&nanoid_prelude());
    // Slice 17b: embed zod 3.25.76 as `tsp:server.zod` so pages can
    // declare / parse schemas without an `import` step (the synthetic
    // entry has no module resolver for arbitrary npm packages, same
    // constraint as nanoid). See `zod_prelude` for the build pipeline.
    out.push_str(&zod_prelude());
    // Slice 17c (revised): surface bun's native `Bun.password` as
    // `tsp:server.password` so pages can call
    // `password.hashSync("...", { algorithm: "bcrypt", cost: 4 })`
    // or `password.verifySync("...", hash)`. No `include_str!` or
    // pre-bundle: bun's builtin is already in the host binary, and
    // the page reaches it through the global `Bun` object. The
    // first attempt (bcryptjs embed, slice 17c) was abandoned when
    // we confirmed `Bun.password` covers bcrypt / argon2id / scrypt
    // with native performance and zero per-request parse cost. See
    // `password_prelude` for the binding contract.
    out.push_str(&password_prelude());
    // Slice 17d: surface bun's native SQL client (`Bun.SQL` /
    // `require("bun").SQL`) as `tsp:server.sql` so pages can do
    // `await sql\`mysql://...\`` for MySQL/PG/SQLite access. No
    // embed: bun's builtin is already in the host binary, and the
    // page reaches it through the synthetic `bun:main` module's own
    // `require()`. The connection factory comes back to the page as
    // a callable; calling it with a tagged-template returns a pooled
    // connection that the page must `close()` (returning it to the
    // per-worker pool). See plan §17.1 for the per-worker pool
    // boundary.
    out.push_str(
        "const __tspSqlNs__ = require(\"bun\").SQL;\n"
    );
    out.push_str("// Generated by TSP v2 PoC 1 slice 16b (jsx.rs)\n");
    out.push_str("// Transformed .tsp -> runnable .js for `bun run tempfile`.\n");
    out.push_str("// Do not edit by hand; the host regenerates this on every request.\n");
    out.push_str("const __tspConsoleLog = globalThis.console ? globalThis.console.log : (...a) => print(...a);\nconst __tspServiceLogs = [];\nconst __tspSessionWrites = [];\nconst __tspCookieWrites = [];\n");
    out.push_str(
        "const React = {\n\
         \x20 Fragment: Symbol.for('react.fragment'),\n\
         \x20 createElement(__type__, __props__, ...__children__) {\n\
         \x20\x20 const __p__ = Object.assign({}, __props__ || {});\n\
         \x20\x20 __p__.children = __children__.length === 1 ? __children__[0] : __children__;\n\
         \x20\x20 return {type: __type__, key: __p__.key || null, ref: __p__.ref || null, props: __p__};\n\
         \x20 },\n\
         };\n\
         globalThis.React = React;\n\
         const __tspFragments = new Map();\n\
         function __tspFragment__(__name__, __handler__) {\n\
         \x20 if (typeof __handler__ !== 'function') throw new Error('fragment handler must be a function');\n\
         \x20 __tspFragments.set(__name__, __handler__);\n\
         \x20 return __handler__;\n\
         }\n\
         function __tspRaw__(__value__) { return {__tspRaw: String(__value__)}; }\n\
         function __tspInit__(__init__) { return Object.assign({}, __init__ || {}); }\n\
         function __tspJson__(__value__, __init__) {\n\
         \x20 const __headers__ = new Headers((__init__ || {}).headers || {});\n\
         \x20 if (!__headers__.has('content-type')) __headers__.set('content-type', 'application/json; charset=utf-8');\n\
         \x20 return new Response(JSON.stringify(__value__), Object.assign(__tspInit__(__init__), {headers: __headers__}));\n\
         }\n\
         function __tspText__(__value__, __init__) {\n\
         \x20 const __headers__ = new Headers((__init__ || {}).headers || {});\n\
         \x20 if (!__headers__.has('content-type')) __headers__.set('content-type', 'text/plain; charset=utf-8');\n\
         \x20 return new Response(String(__value__), Object.assign(__tspInit__(__init__), {headers: __headers__}));\n\
         }\n\
         function __tspHtml__(__value__, __init__) {\n\
         \x20 const __headers__ = new Headers((__init__ || {}).headers || {});\n\
         \x20 if (!__headers__.has('content-type')) __headers__.set('content-type', 'text/html; charset=utf-8');\n\
         \x20 return new Response(String(__value__), Object.assign(__tspInit__(__init__), {headers: __headers__}));\n\
         }\n\
         function __tspRedirect__(__location__, __status__) {\n\
         \x20 const __headers__ = new Headers({'location': String(__location__)});\n\
         \x20 return new Response(null, {status: __status__ === undefined ? 302 : __status__, headers: __headers__});\n\
         }\n\
         function __tspNotFound__() { return __tspText__('Not Found', {status: 404}); }\n\
         class __tspHttpError__ extends Error {\n\
         \x20 constructor(__status__, __message__, __init__) { super(__message__); this.name = 'HttpError'; this.status = __status__; this.headers = new Headers((__init__ || {}).headers || {}); }\n\
         }\n\
         const __tspServer = Object.freeze({json: __tspJson__, redirect: __tspRedirect__, text: __tspText__, html: __tspHtml__, notFound: __tspNotFound__, HttpError: __tspHttpError__, fragment: __tspFragment__, raw: __tspRaw__, nanoid: __tspNanoid, customAlphabet: __tspNanoidCustomAlphabet, customRandom: __tspNanoidCustomRandom, random: __tspNanoidRandom, zod: __tspZodNs__, password: __tspPasswordNs__, sql: __tspSqlNs__});\n"
    );
    out.push_str(
        "function __tspEscape__(__value__) {\n\
         \x20 return String(__value__).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/\\\"/g, '&quot;').replace(/'/g, '&#39;');\n\
         }\n\
         async function __tspRenderNode__(__node__, __child__) {\n\
         \x20 __node__ = await __node__;\n\
         \x20 if (__node__ == null || typeof __node__ === 'boolean') return '';\n\
         \x20 if (typeof __node__ === 'string') return __child__ ? __tspEscape__(__node__) : __node__;\n\
         \x20 if (typeof __node__ === 'number' || typeof __node__ === 'bigint') return String(__node__);\n\
         \x20 if (Array.isArray(__node__)) return (await Promise.all(__node__.map(__n__ => __tspRenderNode__(__n__, true)))).join('');\n\
         \x20 if (__node__.__tspRaw !== undefined) return __node__.__tspRaw;\n\
         \x20 if (typeof __node__ !== 'object' || !__node__.props) throw new Error('TSP3102: object cannot be rendered as an HTML child');\n\
         \x20 const __type__ = __node__.type;\n\
         \x20 const __props__ = __node__.props || {};\n\
         \x20 if (__type__ === React.Fragment) return __tspRenderNode__(__props__.children, true);\n\
         \x20 if (typeof __type__ === 'function') return __tspRenderNode__(__type__(__props__), true);\n\
         \x20 if (typeof __type__ !== 'string') throw new Error('TSP3103: unsupported JSX element type');\n\
         \x20 let __attrs__ = '';\n\
         \x20 for (const [__rawName__, __value__] of Object.entries(__props__)) {\n\
         \x20\x20 if (__rawName__ === 'children' || __rawName__ === 'key' || __rawName__ === 'ref') continue;\n\
         \x20\x20 const __name__ = __rawName__ === 'className' ? 'class' : (__rawName__ === 'htmlFor' ? 'for' : __rawName__);\n\
         \x20\x20 if (typeof __value__ === 'function') throw new Error('TSP3105: function-valued HTML attributes are not serializable');\n\
         \x20\x20 if (__value__ == null || __value__ === false) continue;\n\
         \x20\x20 if (__value__ === true) { __attrs__ += ' ' + __name__; continue; }\n\
         \x20\x20 if (typeof __value__ === 'object') throw new Error('TSP3104: object-valued HTML attributes are not serializable');\n\
         \x20\x20 __attrs__ += ' ' + __name__ + '=\"' + __tspEscape__(__value__) + '\"';\n\
         \x20 }\n\
         \x20 const __void__ = /^(area|base|br|col|embed|hr|img|input|link|meta|param|source|track|wbr)$/i.test(__type__);\n\
         \x20 if (__void__) return '<' + __type__ + __attrs__ + '>';\n\
         \x20 return '<' + __type__ + __attrs__ + '>' + await __tspRenderNode__(__props__.children, true) + '</' + __type__ + '>';\n\
         }\n"
    );
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
             // Slice 16n' (Phase 8): make ctx.signal a real,\n\
             // abortable AbortSignal. The host writes `ABORT_MARKER`\n\
             // (the single marker byte jsc_bridge.rs writes to bun\n\
             // stdin) when the per-request timeout fires. A controller\n\
             // is created once at module scope, the stdin listener\n\
             // (also module-scope) aborts it in time for the page's\n\
             // cooperative-cancel code (spec 13.7).\n\
             const __tspAbortCtrl__ = new AbortController();\n\
             globalThis.process?.stdin?.on('data', () => { try { __tspAbortCtrl__.abort(); } catch (__e__) {} });\n\
             __tspContext.signal = __tspAbortCtrl__.signal;\n\
             __tspContext.fragment = (__name__, __params__) => {\n\
             \x20 const __q__ = new URLSearchParams({route: __tspContext.path, name: String(__name__), token: __tspContext.__tsp_fragment_token, ...(__params__ || {})});\n\
             \x20 return '/__tsp/fragment?' + __q__.toString();\n\
             };\n\
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
             function __tspFormatCookie__(__name__, __value__, __options__) {\n\
             \x20 let __line__ = __name__ + '=' + __value__;\n\
             \x20 const __opts__ = __options__ || {};\n\
             \x20 if (__opts__.path) __line__ += '; Path=' + __opts__.path;\n\
             \x20 if (typeof __opts__.maxAge === 'number') __line__ += '; Max-Age=' + __opts__.maxAge;\n\
             \x20 if (__opts__.domain) __line__ += '; Domain=' + __opts__.domain;\n\
             \x20 if (__opts__.httpOnly) __line__ += '; HttpOnly';\n\
             \x20 if (__opts__.secure) __line__ += '; Secure';\n\
             \x20 if (__opts__.sameSite) __line__ += '; SameSite=' + __opts__.sameSite;\n\
             \x20 if (__opts__.expires instanceof Date && !Number.isNaN(__opts__.expires.getTime())) __line__ += '; Expires=' + __opts__.expires.toUTCString();\n\
             \x20 return __line__;\n\
             }\n\
             __tspContext.cookies = {\n\
             \x20 get(__n__) { return __tspCookieMap__.has(__n__) ? __tspCookieMap__.get(__n__) : undefined; },\n\
             \x20 has(__n__) { return __tspCookieMap__.has(__n__); },\n\
             \x20 set(__n__, __v__, __o__) { __tspCookieWrites.push(__tspFormatCookie__(__n__, __v__, __o__)); },\n\
             \x20 delete(__n__, __o__) { const __optDel__ = Object.assign({maxAge: 0}, __o__ || {}); __tspCookieWrites.push(__tspFormatCookie__(__n__, '', __optDel__)); },\n\
             };\n             // Slice 16j (Phase 8): hydrate ctx.services (spec\n             // sect.17). The host embeds a descriptor snapshot;\n             // kind='logger' becomes a log adapter whose calls\n             // buffer into __tspServiceLogs (carried back in the\n             // envelope -> host flushes into the runtime service);\n             // other descriptors surface read-only -- app code MUST\n             // NOT rely on wrapper identity across requests (17.3).\n             const __tspServicesRaw__ = __tspContext.services || {};\n             __tspContext.services = {};\n             for (const [__sName__, __sDesc__] of Object.entries(__tspServicesRaw__)) {\n             \x20 if (__sDesc__ && __sDesc__.kind === 'logger') {\n             \x20\x20 __tspContext.services[__sName__] = Object.assign({}, __sDesc__, {\n             \x20\x20\x20 info: (...__a__) => __tspServiceLogs.push({svc: __sName__, level: 'info', message: __a__.join(' ')}),\n             \x20\x20\x20 warn: (...__a__) => __tspServiceLogs.push({svc: __sName__, level: 'warn', message: __a__.join(' ')}),\n             \x20\x20\x20 error: (...__a__) => __tspServiceLogs.push({svc: __sName__, level: 'error', message: __a__.join(' ')}),\n             \x20\x20\x20 debug: (...__a__) => __tspServiceLogs.push({svc: __sName__, level: 'debug', message: __a__.join(' ')}),\n             \x20\x20 });\n             \x20 } else {\n             \x20\x20 __tspContext.services[__sName__] = Object.freeze(__sDesc__);\n             \x20 }\n             }\n             // Slice 16k (Phase 8): hydrate ctx.session\n             // (spec sect.16). The host embeds the current\n             // request's session view as {id, data};\n             // calls into the session buffer into\n             // __tspSessionWrites which the wrap carries\n             // back in the envelope -> host applies.\n             const __tspSessionRaw__ = __tspContext.session;\n             const __tspSessionData__ = (__tspSessionRaw__ && __tspSessionRaw__.data) ? __tspSessionRaw__.data : {};\n             const __tspSessionId__ = (__tspSessionRaw__ && __tspSessionRaw__.id) ? __tspSessionRaw__.id : '';\n             function __tspFormatSessionValue__(__v__) {\n             \x20 if (__v__ === null) return null;\n             \x20 if (typeof __v__ === 'boolean' || typeof __v__ === 'number' || typeof __v__ === 'string') return __v__;\n             \x20 if (Array.isArray(__v__)) return __v__.map(__tspFormatSessionValue__);\n             \x20 if (typeof __v__ === 'object') {\n             \x20\x20 const __out__ = {};\n             \x20\x20 for (const __k__ in __v__) if (Object.prototype.hasOwnProperty.call(__v__, __k__)) __out__[__k__] = __tspFormatSessionValue__(__v__[__k__]);\n             \x20\x20 return __out__;\n             \x20 }\n             \x20 return String(__v__);\n             }\n             __tspContext.session = {\n             \x20 id: __tspSessionId__,\n             \x20 get(__k__) { return Object.prototype.hasOwnProperty.call(__tspSessionData__, __k__) ? __tspSessionData__[__k__] : undefined; },\n             \x20 has(__k__) { return Object.prototype.hasOwnProperty.call(__tspSessionData__, __k__); },\n             \x20 set(__k__, __v__) { __tspSessionWrites.push({op: 'set', k: __k__, v: __tspFormatSessionValue__(__v__)}); },\n             \x20 delete(__k__) { __tspSessionWrites.push({op: 'delete', k: __k__}); },\n             \x20 clear() { __tspSessionWrites.push({op: 'clear'}); },\n             \x20 async regenerate() { __tspSessionWrites.push({op: 'regenerate'}); },\n             \x20 async destroy() { __tspSessionWrites.push({op: 'destroy'}); },\n             };\n             "
        );
    }
    // Evaluate the page module before selecting the handler. Fragment
    // exports are `const` bindings and therefore cannot be invoked before
    // the source module has initialized them.
    out.push_str(transformed);
    out.push_str("\nconst __tspFragmentName__ = typeof __tspContext === 'undefined' ? undefined : __tspContext.__tsp_fragment;\n");
    out.push_str("let __tspHandler__;\nif (__tspFragmentName__) __tspHandler__ = __tspFragments.get(__tspFragmentName__);\nelse __tspHandler__ = ");
    out.push_str(method);
    out.push_str(";\nif (typeof __tspHandler__ !== 'function') throw new Error(__tspFragmentName__ ? 'fragment not found: ' + __tspFragmentName__ : 'page does not export function ");
    out.push_str(method);
    out.push_str("()');\n");
    if ctx_json.is_none() {
        out.push_str("const __tspContext = undefined;\n");
    }
    out.push_str("const __tspResultPromise__ = Promise.resolve().then(() => __tspContext === undefined ? __tspHandler__() : __tspHandler__(__tspContext)).then(async (__result__) => typeof __result__ === 'object' && __result__ !== null && !(__result__ instanceof Response) ? await __tspRenderNode__(__result__, false) : __result__).catch((e) => { if (e && Number.isInteger(e.status)) return new Response(String(e.message || ''), {status: e.status, headers: e.headers || {}}); throw e; });\n");
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
        "(async () => {\n         \x20let __tspBody__, __tspStatus__, __tspHeaders__, __tspType__;\n         \x20const __tspResult__ = await __tspResultPromise__;\n         \x20__tspHeaders__ = [];\n         \x20if (__tspResult__ instanceof Response) {\n         \x20\x20__tspType__ = 'response';\n         \x20\x20__tspStatus__ = __tspResult__.status;\n         \x20\x20for (const [__k__, __v__] of __tspResult__.headers) __tspHeaders__.push([__k__, __v__]);\n         \x20\x20__tspBody__ = await __tspResult__.text();\n         \x20} else if (typeof __tspResult__ === 'string') {\n         \x20\x20__tspType__ = 'html';\n         \x20\x20__tspStatus__ = 200;\n         \x20\x20__tspBody__ = __tspResult__;\n         \x20} else {\n         \x20\x20throw new Error('page returned invalid value (expected string or Response, got ' + (typeof __tspResult__) + ')');\n         \x20}\n         \x20// Merge runtime cookie writes into the outgoing headers\n         \x20// (spec sect.15: cookie writes MUST be reflected even when\n         \x20// the handler returns an HtmlNode). Each write becomes a\n         \x20// separate Set-Cookie line so multiple cookies on one\n         \x20// request don't collapse via the response's flatten loop.\n         \x20if (Array.isArray(__tspCookieWrites)) {\n         \x20\x20for (const __cookieLine__ of __tspCookieWrites) {\n         \x20\x20\x20__tspHeaders__.push(['Set-Cookie', __cookieLine__]);\n         \x20\x20}\n         \x20}\n         \x20const __tspEnvelope__ = JSON.stringify({type: __tspType__, status: __tspStatus__, headers: __tspHeaders__, body: __tspBody__, service_logs: __tspServiceLogs, session_writes: __tspSessionWrites});\n         \x20__tspConsoleLog('__TSP_OUT_V1__' + '\\n' + __tspEnvelope__);\n         process.exit(0);\n         })().catch((e) => { console.error(String(e && e.stack || e)); process.exit(1); });\n"
    );
    out
}

/// Adapt the normal TSP wrapper for a Bun VM that stays alive across requests.
///
/// The page execution and response-envelope logic remains identical to the
/// subprocess path. Only the transport changes: the envelope is placed on a
/// well-known global for the native worker to read, and the wrapper does not
/// call `process.exit`, which would tear down the embedded runtime.
pub fn wrap_for_embedded_worker(transformed: &str, method: &str, ctx_json: Option<&str>) -> String {
    let mut wrapped = wrap_for_bun_cli(transformed, method, ctx_json);
    // A worker VM serves more than one request. Clear the previous request's
    // result before loading the next entry point so a failed execution cannot
    // accidentally reuse a stale envelope.
    wrapped = format!(
        "globalThis.__tspEmbeddedResponse = undefined;\nglobalThis.__tspEmbeddedError = undefined;\n{wrapped}"
    );
    let stdout_success =
        "__tspConsoleLog('__TSP_OUT_V1__' + '\\n' + __tspEnvelope__);\n         process.exit(0);";
    let embedded_success = "globalThis.__tspEmbeddedResponse = __tspEnvelope__;";
    if !wrapped.contains(stdout_success) {
        return wrapped;
    }
    wrapped = wrapped.replace(stdout_success, embedded_success);
    let stdout_error = "console.error(String(e && e.stack || e)); process.exit(1);";
    let embedded_error = "globalThis.__tspEmbeddedError = String(e && e.stack || e);";
    wrapped.replace(stdout_error, embedded_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_module_exports_for_bun_tsx_transpiler() {
        let src = "export function GET() { return 'x'; }\n";
        let out = tsx_to_js(src).unwrap();
        assert!(out.contains("function GET"));
        assert!(out.contains("export function GET"));
    }

    #[test]
    fn rewrites_tsp_server_named_imports_and_async_exports() {
        let src = "import { type Context, json, redirect as go } from \"tsp:server\";\nexport async function GET(ctx: Context) { return json({ok: true}); }\n";
        let out = tsx_to_js(src).unwrap();
        assert!(
            out.contains("const { json, redirect: go } = __tspServer;"),
            "got: {out}"
        );
        assert!(!out.contains("tsp:server"), "got: {out}");
        assert!(out.contains("async function GET"), "got: {out}");
        assert!(out.contains("export async function"), "got: {out}");
    }

    #[test]
    fn rewrites_semicolon_free_and_multiline_tsp_server_imports() {
        let src = "import {\n  json,\n  type Context\n} from 'tsp:server'\nexport async function GET(ctx: Context) { return json('ok') }\n";
        let out = tsx_to_js(src).unwrap();
        assert!(out.contains("const { json } = __tspServer;"), "got: {out}");
        assert!(
            out.contains("async function GET(ctx: Context)"),
            "got: {out}"
        );
        assert!(!out.contains("tsp:server"), "got: {out}");
    }

    #[test]
    fn rewrites_relative_imports_to_file_urls_and_rejects_tsp_imports() {
        let dir = std::env::temp_dir().join(format!("tsp-v2-jsx-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("shared.ts"), "export const value = 'ok';\n").unwrap();
        let out = rewrite_local_imports("import { value } from './shared';\n", &dir).unwrap();
        assert!(out.contains("file:///"), "got: {out}");
        assert!(out.contains("shared.ts"), "got: {out}");
        let err = rewrite_local_imports("import page from './page.tsp';\n", &dir).unwrap_err();
        assert!(matches!(err, JsxError::UnsupportedShape { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rewrite_local_imports_converts_static_imports_to_require() {
        // Slice 17d: relative `import { x } from "./y"` declarations
        // cannot survive the wrap preamble (which prepends a
        // multi-line prelude before the page body lands mid-file,
        // so a static `import` is invalid in both CJS and ESM
        // module scopes and bun's synthetic `bun:main` poll loop
        // never sees the response). The rewriter turns them into
        // `const { x } = require("file:///...y?tsp_generation=N")`
        // so the binding is available at a top-level position the
        // prelude can follow up on. Pages still write `import`;
        // the rewriter absorbs the syntactic difference.
        let dir =
            std::env::temp_dir().join(format!("tsp-v2-jsx-rewrite-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("shared.ts"), "export const a = 1;\nexport const b = 2;\n")
            .unwrap();
        let src = "import { a, b as renamed } from './shared';\nexport function GET() { return a + renamed; }\n";
        let out = rewrite_local_imports(src, &dir).unwrap();
        assert!(
            out.contains("const { a, b: renamed } = require("),
            "rewriter must collapse the import to a `const ... = require(...)` destructure; got: {out}"
        );
        assert!(
            !out.contains("import {"),
            "the original `import` syntax must not survive the rewriter; got: {out}"
        );
        assert!(
            out.contains("file://") && out.contains("shared.ts"),
            "rewriter must use an absolute `file://` URL; got: {out}"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn generation_import_urls_bust_persistent_worker_cache() {
        let dir =
            std::env::temp_dir().join(format!("tsp-v2-jsx-generation-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("shared.ts"), "export const value = 'ok';\n").unwrap();
        let out =
            rewrite_local_imports_for_generation("import { value } from './shared';\n", &dir, 17)
                .unwrap();
        assert!(out.contains("tsp_generation=17"), "got: {out}");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rewrites_relative_dynamic_imports_to_file_urls() {
        let dir = std::env::temp_dir().join(format!("tsp-v2-dynamic-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("lazy.ts"), "export const value = 1;\n").unwrap();
        let out = rewrite_local_imports("const lazy = import('./lazy');\n", &dir).unwrap();
        assert!(out.contains("file://"), "got: {out}");
        assert!(!out.contains("./lazy"), "got: {out}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn preserves_typescript_for_bun_transpiler() {
        let src = "export function GET() { return ': Context'; }\n";
        let out = tsx_to_js(src).unwrap();
        assert!(out.contains("return ': Context';"), "got: {out}");
    }

    #[test]
    fn preserves_jsx_for_bun_transpiler() {
        let src = "export function GET() { return <h1>Hello</h1>; }\n";
        let out = tsx_to_js(src).unwrap();
        assert!(out.contains("return <h1>Hello</h1>;"), "got: {out}");
    }

    #[test]
    fn slice_6_fixture_full_transform() {
        let src = "// comment\nexport function GET() {\n  return <h1>Hello from TSP v2</h1>;\n}\n";
        let out = tsx_to_js(src).unwrap();
        assert!(out.contains("export function"));
        assert!(out.contains("function GET"));
        assert!(out.contains("<h1>Hello from TSP v2</h1>"), "got: {out}");
    }

    #[test]
    fn nested_jsx_is_preserved_for_runtime_renderer() {
        let src = "export function GET() { return <div><h1>Hi</h1></div>; }\n";
        let out = tsx_to_js(src).unwrap();
        assert!(out.contains("<div><h1>Hi</h1></div>"), "got: {out}");
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
        assert!(wrapped.contains("__tspHandler__()"));
        assert!(wrapped.contains("__TSP_OUT_V1__"));
        assert!(wrapped.contains("const __tspContext = undefined;"));
    }

    #[test]
    fn embedded_wrapper_does_not_exit_or_write_stdout() {
        let body = "function GET() { return 'ok'; }\n";
        let wrapped = wrap_for_embedded_worker(body, "GET", None);
        assert!(wrapped.contains("__tspEmbeddedResponse"));
        assert!(!wrapped.contains("process.exit(0)"));
        assert!(!wrapped.contains("__tspConsoleLog('__TSP_OUT_V1__"));
    }

    #[test]
    fn wrap_emits_call_to_method_with_ctx() {
        // With a Context JSON, the preamble parses it and
        // passes the result as the method's only argument.
        let body = "function GET(ctx) { return ctx.method; }\n";
        let json = r#"{"method":"GET"}"#;
        let wrapped = wrap_for_bun_cli(body, "GET", Some(json));
        assert!(wrapped.contains("const __tspContext = JSON.parse("));
        assert!(wrapped.contains("__tspHandler__(__tspContext)"));
        assert!(wrapped.contains("__TSP_OUT_V1__"));
    }

    #[test]
    fn wrap_emits_tsp_server_response_helpers() {
        let wrapped =
            wrap_for_bun_cli("function GET() { return json({ok: true}); }\n", "GET", None);
        assert!(
            wrapped.contains("const __tspServer = Object.freeze"),
            "got: {wrapped}"
        );
        assert!(
            wrapped.contains("application/json; charset=utf-8"),
            "got: {wrapped}"
        );
        assert!(wrapped.contains("__tspHttpError__"), "got: {wrapped}");
        assert!(wrapped.contains("Promise.resolve().then"), "got: {wrapped}");
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
        // Slice 16n': the signal is backed by a module-scope
        // AbortController that the stdin 'data' listener aborts
        // on the host's ABORT_MARKER (spec 13.7 timeout).
        assert!(
            wrapped.contains("const __tspAbortCtrl__ = new AbortController()"),
            "got: {wrapped}"
        );
        assert!(
            wrapped.contains("globalThis.process?.stdin?.on('data'"),
            "got: {wrapped}"
        );
        assert!(
            wrapped.contains("__tspAbortCtrl__.abort()"),
            "got: {wrapped}"
        );
        assert!(
            wrapped.contains("__tspContext.signal = __tspAbortCtrl__.signal"),
            "got: {wrapped}"
        );
        // The request body is passed to Bun's native Request
        // for body-bearing methods (POST). Slice 16g
        // changed the wire form: base64 over the JSON
        // field `body_b64`; the wrap preamble atob-decodes
        // it to a Uint8Array so binary multipart survives.
        assert!(
            wrapped.contains("atob(__tspContext.body_b64)"),
            "got: {wrapped}"
        );
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
        assert!(
            wrapped.contains("__tspContext.method !== 'GET'"),
            "got: {wrapped}"
        );
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
        // Full CookieOptions support includes Date-based Expires.
        assert!(
            wrapped.contains("__opts__.expires instanceof Date"),
            "got: {wrapped}"
        );
        assert!(
            wrapped.contains("__opts__.expires.toUTCString()"),
            "got: {wrapped}"
        );
        // Async IIFE merges writes into the response header
        // array (Set-Cookie entries).
        assert!(
            wrapped.contains("['Set-Cookie', __cookieLine__]"),
            "got: {wrapped}"
        );
        // Header wire shape: array of [k, v] pairs (16f),
        // not the slice 16c flat object.
        assert!(
            wrapped.contains("__tspHeaders__.push([__k__, __v__])"),
            "got: {wrapped}"
        );
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

    #[test]
    fn wrap_hydrates_ctx_services() {
        // Slice 16j: the preamble hydrates the host's service
        // descriptor snapshot into `ctx.services`. Logger gets
        // a log adapter (state stays visible, methods buffer
        // into __tspServiceLogs); unknown kinds surface
        // read-only (spec sect.17.3).
        let body = "function GET(ctx) { return ctx.services.logger.total_lines; }\n";
        let json = r#"{"method":"GET","path":"/","query":"","headers":{},"body":"","services":{"logger":{"kind":"logger","scope":"runtime","total_lines":3}}}"#;
        let wrapped = wrap_for_bun_cli(body, "GET", Some(json));
        assert!(wrapped.contains("__tspServiceLogs"), "got: {wrapped}");
        assert!(wrapped.contains("__tspServicesRaw__"), "got: {wrapped}");
        assert!(wrapped.contains("kind === 'logger'"), "got: {wrapped}");
        assert!(
            wrapped.contains("Object.assign({}, __sDesc__, {"),
            "got: {wrapped}"
        );
        assert!(
            wrapped.contains("Object.freeze(__sDesc__)"),
            "got: {wrapped}"
        );
    }

    #[test]
    fn wrap_envelope_carries_service_logs_and_legacy_path_still_works() {
        // The envelope always emits `service_logs`; the buffer
        // is declared unconditionally so the legacy zero-arg
        // fixture (ctx_json None) does not hit a ReferenceError.
        let body = "function GET() { return 'x'; }\n";
        let wrapped = wrap_for_bun_cli(body, "GET", None);
        assert!(
            wrapped.contains("const __tspServiceLogs = [];"),
            "got: {wrapped}"
        );
        assert!(
            wrapped.contains("service_logs: __tspServiceLogs"),
            "got: {wrapped}"
        );
    }

    #[test]
    fn wrap_hydrates_ctx_session() {
        // Slice 16k: the preamble hydrates `ctx.session`
        // (spec sect.16) with id + read methods + a
        // write-buffer (set/delete/clear/regenerate/destroy)
        // that flows back through the envelope.
        let body = "function GET(ctx) { return ctx.session.id; }\n";
        let json = r#"{"method":"GET","path":"/","query":"","headers":{},"body":"","services":{},"session":{"id":"deadbeef","data":{"k":"v"}}}"#;
        let wrapped = wrap_for_bun_cli(body, "GET", Some(json));
        assert!(
            wrapped.contains("const __tspSessionWrites = [];"),
            "got: {wrapped}"
        );
        assert!(wrapped.contains("__tspSessionRaw__"), "got: {wrapped}");
        assert!(wrapped.contains("__tspSessionData__"), "got: {wrapped}");
        assert!(wrapped.contains("id: __tspSessionId__"), "got: {wrapped}");
        assert!(wrapped.contains("op: 'set'"), "got: {wrapped}");
        assert!(wrapped.contains("op: 'delete'"), "got: {wrapped}");
        assert!(wrapped.contains("op: 'clear'"), "got: {wrapped}");
        assert!(wrapped.contains("op: 'regenerate'"), "got: {wrapped}");
        assert!(wrapped.contains("op: 'destroy'"), "got: {wrapped}");
        assert!(
            wrapped.contains("session_writes: __tspSessionWrites"),
            "got: {wrapped}"
        );
    }

    #[test]
    fn wrap_session_buffer_declared_unconditionally_for_legacy_path() {
        // Legacy zero-arg fixtures (ctx_json None) must
        // still see `__tspSessionWrites` declared so the
        // envelope can JSON.stringify it without a
        // ReferenceError.
        let body = "function GET() { return 'x'; }\n";
        let wrapped = wrap_for_bun_cli(body, "GET", None);
        assert!(
            wrapped.contains("const __tspSessionWrites = [];"),
            "got: {wrapped}"
        );
        assert!(
            wrapped.contains("session_writes: __tspSessionWrites"),
            "got: {wrapped}"
        );
    }

    // -----------------------------------------------------------------
    // Multi-route dispatch regression tests
    //
    // The user observed that requesting /, /time, /svc, /users/42
    // against a routes/ tree with 8 distinct .tsp files all returned
    // the same `Hello GET /` body (which is uniquely index.tsp's GET
    // output). To localize the bug between master and worker we want
    // the wrap to be source-specific: a wrap built from time.tsp's
    // source text must differ from a wrap built from index.tsp's
    // source text in BOTH the handler invocation line and the
    // transformed module body. If these asserts pass, the master is
    // sending per-route scripts; if a real server still aliases all
    // requests to index.tsp, the bug lives in the worker's
    // load_entry_point / module-cache path.
    // -----------------------------------------------------------------

    #[test]
    fn wrap_for_embedded_worker_distinguishes_two_routes() {
        let index_src = r#"
export function GET(ctx) {
  return `<h1>Hello ${ctx.method} ${ctx.path}</h1>`;
}
"#;
        let time_src = r#"
export async function GET(ctx) {
  const t = ctx.services.time;
  return new Response(`iso=${t.iso}`, {
    status: 200,
    headers: { 'content-type': 'text/plain' },
  });
}
"#;
        let json = r#"{"method":"GET","path":"/time","query":"","headers":{},"body":""}"#;
        let wrap_index = wrap_for_embedded_worker(index_src, "GET", Some(json));
        let wrap_time = wrap_for_embedded_worker(time_src, "GET", Some(json));

        // The transformed module body must contain route-specific
        // markers. If the wraps are byte-identical, the master is
        // dropping the source somewhere between page::prepare and
        // wrap_for_embedded_worker.
        assert!(
            wrap_index.contains("Hello ${ctx.method}"),
            "index wrap must carry the Hello template; got prefix: {}",
            &wrap_index[..wrap_index.len().min(200)]
        );
        assert!(
            wrap_time.contains("iso=${t.iso}"),
            "time wrap must carry the iso template; got prefix: {}",
            &wrap_time[..wrap_time.len().min(200)]
        );
        // The two routes' source content must NOT be cross-contaminated.
        assert!(
            !wrap_index.contains("iso=${t.iso}"),
            "index wrap leaked the time.tsp template"
        );
        assert!(
            !wrap_time.contains("Hello ${ctx.method}"),
            "time wrap leaked the index.tsp template"
        );
        // Wraps are different lengths because the source differs.
        assert_ne!(
            wrap_index.len(),
            wrap_time.len(),
            "two different sources produced byte-identical wraps"
        );
    }

    #[test]
    fn wrap_for_embedded_worker_method_bakes_into_handler_selection() {
        // Even for the SAME source, the handler-selection line must
        // reference the request's HTTP method. A worker that reuses a
        // cached wrap across methods would dispatch a POST to GET and
        // the page's POST handler would never run.
        let body = r#"
export function GET(ctx) { return 'get-handler'; }
export async function POST(ctx) { return new Response('post-handler', { status: 201 }); }
"#;
        let json_get = r#"{"method":"GET","path":"/x","query":"","headers":{},"body":""}"#;
        let json_post = r#"{"method":"POST","path":"/x","query":"","headers":{},"body":""}"#;
        let wrap_get = wrap_for_embedded_worker(body, "GET", Some(json_get));
        let wrap_post = wrap_for_embedded_worker(body, "POST", Some(json_post));

        // The wrap stamps the method into `__tspHandler__ = METHOD;`.
        // If the master conflated method, both wraps would have the
        // same handler line.
        assert!(
            wrap_get.contains("__tspHandler__ = GET;"),
            "GET wrap must select GET; got prefix: {}",
            &wrap_get[..wrap_get.len().min(200)]
        );
        assert!(
            wrap_post.contains("__tspHandler__ = POST;"),
            "POST wrap must select POST; got prefix: {}",
            &wrap_post[..wrap_post.len().min(200)]
        );
    }

    // -----------------------------------------------------------------
    // nanoid runtime inlining (slice 17a)
    //
    // The wrap preamble inlines nanoid 5.1.6 as top-level
    // function declarations in the page module's scope and
    // re-exports the four functions on the frozen `__tspServer`
    // object. The page reaches them via
    //     import { nanoid } from "tsp:server"
    // which the rewriter turns into
    //     const { nanoid } = __tspServer;
    //
    // Per plan §16.4 the functions MUST NOT be exposed on
    // `globalThis` -- framework API must be imported, not on the
    // global. This test pins both halves: the inlined source is
    // present, the imports/exports are stripped, AND nothing
    // leaks to `globalThis`.
    // -----------------------------------------------------------------

    #[test]
    fn wrap_for_bun_cli_inlines_nanoid_runtime_for_pages() {
        let body = "function GET() { return nanoid(); }";
        let wrapped = wrap_for_bun_cli(body, "GET", Some("{}"));
        // 1) the inlined nanoid function body must be present.
        //    Functions are prefixed `__tspNanoid*` so the module
        //    scope keeps no bare `nanoid` name -- the page-side
        //    rewriter later generates `const { nanoid } =
        //    __tspServer;`, which WOULD collide with a bare
        //    `function nanoid` in the same scope.
        assert!(
            wrapped.contains("function __tspNanoid("),
            "expected inlined `function __tspNanoid(...)` in the wrap; got prefix: {}",
            &wrapped[..wrapped.len().min(400)]
        );
        assert!(
            wrapped.contains("function __tspNanoidCustomAlphabet("),
            "expected inlined `function __tspNanoidCustomAlphabet(...)` in the wrap"
        );
        assert!(
            wrapped.contains("function __tspNanoidCustomRandom("),
            "expected inlined `function __tspNanoidCustomRandom(...)` in the wrap"
        );
        assert!(
            wrapped.contains("function __tspNanoidRandom("),
            "expected inlined `function __tspNanoidRandom(...)` in the wrap"
        );
        // 2) the frozen __tspServer object must expose the four
        //    names to the page (plan §16.4: explicit import
        //    surface, not globalThis).
        assert!(
            wrapped.contains("nanoid: __tspNanoid, customAlphabet: __tspNanoidCustomAlphabet, customRandom: __tspNanoidCustomRandom, random: __tspNanoidRandom"),
            "wrap must expose the four nanoid names on __tspServer; got prefix: {}",
            &wrapped[..wrapped.len().min(800)]
        );
        // 3) the url-alphabet must be inlined as a const so the
        //    relative `./url-alphabet/index.js` import never has
        //    to resolve from the worker's temp file.
        assert!(
            wrapped.contains("const scopedUrlAlphabet = \"useandom-26T198340PX75pxJACKVERYMINDBUSHWOLF_GQZbfghjklqvwyzrict\";"),
            "url-alphabet must be inlined as a const"
        );
        // 4) no `import` or `export` statements from the original
        //    nanoid source may survive in the prelude.
        for forbidden in [
            "import { webcrypto as crypto } from 'node:crypto'",
            "import { urlAlphabet as scopedUrlAlphabet } from './url-alphabet/index.js'",
            "export { urlAlphabet } from './url-alphabet/index.js'",
            "export function nanoid",
            "export function customAlphabet",
            "export function customRandom",
            "export function random",
        ] {
            assert!(
                !wrapped.contains(forbidden),
                "the raw nanoid fragment `{forbidden}` must be transformed away"
            );
        }
        // 5) plan §16.4: framework API must NOT be on globalThis.
        //    The functions are local, exposed only via the frozen
        //    __tspServer object.
        for forbidden in [
            "globalThis.nanoid =",
            "globalThis.customAlphabet =",
            "globalThis.customRandom =",
            "globalThis.random =",
        ] {
            assert!(
                !wrapped.contains(forbidden),
                "the nanoid functions must not be published on globalThis (plan §16.4); found `{forbidden}`"
            );
        }
    }

    #[test]
    fn rewrite_tsp_server_imports_accepts_nanoid_named_exports() {
        // Plan §16.1/§16.4: nanoid and friends are reached via
        //     import { nanoid, customAlphabet, customRandom, random } from "tsp:server";
        // which the rewriter collapses into
        //     const { nanoid, customAlphabet, customRandom, random } = __tspServer;
        let src = "import { nanoid, customAlphabet, customRandom, random } from \"tsp:server\";\nexport function GET() { return nanoid(); }";
        let rewritten = rewrite_tsp_server_imports(src).expect("rewrite should succeed");
        assert!(
            rewritten.contains("const { nanoid, customAlphabet, customRandom, random } = __tspServer;"),
            "rewriter must collapse the four nanoid names into a single destructure; got: {rewritten}"
        );
        // And the page-side import must be gone (replaced).
        assert!(
            !rewritten.contains("from \"tsp:server\""),
            "the `from \"tsp:server\"` clause must be stripped after rewrite; got: {rewritten}"
        );
    }

    // -----------------------------------------------------------------
    // End-to-end pipeline test (real Bun execution)
    //
    // The unit tests above verify the wrap STRING. This one takes the
    // full production pipeline for the actual `routes/nanoid.tsp`
    // shape -- `tsx_to_js` (import rewrite + fragment rewrite), then
    // `wrap_for_bun_cli` (nanoid prelude + envelope transport) -- and
    // executes the generated module with the REAL `bun` binary. If
    // the pipeline is broken (e.g. the import rewrite mis-orders the
    // `__tspServer` binding, or the prelude leaves a dangling
    // reference), bun will fail with a syntax/RuntimeError and this
    // test catches it without needing a 9-minute binary relink.
    // -----------------------------------------------------------------

    #[test]
    fn nanoid_pipeline_generates_runable_module_under_real_bun() {
        // Mirror of production `routes/nanoid.tsp` (import + GET + POST).
        let source = r#"// Slice 17a regression test fixture.
import { nanoid } from "tsp:server";

export function GET(_ctx) {
  return new Response(nanoid(), {
    status: 200,
    headers: { "content-type": "text/plain" },
  });
}

export async function POST(ctx) {
  const body = await ctx.request.text();
  let size = 21;
  if (body) {
    try {
      const parsed = JSON.parse(body);
      if (typeof parsed.size === "number" && parsed.size > 0) {
        size = parsed.size;
      }
    } catch {
      // body wasn't JSON
    }
  }
  return new Response(nanoid(size), {
    status: 200,
    headers: { "content-type": "text/plain", "x-demo": "slice17a" },
  });
}
"#;
        let transformed = tsx_to_js(source).expect("tsx_to_js must succeed");
        // The import must have been rewritten away and the page handler
        // must still reference the destructured binding.
        assert!(
            !transformed.contains("from \"tsp:server\""),
            "import must be rewritten away; got: {transformed}"
        );
        assert!(
            transformed.contains("const { nanoid } = __tspServer;"),
            "rewriter must emit `const {{ nanoid }} = __tspServer;`; got: {transformed}"
        );

        let ctx_json = r#"{"method":"GET","path":"/nanoid","query":"","headers":{"host":"127.0.0.1:1"},"body_b64":""}"#;
        let wrapped = wrap_for_bun_cli(&transformed, "GET", Some(ctx_json));

        // Locate a runnable bun: the workspace bootstrap bun on
        // Windows, or `bun` on PATH on Unix. Skip on CI images that
        // don't have one (the integration tests in start_order.rs
        // have the same skip-if-unbuilt policy).
        let bun_candidates: &[&str] = if cfg!(windows) {
            &[
                r"D:\GitHub\tsp\.bun-bootstrap\node_modules\bun\bin\bun.exe",
                r"D:\GitHub\tsp\.bun-bootstrap\node_modules\bun\bin\bun.exe".trim_start(), // no-op; keep list readable
            ]
        } else {
            &["bun"]
        };
        let mut bun_exe: Option<std::path::PathBuf> = None;
        for cand in bun_candidates {
            let p = std::path::PathBuf::from(cand);
            if p.is_file() {
                bun_exe = Some(p);
                break;
            }
        }
        let Some(bun_exe) = bun_exe else {
            eprintln!("skipping: no bun executable found (CI without bootstrap bun)");
            return;
        };

        let temp_path = std::env::temp_dir().join(format!(
            "tsp-pipeline-nanoid-{}.tsx",
            std::process::id()
        ));
        std::fs::write(&temp_path, &wrapped).expect("wrap must be writable");

        let output = std::process::Command::new(&bun_exe)
            .arg(&temp_path)
            .output()
            .expect("bun must run the generated wrap");
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let _ = std::fs::remove_file(&temp_path);

        assert!(
            output.status.success(),
            "generated wrap must run cleanly under bun\nstdout: {stdout}\nstderr: {stderr}"
        );
        assert!(
            stdout.contains("__TSP_OUT_V1__"),
            "bun must print the envelope marker; stdout: {stdout}"
        );
        // The body must be a 21-char nanoid from the url alphabet.
        // Scan stdout for 21 consecutive alphabet chars (no regex
        // dependency needed in this crate).
        let alphabet = "useandom-26T198340PX75pxJACKVERYMINDBUSHWOLF_GQZbfghjklqvwyzrict";
        let has_21_alphabet_run = stdout
            .as_bytes()
            .windows(21)
            .any(|window| window.iter().all(|b| alphabet.as_bytes().contains(b)));
        assert!(
            has_21_alphabet_run,
            "envelope must contain a 21-char nanoid; stdout: {stdout}"
        );
        assert!(
            !stdout.contains("ReferenceError") && !stdout.contains("SyntaxError"),
            "no JS errors allowed in the generated module; stdout: {stdout}"
        );
    }

    // -----------------------------------------------------------------
    // Slice 17b: zod 3.25.76 as `tsp:server.zod` built-in
    // -----------------------------------------------------------------

    #[test]
    fn wrap_for_bun_cli_inlines_zod_runtime_for_pages() {
        // Pin the wrap-string shape:
        //   1) the zod bundle is inlined inside an IIFE that
        //      returns `module.exports.z` (the zod namespace);
        //   2) the namespace is bound to a single const
        //      (`__tspZodNs__`) and re-exposed on the frozen
        //      `__tspServer` object under its public name `zod`;
        //   3) plan §16.4: framework API must NOT leak through
        //      `globalThis` (no `globalThis.zod = ...`).
        let body = "function GET() { return zod.object({a: zod.string()}).safeParse({a: 'hi'}); }";
        let wrapped = wrap_for_bun_cli(body, "GET", Some("{}"));
        assert!(
            wrapped.contains("var __tspZodNs__ =") || wrapped.contains("const __tspZodNs__ ="),
            "zod prelude must bind a single `__tspZodNs__` const; got prefix: {}",
            &wrapped[..wrapped.len().min(800)]
        );
        // The IIFE shape: the bundle body runs inside a function
        // that provides `module` and `exports` locals, then
        // returns `module.exports.z`. Verifying the literal
        // return line keeps the prelude honest about what it
        // exposes to `__tspServer.zod`.
        assert!(
            wrapped.contains("return module.exports.z;"),
            "zod prelude must return `module.exports.z` from its IIFE"
        );
        assert!(
            wrapped.contains("zod: __tspZodNs__"),
            "wrap must expose zod on __tspServer; got prefix: {}",
            &wrapped[..wrapped.len().min(1200)]
        );
        // The bundled source's top-level `var z` stays INSIDE the
        // IIFE (it's a `var`, so it's function-scoped to the
        // wrapper, not page-module-scoped). The only zod-related
        // identifier that escapes the IIFE is `__tspZodNs__`
        // (the return value bound to a `const` at module
        // scope), and the page sees it exclusively as
        // `__tspServer.zod`. Pin that escape hatch.
        assert!(
            wrapped.contains("const __tspZodNs__ = (function()"),
            "the wrap must bind the IIFE return to `const __tspZodNs__`"
        );
        // Plan §16.4: the framework API must not be published on
        // globalThis. The nanoid block already enforces this for
        // the id namespace; zod gets the same protection.
        for forbidden in [
            "globalThis.zod =",
            "globalThis.z =",
        ] {
            assert!(
                !wrapped.contains(forbidden),
                "zod must not be published on globalThis (plan §16.4); found `{forbidden}`"
            );
        }
    }

    #[test]
    fn rewrite_tsp_server_imports_accepts_zod_named_export() {
        // Plan §16.1/§16.4: zod is reached via
        //     import { zod } from "tsp:server";
        // which the rewriter collapses into
        //     const { zod } = __tspServer;
        let src = r#"import { zod } from "tsp:server";
export function GET() {
  return zod.object({ a: zod.string() }).safeParse({ a: "hi" });
}
"#;
        let rewritten = rewrite_tsp_server_imports(src).expect("rewrite should succeed");
        assert!(
            rewritten.contains("const { zod } = __tspServer;"),
            "rewriter must collapse the zod import into a single destructure; got: {rewritten}"
        );
        assert!(
            !rewritten.contains("from \"tsp:server\""),
            "the `from \"tsp:server\"` clause must be stripped after rewrite; got: {rewritten}"
        );
    }

    // -----------------------------------------------------------------
    // End-to-end pipeline test for zod (real Bun execution)
    //
    // Mirrors the nanoid pipeline test: take a production-shaped
    // .tsp file (`import { zod } from "tsp:server"` -> parse a body
    // -> respond JSON), run the full pipeline
    // (`tsx_to_js` -> `wrap_for_bun_cli` -> temp file -> spawn
    // `bun`), and assert the envelope. Catches JS-level errors
    // (e.g. IIFE-return-shadowed, double `var z` declaration
    // collision, or the `zod` destructure mis-binding) without
    // needing a 9-minute binary relink.
    // -----------------------------------------------------------------

    #[test]
    fn zod_pipeline_generates_runable_module_under_real_bun() {
        let source = r#"// Slice 17b regression test fixture.
import { zod } from "tsp:server";

export function GET() {
  const schema = zod.object({
    name: zod.string(),
    age: zod.coerce.number().int().min(0).max(150),
  });
  const result = schema.safeParse({ name: "alice", age: "30" });
  if (!result.success) {
    return new Response(JSON.stringify({ ok: false, issues: result.error.issues }), {
      status: 400,
      headers: { "content-type": "application/json" },
    });
  }
  return new Response(JSON.stringify({ ok: true, name: result.data.name, age: result.data.age }), {
    status: 200,
    headers: { "content-type": "application/json", "x-demo": "slice17b" },
  });
}
"#;
        let transformed = tsx_to_js(source).expect("tsx_to_js must succeed");
        assert!(
            !transformed.contains("from \"tsp:server\""),
            "import must be rewritten away; got: {transformed}"
        );
        assert!(
            transformed.contains("const { zod } = __tspServer;"),
            "rewriter must emit `const {{ zod }} = __tspServer;`; got: {transformed}"
        );

        let ctx_json = r#"{"method":"GET","path":"/zod","query":"","headers":{"host":"127.0.0.1:1"},"body_b64":""}"#;
        let wrapped = wrap_for_bun_cli(&transformed, "GET", Some(ctx_json));

        let bun_candidates: &[&str] = if cfg!(windows) {
            &[
                r"D:\GitHub\tsp\.bun-bootstrap\node_modules\bun\bin\bun.exe",
            ]
        } else {
            &["bun"]
        };
        let mut bun_exe: Option<std::path::PathBuf> = None;
        for cand in bun_candidates {
            let p = std::path::PathBuf::from(cand);
            if p.is_file() {
                bun_exe = Some(p);
                break;
            }
        }
        let Some(bun_exe) = bun_exe else {
            eprintln!("skipping: no bun executable found (CI without bootstrap bun)");
            return;
        };

        let temp_path = std::env::temp_dir().join(format!(
            "tsp-pipeline-zod-{}.tsx",
            std::process::id()
        ));
        std::fs::write(&temp_path, &wrapped).expect("wrap must be writable");

        let output = std::process::Command::new(&bun_exe)
            .arg(&temp_path)
            .output()
            .expect("bun must run the generated wrap");
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let _ = std::fs::remove_file(&temp_path);

        assert!(
            output.status.success(),
            "generated wrap must run cleanly under bun\nstdout: {stdout}\nstderr: {stderr}"
        );
        assert!(
            stdout.contains("__TSP_OUT_V1__"),
            "bun must print the envelope marker; stdout: {stdout}"
        );
        // The zod parse produced `{ ok: true, name: "alice", age: 30 }`
        // (the `coerce.number()` converted "30" to 30). The page
        // JSON-stringifies that object and returns it as the body,
        // so the envelope's `body` field is itself a JSON-encoded
        // string with the inner `"` escaped. Look for the three
        // escaped fragments -- they're the strongest evidence that
        // the page actually ran and produced the right envelope
        // without going through any zod or prelude error.
        assert!(
            stdout.contains(r#"\"ok\":true"#)
                && stdout.contains(r#"\"name\":\"alice\""#)
                && stdout.contains(r#"\"age\":30"#),
            "envelope must contain the parsed zod result; stdout: {stdout}"
        );
        assert!(
            stdout.contains("x-demo"),
            "envelope must carry the page's custom header"
        );
        assert!(
            !stdout.contains("ReferenceError") && !stdout.contains("SyntaxError"),
            "no JS errors allowed in the generated module; stdout: {stdout}"
        );
    }

    // -----------------------------------------------------------------
    // Slice 17c: bcryptjs 3.0.3 as `tsp:server.bcrypt` built-in
    // -----------------------------------------------------------------

    #[test]
    fn wrap_for_bun_cli_surfaces_bun_sql_factory_for_pages() {
        // Slice 17d: the `sql` namespace in `__tspServer` is
        // bun's native `require("bun").SQL` factory. No
        // `include_str!`, no IIFE -- the wrap simply does
        // `const { SQL } = require("bun");` at the top and
        // freezes the result onto `__tspServer.sql`. This
        // test pins the wrap-string shape so a refactor that
        // accidentally drops the `require("bun")` call (or
        // renames `__tspServer.sql`) is caught at unit-test
        // time, without spinning the real binary.
        let body = "function GET() { return 'ok'; }";
        let wrapped = wrap_for_bun_cli(body, "GET", Some("{}"));
        assert!(
            wrapped.contains(r#"const __tspSqlNs__ = require("bun").SQL;"#),
            "wrap must bridge bun's native SQL factory to `__tspSqlNs__` via `require(\"bun\")`; got prefix: {}",
            &wrapped[..wrapped.len().min(1200)]
        );
        assert!(
            wrapped.contains("sql: __tspSqlNs__"),
            "wrap must expose sql on __tspServer; got prefix: {}",
            &wrapped[..wrapped.len().min(1500)]
        );
        // Plan §16.4: framework API must not be on globalThis.
        for forbidden in ["globalThis.sql =", "globalThis.SQL ="] {
            assert!(
                !wrapped.contains(forbidden),
                "sql must not be published on globalThis (plan §16.4); found `{forbidden}`"
            );
        }
    }

    #[test]
    fn rewrite_tsp_server_imports_accepts_sql_named_export() {
        // Plan §16.1/§16.4: sql is reached via
        //     import { sql } from "tsp:server";
        // which the rewriter collapses into
        //     const { sql } = __tspServer;
        let src = r#"import { sql } from "tsp:server";
export async function GET() {
  const conn = await sql`SELECT 1`;
  conn.close();
  return new Response("ok");
}
"#;
        let rewritten = rewrite_tsp_server_imports(src).expect("rewrite should succeed");
        assert!(
            rewritten.contains("const { sql } = __tspServer;"),
            "rewriter must collapse the sql import into a single destructure; got: {rewritten}"
        );
        assert!(
            !rewritten.contains("from \"tsp:server\""),
            "the `from \"tsp:server\"` clause must be stripped after rewrite; got: {rewritten}"
        );
    }

    #[test]
    fn sql_pipeline_generates_runable_module_under_real_bun() {
        // Mirrors the nanoid / zod / bcrypt pipeline tests:
        // take a production-shaped `.tsp` file, run the full
        // pipeline (tsx_to_js -> wrap_for_bun_cli -> temp file
        // -> spawn `bun`), and assert the envelope. Catches
        // JS-level errors (e.g. `require("bun")` failing in
        // the synthetic `bun:main` context, or the
        // `__tspServer.sql` binding mis-wiring) without needing
        // a 9-minute binary relink.
        //
        // We use bun:sqlite (no MySQL needed) so the test is
        // self-contained: write a tiny in-memory DB, INSERT,
        // SELECT, return the row. The page mirrors
        // `routes/sql_demo.tsp` in production.
        let source = r#"// Slice 17d regression test fixture.
import { sql } from "tsp:server";

export async function GET(ctx) {
  // `sql(url)` is the factory call (returns a connection
  // function from bun's per-worker pool). `sql\`url\`` would
  // be a QUERY, not a connect -- easy to confuse. The page
  // mirrors `routes/sql_demo.tsp` in production.
  const url = "sqlite://" + (process.env.TSP_TEST_DB_FILE || ":memory:");
  const conn = await sql(url);
  try {
    await conn`CREATE TABLE IF NOT EXISTS sql_pipe (id INTEGER PRIMARY KEY, label TEXT NOT NULL)`;
    await conn`INSERT OR REPLACE INTO sql_pipe (id, label) VALUES (1, ${"slice17d"})`;
    const [row] = await conn`SELECT id, label FROM sql_pipe WHERE id = 1`;
    return new Response(
      JSON.stringify({ ok: row && row.label === "slice17d", row }),
      {
        status: 200,
        headers: { "content-type": "application/json", "x-demo": "slice17d" },
      }
    );
  } finally {
    conn.close();
  }
}
"#;
        let transformed = tsx_to_js(source).expect("tsx_to_js must succeed");
        assert!(
            !transformed.contains("from \"tsp:server\""),
            "import must be rewritten away; got: {transformed}"
        );
        assert!(
            transformed.contains("const { sql } = __tspServer;"),
            "rewriter must emit `const {{ sql }} = __tspServer;`; got: {transformed}"
        );

        let ctx_json = r#"{"method":"GET","path":"/sql_pipe","query":"","headers":{"host":"127.0.0.1:1"},"body_b64":""}"#;
        let wrapped = wrap_for_bun_cli(&transformed, "GET", Some(ctx_json));

        let bun_candidates: &[&str] = if cfg!(windows) {
            &[
                r"D:\GitHub\tsp\.bun-bootstrap\node_modules\bun\bin\bun.exe",
            ]
        } else {
            &["bun"]
        };
        let mut bun_exe: Option<std::path::PathBuf> = None;
        for cand in bun_candidates {
            let p = std::path::PathBuf::from(cand);
            if p.is_file() {
                bun_exe = Some(p);
                break;
            }
        }
        let Some(bun_exe) = bun_exe else {
            eprintln!("skipping: no bun executable found (CI without bootstrap bun)");
            return;
        };

        let temp_path = std::env::temp_dir().join(format!(
            "tsp-pipeline-sql-{}.tsx",
            std::process::id()
        ));
        std::fs::write(&temp_path, &wrapped).expect("wrap must be writable");

        let output = std::process::Command::new(&bun_exe)
            .arg(&temp_path)
            .output()
            .expect("bun must run the generated wrap");
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let _ = std::fs::remove_file(&temp_path);

        assert!(
            output.status.success(),
            "generated wrap must run cleanly under bun\nstdout: {stdout}\nstderr: {stderr}"
        );
        assert!(
            stdout.contains("__TSP_OUT_V1__"),
            "bun must print the envelope marker; stdout: {stdout}"
        );
        // The page returned `{ ok: true, row: { id: 1, label: "slice17d" } }`.
        // Body is JSON-escaped inside the envelope, so look for
        // the escaped fragments. `bun:sqlite` is in-process (no
        // server needed) and the row count check confirms the
        // conn was real (not a stub).
        assert!(
            stdout.contains(r#"\"ok\":true"#) && stdout.contains(r#"\"label\":\"slice17d\""#),
            "envelope must contain the bun:sql row; stdout: {stdout}"
        );
        assert!(
            !stdout.contains("ReferenceError") && !stdout.contains("SyntaxError"),
            "no JS errors allowed in the generated module; stdout: {stdout}"
        );
    }

    #[test]
    fn wrap_for_bun_cli_surfaces_bun_password_for_pages() {
        // Slice 17c (revised): the `password` namespace in
        // `__tspServer` is `Bun.password` itself, surfaced
        // through the wrap preamble with a single
        // `const __tspPasswordNs__ = Bun.password;` binding
        // (no `include_str!`, no IIFE -- the page reuses bun's
        // native Rust-backed password API directly). This test
        // pins the wrap-string shape so a refactor that
        // accidentally drops the `Bun.password` bridge (or
        // renames `__tspServer.password`) is caught at unit-test
        // time, without spinning the real binary.
        let body = "function GET() { return password.hashSync('hello', { algorithm: 'bcrypt', cost: 4 }); }";
        let wrapped = wrap_for_bun_cli(body, "GET", Some("{}"));
        assert!(
            wrapped.contains("const __tspPasswordNs__ = Bun.password;"),
            "wrap must bridge bun's native password API to `__tspPasswordNs__` via `Bun.password`; got prefix: {}",
            &wrapped[..wrapped.len().min(1200)]
        );
        assert!(
            wrapped.contains("password: __tspPasswordNs__"),
            "wrap must expose password on __tspServer; got prefix: {}",
            &wrapped[..wrapped.len().min(1500)]
        );
        // Plan §16.4: framework API must not be on globalThis.
        for forbidden in [
            "globalThis.password =",
            "globalThis.bcrypt =",
        ] {
            assert!(
                !wrapped.contains(forbidden),
                "password must not be published on globalThis (plan §16.4); found `{forbidden}`"
            );
        }
    }

    #[test]
    fn rewrite_tsp_server_imports_accepts_password_named_export() {
        // Plan §16.1/§16.4: password is reached via
        //     import { password } from "tsp:server";
        // which the rewriter collapses into
        //     const { password } = __tspServer;
        let src = r#"import { password } from "tsp:server";
export function GET() {
  return password.hashSync("hello", { algorithm: "bcrypt", cost: 4 });
}
"#;
        let rewritten = rewrite_tsp_server_imports(src).expect("rewrite should succeed");
        assert!(
            rewritten.contains("const { password } = __tspServer;"),
            "rewriter must collapse the password import into a single destructure; got: {rewritten}"
        );
        assert!(
            !rewritten.contains("from \"tsp:server\""),
            "the `from \"tsp:server\"` clause must be stripped after rewrite; got: {rewritten}"
        );
    }

    #[test]
    fn password_pipeline_generates_runable_module_under_real_bun() {
        // Mirrors the nanoid / zod / sql pipeline tests: take a
        // production-shaped `.tsp` file, run the full pipeline
        // (tsx_to_js -> wrap_for_bun_cli -> temp file -> spawn
        // `bun`), and assert the envelope. Catches JS-level
        // errors (e.g. `Bun.password` failing in the synthetic
        // `bun:main` context, or the `__tspServer.password`
        // binding mis-wiring) without needing a 9-minute binary
        // relink.
        //
        // We use cost=4 to keep the test under a second. The
        // page mirrors `routes/password.tsp` in production.
        let source = r#"// Slice 17c (revised) regression test fixture.
import { password } from "tsp:server";

export function GET() {
  const bcrypt = password.hashSync("hello", { algorithm: "bcrypt", cost: 4 });
  const argon = password.hashSync("hello", { algorithm: "argon2id" });
  const bcryptOk = password.verifySync("hello", bcrypt);
  const bcryptNo = password.verifySync("world", bcrypt);
  return new Response(
    JSON.stringify({
      ok: bcryptOk === true && bcryptNo === false,
      isBcrypt: bcrypt.startsWith("$2b$"),
      isArgon: argon.startsWith("$argon2id$"),
      bcryptSample: bcrypt.slice(0, 7),
      argonSample: argon.slice(0, 10),
    }),
    {
      status: 200,
      headers: { "content-type": "application/json", "x-demo": "slice17c" },
    }
  );
}
"#;
        let transformed = tsx_to_js(source).expect("tsx_to_js must succeed");
        assert!(
            !transformed.contains("from \"tsp:server\""),
            "import must be rewritten away; got: {transformed}"
        );
        assert!(
            transformed.contains("const { password } = __tspServer;"),
            "rewriter must emit `const {{ password }} = __tspServer;`; got: {transformed}"
        );

        let ctx_json = r#"{"method":"GET","path":"/password","query":"","headers":{"host":"127.0.0.1:1"},"body_b64":""}"#;
        let wrapped = wrap_for_bun_cli(&transformed, "GET", Some(ctx_json));

        let bun_candidates: &[&str] = if cfg!(windows) {
            &[
                r"D:\GitHub\tsp\.bun-bootstrap\node_modules\bun\bin\bun.exe",
            ]
        } else {
            &["bun"]
        };
        let mut bun_exe: Option<std::path::PathBuf> = None;
        for cand in bun_candidates {
            let p = std::path::PathBuf::from(cand);
            if p.is_file() {
                bun_exe = Some(p);
                break;
            }
        }
        let Some(bun_exe) = bun_exe else {
            eprintln!("skipping: no bun executable found (CI without bootstrap bun)");
            return;
        };

        let temp_path = std::env::temp_dir().join(format!(
            "tsp-pipeline-password-{}.tsx",
            std::process::id()
        ));
        std::fs::write(&temp_path, &wrapped).expect("wrap must be writable");

        let output = std::process::Command::new(&bun_exe)
            .arg(&temp_path)
            .output()
            .expect("bun must run the generated wrap");
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let _ = std::fs::remove_file(&temp_path);

        assert!(
            output.status.success(),
            "generated wrap must run cleanly under bun\nstdout: {stdout}\nstderr: {stderr}"
        );
        assert!(
            stdout.contains("__TSP_OUT_V1__"),
            "bun must print the envelope marker; stdout: {stdout}"
        );
        // The page produced
        // `{ ok: true, isBcrypt: true, isArgon: true,
        //    bcryptSample: "$2b$04$", argonSample: "$argon2id" }`.
        // Body is JSON-escaped inside the envelope, so look for
        // the escaped fragments. The presence of the
        // algorithm-specific prefixes proves that the real
        // bcrypt + argon2id implementations ran (not a stub).
        assert!(
            stdout.contains(r#"\"ok\":true"#)
                && stdout.contains(r#"\"isBcrypt\":true"#)
                && stdout.contains(r#"\"isArgon\":true"#)
                && stdout.contains(r#"\"bcryptSample\":\"$2b$04$\""#)
                && stdout.contains(r#"\"argonSample\":\"$argon2id$\""#),
            "envelope must contain the password result fragments\n  ok={}\n  isBcrypt={}\n  isArgon={}\n  bcryptSample={}\n  argonSample={}\n  stdout: {stdout}",
            stdout.contains(r#"\"ok\":true"#),
            stdout.contains(r#"\"isBcrypt\":true"#),
            stdout.contains(r#"\"isArgon\":true"#),
            stdout.contains(r#"\"bcryptSample\":\"$2b$04$\""#),
            stdout.contains(r#"\"argonSample\":\"$argon2id\""#),
        );
        assert!(
            !stdout.contains("ReferenceError") && !stdout.contains("SyntaxError"),
            "no JS errors allowed in the generated module; stdout: {stdout}"
        );
    }
}


