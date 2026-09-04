# TSP native runtime

This workspace is the replacement runtime for the Bun-backed prototype.

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
checkpoints. It must not expose Bun runtime types.

`tsp-runtime` owns route execution orchestration and the worker pool while
depending only on `tsp-core` and `tsp-js`; it does not know JavaScriptCore or
Bun. `tsp-http` adapts sockets to `tsp-core` values, and `tsp-cli` composes the
JSC adapter, host runtime, and HTTP server. The compiler emits the manifest
and bundle consumed here.

Run the current boundary tests with:

```text
cargo test --manifest-path native/Cargo.toml --workspace
```

Build and run the standalone application after a WebKit/JSC build has been
prepared by Bun's dependency builder:

```text
TSP_WEBKIT_ROOT=/path/to/webkit-root \
  cargo build --manifest-path native/Cargo.toml -p tsp-cli
node scripts/native-e2e.mjs
```

`TSP_WEBKIT_ROOT` must contain `include/` and `lib/` from the same target
platform. Without it, contract builds use link-only stubs and the executable
fails before listening; it never silently falls back to Bun or another JS
runtime.
