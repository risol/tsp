# ADR-0002: Preserve allocation ownership across libc, Rust, and Bun

> Status: **Accepted (2026-08-29)**
> Scope: TSP v2 native runtime embedded in Bun
> Related bug: `docs/v2/bugs/0002-mimalloc-operator-delete-sigsegv.md`

## Context

The TSP v2 native runtime is compiled into a Bun binary. In that binary, Rust
allocations use Bun's mimalloc-backed global allocator. Native APIs from libc,
the C++ runtime, WebKit/JSC, and Bun may use different allocation domains and
deallocation functions.

An allocation is not safe to pass across this boundary merely because its
pointer type is compatible. The allocator that owns the allocation must also
be the allocator that releases it.

On Linux, `std::fs::canonicalize` uses `realpath(path, NULL)`. The null output
argument makes glibc allocate the result. Dropping that result inside the Bun
binary sent the glibc-owned buffer through Bun's mimalloc-backed Rust
deallocator and caused a SIGSEGV. This was recorded as BUG-0002.

## Decision

TSP native code must preserve allocation ownership explicitly at every FFI
boundary.

For Linux path resolution, TSP uses `crate::path::canonicalize`, which passes a
caller-owned output buffer to `realpath` and copies the result before returning
a Rust `PathBuf`.

The helper is the only approved path for production TSP canonicalisation. New
call sites must not call `std::fs::canonicalize` directly on Linux.

## Rules

1. Every FFI buffer must have an explicit allocation owner and matching free
   function.
2. Never free glibc/libc-owned memory with Rust's global allocator, Bun's
   allocator, mimalloc, or a C++ `delete` operator.
3. Do not use APIs that hide foreign allocation behind a nullable output
   parameter unless the API documents a matching free function and the code
   calls that function directly.
4. Prefer caller-owned buffers for libc APIs used by the embedded runtime.
5. Keep allocator-sensitive wrappers in one small module and route all
   production call sites through that module.
6. When adding or changing native runtime FFI, run focused Rust tests, a Linux
   embedded-worker release build, and the TSP v2 smoke test.
7. If a foreign allocation must cross a boundary, copy it into memory owned by
   the receiving allocator before storing it in an owning Rust/C++ type.

## Consequences

Positive:

- ownership is visible at the call site;
- the TSP runtime does not depend on Bun's private path-buffer types;
- Linux embedded-worker startup and first-request behavior remain safe;
- the rule applies to future libc/JSC/Bun integrations, not just paths.

Trade-offs:

- some platform-specific wrappers are required;
- FFI code needs explicit comments and targeted regression coverage;
- copying a small result buffer is preferred over an ambiguous zero-copy path.

## Review checklist

Before merging native-runtime changes, reviewers should ask:

- Who allocated each pointer?
- Which exact function frees it?
- Can the pointer enter a Rust `Vec`, `String`, `PathBuf`, `Box`, or C++ owner?
- Does the code behave the same when Bun's mimalloc global allocator is active?
- Is there a Linux embedded-worker smoke test covering the changed path?
