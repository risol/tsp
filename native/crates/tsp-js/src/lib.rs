//! Engine-neutral JavaScript capability boundary.
//!
//! Host runtime code depends on this trait instead of a concrete engine
//! implementation. The interface is intentionally small and will
//! grow around request execution and polling in the async runtime phase.

use std::fmt::Display;

pub trait JsRuntime {
    type Error: Display;

    fn evaluate(&mut self, source: &str, filename: &str) -> Result<String, Self::Error>;
    /// Call a cached global JavaScript function with one JSON argument.
    ///
    /// The engine adapter parses the argument and invokes the function
    /// directly; request data must not be interpolated into source text.
    fn call_json(&mut self, function: &str, argument: &str) -> Result<String, Self::Error>;
    fn drain_microtasks(&mut self) -> Result<(), Self::Error>;
}
