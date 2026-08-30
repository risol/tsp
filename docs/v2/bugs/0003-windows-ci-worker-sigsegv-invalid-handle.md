# BUG-0003: Windows embedded worker SIGSEGV during first module evaluation

> Status: **Fixed and verified**
> Discovered: 2026-08-29 in GitHub Actions `smoke-windows`
> Latest failure: [Windows CI run `33323201126`](https://github.com/risol/tsp/actions/runs/33323201126)
> Verification: [Windows embedded-worker job in CI run `33340458504`](https://github.com/risol/tsp/actions/runs/33340458504/job/99335137362) passed
> Affected: TSP v2 embedded-worker request execution on Windows
> Severity: CI blocker

## Summary

The TSP master starts and listens. Each Windows worker completes native VM
initialization and the Hello/Ready handshake, then crashes while executing the
first generated route module. The master subsequently observes Winsock error
10054 and retries with a replacement worker.

The complete trace establishes this boundary:

```text
master startup: passed
TCP accept: passed
worker native startup: passed
TSP Hello/Ready protocol: passed
first generated module execution: failed
10054: consequence of worker termination
```

The failure is at the Bun/JSC engine boundary, not in the TSP protocol, route
discovery, TCP, or generic JSC startup. The worker reaches the first event-loop
turn and crashes inside `JSC::VM::drainMicrotasks()`.

## Evidence and root cause

The `56bd752049` run passed checkout, compilation, packaging, JSC VM creation,
global-object creation, stack-check setup, and the worker handshake. Every
worker reached `handshake:ready-sent`. The crash occurred only after the master
sent the first generated `index.tsp` request, and every replacement worker
produced the same crash report.

Code comparison with both Bun's process-main CLI and in-process WebWorker paths
found that TSP initially skipped the required post-`VirtualMachine::init`
runtime configuration. That lifecycle defect was fixed: the resolver, defines,
environment state, source printer, process-main VM role, and VM-owned entry path
are now prepared before Ready and module loading. Later CI traces passed all of
those stages and still crashed in the first event-loop turn, so that defect was
real but not the remaining root cause.

The stdin-listener theory was tested in `d30992b8b6`: the generated
`process.stdin?.on('data', ...)` listener was removed, but the Windows CI run
still crashed at the identical `drain-microtasks:begin` boundary. Therefore it
is not the root cause. Removing that obsolete listener remains valid embedded
worker hygiene, but it did not fix BUG-0003.

The earlier lazy-optional-Bun-API change remains valid hardening, but it is not
the root cause of this incident: the corresponding Windows CI run still
crashed at the same `drain-microtasks` boundary after that change.

TSP also passed an IPC-request-owned path to `VirtualMachine::set_main`, which
stores a borrowed slice under a process-lifetime invariant. The slice became
dangling when the request was dropped and could be observed by the next
`clear_entry_point()` call. `EmbeddedVm` now owns reusable path storage and
loads every entry from that stable buffer.

The crash URL encoded 20 executable-relative frames, but bun.report could not
symbolicate the custom build because its debug information was not published.
Mapping those offsets against the local same-commit PDB placed frames in JSC
module loading/evaluation and the TSP worker request path. Because separately
linked binaries can shift individual offsets, that mapping is supporting
evidence rather than the sole root-cause proof. The stage trace and missing
lifecycle calls provide the reproducible boundary.

The WebKit `MicrotaskCallCache` hypothesis was tested and disproven. TSP was
updated from WebKit `0f966e81b78c` to
`b9a6abf2d59854e9004155d156b3b751b638d0ec`, which contains the upstream cache
invalidation change, but [Windows CI run `33319229316`](https://github.com/risol/tsp/actions/runs/33319229316)
still crashed at the same `JSC__JSGlobalObject__drainMicrotasks` boundary.
The newer engine pin is still the correct dependency update, but it is not the
BUG-0003 fix.

The candidate in commit `29761b6191` was rejected by Windows CI. Its explicit
`run_gc(false)` call crashed before the first tick while the module-evaluation
promise was still pending (`load-entry:pre-tick:gc:begin` was the last marker).
This also proves that manual GC is not a valid preparation step for this
state.

The root cause is now confirmed: the embedded worker used
`VirtualMachine::reload_entry_point()` to evaluate a synthetic `bun:main`
ESM module for every request. Even a synchronous route therefore left a
module-evaluation resume job in JSC. The first microtask checkpoint entered
that job and, on the Windows embedded-runtime path, dereferenced an invalid
internal handle (`0xFFFFFFFFFFFFFFFF`) and terminated the worker. Winsock
10054 was only the master's observation of that worker termination.

The final fix keeps the generated wrapper on the request path but does not
load it through the module registry. The embedded worker calls the canonical
Rust transpiler directly with the TSX loader, unwraps the printer's CommonJS
function shell without invoking the module builder, and evaluates the body as
a plain script inside an IIFE. Synchronous handlers publish their envelope
before any microtask checkpoint; asynchronous handlers still use the normal
event-loop fallback. This removes both the crashing ESM resume job and the
failed `require(<temp>.cts)` workaround.

## Independent defects found during investigation

### Packed Windows `Fd` representation

`bun/src/sys/lib.rs` now preserves the packed Windows `Fd` representation in
the opaque `QuietWriter` slot and short-circuits `Fd::INVALID`. The round-trip
test passes. This is required hardening, but the later failing runs prove it is
not the root cause of BUG-0003.

### Process-relative VM role

A TSP child is a worker in the TSP pool but owns the only JSC VM in its
operating-system process. It must use the process-main VM role. The former
`is_main_thread: false`, null `worker_ptr`, and absent `context_id` combination
selected the auxiliary/macro context sentinel and gave the same VM conflicting
identities across Rust and C++.

Initialization now uses one validated `VmRole`; the TSP child uses
`VmRole::ProcessMain`, while only Bun's in-process WebWorker path uses
`VmRole::WebWorker`. The `56bd752049` run completed VM initialization and the
handshake with this correction, then still crashed on the first request. The
role fix is correct but was not sufficient to explain BUG-0003.

### Disabled JSC background work

Disabling concurrent JIT compilation and parallel GC marking did not move the
failure boundary and was reverted. Retaining that change would alter runtime
semantics without fixing the lifecycle defect.

## Fix status

The dependency update changes `bun/scripts/build/deps/webkit.ts` to WebKit
`b9a6abf2d59854e9004155d156b3b751b638d0ec`, whose Windows prebuilt contains the
upstream `MicrotaskCallCache` fix. The build also makes two source-compatibility
adjustments required by that newer WebKit API: an explicit `HexNumber.h`
include and the new module-loader `referrer` parameter. These are valid
dependency/build changes, but CI proved they do not close BUG-0003.

With the new WebKit cache extracted at
`C:\Users\user\.bun\build-cache\webkit-b9a6abf2d59854e9`, the local Windows
smoke test passed the first request, repeated requests, and hot reload. The
final embedded-worker execution change passed the GitHub Windows
embedded-worker job in CI run `33340458504`.

The embedded worker now completes the same module-readiness phase as Bun's
other module-loading VM entry points:

1. Construct the process-main VM.
2. Connect the Transpiler resolver to its env loader.
3. Select normal Bun runtime env behavior and run `configure_defines()`.
4. Load env-derived HTTP/runtime state.
5. Install the per-thread source-code printer.
6. Send Ready only after all preparation succeeds.
7. Copy each request's entry path into VM-owned storage before loading it.

Failures from `configure_defines()` are returned as worker initialization
errors instead of continuing with a partially configured VM.

The wrapper also contains an independent hardening fix:

1. `__tspUtilNs__` is created with accessor properties. Each optional
   `Bun.*` value is read only when the page requests that property.
2. `tsp:server.sql` is installed as an accessor. `require("bun").SQL` is not
   evaluated for routes that do not use SQL.
3. The namespace remains frozen after its lazy descriptors are installed, so
   the public shape and mutation guarantees are unchanged.

These preserve the frozen API and reduce unrelated native work during embedded
worker startup. They do not fix BUG-0003. In particular, the WebKit cache
invalidation is insufficient; do not record it as the root cause until a
Windows CI run proves that the first embedded-worker request survives the JSC
microtask checkpoint.

### Investigated but rejected

Step 8 (Windows JSC park hook disable) was attempted in `61b0fdf9f4` and
reverted in the next slice. The fix cleared `uws::Loop::internal_loop_data.jsc_vm`
to short-circuit the `if (loop->data.jsc_vm)` guard in `us_loop_run` /
`us_loop_run_bun_tick` (which calls `Bun__JSC_onBeforeWait` → on Windows
`mi_on_thread_idle()`). It was a no-op because the Windows first-call crash
is in `EventLoop::tick`, not `EventLoop::auto_tick`: on a fresh worker
`is_active()=false`, so `auto_tick` takes the `else` branch and calls
`tick_without_idle()` → `us_loop_pump()`, which does not invoke the park
hook. The next trace slice (`tick_turn` sub-stages) was added to localise
the real fault site.

## Diagnostics

The Windows smoke job sets `TSP_WORKER_STARTUP_TRACE=1`. The trace is disabled
in normal execution and now covers construction, request entry, the
`reload_entry_point` / `load_entry_point` call chain, and per-iteration
`EventLoop::wait_for_promise` sub-stages. Embedding startup trace storage
on the VM lets per-request VM methods emit the same stage markers as the
worker request handler and the event loop.

```text
tcp-connect:begin/end
jsc-initialize:begin/end
ast-store:end
log-init:end
virtual-machine-init:begin/end
vm-core:mark-binding:end
vm-core:allocation:end
vm-core:console:end
vm-core:fields:end
vm-core:runtime-state:end
vm-core:event-loop-waker:end
vm-core:global-object:begin/end
jsc-vm-create:begin/end
jsc-lock:end
client-data:end
global-create:begin/end
global-publish:end
zig-global-object-create:end
vm-core:stack-check:end
runtime-config:begin
runtime-config:defines:end
runtime-config:end
handshake:read-hello
handshake:hello-received
handshake:ready-sent
request:api-lock:begin/acquired/end
request:clear-entry:begin/end
request:load-entry:begin
  entry-eval:begin
  entry-eval:set-main:end
  entry-eval:debugger:end
  (entry-eval:pre-exec:begin / entry-eval:pre-exec:end, only if --trace-* / --stack-trace-limit is set)
  entry-eval:generate-entry:begin
  entry-eval:generate-entry:end
  (entry-eval:preloads:begin / entry-eval:preloads:early-return | entry-eval:preloads:end, only if preload modules exist)
  entry-eval:module-loader:begin
  entry-eval:module-loader:end
  entry-eval:end
load-entry:reload-end
load-entry:wait:begin
  (load-entry:wait:rejected, only if the entry promise rejected synchronously)
  load-entry:wait:iter:0 / :iter:N          (first 4 iterations only)
  load-entry:wait:tick:begin
  load-entry:wait:tick:end                  (first 4 iterations only)
  (load-entry:wait:auto-tick:begin / :end,  (first 4 iterations only, only if still pending after tick))
  load-entry:wait:resolved                  (printed once when the wait loop exits cleanly)
load-entry:wait:end
  tick:concurrent:initial:begin/end         (inside EventLoop::tick_turn, only on first tick call)
  tick:gc-timer:initial:begin/end
  tick:inner:tick-with-count:begin/end      (only on first inner iteration)
  tick:inner:concurrent:begin/end           (only on first inner iteration)
  tick:inner:rejected:begin/end             (only on first inner iteration)
  tick:microtasks:begin/end
  tick:tail:tick-with-count:begin/end        (only on first tail iteration)
  tick:tail:concurrent:begin/end             (only on first tail iteration)
  tick:rejected:final:begin                 (no :end — the function returns via ?)
  drain-mt:release-weak-refs:begin/end      (inside EventLoop::drain_microtasks_with_global)
  drain-mt:drain-microtasks:begin/end        (JSC__JSGlobalObject__drainMicrotasks FFI; this is
                                            where the synthetic bun:main body's microtasks run)
  drain-mt:deferred-tasks:begin/end          (deferred_tasks.run)
  drain-mt:quic:begin/end                    (drain_quic_if_necessary; only on Windows-with-QUIC)
request:load-entry:end
request:response-ready or request:error-ready
```

The Windows worker manager currently starts child workers with stdout discarded
and stderr inherited. Therefore a worker `console.log` is not reliable evidence
in the master smoke log. Native stage markers must be emitted through the
inherited stderr path or another explicit diagnostic sink.

### Localising the fault with the segmented trace

After the lifecycle fix in `fix(tsp): complete embedded VM module setup`
(every `entry-eval:*` marker prints `:end`, every `vm-core:*` prints `:end`),
the Windows first-call crash remained inside `VirtualMachine::load_entry_point`
between `request:load-entry:begin` and `(crash)` with no internal boundary.
The first trace slice split that window into the five phases below; the
Windows CI run on `31488de3e1` showed that phases 1–4 all printed `:end`
cleanly, which moved the failure boundary one level deeper into
`EventLoop::wait_for_promise` (Phase 5) and required the second trace
slice to split it.

The Windows CI run on `7f0527f0f3` showed that `load-entry:wait:iter:0`
and `load-entry:wait:tick:begin` printed but `tick:end` and
`auto-tick:begin` did not, narrowing the crash to `EventLoop::tick`
(Phase 5.1) and excluding `auto_tick` (Phase 5.2). The third trace
slice added `tick_turn` sub-stages to pinpoint the failing micro-step.

The Windows CI run on `ab1c2f2e6` showed `tick:microtasks:begin` printed
but `tick:microtasks:end` did not, narrowing the crash to
`EventLoop::drain_microtasks_with_global`. The fourth trace slice
added `drain-mt:*` sub-stage markers. Its attempted
`console.log('TSP_PRELUDE_ENTERED')` tripwire was invalid as a diagnostic because
the worker manager discards stdout; it has been removed.

Sub-stages after both slices:

1. `entry-eval:generate-entry` — `ServerEntryPoint::generate` (synthetic
   `bun:main` body) or anything the high-tier hook transitively touches
   (transpiler, resolver, file system). Excluded by the latest run: the
   `:end` marker printed.
2. `entry-eval:pre-exec` — the FFI call to `Bun__preExecutionBootstrap` for
   `internal/process/pre_execution` (only when `--trace-*` /
   `--stack-trace-limit` argv is set; not on the normal TSP path).
3. `entry-eval:preloads` — Bun preload modules (only when `preload` is
   non-empty; not on the normal TSP path).
4. `entry-eval:module-loader` — `JSModuleLoader::load_and_evaluate_module_ptr`
   resolver / fetcher / transpiler / JSC module loader callback chain
   synchronously firing before the returned promise settles. Excluded by
   the latest run: the `:end` marker printed, the promise was handed to
   `wait_for_promise` and was still pending.
5. `load-entry:wait:tick` — `EventLoop::tick` (microtask drain; this is
   where the synthetic `bun:main` body runs and writes
   `__tspEmbeddedResponse`). Excluded by the latest run: the
   `tick:begin` marker printed, the `tick:end` did not.
6. `load-entry:wait:auto-tick` — `EventLoop::auto_tick` → runtime hook
   `tick` → `bun_runtime::jsc_hooks::auto_tick` → uSockets poll on
   Windows (IOCP, `WSARecv` / `GetQueuedCompletionStatus`). Excluded
   by the latest run: the crash is inside `tick`, so `auto-tick:begin`
   never prints.
7. `tick_turn` sub-stages (latest slice):
   - `tick:concurrent:initial` — first `tick_concurrent` call
   - `tick:gc-timer:initial` — `process_gc_timer` call
   - `tick:inner:tick-with-count` — first-iter `tick_with_count` (drains
     the task queue)
   - `tick:inner:concurrent` — first-iter follow-up `tick_concurrent`
   - `tick:inner:rejected` — first-iter `handle_rejected_promises`
   - `tick:microtasks` — `drain_microtasks_with_global` (the most likely
     site — synthetic `bun:main` body runs and writes
     `__tspEmbeddedResponse` here)
   - `tick:tail:tick-with-count` / `:concurrent` — tail refill loop
   - `tick:rejected:final` — final `handle_rejected_promises` sweep
8. `drain_microtasks_with_global` sub-stages:
   - `drain-mt:release-weak-refs` — `jsc_vm.release_weak_refs()` C++ FFI
   - `drain-mt:drain-microtasks` — `JSC__JSGlobalObject__drainMicrotasks`
     C++ FFI; this is where the synthetic `bun:main` body microtasks
     actually run
   - `drain-mt:deferred-tasks` — `self.deferred_tasks.run()` (Rust; runs
     queued host tasks)
   - `drain-mt:quic` — `drain_quic_if_necessary` (Rust; uSockets QUIC
     driver; only on Windows-with-QUIC)
9. Native C++ sub-stage markers added after `d30992b8b6`:
   `TSP worker microtasks: next-tick:drain:begin/end` and
   `TSP worker microtasks: jsc:drain:begin/end`. These distinguish a crash in
   Bun's `JSNextTickQueue` path from a crash in `JSC::VM::drainMicrotasks()`.
10. Synthetic wrapper initialization — the failing wrapper eagerly read every
   optional `Bun.*` utility and `require("bun").SQL` before the route needed
   them. The absence of the old stdout tripwire cannot distinguish individual
   getters because worker stdout is discarded. The source-level fix removes
   this eager native work by using lazy accessors.

The crash address `0xFFFFFFFFFFFFFFFF` was not sufficient to identify an
invalid HANDLE: the stdin candidate was disproven and the Bun report for the
custom build was not symbolicated. The decisive failing run printed every
`entry-eval:*` marker, reached `load-entry:wait:tick:begin`, and stopped after
`drain-mt:drain-microtasks:begin`. After upgrading WebKit to the release with
the cache invalidation, the local Windows smoke test passes the same path.

## Validation

The final embedded-worker direct-transpile path passed CI run `33340458504` on
Linux, macOS, and Windows. The Windows smoke test passed the first request,
repeated requests, metrics, and hot reload without a worker crash.

## Regression-prevention rules

1. VM role is relative to the current operating-system process.
2. Reserve `VmRole::WebWorker` for an in-process worker with both a
   `WorkerMessagingProxy` and concrete context id.
3. Keep role, context id, and worker owner in one validated value.
4. Preserve packed `Fd` values across opaque boundaries and short-circuit
   `Fd::INVALID` before platform I/O.
5. Do not assign root cause from a fault address alone; require a native frame
   or reproducible stage boundary.
6. `VirtualMachine::init` is not sufficient preparation for module loading.
   Complete the resolver, defines, env, and source-printer phase first.
7. Any slice stored by `VirtualMachine::set_main` must be VM-owned or otherwise
   remain valid for the VM lifetime.
8. Test embedded workers with redirected/non-interactive stdio, multiple
   workers, generated module execution, repeated requests, and hot reload.
9. The embedding startup trace must be stashed on the VM (or another
   per-thread slot) so per-request VM methods (`reload_entry_point`,
   `load_entry_point`, future entry-point work) can emit stage markers too.
   Without this, a crash inside those methods shows up as one opaque
   `<outer>:begin … (crash)` pair with no internal boundary and no way to
   attribute the fault to a specific sub-stage.
10. Generated framework namespaces must expose optional native Bun APIs lazily.
    A route that does not use `Bun.password`, `Bun.file`, `Bun.SQL`, or another
    optional builtin must not read it while importing `tsp:server`.
11. Do not infer worker execution from stdout in Windows smoke diagnostics.
    `worker/manager.rs` discards worker stdout; use inherited stderr or an
    explicit diagnostic channel.
12. Generated embedded-worker code must not access standard-input handles that
    the worker manager redirects to null. The old subprocess stdin marker is
    not an embedded-worker control channel; cancellation must be wired through
    the native worker protocol.
13. Treat the WebKit pin as part of the native runtime contract. Before changing
    `WEBKIT_VERSION`, verify that the selected prebuilt contains required JSC
    fixes and matches every target ABI; after changing it, rebuild the native
    binary and run the Windows embedded-worker smoke test. Do not call an
    engine fix sufficient until the target CI path passes, and do not add
    TSP-only event-loop or handle workarounds without a reproducible boundary.

The durable rules are also recorded in `AGENTS.md`,
`docs/v2/adr/0003-windows-fd-representation.md`,
`docs/v2/adr/0004-process-relative-vm-roles.md`, and
`docs/v2/adr/0005-embedded-vm-module-readiness.md`.

## Related files

- `bun/src/runtime/tsp_worker.rs` — embedded VM lifecycle, path ownership, and trace;
- `bun/src/jsc/VirtualMachine.rs` — VM initialization, entry loading, and per-stage trace;
- `bun/src/jsc/bindings/ZigGlobalObject.cpp` — JSC/global-object initialization;
- `bun/src/sys/lib.rs` — independent packed-`Fd` hardening;
- `bun/src/runtime/tsp/worker/manager.rs` — Windows child spawn and handshake;
- `scripts/smoke-tspserver-v2.ps1` — Windows integration smoke test;
- `docs/v2/adr/0005-embedded-vm-module-readiness.md` — module-readiness rules.
