//! TSP-owned runtime contracts independent of Bun.

use serde::Deserialize;
use std::collections::HashMap;

pub use tsp_http::{Request, Response};

pub mod worker;
pub use worker::{WorkerError, WorkerExecutor, WorkerPool};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct RouteSpec {
    pub path: String,
    pub source: String,
    pub output: String,
    pub methods: Vec<String>,
    /// Dynamic parameter names in path order. The wildcard path is rendered
    /// as `*` in the manifest, so its source name is carried here.
    pub parameters: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ModuleSpec {
    pub source: String,
    pub output: String,
    pub exports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CompiledManifest {
    pub version: u32,
    pub compiler: String,
    #[serde(rename = "sourceRoot", alias = "source_root")]
    pub source_root: String,
    pub routes: Vec<RouteSpec>,
    pub modules: Vec<ModuleSpec>,
}

impl CompiledManifest {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteMatch<'a> {
    pub route: &'a RouteSpec,
    pub params: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteError {
    EmptyPath,
    InvalidPath(String),
    DuplicatePath(String),
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPath => formatter.write_str("route path cannot be empty"),
            Self::InvalidPath(path) => write!(formatter, "invalid route path: {path}"),
            Self::DuplicatePath(path) => write!(formatter, "duplicate route path: {path}"),
        }
    }
}

impl std::error::Error for RouteError {}

#[derive(Debug, Default)]
pub struct RouteTable {
    routes: Vec<RouteSpec>,
}

impl RouteTable {
    pub fn new(routes: impl IntoIterator<Item = RouteSpec>) -> Result<Self, RouteError> {
        let mut table = Self::default();
        for route in routes {
            table.insert(route)?;
        }
        table
            .routes
            .sort_by_key(|route| route_priority(&route.path));
        Ok(table)
    }

    pub fn routes(&self) -> &[RouteSpec] {
        &self.routes
    }

    pub fn from_manifest(manifest: &CompiledManifest) -> Result<Self, RouteError> {
        Self::new(manifest.routes.clone())
    }

    pub fn lookup(&self, path: &str) -> Option<RouteMatch<'_>> {
        let request_segments = split_path(path)?;
        self.routes.iter().find_map(|route| {
            let pattern_segments = split_path(&route.path)?;
            let mut params = HashMap::new();
            if match_segments(
                &pattern_segments,
                &request_segments,
                &route.parameters,
                &mut params,
            ) {
                Some(RouteMatch { route, params })
            } else {
                None
            }
        })
    }

    fn insert(&mut self, route: RouteSpec) -> Result<(), RouteError> {
        if route.path.is_empty() {
            return Err(RouteError::EmptyPath);
        }
        if !route.path.starts_with('/') || route.path.contains("//") {
            return Err(RouteError::InvalidPath(route.path));
        }
        if self
            .routes
            .iter()
            .any(|existing| existing.path == route.path)
        {
            return Err(RouteError::DuplicatePath(route.path));
        }
        self.routes.push(route);
        Ok(())
    }
}

fn split_path(path: &str) -> Option<Vec<&str>> {
    if !path.starts_with('/') || path.contains("//") {
        return None;
    }
    if path == "/" {
        return Some(Vec::new());
    }
    Some(path[1..].split('/').collect())
}

fn route_priority(path: &str) -> (u8, usize) {
    let segments = split_path(path).unwrap_or_default();
    let dynamic = segments
        .iter()
        .filter(|segment| segment.starts_with(':'))
        .count();
    let catch_all = segments
        .iter()
        .filter(|segment| segment.starts_with('*'))
        .count();
    (
        dynamic as u8 + catch_all as u8 * 2,
        usize::MAX - segments.len(),
    )
}

fn match_segments(
    pattern: &[&str],
    request: &[&str],
    parameter_names: &[String],
    params: &mut HashMap<String, String>,
) -> bool {
    let mut request_index = 0;
    let mut parameter_index = 0;
    for (pattern_index, segment) in pattern.iter().enumerate() {
        if let Some(name) = segment.strip_prefix('*') {
            if pattern_index + 1 != pattern.len() {
                return false;
            }
            let parameter_name = if name.is_empty() {
                parameter_names.get(parameter_index).map(String::as_str)
            } else {
                Some(name)
            };
            let Some(parameter_name) = parameter_name.filter(|name| !name.is_empty()) else {
                return false;
            };
            params.insert(
                parameter_name.to_owned(),
                request[request_index..].join("/"),
            );
            return true;
        }
        let Some(value) = request.get(request_index) else {
            return false;
        };
        if let Some(name) = segment.strip_prefix(':') {
            if name.is_empty() {
                return false;
            }
            params.insert(name.to_owned(), (*value).to_owned());
            parameter_index += 1;
        } else if segment != value {
            return false;
        }
        request_index += 1;
    }
    request_index == request.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(path: &str) -> RouteSpec {
        RouteSpec {
            path: path.into(),
            source: format!("{path}.tsp"),
            output: format!("{path}.js"),
            methods: vec!["GET".into()],
            parameters: Vec::new(),
        }
    }

    #[test]
    fn static_routes_beat_dynamic_and_catch_all_routes() {
        let table = RouteTable::new([
            RouteSpec {
                parameters: vec!["path".into()],
                ..route("/users/*")
            },
            route("/users/:id"),
            route("/users/me"),
        ])
        .unwrap();
        assert_eq!(table.lookup("/users/me").unwrap().route.path, "/users/me");
        assert_eq!(table.lookup("/users/42").unwrap().params["id"], "42");
        assert_eq!(table.lookup("/users/a/b").unwrap().params["path"], "a/b");
    }

    #[test]
    fn duplicate_routes_are_rejected() {
        let error = RouteTable::new([route("/"), route("/")]).unwrap_err();
        assert_eq!(error, RouteError::DuplicatePath("/".into()));
    }

    #[test]
    fn invalid_paths_do_not_enter_the_table() {
        let error = RouteTable::new([route("users")]).unwrap_err();
        assert_eq!(error, RouteError::InvalidPath("users".into()));
    }

    #[test]
    fn manifest_json_is_the_runtime_route_boundary() {
        let manifest = CompiledManifest::from_json(
            r#"{
                "version": 1,
                "compiler": "tspc-typescript-frontend",
                "source_root": "pages",
                "routes": [{
                    "path": "/users/*",
                    "source": "users/[...path].tsp",
                    "output": "users/[...path].js",
                    "methods": ["GET"],
                    "parameters": ["path"]
                }],
                "modules": []
            }"#,
        )
        .unwrap();
        let table = RouteTable::from_manifest(&manifest).unwrap();
        let matched = table.lookup("/users/a/b").unwrap();
        assert_eq!(matched.route.output, "users/[...path].js");
        assert_eq!(matched.params["path"], "a/b");
    }
}
