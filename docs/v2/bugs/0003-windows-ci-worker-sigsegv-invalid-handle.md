# BUG-0003: Windows embedded worker SIGSEGV during first module evaluation

> Status: **Fix applied — Windows CI validation pending**
> Discovered: 2026-08-29 in GitHub Actions `smoke-windows`
> Latest evidence: [Windows CI run on `56bd752049`](https://github.com/risol/tsp/actions/runs/33283009469)
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

This is a TSP-to-Bun embedding lifecycle bug, not a TSP protocol,
route-discovery, TCP, or generic JSC startup failure.

## Evidence and root cause

The `56bd752049` run passed checkout, compilation, packaging, JSC VM creation,
global-object creation, stack-check setup, and the worker handshake. Every
worker reached `handshake:ready-sent`. The crash occurred only after the master
sent the first generated `index.tsp` request, and every replacement worker
produced the same crash report.

Code comparison with both Bun's process-main CLI and in-process WebWorker paths
found that TSP skipped the required post-`VirtualMachine::init` runtime
configuration. Those paths initialize the resolver env-loader relationship,
run `transpiler.configure_defines()`, load env-derived runtime state, and
install the per-thread source-code printer before loading a module. TSP called
`load_entry_point()` immediately. It therefore created a VM that was ready for
the protocol but was not ready for module transpilation, loading, or error
reporting.

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

## Fix

The embedded worker now completes the same module-readiness phase as Bun's
other module-loading VM entry points:

1. Construct the process-main VM.
2. Connect the Transpiler resolver to its env loader.
3. Select normal Bun runtime env behavior and run `configure_defines()`.
4. Load env-derived HTTP/runtime state.
5. Install the per-thread source-code printer.
6. Send Ready only after all preparation succeeds.
7. Copy each request's entry path into VM-owned storage before loading it.
8. On Windows, clear `uws::Loop::internal_loop_data.jsc_vm` after init so the
   JSC park hook (`Bun__JSC_onBeforeWait`) does not fire. The hook releases
   heap access and — Windows + mimalloc — drives `mi_on_thread_idle()`,
   which dereferences a stale retired-page pointer on the first call
   (`0xFFFFFFFFFFFFFFFF`, the Windows `INVALID_HANDLE_VALUE` sentinel).
   The TSP worker is the JS thread for its process and does not need the
   per-poll heap-access release, so disabling the park hook is a
   process-local optimisation that costs nothing and avoids the crash.

Failures from `configure_defines()` are returned as worker initialization
errors instead of continuing with a partially configured VM.

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
request:load-entry:end
request:response-ready or request:error-ready
```

The smoke script redirects stdout and stderr to concrete files through the
Windows command processor. Workers inherit those file handles, and failures
include all markers plus the final 200 log lines.

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
   `__tspEmbeddedResponse`).
6. `load-entry:wait:auto-tick` — `EventLoop::auto_tick` → runtime hook
   `tick` → `bun_runtime::jsc_hooks::auto_tick` → uSockets poll on
   Windows (IOCP, `WSARecv` / `GetQueuedCompletionStatus`).
7. `load-entry:wait:resolved` (with no preceding `:end`) — a fault
   observed in promise-resolution bookkeeping (status polling, microtask
   folding).

The crash address `0xFFFFFFFFFFFFFFFF` (Windows `INVALID_HANDLE_VALUE` /
`-1`) is consistent with a Windows HANDLE being dereferenced as a pointer.
The latest CI run printed every `entry-eval:*` `:end` marker and reached
`load-entry:wait:begin` but never printed `load-entry:wait:resolved` or
any subsequent `wait:end` — the fault is therefore inside phase 5 or 6,
most likely the `auto_tick` → uSockets Windows poll path. A new
bun.report frame on the `31488de3e1` commit, symbolicated against the
same-commit PDB (offsets can shift between separately linked binaries, so
the previous mapping is supporting evidence only), should land in either
`EventLoop::tick_turn` (microtask drain) or
`bun_runtime::jsc_hooks::auto_tick` / `uws::Loop::wakeup` /
`bun_io::waker::wake` to confirm.

## Required validation

Run Windows CI with the complete module-readiness lifecycle, owned entry
path, and the segmented `load_entry_point` trace. The smoke test must pass
the first request, repeated requests, and hot reload. If it still crashes,
the segmented trace above will identify the failing sub-stage; use that
boundary together with the same-commit PDB symbolication (offsets can shift
between separately linked binaries, so the previous mapping is supporting
evidence only) before changing any further runtime configuration.

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
