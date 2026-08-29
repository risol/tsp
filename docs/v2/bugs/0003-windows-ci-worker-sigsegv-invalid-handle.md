# BUG-0003: Windows embedded worker SIGSEGV during native startup

> Status: **Open — native crash remains unresolved**
> Discovered: 2026-08-29 in GitHub Actions `smoke-windows`
> Latest evidence: [Windows smoke job on `f9f14e4219`](https://github.com/risol/tsp/actions/runs/33242655859/job/99074617924)
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

The latest run still failed after both earlier hypotheses were changed:

1. Preserving the packed Windows `Fd` value across the opaque
   `QuietWriter` adapter fixed a real latent representation defect, but did not
   stop this crash.
2. Initializing the standalone worker with `is_main_thread: false` corrected
   the VM ownership classification, but did not stop this crash either.

In the latest run, checkout, toolchain setup, native compilation, and packaging
all passed. Only the final Windows smoke step failed. This rules out a build
artifact or package assembly failure, but the public job page exposes only the
step annotation; its detailed log and worker trace are unavailable to the
current unauthenticated API session.

The worker initializes JavaScriptCore only after it connects to the master and
before it reads `Hello`. The crash occurs in this native-only interval. The
current evidence therefore points to a Bun/JSC Windows startup problem, but it
does not identify the exact C++ function, allocator operation, or invalid
pointer. The public Actions page does not expose the symbolicated crash frame,
and the pasted log is interleaved/truncated.

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

### Main-thread VM classification

`bun/src/runtime/tsp_worker.rs` now uses `is_main_thread: false` for the
standalone worker. This is the correct ownership model because the process has
no parent Bun VM or `WorkerMessagingProxy`. However, the subsequent Windows CI
run still crashed, so this change is not the BUG-0003 fix.

## Diagnostic change

The Windows smoke jobs set `TSP_WORKER_STARTUP_TRACE=1`. The worker then emits
the following stage markers to inherited stderr:

```text
tcp-connect:begin/end
jsc-initialize:begin/end
ast-store:end
log-init:end
virtual-machine-init:begin/end
handshake:read-hello
handshake:hello-received
handshake:ready-sent
```

The last emitted marker on a failing run identifies the next native boundary to
instrument or bisect. The trace is disabled unless the environment variable is
present.

The `f9f14e4219` diagnostic run failed in the smoke step, but its last marker
cannot be verified from the accessible job metadata. Do not treat the absence
of a visible marker in the job summary as evidence that the worker reached or
passed a particular native phase.

## Required next step

Obtain one untruncated Windows worker stderr/crash report containing either:

- the complete `bun.report` URL and decoded native backtrace; or
- the last startup marker from the diagnostic above.

Only after that evidence should a functional workaround be selected, such as
changing JSC thread/allocator options or altering VM creation. Disabling
concurrent JIT/GC or changing context IDs speculatively would change runtime
semantics/performance without proving this crash path.

## Regression-prevention rules

1. Treat a standalone embedded worker process as an isolated VM, not as a
   `WebWorker` and not as Bun's process-wide main VM.
2. Do not pass `is_main_thread: true` to a VM that has no parent Bun main VM or
   `WorkerMessagingProxy` owner.
3. Preserve packed `Fd` values across opaque boundaries; never reconstruct an
   `Fd` from `Fd::native()`.
4. Treat `Fd::INVALID` as a sentinel and short-circuit it before platform I/O.
5. Test workers with redirected/non-interactive stdio and multiple workers.
6. Do not assign root cause from a fault address alone; require a native frame
   or a reproducible stage boundary.

The durable rules are also recorded in `AGENTS.md` and
`docs/v2/adr/0003-windows-fd-representation.md`.

## Related files

- `bun/src/runtime/tsp_worker.rs` — standalone worker VM lifecycle and startup trace;
- `bun/src/jsc/VirtualMachine.rs` — VM initialization options and global setup;
- `bun/src/jsc/bindings/ZigGlobalObject.cpp` — JSC initialization and global-object setup;
- `bun/src/sys/lib.rs` — independent packed-`Fd` hardening;
- `bun/src/runtime/tsp/worker/manager.rs` — Windows child spawn and handshake;
- `scripts/smoke-tspserver-v2.ps1` — Windows integration smoke test;
- `docs/v2/adr/0003-windows-fd-representation.md` — `Fd` boundary rules.
