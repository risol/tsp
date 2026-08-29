//! TSP v2 native host library (see `tsp-v2-plan.md`).
//!
//! This crate is the native TSP v2 runtime. The binary entry point in
//! `bin/tspserver_v2.rs` links the same library code so tests and embedded
//! callers can drive the host without spawning a process.
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

// Allow the binary entry source to be reused by the bundled Bun executable.
// The standalone `tspserver_v2` binary imports this crate by name, while the
// bundled entry compiles the same source as a module of this crate.
extern crate self as bun_runtime_tsp;

pub mod generation;
pub mod host;
pub mod in_process_jsc;
pub mod invalidation_bus;
pub mod jsc_bridge;
pub mod jsx;
pub mod metrics;
pub mod module_graph;
pub mod page;
pub(crate) mod path;
pub mod pipeline;
pub mod router;
pub mod services;
pub mod session_backend;
pub mod static_files;
pub mod typings;
pub mod watcher;
pub mod worker;

#[path = "bin/tspserver_v2.rs"]
pub mod entry;
