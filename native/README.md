# TSP native runtime

This workspace is TSP's standalone native runtime.

The dependency direction is intentionally layered:

```text
tsp-cli -> tsp-runtime -> tsp-js <- tsp-jsc -> JavaScriptCore/WebKit
             │
             └─────────────── tsp-core
```

`tsp-core` contains the engine- and transport-neutral domain model and
versioned protocol types. `tsp-js` defines the JavaScript capability boundary.
`tsp-jsc` is the only crate allowed to contain JavaScriptCore FFI; it owns VM
thread affinity, native buffer ownership, evaluation errors, and microtask
checkpoints.

`tsp-runtime` owns route execution orchestration, generation lifecycle, and
the process worker manager while depending only on `tsp-core` and `tsp-js`; it
does not know JavaScriptCore. `tsp-http` adapts sockets to `tsp-core` values,
and `tsp-cli` composes the host runtime and HTTP server. The compiler emits the
manifest and bundle consumed here.

Run the current boundary tests with:

```text
cargo test --manifest-path native/Cargo.toml --workspace
```

Build and run the standalone application after a TSP JSC SDK has been
installed:

```text
TSP_JSC_SDK_ROOT=/path/to/tsp-jsc-sdk \
  cargo build --manifest-path native/Cargo.toml -p tsp-cli -p tsp-worker
node scripts/native-e2e.mjs
```

`TSP_JSC_SDK_ROOT` must contain `include/` and `lib/` from the same target
platform. The SDK's ABI-compatible allocator is supplied by the Rust
dependency graph. Without the SDK, contract builds use link-only stubs and the
executable fails before listening; it never silently falls back to another JS
runtime.
