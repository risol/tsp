# Embedded worker operations

The packaged TSP runtime is one `tspserver` executable. The Rust master starts
and supervises embedded Bun worker processes; deployment does not require a
separate worker binary.

## Process model

```text
tspserver master
  ├── worker 1: JavaScript runtime + page modules
  ├── worker 2: JavaScript runtime + page modules
  └── ...
```

The master owns HTTP, routing, request admission, deadlines, worker
replacement, sessions, and services. Each worker owns its process-local module
state and evaluates one request at a time up to the configured admission
limit.

The worker protocol is native and private. Route code must not read standard
input to implement cancellation or worker control; embedded workers receive
control messages through the native protocol.

## Run a packaged server

From an application directory containing `pages/` and `public/`:

```bash
TSP_PORT=9137 \
TSP_ROUTES_DIR="$PWD/pages" \
TSP_PUBLIC_DIR="$PWD/public" \
./tspserver
```

On Windows PowerShell:

```powershell
$env:TSP_PORT = "9137"
$env:TSP_ROUTES_DIR = "$PWD\pages"
$env:TSP_PUBLIC_DIR = "$PWD\public"
.\tspserver.exe
```

The binary defaults to port `3000`; the repository wrapper defaults to `9000`.

## Worker settings

Set `TSP_WORKER_COUNT` to choose the number of worker processes. The default is
`1`. `TSP_WORKER_MAX_IN_FLIGHT` bounds concurrent requests per pool and
defaults to twice the worker count.

Optional recycling settings are:

```text
TSP_WORKER_MAX_REQUESTS
TSP_WORKER_MAX_AGE_MS
TSP_WORKER_MAX_MEMORY_BYTES
```

On Linux, optional cgroup v2 limits are configured with `TSP_CGROUP_ROOT`,
`TSP_WORKER_MEMORY_MAX`, `TSP_WORKER_CPU_MAX`, and `TSP_WORKER_PIDS_MAX`.

## Development and reload

The server watches the route tree and supported local dependencies. A change
builds a candidate page generation and publishes it only after validation.
Requests already in flight remain on their original generation. If the build
fails, the last known-good generation remains available.

With multiple workers, `TSP_INVALIDATION_FILE` can be used as the shared
cross-worker invalidation log. Ensure the path is writable and visible to all
workers.

## Sessions

Sessions use the in-process memory backend by default. Set `TSP_REDIS_URL` to
select the Redis backend. Session data is host-owned and survives page reloads;
page code accesses it through `ctx.session`.

## Diagnostics

Use the following commands before starting a deployment:

```bash
tspserver check --tsc
tspserver routes --json
tspserver graph --json
```

For a local checkout, run the embedded-worker smoke test:

```bash
./tsp.sh test:smoke
```

Set `TSP_DEVELOPMENT=1` for detailed page error HTML during local debugging.
Set `TSP_WORKER_STARTUP_TRACE=1` only when diagnosing native startup stages;
the trace is environment-gated and should not be enabled by default.

## Platform notes

Windows worker smoke tests must use redirected, non-interactive standard I/O,
matching CI. The packed Windows `Fd` representation, process-relative VM
roles, and module-readiness lifecycle are documented in the
[architecture records](./reference/adr/).

For allocator or worker crashes, consult the verified [bug records](./reference/bugs/)
before adding a runtime workaround.
