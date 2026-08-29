# ADR-0003: Preserve packed `Fd` values across opaque output adapters

> Status: **Accepted (2026-08-29)**
> Scope: Bun Windows runtime code used by the TSP embedded worker
> Related investigation: `docs/v2/bugs/0003-windows-ci-worker-sigsegv-invalid-handle.md`

## Context

This ADR records an independent latent defect found while investigating
BUG-0003. It is a valid representation rule, but the packed-`Fd` defect did
not explain the latest failing Windows CI run and is not the claimed root
cause of BUG-0003.

On Windows, Bun's `Fd` is not just a native `HANDLE`. It is a packed `u64`:
the low 63 bits carry the value and bit 63 records whether the value is a
system handle or a libuv file descriptor. `Fd::INVALID` is the zero packed
value; decoding it deliberately yields `INVALID_HANDLE_VALUE` for Win32 calls.

The output adapter is an opaque FFI-shaped value whose first slot stores an
`Fd`. Serializing `Fd::native()` into that slot and later reconstructing it as
an `Fd` loses the kind bit. In particular, `INVALID_HANDLE_VALUE` (`-1`) is
reinterpreted as a libuv descriptor with value `-1`, rather than as the
`Fd::INVALID` sentinel. This is an invalid representation even though both
forms have the same machine word size.

## Decision

Opaque adapters that carry an `Fd` store the packed `Fd` representation and
restore it without converting through the native platform handle. Any path
that writes, closes, or probes an `Fd` must check `Fd::INVALID` first and treat
it as an unavailable resource.

The worker smoke test remains the integration check because the failure is
most likely when a child process is started with redirected or unavailable
standard streams. Interactive PowerShell output is not sufficient coverage.

## Rules

1. Do not serialize `Fd::native()` and later cast it back to `Fd`.
2. Preserve the Windows system/libuv kind bit across opaque boundaries.
3. Short-circuit `Fd::INVALID` before Win32, libuv, or CRT operations.
4. Add a round-trip regression test for every new opaque `Fd` adapter.
5. Verify with the Windows embedded-worker smoke test using redirected stdio.

## Consequences

- Output adapters preserve both valid descriptor kinds and invalid sentinels.
- Invalid standard streams fail as ordinary unavailable output instead of
  entering a different descriptor backend.
- Opaque FFI layouts need explicit representation comments and tests.
