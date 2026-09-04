//! Stable TSP domain types.
//!
//! This crate is deliberately independent from JavaScript engines, HTTP
//! implementations, process management, and the command-line composition
//! root. It is the only shared data model between those layers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const PROTOCOL_VERSION: u16 = 1;
pub const RUNTIME_ABI_VERSION: u16 = 1;

pub const WORKER_PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GenerationId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
pub enum BodyEnvelope {
    Empty,
    Text(String),
    Bytes(Vec<u8>),
}

impl BodyEnvelope {
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Self::Empty => Vec::new(),
            Self::Text(value) => value.into_bytes(),
            Self::Bytes(value) => value,
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Empty => &[],
            Self::Text(value) => value.as_bytes(),
            Self::Bytes(value) => value,
        }
    }
}

impl From<Vec<u8>> for BodyEnvelope {
    fn from(value: Vec<u8>) -> Self {
        if value.is_empty() {
            Self::Empty
        } else {
            Self::Bytes(value)
        }
    }
}

impl From<String> for BodyEnvelope {
    fn from(value: String) -> Self {
        if value.is_empty() {
            Self::Empty
        } else {
            Self::Text(value)
        }
    }
}

impl From<&str> for BodyEnvelope {
    fn from(value: &str) -> Self {
        Self::from(value.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestEnvelope {
    pub version: u16,
    pub request_id: String,
    pub generation: Option<GenerationId>,
    pub method: String,
    pub target: String,
    pub http_version: String,
    pub headers: Vec<(String, String)>,
    pub body: BodyEnvelope,
}

impl RequestEnvelope {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseEnvelope {
    pub version: u16,
    pub request_id: String,
    pub generation: Option<GenerationId>,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: BodyEnvelope,
    pub effects: Effects,
}

impl ResponseEnvelope {
    pub fn new(status: u16, body: impl Into<BodyEnvelope>) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            request_id: String::new(),
            generation: None,
            status,
            headers: Vec::new(),
            body: body.into(),
            effects: Effects::default(),
        }
    }
}

pub type Request = RequestEnvelope;
pub type Response = ResponseEnvelope;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effects {
    pub cookies: Vec<String>,
    pub session: Vec<SessionEffect>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionEffect {
    pub operation: String,
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WorkerCommand {
    Hello {
        version: u16,
    },
    LoadGeneration {
        generation: GenerationId,
        bundle: String,
        filename: String,
    },
    Execute {
        request: RequestEnvelope,
        route: RouteSpec,
        params: HashMap<String, String>,
    },
    Cancel {
        request_id: String,
    },
    Ping,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WorkerEvent {
    Ready {
        version: u16,
    },
    GenerationReady {
        generation: GenerationId,
    },
    Result(ResponseEnvelope),
    Error {
        request_id: Option<String>,
        message: String,
    },
    Pong,
    Exiting,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RouteSpec {
    pub path: String,
    pub source: String,
    pub output: String,
    pub methods: Vec<String>,
    /// Dynamic parameter names in path order. The wildcard path is rendered
    /// as `*` in the manifest, so its source name is carried here.
    pub parameters: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleSpec {
    pub source: String,
    pub output: String,
    pub exports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledManifest {
    pub version: u32,
    #[serde(rename = "runtimeAbi", alias = "runtime_abi")]
    pub runtime_abi: u16,
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
    UnsupportedManifest { version: u32, runtime_abi: u16 },
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPath => formatter.write_str("route path cannot be empty"),
            Self::InvalidPath(path) => write!(formatter, "invalid route path: {path}"),
            Self::DuplicatePath(path) => write!(formatter, "duplicate route path: {path}"),
            Self::UnsupportedManifest {
                version,
                runtime_abi,
            } => write!(
                formatter,
                "unsupported manifest version {version} and runtime ABI {runtime_abi}"
            ),
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
        if manifest.version != 1 || manifest.runtime_abi != RUNTIME_ABI_VERSION {
            return Err(RouteError::UnsupportedManifest {
                version: manifest.version,
                runtime_abi: manifest.runtime_abi,
            });
        }
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
                path: "/users/:id".into(),
                parameters: vec!["id".into()],
                ..route("/users/:id")
            },
            route("/users/new"),
            RouteSpec {
                path: "/users/*".into(),
                parameters: vec!["rest".into()],
                ..route("/users/*")
            },
        ])
        .unwrap();
        assert_eq!(table.lookup("/users/new").unwrap().route.path, "/users/new");
        assert_eq!(table.lookup("/users/42").unwrap().route.path, "/users/:id");
        assert_eq!(table.lookup("/users/a/b").unwrap().route.path, "/users/*");
    }

    #[test]
    fn duplicate_and_invalid_routes_are_rejected() {
        assert_eq!(
            RouteTable::new([route("/"), route("/")]).unwrap_err(),
            RouteError::DuplicatePath("/".into())
        );
        assert_eq!(
            RouteTable::new([route("users")]).unwrap_err(),
            RouteError::InvalidPath("users".into())
        );
    }

    #[test]
    fn envelopes_are_versioned_and_preserve_binary_bodies() {
        let request = Request {
            version: PROTOCOL_VERSION,
            request_id: "r-1".into(),
            generation: None,
            method: "POST".into(),
            target: "/upload".into(),
            http_version: "HTTP/1.1".into(),
            headers: vec![("content-type".into(), "application/octet-stream".into())],
            body: BodyEnvelope::Bytes(vec![0, 1, 2]),
        };
        let serialized = serde_json::to_string(&request).unwrap();
        let decoded: RequestEnvelope = serde_json::from_str(&serialized).unwrap();
        assert_eq!(decoded.version, PROTOCOL_VERSION);
        assert_eq!(decoded.body, BodyEnvelope::Bytes(vec![0, 1, 2]));
        assert_eq!(decoded.request_id, "r-1");
    }
}
