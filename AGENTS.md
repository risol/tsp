# AGENTS.md

This repository contains TSP v2 only. The native runtime is implemented under
`bun/src/runtime/tsp`; the root project contains route fixtures, v2 tooling,
documentation, and packaging helpers.

## Language requirements

Code comments, variable names, and repository documentation must be in English.
User-facing conversation may use any language requested by the user.

## Architecture

The request path is:

```text
HTTP request -> Rust host -> route table -> generation registry -> Bun worker
             -> v2 handler -> response envelope -> HTTP response
```

TSP v2 is intentionally incompatible with the former v1 `Page()` wrapper,
global dependency injection, React page runtime, and `src/main.ts` host.

## Route rules

- `.tsp` files under `routes/` are HTTP route modules.
- Export HTTP methods explicitly, such as `GET`, `POST`, `PUT`, or `DELETE`.
- Use `tsp:server` for `Context`, response helpers, fragments, and errors.
- Use `tsp:html` for trusted HTML and escaping helpers.
- Dynamic route segments use `[name]`, for example `routes/users/[id].tsp`.
- Static assets belong in `public/` and must not be implemented as route handlers.
- Do not import from the deleted v1 `src/` tree or use the v1 global types.

Example:

```tsx
import { type Context, json } from "tsp:server";

export function GET(ctx: Context) {
  return json({ path: ctx.url.pathname, method: ctx.method });
}
```

## Commands

```bash
./tsp.sh build       # Build the single-file runtime and package it
./tsp.sh dev         # Run v2 with route hot reload
./tsp.sh start       # Run v2 using the same route contract
./tsp.sh check       # cargo check for tspserver_v2
./tsp.sh test        # Rust tests and v2 smoke test
```

The packaged `tspserver_v2` accepts configuration through `TSP_PORT`,
`TSP_ROUTES_DIR`, `TSP_PUBLIC_DIR`, `TSP_WORKER_COUNT`, and the other variables
shown by `tspserver_v2 --help`. Worker processes are created by the same
executable; no separate worker binary is required.

## Native runtime and allocator boundaries

The TSP native runtime is embedded in Bun and uses Bun's mimalloc-backed Rust
global allocator. libc, C++, WebKit/JSC, and Bun APIs may use different
allocation domains. Pointer compatibility does not imply allocator ownership
compatibility.

- Every FFI buffer must have an explicit allocator owner and matching free
  function.
- Never release libc/glibc-owned memory through Rust's global allocator, Bun,
  mimalloc, or a C++ `delete` operator.
- On Linux, do not call `std::fs::canonicalize` from TSP production code. Its
  `realpath(path, NULL)` implementation can return glibc-owned memory.
- Use `bun/src/runtime/tsp/path.rs` and `crate::path::canonicalize` for TSP
  path resolution. It supplies a caller-owned buffer and copies the result.
- Prefer caller-owned FFI output buffers. If foreign memory must cross a
  boundary, copy it into memory owned by the receiving allocator before
  storing it in an owning Rust or C++ type.
- Native-runtime comments and wrappers must document allocation ownership when
  the boundary is not obvious.

VM roles are defined relative to the current operating-system process:

- A TSP worker child owns the only JSC VM in its process, so it is that
  process's main VM even though the TSP master treats the process as a worker.
- Reserve `VmRole::WebWorker` for Bun's in-process WebWorker implementation. A
  WebWorker role requires both a non-null `WorkerMessagingProxy` and a concrete
  script-execution-context id greater than 1.
- Do not infer a VM role from process-pool terminology, a null worker pointer,
  or `vm.worker.is_none()` independently. Main-thread publication, context id,
  and worker ownership must come from one `VmRole` value.

An embedded VM has a separate module-readiness phase after low-level VM
construction:

- Before calling `load_entry_point`, wire the transpiler's resolver env loader,
  run `configure_defines`, load the runtime environment, and install the
  per-thread source-code printer. Follow the lifecycle shared by Bun's CLI and
  WebWorker paths; a successful `VirtualMachine::init` or Hello/Ready handshake
  does not prove module readiness.
- `VirtualMachine::set_main` stores a borrowed slice. The backing entry-path
  storage must remain valid for the VM lifetime. IPC request strings and other
  per-request temporaries must be copied into VM-owned storage first.
- Embedded-worker tests must execute at least one generated module; a startup-
  only handshake test cannot validate the module-loading lifecycle.

Windows handle boundaries have a separate representation rule:

- `bun_core::Fd` is a packed value on Windows; its high bit distinguishes a
  system `HANDLE` from a libuv file descriptor.
- Opaque Rust/C interfaces that carry an `Fd` must store and restore the packed
  `Fd` value. Never serialize `Fd::native()` and then reconstruct it as an
  `Fd`, because `INVALID_HANDLE_VALUE` and the kind bit can be misclassified.
- Treat `Fd::INVALID` as a sentinel. Output, close, and adapter paths must
  short-circuit it before invoking a platform API.
- Worker smoke tests must use the same redirected/non-interactive stdio shape
  as CI; an interactive terminal is not equivalent to a Windows runner.
- Do not infer a native root cause from a fault address alone. Require a
  symbolicated frame or a reproducible startup-stage boundary before changing
  allocator, JSC, or VM lifecycle behavior.
- Keep worker startup diagnostics environment-gated; use
  `TSP_WORKER_STARTUP_TRACE=1` when a Windows CI crash must be split between
  JSC initialization, VM creation, and protocol handshake. The trace callback
  must be stashed on the VM (or another per-thread slot) so per-request VM
  methods (`reload_entry_point`, `load_entry_point`, future entry-point
  work) can also emit stage markers — otherwise a crash inside those
  methods shows up as one opaque outer marker pair with no internal
  boundary.

See `docs/v2/adr/0002-cross-allocator-ownership.md`,
`docs/v2/adr/0003-windows-fd-representation.md`,
`docs/v2/adr/0004-process-relative-vm-roles.md`, and
`docs/v2/adr/0005-embedded-vm-module-readiness.md` for the rationale and
boundary rules. See `docs/v2/bugs/0002-mimalloc-operator-delete-sigsegv.md`
and `docs/v2/bugs/0003-windows-ci-worker-sigsegv-invalid-handle.md` for the
verified regressions.

## Verification

Before changing the native runtime, run the focused Rust tests and the smoke
test. Changes to route discovery, generation, workers, or response handling
must preserve the frozen contract in `docs/v2/FREEZE.md`.

## Embedded Bun API boundaries

- Generated `tsp:server` namespaces must not eagerly read optional Bun native
  properties or call `require("bun")` during synthetic module setup.
- Expose optional Bun APIs through lazy getters so a route that does not use an
  API cannot initialize its native subsystem as a side effect of importing
  `tsp:server`.
- Keep the lazy boundary when adding new native helpers, and add a wrapper test
  that proves both the exported shape and the absence of eager lookup.
- On Windows, TSP worker stdout may be discarded by the worker manager. Native
  diagnostics must use inherited stderr or an explicit diagnostic sink rather
  than assuming `console.log` is captured.
- Embedded-worker generated code must not access standard-input handles that
  the worker manager redirects to null. The old subprocess stdin abort marker
  is not an embedded-worker control channel; use the native worker protocol.

Changes to path handling, FFI, allocators, or embedded workers must also run a
Linux embedded-worker release build and the TSP v2 smoke test.
