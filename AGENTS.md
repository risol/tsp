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

See `docs/v2/adr/0002-cross-allocator-ownership.md` and
`docs/v2/bugs/0002-mimalloc-operator-delete-sigsegv.md` for the rationale and
the verified regression.

## Verification

Before changing the native runtime, run the focused Rust tests and the smoke
test. Changes to route discovery, generation, workers, or response handling
must preserve the frozen contract in `docs/v2/FREEZE.md`.

Changes to path handling, FFI, allocators, or embedded workers must also run a
Linux embedded-worker release build and the TSP v2 smoke test.
