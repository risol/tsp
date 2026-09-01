# TSP current contract

This is the compact compatibility checklist for application authors. The
detailed rules live in [`tsp-specification.md`](../tsp-specification.html).
This page contains current behavior only; roadmap ideas belong in
[`tsp-plan.md`](../tsp-plan.html).

## Runtime boundary

- The Rust host owns HTTP, routing, request limits, sessions, services,
  worker processes, and generation publication.
- Embedded Bun workers evaluate `.tsp` modules.
- Page generations are replaceable. Host-owned sessions and services survive a
  route reload.
- TSP renders HTML on the server. It is not a client React runtime.

## `.tsp` modules

- `.tsp` files are standard TypeScript/TSX.
- Supported page handlers are named `GET`, `POST`, `PUT`, `PATCH`, `DELETE`,
  `HEAD`, `OPTIONS`, and `ANY` (the wildcard handler).
- `ANY` handles valid extension methods that have no more specific handler.
- `HEAD` is synthesized from `GET` when no separate handler is used.
- `OPTIONS` reports the route's allowed methods when no page handler is used.
- A `.tsp` file must not import another `.tsp` file.
- Default exports and unknown exported functions fail `tspserver check`.
- Reusable code belongs in `.ts`, `.tsx`, `.js`, or `.jsx` modules.

## Routing

```text
pages/index.tsp            -> /
pages/users.tsp             -> /users
pages/users/index.tsp       -> /users
pages/users/[id].tsp        -> /users/:id
pages/files/[...path].tsp   -> /files/*
```

Static routes take precedence over dynamic routes, and dynamic routes take
precedence over catch-all routes. Ambiguous routes fail discovery. Static files
belong under `public/`.

## Handler results

Handlers may return:

- a JSX node;
- a trusted HTML node from `raw`;
- a top-level HTML string; or
- a standard `Response`.

Arbitrary objects, numbers, booleans, and `undefined` are invalid top-level
results. Use `json`, `text`, `html`, `redirect`, or `notFound` for explicit
responses. `HttpError` maps a thrown status to an HTTP response.

## Context

The request-scoped context exposes:

```text
request, url, method, path, params, query, headers,
cookies, session, services, signal, fragment()
```

The request body is single-consumption. Dynamic parameters are strings.
`ctx.method` is an uppercase `string`, not a TypeScript enum or closed union;
this preserves extension methods for `ANY`. Context values must not be retained
after the handler returns.

## Page configuration

```tsx
export const config = {
  methods: ["GET"],
  cache: "no-store",
  bodyLimit: 1_048_576,
  timeoutMs: 30_000,
};
```

`methods` must match exported page handlers. `cache` accepts `no-store`,
`private`, or `public`. `bodyLimit` is in bytes and cannot exceed the global
limit. `timeoutMs` is in milliseconds; `0` disables the page watchdog.

## Built-ins

| Module | Current surface |
| --- | --- |
| `tsp:server` | response helpers, `HttpError`, `fragment`, `raw`, `nanoid`, `zod`, `sql`, `util` |
| `tsp:html` | `raw` |
| `tsp:runtime` | `runtime.version`, `runtime.env`, `runtime.development` |

Generate the exact declaration surface with `tspserver typings --out .tsp-types`.

## Rendering and safety

- String children and attributes are escaped.
- Nullish and boolean children render empty.
- `className` and `htmlFor` map to HTML `class` and `for`.
- Function-valued and object-valued attributes are rejected.
- `raw` is an explicit trust boundary; callers must sanitize its input.
- Client interaction is expressed through forms, requests, and fragment URLs,
  not server-side event handlers.

## Sessions and services

Sessions expose `get`, `set`, `delete`, `clear`, `regenerate`, and `destroy`.
Values must be JSON-compatible. Memory is the default backend; Redis is
selected with `TSP_REDIS_URL`.

Services are host-owned snapshots exposed through `ctx.services`. Built-in
services include logger, session, and time. Config-driven service descriptors
are loaded from `tsp.config.json`.

## Explicit non-goals

The current contract does not include client hydration, React hooks, framework
globals, middleware chains, streaming responses, or page-owned durable native
services.
