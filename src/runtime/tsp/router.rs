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

/// One row of the route table: the URL path the request must hit, the
/// `.tsp` source file that serves it, and the set of HTTP methods the
/// runtime thinks the file exports (slice 3 says "all of them" until
/// JSC proves otherwise in slice 5).
#[derive(Debug, Clone)]
pub struct Route {
    pub path: String,
    pub source: PathBuf,
    pub methods: Vec<HttpMethod>,
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
    NotFound,
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

    /// Look up `(path, method)`. Returns the most specific match; for
    /// the slice-3 linear table that's just the first hit. Future
    /// slices (radix tree) will encode priority explicitly per
    /// plan sect.6.5.
    pub fn lookup(&self, path: &str, method: HttpMethod) -> MatchResult {
        // Lock the table briefly. The match arms hold a
        // reference into the lock guard; we copy the matched
        // `Route` into a local before returning so the guard
        // can drop without affecting the borrow. `MatchResult`
        // borrows for `'static` because the `Route` is owned
        // by this function (via the local copy), not by the
        // table itself.
        let guard = self.routes.lock().expect("route table lock poisoned");
        let Some(route) = guard.iter().find(|r| r.path == path).cloned() else {
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
        let url_path = url_path_for(root, &path, stem)?;
        out.push(Route {
            path: url_path,
            source: path,
            methods: HttpMethod::REAL.to_vec(),
        });
    }
    Ok(())
}

/// Translate a `.tsp` filename (relative to the `routes/` root) into a
/// URL path. Slice 3 only knows the static + index shapes; anything
/// else is `UnsupportedShape` so the runtime refuses to boot with a
/// half-understood route.
fn url_path_for(root: &Path, abs: &Path, stem: &str) -> Result<String, RouterError> {
    // `relative` is the path under `routes/`, e.g. `users/index.tsp`
    // or just `index.tsp`. Strip the `.tsp` (already done by caller) so
    // we work in segments below.
    let rel = abs.strip_prefix(root).map_err(|_| RouterError::UnsupportedShape {
        path: abs.to_path_buf(),
        reason: "file is not under the routes/ root",
    })?;
    let mut segments: Vec<&str> = rel
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    // Drop the filename -- the directory structure of `rel` is what
    // becomes the URL, the filename's stem is the last segment.
    let _ = segments.pop();

    if stem == "index" {
        // `routes/index.tsp`         -> `/`
        // `routes/users/index.tsp`    -> `/users`
        let joined = segments.join("/");
        return Ok(if joined.is_empty() {
            "/".to_string()
        } else {
            format!("/{joined}")
        });
    }

    if stem.contains('[') || stem.contains(']') {
        return Err(RouterError::UnsupportedShape {
            path: abs.to_path_buf(),
            reason: "dynamic / catch-all segments are not slice-3 supported",
        });
    }

    // `routes/foo.tsp`            -> `/foo`
    // `routes/users/new.tsp`       -> `/users/new`
    segments.push(stem);
    Ok(format!("/{}", segments.join("/")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_path_index_root() {
        // `routes/index.tsp` -> `/`
        let root = Path::new("/app/routes");
        let abs = Path::new("/app/routes/index.tsp");
        assert_eq!(url_path_for(root, abs, "index").unwrap(), "/");
    }

    #[test]
    fn url_path_index_nested() {
        // `routes/users/index.tsp` -> `/users`
        let root = Path::new("/app/routes");
        let abs = Path::new("/app/routes/users/index.tsp");
        assert_eq!(url_path_for(root, abs, "index").unwrap(), "/users");
    }

    #[test]
    fn url_path_static() {
        // `routes/login.tsp` -> `/login`
        let root = Path::new("/app/routes");
        let abs = Path::new("/app/routes/login.tsp");
        assert_eq!(url_path_for(root, abs, "login").unwrap(), "/login");
    }

    #[test]
    fn url_path_static_nested() {
        // `routes/users/new.tsp` -> `/users/new`
        let root = Path::new("/app/routes");
        let abs = Path::new("/app/routes/users/new.tsp");
        assert_eq!(url_path_for(root, abs, "new").unwrap(), "/users/new");
    }

    #[test]
    fn url_path_dynamic_rejected() {
        // `routes/users/[id].tsp` is slice-7+ territory; slice 3 must
        // refuse to start rather than silently 404.
        let root = Path::new("/app/routes");
        let abs = Path::new("/app/routes/users/[id].tsp");
        let err = url_path_for(root, abs, "[id]").unwrap_err();
        assert!(matches!(err, RouterError::UnsupportedShape { .. }));
    }

    #[test]
    fn lookup_found() {
        let table = RouteTable {
            routes: Arc::new(Mutex::new(vec![Route {
                path: "/".to_string(),
                source: PathBuf::from("routes/index.tsp"),
                methods: HttpMethod::REAL.to_vec(),
            }])),
        };
        let m = table.lookup("/", HttpMethod::Get);
        assert!(matches!(m, MatchResult::Found { .. }));
    }

    #[test]
    fn lookup_not_found() {
        let table = RouteTable {
            routes: Arc::new(Mutex::new(vec![Route {
                path: "/".to_string(),
                source: PathBuf::from("routes/index.tsp"),
                methods: HttpMethod::REAL.to_vec(),
            }])),
        };
        assert!(matches!(table.lookup("/nope", HttpMethod::Get), MatchResult::NotFound));
    }

    #[test]
    fn lookup_method_not_allowed() {
        let table = RouteTable {
            routes: Arc::new(Mutex::new(vec![Route {
                path: "/".to_string(),
                source: PathBuf::from("routes/index.tsp"),
                methods: vec![HttpMethod::Get], // POST not present
            }])),
        };
        let m = table.lookup("/", HttpMethod::Post);
        assert!(matches!(m, MatchResult::MethodNotAllowed { .. }));
    }

    #[test]
    fn add_and_remove_by_path_round_trip() {
        let table = RouteTable::empty();
        assert_eq!(table.len(), 0);
        let r = Route {
            path: "/x".to_string(),
            source: PathBuf::from("routes/x.tsp"),
            methods: HttpMethod::REAL.to_vec(),
        };
        table.add(r.clone()).expect("first add");
        assert_eq!(table.len(), 1);
        assert_eq!(table.paths(), vec!["/x".to_string()]);
        // get_by_path returns the cloned Route.
        let got = table.get_by_path("/x").expect("get");
        assert_eq!(got.path, "/x");
        // Duplicate add fails with RouterError::DuplicatePath.
        let dup = table.add(r.clone());
        assert!(matches!(dup, Err(RouterError::DuplicatePath { .. })));
        // remove_by_path returns true on the first remove, false
        // on the second (idempotent no-op).
        assert!(table.remove_by_path("/x"));
        assert!(!table.remove_by_path("/x"));
        assert_eq!(table.len(), 0);
        assert!(table.get_by_path("/x").is_none());
    }
}