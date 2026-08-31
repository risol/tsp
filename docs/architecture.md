# Architecture overview

TSP separates native infrastructure from reloadable application code.

```text
HTTP request
  -> Rust host
  -> route table and request limits
  -> generation registry
  -> embedded Bun worker
  -> response envelope
  -> HTTP response
```

## Ownership boundaries

| Layer | Owns |
| --- | --- |
| Rust host | sockets, HTTP parsing, routing, limits, worker pool, sessions, services, publication |
| Worker process | one JavaScript runtime, module evaluation, handler execution, response envelope |
| Page generation | route module and its reloadable local dependency graph |
| External/runtime module | packages and host-provided built-ins that outlive a page generation |
| Application code | business logic and explicit request/response behavior |

The host must not release memory through an allocator that owns a foreign FFI
buffer. Native wrappers therefore copy foreign data into the receiving
allocator or provide a matching free function. See the [allocator ADR](./reference/adr/0002-cross-allocator-ownership.html).

## Request lifecycle

1. The host parses and normalizes the request.
2. The route table selects a page and method.
3. Global and page-specific limits are applied.
4. The page generation is pinned for the request.
5. The worker receives a request context and evaluates the handler.
6. The worker returns a response envelope, including cookie and session writes.
7. The host commits host-owned writes and sends the HTTP response.

The context and its request body are request-scoped. A page must not retain
them in module-level durable state.

## Generations and reload

The watcher tracks route modules and supported local dependencies. A change
marks affected routes dirty. The runtime builds a candidate generation,
validates exports and configuration, and publishes it atomically. Existing
requests continue using the generation they already pinned.

If candidate evaluation fails, the last known-good generation remains active.
This makes a syntax or dependency error visible without taking an otherwise
healthy route offline.

## Worker model

Each worker process owns its JavaScript runtime and its process-local module
state. The host controls admission, cancellation, recycling, and replacement.
Worker processes communicate with the host through the native worker protocol;
generated route code must not use redirected standard input as a control
channel.

On Windows, packed `Fd` values retain their representation across opaque
interfaces. A system handle must not be reconstructed from `Fd::native()`.
See the [Windows Fd ADR](./reference/adr/0003-windows-fd-representation.html)
and [process-relative VM roles ADR](./reference/adr/0004-process-relative-vm-roles.html).

## Runtime readiness

Embedded VM startup has two phases: low-level VM creation and module
readiness. The resolver environment, defines, runtime environment, and source
printer must be configured before loading an entry module. A successful startup
handshake alone does not prove that a generated module can execute.

See the [embedded VM readiness ADR](./reference/adr/0005-embedded-vm-module-readiness.html).

## Roadmap boundary

The current implementation is intentionally server-rendered and explicit. It
does not promise client hydration, React hooks, framework globals, middleware
chains, or page-owned durable native resources. Planned work belongs in the
[architecture plan](./tsp-plan.html), while shipped behavior belongs in the
[runtime specification](./tsp-specification.html).
