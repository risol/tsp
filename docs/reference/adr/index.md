# Architecture decision records

These records preserve decisions about native/runtime boundaries. They are
historical engineering references, not additional application APIs.

- [ADR-0002: allocation ownership](./0002-cross-allocator-ownership.html)
- [ADR-0003: Windows packed `Fd` representation](./0003-windows-fd-representation.html)
- [ADR-0004: process-relative VM roles](./0004-process-relative-vm-roles.html)
- [ADR-0005: embedded VM module readiness](./0005-embedded-vm-module-readiness.html)

When a new native boundary is introduced, add an ADR before changing the
ownership rule, then link the decision from the architecture overview.
