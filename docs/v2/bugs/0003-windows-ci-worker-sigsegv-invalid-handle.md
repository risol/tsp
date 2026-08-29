# BUG-0003: Windows CI worker SIGSEGV at `0xFFFFFFFFFFFFFFFF`

> Status: **Fixed in source; Windows 11 CI rerun pending**
> Discovered: 2026-08-29 in GitHub Actions `smoke-windows` on `4ce77078e`
> Affected: TSP v2 embedded-worker startup with redirected Windows stdio
> Severity: CI blocker

## Summary

The TSP master process started normally, but the child worker crashed during
Bun VM startup before completing the Hello/Ready handshake. The observed
fault address was `0xFFFFFFFFFFFFFFFF`; the master consequently saw Winsock
error 10054 because the worker reset the TCP connection.

The original investigation incorrectly treated the address as proof that a
Win32 `HANDLE` was directly dereferenced. The relevant defect is a more
specific representation error in Bun's output adapter: a packed Windows
`Fd` was converted to a native handle and then reconstructed as a packed `Fd`.
When standard output was unavailable, `Fd::INVALID` became the raw native
value `INVALID_HANDLE_VALUE` and was later reinterpreted as a libuv `Fd` with
value `-1`.

## Root cause

The affected path is exercised during worker VM creation:

```text
tspserver_v2 --tsp-worker
  -> bun main startup / redirected stdio setup
  -> VirtualMachine::init
  -> ConsoleObject::init_in_place
  -> QuietWriter::adapt_to_new_api
  -> qw_set_fd(Fd)
  -> native HANDLE stored in opaque slot
  -> qw_fd() reconstructs packed Fd from that HANDLE
```

On Windows, `Fd` is a packed `u64`; bit 63 distinguishes a system handle
from a libuv descriptor. `Fd::INVALID` is the packed zero sentinel, while its
Win32 decode is `INVALID_HANDLE_VALUE`. Therefore this conversion is not
round-trippable:

```text
Fd::INVALID (packed 0)
  -> native() = HANDLE(-1)
  -> from_native(0xFFFFFFFFFFFFFFFF)
  -> FdKind::Uv(-1)          # wrong kind and wrong sentinel
```

The defect is in the Bun fork's Windows Fd/output boundary, not in TSP route
discovery or the TSP TCP protocol. TSP is the code path that makes the
separate Bun worker start, so it exposed the defect; this is why the symptom
looked TSP-specific.

`ParentDeathWatchdog` is not the root cause: it is disabled unless
`BUN_FEATURE_FLAG_NO_ORPHANS` is enabled, and its Windows failure paths use
`NULL`/`BOOL` checks rather than treating a failed handle as a valid pointer.
The `is_main_thread: true` option is also intentionally retained: this worker
is a separate process whose only JS thread is its process main thread, not a
`WebWorker` with a `WorkerMessagingProxy`.

## Fix

`bun/src/sys/lib.rs` now:

1. stores the packed `Fd` value in the opaque `QuietWriter` slot;
2. restores the same packed value, preserving the Windows kind bit;
3. returns immediately when a quiet write receives `Fd::INVALID`.

A regression test verifies round trips for the invalid sentinel, ordinary
native descriptors, and the Windows libuv `-1` representation.

## Verification

Completed locally:

- `cargo check -p bun_sys --lib` — passed;
- `cargo check -p bun_bin --target x86_64-pc-windows-msvc` — passed.
- Full Windows release Rust/native compilation and linking — passed;
- `scripts/smoke-tspserver-v2.ps1` with the rebuilt `tspserver_v2.exe` — passed
  on Windows 10 22H2 with redirected stdout/stderr and two workers.

The standalone `cargo test -p bun_sys --lib` command cannot link in this
checkout because it omits Bun's generated/native libraries; the test itself
is included in the normal Bun build. The normal release command's final copy
to `bun.exe` was blocked because the command was itself running from the old
`bun.exe`; the newly linked `bun-profile.exe` was copied under the required
`tspserver_v2.exe` basename and passed the smoke test. The current host is
Windows 10 rather than the failing Windows 11 runner, so a GitHub Actions
rerun is still required to close the CI-specific part of this bug.

## Regression-prevention rules

The durable rules are recorded in:

- `AGENTS.md`, under **Native runtime and allocator boundaries**;
- `docs/v2/adr/0003-windows-fd-representation.md`.

Short version:

1. Preserve packed `Fd` values across opaque boundaries.
2. Never reconstruct `Fd` from `Fd::native()`.
3. Treat `Fd::INVALID` as a sentinel and short-circuit it before platform I/O.
4. Test workers with redirected/non-interactive stdio, not only an interactive
   terminal.

## Related files

- `bun/src/sys/lib.rs` — fixed `QuietWriter` Fd transport and regression test;
- `bun/src/bun_core/util.rs` — Windows packed `Fd` representation;
- `bun/src/runtime/tsp_worker.rs` — worker entry and VM startup;
- `bun/src/runtime/tsp/worker/manager.rs` — Windows child spawn and redirected
  stdio;
- `scripts/smoke-tspserver-v2.ps1` — Windows integration smoke test;
- `docs/v2/adr/0003-windows-fd-representation.md` — permanent boundary rule.
