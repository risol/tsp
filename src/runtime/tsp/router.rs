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

/// Standard HTTP methods the v2 page protocol recognises (plan sect.4.2).
/// `HEAD` is intentionally absent: when a page exports `GET` but no
/// explicit `HEAD`, the runtime synthesises a body-less HEAD from the
/// `GET` response (plan sect.42). `OPTIONS` is also synthesized when the
/// page omits it (plan sect.42).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Options,
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
        }
    }

    /// All standard methods. Used for the `Allow:` header on 405 and as
    /// the default for a freshly scanned route before JSC validates the
    /// actual exports.
    pub const ALL: [Self; 6] = [
        Self::Get,
        Self::Post,
        Self::Put,
        Self::Patch,
        Self::Delete,
        Self::Options,
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
pub enum MatchResult<'a> {
    Found { route: &'a Route, method: HttpMethod },
    MethodNotAllowed { route: &'a Route, requested: HttpMethod },
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
        }
    }
}

impl std::error::Error for RouterError {}

use std::io;

/// In-memory route table. Slice 3 is a `Vec<Route>`; later slices swap
/// the backing store for a radix tree without changing this surface.
#[derive(Debug, Default, Clone)]
pub struct RouteTable {
    routes: Vec<Route>,
}

impl RouteTable {
    pub fn empty() -> Self {
        Self::default()
    }

    /// Number of routes currently in the table. Useful for the boot
    /// banner (`scanned N routes`) and for the dev inspector later.
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Whether the table is empty. Tests use this; production code does
    /// not because the table is populated once at boot.
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
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
        Ok(Self { routes })
    }

    /// Look up `(path, method)`. Returns the most specific match; for
    /// the slice-3 linear table that's just the first hit. Future
    /// slices (radix tree) will encode priority explicitly per
    /// plan sect.6.5.
    pub fn lookup(&self, path: &str, method: HttpMethod) -> MatchResult<'_> {
        let Some(route) = self.routes.iter().find(|r| r.path == path) else {
            return MatchResult::NotFound;
        };
        if route.methods.contains(&method) {
            MatchResult::Found { route, method }
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
            methods: HttpMethod::ALL.to_vec(),
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
            routes: vec![Route {
                path: "/".to_string(),
                source: PathBuf::from("routes/index.tsp"),
                methods: HttpMethod::ALL.to_vec(),
            }],
        };
        let m = table.lookup("/", HttpMethod::Get);
        assert!(matches!(m, MatchResult::Found { .. }));
    }

    #[test]
    fn lookup_not_found() {
        let table = RouteTable {
            routes: vec![Route {
                path: "/".to_string(),
                source: PathBuf::from("routes/index.tsp"),
                methods: HttpMethod::ALL.to_vec(),
            }],
        };
        assert!(matches!(table.lookup("/nope", HttpMethod::Get), MatchResult::NotFound));
    }

    #[test]
    fn lookup_method_not_allowed() {
        let table = RouteTable {
            routes: vec![Route {
                path: "/".to_string(),
                source: PathBuf::from("routes/index.tsp"),
                methods: vec![HttpMethod::Get], // POST not present
            }],
        };
        let m = table.lookup("/", HttpMethod::Post);
        assert!(matches!(m, MatchResult::MethodNotAllowed { .. }));
    }
}