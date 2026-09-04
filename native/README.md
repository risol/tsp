# TSP native runtime

This workspace is the replacement runtime for the Bun-backed prototype.

The dependency direction is intentionally one-way:

```text
tsp-cli -> tsp-runtime -> tsp-jsc -> JavaScriptCore/WebKit
```

`tsp-jsc` is the only crate allowed to contain JavaScriptCore FFI. It owns VM
thread affinity, native buffer ownership, evaluation errors, and microtask
checkpoints. It must not expose Bun runtime types.

`tsp-runtime` owns route matching and request/response contracts. The compiler
emits the `RouteSpec` data consumed here; workers will evaluate the emitted
JavaScript through `tsp-jsc` after the native binding smoke test is complete.

Run the current boundary tests with:

```text
cargo test --manifest-path native/Cargo.toml --workspace
```
