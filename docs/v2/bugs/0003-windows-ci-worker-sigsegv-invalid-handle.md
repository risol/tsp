# BUG-0003: Windows embedded worker SIGSEGV during native startup

> Status: **Fix candidate implemented — Windows CI validation pending**
> Discovered: 2026-08-29 in GitHub Actions `smoke-windows`
> Latest evidence: [Windows CI run on `d05763182a`](https://github.com/risol/tsp/actions/runs/33269233314)
> Affected: TSP v2 embedded-worker startup on Windows
> Severity: CI blocker

## Summary

The TSP master starts and listens. A Windows worker repeatedly connects to the
master, then Bun prints a native segmentation fault and exits before the
Hello/Ready handshake. The master subsequently observes Winsock error 10054
and retries the worker connection.

This establishes the failure boundary:

```text
master startup: passed
TCP accept: passed
worker native startup: failed
TSP Hello/Ready protocol: never reached
10054: consequence of worker termination
```

The bug is real and reproducible in the Windows GitHub Actions environment.
It is not currently justified to call it a TSP protocol or route-discovery
bug.

## Evidence and current diagnosis

The latest run still failed after the earlier descriptor and JSC-thread
hypotheses were changed:

1. Preserving the packed Windows `Fd` value across the opaque
   `QuietWriter` adapter fixed a real latent representation defect, but did not
   stop this crash.
2. Disabling JSC background work did not move the failure boundary and was
   reverted.

In the latest run, checkout, toolchain setup, native compilation, and packaging
all passed. Only the final Windows smoke step failed. Captured startup markers
show that TCP connection, JSC process initialization, AST-store initialization,
and log initialization complete. The crash happens after
`virtual-machine-init:begin` and before `virtual-machine-init:end`.

The worker initializes JavaScriptCore only after it connects to the master and
before it reads `Hello`. The crash occurs inside `VirtualMachine::init`, in this
native-only interval. It is not a route, request, handshake, or TCP failure.

Code review also found a definite TSP VM-role defect. A TSP child is a worker in
the TSP process pool, but it owns the only JSC VM in its operating-system
process. It must therefore use the process-main VM role. The previous
`is_main_thread: false`, null `worker_ptr`, and absent `context_id` combination
selected the auxiliary/macro context sentinel. C++ then created a non-main
`ScriptExecutionContext`, while Rust code could still classify the VM as main
because `vm.worker` was empty. This inconsistent identity is a TSP integration
bug, although the available trace does not yet prove that it is the only native
instruction responsible for the Windows SIGSEGV.

The `0xffffffffffffffff` fault address is not enough to identify the cause.
Bun/JSC on Windows has other independent native crash classes that report the
same address, including allocator and event-loop paths. It must not be used as
proof of an invalid `Fd` without a matching stack frame.

## Disproved hypotheses

### Packed `Fd` representation

`bun/src/sys/lib.rs` now preserves the packed Windows `Fd` representation in
the opaque `QuietWriter` slot and short-circuits `Fd::INVALID`. The round-trip
test passes. This remains required hardening, but the failing CI run after
that change proves it is not a sufficient explanation for BUG-0003.

### Incorrect main-thread VM classification

The earlier investigation incorrectly treated "TSP worker" as synonymous with
"Bun WebWorker" and changed the child to `is_main_thread: false`. That rule is
retracted. A parent Bun VM or `WorkerMessagingProxy` is required for a
WebWorker, not for a process-main VM. The TSP child has no parent VM precisely
because it is a separate process and owns its process-main VM.

Initialization now uses a single `VmRole` value so main-thread publication,
script-execution-context id, and worker ownership cannot drift independently.
The TSP child uses `VmRole::ProcessMain`; only Bun's in-process WebWorker path
uses `VmRole::WebWorker`.

On 2026-08-30, a complete Windows release rebuild and the redirected two-worker
smoke test passed locally. With tracing enabled, every detailed VM marker
completed through `handshake:ready-sent`. This validates the source and native
link locally but does not close the bug until the GitHub-hosted Windows runner
passes or supplies the remaining failing substage.

## Diagnostic change

The Windows smoke jobs set `TSP_WORKER_STARTUP_TRACE=1`. The worker emits the
top-level stages plus detailed VM stages to inherited stderr:

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
vm-core:global-object:begin
jsc-vm-create:begin/end
jsc-lock:end
client-data:end
global-create:begin/end
global-publish:end
zig-global-object-create:end
vm-core:global-object:end
vm-core:stack-check:end
handshake:read-hello
handshake:hello-received
handshake:ready-sent
```

The last emitted marker on a failing run identifies whether the remaining
failure is Rust VM storage/runtime setup, JSC VM creation, client-data wiring,
or global-object construction. The trace is disabled unless the environment
variable is present.

## Rejected mitigation: disable JSC background threads

Disabling concurrent JIT compilation and parallel GC marking for the Windows
TSP worker did not change the failing CI boundary. The rebuilt executable also
passed the redirected-stdio smoke test locally, so the result rules out neither
JSC nor Windows-specific state, but it does rule out that startup-mode change
as a sufficient fix. It has therefore been removed rather than retaining an
unproven performance and runtime-semantics change.

The next CI run redirects stdout and stderr to concrete files through the
Windows command processor, not to PowerShell-owned pipes and not to the
runner's non-interactive console handles. Workers inherit those file handles.
On failure, the smoke script prints all startup markers and the final 200 lines
from each file before deleting its temporary directory. This avoids both the
pipe-liveness hazard and the lack of a usable console in GitHub Actions.

## Required next step

Run Windows CI with the coherent process-main role and detailed trace. If it
still crashes, obtain either:

- the complete `bun.report` URL and decoded native backtrace; or
- the last startup marker from the diagnostic above.

Use the last detailed marker to choose the next boundary. Do not change JSC
startup options, allocators, or unrelated TSP protocol behavior speculatively.

## Regression-prevention rules

1. VM role is relative to the current operating-system process. A TSP child
   owns that process's main VM even though it is a worker in the TSP pool.
2. Reserve `VmRole::WebWorker` for Bun's in-process worker path, which supplies
   both a `WorkerMessagingProxy` and a concrete context id.
3. Keep VM role, context id, and worker owner in one validated value; do not
   infer them independently.
4. Preserve packed `Fd` values across opaque boundaries; never reconstruct an
   `Fd` from `Fd::native()`.
5. Treat `Fd::INVALID` as a sentinel and short-circuit it before platform I/O.
6. Test workers with redirected/non-interactive stdio and multiple workers.
7. Do not assign root cause from a fault address alone; require a native frame
   or a reproducible stage boundary.

The durable rules are also recorded in `AGENTS.md`,
`docs/v2/adr/0003-windows-fd-representation.md`, and
`docs/v2/adr/0004-process-relative-vm-roles.md`.

## Related files

- `bun/src/runtime/tsp_worker.rs` — standalone worker VM lifecycle and startup trace;
- `bun/src/jsc/VirtualMachine.rs` — VM initialization options and global setup;
- `bun/src/jsc/bindings/ZigGlobalObject.cpp` — JSC initialization and global-object setup;
- `bun/src/sys/lib.rs` — independent packed-`Fd` hardening;
- `bun/src/runtime/tsp/worker/manager.rs` — Windows child spawn and handshake;
- `scripts/smoke-tspserver-v2.ps1` — Windows integration smoke test;
- `docs/v2/adr/0003-windows-fd-representation.md` — `Fd` boundary rules;
- `docs/v2/adr/0004-process-relative-vm-roles.md` — process-relative VM role rules.
