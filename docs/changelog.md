# Changelog

All notable changes to TSP will be documented in this file.

## [Unreleased]

## [0.3.1] - 2026-08-31

### Fixed
- stabilized embedded worker recovery after a worker disconnects, while
  avoiding automatic replay of non-idempotent requests.
- disabled the JSC/Bun source caches that could terminate an embedded worker
  during repeated route compilation.

### Changed

- `pages/` is now the canonical application source and package directory.
- Windows release assets are ZIP archives, and all release asset filenames include the release tag.
- removed the temporary architecture version from the TSP CLI, documentation,
  CI, package layout, type declarations, and release asset names.
- removed obsolete legacy architecture notes and the old development progress
  log.

## [0.3.0] - 2026-08-31

### Added
- embedded-worker Master + IPC embedded Bun Workers with cross-platform lifecycle,
  timeout, crash replacement, backpressure, and hot-reload coverage.
- Native current build, package, benchmark, CI, release, and smoke-test workflows.
- route fixtures covering methods, dynamic parameters, cookies, sessions,
  request bodies, cancellation, static assets, and response handling.
- current root workflow. The former TypeScript/Bun application host and its
  compatibility surface are no longer shipped.
- `config.bodyLimit` per-page request body cap (`contract.md` §11). POST / PUT /
  PATCH / DELETE with body over the per-page cap return 413; the cap is
  silently clamped to the global `TSP_MAX_BODY_BYTES`. Hand-rolled parser
  supports `int`, `int * int * ...`, and underscore separators.
- `config.cache` per-page default `Cache-Control` header (plan §55, `contract.md`
  §11). The runtime applies the value as a default header; the page's own
  `Response.headers` Cache-Control always wins.
- `config.timeoutMs` per-page request timeout (spec §7 current contract PageConfig).
  Overrides the global `TSP_TIMEOUT_MS` per request; `0` disables the watchdog.
- `config.methods` static validation (contract §11): `tspserver check`
  reports three new categories of spec §46 export-validation violations at
  check time (the runtime still serves the page; full generation-build
  enforcement lands with the AST detector in a future slice):
  - `config.methods` mismatch (declared set vs. actual exports)
  - unknown runtime exports (`export function NAME(...)` whose NAME is
    not a standard HTTP method handler)
  - `export default` violation (top-level default export is silently
    ignored at runtime; `check` surfaces it)
- `TSP3001: handler returned unsupported value <Type>. Expected HtmlNode or
  Response.` typed error for invalid handler return values (spec §6.3 / plan
  §10.4 / contract item 5). The wrap-side helper distinguishes top-level
  contract violations (`TSP3001`) from nested JSX-child errors
  (`TSP3102`) via the `__child__` flag.
- `tspserver --version` flag and a fully-documented `--help` output
  that lists every env var the host honors (including the previously
  undocumented `TSP_TIMEOUT_MS` and `TSP_DEVELOPMENT`).
- `tspserver typings` subcommand (with `tsp.sh typings` and
  `TSP_TYPINGS_DIR` / `--out <DIR>` overrides) emits the typed
  declaration files for `tsp:server`, `tsp:html`, and `tsp:runtime`
  (contract item 7, plan §11).
- `tspserver check --tsc` runs a real `tsc --noEmit` against the
  application routes so type errors are surfaced as part of `check`,
  not only as runtime failures.
- `/__tsp/metrics` endpoint pinned end-to-end (Amendment 10): HEAD and
  405 paths covered; `nosniff` added to metrics responses.
- Per-page `config.methods` HEAD body-drop + OPTIONS 204 responses
  preserve the frozen host contract.
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
  so a successful wait does not flood the trace.
- `tick_turn` sub-stage markers (`bun/src/jsc/event_loop.rs`):
  `tick:concurrent:initial:begin/end`, `tick:gc-timer:initial:begin/end`,
  `tick:inner:tick-with-count:begin/end`, `tick:inner:concurrent:begin/end`,
  `tick:inner:rejected:begin/end`, `tick:microtasks:begin/end`,
  `tick:tail:tick-with-count:begin/end`, `tick:tail:concurrent:begin/end`,
  `tick:rejected:final:begin`. Per-iteration markers inside the inner
  and tail loops are rate-limited to the first iteration.
- `drain_microtasks_with_global` sub-stage markers
  (`bun/src/jsc/event_loop.rs`): `drain-mt:release-weak-refs:begin/end`,
  `drain-mt:drain-microtasks:begin/end`,
  `drain-mt:deferred-tasks:begin/end`, `drain-mt:quic:begin/end`.

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
- BUG-0003 (`docs/reference/bugs/0003-windows-ci-worker-sigsegv-invalid-handle.md`):
  the embedded worker's synthetic ESM `bun:main` evaluation left a
  module-resume job that crashed in the first Windows JSC microtask
  checkpoint. The final worker path directly transpiles and evaluates
  the wrapper body as a plain script, avoiding both that resume job and
  the failed CommonJS module-builder workaround. Local Windows smoke
  and [CI run `33340458504`](https://github.com/risol/tsp/actions/runs/33340458504)
  pass on Linux, macOS, and Windows.
- Embedded worker no longer installs a generated-worker stdin listener,
  and the optional Bun API surface is exposed through lazy getters so
  importing `tsp:server` does not initialize a native subsystem as a
  side effect.
- Reverted the `jsc_vm=null` change for the TSP embedded worker: the
  Windows CI run on `61b0fdf9f4` showed the crash is in
  `EventLoop::tick`, not `auto_tick`; on a fresh worker
  `is_active()=false`, so `auto_tick` takes the `else` branch
  (`tick_without_idle()` → `us_loop_pump()`) which does not invoke the
  park hook. The `jsc_vm=null` change was a no-op for the crash.

### Diagnostics
- BUG-0003 confirmed root cause recorded in
  `docs/reference/bugs/0003-windows-ci-worker-sigsegv-invalid-handle.md`. The
  Windows worker crash is no longer opaque: the segmented trace
  narrows the fault to a single sub-stage of `drain_microtasks_with_global`
  before the direct-transpile fix.
- The temporary `TSP_PRELUDE_ENTERED` stdout tripwire was removed. The
  Windows worker manager discards worker stdout, so native diagnostics
  use inherited stderr or an explicit sink.

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
