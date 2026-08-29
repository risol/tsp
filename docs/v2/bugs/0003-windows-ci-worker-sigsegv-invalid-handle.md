# BUG-0003: Windows embedded worker SIGSEGV during VM startup

> Status: **Fix candidate in source; remote Windows CI validation pending**
> Discovered: 2026-08-29 in GitHub Actions `smoke-windows` on `4ce77078e`
> Latest evidence: [Windows smoke job on `9db7c6a6d9`](https://github.com/risol/tsp/actions/runs/33235574129/job/99055958371)
> Affected: TSP v2 embedded-worker startup on Windows
> Severity: CI blocker

## Summary

The TSP master starts and listens, but a Windows worker process crashes before
the Hello/Ready handshake. The master then observes Winsock error 10054 and
retries the worker connection. The crash is therefore in the worker process,
not in route discovery or the TSP wire protocol.

The first fix attempt preserved packed Windows `Fd` values across the output
adapter. That change fixes a real latent representation bug, but the latest
Windows smoke job still failed after it was deployed. It is not sufficient
evidence for the observed BUG-0003 crash and must not be described as its root
cause.

## Root cause

The worker is a standalone child process with one JavaScript thread. It is not
a Bun `WebWorker`: there is no parent VM and no `WorkerMessagingProxy`. The
TSP worker nevertheless initialized its VM with:

```text
is_main_thread: true
worker_ptr: null
```

That combination publishes the VM in `MAIN_THREAD_VM` and selects the normal
main-global initialization path, while the C++ worker-specific path is only
meaningful when a worker owner is present. This is an invalid lifecycle model
for the standalone worker and leaves JSC startup dependent on the wrong global
and VM classification. The failure occurs before the protocol handshake,
which explains why the master only sees a reset connection.

The exact symbolicated native frame is not available from the public Actions
page, so the claim above is the source-level root cause and trigger boundary;
the CI rerun is required to confirm the platform-specific crash is gone.

## Fix

`bun/src/runtime/tsp_worker.rs` now initializes the standalone worker with
`is_main_thread: false`. This keeps the VM local to the worker thread, avoids
publishing it as the process-wide Bun main VM, and selects the isolated global
path used by other non-main VM initialization paths. The worker still runs on
the process's OS entry thread; “not main VM” is a runtime ownership decision,
not an OS thread claim.

The earlier `bun/src/sys/lib.rs` change is retained as independent hardening:
it preserves packed `Fd` values in the opaque `QuietWriter` slot and rejects
`Fd::INVALID` before I/O. That defect is documented separately in
`docs/v2/adr/0003-windows-fd-representation.md` and is not the proven cause of
the CI crash.

## Verification

Completed locally with the rebuilt Windows executable:

- `cargo check -p bun_bin --target x86_64-pc-windows-msvc --locked` — passed;
- full Windows release Rust/native compilation and linking — passed;
- `scripts/smoke-tspserver-v2.ps1` — passed with redirected stdio, two workers,
  HTTP requests, metrics, and hot reload on Windows 10 22H2.

The local host is Windows 10, while GitHub Actions uses the Windows runner
that reproduced the failure. The fix remains open until that job passes.

## Regression-prevention rules

1. Treat a standalone embedded worker process as an isolated VM, not as a
   `WebWorker` and not as Bun's process-wide main VM.
2. Do not pass `is_main_thread: true` to a VM that has no parent Bun main VM or
   `WorkerMessagingProxy` owner.
3. Preserve packed `Fd` values across opaque boundaries; never reconstruct an
   `Fd` from `Fd::native()`.
4. Treat `Fd::INVALID` as a sentinel and short-circuit it before platform I/O.
5. Test workers with redirected/non-interactive stdio and multiple workers.

The durable rules are also recorded in `AGENTS.md` and
`docs/v2/adr/0003-windows-fd-representation.md`.

## Related files

- `bun/src/runtime/tsp_worker.rs` — standalone worker VM lifecycle;
- `bun/src/jsc/VirtualMachine.rs` — VM initialization options and global setup;
- `bun/src/jsc/bindings/ZigGlobalObject.cpp` — C++ global-object selection;
- `bun/src/sys/lib.rs` — independent packed-`Fd` hardening;
- `bun/src/runtime/tsp/worker/manager.rs` — Windows child spawn and handshake;
- `scripts/smoke-tspserver-v2.ps1` — Windows integration smoke test;
- `docs/v2/adr/0003-windows-fd-representation.md` — `Fd` boundary rules.
