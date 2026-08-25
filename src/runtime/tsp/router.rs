//! Filesystem route scanner + matcher for TSP v2 PoC 1 slice 3.
//!
//! See `tsp-v2-plan.md` sect.6 (filesystem routing), sect.4.2 (named HTTP
//! method exports), and sect.42 (method dispatch / 405 / HEAD / OPTIONS).
//!
//! Slice 3 implements only the static portion of the routing table:
//! - `routes/index.tsp`             -> `/`
//! - `routes/foo.tsp`               -> `/foo`
//! - `routes/foo/bar.tsp`           -> `/foo/bar`
//! - `routes/foo/index.tsp`         -> `/foo`
//!
//! Dynamic segments `[id]`, catch-all `[...path]`, the radix tree, and
//! any priority-based conflict detection land in slice 7+ alongside the
//! full Context bridge. For now the matcher is linear over a `Vec<Route>`
//! which is fine because PoC 1 only has `routes/index.tsp`.
//!
//! Method validation against the actual file source (does `index.tsp`
//! export `GET`, `POST`, ...?) lands in slice 5 once JSC can evaluate
//! the module namespace. For slice 3 each scanned route is reported as
//! "all standard methods present" so 405 requires a future refinement.
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Standard HTTP methods the v2 page protocol recognises (plan sect.4.2,
/// spec sect.6.1/6.5/6.6).
///
/// `HEAD` is the synthetic-fallback case: when a page exports `GET` but
/// no explicit `HEAD`, the runtime synthesises a body-less HEAD from
/// the `GET` response (spec sect.6.5, plan sect.42). The router
/// detects this in `lookup` and returns `FoundHeadOverGet`; the host
/// then runs the GET path and drops the body. An explicit `HEAD`
/// export would short-circuit this -- but PoC 1's slice-5 detector
/// only looks at the file's own export set, and the spec does not
/// require a `.tsp` author to declare `HEAD` separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Options,
    Head,
}

impl HttpMethod {
    /// Parse the verb out of an HTTP/1.1 request line. Returns `None` for
    /// unknown verbs (e.g. `BREW`, `PURGE`) so the caller can 501 or 405
    /// with a clean error rather than crashing the listener.
    pub fn from_request_line(s: &str) -> Option<Self> {
        match s {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "PUT" => Some(Self::Put),
            "PATCH" => Some(Self::Patch),
            "DELETE" => Some(Self::Delete),
            "OPTIONS" => Some(Self::Options),
            "HEAD" => Some(Self::Head),
            _ => None,
        }
    }

    /// Canonical wire spelling (uppercase, ASCII).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
            Self::Options => "OPTIONS",
            Self::Head => "HEAD",
        }
    }

    /// All standard methods. Used for the `Allow:` header on 405 and as
    /// the default for a freshly scanned route before JSC validates the
    /// actual exports.
    pub const ALL: [Self; 7] = [
        Self::Get,
        Self::Post,
        Self::Put,
        Self::Patch,
        Self::Delete,
        Self::Options,
        Self::Head,
    ];

    /// The methods a `.tsp` file can export directly. Excludes
    /// `Head` and `Options`, which are fallback-handled by the
    /// host (spec sect.6.5 / 6.6): the slice 5 detector parses
    /// the file's own export set against this list, and the
    /// router marks the route as "exports X" only if `export
    /// function X` appears in the source. `Head` and `Options`
    /// are never exported directly; the host synthesises them
    /// when the request verb is one of those two and the
    /// corresponding real method is present (HEAD -> GET,
    /// OPTIONS -> auto-Allow response).
    pub const REAL: [Self; 5] = [
        Self::Get,
        Self::Post,
        Self::Put,
        Self::Patch,
        Self::Delete,
    ];
}

/// One segment of a route pattern. The URL template uses
/// the conventional `:name` for one-segment dynamic and
/// `*name` for catch-all (the form most Node web frameworks
/// use; the wire `path` keeps the colon / star so the
/// registry / dev-inspector / fragment router can show the
/// original shape).
///
/// Spec sect.11.3 / 11.4 define the matching rule:
/// - `Static("users")` matches the literal segment "users".
/// - `Param("id")` matches exactly one segment (non-empty,
///   no `/`) and binds it to `ctx.params.id`.
/// - `CatchAll("path")` matches the remaining segments
///   (possibly zero) and joins them with `/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Segment {
    Static(String),
    Param(String),
    CatchAll(String),
}

impl Segment {
    /// Render this segment to its URL-template form
    /// (`:name` / `*name`) for the canonical `Route.path`
    /// string the registry / inspector sees.
    fn template(&self) -> String {
        match self {
            Segment::Static(s) => s.clone(),
            Segment::Param(name) => format!(":{name}"),
            Segment::CatchAll(name) => format!("*{name}"),
        }
    }

    /// Precedence score (spec sect.11.6). Lower wins; the
    /// matcher picks the route with the lowest score when
    /// multiple candidates match the request.
    /// - Static: 0
    /// - Dynamic: 1
    /// - Catch-all: 2
    fn priority(&self) -> u8 {
        match self {
            Segment::Static(_) => 0,
            Segment::Param(_) => 1,
            Segment::CatchAll(_) => 2,
        }
    }
}

/// One row of the route table: the URL path the request must hit, the
/// `.tsp` source file that serves it, and the set of HTTP methods the
/// runtime thinks the file exports (slice 3 says "all of them" until
/// JSC proves otherwise in slice 5).
///
/// Slice 16e adds `segments` (the pattern broken into Static / Param /
/// CatchAll per spec sect.11.3-11.4) and `params` (the per-request
/// bind map, populated by `lookup` and read by the host when it
/// builds the per-request Context). The `path` field stays as the
/// canonical URL template -- `routes/users/[id].tsp` scans to
/// `path = "/users/:id"` -- and is still the lookup key for
/// `add` / `get_by_path` / the PageRegistry's `(route, method)`
/// cache key. Dynamic routes therefore share a PageSlot across
/// every concrete URL; the host's per-request cache decision
/// (slice 16d's `render_per_request`) handles that.
#[derive(Debug, Clone)]
pub struct Route {
    pub path: String,
    pub source: PathBuf,
    pub methods: Vec<HttpMethod>,
    pub segments: Vec<Segment>,
    /// `params` is empty in the table; `lookup` returns a
    /// `Route` with `params` populated for the matched
    /// request. Keeping it on the same struct avoids a
    /// `Found { route, params }` migration in the host.
    pub params: std::collections::HashMap<String, String>,
}

/// Outcome of looking up a request. Three states per plan sect.6.5 and
/// sect.42: the route exists and the method is supported, the route
/// exists but the method is not (so the caller emits 405 with
/// `Allow:`), or the route does not exist (404).
///
/// No `PartialEq` derive: `Route` contains a `PathBuf` which is
/// `PartialEq` only (not `Eq`) on stable, and `&'a Route` is
/// `PartialEq` by pointer not by value. Tests use `matches!` instead
/// of `assert_eq!` to sidestep the derive constraint.
#[derive(Debug, Clone)]
pub enum MatchResult {
    Found { route: Route, method: HttpMethod },
    /// The request verb was `HEAD` and the route exports `GET` but no
    /// explicit `HEAD`. Per spec sect.6.5, the host must run the GET
    /// handler and emit a body-less 200 with the GET response's
    /// Content-Length (and any headers, but no body). The host treats
    /// this the same as a `Found { method: Get }` except the response
    /// writer drops the body.
    FoundHeadOverGet { route: Route },
    MethodNotAllowed { route: Route, requested: HttpMethod },
    /// The request path contained an invalid percent escape or invalid
    /// UTF-8 after decoding. The host returns a typed 400 response.
    MalformedPath { error: PathDecodeError },
    NotFound,
}

/// A request-path percent-decoding failure. The offset is relative to the
/// offending path segment, which is sufficient for a useful 400 diagnostic
/// without echoing the whole request target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathDecodeError {
    pub offset: usize,
    pub reason: &'static str,
}

impl std::fmt::Display for PathDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid percent-encoded path at byte {}: {}", self.offset, self.reason)
    }
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Decode one URL path segment without turning an encoded slash into a
/// routing separator. Splitting happens before this function is called, so
/// `/files/a%2Fb` binds `path` to `a/b` rather than becoming two segments.
fn decode_path_segment(segment: &str) -> Result<String, PathDecodeError> {
    let bytes = segment.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'%' {
            decoded.push(bytes[i]);
            i += 1;
            continue;
        }
        if i + 2 >= bytes.len() {
            return Err(PathDecodeError { offset: i, reason: "truncated escape" });
        }
        let Some(hi) = hex_value(bytes[i + 1]) else {
            return Err(PathDecodeError { offset: i, reason: "non-hex escape" });
        };
        let Some(lo) = hex_value(bytes[i + 2]) else {
            return Err(PathDecodeError { offset: i + 1, reason: "non-hex escape" });
        };
        decoded.push((hi << 4) | lo);
        i += 3;
    }
    String::from_utf8(decoded).map_err(|_| PathDecodeError {
        offset: 0,
        reason: "escape sequence is not valid UTF-8",
    })
}

#[derive(Debug)]
pub enum RouterError {
    /// The `routes/` directory does not exist or is not a directory.
    RoutesDirMissing { path: PathBuf },
    /// A filesystem entry could not be stat'd. Bubble the underlying
    /// `io::Error` so the operator can fix permissions / mount issues.
    Io { path: PathBuf, source: io::Error },
    /// A `.tsp` file under `routes/` has a name we cannot yet translate
    /// to a URL path. Slice 3 only knows the static + index shapes, so
    /// a `[id].tsp` or `[...path].tsp` in the tree is reported here.
    /// The runtime refuses to start with an unknown shape -- silently
    /// ignoring it would let a typo'd dynamic segment 404 forever.
    UnsupportedShape { path: PathBuf, reason: &'static str },
    /// The watcher tried to add a route whose URL path is already
    /// in the table. Should not happen in normal operation
    /// (the routes dir cannot contain two files that produce
    /// the same URL path -- the boot-time scan would have
    /// failed). Reaching here means a race between two watcher
    /// ticks or a stale `last_seen`. Surface it as an error
    /// so the operator notices.
    DuplicatePath { path: String },
}

impl std::fmt::Display for RouterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RoutesDirMissing { path } => {
                write!(f, "routes directory not found: {}", path.display())
            }
            Self::Io { path, source } => {
                write!(f, "stat {} failed: {source}", path.display())
            }
            Self::UnsupportedShape { path, reason } => write!(
                f,
                "unsupported route shape at {}: {reason}",
                path.display()
            ),
            Self::DuplicatePath { path } => {
                write!(f, "duplicate route path: {path}")
            }
        }
    }
}

impl RouterError {
    /// The TSP-NNNN code for this router failure
    /// (spec sect.6.3 / slice 16h). All four variants
    /// land in the 1xxx routing range; the page-surface
    /// codes (spec sect.6.3 "dev diagnostics") document
    /// a slightly different mapping but the prefixes
    /// are stable within the runtime.
    pub fn code(&self) -> &'static str {
        match self {
            Self::RoutesDirMissing { .. } => "TSP1001",
            Self::UnsupportedShape { .. } => "TSP1002",
            Self::DuplicatePath { .. } => "TSP1003",
            Self::Io { .. } => "TSP1004",
        }
    }
}

impl std::error::Error for RouterError {}

use std::io;

/// In-memory route table. Slice 3 was a `Vec<Route>`; slice 15a
/// wraps the vec in a `Mutex` so the watcher (a different
/// thread from the request threads) can add and remove routes
/// while requests are in flight. The mutex is held only for the
/// duration of `lookup` / `add` / `remove`; the host's request
/// path does not hold the guard past the function call.
///
/// Cheap to clone (it's a `Clone` of `Arc<Mutex<...>>`), so the
/// host thread and the watcher thread can each hold an
/// independent handle to the same backing storage without a
/// `Box::leak` (the slice 10b pattern). The `Send + Sync`
/// requirement for cross-thread use is automatic via `Mutex`.
#[derive(Debug, Default, Clone)]
pub struct RouteTable {
    routes: Arc<Mutex<Vec<Route>>>,
}

impl RouteTable {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Number of routes currently in the table. Useful for the boot
    /// banner (`scanned N routes`) and for the dev inspector later.
    pub fn len(&self) -> usize {
        self.routes.lock().expect("route table lock poisoned").len()
    }

    /// Whether the table is empty. Tests use this; production code does
    /// not because the table is populated once at boot.
    pub fn is_empty(&self) -> bool {
        self.routes.lock().expect("route table lock poisoned").is_empty()
    }

    /// Iterate over all routes. Used by the boot-time
    /// `PageRegistry` builder to register one slot per
    /// (route, method) pair without forcing the caller to
    /// know the internal storage layout.
    /// Iterate over a snapshot of all routes. The returned
    /// iterator borrows from the lock guard, so the guard is
    /// held for the lifetime of the iteration. Callers should
    /// collect into a `Vec` if they need the data outside the
    /// iterator (e.g. for the boot-time `PageRegistry`
    /// builder, which wants to drop the lock before doing the
    /// I/O-heavy `page::prepare`).
    pub fn iter(&self) -> Vec<Route> {
        self.routes.lock().expect("route table lock poisoned").clone()
    }

    /// Walk `routes_dir` recursively, collect every `.tsp` file, and
    /// translate its path under `routes_dir` to a URL path. The
    /// directory must exist; an absent `routes/` is a configuration
    /// error, not an empty route table.
    pub fn scan(routes_dir: &Path) -> Result<Self, RouterError> {
        let meta = fs::metadata(routes_dir).map_err(|e| match e.kind() {
            io::ErrorKind::NotFound => RouterError::RoutesDirMissing {
                path: routes_dir.to_path_buf(),
            },
            _ => RouterError::Io {
                path: routes_dir.to_path_buf(),
                source: e,
            },
        })?;
        if !meta.is_dir() {
            return Err(RouterError::Io {
                path: routes_dir.to_path_buf(),
                source: io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "routes_dir is not a directory",
                ),
            });
        }

        let mut routes = Vec::new();
        scan_recursive(routes_dir, routes_dir, &mut routes)?;
        // Sort for deterministic 405 / dev-inspector output. Slice 3 has
        // at most one route so this is a no-op in practice; the sort
        // becomes load-bearing once slice 3+ adds real files.
        routes.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(Self {
            routes: Arc::new(Mutex::new(routes)),
        })
    }

    /// Add a new route at runtime. The watcher's slice-15a
    /// path uses this to register `.tsp` files that appeared
    /// in the routes dir after boot. Duplicate paths are
    /// rejected (a 500 in the watcher's caller; we keep the
    /// API strict so a misbehaving watcher is loud, not
    /// silent).
    pub fn add(&self, route: Route) -> Result<(), RouterError> {
        let mut guard = self.routes.lock().expect("route table lock poisoned");
        if guard.iter().any(|r| r.path == route.path) {
            return Err(RouterError::DuplicatePath { path: route.path });
        }
        guard.push(route);
        // Re-sort so lookup order stays deterministic. The set
        // is small (one entry added) so this is cheap.
        guard.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(())
    }

    /// Replace the route with the given URL path. The replacement is
    /// performed under the same lock as lookup, so callers never expose a
    /// partially updated route descriptor.
    pub fn replace_by_path(&self, route: Route) -> bool {
        let mut guard = self.routes.lock().expect("route table lock poisoned");
        let Some(existing) = guard.iter_mut().find(|r| r.path == route.path) else {
            return false;
        };
        *existing = route;
        guard.sort_by(|a, b| a.path.cmp(&b.path));
        true
    }

    /// Remove a route by URL path. No-op if the path is not
    /// in the table. The watcher's slice-15a path uses this
    /// when a `.tsp` file disappears. In-flight requests that
    /// already pinned this route's generation continue to
    /// serve from the LKG / current; new requests get a 404
    /// (spec sect.33.5).
    pub fn remove_by_path(&self, url_path: &str) -> bool {
        let mut guard = self.routes.lock().expect("route table lock poisoned");
        let before = guard.len();
        guard.retain(|r| r.path != url_path);
        before != guard.len()
    }

    /// Snapshot all URL paths currently in the table. The
    /// watcher uses this to compute the add/remove diff
    /// against the filesystem snapshot.
    pub fn paths(&self) -> Vec<String> {
        self.routes
            .lock()
            .expect("route table lock poisoned")
            .iter()
            .map(|r| r.path.clone())
            .collect()
    }

    /// Look up the `Route` for a URL path. Returns a clone so
    /// the caller can take ownership without contending on
    /// the table lock. Used by the watcher's slice 15a
    /// reconcile path to fetch the freshly-scanned `Route`
    /// for each added path before `add`ing it to the live
    /// table.
    pub fn get_by_path(&self, url_path: &str) -> Option<Route> {
        self.routes
            .lock()
            .expect("route table lock poisoned")
            .iter()
            .find(|r| r.path == url_path)
            .cloned()
    }

    /// Look up `(path, method)`. The path is the request URL
    /// path (e.g. `/users/42`); each segment is percent-decoded
    /// before matching, while encoded `/` remains inside that
    /// segment. Invalid escapes return `MalformedPath`. The
    /// matcher iterates the table and picks the highest-priority
    /// route that matches (spec sect.11.6 precedence: static >
    /// dynamic > catch-all). The returned `Route` carries a
    /// populated `params` map for the matched request.
    ///
    /// Trailing-slash equivalence (spec sect.11.9): `/foo` and
    /// `/foo/` both match the same route, except for the root
    /// `/` which is its own canonical form.
    pub fn lookup(&self, path: &str, method: HttpMethod) -> MatchResult {
        let normalized = if path == "/" {
            "/".to_string()
        } else {
            path.trim_end_matches('/').to_string()
        };
        let raw_segs: Vec<&str> = normalized
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        let decoded_segs: Vec<String> = match raw_segs
            .iter()
            .map(|segment| decode_path_segment(segment))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(segments) => segments,
            Err(error) => return MatchResult::MalformedPath { error },
        };
        let req_segs: Vec<&str> = decoded_segs.iter().map(String::as_str).collect();
        let guard = self.routes.lock().expect("route table lock poisoned");
        // Find ALL candidates, then pick the best by priority
        // (lowest total score; ties broken by source order
        // which `routes.sort_by` makes the scan order).
        let mut best: Option<(Route, u32)> = None;
        for r in guard.iter() {
            if let Some(params) = match_segments(&r.segments, &req_segs) {
                let score: u32 = r
                    .segments
                    .iter()
                    .map(|s| s.priority() as u32)
                    .sum();
                let better = match &best {
                    None => true,
                    Some((_, s)) => score < *s,
                };
                if better {
                    let mut route = r.clone();
                    route.params = params;
                    best = Some((route, score));
                }
            }
        }
        let Some((route, _)) = best else {
            return MatchResult::NotFound;
        };
        if route.methods.contains(&method) {
            MatchResult::Found { route, method }
        } else if method == HttpMethod::Head
            && route.methods.contains(&HttpMethod::Get)
        {
            // Spec sect.6.5: HEAD with no explicit HEAD export -> use
            // GET, drop the body. We do not look at a separate "head
            // exports" set here; the slice 5 detector treats any page
            // without `export function HEAD` as no explicit HEAD.
            MatchResult::FoundHeadOverGet { route }
        } else {
            MatchResult::MethodNotAllowed {
                route,
                requested: method,
            }
        }
    }
}

/// Try to match the request's path segments against a route's
/// pattern. Returns the populated `params` map on success.
///
/// Matching rules (spec sect.11.3-11.4):
/// - `Static(s)` matches the request segment when equal.
/// - `Param(name)` matches any single non-empty request
///   segment and binds it to `params[name]`.
/// - `CatchAll(name)` matches zero or more trailing request
///   segments and binds them joined by `/` to `params[name]`.
///   A catch-all can only appear in the final pattern
///   position; `match_segments` enforces this by returning
///   `None` when the catch-all is followed by anything that
///   did not also match (which the spec already disallows at
///   scan time -- this is a defence-in-depth check).
fn match_segments(
    pattern: &[Segment],
    req: &[&str],
) -> Option<std::collections::HashMap<String, String>> {
    let mut params: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut pi = 0;
    let mut ri = 0;
    while pi < pattern.len() {
        match &pattern[pi] {
            Segment::Static(s) => {
                if ri >= req.len() || req[ri] != s.as_str() {
                    return None;
                }
                pi += 1;
                ri += 1;
            }
            Segment::Param(name) => {
                if ri >= req.len() {
                    return None;
                }
                params.insert(name.clone(), req[ri].to_string());
                pi += 1;
                ri += 1;
            }
            Segment::CatchAll(name) => {
                // Join the rest of the request. An empty join
                // is allowed only when no segments remain.
                let rest: Vec<&str> = req[ri..].to_vec();
                params.insert(name.clone(), rest.join("/"));
                pi = pattern.len();
                ri = req.len();
            }
        }
    }
    if ri != req.len() {
        return None;
    }
    Some(params)
}

fn scan_recursive(
    root: &Path,
    dir: &Path,
    out: &mut Vec<Route>,
) -> Result<(), RouterError> {
    let entries = fs::read_dir(dir).map_err(|e| RouterError::Io {
        path: dir.to_path_buf(),
        source: e,
    })?;
    for entry in entries {
        let entry = entry.map_err(|e| RouterError::Io {
            path: dir.to_path_buf(),
            source: e,
        })?;
        let file_type = entry.file_type().map_err(|e| RouterError::Io {
            path: entry.path(),
            source: e,
        })?;
        let path = entry.path();
        if file_type.is_dir() {
            scan_recursive(root, &path, out)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some(stem) = name.strip_suffix(".tsp") else {
            continue;
        };
        let (url_path, segments) = url_path_for(root, &path, stem)?;
        out.push(Route {
            path: url_path,
            source: path,
            methods: HttpMethod::REAL.to_vec(),
            segments,
            params: std::collections::HashMap::new(),
        });
    }
    Ok(())
}

/// Translate a `.tsp` filename (relative to the `routes/` root) into
/// a (template path, segments) pair. The template path is the
/// canonical URL form (`/users/:id` for `routes/users/[id].tsp`);
/// the segments vector is the matching pattern used by `lookup`.
///
/// Slice 16e supports the spec sect.11.3/11.4 dynamic / catch-all
/// shapes:
/// - `routes/users/[id].tsp`    -> `/users/:id` with [Static("users"), Param("id")]
/// - `routes/files/[...path].tsp` -> `/files/*path` with [Static("files"), CatchAll("path")]
/// - directory segments work the same way: `routes/users/[id]/posts.tsp`
///   -> `/users/:id/posts` with [Static("users"), Param("id"), Static("posts")]
/// - the optional catch-all shape `[name...]` (matches zero or more
///   segments) is not in v2.0 (FREEZE item 3); `[...name]` requires
///   at least one segment.
///
/// The segment name must satisfy FREEZE item 3's pattern
/// `[A-Za-z_][A-Za-z0-9_]*` -- this is the only place we reject
/// dynamic / catch-all shapes other than the unsupported shapes the
/// spec never defined.
fn url_path_for(
    root: &Path,
    abs: &Path,
    stem: &str,
) -> Result<(String, Vec<Segment>), RouterError> {
    // `relative` is the path under `routes/`, e.g. `users/index.tsp`
    // or just `index.tsp`. Strip the `.tsp` (already done by caller) so
    // we work in segments below.
    let rel = abs.strip_prefix(root).map_err(|_| RouterError::UnsupportedShape {
        path: abs.to_path_buf(),
        reason: "file is not under the routes/ root",
    })?;
    let dir_segments: Vec<&str> = rel
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    let (dir_segs, stem_segs) = dir_segments.split_at(dir_segments.len() - 1);

    let parse_one = |s: &str, file: &Path| -> Result<Segment, RouterError> {
        parse_segment(s, file)
    };

    if stem == "index" {
        // `routes/index.tsp`         -> `/`
        // `routes/users/index.tsp`    -> `/users`
        let mut segments: Vec<Segment> = Vec::with_capacity(dir_segs.len());
        for s in dir_segs {
            segments.push(parse_one(s, abs)?);
        }
        let template = if segments.is_empty() {
            "/".to_string()
        } else {
            format!("/{}", segments_template(&segments))
        };
        return Ok((template, segments));
    }

    let mut segments: Vec<Segment> = Vec::with_capacity(dir_segs.len() + 1);
    for s in dir_segs {
        segments.push(parse_one(s, abs)?);
    }
    let stem_seg = parse_one(stem, abs)?;
    // FREEZE item 3: a catch-all can only appear as the LAST segment
    // of the path -- the segment stream must end with `[...name]` or
    // a static segment, never a `*name` followed by anything else.
    if let Segment::CatchAll(_) = stem_seg {
        // OK -- the file stem is the last position.
    }
    segments.push(stem_seg);
    // Reject a catch-all in any non-final position: walk the segments
    // and check that nothing follows a CatchAll.
    for (i, seg) in segments.iter().enumerate() {
        if matches!(seg, Segment::CatchAll(_)) && i + 1 < segments.len() {
            return Err(RouterError::UnsupportedShape {
                path: abs.to_path_buf(),
                reason: "catch-all `[...name]` must be the last segment",
            });
        }
    }
    let template = format!("/{}", segments_template(&segments));
    let _ = stem_segs; // kept for symmetry with the dir_segments split
    Ok((template, segments))
}

fn segments_template(segs: &[Segment]) -> String {
    segs.iter().map(|s| s.template()).collect::<Vec<_>>().join("/")
}

/// Parse a single segment token (from a directory name or the file
/// stem) into a `Segment`. Static segments are returned as-is; a
/// `[name]` token is `Param(name)`; a `[...name]` token is
/// `CatchAll(name)`. Anything else (mismatched brackets, empty
/// `[]`, names that fail the FREEZE item 3 identifier pattern) is
/// `UnsupportedShape` so the runtime refuses to boot.
fn parse_segment(token: &str, file: &Path) -> Result<Segment, RouterError> {
    if !token.contains('[') && !token.contains(']') {
        return Ok(Segment::Static(token.to_string()));
    }
    // Catch-all: `[...name]`
    if let Some(rest) = token.strip_prefix("[...") {
        let name = rest.strip_suffix(']').ok_or_else(|| RouterError::UnsupportedShape {
            path: file.to_path_buf(),
            reason: "malformed catch-all: missing `]`",
        })?;
        validate_segment_name(name, file)?;
        return Ok(Segment::CatchAll(name.to_string()));
    }
    // One-segment dynamic: `[name]`
    if let Some(rest) = token.strip_prefix('[') {
        let name = rest.strip_suffix(']').ok_or_else(|| RouterError::UnsupportedShape {
            path: file.to_path_buf(),
            reason: "malformed dynamic segment: missing `]`",
        })?;
        validate_segment_name(name, file)?;
        return Ok(Segment::Param(name.to_string()));
    }
    Err(RouterError::UnsupportedShape {
        path: file.to_path_buf(),
        reason: "unbalanced brackets in segment name",
    })
}

/// FREEZE item 3 segment-name rule: `[A-Za-z_][A-Za-z0-9_]*`.
/// Empty `[]` / non-identifier names are rejected at scan time
/// so a typo'd `routes/users/[1st].tsp` refuses to boot rather
/// than silently 404'ing.
fn validate_segment_name(name: &str, file: &Path) -> Result<(), RouterError> {
    if name.is_empty() {
        return Err(RouterError::UnsupportedShape {
            path: file.to_path_buf(),
            reason: "empty segment name `[]`",
        });
    }
    let mut chars = name.chars();
    let first = chars.next().expect("non-empty checked above");
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(RouterError::UnsupportedShape {
            path: file.to_path_buf(),
            reason: "segment name must start with [A-Za-z_]",
        });
    }
    for c in chars {
        if !(c.is_ascii_alphanumeric() || c == '_') {
            return Err(RouterError::UnsupportedShape {
                path: file.to_path_buf(),
                reason: "segment name must be [A-Za-z0-9_]",
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rt(path: &str, segments: Vec<Segment>, methods: Vec<HttpMethod>) -> Route {
        // For tests, a stable source path that mirrors the URL.
        // Dynamic / catch-all templates use `_` so the test
        // source strings are predictable.
        let source = if path == "/" {
            "routes/index.tsp".to_string()
        } else {
            format!(
                "routes{}.tsp",
                path.replace(':', "_").replace('*', "_star_")
            )
        };
        Route {
            path: path.to_string(),
            source: PathBuf::from(source),
            methods,
            segments,
            params: std::collections::HashMap::new(),
        }
    }

    #[test]
    fn url_path_index_root() {
        // `routes/index.tsp` -> `/`
        let root = Path::new("/app/routes");
        let abs = Path::new("/app/routes/index.tsp");
        let (template, segs) = url_path_for(root, abs, "index").unwrap();
        assert_eq!(template, "/");
        assert!(segs.is_empty());
    }

    #[test]
    fn url_path_index_nested() {
        // `routes/users/index.tsp` -> `/users`
        let root = Path::new("/app/routes");
        let abs = Path::new("/app/routes/users/index.tsp");
        let (template, segs) = url_path_for(root, abs, "index").unwrap();
        assert_eq!(template, "/users");
        assert_eq!(segs, vec![Segment::Static("users".to_string())]);
    }

    #[test]
    fn url_path_static() {
        let root = Path::new("/app/routes");
        let abs = Path::new("/app/routes/login.tsp");
        let (template, segs) = url_path_for(root, abs, "login").unwrap();
        assert_eq!(template, "/login");
        assert_eq!(segs, vec![Segment::Static("login".to_string())]);
    }

    #[test]
    fn url_path_static_nested() {
        let root = Path::new("/app/routes");
        let abs = Path::new("/app/routes/users/new.tsp");
        let (template, segs) = url_path_for(root, abs, "new").unwrap();
        assert_eq!(template, "/users/new");
        assert_eq!(
            segs,
            vec![
                Segment::Static("users".to_string()),
                Segment::Static("new".to_string()),
            ]
        );
    }

    #[test]
    fn url_path_dynamic_param() {
        // `routes/users/[id].tsp` -> `/users/:id` with [Static("users"), Param("id")]
        let root = Path::new("/app/routes");
        let abs = Path::new("/app/routes/users/[id].tsp");
        let (template, segs) = url_path_for(root, abs, "[id]").unwrap();
        assert_eq!(template, "/users/:id");
        assert_eq!(
            segs,
            vec![
                Segment::Static("users".to_string()),
                Segment::Param("id".to_string()),
            ]
        );
    }

    #[test]
    fn url_path_dynamic_directory_segment() {
        // `routes/users/[id]/posts.tsp` -> `/users/:id/posts`
        let root = Path::new("/app/routes");
        let abs = Path::new("/app/routes/users/[id]/posts.tsp");
        let (template, segs) = url_path_for(root, abs, "posts").unwrap();
        assert_eq!(template, "/users/:id/posts");
        assert_eq!(
            segs,
            vec![
                Segment::Static("users".to_string()),
                Segment::Param("id".to_string()),
                Segment::Static("posts".to_string()),
            ]
        );
    }

    #[test]
    fn url_path_catch_all() {
        // `routes/files/[...path].tsp` -> `/files/*path` with [Static("files"), CatchAll("path")]
        let root = Path::new("/app/routes");
        let abs = Path::new("/app/routes/files/[...path].tsp");
        let (template, segs) = url_path_for(root, abs, "[...path]").unwrap();
        assert_eq!(template, "/files/*path");
        assert_eq!(
            segs,
            vec![
                Segment::Static("files".to_string()),
                Segment::CatchAll("path".to_string()),
            ]
        );
    }

    #[test]
    fn url_path_rejects_invalid_segment_names() {
        // FREEZE item 3 pattern: `1st` is not a valid identifier.
        let root = Path::new("/app/routes");
        let abs = Path::new("/app/routes/users/[1st].tsp");
        let err = url_path_for(root, abs, "[1st]").unwrap_err();
        assert!(matches!(err, RouterError::UnsupportedShape { .. }));
        // Empty `[]`.
        let abs2 = Path::new("/app/routes/users/[].tsp");
        let err2 = url_path_for(root, abs2, "[]").unwrap_err();
        assert!(matches!(err2, RouterError::UnsupportedShape { .. }));
        // Unbalanced `[id`.
        let abs3 = Path::new("/app/routes/users/[id.tsp");
        let err3 = url_path_for(root, abs3, "[id").unwrap_err();
        assert!(matches!(err3, RouterError::UnsupportedShape { .. }));
    }

    #[test]
    fn url_path_rejects_non_final_catch_all() {
        // A catch-all followed by anything else is not allowed
        // (FREEZE item 3: catch-all is the last segment).
        let root = Path::new("/app/routes");
        let abs = Path::new("/app/routes/[...path]/tail.tsp");
        let err = url_path_for(root, abs, "tail").unwrap_err();
        assert!(matches!(err, RouterError::UnsupportedShape { .. }));
    }

    fn table_with(routes: Vec<Route>) -> RouteTable {
        RouteTable { routes: Arc::new(Mutex::new(routes)) }
    }

    #[test]
    fn lookup_found() {
        let table = table_with(vec![rt(
            "/",
            vec![],
            HttpMethod::REAL.to_vec(),
        )]);
        let m = table.lookup("/", HttpMethod::Get);
        assert!(matches!(m, MatchResult::Found { .. }));
    }

    #[test]
    fn lookup_not_found() {
        let table = table_with(vec![rt("/", vec![], HttpMethod::REAL.to_vec())]);
        assert!(matches!(table.lookup("/nope", HttpMethod::Get), MatchResult::NotFound));
    }

    #[test]
    fn lookup_method_not_allowed() {
        let table = table_with(vec![rt("/", vec![], vec![HttpMethod::Get])]);
        let m = table.lookup("/", HttpMethod::Post);
        assert!(matches!(m, MatchResult::MethodNotAllowed { .. }));
    }

    #[test]
    fn lookup_dynamic_segment_binds_params() {
        let table = table_with(vec![rt(
            "/users/:id",
            vec![Segment::Static("users".to_string()), Segment::Param("id".to_string())],
            HttpMethod::REAL.to_vec(),
        )]);
        match table.lookup("/users/42", HttpMethod::Get) {
            MatchResult::Found { route, .. } => {
                assert_eq!(route.params.get("id").map(String::as_str), Some("42"));
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn lookup_decodes_dynamic_segment_utf8_and_spaces() {
        let table = table_with(vec![rt(
            "/users/:id",
            vec![Segment::Static("users".to_string()), Segment::Param("id".to_string())],
            HttpMethod::REAL.to_vec(),
        )]);
        match table.lookup("/users/hello%20%E4%B8%96%E7%95%8C", HttpMethod::Get) {
            MatchResult::Found { route, .. } => {
                assert_eq!(route.params.get("id").map(String::as_str), Some("hello 世界"));
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn lookup_keeps_encoded_slash_inside_one_dynamic_segment() {
        let table = table_with(vec![rt(
            "/users/:id",
            vec![Segment::Static("users".to_string()), Segment::Param("id".to_string())],
            HttpMethod::REAL.to_vec(),
        )]);
        match table.lookup("/users/a%2Fb", HttpMethod::Get) {
            MatchResult::Found { route, .. } => {
                assert_eq!(route.params.get("id").map(String::as_str), Some("a/b"));
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn lookup_rejects_malformed_percent_escape() {
        let table = table_with(vec![rt(
            "/users/:id",
            vec![Segment::Static("users".to_string()), Segment::Param("id".to_string())],
            HttpMethod::REAL.to_vec(),
        )]);
        assert!(matches!(
            table.lookup("/users/%E4%ZZ", HttpMethod::Get),
            MatchResult::MalformedPath { .. }
        ));
    }

    #[test]
    fn lookup_dynamic_segment_does_not_over_match() {
        // `/users/42/extra` must not match `/users/:id` -- a Param
        // binds exactly one segment.
        let table = table_with(vec![rt(
            "/users/:id",
            vec![Segment::Static("users".to_string()), Segment::Param("id".to_string())],
            HttpMethod::REAL.to_vec(),
        )]);
        assert!(matches!(
            table.lookup("/users/42/extra", HttpMethod::Get),
            MatchResult::NotFound
        ));
    }

    #[test]
    fn lookup_catch_all_binds_remaining() {
        let table = table_with(vec![rt(
            "/files/*path",
            vec![Segment::Static("files".to_string()), Segment::CatchAll("path".to_string())],
            HttpMethod::REAL.to_vec(),
        )]);
        match table.lookup("/files/a/b/c.txt", HttpMethod::Get) {
            MatchResult::Found { route, .. } => {
                assert_eq!(
                    route.params.get("path").map(String::as_str),
                    Some("a/b/c.txt")
                );
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn lookup_prefers_static_over_dynamic() {
        // Spec sect.11.6: when two routes match, static beats
        // dynamic. /users/me must match the static `/users/me`
        // route, not the dynamic `/users/:name`.
        let table = table_with(vec![
            rt(
                "/users/:name",
                vec![Segment::Static("users".to_string()), Segment::Param("name".to_string())],
                HttpMethod::REAL.to_vec(),
            ),
            rt(
                "/users/me",
                vec![Segment::Static("users".to_string()), Segment::Static("me".to_string())],
                HttpMethod::REAL.to_vec(),
            ),
        ]);
        match table.lookup("/users/me", HttpMethod::Get) {
            MatchResult::Found { route, .. } => {
                assert_eq!(route.path, "/users/me");
                assert!(route.params.is_empty());
            }
            other => panic!("expected Found, got {other:?}"),
        }
        // The dynamic route still wins when no static matches.
        match table.lookup("/users/other", HttpMethod::Get) {
            MatchResult::Found { route, .. } => {
                assert_eq!(route.path, "/users/:name");
                assert_eq!(route.params.get("name").map(String::as_str), Some("other"));
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn lookup_prefers_dynamic_over_catch_all() {
        let table = table_with(vec![
            rt(
                "/files/*path",
                vec![Segment::Static("files".to_string()), Segment::CatchAll("path".to_string())],
                HttpMethod::REAL.to_vec(),
            ),
            rt(
                "/files/:id",
                vec![Segment::Static("files".to_string()), Segment::Param("id".to_string())],
                HttpMethod::REAL.to_vec(),
            ),
        ]);
        match table.lookup("/files/readme", HttpMethod::Get) {
            MatchResult::Found { route, .. } => {
                assert_eq!(route.path, "/files/:id");
                assert_eq!(route.params.get("id").map(String::as_str), Some("readme"));
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn lookup_trailing_slash_normalised() {
        // Spec sect.11.9: trailing slash is not a distinct
        // route identity. /foo and /foo/ both match.
        let table = table_with(vec![rt(
            "/foo",
            vec![Segment::Static("foo".to_string())],
            HttpMethod::REAL.to_vec(),
        )]);
        assert!(matches!(
            table.lookup("/foo", HttpMethod::Get),
            MatchResult::Found { .. }
        ));
        assert!(matches!(
            table.lookup("/foo/", HttpMethod::Get),
            MatchResult::Found { .. }
        ));
        // Root is the canonical form `/`.
        let root_table = table_with(vec![rt("/", vec![], HttpMethod::REAL.to_vec())]);
        assert!(matches!(
            root_table.lookup("/", HttpMethod::Get),
            MatchResult::Found { .. }
        ));
    }

    #[test]
    fn add_and_remove_by_path_round_trip() {
        let table = RouteTable::empty();
        assert_eq!(table.len(), 0);
        let r = rt("/x", vec![Segment::Static("x".to_string())], HttpMethod::REAL.to_vec());
        table.add(r.clone()).expect("first add");
        assert_eq!(table.len(), 1);
        assert_eq!(table.paths(), vec!["/x".to_string()]);
        let got = table.get_by_path("/x").expect("get");
        assert_eq!(got.path, "/x");
        let dup = table.add(r.clone());
        assert!(matches!(dup, Err(RouterError::DuplicatePath { .. })));
        assert!(table.remove_by_path("/x"));
        assert!(!table.remove_by_path("/x"));
        assert_eq!(table.len(), 0);
        assert!(table.get_by_path("/x").is_none());
    }

    #[test]
    fn router_error_codes_are_stable() {
        // Slice 16h: the host's boot-time path threads
        // RouterError::code() into a 5xx body. Pin the
        // 1xxx partition so a future refactor cannot
        // silently renumber these -- the spec's
        // ambiguous-routes example references
        // `TSP1004` (FREEZE item 14) and the rest are
        // discoverable by name.
        let cases: &[(RouterError, &str)] = &[
            (
                RouterError::RoutesDirMissing { path: PathBuf::from("/x") },
                "TSP1001",
            ),
            (
                RouterError::UnsupportedShape {
                    path: PathBuf::from("/x"),
                    reason: "x",
                },
                "TSP1002",
            ),
            (
                RouterError::DuplicatePath { path: "/x".to_string() },
                "TSP1003",
            ),
            (
                RouterError::Io {
                    path: PathBuf::from("/x"),
                    source: std::io::Error::new(std::io::ErrorKind::Other, "x"),
                },
                "TSP1004",
            ),
        ];
        for (err, want) in cases {
            assert_eq!(err.code(), *want, "err = {err:?}");
        }
    }
}
