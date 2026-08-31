# TSP Context, PageConfig, and Builtins

> Phase 0 topic doc. Source of truth: `../tsp-specification.md` §7-§8,
> §10, §14, §16, §18-§19 and `../tsp-plan.md` §7-§8, §10, §14, §16,
> §18-§19, §60 freeze items 5, 6, 7, 8, 11.

`Context` is the per-request value the host passes to every page
handler. The frozen surface is small. This document is the
authoritative description; the `tsp:*` builtin modules are the
**only** way application code reaches the framework.

## Context shape (freeze 6)

```ts
interface Context<S = Services> {
  readonly request: TspRequest;
  readonly url: URL;
  readonly method: string;
  readonly params: Readonly<Record<string, string>>;
  readonly query: URLSearchParams;
  readonly cookies: Cookies;
  readonly session: Session;
  readonly services: S;
  readonly signal: AbortSignal;
  readonly route: RouteInfo;
  fragment(name: string, params?: Record<string, string>): string;
}
```

Notes:

- `request` follows the Web `Request` shape (method, headers,
  body via `text() / json() / formData() / arrayBuffer()`); it is
  not a `Request` instance. The body can be consumed exactly once
  (Web semantics).
- `params` is the dynamic segment values, ALWAYS strings
  (e.g. `params.id === "42"`). Convert at the call site.
- `query` is the URL query as `URLSearchParams`.
- `services` is the host's Service Registry view (see below).
- `signal` is `AbortSignal`: aborted on client disconnect, request
  timeout, server shutdown, or explicit cancellation.
- `route` carries the route metadata (path, methods, source file,
  current generation id).
- `fragment(name, params?)` returns the URL for the named
  fragment. The URL shape is host-defined and MUST NOT be
  hard-coded by application code.

The generated URL includes an opaque per-process capability. The internal
dispatch endpoint rejects missing or invalid capabilities; application code
must only use the value returned by `ctx.fragment()`.

`Context` is per-request. It is owned by the native runtime and
becomes invalid after the handler returns. Any subsequent access
raises `TSP RuntimeError: Request context is no longer active`
(plan §8.3). This prevents accidental lifetime leaks via
`globalThis.ctx = ctx`.

## TspRequest (freeze 6, supporting)

```ts
interface TspRequest {
  readonly method: string;
  readonly headers: Headers;
  readonly url: string;
  text(): Promise<string>;
  json<T = unknown>(): Promise<T>;
  formData(): Promise<FormData>;
  arrayBuffer(): Promise<ArrayBuffer>;
}
```

Web `Request` semantics apply: the body can be consumed exactly
once, and the global `bodyLimit` (`tsp.toml [http]
max_body_size`) applies first; per-page `config.bodyLimit` can only
lower it.

For multipart bodies, the host parses them natively. The
`FormData` value carries `UploadedFile` entries that hold a
`tempPath` plus `arrayBuffer()` / `stream()` accessors so the
application can stream large files without loading them entirely
into the JS heap (plan §9.3).

## Cookies and Session

```ts
interface Cookies {
  get(name: string): string | undefined;
  set(name: string, value: string, options?: CookieOptions): void;
  delete(name: string, options?: CookieOptions): void;
}
```

`Cookies.set` writes to the response via the host's response
builder; the application never sets the `Set-Cookie` header
directly. This prevents multiple Set-Cookie headers from clobbering
each other (plan §19).

```ts
interface Session {
  readonly id: string | null;
  get<T = unknown>(key: string): T | undefined;
  set(key: string, value: unknown): void;
  delete(key: string): void;
  clear(): void;
  regenerate(): Promise<void>;
  destroy(): Promise<void>;
}
```

Sessions hold JSON-compatible data only. Functions, JSC object
references, native pointers, and page module objects are
**rejected** (plan §18.2). This keeps the session independent of
the page generation -- a reload of the page does not invalidate
the session.

## Service Registry (freeze 6, supporting)

```ts
interface ServiceRegistry {
  logger: Logger;
  sessionStore: SessionStore;
  db: { main: DbClient; /* ... */ };
  redis: { main: RedisClient; /* ... */ };
  // Application-defined services, declared in tsp.config.ts
  // or tsp.toml
}
```

`singleton` services live for the process lifetime; `request`
services live for the request. The host owns the actual resources
(DB pool, Redis connection, etc.); the page module sees a
*reference* through `ctx.services` (plan §17.3).

`ctx.services.db` is a `ServiceHandle` (a numeric id into the
host's service registry). The page does not own the pool;
reloading the page does not destroy the pool (plan §3.4).

## HandlerResult helpers (freeze 5, 6)

```ts
// from "tsp:server"
function json(value, init?): Response;
function redirect(location, status?): Response;
function text(value, init?): Response;
function html(node, init?): Response;
function notFound(): Response;

class HttpError extends Error {
  constructor(status: number, message: string, init?: ResponseInit);
}
```

`new HttpError(404, "not found")` thrown from a handler is caught by
the host and turned into a 404 response. The `init` parameter
mirrors `ResponseInit`.

## PageConfig (freeze 11)

```ts
interface PageConfig {
  auth?: "none" | "optional" | "required";
  cache?: "no-store" | "private" | "public";
  bodyLimit?: number;       // bytes; cannot exceed global hard limit
  timeoutMs?: number;       // request-level timeout
  methods?: readonly HttpMethod[];
}

export const config = {
  auth: "required",
  cache: "no-store",
  bodyLimit: 2 * 1024 * 1024,
  timeoutMs: 30000,
} satisfies PageConfig;
```

`config` MUST be a plain object literal; it is read at module
evaluate time. Async initialisers like `export const config = await
fetch(...)` are rejected. The host validates `config` against
`PageConfig` after the module evaluates and refuses to publish a
generation with an invalid config (plan §48).

## fragment() (freeze 7)

```ts
import { fragment } from "tsp:server";

// Recommended: explicit method
export const list = fragment({
  method: "GET",
  async handler(ctx) {
    const users = await ctx.services.users.list();
    return <UserList users={users} />;
  },
});

// Shorthand: defaults to method: "GET"
export const list = fragment(async (ctx) => { ... });
```

The fragment is exposed at an opaque URL. The application gets
the URL via `ctx.fragment("list")` (and, in a future slice, via
`fragmentUrl(list)`). The host never publishes a
`/__tsp/fragment/...` path as part of the public contract
(plan §14.3).

current contract default method is `GET`. `POST` fragments are allowed via the
explicit `{ method: "POST", handler }` form. `HEAD` and `OPTIONS`
on fragments follow the same synthesise-from-GET rule as page
handlers.

## `tsp:*` builtin modules (freeze 8)

Three modules, no more for current contract:

```text
tsp:server
  Context, PageConfig, HttpError
  fragment, json, redirect, text, html, notFound

tsp:html
  HtmlNode, TrustedHtml
  raw, escape

tsp:runtime
  runtime.version
  runtime.env
  runtime.development
```

`tsp:runtime` does NOT expose the host's internal API surface --
no service registry handles, no JSC value inspectors, no module
loaders. It is metadata only (plan §16.3).

There is no `globalThis.Page`, no `globalThis.Fragment`, no
`globalThis.__tspBuiltins` (plan §16.4). Application code MUST
import from `tsp:*` explicitly.

## Cancellation and timeout (freeze 6, supporting)

`ctx.signal` aborts when:

- the client disconnects,
- the request hits the `timeoutMs` ceiling,
- the server is shutting down, or
- the application explicitly cancels.

Native services SHOULD accept `ctx.signal` and propagate
cancellation. Page-handler timeout is `TSP3008 RequestTimeout`;
if the JS promise never resolves, the host enforces an
interrupt (slice 9+ specifies the JSC hard-interruption strategy
in detail; plan §53).

## What is NOT in current contract

- No `(req, res, next)` middleware chain (plan §44). current contract uses
  `PageConfig.auth` + an auth service hook on `ctx.services.auth`.
- No per-component error boundary. Errors bubble to the
  handler-level error path; the host's built-in 500 page is the
  current contract default (plan §12.4).
- No streaming responses. The handler's return is fully rendered
  before the response is sent (plan §13).
- No `import.meta.env` / build-time env injection. Runtime
  metadata comes from `tsp:runtime`; build-time injection is
  outside the current contract surface.
