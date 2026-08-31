# Context and server APIs

Every page handler receives a fresh request context. The host creates it for
one request and commits cookie/session writes after the worker returns.

## Context shape

```ts
interface Context {
  readonly request: Request;
  readonly url: URL;
  readonly method: string;
  readonly path: string;
  readonly params: Readonly<Record<string, string>>;
  readonly query: URLSearchParams;
  readonly headers: Readonly<Record<string, string>>;
  readonly cookies: Cookies;
  readonly session: Session;
  readonly services: Readonly<Record<string, unknown>>;
  readonly signal: AbortSignal;
  fragment(name: string, params?: Record<string, string>): string;
}
```

`ctx.params` values are always strings. `ctx.query` is the URL's
`URLSearchParams`. `ctx.signal` is the request cancellation signal. Do not
retain `ctx` or its request-scoped values after the handler returns.

## Request body

The request uses the Web `Request` interface:

```ts
const text = await ctx.request.text();
const payload = await ctx.request.json<MyPayload>();
const form = await ctx.request.formData();
const bytes = await ctx.request.arrayBuffer();
```

Only one body reader should be consumed for a request. The host enforces
`TSP_MAX_BODY_BYTES` before dispatch and can apply a smaller page-level
`config.bodyLimit`.

## Cookies

```ts
interface Cookies {
  get(name: string): string | undefined;
  has(name: string): boolean;
  set(name: string, value: string, options?: CookieOptions): void;
  delete(name: string, options?: CookieOptions): void;
}
```

Supported options include `path`, `domain`, `maxAge`, `expires`, `httpOnly`,
`secure`, and `sameSite`. Writes are buffered and emitted as separate
`Set-Cookie` lines, including when the handler returns JSX.

## Sessions

```ts
interface Session {
  readonly id: string;
  get<T = unknown>(key: string): T | undefined;
  has(key: string): boolean;
  set(key: string, value: unknown): void;
  delete(key: string): void;
  clear(): void;
  regenerate(): Promise<void>;
  destroy(): Promise<void>;
}
```

Session writes are committed by the host. Values are converted to the
JSON-compatible session domain; functions and runtime object references do not
belong in a session. Memory is the default backend. Set `TSP_REDIS_URL` to
use Redis for session storage.

## Responses

```ts
json(value: unknown, init?: ResponseInit): Response;
text(value: unknown, init?: ResponseInit): Response;
html(value: unknown, init?: ResponseInit): Response;
redirect(location: string, status?: 301 | 302 | 303 | 307 | 308): Response;
notFound(): Response;
```

The helpers set a suitable content type when the caller has not supplied one.
Use `HttpError` for early status-bearing failures:

```ts
import { HttpError } from "tsp:server";

if (!user) throw new HttpError(404, "User not found");
```

## Fragments

```tsx
import { fragment } from "tsp:server";

export const list = fragment(async () => <ul><li>One</li></ul>);

export function GET(ctx: Context) {
  return <a href={ctx.fragment("list")}>Load list</a>;
}
```

`ctx.fragment()` returns an opaque, capability-protected URL. Do not construct
the internal path manually. The current `fragment(handler)` form uses `GET`.

## Services

`ctx.services` is a read-only request snapshot of host-owned services. Built-in
descriptors include logger, session, and time; configuration can add service
descriptors such as counters, key/value maps, feature flags, and rate limits.

```ts
ctx.services.logger.info("user loaded");
const time = ctx.services.time;
```

Service wrapper identity is not stable across requests. A service must not
store a page module, request context, or native pointer.

## Built-in modules

```ts
import { json, util, zod } from "tsp:server";
import { raw } from "tsp:html";
import { runtime } from "tsp:runtime";
```

`util` exposes the supported, wrapped Bun helpers. Environment access is
limited to `get` and `has`; pages cannot dump the whole process environment.
Optional native APIs are lazy and must not initialize unused subsystems merely
because `tsp:server` was imported.
