//! Process-level worker protocol and lifecycle primitives for TSP v2.4.
//!
//! The worker boundary is deliberately independent from the current JSC
//! bridge. The master must be able to speak the same versioned protocol to a
//! worker whose Bun runtime is embedded in the worker executable.

pub mod protocol;
pub mod manager;
pub mod pool;
pub mod application;
pub mod lifecycle;
pub mod sandbox;
