# ADR 0006: TSP-owned compiler and runtime boundary

## Status

Accepted for implementation on the native runtime branch.

## Context

The existing TSP worker embeds Bun's runtime around JavaScriptCore. That makes
route execution depend on Bun VM startup, Bun's event loop, Bun's transpiler,
Bun's module loader, and Bun-specific process and handle state. The Windows CI
failures showed that a successful worker handshake does not guarantee that the
request execution lifecycle is safe.

JavaScriptCore is an ECMAScript engine, not a TypeScript compiler or a server
runtime. TSP therefore needs an explicit compiler/runtime boundary instead of
using Bun as an implicit framework.

## Decision

TSP will own the HTTP server, router, request lifecycle, response model,
worker protocol, module graph, compiler orchestration, and built-in modules.
Only the minimum JavaScriptCore binding and build integration from Bun may be
reused. TSP code must not depend on Bun's runtime, event loop, transpiler,
resolver, HTTP types, or worker manager.

The compiler has two separate responsibilities:

1. The build-time frontend parses TypeScript and TSX, erases types, lowers
   JSX, validates TSP route exports, resolves the TSP module graph, and emits
   ordinary JavaScript plus a manifest.
2. The optional type-checking step runs outside the server process. A type
   error prevents a build, but the production server does not embed the
   TypeScript language service.

The first compiler frontend uses the pinned TypeScript compiler API as a
build-time dependency. The TSP-specific pipeline remains owned by TSP. This
keeps full TypeScript/TSX syntax support without coupling the server to Bun.
The frontend can later be replaced by Oxc without changing the manifest or
runtime contracts.

Workers receive compiled JavaScript artifacts. They never transpile source code
on the first request. JSC is exposed through a small `tsp-jsc` layer that owns
VM affinity, exception conversion, value lifetime, promise settlement, and
microtask checkpoints.

## Initial artifact contract

`tools/tspc.mjs` is the first build-time implementation. It emits:

- one JavaScript file for each source module;
- one `manifest.json` containing routes, methods, source names, and outputs;
- TSP route validation errors before a worker is started.

The artifact format is deliberately independent of Bun's module loader. The
native runtime will consume this format after the JSC smoke layer is stable.

## Consequences

- Runtime startup no longer requires Bun transpilation or Bun CLI discovery.
- TypeScript compilation failures become deterministic build errors.
- The old Bun-backed runtime remains available as a compatibility reference
  until the new runtime passes the complete application E2E suite.
- The TSP compiler must define its supported module and JSX semantics
  explicitly; JSC alone cannot provide them.
- A minimal JSC microtask test is a release gate. Rewriting HTTP or workers
  cannot hide a native JSC lifetime defect.

## Migration gates

1. Cross-platform JSC binding tests pass for repeated synchronous evaluation,
   exceptions, promises, microtasks, and multiple VM instances.
2. Compiler golden tests pass for every route shape and every supported
   handler result.
3. The new HTTP and worker layers pass protocol and lifecycle tests without
   linking Bun runtime crates.
4. Linux, macOS, and Windows application E2E pass before the old worker path is
   removed.
