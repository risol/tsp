//! In-process JSC bridge -- SLICE 7 SPIKE / NOT YET WIRED.
//!
//! See `tsp-v2-plan.md` sect.25.3 ("v2 更推荐直接在 Bun Rust runtime
//! 内实现 TSP host"). Slice 7 added `bun_runtime` as a dep so this
//! crate can call into Bun's JSC VM directly instead of shelling out
//! to the vendored `bun.exe` (the PoC 1 slice 6 path in
//! `jsc_bridge.rs`).
//!
//! Status: spike only. `bun_runtime`'s public Rust API is designed
//! for Bun's own use -- `VirtualMachine::init` is 100+ lines of
//! `addr_of_mut!`-style self-referential in-place init that depends
//! on the surrounding CLI / dispatch / loader hooks being live.
//! There is no "embed me in your binary" high-level wrapper, and
//! every observed call site (`jsc_hooks.rs`, `test_runner`, `bake`)
//! runs in the context of a full Bun process bootstrap.
//!
//! What this file does today:
//! - Documents the gap and the integration surface a future slice
//!   has to land.
//! - Verifies that `bun_runtime` is reachable from this crate
//!   (the dep compiles, the symbol resolves) without actually
//!   initialising a VM.
//!
//! What a future slice has to do to close the gap:
//! 1. Replicate Bun's startup sequence up to the point of VM
//!    creation -- the slice 7+ entry point is
//!    `bun_runtime::init()` followed by
//!    `bun_jsc::VirtualMachine::init(InitOptions::default())`,
//!    but the surrounding `bun_runtime` environment (timer pool,
//!    resolver, env loader, loader hooks, dispatch table) has to
//!    be installed first. The low-tier extern symbols
//!    `__BUN_RUNTIME_HOOKS` and `__BUN_LOADER_HOOKS` are the
//!    wiring points; today they live in `bun_runtime::jsc_hooks`
//!    and are statically linked into the `bun` binary.
//! 2. Provide a host function or builtin that gives JS code the
//!    `tsp:server` / `tsp:jsx-runtime` modules the spec promises.
//!    Today the only way to ship those is via the bundler's
//!    HardcodedModule pipeline, which means dragging in the
//!    `bun_resolve_builtins` machinery and the `HardcodedModule`
//!    enum in `bun_jsc::ModuleLoader` -- a substantial crate-graph
//!    surface that the PoC 1 path intentionally avoided by using
//!    the subprocess instead.
//! 3. Wire the host's request flow to call into the new VM
//!    instead of writing a temp .js file for `bun run`. Slice 6
//!    already isolated the call site (`jsc_bridge::execute`) so
//!    the swap is local: a second `execute` impl that takes
//!    `&InProcessVm` and returns the same `String`.
//!
//! The `bun_runtime` dep stays in `Cargo.toml` so the workspace
//! is ready for slice 7+; the `bin/tspserver_v2.rs` still picks
//! the subprocess path by default, gated on `TSP_JSC=in-process`
//! when the wiring above lands. Until then, this module is
//! documentation + compile check.