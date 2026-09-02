# Configuration reference

TSP reads runtime settings from environment variables and `tsp.config.json`.
The packaged binary also accepts `--config, -c <PATH>` for the JSON
configuration file.

## Paths and server

| Variable | Default | Meaning |
| --- | --- | --- |
| `TSP_PORT` | `3000` | HTTP listener port |
| `TSP_ROUTES_DIR` | `pages` | `.tsp` route root |
| `TSP_PUBLIC_DIR` | `public` | static asset root |
| `TSP_CONFIG` | `tsp.config.json` | runtime and service configuration path |
| `TSP_APPLICATION_NAME` | `main` | application registry name |
| `TSP_DEVELOPMENT` | `0` | expose detailed HTML errors when set to `1` |

The repository wrapper `./tsp.sh start` supplies `TSP_PORT=9000`, the current
checkout's route and public directories, and `TSP_CONFIG=tsp.config.json`.

## Request limits

| Variable | Default | Meaning |
| --- | --- | --- |
| `TSP_TIMEOUT_MS` | `30000` | request watchdog; `0` disables it |
| `TSP_MAX_BODY_BYTES` | `1048576` | global request body cap |

## Image processing limits

`Image` is available from `tsp:server`. These limits apply inside each
embedded worker process and protect image decoding from oversized compressed
inputs, decompression bombs, and excessive concurrent image work.

| Variable | Default | Meaning |
| --- | ---: | --- |
| `TSP_IMAGE_MAX_INPUT_BYTES` | `268435456` | maximum encoded image input size |
| `TSP_IMAGE_MAX_PIXELS` | `268402689` | maximum decoded width × height |
| `TSP_IMAGE_MAX_CONCURRENT_TASKS` | `4` | maximum image pipelines in flight per worker |

A route may lower the body cap with `config.bodyLimit` or override the timeout
with `config.timeoutMs`. The global body limit is always checked first.

## Workers

| Variable | Meaning |
| --- | --- |
| `TSP_WORKER_COUNT` | number of embedded worker processes; default `1` |
| `TSP_WORKER_MAX_IN_FLIGHT` | admission limit per worker pool; default is `2 * count` |
| `TSP_WORKER_MAX_REQUESTS` | recycle a worker after this many requests |
| `TSP_WORKER_MAX_AGE_MS` | recycle a worker after this age |
| `TSP_WORKER_MAX_MEMORY_BYTES` | recycle a worker at this RSS limit |
| `TSP_INVALIDATION_FILE` | shared invalidation log for worker processes |
| `TSP_WORKER_MEMORY_MAX` | optional Linux cgroup memory limit |
| `TSP_WORKER_CPU_MAX` | optional Linux cgroup CPU limit |
| `TSP_WORKER_PIDS_MAX` | optional Linux cgroup process limit |
| `TSP_CGROUP_ROOT` | Linux cgroup v2 parent directory |

Limits are operational safeguards. They do not change the `.tsp` module
contract.

## Sessions and worker executable

| Variable | Meaning |
| --- | --- |
| `TSP_REDIS_URL` | select the Redis session backend |
| `TSP_WORKER_BIN` | worker executable used by the host when applicable |
| `TSP_EMBEDDED_WORKER` | enable the embedded worker path |
| `TSP_BUN_BIN` | Bun executable used by `tsp.sh build:worker` |
| `TSP_RUST_TOOLCHAIN` | Rust toolchain used by `tsp.sh` |
| `TSP_TYPINGS_DIR` | default output directory for `tsp.sh typings` |

Without `TSP_REDIS_URL`, sessions use the in-process memory backend. Redis
failure is reported by the host and the runtime falls back to memory during
startup.

## `tsp.config.json`

The packaged default file is valid JSON and includes every server setting with
its default value. `services` is an empty custom-service registry by default;
the built-in `logger`, `session`, and `time` services are registered by the
host separately:

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 3000,
    "routesDir": "./pages",
    "publicDir": "./public",
    "publicPrefix": "/static",
    "timeoutMs": 30000,
    "maxBodyBytes": 1048576,
    "development": false
  },
  "image": {
    "maxInputBytes": 268435456,
    "maxPixels": 268402689,
    "maxConcurrentTasks": 4
  },
  "worker": {
    "count": 1,
    "maxInFlight": 2,
    "maxRequests": 0,
    "maxAgeMs": 0,
    "maxMemoryBytes": 0
  },
  "application": {
    "name": "main"
  },
  "session": {
    "redisUrl": null
  },
  "services": {}
}
```

Relative `server.routesDir` and `server.publicDir` paths are resolved relative
to the configuration file. `server.publicPrefix` controls the URL path where
the public directory is mounted. For example, with `publicDir: "./www/static"`
and `publicPrefix: "/static"`, the file `./www/static/app.css` is served at
`/static/app.css`. The prefix must be an absolute, plain URL path and is
matched on segment boundaries. A missing prefix mounts the directory at `/`.
Environment variables override their corresponding configuration values. The
`services` object may be omitted when no config-driven services are needed.

Config-driven services are named entries under `services`. The current host
supports the service kinds implemented by the runtime, including `counter`,
`kv`, `feature_flag`, and `rate_limit`. Invalid service kinds fail fast during
startup. Service configuration changes are watched in development; a
malformed new snapshot leaves the previous valid snapshot in place.
Server and worker settings are read at startup; restart the server after
changing those settings.

## Page configuration

Route-local configuration is declared in the `.tsp` file:

```tsx
export const config = {
  methods: ["GET"],
  cache: "no-store",
  bodyLimit: 256 * 1024,
  timeoutMs: 10_000,
};
```

`methods` must match the page's exported handlers. `cache` accepts
`no-store`, `private`, or `public`. `bodyLimit` is in bytes. `timeoutMs` is in
milliseconds.

## Configuration precedence

For the JSON service configuration path:

```text
--config / -c -> TSP_CONFIG -> ./tsp.config.json
```

For server settings, the precedence is:

```text
environment variable -> tsp.config.json -> built-in default
```

Use `tspserver --help` as the final authority for the binary shipped in a
release.

## Generated typings

Generate declarations for the built-in modules:

```bash
tspserver typings --out .tsp-types
```

The command writes `tsp-server.d.ts`, `tsp-html.d.ts`, and
`tsp-runtime.d.ts`. Do not edit these files by hand; regenerate them when the
runtime changes.
