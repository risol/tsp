//! Engine-neutral JavaScript capability boundary.
//!
//! Host runtime code depends on this trait instead of a JavaScriptCore or Bun
//! implementation. The interface is intentionally small in Phase 2 and will
//! grow around request execution and polling in the async runtime phase.

use std::fmt::Display;

pub trait JsRuntime {
    type Error: Display;

    fn evaluate(&mut self, source: &str, filename: &str) -> Result<String, Self::Error>;
    fn drain_microtasks(&mut self) -> Result<(), Self::Error>;
}
