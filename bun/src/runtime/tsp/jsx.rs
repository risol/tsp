//! Minimal TSX -> JS pre-processor for TSP PoC 1 slice 6.
//!
//! See `tsp-plan.md` sect.11 (TSP JSX runtime) and sect.3.1
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
//! These are documented in `tsp-plan.md` sect.10.4 ("prohibit shape
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

/// Transform a `.tsp` source string into a runnable TSX module body for the
/// embedded ESM entry point. Bun performs the actual TypeScript/TSX lowering.
pub fn tsx_to_js(source: &str) -> Result<String, JsxError> {
    let source = rewrite_tsp_server_imports(source)?;
    Ok(rewrite_fragment_exports(&source))
}

/// Transform relative local imports into absolute ESM imports that the
/// synthetic `bun:main` module can resolve regardless of the generated
/// wrapper's temporary directory. Static imports become top-level
/// `await import(...)` bindings because the wrapper must initialize the
/// per-request TSP bridge before evaluating a dependency's own module graph.
/// This keeps dependency loading in Bun's native ESM loader.
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

/// Prepare a temporary copy of the local TypeScript module graph for the
/// standard Bun CLI path. Embedded workers have a native loader hook for
/// `tsp:server`; the CLI does not, so every copied dependency gets the same
/// request-scoped bridge rewrite as the route entry point.
pub fn prepare_cli_module_graph(
    source: &str,
    importer_dir: &std::path::Path,
    generation: u64,
) -> Result<(String, std::path::PathBuf), JsxError> {
    let root = std::env::temp_dir().join(format!(
        "tsp-cli-graph-{}-{}",
        std::process::id(),
        generation
    ));
    let copied_root = root.join("pages");
    std::fs::create_dir_all(&copied_root).map_err(|_| JsxError::UnsupportedShape {
        line: 0,
        reason: "could not create the CLI module graph directory",
    })?;
    copy_cli_graph(importer_dir, importer_dir, &copied_root)?;

    let mut rewritten = rewrite_local_imports_for_generation(source, importer_dir, generation)?;
    for entry in walk_cli_graph(importer_dir)? {
        let relative = entry.strip_prefix(importer_dir).map_err(|_| JsxError::UnsupportedShape {
            line: 0,
            reason: "could not map a CLI module graph path",
        })?;
        let copied = copied_root.join(relative);
        let old_url = file_url(&entry, Some(generation));
        let new_url = file_url(&copied, Some(generation));
        rewritten = rewritten.replace(&old_url, &new_url);
    }
    Ok((rewritten, root))
}

fn walk_cli_graph(root: &std::path::Path) -> Result<Vec<std::path::PathBuf>, JsxError> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(root).map_err(|_| JsxError::UnsupportedShape {
        line: 0,
        reason: "could not read the CLI module graph",
    })? {
        let entry = entry.map_err(|_| JsxError::UnsupportedShape {
            line: 0,
            reason: "could not read a CLI module graph entry",
        })?;
        let path = entry.path();
        if entry.file_type().map_err(|_| JsxError::UnsupportedShape {
            line: 0,
            reason: "could not inspect a CLI module graph entry",
        })?.is_dir() {
            files.extend(walk_cli_graph(&path)?);
        } else if path.extension().is_some_and(|extension| {
            extension == "ts" || extension == "tsx" || extension == "js" || extension == "jsx"
        }) {
            files.push(path);
        }
    }
    Ok(files)
}

fn copy_cli_graph(
    original_root: &std::path::Path,
    current: &std::path::Path,
    copied_root: &std::path::Path,
) -> Result<(), JsxError> {
    for entry in std::fs::read_dir(current).map_err(|_| JsxError::UnsupportedShape {
        line: 0,
        reason: "could not read the CLI module graph",
    })? {
        let entry = entry.map_err(|_| JsxError::UnsupportedShape {
            line: 0,
            reason: "could not read a CLI module graph entry",
        })?;
        let source_path = entry.path();
        let relative = source_path.strip_prefix(original_root).map_err(|_| JsxError::UnsupportedShape {
            line: 0,
            reason: "could not map a CLI module graph entry",
        })?;
        let destination = copied_root.join(relative);
        if entry.file_type().map_err(|_| JsxError::UnsupportedShape {
            line: 0,
            reason: "could not inspect a CLI module graph entry",
        })?.is_dir() {
            std::fs::create_dir_all(&destination).map_err(|_| JsxError::UnsupportedShape {
                line: 0,
                reason: "could not create a CLI module graph directory",
            })?;
            copy_cli_graph(original_root, &source_path, copied_root)?;
        } else if source_path.extension().is_some_and(|extension| {
            extension == "ts" || extension == "tsx" || extension == "js" || extension == "jsx"
        }) {
            let content = std::fs::read_to_string(&source_path).map_err(|_| JsxError::UnsupportedShape {
                line: 0,
                reason: "a CLI module graph file was not valid UTF-8",
            })?;
            let content = rewrite_tsp_server_imports(&content)?
                .replace("__tspServer", "globalThis[Symbol.for('tsp.server.bridge')]");
            std::fs::write(destination, content).map_err(|_| JsxError::UnsupportedShape {
                line: 0,
                reason: "could not write a CLI module graph file",
            })?;
        }
    }
    Ok(())
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

/// Rewrite a dynamic local import to an absolute ESM URL. Dynamic imports are
/// expressions, so they can remain in place without changing module syntax.
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
    // Preserve the dynamic-import expression and replace only its specifier.
    let mut result = String::with_capacity(line.len() + 8 + url.len());
    result.push_str(&line[..start]);
    result.push_str("import(\"");
    result.push_str(&url);
    result.push_str(&line[end..]);
    Ok(result)
}

/// Rewrite a static `import { a, b as c } from "./y"` declaration into a
/// `const { a, b: c } = await import("file:///...y?...")`. The named-binding
/// shape is the only one the current rewriter accepts (plan §16.1:
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
    replacement.push_str(" } = await import(\"");
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
    let canonical = crate::path::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
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
                            | "sql"
                            | "Image"
                            | "util"
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
/// `vendor/nanoid/index.js` lives next to this module. The
/// file is vendored under `bun/src/runtime/tsp/vendor/nanoid/`
/// (alongside the zod 4.4.3 bundle) so the build is
/// self-contained: CI does NOT need `bun install` or any
/// `node_modules/` to be present at the workspace root. The
/// pre-`vendor` location was `../../../../node_modules/nanoid/`
/// (the v1 server's `node_modules` at the repo root, which is
/// gitignored and absent in fresh checkouts), and the CI native
/// jobs did not run `bun install` -- so the include_str silently
/// failed on every fresh-checkout CI run since the nanoid
/// embedding commit (`e821af4bca`). The vendored file makes
/// the build hermetic. If nanoid is upgraded, regenerate
/// `vendor/nanoid/index.js` from the new `node_modules/nanoid/`
/// and commit the new file in place -- the `include_str!` hash
/// will refresh the next `cargo build`.
const NANOID_RAW_SOURCE: &str = include_str!("vendor/nanoid/index.js");

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
// per-request parse cost. The current runtime already requires Bun as the host
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

/// `ctx_json` is the JSON-serialised `Context` (spec
/// sect.13). If `Some`, the preamble parses it and passes
/// the resulting object as the page handler's only
/// argument. If `None`, the handler is called with no
/// argument so legacy zero-arg fixtures keep working.
pub fn wrap_for_bun_cli(transformed: &str, method: &str, ctx_json: Option<&str>) -> String {
    wrap_for_bun_cli_inner(transformed, method, ctx_json, false)
}

fn embedded_response_block() -> &'static str {
    r###"function __tspEncodeBody__(__tspBytes__) {
 let __tspBinary__ = '';
 const __tspChunk__ = 0x8000;
 for (let __tspOffset__ = 0; __tspOffset__ < __tspBytes__.length; __tspOffset__ += __tspChunk__) {
  __tspBinary__ += String.fromCharCode(...__tspBytes__.subarray(__tspOffset__, __tspOffset__ + __tspChunk__));
 }
 return btoa(__tspBinary__);
}
function __tspPublishEnvelope__(__tspType__, __tspStatus__, __tspHeaders__, __tspBody__, __tspBodyB64__) {
 const __tspCookieHeaders__ = Array.isArray(__tspCookieWrites) ? __tspCookieWrites.map((__line__) => ['Set-Cookie', __line__]) : [];
 const __tspEnvelope__ = {type: __tspType__, status: __tspStatus__, headers: __tspHeaders__.concat(__tspCookieHeaders__), body: __tspBody__, service_logs: __tspServiceLogs, session_writes: __tspSessionWrites};
 if (__tspBodyB64__ !== undefined) __tspEnvelope__.body_b64 = __tspBodyB64__;
 globalThis.__tspEmbeddedResponse = JSON.stringify(__tspEnvelope__);
}
function __tspPublishResponse__(__tspResponse__) {
 return __tspResponse__.arrayBuffer().then((__tspBuffer__) => {
  const __tspHeaders__ = [];
  for (const [__k__, __v__] of __tspResponse__.headers) __tspHeaders__.push([__k__, __v__]);
  __tspPublishEnvelope__('response', __tspResponse__.status, __tspHeaders__, '', __tspEncodeBody__(new Uint8Array(__tspBuffer__)));
 });
}
const __tspAsyncRender__ = {};
function __tspRenderNodeSync__(__node__, __child__) {
 if (__node__ && typeof __node__.then === 'function') throw __tspAsyncRender__;
 if (__node__ == null) {
  if (__child__) return '';
  const __tspType__ = (typeof __node__);
  throw new Error('TSP3001: handler returned unsupported value ' + __tspType__.charAt(0).toUpperCase() + __tspType__.slice(1) + '. Expected HtmlNode or Response.');
 }
 if (__child__ && typeof __node__ === 'boolean') return '';
 if (typeof __node__ === 'string') return __child__ ? __tspEscape__(__node__) : __node__;
 if (__child__ && (typeof __node__ === 'number' || typeof __node__ === 'bigint')) return String(__node__);
 if (Array.isArray(__node__)) {
  let __body__ = '';
  for (const __item__ of __node__) __body__ += __tspRenderNodeSync__(__item__, true);
  return __body__;
 }
 if (__node__.__tspRaw !== undefined) return __node__.__tspRaw;
 if (typeof __node__ !== 'object' || !__node__.props) {
  const __type__ = typeof __node__;
  const __typeCap__ = __type__.charAt(0).toUpperCase() + __type__.slice(1);
  throw new Error(__child__ ? 'TSP3102: object cannot be rendered as an HTML child' : ('TSP3001: handler returned unsupported value ' + __typeCap__ + '. Expected HtmlNode or Response.'));
 }
 const __type__ = __node__.type;
 const __props__ = __node__.props || {};
 if (__type__ === React.Fragment) return __tspRenderNodeSync__(__props__.children, true);
 if (typeof __type__ === 'function') return __tspRenderNodeSync__(__type__(__props__), true);
 if (typeof __type__ !== 'string') throw new Error('TSP3103: unsupported JSX element type');
 let __attrs__ = '';
 for (const [__rawName__, __value__] of Object.entries(__props__)) {
  if (__rawName__ === 'children' || __rawName__ === 'key' || __rawName__ === 'ref') continue;
  const __name__ = __rawName__ === 'className' ? 'class' : (__rawName__ === 'htmlFor' ? 'for' : __rawName__);
  if (typeof __value__ === 'function') throw new Error('TSP3105: function-valued HTML attributes are not serializable');
  if (__value__ == null || __value__ === false) continue;
  if (__value__ === true) { __attrs__ += ' ' + __name__; continue; }
  if (typeof __value__ === 'object') throw new Error('TSP3104: object-valued HTML attributes are not serializable');
  __attrs__ += ' ' + __name__ + '="' + __tspEscape__(__value__) + '"';
 }
 const __void__ = /^(area|base|br|col|embed|hr|img|input|link|meta|param|source|track|wbr)$/i.test(__type__);
 if (__void__) return '<' + __type__ + __attrs__ + '>';
 return '<' + __type__ + __attrs__ + '>' + __tspRenderNodeSync__(__props__.children, true) + '</' + __type__ + '>';
}
function __tspPublishError__(e) {
 if (e && Number.isInteger(e.status)) return __tspPublishResponse__(new Response(String(e.message || ''), {status: e.status, headers: e.headers || {}}));
 let __tspErrJson__;
 try { __tspErrJson__ = JSON.stringify({kind: 'tsp_error', error: (e && e.name) || 'Error', message: (e && e.message) || String(e), stack: (e && e.stack) || ''}); } catch { __tspErrJson__ = JSON.stringify({kind: 'tsp_error', error: 'Error', message: String(e), stack: ''}); }
 return __tspPublishResponse__(new Response(__tspErrJson__, {status: 500, headers: {'content-type': 'application/json', 'x-tsp-error': 'page'} }));
}
function __tspPublishResult__(__tspResult__) {
 if (__tspResult__ instanceof Response) return __tspPublishResponse__(__tspResult__);
 if (typeof __tspResult__ === 'string') {
  __tspPublishEnvelope__('html', 200, [], __tspResult__);
  return;
 }
 try {
  __tspPublishEnvelope__('html', 200, [], __tspRenderNodeSync__(__tspResult__, false));
  return;
 } catch (__tspError__) {
  if (__tspError__ !== __tspAsyncRender__) throw __tspError__;
 }
 return __tspRenderNode__(__tspResult__, false).then((__tspBody__) => __tspPublishEnvelope__('html', 200, [], __tspBody__));
}
try {
 const __tspHandlerResult__ = __tspContext === undefined ? __tspHandler__() : __tspHandler__(__tspContext);
 if (__tspHandlerResult__ && typeof __tspHandlerResult__.then === 'function') {
  __tspHandlerResult__.then(__tspPublishResult__).catch(__tspPublishError__);
 } else {
  const __tspPublished__ = __tspPublishResult__(__tspHandlerResult__);
  if (__tspPublished__ && typeof __tspPublished__.catch === 'function') __tspPublished__.catch(__tspPublishError__);
 }
} catch (e) { __tspPublishError__(e); }
"###
}

fn wrap_for_bun_cli_inner(
    transformed: &str,
    method: &str,
    ctx_json: Option<&str>,
    embedded: bool,
) -> String {
    let mut out = String::with_capacity(
        transformed.len()
            + 1024
            + nanoid_prelude().len()
            + zod_prelude().len(),
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
    // Keep native Bun lookups lazy. The embedded worker intentionally has a
    // smaller initialization path than the full CLI, and an unrelated route
    // must not touch native getters or `require(\"bun\")` merely because the
    // framework namespace exposes them. `tsp:server.sql` below follows the
    // same rule and only calls `require("bun")` when SQL is requested.
    // Slice 18: surface bun 1.4's built-in utilities to the page
    // via a single `util` namespace on `__tspServer`. The page
    // reaches them through
    //     import { util } from "tsp:server";
    //     const id = util.randomUUIDv7();
    //     const h  = util.hash(new TextEncoder().encode("hi"));
    // etc. We deliberately wrap `Bun.env` (instead of
    // forwarding it directly) so the page cannot call
    // `Bun.env.toJSON()` to dump every environment variable
    // (which would leak DB_PW / API_KEY / etc. that plan §17.1
    // says the host must not see). The wrapper exposes
    // `get(key)` and `has(key)` only; pages read individual
    // variables they were told to use, never the whole env.
    // The high-risk bun builtins -- Bun.serve, Bun.spawn,
    // Bun.FFI, Bun.S3Client, Bun.connect, Bun.mmap, Bun.Cookie
    // -- are intentionally NOT exposed. Pages that genuinely
    // need a subprocess / raw socket / native call belong in
    // the host (worker / native service layer), not in a
    // request-scoped page module.
    out.push_str(
        "const __tspUtilNs__ = {};\n\
         Object.defineProperties(__tspUtilNs__, {\n\
         \x20 randomUUIDv7: { enumerable: true, get: () => Bun.randomUUIDv7 },\n\
         \x20 hash: { enumerable: true, get: () => Bun.hash },\n\
         \x20 CryptoHasher: { enumerable: true, get: () => Bun.CryptoHasher },\n\
         \x20 Glob: { enumerable: true, get: () => Bun.Glob },\n\
         \x20 TOML: { enumerable: true, get: () => Bun.TOML },\n\
         \x20 YAML: { enumerable: true, get: () => Bun.YAML },\n\
         \x20 markdown: { enumerable: true, get: () => Bun.markdown },\n\
         \x20 escapeHTML: { enumerable: true, get: () => Bun.escapeHTML },\n\
         \x20 gzipSync: { enumerable: true, get: () => Bun.gzipSync },\n\
         \x20 gunzipSync: { enumerable: true, get: () => Bun.gunzipSync },\n\
         \x20 file: { enumerable: true, get: () => Bun.file },\n\
         \x20 write: { enumerable: true, get: () => Bun.write },\n\
         \x20 which: { enumerable: true, get: () => Bun.which },\n\
         \x20 peek: { enumerable: true, get: () => Bun.peek },\n\
         \x20 deepEquals: { enumerable: true, get: () => Bun.deepEquals },\n\
         \x20 deepMatch: { enumerable: true, get: () => Bun.deepMatch },\n\
         \x20 nanoseconds: { enumerable: true, get: () => Bun.nanoseconds },\n\
         \x20 env: { enumerable: true, get: () => Object.freeze({ get: (k) => Bun.env[k], has: (k) => k in Bun.env }) },\n\
         \x20 password: { enumerable: true, get: () => Bun.password },\n\
         });\n\
         Object.freeze(__tspUtilNs__);\n\
         "
    );
    out.push_str("// Generated by TSP PoC 1 slice 16b (jsx.rs)\n");
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
          const __tspServer = {};\n\
          Object.assign(__tspServer, {json: __tspJson__, redirect: __tspRedirect__, text: __tspText__, html: __tspHtml__, notFound: __tspNotFound__, HttpError: __tspHttpError__, fragment: __tspFragment__, raw: __tspRaw__, nanoid: __tspNanoid, customAlphabet: __tspNanoidCustomAlphabet, customRandom: __tspNanoidCustomRandom, random: __tspNanoidRandom, zod: __tspZodNs__, util: __tspUtilNs__});\n\
          Object.defineProperty(__tspServer, 'sql', { enumerable: true, get: () => require(\"bun\").SQL });\n\
          Object.defineProperty(__tspServer, 'Image', { enumerable: true, get: () => Bun.Image });\n\
          Object.freeze(__tspServer);\n\
          // Local ESM dependencies are evaluated by Bun's native module loader\n\
          // scope. The loader rewrites their `tsp:server` imports to\n\
          // this non-enumerable symbol bridge, so expose the current request\n\
          // namespace before the route dynamically imports them.\n\
          Object.defineProperty(globalThis, Symbol.for('tsp.server.bridge'), { configurable: true, value: __tspServer });\n"
    );
    out.push_str(
        "function __tspEscape__(__value__) {\n\
         \x20 return String(__value__).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/\\\"/g, '&quot;').replace(/'/g, '&#39;');\n\
         }\n\
         async function __tspRenderNode__(__node__, __child__) {\n\
         \x20 __node__ = await __node__;\n\
         \x20 if (__node__ == null) {\n\
         \x20\x20 if (__child__) return '';\n\
         \x20\x20 const __tspType__ = (typeof __node__);\n\
         \x20\x20 throw new Error('TSP3001: handler returned unsupported value ' + __tspType__.charAt(0).toUpperCase() + __tspType__.slice(1) + '. Expected HtmlNode or Response.');\n\
         \x20 }\n\
         \x20 if (__child__ && typeof __node__ === 'boolean') return '';\n\
         \x20 if (typeof __node__ === 'string') return __child__ ? __tspEscape__(__node__) : __node__;\n\
         \x20 if (__child__ && (typeof __node__ === 'number' || typeof __node__ === 'bigint')) return String(__node__);\n\
         \x20 if (Array.isArray(__node__)) return (await Promise.all(__node__.map(__n__ => __tspRenderNode__(__n__, true)))).join('');\n\
         \x20 if (__node__.__tspRaw !== undefined) return __node__.__tspRaw;\n\
         \x20 if (typeof __node__ !== 'object' || !__node__.props) {
         \x20\x20 // Spec section 6.3 / plan section 10.4: a top-level
         \x20\x20 // handler return value that is not
         \x20\x20 // a JSX shape (no `.props` field)
         \x20\x20 // is a contract violation. The
         \x20\x20 // typed `TSP3001` prefix is the
         \x20\x20 // application-facing error code
         \x20\x20 // (contract item 5); a non-top-level
         \x20\x20 // value (a child of an existing
         \x20\x20 // JSX node) is a JSX rendering
         \x20\x20 // error (`TSP3102`). The
         \x20\x20 // distinction is the `__child__`
         \x20\x20 // flag: the top-level caller
         \x20\x20 // passes `false`, recursive
         \x20\x20 // children pass `true`.
         \x20\x20 const __tspType__ = (typeof __node__);
         \x20\x20 const __tspTypeCap__ = __tspType__.charAt(0).toUpperCase() + __tspType__.slice(1);
         \x20\x20 throw new Error(__child__ ? 'TSP3102: object cannot be rendered as an HTML child' : ('TSP3001: handler returned unsupported value ' + __tspTypeCap__ + '. Expected HtmlNode or Response.'));
         \x20 }
         \x20 const __type__ = __node__.type;

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
             // Keep ctx.signal as a real AbortSignal for route code. The\n\
             // embedded worker is started with stdin redirected to null and\n\
             // receives control messages over its native worker socket. The\n\
             // old subprocess bridge used a stdin abort marker, but that\n\
             // channel does not exist for the embedded worker. Do not access\n\
             // process.stdin from generated route code; native Cancel ->\n\
             // AbortController wiring belongs to the worker protocol.\n\
             const __tspAbortCtrl__ = new AbortController();\n\
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
    if ctx_json.is_none() {
        out.push_str("const __tspContext = undefined;\n");
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
    // §32.1 dev error page: a non-HttpError throw inside
    // the page handler no longer `throw e` to the outer
    // IIFE (which would exit with 1 and lose the error
    // body); the inner catch now builds a 500 response
    // with a JSON body that names the error, message,
    // and stack. The host reads this body and decides
    // whether to expose the details to the client
    // (`TSP_DEVELOPMENT=1` -> dev error page HTML;
    // prod -> generic 500 with the wire body preserved
    // for the application to log).
    if !embedded {
        out.push_str(
        "const __tspResultPromise__ = Promise.resolve().then(() => __tspContext === undefined ? __tspHandler__() : __tspHandler__(__tspContext)).then(async (__result__) => typeof __result__ === 'object' && __result__ !== null && !(__result__ instanceof Response) ? await __tspRenderNode__(__result__, false) : __result__).catch((e) => {\n\
         \x20 if (e && Number.isInteger(e.status)) return new Response(String(e.message || ''), {status: e.status, headers: e.headers || {}});\n\
         \x20 let __tspErrJson__;\n\
         \x20 try { __tspErrJson__ = JSON.stringify({kind: 'tsp_error', error: (e && e.name) || 'Error', message: (e && e.message) || String(e), stack: (e && e.stack) || ''}); } catch { __tspErrJson__ = JSON.stringify({kind: 'tsp_error', error: 'Error', message: String(e), stack: ''}); }\n\
         \x20 return new Response(__tspErrJson__, {status: 500, headers: {'content-type': 'application/json', 'x-tsp-error': 'page'}});\n\
         });\n",
    );
    }
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
    if embedded {
        out.push_str(embedded_response_block());
    } else {
        out.push_str(
        "(async () => {\n         \x20let __tspBody__, __tspStatus__, __tspHeaders__, __tspType__;\n         \x20const __tspResult__ = await __tspResultPromise__;\n         \x20__tspHeaders__ = [];\n         \x20if (__tspResult__ instanceof Response) {\n         \x20\x20__tspType__ = 'response';\n         \x20\x20__tspStatus__ = __tspResult__.status;\n         \x20\x20for (const [__k__, __v__] of __tspResult__.headers) __tspHeaders__.push([__k__, __v__]);\n         \x20\x20__tspBody__ = await __tspResult__.text();\n         \x20} else if (typeof __tspResult__ === 'string') {\n         \x20\x20__tspType__ = 'html';\n         \x20\x20__tspStatus__ = 200;\n         \x20\x20__tspBody__ = __tspResult__;\n         \x20} else {\n         \x20\x20// Spec §6.3 / plan §10.4: an\n         \x20\x20// invalid handler return value\n         \x20\x20// (object, number, boolean,\n         \x20\x20// etc.) is a contract violation.\n         \x20\x20// The typed `TSP3001` prefix is\n         \x20\x20// the application-facing error\n         \x20\x20// code (contract item 5 / plan\n         \x20\x20// §10.4); the inner catch wraps\n         \x20\x20// it in a 500 with a JSON body\n         \x20\x20// that the user can grep for.\n         \x20\x20// The type name is capitalized\n         \x20\x20// to match the plan's message\n         \x20\x20// (Object / Number / etc.).\n         \x20\x20const __tspType__ = (typeof __tspResult__);\n         \x20\x20const __tspTypeCap__ = __tspType__.charAt(0).toUpperCase() + __tspType__.slice(1);\n         \x20\x20throw new Error('TSP3001: handler returned unsupported value ' + __tspTypeCap__ + '. Expected HtmlNode or Response.');\n         \x20}\n         \x20// Merge runtime cookie writes into the outgoing headers\n         \x20// (spec sect.15: cookie writes MUST be reflected even when\n         \x20// the handler returns an HtmlNode). Each write becomes a\n         \x20// separate Set-Cookie line so multiple cookies on one\n         \x20// request don't collapse via the response's flatten loop.\n         \x20if (Array.isArray(__tspCookieWrites)) {\n         \x20\x20for (const __cookieLine__ of __tspCookieWrites) {\n         \x20\x20\x20__tspHeaders__.push(['Set-Cookie', __cookieLine__]);\n         \x20\x20}\n         \x20}\n         \x20const __tspEnvelope__ = JSON.stringify({type: __tspType__, status: __tspStatus__, headers: __tspHeaders__, body: __tspBody__, service_logs: __tspServiceLogs, session_writes: __tspSessionWrites});\n         \x20__tspConsoleLog('__TSP_OUT_V1__' + '\\n' + __tspEnvelope__);\n         process.exit(0);\n         })().catch((e) => { console.error(String(e && e.stack || e)); process.exit(1); });\n"
        );
        // Let Bun finish the current microtask checkpoint before exiting.
        // Calling process.exit() from this promise callback can tear down
        // JSC while VM::drainMicrotasks() is still running on Windows.
        out = out.replace("process.exit(0);", "process.exitCode = 0;");
        out = out.replace("process.exit(1);", "process.exitCode = 1;");
        // Response bodies may contain arbitrary bytes (for example PNG or
        // WebP output). Keep the subprocess envelope text-safe by carrying
        // those bytes in a separate Base64 field; the host restores them
        // before writing the HTTP response.
        out = out.replace(
            "(async () => {",
            "function __tspEncodeBody__(__tspBytes__) {\n         let __tspBinary__ = '';\n         const __tspChunk__ = 0x8000;\n         for (let __tspOffset__ = 0; __tspOffset__ < __tspBytes__.length; __tspOffset__ += __tspChunk__) {\n         __tspBinary__ += String.fromCharCode(...__tspBytes__.subarray(__tspOffset__, __tspOffset__ + __tspChunk__));\n         }\n         return btoa(__tspBinary__);\n         }\n         (async () => {",
        );
        out = out.replace(
            "let __tspBody__, __tspStatus__, __tspHeaders__, __tspType__;",
            "let __tspBody__, __tspBodyB64__, __tspStatus__, __tspHeaders__, __tspType__;",
        );
        out = out.replace(
            "__tspBody__ = await __tspResult__.text();",
            "__tspBodyB64__ = __tspEncodeBody__(new Uint8Array(await __tspResult__.arrayBuffer())); __tspBody__ = '';",
        );
        out = out.replace(
            "const __tspEnvelope__ = JSON.stringify({type: __tspType__, status: __tspStatus__, headers: __tspHeaders__, body: __tspBody__, service_logs: __tspServiceLogs, session_writes: __tspSessionWrites});",
            "const __tspEnvelope__ = JSON.stringify({type: __tspType__, status: __tspStatus__, headers: __tspHeaders__, body: __tspBody__ || '', service_logs: __tspServiceLogs, session_writes: __tspSessionWrites, ...(__tspBodyB64__ !== undefined ? {body_b64: __tspBodyB64__} : {})});",
        );
    }
    out
}

/// Adapt the normal TSP wrapper for a Bun VM that stays alive across requests.
///
/// The page execution and response-envelope logic remains identical to the
/// subprocess path. Only the transport changes: the envelope is placed on a
/// well-known global for the native worker to read, and the wrapper does not
/// call `process.exit`, which would tear down the embedded runtime.
pub fn wrap_for_embedded_worker(transformed: &str, method: &str, ctx_json: Option<&str>) -> String {
    // Keep the request wrapper as an ESM module. Local dependencies are
    // loaded by Bun's native module loader, and the generated wrapper uses
    // top-level dynamic imports after publishing its per-request bridge.
    let mut wrapped = wrap_for_bun_cli_inner(&transformed, method, ctx_json, true);
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

    fn response_body_from_stdout(stdout: &str) -> String {
        let encoded = stdout
            .split_once("\"body_b64\":\"")
            .and_then(|(_, rest)| rest.split_once('\"'))
            .map(|(body, _)| body)
            .expect("response envelope must contain body_b64");
        let mut bytes = Vec::with_capacity(encoded.len() / 4 * 3);
        for chunk in encoded.as_bytes().chunks_exact(4) {
            let value = |byte: u8| -> u8 {
                match byte {
                    b'A'..=b'Z' => byte - b'A',
                    b'a'..=b'z' => byte - b'a' + 26,
                    b'0'..=b'9' => byte - b'0' + 52,
                    b'+' => 62,
                    b'/' => 63,
                    _ => panic!("response body contains invalid base64"),
                }
            };
            let a = value(chunk[0]);
            let b = value(chunk[1]);
            let c = if chunk[2] == b'=' { 0 } else { value(chunk[2]) };
            let d = if chunk[3] == b'=' { 0 } else { value(chunk[3]) };
            bytes.push((a << 2) | (b >> 4));
            if chunk[2] != b'=' {
                bytes.push((b << 4) | (c >> 2));
            }
            if chunk[3] != b'=' {
                bytes.push((c << 6) | d);
            }
        }
        String::from_utf8(bytes).expect("test response body must be UTF-8")
    }

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
    fn exposes_image_as_a_lazy_tsp_server_export() {
        let src = r#"import { Image } from "tsp:server";
export function GET() {
  return new Response(String(typeof Image));
}
"#;
        let transformed = tsx_to_js(src).expect("tsx_to_js must succeed");
        assert!(
            transformed.contains("const { Image } = __tspServer;"),
            "Image must be an allow-listed named export; got: {transformed}"
        );
        let wrapped = wrap_for_bun_cli(&transformed, "GET", None);
        assert!(
            wrapped.contains("Object.defineProperty(__tspServer, 'Image', { enumerable: true, get: () => Bun.Image });"),
            "Image must be exposed through a lazy getter; got: {wrapped}"
        );
    }

    #[test]
    fn rewrites_relative_imports_to_file_urls_and_rejects_tsp_imports() {
        let dir = std::env::temp_dir().join(format!("tspserver-jsx-import-{}", std::process::id()));
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
    fn rewrite_local_imports_preserves_esm_loading() {
        // The embedded worker writes the generated wrapper as a real ESM
        // entry point. Static local imports therefore use top-level await
        // instead of entering Bun's CommonJS loader recursively.
        let dir =
            std::env::temp_dir().join(format!("tspserver-jsx-rewrite-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("shared.ts"), "export const a = 1;\nexport const b = 2;\n")
            .unwrap();
        let src = "import { a, b as renamed } from './shared';\nexport function GET() { return a + renamed; }\n";
        let out = rewrite_local_imports(src, &dir).unwrap();
        assert!(
            out.contains("const { a, b: renamed } = await import("),
            "rewriter must keep local dependencies on the native ESM loader; got: {out}"
        );
        assert!(
            !out.contains("require("),
            "the local dependency rewriter must not introduce CommonJS require; got: {out}"
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
            std::env::temp_dir().join(format!("tspserver-jsx-generation-{}", std::process::id()));
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
        let dir = std::env::temp_dir().join(format!("tspserver-dynamic-{}", std::process::id()));
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
        let src = "// comment\nexport function GET() {\n  return <h1>Hello from TSP</h1>;\n}\n";
        let out = tsx_to_js(src).unwrap();
        assert!(out.contains("export function"));
        assert!(out.contains("function GET"));
        assert!(out.contains("<h1>Hello from TSP</h1>"), "got: {out}");
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
    fn normal_wrapper_preserves_binary_response_bodies() {
        let wrapped = wrap_for_bun_cli(
            "function GET() { return new Response(Uint8Array.from([0, 255]), {headers: {'content-type': 'image/png'}}); }\n",
            "GET",
            None,
        );
        assert!(wrapped.contains("arrayBuffer()"), "got: {wrapped}");
        assert!(wrapped.contains("body_b64"), "got: {wrapped}");
        assert!(wrapped.contains("__tspEncodeBody__"), "got: {wrapped}");
    }

    #[test]
    fn image_pipeline_generates_a_binary_response_envelope_under_bun() {
        let source = r#"
import { Image } from "tsp:server";

export async function GET() {
  const image = new Image("data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=");
  const bytes = await image.png().bytes();
  return new Response(bytes, { headers: { "content-type": "image/png" } });
}
"#;
        let transformed = tsx_to_js(source).expect("image source must transform");
        let wrapped = wrap_for_bun_cli(&transformed, "GET", None);
        let candidates = [
            std::path::PathBuf::from(r"D:\GitHub\tsp\bun\build\release-dev\bun.exe"),
            std::path::PathBuf::from(r"D:\GitHub\tsp\.bun-bootstrap\node_modules\bun\bin\bun.exe"),
        ];
        let Some(bun_exe) = candidates.into_iter().find(|path| path.is_file()) else {
            eprintln!("skipping: no bun executable found (CI without Bun build)");
            return;
        };
        let temp_path = std::env::temp_dir().join(format!(
            "tsp-pipeline-image-{}.tsx",
            std::process::id()
        ));
        std::fs::write(&temp_path, wrapped).expect("wrap must be writable");
        let output = std::process::Command::new(&bun_exe)
            .arg(&temp_path)
            .output()
            .expect("bun must run the generated image wrap");
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let _ = std::fs::remove_file(&temp_path);
        assert!(
            output.status.success(),
            "image wrap must run cleanly under bun\nstdout: {stdout}\nstderr: {stderr}"
        );
        assert!(stdout.contains("\"type\":\"response\""), "stdout: {stdout}");
        assert!(stdout.contains("\"content-type\",\"image/png\""), "stdout: {stdout}");
        assert!(stdout.contains("\"body_b64\":\""), "stdout: {stdout}");
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
    fn embedded_wrapper_does_not_queue_handler_for_sync_routes() {
        let wrapped = wrap_for_embedded_worker(
            "function GET() { return <h1>Hello</h1>; }\n",
            "GET",
            None,
        );
        assert!(wrapped.contains("__tspRenderNodeSync__"));
        assert!(!wrapped.contains("const __tspResultPromise__ = Promise.resolve().then"));
    }

    #[test]
    fn embedded_wrapper_preserves_route_exports_for_esm_loading() {
        let transformed = "export const config = { cache: 'no-store' };\nexport async function GET() { return 'ok'; }\n";
        let wrapped = wrap_for_embedded_worker(transformed, "GET", None);
        assert!(wrapped.contains("export const config = { cache: 'no-store' }"));
        assert!(wrapped.contains("export async function GET()"));
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
            wrapped.contains("const __tspServer = {};"),
            "got: {wrapped}"
        );
        assert!(
            wrapped.contains("application/json; charset=utf-8"),
            "got: {wrapped}"
        );
        assert!(wrapped.contains("__tspHttpError__"), "got: {wrapped}");
        assert!(wrapped.contains("Promise.resolve().then"), "got: {wrapped}");
        assert!(wrapped.contains("process.exitCode = 0;"), "got: {wrapped}");
        assert!(!wrapped.contains("process.exit(0)"), "got: {wrapped}");
    }

    #[test]
    fn wrap_exposes_hidden_tsp_server_bridge_for_local_dependencies() {
        let wrapped = wrap_for_bun_cli(
            "function GET() { return new Response('ok'); }\n",
            "GET",
            None,
        );
        assert!(wrapped.contains("Symbol.for('tsp.server.bridge')"));
        assert!(wrapped.contains(
            "Object.defineProperty(globalThis, Symbol.for('tsp.server.bridge'), { configurable: true, value: __tspServer })"
        ));
        assert!(!wrapped.contains("globalThis.sql ="));
    }

    /// §32.1 dev error page: a non-HttpError throw inside
    /// the page handler must NOT propagate to the outer
    /// IIFE (which would `process.exit(1)` and lose the
    /// error body). The inner catch must build a 500
    /// response with a JSON body that carries the error
    /// name, message, and stack so the host can render
    /// the dev error page in `TSP_DEVELOPMENT=1` mode.
    #[test]
    fn wrap_emits_dev_error_page_500_for_non_http_errors() {
        let wrapped = wrap_for_bun_cli(
            "function GET() { throw new Error('boom'); }\n",
            "GET",
            None,
        );
        // The HttpError fast path is still present.
        assert!(
            wrapped.contains("e.status"),
            "the HttpError fast path must be preserved; got: {wrapped}"
        );
        // The new fallback branch must serialize the
        // error into a JSON body the host can parse.
        assert!(
            wrapped.contains("JSON.stringify({kind: 'tsp_error'"),
            "non-HttpError catch must serialize a `tsp_error` JSON body; got: {wrapped}"
        );
        assert!(
            wrapped.contains("error: (e && e.name)"),
            "the serialized body must carry the error name; got: {wrapped}"
        );
        assert!(
            wrapped.contains("stack: (e && e.stack)"),
            "the serialized body must carry the stack; got: {wrapped}"
        );
        // The 500 response header signals dev-mode host
        // dispatch.
        assert!(
            wrapped.contains("'x-tsp-error': 'page'"),
            "the 500 response must carry `x-tsp-error: page` so the host recognizes the dev path; got: {wrapped}"
        );
    }

    /// Spec §6.3 / plan §10.4: an invalid handler
    /// return value (object / number / boolean / etc.)
    /// is a contract violation. The wrap throws
    /// with the typed `TSP3001` prefix so the user
    /// can grep for it in the 500 body's
    /// `message` field. The type name is
    /// capitalized to match the plan's
    /// "Object" / "Number" / etc. wording.
    #[test]
    fn wrap_invalid_return_value_uses_tsp3001_prefix() {
        let wrapped = wrap_for_bun_cli(
            "function GET() { return 42; }\n",
            "GET",
            None,
        );
        // The wrap must throw with the
        // typed `TSP3001:` prefix (so the
        // inner catch's JSON body carries
        // the code in its `message` field).
        assert!(
            wrapped.contains("TSP3001: handler returned unsupported value"),
            "invalid-return wrap must carry the `TSP3001:` prefix; got: {wrapped}"
        );
        // The type name is capitalized to
        // match the plan's wording.
        assert!(
            wrapped.contains("Number"),
            "the type name in the message must be capitalized (Number for `return 42`); got: {wrapped}"
        );
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
        // The embedded worker must not access process.stdin: its stdin is
        // redirected to null and control traffic uses the native worker
        // socket. The signal remains live, while cancellation wiring belongs
        // to that protocol boundary.
        assert!(
            wrapped.contains("const __tspAbortCtrl__ = new AbortController()"),
            "got: {wrapped}"
        );
        assert!(
            !wrapped.contains("process.stdin?.on"),
            "embedded wrapper must not register a standard-input listener; got: {wrapped}"
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
        assert!(wrapped.contains("TSP3001: handler returned unsupported value"));
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
    fn rewrite_fragment_exports_injects_name_as_first_arg() {
        // Phase 9 (plan §14): `export const X = fragment(handler)`
        // declares a fragment. The rewriter injects the export
        // name as the first arg so the wrap-prelude's
        // `__tspFragment__` registry function can store the
        // handler under the name the host will look up. Without
        // this rewrite, the wrap sees `fragment(handler)` and
        // has no way to map a request's `name=...` query back
        // to the right handler.
        let out = rewrite_fragment_exports(
            "export const userList = fragment(async (ctx) => '<ul></ul>');\n",
        );
        assert!(
            out.contains("const userList = fragment(\"userList\", async (ctx) => '<ul></ul>');"),
            "rewriter must insert the export name as the first fragment arg; got: {out}"
        );
        assert!(
            !out.contains("export const userList = fragment("),
            "the `export` keyword must be stripped (the re-emit is plain `const` so the runtime \
             can register before the wrap selects a handler); got: {out}"
        );
    }

    #[test]
    fn rewrite_fragment_exports_does_not_touch_other_exports() {
        // The rewriter only fires on the line-shape
        // `export const X = fragment(` -- `export function GET(...)`
        // and `export const plain = 5` must pass through unchanged
        // (a regression here would break every other page route).
        let src = "\
export function GET() { return 'x'; }\n\
export const plain = 5;\n\
export const frag = fragment(() => null);\n\
";
        let out = rewrite_fragment_exports(src);
        assert!(out.contains("export function GET()"), "got: {out}");
        assert!(
            out.contains("export const plain = 5;"),
            "non-fragment const exports must not be rewritten; got: {out}"
        );
        assert!(
            out.contains("const frag = fragment(\"frag\", () => null);"),
            "the fragment line must be rewritten; got: {out}"
        );
        assert!(
            !out.contains("export const frag = fragment("),
            "the fragment line's `export` must be stripped; got: {out}"
        );
    }

    #[test]
    fn wrap_emits_fragment_registry_and_dispatch() {
        // Phase 9 (plan §14.2): the wrap preamble installs
        // (a) the `__tspFragments` map, (b) the `__tspFragment__`
        // registry function, (c) the dispatch that picks a
        // fragment by name from `__tspContext.__tsp_fragment`,
        // and (d) the `ctx.fragment(name, params)` URL builder.
        // The `__tspServer` freeze also exposes the user-facing
        // `fragment` name. All five shapes must show up.
        let body =
            "function GET(ctx) { return ctx.fragment('userList'); }\nexport const userList = fragment(() => null);\n";
        let json = r#"{"method":"GET","path":"/fragments","query":"","headers":{}}"#;
        let wrapped = wrap_for_bun_cli(body, "GET", Some(json));
        assert!(
            wrapped.contains("const __tspFragments = new Map();"),
            "wrap must declare the fragment registry map; got: {wrapped}"
        );
        assert!(
            wrapped.contains("function __tspFragment__(__name__, __handler__)"),
            "wrap must define the fragment registry function; got: {wrapped}"
        );
        assert!(
            wrapped.contains("__tspFragments.set(__name__, __handler__);"),
            "registry function must store the handler under the export name; got: {wrapped}"
        );
        assert!(
            wrapped.contains("__tspFragmentName__ = typeof __tspContext"),
            "wrap must read the per-request fragment selector from the context; got: {wrapped}"
        );
        assert!(
            wrapped.contains("__tspHandler__ = __tspFragments.get(__tspFragmentName__)"),
            "wrap must dispatch to the fragment handler when a fragment selector is present; got: {wrapped}"
        );
        assert!(
            wrapped.contains("__tspContext.fragment ="),
            "wrap must expose `ctx.fragment(name, params?)` to the page; got: {wrapped}"
        );
        assert!(
            wrapped.contains("'/__tsp/fragment?'"),
            "fragment URL builder must point at the internal fragment endpoint; got: {wrapped}"
        );
        assert!(
            wrapped.contains("fragment: __tspFragment__"),
            "__tspServer.fragment must be the registry function; got: {wrapped}"
        );
    }

    #[test]
    fn wrap_context_fragment_url_bakes_token_route_and_extra_params() {
        // Phase 9: `ctx.fragment("echo", { msg: "hi" })` must
        // produce a URL whose query string carries the parent
        // page's path (`route`), the fragment name (`name`),
        // the per-process capability (`token`), and the user's
        // extra params (`msg=hi`). The wrap must NOT bake the
        // token from a hard-coded value -- it must read it from
        // `__tspContext.__tsp_fragment_token` so the host can
        // rotate the token per process.
        let body = "function GET(ctx) { return ctx.fragment('echo', { msg: 'hi' }); }\n";
        let json = r#"{"method":"GET","path":"/fragments","query":"","headers":{}}"#;
        let wrapped = wrap_for_bun_cli(body, "GET", Some(json));
        assert!(
            wrapped.contains("new URLSearchParams({route: __tspContext.path"),
            "URL builder must read the parent path from the context; got: {wrapped}"
        );
        assert!(
            wrapped.contains("name: String(__name__)"),
            "URL builder must use the fragment name; got: {wrapped}"
        );
        assert!(
            wrapped.contains("token: __tspContext.__tsp_fragment_token"),
            "URL builder must read the token from the per-process context, not a hard-coded value; got: {wrapped}"
        );
        assert!(
            wrapped.contains("...(__params__ || {})"),
            "URL builder must spread the user's extra params; got: {wrapped}"
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
    // against a pages/ tree with 8 distinct .tsp files all returned
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

    #[test]
    fn wrap_for_bun_cli_defers_embedded_builtin_lookups() {
        let wrapped = wrap_for_bun_cli("function GET() { return 'ok'; }", "GET", Some("{}"));
        assert!(
            wrapped.contains("const __tspUtilNs__ = {};\n"),
            "util namespace must be created without reading Bun builtins"
        );
        assert!(
            wrapped.contains("Object.defineProperties(__tspUtilNs__,"),
            "util namespace must expose lazy properties"
        );
        assert!(
            wrapped.contains("get: () => Bun.password"),
            "password must be read only when requested"
        );
        assert!(
            wrapped.contains(
                r#"Object.defineProperty(__tspServer, 'sql', { enumerable: true, get: () => require("bun").SQL })"#
            ),
            "SQL must be required only when requested"
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
    // full production pipeline for the actual `pages/nanoid.tsp`
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
        // Mirror of production `pages/nanoid.tsp` (import + GET + POST).
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
        // The response wrapper carries every Response body as Base64
        // so binary responses remain lossless. Decode it before checking
        // the JSON returned by the page.
        let response_body = response_body_from_stdout(&stdout);
        assert!(
            response_body.contains(r#""ok":true"#)
                && response_body.contains(r#""name":"alice""#)
                && response_body.contains(r#""age":30"#),
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
    fn wrap_for_bun_cli_surfaces_bun_builtin_utilities_for_pages() {
        // Slice 18: `__tspServer.util` is a frozen namespace that
        // exposes the bun 1.4 builtins we whitelist for pages
        // (`randomUUIDv7`, `hash`, `CryptoHasher`, `Glob`, `TOML`,
        // `YAML`, `markdown`, `escapeHTML`, `gzipSync`,
        // `gunzipSync`, `file`, `write`, `which`, `peek`,
        // `deepEquals`, `deepMatch`, `nanoseconds`, plus an
        // `env` wrapper that exposes `get` / `has` but not
        // `toJSON`). The high-risk bun builtins (`Bun.serve`,
        // `Bun.spawn`, `Bun.FFI`, `Bun.S3Client`, `Bun.connect`,
        // `Bun.mmap`, `Bun.Cookie`, `Bun.Transpiler`) are
        // intentionally absent.
        let body = "function GET() { return 'ok'; }";
        let wrapped = wrap_for_bun_cli(body, "GET", Some("{}"));
        // 1. The util namespace is on __tspServer.
        assert!(
            wrapped.contains("util: __tspUtilNs__"),
            "wrap must expose util on __tspServer; got prefix: {}",
            &wrapped[..wrapped.len().min(1500)]
        );
        // 2. The util namespace freezes its keys (so the page
        //    can't accidentally mutate the host-side globals).
        assert!(
            wrapped.contains("Object.defineProperties(__tspUtilNs__,"),
            "wrap must define and freeze the util namespace; got prefix: {}",
            &wrapped[..wrapped.len().min(2000)]
        );
        // 3. Spot-check the high-risk APIs are NOT exposed.
        for forbidden in [
            "Bun.serve",
            "Bun.spawn",
            "Bun.FFI",
            "Bun.S3Client",
            "Bun.mmap",
            "Bun.Transpiler",
        ] {
            assert!(
                !wrapped.contains(forbidden),
                "{forbidden} must NOT be exposed to the page (high-risk builtin); got prefix: {}",
                &wrapped[..wrapped.len().min(2000)]
            );
        }
        // 4. `Bun.env` is wrapped, not forwarded. The
        //    `toJSON()` shape on the original is hidden.
        assert!(
            wrapped.contains("env: { enumerable: true, get: () => Object.freeze({ get: (k) => Bun.env[k], has: (k) => k in Bun.env }) }"),
            "env wrapper must hide Bun.env.toJSON(); got prefix: {}",
            &wrapped[..wrapped.len().min(3000)]
        );
        // 5. Each expected namespace key is lazy on the util object.
        for key in [
            "get: () => Bun.randomUUIDv7",
            "get: () => Bun.hash",
            "get: () => Bun.CryptoHasher",
            "get: () => Bun.Glob",
            "get: () => Bun.TOML",
            "get: () => Bun.YAML",
            "get: () => Bun.markdown",
            "get: () => Bun.escapeHTML",
            "get: () => Bun.gzipSync",
            "get: () => Bun.gunzipSync",
            "get: () => Bun.file",
            "get: () => Bun.write",
            "get: () => Bun.which",
            "get: () => Bun.peek",
            "get: () => Bun.deepEquals",
            "get: () => Bun.deepMatch",
            "get: () => Bun.nanoseconds",
        ] {
            assert!(
                wrapped.contains(key),
                "util must expose `{key}`; got prefix: {}",
                &wrapped[..wrapped.len().min(3000)]
            );
        }
    }

    #[test]
    fn rewrite_tsp_server_imports_accepts_util_named_export() {
        // Plan §16.1/§16.4: util is the single named export for
        // the slice-18 builtin bundle. Pages write
        //     import { util } from "tsp:server";
        //     util.randomUUIDv7();
        // and the rewriter collapses it to
        //     const { util } = __tspServer;
        let src = r#"import { util } from "tsp:server";
export function GET() {
  return new Response(util.randomUUIDv7(), { headers: { "content-type": "text/plain" } });
}
"#;
        let rewritten = rewrite_tsp_server_imports(src).expect("rewrite should succeed");
        assert!(
            rewritten.contains("const { util } = __tspServer;"),
            "rewriter must collapse the util import into a single destructure; got: {rewritten}"
        );
        assert!(
            !rewritten.contains("from \"tsp:server\""),
            "the `from \"tsp:server\"` clause must be stripped after rewrite; got: {rewritten}"
        );
    }

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
            wrapped.contains(
                r#"Object.defineProperty(__tspServer, 'sql', { enumerable: true, get: () => require("bun").SQL })"#
            ),
            "wrap must expose a lazy native SQL factory via `require(\"bun\")`; got prefix: {}",
            &wrapped[..wrapped.len().min(1200)]
        );
        assert!(
            !wrapped.contains("const __tspSqlNs__"),
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
        // `pages/sql_demo.tsp` in production.
        let source = r#"// Slice 17d regression test fixture.
import { sql } from "tsp:server";

export async function GET(ctx) {
  // `sql(url)` is the factory call (returns a connection
  // function from bun's per-worker pool). `sql\`url\`` would
  // be a QUERY, not a connect -- easy to confuse. The page
  // mirrors `pages/sql_demo.tsp` in production.
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
        // Decode the Base64 response body before checking the returned
        // row. `bun:sqlite` is in-process (no server needed) and the row
        // value confirms the connection was real (not a stub).
        let response_body = response_body_from_stdout(&stdout);
        assert!(
            response_body.contains(r#""ok":true"#)
                && response_body.contains(r#""label":"slice17d""#),
            "envelope must contain the bun:sql row; stdout: {stdout}"
        );
        assert!(
            !stdout.contains("ReferenceError") && !stdout.contains("SyntaxError"),
            "no JS errors allowed in the generated module; stdout: {stdout}"
        );
    }

    #[test]
    fn wrap_for_bun_cli_surfaces_bun_password_for_pages() {
        // Slice 22 follow-up: `password` was merged into
        // `util` so the "bun builtins via util" surface stays
        // unified. The wrap preamble still bridges bun's
        // native `Bun.password` to the page, but now through
        // the frozen `__tspUtilNs__` object (a single
        // `password: Bun.password` field) instead of a
        // dedicated top-level `__tspServer.password` slot.
        // Pages reach it via
        //     import { util } from "tsp:server";
        //     util.password.hashSync("hello", { algorithm: "bcrypt", cost: 4 });
        //
        // This test pins the wrap-string shape so a refactor
        // that accidentally drops the `Bun.password` bridge
        // (or splits `password` back out of `util`) is caught
        // at unit-test time, without spinning the real
        // binary.
        let body = "function GET() { return util.password.hashSync('hello', { algorithm: 'bcrypt', cost: 4 }); }";
        let wrapped = wrap_for_bun_cli(body, "GET", Some("{}"));
        // The password bridge must live INSIDE the util
        // namespace -- a top-level `__tspServer.password`
        // slot is the shape we just collapsed away from.
        assert!(
            !wrapped.contains("const __tspPasswordNs__ = Bun.password;"),
            "password must no longer have its own top-level binding (it lives under util now); \
             got prefix: {}",
            &wrapped[..wrapped.len().min(1200)]
        );
        assert!(
            !wrapped.contains("password: __tspPasswordNs__"),
            "password must no longer be a top-level __tspServer field; got prefix: {}",
            &wrapped[..wrapped.len().min(1500)]
        );
        // And it must be a lazy `Bun.password` getter inside the
        // `__tspUtilNs__` namespace.
        assert!(
            wrapped.contains("password: { enumerable: true, get: () => Bun.password }"),
            "wrap must lazily bridge bun's native password API to `util.password`; \
             got prefix: {}",
            &wrapped[..wrapped.len().min(2500)]
        );
        // The util namespace must still be on __tspServer
        // (the new home for password).
        assert!(
            wrapped.contains("util: __tspUtilNs__"),
            "wrap must expose util on __tspServer; got prefix: {}",
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
    fn rewrite_tsp_server_imports_rejects_password_after_merge_into_util() {
        // Slice 22 follow-up: `password` was merged into
        // `util` (one less top-level export, one less
        // rewriter allow-list entry). Pages now reach
        // `Bun.password` via
        //     import { util } from "tsp:server";
        //     util.password.hashSync(...)
        //
        // The old `import { password } from "tsp:server"`
        // shape must be rejected by the rewriter so a
        // forgotten code path fails fast at transpile time
        // (rather than silently rendering an undefined
        // `password` global at runtime). Plan §16.4
        // requires the framework API to be reached through
        // an explicit allow-listed import; "password" is no
        // longer on that list.
        let src = r#"import { password } from "tsp:server";
export function GET() {
  return password.hashSync("hello", { algorithm: "bcrypt", cost: 4 });
}
"#;
        let err = rewrite_tsp_server_imports(src)
            .expect_err("rewriter must reject `import { password }` after the slice 22 merge");
        let msg = format!("{err:?}");
        assert!(
            msg.contains("unsupported tsp:server named import")
                || msg.contains("password"),
            "rejection must name the offending import (plan §16.4); got: {msg}"
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
        // page mirrors `pages/password.tsp` in production.
        //
        // Slice 22 follow-up: `password` was merged into
        // `util` (one less top-level export, one less
        // rewriter allow-list entry). The fixture below
        // mirrors the production shape after the merge --
        // `import { util } from "tsp:server";
        //  util.password.hashSync(...)` -- so the pipeline
        // test catches any future regression that re-splits
        // the two namespaces.
        let source = r#"// Slice 17c (revised) + slice 22 follow-up fixture.
import { util } from "tsp:server";

export function GET() {
  const bcrypt = util.password.hashSync("hello", { algorithm: "bcrypt", cost: 4 });
  const argon = util.password.hashSync("hello", { algorithm: "argon2id" });
  const bcryptOk = util.password.verifySync("hello", bcrypt);
  const bcryptNo = util.password.verifySync("world", bcrypt);
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
            transformed.contains("const { util } = __tspServer;"),
            "rewriter must emit `const {{ util }} = __tspServer;` (password lives under util now); got: {transformed}"
        );
        assert!(
            !transformed.contains("const { password } = __tspServer;"),
            "rewriter must NOT emit a top-level `const {{ password }} = __tspServer;` after the merge; got: {transformed}"
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
        // Decode the Base64 response body before checking the result. The
        // algorithm-specific prefixes prove that the real bcrypt and
        // argon2id implementations ran (not a stub).
        let response_body = response_body_from_stdout(&stdout);
        assert!(
            response_body.contains(r#""ok":true"#)
                && response_body.contains(r#""isBcrypt":true"#)
                && response_body.contains(r#""isArgon":true"#)
                && response_body.contains(r#""bcryptSample":"$2b$04$""#)
                && response_body.contains(r#""argonSample":"$argon2id$""#),
            "envelope must contain the password result fragments\n  ok={}\n  isBcrypt={}\n  isArgon={}\n  bcryptSample={}\n  argonSample={}\n  stdout: {stdout}",
            response_body.contains(r#""ok":true"#),
            response_body.contains(r#""isBcrypt":true"#),
            response_body.contains(r#""isArgon":true"#),
            response_body.contains(r#""bcryptSample":"$2b$04$""#),
            response_body.contains(r#""argonSample":"$argon2id""#),
        );
        assert!(
            !stdout.contains("ReferenceError") && !stdout.contains("SyntaxError"),
            "no JS errors allowed in the generated module; stdout: {stdout}"
        );
    }

    /// §32.2 (plan sect.32.2, contract Amendment 9):
    /// the embedded-worker path emits a `//# sourceURL=...`
    /// directive so bun:runtime can attribute the
    /// script to the original `.tsp` file (the
    /// directive value is the absolute path with the
    /// `tsp://` scheme + the current execution
    /// generation, e.g. `tsp://D:/GitHub/tsp/pages/foo.tsp?generation=42`).
    /// A future slice adds the matching
    /// `//# sourceMappingURL=data:...` directive (which
    /// would remap the transpiled line/col back to the
    /// original); today's bun 1.4 honors `//# sourceURL=`
    /// for in-line eval'd scripts but not for the
    /// file-loaded path the worker uses, so the
    /// production stack trace still shows the temp
    /// file path (`tsp-embedded-worker-<pid>.tsx`)
    /// with the WRAPPED line/col -- not the original
    /// `.tsp` line/col. The dev error page (Amendment
    /// 7) still surfaces the raw stack so the dev can
    /// trace it; the file name + line/col are just
    /// unhelpful until the bun-side change lands.
    #[test]
    fn wrap_for_embedded_worker_emits_sourceurl_directive() {
        let body = "function GET() { return 'ok'; }\n";
        let wrapped = wrap_for_embedded_worker(body, "GET", None);
        // The wrap itself does NOT append the
        // `//# sourceURL=` directive -- that lives in
        // `jsc_bridge::execute_inner` and is appended
        // AFTER the wrap, with the real source path.
        // The wrap's contract is just the wrap shape;
        // the test that pins the sourceURL emission
        // lives in `jsc_bridge.rs` (next to the
        // `execute_inner` call site) -- not here.
        // What we CAN pin from the wrap is the absence
        // of any pre-existing `//# sourceURL=` (the
        // wrap would be wrong if it emitted one with
        // a placeholder path).
        assert!(
            !wrapped.contains("//# sourceURL="),
            "wrap_for_embedded_worker must NOT emit a sourceURL -- the real directive is appended by jsc_bridge::execute_inner with the actual source path; got: {wrapped}"
        );
        // And no sourceMappingURL either (the v1
        // contract is `//# sourceURL=` only; a
        // follow-up slice adds the source map when
        // bun's file-loaded path honors it).
        assert!(
            !wrapped.contains("sourceMappingURL"),
            "wrap_for_embedded_worker must NOT emit a sourceMappingURL -- v1 ships the sourceURL directive only; got: {wrapped}"
        );
    }

    /// §32.2: pin the jsc_bridge side of the
    /// `//# sourceURL=` directive. The wrap itself
    /// does not emit it (see the previous test);
    /// `jsc_bridge::execute_inner` appends the
    /// directive AFTER the wrap, with the absolute
    /// source path. The directive value is
    /// `tsp://<path>?generation=<N>` -- the
    /// `tsp://` scheme + the absolute path +
    /// the current execution generation, which
    /// changes on every reload so the JS module
    /// registry cache (the `bun:main` slot the
    /// BUG-0001 fix clears) is busted even when the
    /// file content is unchanged.
    ///
    /// This is a string-pin: the test does NOT run
    /// bun (the wrap-only check is the jsc_bridge's
    /// `execute_inner`, which is a real-binary
    /// pipeline). A future bun-side change that
    /// flips bun's file-loaded-script policy would
    /// make the `//# sourceURL=` work for stack
    /// traces; the pin keeps the directive shape
    /// stable across that change.
    #[test]
    fn jsc_bridge_appends_tsp_sourceurl_with_generation() {
        // The directive string lives in
        // `jsc_bridge::execute_inner`; this test
        // pins its shape so a refactor cannot
        // accidentally drop the generation suffix
        // (which would break the BUG-0001 fix's
        // per-request cache-bust).
        let needle = "//# sourceURL=";
        // We can't easily call execute_inner from
        // a unit test (it needs a real BunRuntime),
        // so this test pins the literal string by
        // asserting the surrounding format. A
        // grep against the source confirms the
        // shape; a future refactor that changes the
        // format will be caught by the start_order
        // e2e (which exercises the full pipeline).
        let src = include_str!("jsc_bridge.rs");
        assert!(
            src.contains(needle),
            "jsc_bridge.rs must contain a {needle:?} directive emission; \
             a refactor that drops it breaks the dev error page's file-name \
             attribution and the BUG-0001 per-request cache bust"
        );
        // The `?generation=` suffix is the per-request
        // cache-bust for `bun:main` (see BUG-0001).
        assert!(
            src.contains("?generation="),
            "jsc_bridge.rs must include the `?generation=` query in the sourceURL \
             so each request's wrap gets a unique module-registry key"
        );
    }
}
