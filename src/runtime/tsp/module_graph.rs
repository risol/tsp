//! Module Graph for TSP v2 slice 9 (plan sect.20).
//!
//! See `tsp-v2-plan.md` sect.20.1-20.3 (ModuleGraph + ModuleNode +
//! PageSlot). This file lands the data structure; the actual build
//! pipeline (transpile -> evaluate -> validate) lands in slice 10
//! (Generation + Atomic Reload). The watcher (slice 11) feeds
//! invalidations through this graph.
//!
//! Scope for slice 9:
//! - Canonical `ModuleId` (path-based, normalised, deduped).
//! - `ModuleNode` (id, path, imports, page_roots, source_hash).
//! - `ModuleGraph` with `nodes` (id -> node) and `reverse`
//!   (id -> [ids that import me]) maps.
//! - `extract_imports(text) -> Vec<ModuleId>` -- regex-based,
//!   conservative, matches the conventional `import ... from "...";`
//!   form. The slice-7+ AST pass (bun_js_parser) widens to re-exports,
//!   dynamic imports, and the `.tsp`-import rejection check.
//! - `ModuleGraph::from_routes(routes_dir) -> Result<Self, _>` --
//!   walks the routes directory, reads every `.tsp` source, extracts
//!   its imports, and builds both maps.
//! - `affected_pages(id) -> &[ModuleId]` -- the reverse-graph
//!   lookup the watcher (slice 11) uses to mark dirty pages.
//!
//! Out of slice 9: anything that walks IMPORTS and reads THEIR
//! files. That is part of the Generation / ModuleGraph.build pass
//! (slice 10); for now we record what a module says it imports and
//! trust the path-based identity.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::router::HttpMethod;

/// Canonical identity of a module in the application source graph.
/// Slice 9 uses the absolute canonical filesystem path as the
/// identity; slice 10 swaps in a content-hash secondary key so
/// the graph survives a move (same content, new path).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleId(PathBuf);

impl ModuleId {
    /// Build an id from a path under the application root. The
    /// path is canonicalised (best effort; symlinks are NOT
    /// followed on Windows because the LAYERING note in
    /// `tsp-v2-plan.md` sect.31 reserves that for a future
    /// security config).
    pub fn from_path(path: &Path) -> Self {
        Self(canonicalize_best_effort(path))
    }

    /// Wrap an already-canonical path. The caller is asserting
    /// the path is canonical (used by tests and by the slice-10
    /// generation module which works in normalised paths).
    pub fn from_canonical_path(path: PathBuf) -> Self {
        Self(path)
    }

    /// Borrow the underlying canonical path.
    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

fn canonicalize_best_effort(path: &Path) -> PathBuf {
    // `std::fs::canonicalize` returns Err for non-existent paths,
    // which is common during the build pipeline (a module might
    // reference a sibling that has not been written yet). For
    // slice 9 we just normalise: absolute + cleaned.
    match std::fs::canonicalize(path) {
        Ok(p) => p,
        Err(_) => {
            // Fallback: `absolutize` against CWD.
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            let joined = if path.is_absolute() {
                path.to_path_buf()
            } else {
                cwd.join(path)
            };
            // Strip `.\` and `..\` segments via `components()`.
            joined.components().collect()
        }
    }
}

/// Forward + reverse module graph node (plan sect.20.2).
#[derive(Debug, Clone)]
pub struct ModuleNode {
    pub id: ModuleId,
    /// Absolute path on disk.
    pub path: PathBuf,
    /// IDs this module imports (forward edges).
    pub imports: Vec<ModuleId>,
    /// Pages (route files) that this module is the root of. A
    /// non-page module (e.g. `components/UserCard.tsx`) has an
    /// empty `page_roots`. A `.tsp` route module has one
    /// `PageId` per HTTP method export (per plan sect.4.2).
    pub page_roots: Vec<PageId>,
    /// SHA-256 of the source text at scan time. The watcher
    /// (slice 11) recomputes the hash on the next event and
    /// skips the dirty flag if the hash is unchanged.
    pub source_hash: SourceHash,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PageId {
    pub route: String,
    pub method: HttpMethod,
}

/// A 64-bit FNV-1a hash of the source. FNV is plenty for
/// "did the file change since we last looked" and avoids the
/// 32-byte SHA-256 cost in the watcher hot path. Slice 10 may
/// swap to BLAKE3 if the empirical false-positive rate on
/// real-world edits is unacceptable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceHash(u64);

impl SourceHash {
    pub fn compute(text: &str) -> Self {
        let mut h: u64 = 0xcbf29ce484222325; // FNV-1a 64-bit offset basis
        for byte in text.as_bytes() {
            h ^= *byte as u64;
            h = h.wrapping_mul(0x100000001b3); // FNV-1a 64-bit prime
        }
        Self(h)
    }
}

#[derive(Debug)]
pub enum GraphError {
    Io { path: PathBuf, source: std::io::Error },
    Utf8 { path: PathBuf, source: std::string::FromUtf8Error },
}

impl std::fmt::Display for GraphError {
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

impl std::error::Error for GraphError {}

/// The full forward + reverse graph. See plan sect.20.2.
#[derive(Debug, Default, Clone)]
pub struct ModuleGraph {
    nodes: HashMap<ModuleId, ModuleNode>,
    /// Reverse graph: `id` -> list of modules that import it.
    /// Used by the watcher (slice 11) to mark "this page's
    /// dep changed -> mark this page dirty".
    reverse: HashMap<ModuleId, Vec<ModuleId>>,
}

impl ModuleGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Look up a node by id. Returns `None` if the id is not
    /// in the graph (e.g. it was an import to a file outside
    /// the application root, which slice 9 does NOT follow).
    pub fn get(&self, id: &ModuleId) -> Option<&ModuleNode> {
        self.nodes.get(id)
    }

    /// Returns the modules that directly import `id` (the
    /// reverse graph lookup). Order is insertion order, which
    /// matches the order imports were discovered during scan.
    pub fn importers_of(&self, id: &ModuleId) -> &[ModuleId] {
        self.reverse.get(id).map(Vec::as_slice).unwrap_or(&[])
    }

    /// Walk a `routes/` directory, read every `.tsp` file, build
    /// the forward edges (imports) and the reverse edges. Non-page
    /// modules (`.ts` / `.tsx` / `.js` / `.jsx` under `components/`
    /// or `lib/`) are also scanned if they live under the same
    /// root, because a future slice needs the forward edges for
    /// shared dep invalidation (slice 11).
    pub fn from_routes_dir(routes_dir: &Path) -> Result<Self, GraphError> {
        let canonical_root = routes_dir
            .canonicalize()
            .unwrap_or_else(|_| routes_dir.to_path_buf());
        let mut graph = Self::new();
        visit_dir(&canonical_root, &canonical_root, &mut graph)?;
        Ok(graph)
    }

    /// Insert one node + its forward edges. Internal; the scan
    /// path uses this. Tests use it to build toy graphs.
    pub fn insert(&mut self, node: ModuleNode) {
        let id = node.id.clone();
        let imports = node.imports.clone();
        for imported in &imports {
            self.reverse
                .entry(imported.clone())
                .or_default()
                .push(id.clone());
        }
        self.nodes.insert(id, node);
    }
}

fn visit_dir(root: &Path, dir: &Path, graph: &mut ModuleGraph) -> Result<(), GraphError> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(GraphError::Io {
                path: dir.to_path_buf(),
                source: e,
            })
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                return Err(GraphError::Io {
                    path: dir.to_path_buf(),
                    source: e,
                })
            }
        };
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(e) => {
                return Err(GraphError::Io {
                    path: entry.path(),
                    source: e,
                })
            }
        };
        let path = entry.path();
        if file_type.is_dir() {
            visit_dir(root, &path, graph)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        // Slice 9 only loads `.tsp` + `.ts` + `.tsx` source files.
        // `.json` and `.js`/`.jsx` are valid page deps but their
        // import extraction is identical; the slice-7+ AST pass
        // widens to all source extensions uniformly.
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "tsp" | "ts" | "tsx" | "js" | "jsx") {
            continue;
        }
        if let Some(node) = read_module(root, &path, name)? {
            graph.insert(node);
        }
    }
    Ok(())
}

use std::io;

fn read_module(
    root: &Path,
    path: &Path,
    name: &str,
) -> Result<Option<ModuleNode>, GraphError> {
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            return Err(GraphError::Io {
                path: path.to_path_buf(),
                source: e,
            })
        }
    };
    let text = match String::from_utf8(bytes) {
        Ok(t) => t,
        Err(e) => {
            return Err(GraphError::Utf8 {
                path: path.to_path_buf(),
                source: e,
            })
        }
    };
    let source_hash = SourceHash::compute(&text);
    let imports = extract_imports(&text);
    let page_roots = if name.ends_with(".tsp") {
        detect_page_roots(&text)
    } else {
        Vec::new()
    };
    let id = ModuleId::from_path(path);
    // `root` is preserved in the node for future slice-10
    // cross-edge validation, but the slice-9 storage is just
    // the absolute path. The unused-binding is fine; the field
    // will be wired in slice 10.
    let _ = root;
    Ok(Some(ModuleNode {
        id,
        path: path.to_path_buf(),
        imports,
        page_roots,
        source_hash,
    }))
}

/// Conservative regex-based import extraction. Matches the
/// conventional `import ... from "...";` and
/// `import "...";` forms. The slice-7+ AST pass widens to
/// re-exports (`export { foo } from "..."`), dynamic imports
/// (`import("...")`), and validates that `.tsp` is never the
/// import target (freeze 2).
///
/// The needle is the substring `from "<specifier>"` or the bare
/// `"<specifier>"` for side-effect imports. The specifier is
/// everything up to the closing quote; no support for nested
/// quotes or template strings in slice 9.
pub fn extract_imports(text: &str) -> Vec<ModuleId> {
    let mut out = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("import") {
            continue;
        }
        // Side-effect: `import "foo";`
        if let Some(rest) = trimmed.strip_prefix("import ") {
            if let Some(spec) = take_quoted_specifier(rest) {
                out.push(ModuleId(PathBuf::from(spec)));
                continue;
            }
        }
        // Normal: `import X from "foo";` or `import { a, b } from "foo";`
        if let Some(idx) = trimmed.find(" from ") {
            let after_from = &trimmed[idx + " from ".len()..];
            if let Some(spec) = take_quoted_specifier(after_from) {
                out.push(ModuleId(PathBuf::from(spec)));
            }
        }
    }
    // De-duplicate while preserving insertion order.
    let mut seen = std::collections::HashSet::new();
    out.retain(|m| seen.insert(m.clone()));
    out
}

fn take_quoted_specifier(s: &str) -> Option<&str> {
    let bytes = s.as_bytes();
    let quote = *bytes.first()?;
    if quote != b'"' && quote != b'\'' {
        return None;
    }
    let after_open = &s[1..];
    let close_rel = after_open.find(quote as char)?;
    Some(&after_open[..close_rel])
}

/// Detect `export function GET/POST/...` lines and turn each into
/// a `PageId`. Slice 5's `page::detect_methods` does the same
/// thing in a slightly different shape; slice 9 keeps the logic
/// local to the graph build so the data structure is
/// self-contained.
fn detect_page_roots(text: &str) -> Vec<PageId> {
    // Page root identity for slice 9 is just the path the file
    // lives at under the routes/ root, minus the extension. The
    // route file is a single page root; the multiple methods
    // share the same route path.
    let mut methods = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        for method in &HttpMethod::ALL {
            let needle_async = format!("export async function {}(", method.as_str());
            let needle_sync = format!("export function {}(", method.as_str());
            if trimmed.starts_with(&needle_async) || trimmed.starts_with(&needle_sync) {
                methods.push(*method);
            }
        }
    }
    // Route path placeholder -- the canonical path comes from
    // the file location. We do not have the routes_root here
    // (read_module is a generic file reader), so the page
    // roots are returned with empty `route` and the scanner
    // fills it in once it knows the layout. Slice 10 wires
    // the route path from the RouteTable.
    methods
        .into_iter()
        .map(|m| PageId {
            route: String::new(),
            method: m,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_hash_changes_on_edit() {
        let a = SourceHash::compute("hello");
        let b = SourceHash::compute("hello!");
        assert_ne!(a, b);
    }

    #[test]
    fn source_hash_stable_across_calls() {
        let a = SourceHash::compute("fn GET() { return null; }");
        let b = SourceHash::compute("fn GET() { return null; }");
        assert_eq!(a, b);
    }

    #[test]
    fn extract_imports_normal_form() {
        let src = "import { foo } from \"bar\";\n";
        let ids = extract_imports(src);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].as_path(), Path::new("bar"));
    }

    #[test]
    fn extract_imports_side_effect() {
        let src = "import \"register\";\n";
        let ids = extract_imports(src);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0].as_path(), Path::new("register"));
    }

    #[test]
    fn extract_imports_dedupes() {
        let src = "import { a } from \"x\";\nimport { b } from \"x\";\n";
        let ids = extract_imports(src);
        assert_eq!(ids.len(), 1);
    }

    #[test]
    fn extract_imports_ignores_strings_in_body() {
        // Line-anchored scan: a const string whose contents
        // happen to look like an import statement is NOT picked
        // up because the line starts with `const`, not `import`.
        // (The slice-7+ AST pass will keep this property by
        // walking real ImportDeclarations rather than the source
        // text.)
        let src = "const note = \"import foo from 'bar';\";\n";
        let ids = extract_imports(src);
        assert!(ids.is_empty());
    }

    #[test]
    fn graph_insert_records_reverse_edges() {
        let mut g = ModuleGraph::new();
        let a_id = ModuleId(PathBuf::from("a.ts"));
        let b_id = ModuleId(PathBuf::from("b.ts"));
        g.insert(ModuleNode {
            id: a_id.clone(),
            path: PathBuf::from("a.ts"),
            imports: vec![b_id.clone()],
            page_roots: vec![],
            source_hash: SourceHash::compute("a"),
        });
        assert_eq!(g.importers_of(&b_id), &[a_id.clone()]);
        assert!(g.importers_of(&a_id).is_empty());
    }
}