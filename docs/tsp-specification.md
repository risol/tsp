# TSP runtime specification

Status: current repository contract. This document is normative for the
runtime shipped in this repository. The short compatibility summary is in
[`reference/contract.md`](./reference/contract.html).

The words **MUST**, **MUST NOT**, **SHOULD**, and **MAY** have their usual
normative meanings.

## 1. Runtime model

The request path is:

```text
HTTP request
  -> Rust host
  -> filesystem route table
  -> page preparation and generation cache
  -> embedded Bun worker
  -> handler result envelope
  -> HTTP response
```

The Rust host owns process lifetime, sockets, route discovery, request limits,
worker admission, sessions, service state, and generation publication. A Bun
worker evaluates application modules. A route generation is disposable; a
runtime service is not owned by a generation.

TSP is server-rendered. JSX is not React: there is no browser hydration,
client component state, reconciliation, or event-handler attachment.

## 2. Application layout

The conventional layout is:

```text
app/
├── pages/                 # .tsp route modules and local imports
├── public/                # static files, never executable routes
├── tsp.config.json        # optional runtime and service configuration
└── .tsp-types/            # generated declarations from tspserver typings
```

`TSP_ROUTES_DIR` and `TSP_PUBLIC_DIR` can change the first two roots.
`tsp.config.json` may set `publicDir` when the environment variable is not
provided.

## 3. Page modules

A `.tsp` file MUST be standard TypeScript/TSX. It MUST NOT use a template
language, frontmatter, decorators, or a special page class.

HTTP handlers are named exports:

```tsx
import { type Context } from "tsp:server";

export function GET(ctx: Context) {
  return <h1>Hello from {ctx.url.pathname}</h1>;
}
```

The supported handler names are `GET`, `POST`, `PUT`, `PATCH`, `DELETE`,
`HEAD`, `OPTIONS`, and the wildcard handler `ANY`. `ANY` receives every valid
HTTP method that has no more specific handler, including extension methods such
as `TRACE` or `BREW`. `HEAD` is synthesized from `GET` when needed. `OPTIONS`
returns the route's allowed methods when no page handler is provided. A
specific method handler takes precedence over `ANY`.

The host ignores a default export for dispatch, and `tspserver check` reports
it as a contract violation. Unknown exported functions are also rejected by
the check command. A `.tsp` route module MUST NOT import another `.tsp`
module; reusable code belongs in `.ts`, `.tsx`, `.js`, or `.jsx` modules.

## 4. Filesystem routing

Route paths are derived from filenames:

```text
pages/index.tsp             -> /
pages/login.tsp             -> /login
pages/users/index.tsp       -> /users
pages/users/[id].tsp        -> /users/:id
pages/files/[...path].tsp   -> /files/*
```

Dynamic names MUST be valid identifiers. Their values are strings in
`ctx.params`. Static routes have precedence over dynamic routes, and dynamic
routes have precedence over catch-all routes. Two routes with the same shape
are ambiguous and MUST fail route discovery.

## 5. Page configuration

A route MAY export a plain `config` object. The currently supported fields are:

```tsx
export const config = {
  methods: ["GET", "POST"],
  cache: "no-store",
  bodyLimit: 2 * 1024 * 1024,
  timeoutMs: 30_000,
};
```

- `methods` declares the exact set of exported page methods. If present, it
  MUST match the handler exports.
- `cache` is `no-store`, `private`, or `public`. When the handler does not
  set `Cache-Control`, the host supplies the matching default value.
- `bodyLimit` is a per-page byte limit and MUST NOT exceed the global limit.
- `timeoutMs` overrides the global request timeout for this page. `0` disables
  the watchdog for the page.

`auth` and arbitrary configuration fields are not part of the current runtime
contract. Configuration is statically inspected during route checking and
preparation; unsupported or malformed values MUST NOT be treated as valid
configuration.

## 6. Request context

Every handler receives a fresh request context:

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

The context is valid during the handler call only. Application code MUST NOT
retain it for later requests or place it in durable module state.

`ctx.request` follows the Web Request body model. Use `text()`, `json()`,
`formData()`, or `arrayBuffer()`. The body is single-consumption. The global
`TSP_MAX_BODY_BYTES` limit is checked before a page-specific `bodyLimit`.

`ctx.signal` is the cancellation signal supplied to route code. Async work
SHOULD pass it to downstream APIs where supported.

## 7. Cookies and sessions

Cookies are read from the request and writes are buffered by the worker. Each
write becomes a separate `Set-Cookie` header; application code SHOULD use the
cookie API instead of constructing that header manually.

```ts
ctx.cookies.get("theme");
ctx.cookies.has("theme");
ctx.cookies.set("theme", "dark", { path: "/", httpOnly: true });
ctx.cookies.delete("theme", { path: "/" });
```

Sessions are host-owned and survive page generation replacement:

```ts
const count = Number(ctx.session.get("count") ?? 0);
ctx.session.set("count", count + 1);
await ctx.session.regenerate();
await ctx.session.destroy();
```

Session values MUST be JSON-compatible. The default backend is in-memory;
`TSP_REDIS_URL` selects the Redis backend. A new session receives a `tsp_sid`
cookie when the host commits session state.

## 8. Handler results and errors

Handlers MAY return a `Response`, a rendered JSX node, a trusted HTML node, or
a top-level string. A top-level string is emitted as HTML. Arbitrary objects,
numbers, booleans, and `undefined` are invalid top-level results.

Use the helpers from `tsp:server`:

```ts
return json({ ok: true });
return text("accepted", { status: 202 });
return html(<main>...</main>);
return redirect("/login", 303);
return notFound();
```

`HttpError` can be thrown when a handler needs to stop early:

```ts
throw new HttpError(404, "User not found");
```

The host turns it into a response with the supplied status. Other handler
errors become HTTP 500 responses. With `TSP_DEVELOPMENT=1`, the response
includes a self-contained diagnostic HTML page; production responses hide
implementation details.

## 9. JSX rendering

The automatic JSX runtime is configured as follows:

```json
{
  "compilerOptions": {
    "jsx": "react-jsx",
    "jsxImportSource": "tsp"
  }
}
```

String children are HTML-escaped. `null`, `undefined`, and boolean children
render nothing. Numbers and bigints render as text. Arrays and promises are
flattened. JSX nodes and function components render recursively. Unsupported
objects fail with a rendering error.

Attributes follow HTML serialization rules: `false` and nullish values are
dropped, `true` emits a boolean attribute, and strings/numbers are escaped.
Function-valued and object-valued attributes are rejected. `className` maps to
`class`, and `htmlFor` maps to `for`.

Raw HTML is an explicit escape hatch:

```tsx
import { raw } from "tsp:html";

return <article>{raw(sanitizedHtml)}</article>;
```

The caller is responsible for making the value safe before passing it to
`raw`.

## 10. Fragments

A fragment is a named handler that returns a subtree or response:

```tsx
import { fragment } from "tsp:server";

export const userList = fragment(async () => <ul><li>One</li></ul>);

export function GET(ctx: Context) {
  return <a href={ctx.fragment("userList")}>Load users</a>;
}
```

The URL returned by `ctx.fragment()` is opaque and capability-protected. The
application MUST NOT construct or persist the internal fragment path itself.
The current shorthand uses `GET`; other fragment method forms are not part of
the current contract.

## 11. Built-in modules

Application code imports framework APIs explicitly:

```text
tsp:server  Context, response helpers, HttpError, fragment, raw,
            nanoid, zod, sql, util
tsp:html    raw
tsp:runtime runtime.version, runtime.env, runtime.development
```

The exact generated declaration files are produced by:

```text
tspserver typings --out .tsp-types
```

Optional native capabilities are lazy. Importing `tsp:server` MUST NOT
initialize a database or another native subsystem that the route does not use.
The `util.env` wrapper exposes individual `get` and `has` operations; it does
not expose a process-wide environment dump.

## 12. Generations and reloads

The host builds an immutable generation for a route. A source or dependency
change marks the affected route dirty; the next request builds and validates a
candidate. Publication is atomic:

```text
old generation -> build candidate -> validate -> publish candidate
                                      \-> failure: keep old generation
```

Requests pin the generation selected at dispatch, so an in-flight request is
not switched halfway through execution. A failed candidate leaves the last
known-good generation serving while the error is reported to diagnostics.

Runtime services, session storage, and worker coordination are outside page
generations and therefore survive reloads.

## 13. Static files and security

Files under `public/` are static assets. They are not parsed as route modules
and MUST NOT be placed in `pages/` as a substitute for static hosting.

The native host serves a matching public file for `GET` and `HEAD` requests;
the query string is ignored when resolving the file. `/` and directory URLs
ending in `/` serve `index.html` when present. The response includes a
content type inferred from the file extension, a byte-accurate
`Content-Length`, `X-Content-Type-Options: nosniff`, and
`Cache-Control: public, max-age=3600`. A missing public file falls through to
normal page routing, so an application route may handle that URL.

Public paths are URL-decoded before lookup, and traversal segments, NUL bytes,
Windows separators, and symlinks escaping the configured public root MUST NOT
be served. Static files take precedence over a page route with the same URL.

The host MUST prevent path traversal, reject ambiguous routes, enforce request
body limits before worker execution, and keep internal fragment capabilities
opaque. Credentials belong in the operator environment or host configuration,
not in page source or session values.

## 14. CLI contract

The packaged binary supports:

```text
tspserver                    run the HTTP server
tspserver check              validate routes and local imports
tspserver check --tsc        also run TypeScript checking
tspserver routes             list routes and exported methods
tspserver graph              print the module graph
tspserver typings --out DIR  write tsp:* declarations
tspserver --version          print the version
tspserver --help             print help
```

See [configuration](./configuration.html) for server settings and
[worker operations](./worker.html) for process and deployment behavior.

## 15. Compatibility boundary

The current runtime does not promise client hydration, React hooks, browser
event handlers, streaming responses, middleware chains, or page-owned durable
native services. Such features require an explicit contract change and tests.
