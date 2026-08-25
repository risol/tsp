//! TSP v2 native host library (see `tsp-v2-plan.md`).
//!
//! This crate is the side-by-side v2 runtime; v1 (`src/main.ts`) is
//! unchanged. The binary entry point in `bin/tspserver_v2.rs` links the
//! same library code so future tests and embedded callers can drive the
//! host without spawning a process.
//!
//! Module layout (from plan sect.26):
//! ```text
//! mod cli;         // CLI args
//! mod config;      // tsp.toml loader
//! mod host;        // HTTP listener bootstrap
//! mod router;      // route matcher (PoC 1: linear / only)
//! mod request;     // HTTP request -> Context bridge
//! mod response;    // Response builder
//! mod jsx;         // TSP JSX runtime (PoC 1: subprocess path)
//! mod page_registry; // PageSlot, Generation tracking
//! mod module_graph;  // forward/reverse edges
//! mod generation;    // Generation + LKG
//! mod pipeline;     // slice 10b sync build pipeline
//! mod errors;        // TSP1xxx-5xxx codes
//! mod jsc_bridge;    // JSC exec (PoC 1: spawns bun.exe)
//! mod in_process_jsc; // JSC exec (slice 7+ spike: dep reachable, API not yet wired)
//! ```
//!
//! Each `mod` is added the slice it lands in.

#![doc = "Slice 11: filesystem watcher (polling backend) + lazy reload."]

pub mod generation;
pub mod host;
pub mod in_process_jsc;
pub mod jsc_bridge;
pub mod jsx;
pub mod module_graph;
pub mod page;
pub mod pipeline;
pub mod router;
pub mod services;
pub mod session_backend;
pub mod watcher;
