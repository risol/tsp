# Verified bugs and regressions

These records describe investigated failures, evidence, fixes, and regression
tests. They are useful when debugging native runtime behavior; they do not
define new application features.

- [BUG-0001: route aliases and request body reuse](./0001-multi-route-aliases-to-first-request.html)
- [BUG-0002: allocator mismatch while freeing canonicalize output](./0002-mimalloc-operator-delete-sigsegv.html)
- [BUG-0003: Windows embedded-worker startup SIGSEGV](./0003-windows-ci-worker-sigsegv-invalid-handle.html)

Add a record only after the root cause is supported by a reproducible boundary
or symbolicated evidence. Link fixes to the regression test that prevents the
failure from returning.
