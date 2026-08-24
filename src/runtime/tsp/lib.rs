//! TSP v2 native host library (see `tsp-v2-plan.md`).
//!
//! This crate is the side-by-side v2 runtime; v1 (`src/main.ts`) is
//! unchanged. The binary entry point in `bin/tspserver_v2.rs` links the
//! same library code so future tests and embedded callers can drive the
//! host without spawning a process.
//!
//! Module layout (from plan §26):
//! ```text
//! mod cli;         // CLI args
//! mod config;      // tsp.toml loader
//! mod host;        // HTTP listener bootstrap
//! mod router;      // route matcher (PoC 1: linear / only)
//! mod request;     // HTTP request -> Context bridge
//! mod response;    // Response builder
//! mod jsx;         // TSP JSX runtime (PoC 1: stub)
//! mod page_registry; // PageSlot, Generation tracking
//! mod module_graph;  // forward/reverse edges
//! mod generation;    // Generation + LKG
//! mod errors;        // TSP1xxx-5xxx codes
//! mod jsc_bridge;    // JSC + TSX transpile + execute
//! ```
//!
//! All modules are `pub(super)` for now; the binary is the only consumer
//! until Slice 4 wires up the real JSC bridge. The exact set of `mod`
//! declarations here is the per-slice progress marker.

#![doc = "Slice 1: layout + boot stub. See `bin/tspserver_v2.rs` for the entry point."]
