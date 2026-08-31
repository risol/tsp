# BUG-0002: Linux SIGSEGV while freeing `canonicalize` output

> Status: **Resolved in TSP**
> Discovered: 2026-08-28
> Resolved: 2026-08-29
> Affected: Linux x64 Bun builds embedding the TSP runtime
> Severity: Blocker during TSP startup or the first request

## Summary

The original report incorrectly classified this as an upstream mimalloc
bucket-initialisation bug. The actual failure is an allocator ownership
violation at the TSP/Bun/glibc boundary.

TSP called `std::fs::canonicalize`. On Linux, Rust's implementation uses
`realpath(path, NULL)`, which returns a buffer allocated by glibc. Bun replaces
Rust's global allocator with mimalloc. When the resulting `PathBuf` was later
dropped, its buffer was released through Bun's mimalloc-backed Rust allocator,
even though glibc owned the allocation. The free path then crashed inside
mimalloc's `operator delete[]` (`_ZdaPv`).

This is therefore:

- a TSP bug because TSP used a standard-library API whose allocation ownership
  is incompatible with the embedded Bun allocator;
- an allocator-boundary integration hazard in Bun because foreign libc-owned
  memory is allowed to reach the Rust global deallocator;
- not evidence that mimalloc's page-map or bucket implementation is broken.

The fix belongs in TSP's path handling: provide a caller-owned output buffer to
`realpath` and never let a libc-owned allocation cross into Rust/mimalloc.

## Original symptom

The failure was reproducible in the Linux build container by starting the
embedded worker binary under the required `tspserver` basename and sending
the first request:

```bash
cd /src
cp bun/build/release/bun-profile /tmp/tspserver
cp -R tests/smoke/routes /tmp/tspserver-routes

TSP_PORT=9215 \
TSP_ROUTES_DIR=/tmp/tspserver-routes \
TSP_EMBEDDED_WORKER=1 \
TSP_WORKER_COUNT=2 \
/tmp/tspserver
```

The server reached its listening state and then crashed while loading or
handling the first route request. The fault address varied with ASLR, but the
crash consistently landed in mimalloc's `_ZdaPv`.

## Evidence for the root cause

The relevant call chain is:

```text
TSP ModuleGraph / static-file / watcher path handling
  -> std::fs::canonicalize
  -> std::sys::fs::unix::canonicalize
  -> realpath(path, NULL)
  -> glibc-owned heap buffer
  -> PathBuf drop
  -> Bun Rust global allocator
  -> _ZdaPv / mi_free
  -> SIGSEGV
```

The investigation established all of the following:

1. `ModuleGraph::from_routes_dir` was on the startup path. TSP also used
   `canonicalize` in JSX import resolution, static-file checks, and watcher
   bookkeeping.
2. Rust's Linux `std::fs::canonicalize` calls the `realpath(path, NULL)` form.
   The null output argument instructs libc to allocate the result.
3. The returned pointer was in the ordinary glibc `[heap]` mapping, not in a
   mimalloc-owned mapping.
4. Bun's allocator override forwards Rust/C++ delete operations to mimalloc;
   the relevant override maps `_ZdaPv` to `mi_free`.
5. The crash occurred when mimalloc interpreted the glibc pointer as one of
   its own allocations. The register state and disassembly showed the fault
   before a valid mimalloc page entry could be read.

This explains why the crash appeared in mimalloc even though the invalid
ownership decision happened earlier in the TSP path layer.

## Fix

TSP now has a single internal path helper:

```text
bun/src/runtime/tsp/path.rs
```

On Linux it:

1. creates the input C string with Rust-owned storage;
2. allocates a `PATH_MAX` output buffer in Rust/Bun-owned memory;
3. calls `realpath(input, output_buffer)` with a non-null output pointer;
4. copies the resolved bytes into the returned `PathBuf` before the buffer is
   dropped.

The production call sites now use this helper in:

- `module_graph.rs`;
- `static_files.rs`;
- `watcher.rs`;
- `jsx.rs`.

Non-Linux platforms retain the standard-library implementation because this
incident is specific to the Linux Bun allocator configuration.

The implementation intentionally does not import Bun's internal `bun_core` or
`bun_sys` helpers. TSP must remain independently compilable, and the ownership
guarantee is simple enough to enforce locally with a caller-owned buffer.

## What was not changed

- No mimalloc page-map or bucket code was patched.
- No allocator switch was introduced.
- No Linux smoke test was skipped.
- The TSP route contract and response protocol were unchanged.

Changing mimalloc would have treated the symptom at the wrong layer and would
not have made the glibc/mimalloc ownership violation valid.

## Verification

The fix was verified in both standalone and embedded forms:

```text
Windows standalone TSP tests: 267 passed
Linux standalone TSP tests:   268 passed
Linux Bun release build:      passed
Embedded-worker smoke:        passed
HTTP route handling:          passed
Hot reload smoke:             passed
```

The embedded smoke used a rebuilt Linux Bun binary copied to the exact
`tspserver` basename. This is important because the Bun multi-call binary
selects TSP mode from its executable name.

## Regression-prevention rules

The durable rules are recorded in:

- `AGENTS.md`, under **Native runtime and allocator boundaries**;
- `docs/reference/adr/0002-cross-allocator-ownership.md`.

The short version is:

1. Do not use Linux `std::fs::canonicalize` in TSP production code embedded in
   Bun.
2. Use `crate::path::canonicalize` for TSP path resolution.
3. Never pass libc-owned memory to Rust, C++, Bun, or mimalloc deallocation.
4. At every FFI boundary, document who allocates, who owns, and who frees the
   buffer.
5. Changes to path handling, allocators, workers, or embedding must run both
   the focused TSP tests and the Linux embedded-worker smoke test.

## Related files

- `bun/src/runtime/tsp/path.rs` — allocator-safe path resolution;
- `bun/src/runtime/tsp/module_graph.rs` — route/module graph call sites;
- `bun/src/runtime/tsp/static_files.rs` — static-file path call sites;
- `bun/src/runtime/tsp/watcher.rs` — watcher path call sites;
- `bun/src/runtime/tsp/jsx.rs` — local import/file URL call sites;
- `scripts/smoke-tspserver.sh` — embedded-worker regression test;
- `docs/reference/adr/0002-cross-allocator-ownership.md` — permanent design rule.
