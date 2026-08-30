# Changelog

All notable changes to TSP will be documented in this file.

## [Unreleased]

### Added
- v2.4 Master + IPC embedded Bun Workers with cross-platform lifecycle,
  timeout, crash replacement, backpressure, and hot-reload coverage.
- Native v2 build, package, benchmark, CI, release, and smoke-test workflows.
- v2 route fixtures covering methods, dynamic parameters, cookies, sessions,
  request bodies, cancellation, static assets, and response handling.
- v2-only root workflow. The former TypeScript/Bun application host and its
  compatibility surface are no longer shipped.
- `config.bodyLimit` per-page request body cap (FREEZE.md §11). POST / PUT /
  PATCH / DELETE with body over the per-page cap return 413; the cap is
  silently clamped to the global `TSP_MAX_BODY_BYTES`. Hand-rolled parser
  supports `int`, `int * int * ...`, and underscore separators.
- `config.cache` per-page default `Cache-Control` header (plan §55, FREEZE.md
  §11). The runtime applies the value as a default header; the page's own
  `Response.headers` Cache-Control always wins.
- `config.timeoutMs` per-page request timeout (spec §7 v2.0 core PageConfig).
  Overrides the global `TSP_TIMEOUT_MS` per request; `0` disables the watchdog.
- `tspserver_v2 check` reports three new categories of spec §46 export-
  validation violations at check time (the runtime still serves the page;
  full generation-build enforcement lands with the AST detector in a
  future slice):
  - `config.methods` mismatch (declared set vs. actual exports)
  - unknown runtime exports (`export function NAME(...)` whose NAME is
    not a standard HTTP method handler)
  - `export default` violation (top-level default export is silently
    ignored at runtime; `check` surfaces it)
- `TSP3001: handler returned unsupported value <Type>. Expected HtmlNode or
  Response.` typed error for invalid handler return values (spec §6.3 / plan
  §10.4 / FREEZE item 5). The wrap-side helper distinguishes top-level
  contract violations (`TSP3001`) from nested JSX-child errors
  (`TSP3102`) via the `__child__` flag.
- `tspserver_v2 --version` flag and a fully-documented `--help` output
  that lists every env var the host honors (including the previously
  undocumented `TSP_TIMEOUT_MS` and `TSP_DEVELOPMENT`).

### Fixed
- Pre-existing port collision between `multi_route_dispatch_does_not_alias_to_first_request`
  and `metrics_endpoint_serves_prometheus_text_after_priming_requests` (both
  baselined on `30_000 + pid%500/1000`); the multi-route test now uses a
  unique `43_000 + pid%500` range.
- TCP teardown race in the 413 path: the host now drains the body off the
  socket AFTER writing the 413 response, so a client sending a body larger
  than the kernel buffer no longer sees `ConnectionReset`. The drain is
  bounded by `max_bytes`, a 1-second per-read timeout, and a 5-second
  total wall-clock budget. The body-limit e2e's 1.5 MiB scenario
  (per-page cap > global) is now pinned end-to-end.
- `wait_for_marker` regression: a previous "TCP-buffer" fix had removed
  the `child.stderr = Some(stderr);` put-back, breaking 11+ tests that
  call `wait_for_marker` twice on the same child (boot + hot-reload).
  The put-back is restored.
- `http_send_raw` test helper is now tolerant of a trailing
  `ConnectionReset` on the LAST read (the response body has already been
  received; the RST just terminates the stream). The tolerance is
  restricted to AFTER the first successful read.

## Unreleased — 2026-08-30

### Added
- `VirtualMachine::vm_trace` helper plus a per-VM `startup_trace` slot
  (`bun/src/jsc/VirtualMachine.rs`) so embedding startup-trace callbacks
  remain available to per-request VM methods, not just `init`. The slot
  is a `fn(&str)` pointer (no captures, no VM/JSC work) populated once
  by `init` from `InitOptions::startup_trace` and read by `vm_trace`.
- Segmented stage markers inside `VirtualMachine::reload_entry_point` /
  `load_entry_point` (`entry-eval:begin`, `:set-main:end`,
  `:debugger:end`, `:pre-exec:begin/end`, `:generate-entry:begin/end`,
  `:preloads:begin/end`, `:module-loader:begin/end`, `:end`,
  `load-entry:reload-end`, `:wait:begin`, `:wait:rejected`, `:wait:end`)
  so a Windows first-call crash inside `load_entry_point` is
  attributable to one of: synthetic `bun:main` generation, pre-execution
  bootstrap, preload evaluation, `JSModuleLoader` evaluation, or
  promise resolution + event-loop tick.
- Per-iteration markers inside `EventLoop::wait_for_promise`
  (`bun/src/jsc/event_loop.rs`): `load-entry:wait:iter:0` /
  `:iter:N`, `load-entry:wait:tick:begin` / `:tick:end`,
  `load-entry:wait:auto-tick:begin` / `:auto-tick:end`, and
  `load-entry:wait:resolved`. Rate-limited to the first four iterations
  so a successful wait does not flood the trace. The Windows first-call
  crash was bounded to this function by the previous slice's trace; the
  new markers split `tick` (microtask drain) from `auto_tick`
  (runtime hook → uSockets poll → Windows HANDLE) so the next run can
  attribute the fault to one of those.

### Diagnostics
- BUG-0003 (`docs/v2/bugs/0003-windows-ci-worker-sigsegv-invalid-handle.md`)
  now records that the crash remains inside the C++ microtask-drain entry point
  after module evaluation. Removing the obsolete generated-worker stdin
  listener in `d30992b8b6` did not move the Windows CI boundary, so that theory
  is rejected as the root cause. Lazy optional-Bun-API access remains
  independent hardening.
- Added native stderr markers around the `JSNextTickQueue` and
  `JSC::VM::drainMicrotasks()` sub-stages. BUG-0003 remains open pending the
  next Windows CI result.

### Fixed
- Windows embedded-worker SIGSEGV during the first module checkpoint: the
  WebKit/JSC pin now uses `b9a6abf2d598`, which contains WebKit's
  `MicrotaskCallCache` invalidation when detached `CodeBlock` objects are
  deleted. The candidate passes the local Windows embedded-worker smoke test;
  Windows CI confirmation is pending.
- ~~Windows first-call SIGSEGV in the TSP embedded worker at
  `0xFFFFFFFFFFFFFFFF`. The `VirtualMachine::init` call leaves
  `uws::Loop::internal_loop_data.jsc_vm` non-null, so the JSC park hook
  (`Bun__JSC_onBeforeWait`) fires from `us_loop_run` /
  `us_loop_run_bun_tick` and — Windows + mimalloc — drives
  `mi_on_thread_idle()` against an unvalidated retired-page list. The
  worker now clears `jsc_vm` after init (`bun/src/runtime/tsp_worker.rs`),
  which short-circuits the `if (loop->data.jsc_vm)` guard in uSockets and
  skips the park hook. The TSP worker is the JS thread for its process
  and never re-enters from another thread, so the per-poll heap-access
  release is unnecessary. Windows CI is expected to pass the first
  request, repeated requests, and hot reload.~~ Reverted: the
  Windows CI run on `61b0fdf9f4` showed the crash is in
  `EventLoop::tick`, not `auto_tick`; on a fresh worker
  `is_active()=false`, so `auto_tick` takes the `else` branch
  (`tick_without_idle()` → `us_loop_pump()`) which does not invoke the
  park hook. The `jsc_vm=null` change is therefore a no-op for the
  crash and was reverted.

### Diagnostics
- Added `tick_turn` sub-stage markers (`bun/src/jsc/event_loop.rs`):
  `tick:concurrent:initial:begin/end`, `tick:gc-timer:initial:begin/end`,
  `tick:inner:tick-with-count:begin/end`, `tick:inner:concurrent:begin/end`,
  `tick:inner:rejected:begin/end`, `tick:microtasks:begin/end`,
  `tick:tail:tick-with-count:begin/end`, `tick:tail:concurrent:begin/end`,
  `tick:rejected:final:begin`. Per-iteration markers inside the inner
  and tail loops are rate-limited to the first iteration (the Windows
  first-call crash is deterministic on iteration 1). The next failing
  CI run will show which sub-stage's `:end` marker is the first one
  not to print, localising the fault to a specific `tick_turn` step
  (most likely `tick:microtasks:begin/end` for the
  `drain_microtasks_with_global` JSC microtask drain where the
  synthetic `bun:main` body runs).
- Added `drain_microtasks_with_global` sub-stage markers
  (`bun/src/jsc/event_loop.rs`): `drain-mt:release-weak-refs:begin/end`,
  `drain-mt:drain-microtasks:begin/end`,
  `drain-mt:deferred-tasks:begin/end`, `drain-mt:quic:begin/end`. The
  Windows CI run on `ab1c2f2e6` showed `tick:microtasks:begin`
  printed but `tick:microtasks:end` did not, narrowing the fault to
  this function. The four markers split the 4-step drain to identify
  the failing FFI or Rust call.
- The temporary `TSP_PRELUDE_ENTERED` stdout tripwire was removed. The Windows
  worker manager discards worker stdout, so native diagnostics use inherited
  stderr or an explicit sink.

### Test count
332 tests, all green on 5 consecutive full-suite runs.
Breakdown: 267 lib + 4 worker_integration + 15 process_model + 38
start_order e2e + 8 tsp_worker_test_stub.

## [0.1.0] - 2026-03-02

### Added
- Initial release
- TSP (TypeScript Server Page) template server using Deno + TSX + React
- Direct `.tsp` file execution (like PHP)
- Intelligent module caching with hot reload support
- Type-safe dependency injection system
- Built-in file manager with password protection
- MySQL Schema-first API with Zod validation
- Redis client support
- LDAP client support
- ExcelJS integration for Excel file operations
- Session management
- Cookie management
- Static file serving with caching
- Configuration auto-reload

### Features
- `.tsp` file suffix as route files
- Global type declarations (no imports needed)
- Schema-first data validation
- Hot reload via Deno's watch mode
- Cross-platform compilation
