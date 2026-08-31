# Troubleshooting

## The server cannot find the routes directory

Check `TSP_ROUTES_DIR` and run:

```bash
tspserver routes
```

The directory must exist and contain `.tsp` route modules. A missing route root
is a startup error, not an empty application.

## A route is not found

Confirm the file mapping:

```text
pages/index.tsp       -> /
pages/users.tsp       -> /users
pages/users/[id].tsp  -> /users/:id
```

Then inspect the route table with `tspserver routes --json`. Static routes win
over dynamic routes. A malformed dynamic segment or ambiguous route prevents
the route table from being accepted.

## The route returns 405

The page does not export the requested method. Export one of `GET`, `POST`,
`PUT`, `PATCH`, or `DELETE`, or make `config.methods` match the exported set.
`HEAD` and `OPTIONS` have host-provided behavior described in the
[specification](./tsp-specification.html).

## `tspserver check` fails

Run the basic check first, then the TypeScript check:

```bash
tspserver check
tspserver check --tsc --no-color
```

Common causes are an unknown exported function, a default export, a
`config.methods` mismatch, a local import outside the application root, or a
missing generated declaration directory.

## The response is 500

Set `TSP_DEVELOPMENT=1` locally to receive the self-contained diagnostic HTML
page. Do not enable it on a public production endpoint. The error code prefix
usually identifies the phase:

| Prefix | Phase |
| --- | --- |
| `TSP1xxx` | route or filesystem configuration |
| `TSP2xxx` | request parsing or limits |
| `TSP3xxx` | page preparation, JSX, worker, or handler execution |

An arbitrary object returned directly from a handler is invalid. Use JSX,
`Response`, or a helper such as `json`, `text`, `html`, `redirect`, or
`notFound`.

## Changes are not visible

Run `tspserver graph` and verify that the changed file is in the module graph.
The watcher tracks supported local source files. A failed candidate keeps the
last known-good generation active; inspect stderr for the candidate error.

When multiple workers are enabled, the invalidation file must be writable and
shared by the workers. See [worker operations](./worker.html).

## Requests are rejected with 413

The body exceeds `TSP_MAX_BODY_BYTES` or the route's smaller `config.bodyLimit`.
Check the effective values and lower the upload size or raise the global limit
carefully.

## Requests time out

Check `TSP_TIMEOUT_MS` and the route's `config.timeoutMs`. A value of `0`
disables the watchdog. Long-running application work should observe
`ctx.signal`; disabling the timeout is an operational decision, not a fix for
work that cannot be cancelled.

## The worker fails on startup

Confirm the packaged runtime exists and run the embedded-worker smoke test:

```bash
./tsp.sh test:smoke
```

On Windows, use the same redirected, non-interactive stdio shape as CI. Set
`TSP_WORKER_STARTUP_TRACE=1` only while diagnosing startup stage boundaries;
the trace is intentionally environment-gated.

For allocator, Windows handle, VM role, and module-readiness failures, consult
the [ADR index](./reference/adr/) and [bug index](./reference/bugs/) before
changing native lifecycle code.
