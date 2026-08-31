# TSP Embedded Bun Worker

TSP embedded-worker ships one `tspserver` executable. It runs a Rust Master and
persistent Bun Worker child processes connected by
the versioned `TSPW` IPC protocol. The Master owns HTTP, routing, request
context, scheduling, deadlines, and worker replacement. Each Worker owns one
embedded Bun VM and never launches a Bun grandchild.

## Build

The root workflow builds the single runtime executable:

```bash
./tsp.sh build
```

The packaged runtime, default `tsp.config.json`, pages, and public assets are
written to `dist/tspserver/`. No separate runtime manifest is required because
the Master creates worker children automatically from the packaged executable.

## Run

```bash
TSP_PORT=9137 \
TSP_ROUTES_DIR="$PWD/tests/smoke/pages" \
./tsp.sh start
```

On Linux the Master pre-forks worker children before it starts request and
watcher threads. On Windows it self-spawns the same executable in an internal
`--tsp-worker` mode. Neither deployment needs a separate Bun executable.

## Configuration

The package includes the root `tsp.config.json` as its default configuration.
Use `tspserver --config <PATH>` (or `-c <PATH>`) to select another file. The
resolution order is the command-line flag, `TSP_CONFIG`, then
`./tsp.config.json`. `tsp.sh start` and `tsp.sh dev` automatically use the
repository-root configuration unless `TSP_CONFIG` or `--config` overrides it.

## Operational settings

- `TSP_WORKER_COUNT`: number of isolated Worker processes.
- `TSP_WORKER_MAX_IN_FLIGHT`: bounded Master-side admission capacity.
- `TSP_WORKER_MAX_REQUESTS`: recycle a Worker after a request count.
- `TSP_WORKER_MAX_AGE_MS`: recycle a Worker after an age limit.
- `TSP_WORKER_MAX_MEMORY_BYTES`: recycle on Linux RSS.
- `TSP_CGROUP_ROOT`: explicit Linux cgroup v2 parent for optional limits.
- `TSP_WORKER_MEMORY_MAX`, `TSP_WORKER_CPU_MAX`, `TSP_WORKER_PIDS_MAX`:
  optional cgroup v2 limits.

Resource limits are disabled by default. A cgroup root must be explicitly
configured; the application never writes to an inferred or system-wide cgroup.

## Verification

The smoke test checks repeated requests, Prometheus metrics, and hot reload:

```bash
./tsp.sh test:smoke
```

Worker Manager integration tests use a protocol-only test Worker and cover
reuse, heartbeat, crash replacement, timeout replacement, pool admission, and
the platform transport adapter:

```bash
cargo test -p bun_runtime_tsp --test worker_integration --no-fail-fast
```

For latency baselines, run the packaged executable directly:

```bash
./scripts/benchmark-tspserver.sh \
  dist/tspserver/tspserver \
  tests/smoke/pages \
  50
```
