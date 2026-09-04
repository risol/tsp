# TSP native runtime

This workspace is the replacement runtime for the Bun-backed prototype.

The dependency direction is intentionally one-way:

```text
tsp-cli -> tsp-runtime -> tsp-jsc -> JavaScriptCore/WebKit
```

`tsp-jsc` is the only crate allowed to contain JavaScriptCore FFI. It owns VM
thread affinity, native buffer ownership, evaluation errors, and microtask
checkpoints. It must not expose Bun runtime types.

`tsp-runtime` owns route matching, request/response contracts, the worker pool,
and the request-to-JavaScript dispatch protocol. The compiler emits the
manifest and bundle consumed here. `tsp-cli` loads that bundle into one JSC VM
per worker and serves it over the TSP-owned HTTP/1.1 server.

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
