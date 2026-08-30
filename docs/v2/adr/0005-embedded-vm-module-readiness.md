# ADR-0005: Complete embedded VM module readiness before loading entries

> Status: **Accepted (2026-08-30)**
> Scope: TSP worker processes embedding Bun/JSC
> Related investigation: `docs/v2/bugs/0003-windows-ci-worker-sigsegv-invalid-handle.md`

## Context

`VirtualMachine::init` constructs JSC, the global object, the event loop, and a
Transpiler. It does not complete all runtime configuration required to load a
user module. Bun's CLI and WebWorker paths perform a second phase that connects
the resolver to the env loader, configures defines, loads env-derived runtime
settings, and installs the thread-local source-code printer.

TSP originally skipped that phase and called `load_entry_point` immediately.
The worker could therefore complete Hello/Ready while its transpiler and error
reporting state were not ready for the first generated route module.

The VM also stores its main entry as a borrowed slice. Normal Bun callers use
process-lifetime argv or worker-owned storage. TSP used a path owned by the
current IPC request, violating the lifetime contract after the request ended.

## Decision

Every TSP embedded worker performs these steps in order:

1. Construct the process-main VM with `VirtualMachine::init`.
2. Set the resolver env loader to the Transpiler-owned loader.
3. Select the normal Bun runtime env behavior and run `configure_defines`.
4. Load env-derived HTTP/runtime state and install the source-code printer.
5. Only then send Ready and accept module execution requests.

`EmbeddedVm` owns a reusable entry-path buffer. Before each load it copies the
request path into that buffer and passes the stable slice to
`load_entry_point`.

## Rules

1. Treat low-level VM construction and module readiness as separate phases.
2. Do not send Ready until both phases have completed successfully.
3. Keep TSP's preparation sequence aligned with Bun's CLI/WebWorker lifecycle.
4. Propagate `configure_defines` failures as worker initialization errors; do
   not continue with a partially configured Transpiler.
5. Any path passed to `set_main` must outlive every later read by that VM.
6. Copy IPC-owned or request-owned paths into VM-owned storage before loading.
7. Regression tests must execute generated modules, repeated requests, and hot
   reload; handshake-only tests are insufficient.
8. Treat the pinned WebKit/JSC release as part of embedded-worker readiness.
   When a failure is inside `JSC::VM::drainMicrotasks`, check the pinned engine
   for upstream CodeBlock or microtask-cache fixes before changing TSP event-loop
   or handle logic. Validate the selected release's target ABI, rebuild the
   native binary, and run the Windows embedded-worker smoke test. A local
   pass or an engine change alone is not sufficient; the target Windows CI
   path must pass before recording the engine update as the root-cause fix.
9. Do not call `run_gc(false)` before a pending module-evaluation promise has
   settled. GC is a post-entry maintenance operation in Bun's CLI lifecycle,
   not a readiness barrier; moving it before the first event-loop turn can
   crash the embedded Windows worker.
10. Generated embedded-worker wrappers must not create asynchronous work for
    a synchronous handler unless the response type requires it. Keep a
    synchronous fast path, with an explicit promise fallback for async
    handlers, so the first JSC microtask checkpoint contains only required
    work.

## Consequences

- Ready means that the worker can load a route, not merely that JSC exists.
- Module resolution, define expansion, env access, and error formatting use
  initialized per-thread state.
- Repeated requests cannot observe a dangling previous entry path.
