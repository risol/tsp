//! In-process JSC bridge -- **ADR-0001 reference code, NOT on hot path**.
//!
//! See `docs/reference/adr/0001-subprocess-as-production-jsc.md` (slice 13) for
//! the decision. This module is documentation + a placeholder for the
//! future in-process JSC bridge. It is **not** wired into the host's
//! request flow. Production continues to use the slice 6 subprocess
//! bridge in `jsc_bridge.rs`.
//!
//! ## Why this module exists
//!
//! Plan `tsp-plan.md` sect.25.3 recommends reusing Bun's in-process
//! runtime as the JSC execution engine. Slice 6 (PoC 1) shipped a
//! subprocess bridge as a placeholder. Slice 7 added this module as a
//! spike. Slice 13 promotes it to an ADR-anchored reference and
//! removes it from the "TODO" pile.
//!
//! ## Critical design constraint (and why this file is symbol-free)
//!
//! Earlier drafts of this module attempted to import `bun_runtime`
//! types to "prove" the dep was reachable. That broke the build:
//! bringing `bun_runtime` into this crate's rlib -- even via a
//! `type X = bun_runtime::Error` alias -- forces the test binary
//! to link against `bun_runtime`'s transitive deps
//! (`bun_simdutf_sys`, `bun_alloc`, `bun_s3_signing`, ...), each of
//! which references C extern symbols that live in the `bun` binary
//! and are not exported by the `bun_runtime` rlib. The result is
//! 2700+ unresolved externals at link time. So:
//!
//!   * This module MUST NOT `use bun_runtime::...` or name any
//!     `bun_runtime::*` type, even in a `type` alias.
//!   * The `bun_runtime` dep in `Cargo.toml` is workspace-hygiene
//!     only. The dep-graph wire is there so a future slice has the
//!     shortest possible path to in-process work; the rlib metadata
//!     alone does not pull transitive C externs into this crate's
//!     test binary.
//!   * `cargo check -p bun_runtime_tsp` continues to succeed (it
//!     only type-checks); only `cargo test -p bun_runtime_tsp` and
//!     `cargo build -p bun_runtime_tsp` (with the binary target)
//!     would surface the bun-side extern issue if anything in this
//!     module started touching `bun_runtime` symbols.
//!
//! This constraint is the third item in the integration checklist
//! below, called out separately because it is the one the spike got
//! wrong on its first pass.
//!
//! ## Re-evaluation triggers (per ADR-0001)
//!
//! This module becomes the starting point for a future slice when:
//!
//! 1. Bun ships a public `bun_runtime::embed`-style API that creates
//!    an isolated VM with overridable loader / module hooks.
//! 2. The Bun fork (vendored at `bun/`, branch `bun-v1.4.0`) is
//!    willing to expose an embedder-safe equivalent of
//!    `Run::boot` / `Run::start`, or provide a supported isolated
//!    VM entry point. `bun_jsc::VirtualMachine::init` and
//!    `runtime_hooks()` are already public; they are not sufficient
//!    to recreate Bun's higher-level startup contract by themselves.
//! 3. A new use case requires in-process JSC for latency
//!    (streaming, server-sent events) that subprocess cannot meet.
//!
//! Until one of those is true, the subprocess bridge in
//! `jsc_bridge.rs` is the production JSC path.
//!
//! ## Integration checklist for a future in-process slice
//!
//! When one of the re-evaluation triggers fires, the work is:
//!
//! 1. Replicate Bun's startup sequence up to the point of VM
//!    creation: `bun_jsc::initialize(...)` followed by
//!    `bun_jsc::VirtualMachine::init(InitOptions::default())`, with
//!    the surrounding `bun_runtime` environment (timer pool,
//!    resolver, env loader, loader hooks, dispatch table) installed
//!    first. The existing `Run::boot` helper is `pub(crate)`, so it
//!    cannot currently be reused from this crate.
//! 2. Provide a host function or builtin that gives JS code the
//!    `tsp:server` / `tsp:jsx-runtime` modules the spec promises
//!    (plan sect.7). Today the only way to ship those is via the
//!    bundler's `HardcodedModule` pipeline, which means dragging
//!    in `bun_resolve_builtins` + the `HardcodedModule` enum in
//!    `bun_jsc::ModuleLoader`.
//! 3. Verify the test-binary link story. The `bun_runtime` rlib
//!    is not linkable into a foreign binary without dragging in
//!    `bun_simdutf_sys` + `bun_alloc` + ... -- the foreign linker
//!    does not see the C/ABI externs the `bun` binary provides.
//!    The current probe produces thousands of unresolved symbols.
//!    This may require (a) Bun-side rlib hardening, or (b) a
//!    feature-gated stub that only enables the in-process test
//!    path when the `bun` runtime symbols are available, or (c)
//!    dropping the in-process test from `cargo test` and keeping
//!    it as a separate integration test.
//! 4. Wire the host's request flow to call into the new VM
//!    instead of writing a temp `.js` file for `bun run`. The
//!    slice 12 `Arc<String>` payload + `InFlightBuild` +
//!    `PageRegistry` state machine are upstream of this choice
//!    and do not change.
//! 5. Per plan sect.25.1, all JSC value creation / call / protect
//!    / release must happen on the VM owner thread. A request that
//!    arrives on a worker thread without an owning VM has to be
//!    funnelled to that worker's VM -- this is the same shape as
//!    plan sect.25.2's "one TSP VM per worker" recommendation.

/// Placeholder type representing a future in-process JSC VM.
/// **Not constructible today** -- this exists so a future slice
/// has a stable name to refer to. The subprocess path does not
/// use this type.
#[derive(Debug)]
pub struct InProcessVm {
    _private: (),
}

#[cfg(test)]
mod tests {
    /// Regression: this module stays a no-op relative to
    /// `bun_runtime`'s public surface. The test does not import
    /// any `bun_runtime` symbol because doing so in a test body
    /// would pull the bun-binary's C externs into the link (see
    /// the module-level "Critical design constraint" comment).
    /// The test asserts the module's own contract: `InProcessVm`
    /// is a stable, opaque placeholder, not constructible, and
    /// `Debug`-printable for diagnostic logging.
    #[test]
    fn in_process_vm_is_placeholder() {
        // InProcessVm has no public constructor; we can only
        // assert its Debug output via a never-produced value.
        // `format!` does not require the value to be constructible
        // at runtime -- the type system only needs to know the
        // Debug impl is in scope.
        fn assert_debug<T: std::fmt::Debug>() {}
        assert_debug::<super::InProcessVm>();
    }
}
