# Current TSP Runtime Analysis

Date: 2026-08-25

## Scope

The repository currently contains two TSP execution paths. They must not be
treated as one implementation:

1. The v1 TypeScript/Bun server in `src/` is the default server started by
   `tsp.sh`.
2. The v2 native host in `bun/src/runtime/tsp/` is a side-by-side runtime
   started as `tspserver_v2`.

The persistent isolated Bun Worker migration should target the v2 native host
first. Replacing the v1 runtime at the same time would mix two unrelated
contracts and make rollback difficult.

## v1 startup and request flow

### Startup

`tsp.sh` resolves a Bun executable, then runs `src/main.ts`. The main module:

1. Parses CLI arguments and configuration.
2. Registers framework dependencies and bundled modules in
   `globalThis.__tspBuiltins`.
3. Creates the logger, session manager, Redis integration, file manager, and
   other framework services.
4. Starts `Bun.serve()` in normal mode.

The normal v1 process is persistent. It does not create a Bun process for each
request.

### Request flow

`handleRequest()` in `src/main.ts` performs the following work in the Bun
server process:

1. Reloads configuration when the config file changed.
2. Intercepts file-manager routes and the built-in HTMX asset.
3. Resolves the URL to a `.tsp` file and applies path/security checks.
4. Serves configured static files directly.
5. Parses JSON, URL-encoded, multipart, text bodies, uploaded files, cookies,
   and query parameters.
6. Builds the internal page context.
7. Calls `loadPage()` from `src/runtime/tsp.ts`.
8. Invokes the page default export or a fragment export.
9. Resolves HTMX fragments, merges cookies, renders React JSX, and converts
   redirects or `Response` objects to the final HTTP response.

The page module is therefore evaluated in the same persistent Bun process as
the HTTP listener in v1. This is the boundary that the new architecture must
not accidentally claim to have already migrated.

### v1 page loading and hot reload

`src/runtime/tsp.ts` owns the application-side page cache and in-flight load
deduplication. `src/runtime/module-graph.ts` discovers local dependencies,
tracks source stamps, propagates dirty state to owning pages, and preserves a
last-known-good page when a reload fails.

The TSP-enabled Bun fork owns the actual `.tsp`/TSX loader and canonical module
identity through `Bun.TSP.loadPage()`. On reload, the application passes the
union of old and current module paths for targeted invalidation.

### Existing v1 workers mode

The optional `config.workers` mode is a pre-start cluster mode. The parent
process starts multiple copies of `src/main.ts`, and each child binds the same
port with `reusePort`. This is process-level HTTP distribution, not a request
dispatcher with an IPC worker pool. It also does not move page execution out
of each child process.

## v2 startup and request flow

### Startup

`bun/src/runtime/tsp/bin/tspserver_v2.rs` is the v2 entry point. It:

1. Resolves the port and routes root.
2. Scans routes and builds the route table.
3. Prepares route exports and registers one `PageSlot` per route/method.
4. Resolves the Bun executable used by the JSC bridge.
5. Creates host-owned services and the memory or Redis session backend.
6. Builds the module graph.
7. Starts the filesystem watcher.
8. Starts the native TCP listener through `host::serve()`.

The v2 host is separate from v1. Its own source comments explicitly describe
v1 as the default working server and v2 as a side-by-side runtime.

### v2 request ownership

The Rust host owns:

- HTTP parsing and response writing;
- route lookup and method checks;
- static-file serving;
- request body limits;
- cookies and session resolution;
- service registry access;
- request cancellation and timeout monitoring;
- route/module registry and generation state.

The current v2 host creates a connection thread, builds a JSON-serializable
request context, and calls the JSC bridge for page execution. Requests with
query parameters, body data, dynamic parameters, or request-varying services
bypass the rendered-body generation cache. Stable body-less requests can use a
generation payload held by `PageRegistry`.

### v2 generation and reload flow

`PageRegistry` tracks the current generation, last-known-good generation,
dirty/building states, in-flight build coordination, and request-pinned
payloads. The watcher marks affected slots dirty; it does not directly replace
generation payloads. The next request performs the build and publishes the
candidate generation.

This gives v2 the important semantics that an in-flight request keeps the
generation it already pinned, while later requests can observe a newly
published generation.

## Current v2 JSC bridge

`bun/src/runtime/tsp/jsc_bridge.rs` is currently one-shot subprocess
execution:

1. Read and prepare the route source.
2. Rewrite local imports where required.
3. Transform TSX to JavaScript.
4. Write a temporary file.
5. Spawn the resolved Bun executable.
6. Pass the serialized context through the wrapper/environment.
7. Capture the response envelope from stdout.
8. On cancellation or timeout, send the abort marker, wait for the grace
   period, and hard-kill the child if it does not exit.
9. Remove the temporary file unless diagnostic retention is enabled.

This is the execution path frozen by
`docs/v2/adr/0001-subprocess-as-production-jsc.md`. A persistent Worker change
would therefore be a deliberate replacement of this bridge, not an extraction
from the v1 `src/main.ts` process.

## Relevant existing response contract

The worker boundary must preserve the current v2 observable behavior:

- status code and response headers;
- response body bytes, including non-UTF-8 data if supported later;
- 405 and `Allow` handling;
- 499-style client cancellation behavior;
- 504 timeout behavior;
- page and fragment responses;
- session cookie updates;
- diagnostics and stable TSP error codes;
- request body, headers, query, route parameters, services, and session data.

The current v2 bridge already has a response envelope and a cancellation
watchdog, so these are the most useful compatibility points for a persistent
worker protocol.

## Worker migration boundary

The safest first migration is:

```text
HTTP / routing / sessions / generations / watcher
                 Rust v2 host
                         |
                 Worker Manager
                         |
             one persistent Bun process
                         |
                 TSP page execution
```

The first worker milestone should keep the Rust host as the owner of HTTP,
routing, sessions, services, generation publication, and request deadlines.
Only the per-request page execution currently performed by
`jsc_bridge::execute_from_path()` should move behind a persistent IPC channel.

The initial implementation should use one worker and one application. Worker
pools, application groups, cgroups, namespaces, and multi-application routing
should follow only after the single-worker lifecycle and protocol are proven.

## Findings that affect the implementation plan

1. The migration target is the Rust v2 bridge, not the v1 `src/` server.
2. The current repository already has generation, session, timeout,
   cancellation, watcher, and response semantics that must remain stable.
3. A persistent worker must not reuse the current rendered-body cache as a
   substitute for module isolation; page execution state and response payload
   caching are separate concerns.
4. The worker protocol needs explicit framing, versioning, size limits,
   cancellation, worker readiness, shutdown, and crash semantics.
5. The worker manager must distinguish a worker crash from a request failure
   and must not retry a non-idempotent request without an explicit policy.
6. Cross-platform process supervision is a separate requirement from the
   Linux-only cgroup/namespace work.

## Recommended next task

Replace the original plan's broad extraction task with a v2-specific vertical
slice:

1. Define the worker wire envelope and lifecycle states.
2. Start one persistent Bun child during v2 host startup.
3. Execute one request through the worker and return the existing response
   envelope.
4. Add timeout/cancellation and child-crash replacement tests.
5. Keep the current subprocess bridge behind a feature flag as a fallback.

This preserves the existing v2 architecture while changing only execution
ownership, which is the stated goal of the persistent-worker migration.

## Implemented first vertical slice

The legacy external persistent slice remains available behind
`TSP_PERSISTENT_WORKER=1` for compatibility. It is not the v2.4 target.

## v2.4 embedded-worker vertical slice

The v2.4 path is enabled with `TSP_EMBEDDED_WORKER=1` and starts the declared
worker executable from `TSP_WORKER_BIN`. The master-side code links neither
Bun nor JSC. The worker executable is the Bun fork itself, entered with
`--tsp-worker`, and its Rust entry point initializes one embedded VM before
the request loop. It never launches `bun`, `bun.exe`, or a Bun grandchild.

The current v2.4 slice now includes:

- versioned binary `TSPW` framing with size and UTF-8 validation;
- Rust Worker Manager startup, READY handshake, heartbeat, shutdown, restart,
  and deadline-driven worker replacement;
- least-active WorkerPool scheduling with a configurable worker count and
  in-flight backpressure;
- Application -> WorkerGroup -> WorkerPool ownership types;
- embedded wrapper execution that reuses the same VM and clears the previous
  response globals on every request;
- request-level `ExecuteRequest` IPC carrying method, path, headers, body,
  generated script, and serialized Context;
- a reproducible `bun-debug.exe` build containing the worker entry point.

The transport keeps the same binary protocol on every platform: Unix Domain
Sockets on Linux/macOS and loopback TCP on Windows. The Windows transport is
only a platform adapter; it does not fall back to an external Bun process.

The pool can optionally recycle workers by request count, age, or Linux RSS.
Linux deployments may also opt into cgroup v2 `memory.max`, `cpu.max`, and
`pids.max` through an explicit cgroup root. These limits are disabled by
default and are not emulated on non-Linux platforms.

Filesystem reloads advance a master-side execution generation. Embedded local
imports receive that generation in their file URL, so a persistent VM loads
changed dependencies under a new module identity while in-flight requests
continue using their already prepared script.

The v2.4 request flow is intentionally Master + IPC:

```text
HTTP client -> Rust Master (HTTP / routing / Context)
                    |
             ExecuteRequest IPC
                    |
       persistent embedded Bun Worker VM
                    |
             ExecuteResponse IPC
```

The legacy external slice below remains for comparison:

- one persistent Bun child is started during v2 startup;
- generated `.tsx` wrappers are dynamically imported by that child;
- the existing `__TSP_OUT_V1__` response envelope is preserved;
- requests are serialized through the single worker;
- timeout/cancellation sends a worker cancel command;
- a worker that does not stop within the grace period is killed and replaced;
- the original one-shot bridge remains the default fallback.

The slice is intentionally not yet a worker pool, multi-application manager,
or OS resource sandbox. Those features should be added only after this
single-worker lifecycle is promoted from the feature flag and its protocol is
made explicit.
