//! Synchronous build pipeline for TSP v2 slice 10b (plan sect.21).
//!
//! Combines `page::prepare` (slice 5 source reader) and
//! `jsc_bridge::execute` (slice 6 bun subprocess bridge) into
//! a single `build(route, method, bun)` call. The host's
//! request flow calls this when a `PageSlot` is `Unloaded` or
//! `Dirty` and we are the thread that wins the `begin_build`
//! race.
//!
//! Scope for slice 10b:
//! - `build(route, method, bun) -> Result<String, BuildError>`
//!   where the `String` is the rendered HTTP body (the page
//!   handler\'s return value, post-bun-eval).
//! - The payload is then stored on the `Generation` via a new
//!   `payload` field so the next request can serve from the
//!   registry without re-running the build.
//!
//! Out of slice 10b (deferred to slice 10c):
//! - In-flight dedup (concurrent requests on a Building slot
//!   share the build future). Slice 10b lets the second
//!   request hit `begin_build` and get a `NotBuildable`; the
//!   host then serves from LKG or 503.
//! - Request pinning (a request on generation N finishes on N
//!   even if the file changes mid-flight).

use crate::jsc_bridge::{self, BunRuntime};
use crate::page::{self, PrepareError};
use crate::router::{HttpMethod, Route};

#[derive(Debug)]
pub enum BuildError {
    Prepare(PrepareError),
    Jsc(jsc_bridge::JscError),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Prepare(e) => write!(f, "prepare: {e}"),
            Self::Jsc(e) => write!(f, "jsc: {e}"),
        }
    }
}

impl std::error::Error for BuildError {}

/// Build a page: read the source, transpile + evaluate via
/// `bun`, return the rendered HTTP body. The body is what the
/// `HttpResponse` field of the `Generation` carries to the
/// request handler.
pub fn build(route: &Route, method: HttpMethod, bun: &BunRuntime) -> Result<String, BuildError> {
    let source = page::prepare(route).map_err(BuildError::Prepare)?;
    jsc_bridge::execute(bun, &source.text, method).map_err(BuildError::Jsc)
}