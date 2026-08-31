# TSP architecture plan

Status: non-normative engineering plan. This document explains why the
runtime is shaped as it is and records likely next steps. It does not add an
application API. Shipped behavior is defined by the
[runtime specification](./tsp-specification.html) and summarized by the
[current contract](./reference/contract.html).

## 1. Direction

TSP is a small native web runtime for TypeScript Server Pages. The goal is a
clear boundary between native infrastructure and reloadable application code:

```text
HTTP -> Rust host -> route/generation registry -> Bun worker -> HTTP response
```

The page format remains ordinary TypeScript/TSX. HTTP methods are explicit
named exports. JSX is a server-rendering format rather than a client framework.

## 2. Design principles

### Explicit route modules

`.tsp` files are route entry points, not reusable library modules. This keeps
one route's generation graph from depending on another route's lifecycle.
Shared code belongs in ordinary `.ts`, `.tsx`, `.js`, or `.jsx` modules.

### Native ownership of durable state

Sessions, worker coordination, service registries, request admission, and
other long-lived resources belong to the host. A page generation may be
replaced at any time, so it must not own resources that need to outlive it.

### Disposable generations

The runtime builds an immutable candidate generation from a route and its local
dependencies. It validates the candidate before publishing it atomically.
Requests pin their generation. A failed candidate leaves the last known-good
generation active.

### Narrow host/worker protocol

Workers receive serialized request data and return a serialized response
envelope. Cookie writes, session writes, service logs, and response headers
cross the boundary explicitly. Native pointers and allocator-owned buffers do
not cross as borrowed application objects.

### Safety by default

The host enforces path safety, body limits, method validation, and worker
limits. JSX escapes ordinary strings and attributes. Raw HTML is an explicit
trust boundary. Environment access exposed to pages is deliberately narrower
than the process environment.

## 3. Ownership model

| Concern | Owner | Lifetime |
| --- | --- | --- |
| HTTP listener and routing | Rust host | process |
| Worker process and VM | worker manager | worker |
| Page module graph | generation registry | generation |
| Session store | host service | process/backend |
| Config-driven services | host service registry | process, reloadable snapshot |
| Request context and body | host + worker | request |
| JSX tree and handler locals | worker | request |

The allocator that creates an FFI buffer is responsible for releasing it. A
foreign pointer must be copied into receiving-owned storage or paired with its
foreign free function. See [ADR-0002](./reference/adr/0002-cross-allocator-ownership.html).

## 4. Request pipeline

1. Parse the request and normalize its path.
2. Match the path and method against the route table.
3. Apply global and page-level request limits.
4. Select and pin the current route generation.
5. Build the request context and send it to a worker.
6. Evaluate the page module and invoke the selected handler.
7. Decode the response envelope and commit host-owned writes.
8. Emit the HTTP response and release request resources.

Cancellation is represented by `AbortSignal` in page code and a native worker
control message. Redirected worker stdin is not a control channel.

## 5. Route and module graph

The route scanner maps the filesystem to URL paths and records exported
methods. The module graph records canonical local imports and reverse
dependents. Static routes outrank dynamic routes; dynamic routes outrank
catch-all routes.

The graph must reject:

- ambiguous route shapes;
- local imports outside the configured application root;
- route-to-route `.tsp` imports;
- unknown runtime exports; and
- invalid page configuration.

The graph is intentionally based on canonical paths. A route alias must not
cause one request to reuse another request's prepared module or body.

## 6. Generation lifecycle

```text
source change
    -> mark dirty
    -> build candidate
    -> validate exports/config/module graph
    -> publish atomically
    -> pin for new requests
```

The watcher may poll filesystem metadata and source hashes. A candidate build
is deduplicated so concurrent requests do not rebuild the same route in
parallel. A failed build reports diagnostics while the last known-good
generation continues serving.

Generation identity is distinct from canonical module identity. Module
identity answers “which source file is this?”; generation identity answers
“which evaluated version did this request use?”

## 7. Worker lifecycle

The master owns a pool of self-spawned worker processes. Each worker owns one
JavaScript runtime in its process. The master controls:

- startup readiness;
- bounded admission;
- request cancellation and timeout;
- maximum request count, age, and memory;
- crash detection and replacement; and
- cross-worker invalidation.

On Windows, `bun_core::Fd` is a packed value. Opaque adapters must preserve
the packed value and treat `Fd::INVALID` as a sentinel. VM role classification
is process-relative and must use one authoritative `VmRole` value. See
[ADR-0003](./reference/adr/0003-windows-fd-representation.html) and
[ADR-0004](./reference/adr/0004-process-relative-vm-roles.html).

## 8. Embedded VM readiness

VM construction is not module readiness. Before loading an entry module, the
worker must configure the resolver environment, defines, runtime environment,
and source-code printer. The entry-path backing storage must remain valid for
the VM lifetime.

Embedded-worker tests must execute a generated module, not merely complete a
startup handshake. See [ADR-0005](./reference/adr/0005-embedded-vm-module-readiness.html).

## 9. Service and session boundaries

The service registry is host-owned and exposes request snapshots to pages.
Built-in services such as logger, session, and time are separate from the page
generation. Config-driven service snapshots may be replaced by the watcher,
while built-in runtime services remain alive.

Sessions use an explicit backend interface. The in-memory backend is the
default; Redis is selected by `TSP_REDIS_URL`. Page session writes cross the
worker boundary as JSON-compatible operations and are committed by the host.

Database and other native resources must not be hidden in a disposable page
generation. Optional built-ins are lazy so importing an unused namespace does
not initialize a native subsystem.

## 10. Error model

Errors are grouped by phase:

| Family | Meaning |
| --- | --- |
| `TSP1xxx` | route and filesystem configuration |
| `TSP2xxx` | request parsing and limits |
| `TSP3xxx` | preparation, JSX, worker, and handler execution |

Development mode may expose structured HTML diagnostics. Production responses
must not leak source paths, stacks, credentials, or native internals.

## 11. Verification strategy

Every new runtime feature should have:

1. a focused Rust unit test;
2. a worker or process-model integration test when a boundary is involved;
3. a smoke or end-to-end test when request behavior changes;
4. generated typings or example coverage when the public API changes; and
5. documentation that labels the feature current or planned.

Native changes that affect paths, allocators, FFI, or embedded workers require
the focused tests, a Linux release build, and the smoke test. Windows worker
changes require the redirected non-interactive smoke shape used by CI.

## 12. Roadmap

### Current baseline

- filesystem routing and explicit method handlers;
- request context, responses, cookies, sessions, and fragments;
- embedded worker pool and bounded request execution;
- module graph, hot reload, atomic generations, and last-known-good serving;
- generated `tsp:*` declarations;
- native checks, route listing, graph inspection, and smoke tests.

### Candidate future work

These are intentionally not current API promises:

- streamed response bodies;
- a richer, validated fragment method form;
- first-class layouts and middleware;
- more host-owned service adapters;
- improved inspector and source-map tooling; and
- performance work after contract behavior is stable.

Each item needs a contract amendment, implementation, conformance tests, and a
documentation update before it becomes current.

## 13. Definition of done for a change

A feature is complete when its ownership is explicit, its failure behavior is
tested, its public names have generated typings where appropriate, its docs do
not contradict the runtime, and the relevant platform smoke tests pass.
