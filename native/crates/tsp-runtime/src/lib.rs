//! TSP host runtime contracts.
//!
//! Domain values come from `tsp-core`; JavaScript execution is injected via
//! `tsp-js`. This crate owns worker scheduling and request lifecycle only.

pub use tsp_core::{
    BodyEnvelope, CompiledManifest, Effects, ModuleSpec, PROTOCOL_VERSION, RUNTIME_ABI_VERSION,
    Request, RequestEnvelope, Response, ResponseEnvelope, RouteError, RouteMatch, RouteSpec,
    RouteTable, SessionEffect,
};

pub mod worker;
pub use worker::{RouteExecutor, WorkerError, WorkerExecutor, WorkerPool};
