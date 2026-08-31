# TSP Specification

**Status:** Draft 0.1  
**Target:** TSP current contract
**Compatibility:** Intentionally incompatible with legacy TSP
**Primary language:** TypeScript / TSX  
**Host runtime:** Native Rust host integrated with Bun/JavaScriptCore  
**Document type:** Normative language and runtime specification

---

## 0. Purpose

This document defines the normative contract of TSP.

It specifies:

- the meaning of a `.tsp` file;
- legal module exports;
- file-system routing;
- module resolution boundaries;
- request and response behavior;
- the TSP JSX runtime;
- component semantics;
- fragment semantics;
- services, cookies, and sessions;
- page generations and hot reload behavior;
- error behavior;
- runtime lifecycle guarantees;
- minimum tooling and conformance requirements.

This document intentionally does **not** describe the migration path from legacy TSP and does **not** require compatibility with v1 APIs or file formats.

Implementation plans, repository layout, milestones, and experimental alternatives belong in the separate TSP Plan document.

---

# 1. Normative language

The key words **MUST**, **MUST NOT**, **REQUIRED**, **SHALL**, **SHALL NOT**, **SHOULD**, **SHOULD NOT**, **RECOMMENDED**, **MAY**, and **OPTIONAL** are to be interpreted as normative requirements.

A conforming TSP runtime MUST implement every requirement marked MUST or MUST NOT.

A conforming application MAY rely on every behavior defined as REQUIRED by this specification.

---

# 2. Product definition

TSP is a server-side web application runtime with the following architectural model:

```text
Native TSP Host
    Rust
      │
      ├── HTTP server
      ├── router
      ├── path/security validation
      ├── static files
      ├── configuration
      ├── cookies/session
      ├── service registry
      ├── module graph
      ├── file watcher
      ├── generation manager
      └── page lifecycle
      │
      ▼
JavaScriptCore VM
      │
      ├── .tsp modules
      ├── .ts/.tsx application modules
      ├── JavaScript dependencies
      └── TSP JSX runtime bindings
```

TSP is **not** defined as a JavaScript web framework that happens to use a Rust executable.

The native runtime owns the server and long-lived application lifecycle.

JavaScriptCore is an execution VM for application TypeScript/JavaScript code.

---

# 3. Core invariants

The following invariants are fundamental TSP rules.

## 3.1 `.tsp` is TSX

A `.tsp` file MUST use ordinary TypeScript/TSX lexical and syntactic rules.

TSP MUST NOT introduce a second template grammar such as:

```text
<script>
<template>
<style>
```

TSP MUST NOT introduce PHP/JSP-style embedded code delimiters.

The loader MAY enforce semantic restrictions beyond TypeScript syntax, but it MUST NOT require a custom source-language parser before TypeScript/TSX parsing.

## 3.2 `.tsp` is a route root

A `.tsp` module represents one route entry point and one page-generation root.

A `.tsp` module MUST NOT be used as a reusable library module.

## 3.3 Persistent state belongs outside page generations

Long-lived resources MUST NOT be owned by a disposable page generation.

Examples include:

- database pools;
- Redis pools/connections;
- session stores;
- loggers;
- runtime configuration state;
- worker coordination;
- file watchers;
- native service registries.

## 3.4 Page generations are disposable

Application page modules and their reloadable dependencies MUST be treated as replaceable generations.

Application correctness MUST NOT depend on a page module remaining alive forever.

## 3.5 JSX is not React

TSP JSX MUST be defined by TSP's own JSX runtime contract.

React MUST NOT be required by the TSP core runtime.

Applications MAY use React as a separate library if an adapter exists, but React semantics are not part of the TSP language contract.

---

# 4. Application layout

A TSP application has an application root.

The default layout is:

```text
app/
├── tsp.toml
├── routes/
│   ├── index.tsp
│   └── ...
├── components/
│   └── ...
├── lib/
│   └── ...
└── public/
    └── ...
```

The following directories have special meaning by default:

- `routes/` — route-root `.tsp` files;
- `public/` — static files.

The runtime MAY allow these locations to be changed through configuration.

---

# 5. `.tsp` source format

## 5.1 Encoding

A `.tsp` source file MUST be UTF-8.

A UTF-8 BOM MAY be accepted and ignored.

Invalid UTF-8 MUST produce a source-load error.

## 5.2 Syntax

A `.tsp` file MUST parse as TypeScript with JSX enabled.

The file MAY contain:

- imports;
- local type declarations;
- local variables;
- local functions;
- local components;
- top-level await;
- legal TSP exports defined in this specification.

## 5.3 No default export

A `.tsp` module MUST NOT expose a runtime `default` export.

The following is invalid:

```tsx
export default function Page() {
  return <h1>Hello</h1>;
}
```

The runtime MUST reject such a module during export validation.

## 5.4 Legal runtime exports

The only legal runtime exports of a `.tsp` module are:

- `config`;
- `GET`;
- `HEAD`;
- `POST`;
- `PUT`;
- `PATCH`;
- `DELETE`;
- `OPTIONS`;
- named fragment descriptors.

Type-only exports are allowed because they do not create runtime exports.

Example:

```tsx
export type UserId = string;
```

Unknown runtime exports MUST cause export validation to fail.

This rule is intentionally strict.

For example, this is invalid:

```tsx
export const helper = () => "hello";
```

A helper that is local to the route MUST not be exported:

```tsx
const helper = () => "hello";
```

Reusable helpers belong in `.ts` or `.tsx` modules.

---

# 6. HTTP handler exports

## 6.1 Handler names

HTTP handlers are declared through uppercase named exports.

Example:

```tsx
export async function GET(ctx: Context) {
  return <h1>Hello</h1>;
}

export async function POST(ctx: Context) {
  return new Response(null, { status: 204 });
}
```

Handler names are case-sensitive.

`get` is not equivalent to `GET`.

## 6.2 Handler type

The conceptual handler type is:

```ts
export type Handler = (
  ctx: Context,
) => HandlerResult | Promise<HandlerResult>;
```

Where:

```ts
export type HandlerResult = HtmlNode | Response;
```

A handler MUST return one of those values, directly or through a Promise.

## 6.3 Invalid return values

The following values are invalid handler results:

- `undefined`;
- `null`;
- booleans;
- plain JavaScript objects;
- strings returned directly as the entire response;
- numbers returned directly as the entire response;
- arrays returned directly as the entire response;
- arbitrary framework-shaped objects.

For example, this MUST NOT be treated as a redirect:

```ts
return { redirect: "/login" };
```

Applications MUST use an explicit `Response` or helper:

```ts
return redirect("/login");
```

## 6.4 Missing method

If a route exists but does not implement the incoming HTTP method, the runtime MUST return `405 Method Not Allowed`.

The response MUST include an `Allow` header containing the methods available for the route.

## 6.5 HEAD fallback

If `HEAD` is not exported but `GET` is exported, the runtime MUST treat `GET` as the fallback handler for `HEAD`.

The handler MUST execute using normal GET semantics, but the HTTP response body MUST be omitted.

If `HEAD` is explicitly exported, the explicit `HEAD` handler takes precedence.

## 6.6 OPTIONS fallback

If `OPTIONS` is not exported, the runtime MUST provide an automatic OPTIONS response for the route.

The automatic response MUST include `Allow`.

An explicit `OPTIONS` export takes precedence.

---

# 7. `config` export

A `.tsp` module MAY export a `config` value.

Example:

```tsx
import type { PageConfig } from "tsp:server";

export const config = {
  bodyLimit: 2 * 1024 * 1024,
  timeoutMs: 15_000,
} satisfies PageConfig;
```

The current contract `PageConfig` contract is:

```ts
export interface PageConfig {
  bodyLimit?: number;
  timeoutMs?: number;
}
```

`bodyLimit` is measured in bytes.

`timeoutMs` is measured in milliseconds.

A runtime MAY add implementation-specific configuration under explicitly namespaced extension keys, but MUST NOT silently reinterpret unknown top-level standard keys.

An invalid standard `config` value MUST fail generation validation.

---

# 8. Import rules

## 8.1 `.tsp` cannot be imported

Application code MUST NOT import a `.tsp` file.

This restriction applies to:

- static imports;
- dynamic imports;
- re-exports;
- absolute paths;
- relative paths;
- aliases that resolve to `.tsp`.

Invalid:

```ts
import { GET } from "./other.tsp";
```

Invalid:

```ts
await import("./other.tsp");
```

The runtime MUST reject such an edge.

## 8.2 `.tsp` may import application modules

A `.tsp` module MAY import ordinary application modules such as:

- `.ts`;
- `.tsx`;
- `.js`;
- `.jsx`;
- supported data modules.

Example:

```tsx
import { UserCard } from "../components/UserCard.tsx";
import { findUser } from "../lib/users.ts";
```

## 8.3 Explicit local extensions

Relative or absolute application-file imports SHOULD use explicit extensions.

A conforming current contract runtime MUST support explicit extensions.

A runtime MAY additionally support extension inference, but portable TSP applications SHOULD NOT rely on it.

## 8.4 Built-in modules

TSP runtime modules use the reserved `tsp:` scheme.

Examples:

```ts
import { redirect } from "tsp:server";
import { unsafeHtml } from "tsp:html";
```

The `tsp:` scheme MUST NOT be resolved through the filesystem or npm package lookup.

## 8.5 External packages

Bare package specifiers are external dependencies.

Example:

```ts
import { z } from "zod";
```

The runtime MUST resolve supported external packages using its external package resolver.

The exact package installation mechanism is not part of this specification.

## 8.6 Dynamic imports

A relative dynamic import into the reloadable application graph MUST use a string-literal specifier in TSP current contract.

Valid:

```ts
const mod = await import("../lib/report.ts");
```

Not portable and MUST be rejected for reloadable local modules:

```ts
const mod = await import(`../lib/${name}.ts`);
```

This restriction exists so the runtime can construct a deterministic dependency graph.

Dynamic imports of external package specifiers MAY be supported according to the host runtime's package semantics.

---

# 9. Reloadable and persistent module domains

TSP defines two module ownership domains.

## 9.1 Reloadable application modules

Reloadable application modules include:

- route `.tsp` files;
- application `.ts` files;
- application `.tsx` files;
- application `.js` / `.jsx` files when enabled;
- other application-local modules declared reloadable by the runtime.

These modules participate in page generations.

## 9.2 Persistent external modules

Persistent modules include:

- `tsp:*` built-ins;
- runtime-provided native bindings;
- external package modules unless configured otherwise;
- long-lived service bridge modules.

Persistent modules do not belong to a page generation.

## 9.3 Persistent state rule

If application code needs state that must survive a page reload, the state MUST be owned by a persistent service or persistent runtime module.

Application-local module top-level variables MUST NOT be used as durable application state.

---

# 10. Module identity

## 10.1 Canonical source identity

Every reloadable source file MUST have one canonical source identity.

The runtime MUST normalize application file identity before dependency graph insertion.

For filesystem modules, canonicalization MUST account for:

- absolute path normalization;
- `.` and `..` elimination;
- path separator normalization;
- symlink resolution or equivalent duplicate-identity prevention;
- application-root security validation.

## 10.2 Generation instance identity

A JavaScript module instance in the reloadable graph is identified conceptually by:

```text
(PageSlotId, GenerationId, CanonicalModuleId)
```

The same canonical source module imported by two independent page generations MAY therefore have two independent JavaScript module instances.

This is REQUIRED behavior from the application's point of view: application code MUST NOT assume that a local `.ts` module is a process-wide singleton.

## 10.3 Singleton within one generation

Within one page generation, repeated imports of the same canonical source module MUST resolve to the same module instance.

---

# 11. File-system routing

## 11.1 Route directory

By default, route modules are discovered below:

```text
<app-root>/routes
```

Only `.tsp` files under the configured route directory participate in automatic file-system routing.

## 11.2 Basic mapping

The following mappings are REQUIRED:

```text
routes/index.tsp          -> /
routes/login.tsp          -> /login
routes/users/index.tsp    -> /users
routes/users/new.tsp      -> /users/new
```

## 11.3 Dynamic segment

A filename or directory named `[name]` defines one dynamic path segment.

Example:

```text
routes/users/[id].tsp -> /users/:id
```

For a request to:

```text
/users/42
```

The context MUST contain:

```ts
ctx.params.id === "42"
```

## 11.4 Catch-all segment

A filename or directory named `[...name]` defines a catch-all segment.

Example:

```text
routes/files/[...path].tsp
```

The matched parameter MUST be exposed as a slash-joined string after URL path decoding.

Example:

```text
/files/a/b/c.txt
```

Produces conceptually:

```ts
ctx.params.path === "a/b/c.txt"
```

## 11.5 Segment-name validity

Dynamic parameter names MUST be non-empty valid identifier-like names accepted by the TSP route parser.

Duplicate parameter names within one route MUST be rejected at route-table construction time.

## 11.6 Route precedence

When multiple route patterns could match, precedence MUST be:

1. static segment;
2. dynamic segment;
3. catch-all segment.

Example:

```text
/users/new
```

MUST match:

```text
routes/users/new.tsp
```

before:

```text
routes/users/[id].tsp
```

## 11.7 Ambiguous routes

Two route files that produce the same route pattern and precedence MUST be treated as a configuration error.

The runtime MUST NOT resolve such ambiguity according to filesystem iteration order.

## 11.8 URL decoding

The URL path MUST be parsed as a URL path, not as a raw filesystem path.

Percent decoding MUST occur according to URL semantics.

Malformed percent encoding MUST produce `400 Bad Request`.

Path traversal sequences MUST NOT permit escaping the route or application root.

## 11.9 Trailing slash

Except for `/`, a trailing slash MUST NOT create a distinct route identity in current contract.

For route matching purposes:

```text
/users
/users/
```

match the same page.

The runtime MUST NOT automatically redirect solely because of a trailing slash unless explicitly configured by a future extension.

---

# 12. Route table lifecycle

The runtime MUST build a route table from the route directory.

In development mode, route file creation, deletion, and rename MUST eventually update the route table without requiring a process restart.

A route-table update MUST be atomic from the point of view of new requests.

An already-running request MUST NOT have its route target changed mid-request.

---

# 13. Request context

Every handler receives a `Context` object.

The core conceptual interface is:

```ts
export interface Context<
  Params extends Record<string, string> = Record<string, string>,
  Services extends Record<string, unknown> = AppServices,
> {
  readonly request: Request;
  readonly url: URL;
  readonly method: string;
  readonly params: Readonly<Params>;
  readonly query: URLSearchParams;
  readonly cookies: Cookies;
  readonly session: Session;
  readonly services: Services;
  readonly signal: AbortSignal;
  readonly route: RouteInfo;

  fragment(name: string, options?: FragmentUrlOptions): string;
}
```

## 13.1 Per-request lifetime

A new logical Context MUST be created for every request.

A Context MUST NOT be reused for a later request.

## 13.2 Context invalidation

Request-scoped native resources attached to a Context MAY be invalidated after the handler and renderer complete.

Application code MUST NOT retain Context and use it after request completion.

## 13.3 `request`

`ctx.request` MUST implement the Web `Request` contract supported by TSP.

The application SHOULD use standard APIs such as:

```ts
await ctx.request.text();
await ctx.request.json();
await ctx.request.formData();
```

## 13.4 `url`

`ctx.url` MUST represent the effective request URL as a Web `URL` object.

## 13.5 `query`

`ctx.query` MUST be equivalent to:

```ts
ctx.url.searchParams
```

It is a convenience reference, not a separately parsed query representation.

## 13.6 `params`

`ctx.params` MUST contain route parameters from file-system routing.

It MUST be read-only from the application's point of view.

## 13.7 `signal`

`ctx.signal` MUST be aborted when the runtime determines the request is no longer executable, including applicable timeout or disconnect conditions.

Application services SHOULD honor the signal when possible.

---

# 14. Request body semantics

## 14.1 Single consumption

The request body follows Web Request body-consumption semantics.

A body MUST NOT be consumed multiple times unless explicitly cloned according to supported Web APIs.

## 14.2 Body size

The runtime MUST enforce a configured body limit before unbounded body buffering.

If the body exceeds the effective limit, the runtime MUST return `413 Payload Too Large` unless the handler has already taken ownership of an allowed streaming body API.

## 14.3 Multipart

`request.formData()` MUST support standards-compatible multipart form parsing within configured resource limits.

Temporary-file and in-memory buffering strategy is implementation-defined.

The runtime MUST enforce size and resource limits.

---

# 15. Cookies

TSP exposes a request cookie interface through `ctx.cookies`.

Conceptually:

```ts
export interface Cookies {
  get(name: string): string | undefined;
  has(name: string): boolean;
  set(name: string, value: string, options?: CookieOptions): void;
  delete(name: string, options?: CookieDeleteOptions): void;
}
```

Cookie writes MUST be reflected in the outgoing response even when the handler returns an `HtmlNode`.

If the handler returns an explicit `Response`, TSP MUST merge runtime-managed cookie mutations into that response unless the response has become immutable under the host implementation.

Conflicting explicit `Set-Cookie` and context cookie operations SHOULD preserve all valid cookie header lines rather than comma-joining them.

---

# 16. Session

TSP defines a logical session service at `ctx.session`.

Conceptually:

```ts
export interface Session {
  readonly id: string;

  get<T extends SessionValue = SessionValue>(key: string): T | undefined;
  has(key: string): boolean;
  set(key: string, value: SessionValue): void;
  delete(key: string): void;
  clear(): void;

  regenerate(): Promise<void>;
  destroy(): Promise<void>;
}
```

## 16.1 Session value domain

Portable session values MUST be JSON-compatible values:

```ts
export type SessionValue =
  | null
  | boolean
  | number
  | string
  | SessionValue[]
  | { [key: string]: SessionValue };
```

Functions, Symbols, native handles, DOM-like nodes, Context objects, and arbitrary cyclic JavaScript objects MUST NOT be accepted as portable session values.

## 16.2 Persistence

Session state MUST survive page generation reloads.

Reloading a `.tsp`, `.ts`, or `.tsx` module MUST NOT recreate or discard the session store.

## 16.3 Regeneration

`session.regenerate()` MUST replace the session identifier while preserving session data unless the configured session backend documents stricter security semantics.

## 16.4 Destroy

After successful `session.destroy()`, the current logical session MUST no longer be usable as an authenticated persistent session.

---

# 17. Services

TSP exposes application and runtime services through:

```ts
ctx.services
```

## 17.1 Service ownership

A service may be:

- runtime-scoped;
- request-scoped.

Runtime-scoped services MUST survive page generation replacement.

Request-scoped services MUST NOT escape the lifetime of the request that created them.

## 17.2 No page-owned durable services

A `.tsp` module MUST NOT be the owner of a durable database pool, Redis pool, logger, session store, or equivalent application resource.

Creating such resources at application-module top level is non-portable and SHOULD be diagnosed in development tooling where practical.

## 17.3 Wrapper identity

The logical service may be persistent while the JavaScript wrapper object is recreated.

Application code MUST NOT depend on JavaScript object identity of `ctx.services.x` across different requests.

## 17.4 Type augmentation

Tooling SHOULD support application service typing through TypeScript module augmentation or generated declarations.

For example:

```ts
declare module "tsp:server" {
  interface AppServices {
    users: UserService;
    billing: BillingService;
  }
}
```

The exact service registration configuration is outside the core `.tsp` language grammar.

---

# 18. Handler responses

A handler may return either:

1. `HtmlNode`; or
2. Web `Response`.

## 18.1 `HtmlNode`

When a handler returns an `HtmlNode`, the runtime MUST render it as HTML.

Unless overridden by a future explicit API, the response MUST use:

```text
Content-Type: text/html; charset=utf-8
```

## 18.2 `Response`

When a handler returns a Web `Response`, the runtime MUST respect its:

- status;
- headers;
- body;
- content type.

Runtime-required headers such as valid session-cookie changes MAY be merged as defined by this specification.

## 18.3 Helpers

`tsp:server` SHOULD expose explicit response helpers including at least:

```ts
redirect(url: string | URL, status?: RedirectStatus): Response;
json(value: unknown, init?: ResponseInit): Response;
text(value: string, init?: ResponseInit): Response;
```

The helper result MUST be an ordinary `Response` from the application's point of view.

## 18.4 No response-shape inference

The runtime MUST NOT infer response meaning from arbitrary object keys.

---

# 19. TSP JSX runtime

## 19.1 Automatic JSX runtime

TSP applications MUST support an automatic JSX transform targeting the TSP JSX runtime.

Conceptually, TSX may compile to imports from:

```text
tsp:jsx-runtime
```

Development builds MAY use:

```text
tsp:jsx-dev-runtime
```

## 19.2 Required runtime exports

`tsp:jsx-runtime` MUST provide semantics corresponding to:

```ts
jsx(type, props, key?)
jsxs(type, props, key?)
Fragment
```

The concrete native/JS implementation is not part of the public ABI.

## 19.3 `HtmlNode`

`HtmlNode` is an opaque TSP value representing server-renderable HTML output.

Applications MUST NOT depend on its internal object layout.

A conforming runtime MAY represent `HtmlNode` as:

- a JavaScript object;
- a native-backed object;
- a tagged value;
- a lazy render instruction;
- another opaque representation.

Only behavior defined by this specification is public.

---

# 20. JSX intrinsic elements

Lowercase JSX element names represent HTML intrinsic elements.

Example:

```tsx
<div class="card">Hello</div>
```

## 20.1 HTML attribute names

TSP JSX follows HTML attribute naming rather than React compatibility naming.

Portable TSP code SHOULD use:

```tsx
<label for="name">Name</label>
<div class="card" />
```

rather than React-specific aliases such as `htmlFor` and `className`.

A runtime MAY support compatibility aliases, but applications conforming only to this specification MUST NOT depend on them.

## 20.2 Boolean attributes

For a valid HTML boolean attribute:

```tsx
<input disabled={true} />
```

MUST render the attribute as present.

```tsx
<input disabled={false} />
```

MUST omit the attribute.

## 20.3 Nullish attributes

An attribute value of `null` or `undefined` MUST omit the attribute.

## 20.4 String escaping

Attribute string values MUST be escaped for HTML attribute context.

Text children MUST be escaped for HTML text context.

At minimum, the renderer MUST prevent raw `<`, `>`, and `&` text from becoming markup and MUST escape quote characters where required by the chosen attribute quoting form.

## 20.5 Function-valued DOM attributes

Function-valued intrinsic-element attributes MUST NOT be serialized.

For example:

```tsx
<button onclick={() => doThing()} />
```

MUST produce a render error rather than embedding a JavaScript function.

TSP does not define React-style browser event binding or hydration.

## 20.6 `key`

A JSX `key` is renderer metadata and MUST NOT be emitted as an HTML attribute.

The runtime MAY ignore `key` in non-diffing server rendering.

## 20.7 `ref`

React-style `ref` is not part of the TSP core JSX contract.

A `ref` value supplied to an intrinsic element SHOULD produce a development diagnostic.

---

# 21. JSX child values

The renderer MUST recursively process supported child values.

## 21.1 Text

Strings render as escaped text.

Example:

```tsx
<div>{"<b>"}</div>
```

MUST render text equivalent to:

```html
<div>&lt;b&gt;</div>
```

## 21.2 Numbers

Finite numbers render using their ordinary string representation.

## 21.3 BigInt

BigInt values MAY be rendered using decimal string representation.

Applications requiring maximum portability SHOULD explicitly convert BigInt to string.

## 21.4 Empty values

The following child values render nothing:

- `null`;
- `undefined`;
- `false`;
- `true`.

## 21.5 Arrays and iterables

Arrays MUST be recursively flattened in iteration order.

A runtime MAY support additional synchronous iterables.

## 21.6 Promises

Promise-like child values originating from TSP components MUST be awaited by the renderer.

## 21.7 Invalid objects

A plain object that is not an `HtmlNode` and is not otherwise explicitly supported MUST produce a render error.

The runtime MUST NOT serialize arbitrary objects as `[object Object]`.

---

# 22. Components

An uppercase JSX tag denotes a TSP server component.

A portable TSP component is a function.

Example:

```tsx
function UserCard(props: { name: string }) {
  return <article>{props.name}</article>;
}
```

## 22.1 Component result

A component MAY return:

- `HtmlNode`;
- a supported JSX child value;
- a Promise resolving to a supported component result.

## 22.2 Async components

Async components are REQUIRED functionality.

Example:

```tsx
async function UserName({ id }: { id: string }) {
  const user = await loadUser(id);
  return <strong>{user.name}</strong>;
}
```

The renderer MUST correctly await the component.

## 22.3 Class components

React-style class components are not part of the TSP core contract.

## 22.4 Component state

TSP server components have no standardized client lifecycle, hooks, reconciliation, or browser-local state.

TSP MUST NOT imply React hook semantics.

## 22.5 Component errors

An exception thrown by a component is a render-phase exception unless a future explicit TSP error-boundary API handles it.

TSP current contract does not require component-level error boundaries.

---

# 23. Fragment

A fragment is a separately addressable server handler declared from a `.tsp` module.

Fragments are intended for partial HTML requests such as HTMX-driven updates, but the protocol is not tied to HTMX.

## 23.1 Declaration

`tsp:server` MUST provide a `fragment()` declaration helper.

The canonical current contract form is:

```tsx
export const userList = fragment(
  async (ctx) => {
    return <ul>...</ul>;
  },
  { method: "GET" },
);
```

A fragment export is legal only if its runtime value is a valid TSP fragment descriptor.

## 23.2 Fragment name

The JavaScript export name is the fragment's logical name.

The following names are reserved and MUST NOT be used as fragment names:

- `config`;
- all standard HTTP method export names.

## 23.3 Fragment result

A fragment handler follows the same `HandlerResult` contract as a page method handler.

An `HtmlNode` fragment response MUST be rendered as an HTML fragment and MUST NOT automatically receive an HTML document doctype.

## 23.4 Fragment URL

The concrete URL path used internally to address a fragment is **not** part of the public TSP protocol.

Applications MUST obtain fragment URLs through the runtime.

Example:

```tsx
<a href={ctx.fragment("userList")}>Refresh</a>
```

The runtime MAY change the internal fragment URL format without a major TSP version change.

## 23.5 Fragment HTTP method

The fragment descriptor MUST declare one HTTP method or use the runtime's documented default.

The current contract default method is `GET`.

Requests using a different method MUST receive `405 Method Not Allowed` unless that method is explicitly supported by the descriptor.

---

# 24. HTML document rendering

## 24.1 Full page doctype

When a page HTTP handler returns an `HtmlNode` whose root intrinsic element is `html`, the renderer MUST prepend an HTML5 doctype:

```html
<!doctype html>
```

unless the returned value is already an explicit full `Response` whose body is controlled by the application.

## 24.2 Fragment rendering

Fragment handlers MUST NOT receive an automatic doctype.

## 24.3 Non-`html` page root

A page handler MAY return a non-`html` root `HtmlNode`.

In that case the runtime MUST render exactly that fragment without adding a doctype.

This permits routes that intentionally return partial HTML.

---

# 25. Unsafe HTML

`tsp:html` MUST provide an explicitly unsafe escape hatch for trusted pre-rendered HTML.

The API SHOULD use a name that clearly communicates risk, such as:

```ts
unsafeHtml(value: string): HtmlNode;
```

The runtime MUST NOT sanitize such content automatically.

Applications are responsible for ensuring that untrusted input is not passed to this API.

TSP MUST NOT support React's `dangerouslySetInnerHTML` as a core requirement.

---

# 26. Built-in modules

The following modules are reserved by TSP.

## 26.1 `tsp:server`

`tsp:server` SHOULD expose the stable server-facing API, including:

```ts
Context
PageConfig
Handler
HandlerResult
AppServices
fragment
redirect
json
text
```

## 26.2 `tsp:html`

`tsp:html` SHOULD expose HTML-specific helpers including:

```ts
unsafeHtml
```

## 26.3 `tsp:jsx-runtime`

Used by the automatic JSX transform.

Applications normally SHOULD NOT import it manually.

## 26.4 `tsp:jsx-dev-runtime`

MAY provide development-only JSX metadata and diagnostics.

## 26.5 No framework globals

Core TSP APIs MUST NOT require global declarations such as:

```text
globalThis.Page
globalThis.Fragment
globalThis.HtmxFragment
```

Standard Web globals provided by the host runtime are unaffected by this rule.

---

# 27. Top-level code and side effects

A `.tsp` module and its reloadable application dependencies may execute top-level code during generation construction.

## 27.1 Execution frequency

Top-level code MAY run again whenever a new page generation is built.

Application code MUST NOT assume it runs exactly once per process.

## 27.2 Top-level await

Top-level await MUST be supported in reloadable application modules.

A generation MUST NOT be published until required module evaluation and top-level await complete successfully.

## 27.3 Durable resources

Top-level code SHOULD NOT create durable process resources such as:

- listening sockets;
- connection pools;
- permanent timers;
- process-global watchers;
- process-global mutable registries.

Such resources belong in the persistent service/runtime layer.

## 27.4 Failed top-level evaluation

If top-level evaluation throws or rejects while building a candidate generation, the candidate MUST fail and MUST NOT replace the current published generation.

---

# 28. Page registry

Every discovered route MUST correspond to a logical `PageSlot`.

A PageSlot conceptually contains:

```text
PageSlot
├── Route identity
├── Source .tsp identity
├── Current published generation
├── Dirty state
├── Build-in-flight state
└── Last reload error metadata
```

This data structure is conceptual; exact Rust layout is implementation-defined.

---

# 29. Generation model

A generation is an immutable, successfully constructed executable version of a page and its reloadable dependency graph.

## 29.1 Generation identity

Every published generation MUST have an identity that is distinct from previous generations of the same PageSlot.

Generation identifiers need not be globally stable across process restart.

## 29.2 Immutability after publish

After a generation is published, its module membership and evaluated module instances MUST NOT be mutated in place to represent a later source version.

A source update creates a candidate for a **new** generation.

## 29.3 Atomic publication

A candidate generation MUST become visible to new requests atomically.

A request MUST NOT observe a half-updated graph containing an arbitrary mixture of old and newly evaluated reloadable modules.

## 29.4 Request pinning

When request dispatch selects a generation, that request MUST remain pinned to the selected generation until its handler/render lifecycle completes.

If generation B is published while a request is executing generation A:

```text
old request  -> generation A
new request  -> generation B
```

The old request MUST NOT switch to B mid-execution.

## 29.5 Generation retirement

An old generation MAY be retired only after it is no longer required by in-flight requests or other runtime references.

The exact memory-reclamation mechanism is implementation-defined.

---

# 30. Page-generation module isolation

Within a page generation, reloadable application modules behave as ESM singletons for that generation.

Across independent PageSlots or generations, application-local module state is not guaranteed to be shared.

Example:

```ts
// lib/counter.ts
let count = 0;
export function next() { return ++count; }
```

If imported by two unrelated `.tsp` routes, a portable TSP application MUST NOT assume those routes share the same `count` variable.

Applications requiring shared state MUST use `ctx.services` or another persistent runtime facility.

This isolation rule is deliberate and is part of the architecture.

---

# 31. Dependency graph

The runtime MUST maintain enough dependency information to determine which PageSlots are affected by a reloadable source change.

Conceptually:

```text
Page A.tsp -> component.tsx -> format.ts
Page B.tsp -> component.tsx -> format.ts
```

Changing `format.ts` affects both Page A and Page B.

## 31.1 Static imports

Static local imports MUST create dependency graph edges.

## 31.2 Literal dynamic imports

Supported literal dynamic local imports MUST create or update dependency graph edges when resolved.

## 31.3 Reverse dependency lookup

The runtime MUST be capable of discovering affected page roots from a changed reloadable module without globally restarting the application process.

---

# 32. Development hot reload

Hot reload in this specification means **server module generation replacement**, not browser HMR.

## 32.1 Dirty marking

When the development watcher observes a relevant application source change, every affected PageSlot MUST eventually become dirty.

## 32.2 Rebuild

Before a dirty PageSlot can publish new source behavior, the runtime MUST construct a candidate generation.

## 32.3 Candidate validation

A candidate generation MUST complete all required phases before publication:

1. source resolution;
2. source loading;
3. transpilation/parsing;
4. dependency resolution;
5. module instantiation;
6. module evaluation;
7. `.tsp` export validation.

A runtime MAY perform additional validation.

## 32.4 Deduplication

Concurrent requests that trigger rebuilding the same dirty PageSlot MUST NOT cause unbounded duplicate candidate builds.

The runtime MUST deduplicate the in-flight build or otherwise provide equivalent correctness.

## 32.5 Last Known Good

If a candidate generation fails and a previously published generation exists, the runtime MUST keep the previous generation published.

In development mode, the default behavior MUST be to continue serving the Last Known Good generation for new requests while exposing the reload error through development diagnostics.

## 32.6 No Last Known Good

If no successful generation has ever been published for the route, a generation construction failure MUST result in a server error response.

## 32.7 Recovery

After a failed candidate build, a later source change or rebuild attempt MAY succeed and publish a new generation without process restart.

---

# 33. File change semantics

## 33.1 Changed module

Changing a reloadable module MUST invalidate all page roots that depend on it.

## 33.2 New dependency

If a newly built generation introduces a new dependency edge, the dependency graph MUST reflect that edge after successful publication.

## 33.3 Removed dependency

If a new generation removes an old dependency edge, future invalidation MUST no longer treat that removed edge as part of the current generation graph.

## 33.4 Deleted dependency

Deleting a required dependency MUST cause candidate generation construction to fail.

If an LKG generation exists, it remains published.

## 33.5 Deleted route

Deleting a route `.tsp` file in development MUST eventually remove the route from the route table for new requests.

In-flight requests already pinned to the deleted route MAY complete.

---

# 34. Runtime modes

TSP defines at least two logical modes:

- development;
- production.

## 34.1 Development

Development mode MUST prioritize:

- source diagnostics;
- file watching;
- generation reload;
- source maps;
- readable error output.

## 34.2 Production

Production mode SHOULD prioritize:

- deterministic startup;
- reduced diagnostics exposure;
- stable performance;
- disabled file watching unless explicitly configured.

Production mode MUST NOT expose development stack/source pages to untrusted clients by default.

---

# 35. Error phases

TSP SHOULD classify errors by phase.

Recommended standard phases are:

```text
route
resolve
load
transpile
instantiate
evaluate
validate
request
handler
render
service
session
static
internal
```

The exact Rust error types are implementation-defined, but development diagnostics SHOULD expose a stable error code and phase.

Example conceptual diagnostic:

```text
TSP-E-TRANSPILE
TSP-E-EXPORT
TSP-E-RENDER
```

---

# 36. Error responses

## 36.1 400

Malformed request targets, invalid path encodings, or other client request syntax failures SHOULD produce `400 Bad Request`.

## 36.2 404

No matching route and no matching static resource MUST produce `404 Not Found`.

## 36.3 405

Matching route with unsupported method MUST produce `405 Method Not Allowed` with `Allow`.

## 36.4 413

Request body limit violations MUST produce `413 Payload Too Large`.

## 36.5 500

Unhandled page execution, render, initial-generation, or internal runtime failures SHOULD produce `500 Internal Server Error` unless a more specific status is defined.

## 36.6 Timeout

If the runtime enforces a handler timeout and can still send an HTTP response, expiration SHOULD produce `504 Gateway Timeout` or another explicitly configured timeout response.

The request AbortSignal MUST be aborted.

---

# 37. Development diagnostics

Development errors SHOULD include:

- TSP error code;
- error phase;
- route;
- source file;
- source line/column when available;
- original TypeScript/TSX source location;
- stack trace;
- candidate generation identity;
- Last Known Good status;
- dependency chain when useful.

Source maps MUST map transpiled JavaScript failures back to original TypeScript/TSX source when mapping information is available.

Reload errors MUST be visible without replacing a valid LKG generation.

---

# 38. Timeout and abort semantics

A page MAY have an effective timeout from application or page configuration.

When the timeout expires:

1. `ctx.signal` MUST become aborted;
2. the runtime SHOULD attempt to cancel abort-aware service operations;
3. the runtime MUST stop waiting indefinitely for normal request completion;
4. the runtime MUST preserve runtime consistency.

The runtime MUST NOT destroy process-global services merely because one request timed out.

Hard VM interruption semantics are implementation-defined and MUST prioritize runtime safety.

---

# 39. Concurrency semantics

TSP does not guarantee that two requests execute serially.

Application code MUST assume handlers may overlap in time.

Runtime-scoped services MUST provide appropriate concurrency safety for their implementation.

A page generation MAY have multiple concurrent in-flight requests.

Publishing a new generation MUST NOT require terminating requests using the old generation.

---

# 40. Rendering model in current contract

The mandatory TSP current contract rendering contract is buffered server rendering.

The runtime MAY internally optimize rendering, but an application MUST be able to rely on the fact that render-phase failure before response commitment can still produce a normal server-error response.

Streaming HTML rendering is OPTIONAL in current contract and MUST use an explicit API or capability when introduced.

The core `HtmlNode` return type MUST NOT silently change from buffered semantics to partially committed streaming semantics in a way that changes error behavior.

---

# 41. Static files

Static files are served from the configured public directory, defaulting to:

```text
<app-root>/public
```

## 41.1 No executable interpretation

Files in `public/` MUST be treated as static resources and MUST NOT be executed as `.tsp` route modules.

## 41.2 Path safety

Static file resolution MUST reject traversal outside the configured public root, including traversal through symlinks where applicable.

## 41.3 Route/static conflict

A runtime MUST define a deterministic priority between dynamic routes and public static files.

TSP current contract defines:

```text
explicit route match > static-file fallback
```

A matching `.tsp` route therefore takes precedence over a public file at the same URL path.

---

# 42. Security requirements

A conforming runtime MUST protect filesystem boundaries.

At minimum it MUST prevent:

- `..` path traversal outside configured roots;
- encoded traversal bypasses;
- symlink escape from route/public/application roots where the operation is restricted to those roots;
- direct serving of arbitrary application source files as static content merely because the URL resembles a filesystem path.

The runtime MUST NOT expose stack traces or source code in production error pages by default.

Automatic HTML rendering MUST escape ordinary text and attribute values.

Unsafe raw HTML MUST require an explicit unsafe API.

---

# 43. Configuration file

The canonical native TSP configuration file is:

```text
tsp.toml
```

A minimal example is:

```toml
[app]
routes = "routes"
public = "public"

[server]
host = "127.0.0.1"
port = 3000

[request]
body_limit = 10485760
timeout_ms = 30000

[dev]
watch = true
```

## 43.1 Native parsing

Core runtime configuration MUST be readable without executing application JavaScript.

A conforming current contract implementation MUST NOT require `tsp.config.ts` merely to start the native HTTP runtime.

## 43.2 Unknown keys

The configuration parser SHOULD reject unknown standard keys by default or emit a clear diagnostic.

## 43.3 Environment overrides

The runtime MAY support environment-variable and CLI overrides.

Precedence MUST be deterministic and documented.

---

# 44. Runtime startup

A conforming runtime startup sequence SHOULD follow these logical phases:

```text
parse CLI
  ↓
load tsp.toml
  ↓
validate app roots
  ↓
initialize persistent runtime services
  ↓
initialize JS VM / persistent builtins
  ↓
build route table
  ↓
initialize watcher in dev
  ↓
start HTTP listener
```

The runtime MUST NOT require loading every `.tsp` generation before binding the HTTP port unless configured for eager validation.

A runtime MAY support eager startup validation as a mode.

---

# 45. Page load pipeline

When a generation must be constructed, the logical pipeline is:

```text
route source
  ↓
canonicalize
  ↓
load source
  ↓
parse/transpile TS/TSX
  ↓
resolve local dependencies
  ↓
construct generation module namespace
  ↓
instantiate ESM graph
  ↓
evaluate ESM graph
  ↓
validate .tsp exports
  ↓
create immutable candidate generation
  ↓
atomic publish
```

A candidate MUST NOT publish before all mandatory validation succeeds.

---

# 46. Export validation

After module evaluation, the runtime MUST validate `.tsp` runtime exports.

Validation includes at least:

- no `default` export;
- recognized HTTP handler exports are callable;
- `config`, if present, has a valid type/shape;
- fragment exports are valid branded fragment descriptors;
- no unknown runtime exports.

Validation failure is a generation-build failure.

---

# 47. Fragment and generation relationship

Fragments declared by a `.tsp` module belong to the same PageSlot generation as that page.

A fragment request MUST pin a generation using the same generation-safety rules as normal page requests.

If a page generation is replaced, new fragment requests MUST use the new published generation.

In-flight fragment requests MAY complete using the old generation.

---

# 48. Service and generation relationship

Runtime-scoped services are outside page-generation ownership.

Therefore this sequence MUST be valid:

```text
Generation A -> ctx.services.db -> DB pool X
Generation B -> ctx.services.db -> DB pool X
```

where X is the same logical runtime service even though JavaScript wrappers MAY differ.

A source reload MUST NOT recreate X merely because a page generation changed.

---

# 49. JavaScript VM boundary

The public TSP specification does not expose JavaScriptCore-specific object APIs.

Application code MUST interact through Web APIs and `tsp:*` APIs rather than raw JSC handles.

The native runtime MUST ensure that native objects exposed to JavaScript obey safe lifetime rules.

A page generation MUST NOT retain native request resources after their request lifetime without an explicit persistent ownership transfer API.

---

# 50. Garbage collection and lifecycle

JavaScript garbage collection is implementation-defined, but the following observable guarantees apply:

- an in-flight request's pinned generation MUST remain valid;
- a published current generation MUST remain valid;
- retired generations MAY be collected after they are no longer referenced;
- collection of a page generation MUST NOT destroy independent persistent services;
- native handles referenced by live JS wrappers MUST not become unsafely dangling.

Applications MUST NOT rely on deterministic JavaScript finalization timing.

---

# 51. External package lifecycle

External package modules MAY live in a persistent module domain.

Application code MUST therefore avoid assuming that editing an installed external package automatically causes page hot reload.

Development tooling MAY watch selected workspace packages as reloadable sources when explicitly configured.

A workspace package promoted into the reloadable application domain MUST follow generation isolation semantics.

---

# 52. No implicit browser runtime

TSP is server-first.

The core specification does not define:

- client hydration;
- virtual DOM reconciliation;
- React hooks;
- browser component state;
- client bundles generated from server components;
- server actions protocol.

Applications MAY include ordinary scripts:

```tsx
<script src="/app.js"></script>
```

or use libraries such as HTMX independently.

Such browser behavior is outside the core page execution protocol unless explicitly added by a later TSP specification.

---

# 53. Recommended `.tsp` example

A canonical current page looks like:

```tsx
import {
  type Context,
  type PageConfig,
  fragment,
  redirect,
} from "tsp:server";

import { UserList } from "../components/UserList.tsx";

export const config = {
  bodyLimit: 1024 * 1024,
  timeoutMs: 10_000,
} satisfies PageConfig;

export async function GET(ctx: Context) {
  const users = await ctx.services.users.list();

  return (
    <html lang="en">
      <head>
        <meta charset="utf-8" />
        <title>Users</title>
      </head>
      <body>
        <h1>Users</h1>

        <button
          hx-get={ctx.fragment("userList")}
          hx-target="#user-list"
        >
          Refresh
        </button>

        <div id="user-list">
          <UserList users={users} />
        </div>
      </body>
    </html>
  );
}

export async function POST(ctx: Context) {
  const form = await ctx.request.formData();
  const name = String(form.get("name") ?? "");

  await ctx.services.users.create({ name });

  return redirect("/users", 303);
}

export const userList = fragment(
  async (ctx: Context) => {
    const users = await ctx.services.users.list();
    return <UserList users={users} />;
  },
  { method: "GET" },
);
```

The page demonstrates the intended TSP principles:

- ordinary TSX;
- explicit imports;
- no framework globals;
- no `Page()` wrapper;
- no default export;
- named HTTP methods;
- persistent services through Context;
- explicit fragment declaration;
- explicit response helpers;
- custom TSP JSX semantics rather than React semantics.

---

# 54. TypeScript declaration contract

A TSP SDK SHOULD provide declarations conceptually equivalent to the following.

```ts
// tsp:server

export interface AppServices extends Record<string, unknown> {}

export interface RouteInfo {
  readonly pattern: string;
  readonly source: string;
}

export interface CookieOptions {
  path?: string;
  domain?: string;
  secure?: boolean;
  httpOnly?: boolean;
  sameSite?: "strict" | "lax" | "none";
  maxAge?: number;
  expires?: Date;
}

export interface CookieDeleteOptions {
  path?: string;
  domain?: string;
}

export interface Cookies {
  get(name: string): string | undefined;
  has(name: string): boolean;
  set(name: string, value: string, options?: CookieOptions): void;
  delete(name: string, options?: CookieDeleteOptions): void;
}

export type SessionValue =
  | null
  | boolean
  | number
  | string
  | SessionValue[]
  | { [key: string]: SessionValue };

export interface Session {
  readonly id: string;
  get<T extends SessionValue = SessionValue>(key: string): T | undefined;
  has(key: string): boolean;
  set(key: string, value: SessionValue): void;
  delete(key: string): void;
  clear(): void;
  regenerate(): Promise<void>;
  destroy(): Promise<void>;
}

export interface FragmentUrlOptions {
  query?: Record<string, string | number | boolean | null | undefined>;
}

export interface Context<
  Params extends Record<string, string> = Record<string, string>,
  Services extends Record<string, unknown> = AppServices,
> {
  readonly request: Request;
  readonly url: URL;
  readonly method: string;
  readonly params: Readonly<Params>;
  readonly query: URLSearchParams;
  readonly cookies: Cookies;
  readonly session: Session;
  readonly services: Services;
  readonly signal: AbortSignal;
  readonly route: RouteInfo;

  fragment(name: string, options?: FragmentUrlOptions): string;
}

export interface PageConfig {
  bodyLimit?: number;
  timeoutMs?: number;
}

export interface HtmlNode {
  readonly __tspHtmlNodeBrand: unique symbol;
}

export type HandlerResult = HtmlNode | Response;

export type Handler<
  Params extends Record<string, string> = Record<string, string>,
  Services extends Record<string, unknown> = AppServices,
> = (
  ctx: Context<Params, Services>,
) => HandlerResult | Promise<HandlerResult>;

export type FragmentMethod =
  | "GET"
  | "HEAD"
  | "POST"
  | "PUT"
  | "PATCH"
  | "DELETE";

export interface FragmentOptions {
  method?: FragmentMethod;
}

export interface FragmentDescriptor {
  readonly __tspFragmentBrand: unique symbol;
}

export function fragment(
  handler: Handler,
  options?: FragmentOptions,
): FragmentDescriptor;

export function redirect(
  url: string | URL,
  status?: 301 | 302 | 303 | 307 | 308,
): Response;

export function json(value: unknown, init?: ResponseInit): Response;
export function text(value: string, init?: ResponseInit): Response;
```

The unique-symbol brands above are illustrative typing devices.

Applications MUST treat `HtmlNode` and `FragmentDescriptor` as opaque.

---

# 55. JSX type contract

The TSP SDK SHOULD provide JSX namespace declarations so TypeScript can type-check TSP intrinsic HTML elements without importing React types.

A conforming project MUST NOT require `@types/react` merely to type-check ordinary TSP JSX.

The SDK SHOULD provide:

```ts
namespace JSX {
  type Element = HtmlNode;
  interface IntrinsicElements {
    // HTML elements and TSP-supported attributes
  }
}
```

The exact completeness of HTML attribute typing may evolve in minor versions as long as runtime semantics do not change incompatibly.

---

# 56. Source maps

The transpilation pipeline MUST preserve enough mapping information for development diagnostics to map runtime errors to original source locations.

For errors caused by:

- `.tsp`;
- `.ts`;
- `.tsx`;

TSP SHOULD report the original source file and line/column rather than only generated JavaScript locations.

---

# 57. Tooling commands

A conforming TSP distribution SHOULD provide the following CLI concepts:

```text
tsp dev
tsp serve
tsp check
tsp build
```

## 57.1 `tsp dev`

Starts development mode with file watching and development diagnostics.

## 57.2 `tsp serve`

Starts production-oriented serving behavior.

## 57.3 `tsp check`

Performs static and TSP-contract checks without starting the HTTP server.

At minimum it SHOULD detect:

- TypeScript errors;
- invalid `.tsp` imports;
- invalid route patterns;
- route collisions;
- invalid `.tsp` runtime exports where statically discoverable;
- invalid TSP configuration.

## 57.4 `tsp build`

Prepares a deployable application artifact according to the implementation's packaging model.

The application page source MAY remain externally mutable when the deployment mode is configured for runtime source loading.

---

# 58. Deployment contract

TSP MUST support a deployment model where the native server executable is separable from application page source.

Conceptually:

```text
deploy/
├── tspserver
├── tsp.toml
├── routes/
├── components/
├── lib/
├── public/
└── external dependencies / runtime assets as required
```

The executable MAY embed TSP runtime libraries and built-ins.

The core specification does not require `.tsp` pages to be bundled into the executable.

A deployment mode that keeps `.tsp` and local application modules external MUST load their current filesystem source according to the configured runtime mode.

---

# 59. Worker/process semantics

A TSP implementation MAY use one or multiple workers or processes.

Regardless of implementation, observable semantics MUST preserve:

- route correctness;
- session-store correctness;
- generation atomicity per request;
- valid LKG behavior;
- consistent file-change propagation within documented development bounds.

A runtime MUST NOT advertise successful hot reload while indefinitely serving incompatible generations across workers without coordination.

Exact watcher/broadcast architecture is implementation-defined.

---

# 60. Logging

TSP SHOULD provide structured request and runtime logging.

A request log SHOULD be capable of including:

- request ID;
- method;
- route;
- status;
- duration;
- generation ID;
- worker/process identity;
- reload/LKG diagnostic state when relevant.

Sensitive values such as cookies, authorization headers, passwords, and session contents MUST NOT be logged by default.

---

# 61. Request identity

Every request SHOULD have a runtime request identifier.

The identifier SHOULD be available to logging and MAY be exposed through Context in a later additive API.

Request identity MUST NOT be used as a security credential.

---

# 62. Content caching

TSP current contract does not define automatic page-output caching.

A handler that requires HTTP caching SHOULD explicitly set appropriate response headers or use future cache APIs.

The runtime MUST NOT cache dynamic `HtmlNode` responses across requests by default.

Static-file caching MAY follow ordinary HTTP caching behavior.

---

# 63. Middleware

User-defined arbitrary middleware is **not** part of the mandatory TSP current contract page contract.

This omission is deliberate.

Cross-cutting capabilities SHOULD initially be implemented through:

- native runtime facilities;
- services;
- explicit handler helpers;
- future typed hook APIs.

A future middleware specification MUST preserve generation and request-lifetime semantics defined here.

---

# 64. Layouts

A special layout file convention is **not** part of the mandatory TSP current contract specification.

Reusable layouts SHOULD initially be ordinary `.tsx` components.

Example:

```tsx
import { AppLayout } from "../components/AppLayout.tsx";

export function GET() {
  return (
    <AppLayout title="Home">
      <h1>Home</h1>
    </AppLayout>
  );
}
```

This avoids introducing hidden nested route lifecycles in current contract.

A future layout protocol MUST define generation ownership explicitly before becoming standard.

---

# 65. Authentication

Authentication policy is not encoded into the `.tsp` grammar in current contract.

Authentication SHOULD be implemented through persistent services and explicit handler code or future typed runtime policy hooks.

Example:

```tsx
export async function GET(ctx: Context) {
  const user = await ctx.services.auth.requireUser(ctx);
  return <h1>Hello {user.name}</h1>;
}
```

TSP MUST NOT infer authentication semantics from arbitrary exported object shapes.

---

# 66. Compatibility and versioning

## 66.1 v1 incompatibility

TSP does not guarantee compatibility with:

- v1 `Page()` wrappers;
- v1 global injection APIs;
- v1 default page exports;
- v1 fragment URL formats;
- v1 React runtime assumptions;
- v1 `main.ts` server architecture.

## 66.2 Major version changes

A change requiring ordinary valid current `.tsp` source to be rewritten is normally a major-version change.

Examples include:

- changing legal handler export names;
- changing `HtmlNode` child semantics;
- changing Context core property meanings;
- changing route mapping syntax;
- allowing `.tsp` import in a way that changes generation identity semantics;
- changing generation pinning guarantees.

## 66.3 Minor version changes

Additive APIs MAY be introduced in minor versions if existing conforming source retains the same behavior.

---

# 67. Frozen current contract protocol decisions

Before current contract is declared stable, the following decisions SHOULD be treated as protocol freeze candidates.

1. `.tsp` is standard TypeScript + JSX syntax.
2. `.tsp` is a route/generation root.
3. `.tsp` cannot be imported by application code.
4. No runtime default export.
5. HTTP methods use uppercase named exports.
6. Unknown runtime exports are invalid.
7. `HandlerResult = HtmlNode | Response`.
8. No arbitrary object response magic.
9. Core JSX uses TSP runtime, not React.
10. Async components are supported.
11. Ordinary child content is HTML-escaped.
12. Raw HTML requires an explicit unsafe API.
13. `ctx.request` and explicit `Response` follow Web API semantics.
14. Durable state lives in persistent services, not page modules.
15. Page reload creates a new immutable generation.
16. New requests atomically switch to a new published generation.
17. Old requests remain pinned to the old generation.
18. Failed candidate reload preserves Last Known Good in development.
19. Shared application-local modules are not guaranteed process-wide singleton identity across PageSlots.
20. `tsp:*` is the reserved runtime module namespace.
21. Framework globals are not required.
22. Core native configuration does not require evaluating app JavaScript.
23. Fragment URL layout is internal and obtained through runtime APIs.
24. React/browser hydration is outside the core current contract.
25. Layout/middleware magic is intentionally deferred.

Changing one of these after ecosystem adoption should require strong justification and, in many cases, a major version.

---

# 68. Required conformance tests

A conforming implementation SHOULD maintain an executable conformance suite derived directly from this document.

At minimum, the suite MUST cover the following areas before current contract release.

## 68.1 `.tsp` module contract

- valid GET-only page;
- valid multiple HTTP methods;
- default export rejected;
- unknown runtime export rejected;
- invalid handler type rejected;
- invalid config rejected;
- valid type-only exports accepted;
- valid fragment descriptor accepted;
- invalid fragment export rejected.

## 68.2 Imports

- `.tsp` imports `.ts`;
- `.tsp` imports `.tsx`;
- local module imports another local module;
- local module cannot import `.tsp`;
- `.tsp` cannot import `.tsp`;
- literal dynamic local import works;
- non-literal dynamic local import is rejected;
- built-in `tsp:*` resolution works;
- external package resolution works.

## 68.3 Routing

- `/` from `index.tsp`;
- nested index route;
- static route;
- dynamic segment;
- catch-all segment;
- static beats dynamic;
- dynamic beats catch-all;
- duplicate route rejected;
- trailing slash equivalence;
- malformed URL encoding gives 400;
- traversal cannot escape root.

## 68.4 HTTP methods

- GET;
- POST;
- PUT;
- PATCH;
- DELETE;
- explicit HEAD;
- HEAD fallback to GET;
- automatic OPTIONS;
- explicit OPTIONS;
- 405 + Allow.

## 68.5 Context

- params;
- query;
- URL;
- Request body APIs;
- AbortSignal;
- cookies;
- session;
- services;
- fragment URL generation.

## 68.6 JSX

- intrinsic elements;
- escaped text;
- escaped attributes;
- boolean attributes;
- nullish attributes omitted;
- arrays flattened;
- null/undefined/boolean children omitted;
- number children;
- sync component;
- async component;
- nested async component;
- invalid object child rejected;
- function DOM attribute rejected;
- unsafeHtml bypasses escaping;
- key not serialized;
- HTML root receives doctype;
- fragment output does not receive doctype.

## 68.7 Response

- HtmlNode -> HTML response;
- explicit Response status preserved;
- explicit headers preserved;
- JSON helper;
- redirect helper;
- invalid plain-object return rejected;
- cookies merged into outgoing response.

## 68.8 Generation

- initial generation load;
- source edit creates new generation;
- transitive dependency edit dirties page;
- shared dependency dirties all dependent pages;
- old request continues old generation;
- new request sees new generation;
- two concurrent reload triggers deduplicate;
- failed transpile preserves LKG;
- failed evaluation preserves LKG;
- failed export validation preserves LKG;
- recovery after failure;
- removed dependency updates graph;
- new dependency updates graph.

## 68.9 Persistent state

- session survives page reload;
- runtime service survives page reload;
- DB/Redis-style mock pool is not recreated on page generation change;
- page-local module top-level state is generation-scoped;
- separate PageSlots do not require shared local-module singleton state.

## 68.10 Security

- static traversal blocked;
- route traversal blocked;
- symlink escape blocked where applicable;
- production errors hide source;
- ordinary JSX escapes XSS payload;
- unsafeHtml is the only intentional raw bypass in the core renderer.

---

# 69. Non-goals for TSP current contract

The following are explicitly not required for current contract conformance:

- React compatibility;
- React Server Components;
- browser hydration;
- client-side router;
- virtual DOM;
- class components;
- React hooks;
- Next.js-compatible layouts;
- route groups;
- parallel routes;
- intercepting routes;
- server actions;
- arbitrary user middleware;
- automatic ORM;
- automatic RPC generation;
- automatic response caching;
- HTML streaming as default semantics;
- custom template-language syntax;
- compatibility with legacy TSP global APIs;
- preserving `main.ts`.

These may be implemented later only if they do not weaken the core lifecycle and module rules.

---

# 70. Reference execution flow

The required externally observable behavior can be summarized by this reference flow:

```text
HTTP request
   │
   ▼
Native HTTP runtime
   │
   ├── normalize URL
   ├── security checks
   ├── static/route resolution
   ▼
Route PageSlot
   │
   ├── current generation clean?
   │       │
   │       ├── yes ───────────────┐
   │       │                       │
   │       └── no                 │
   │            │                  │
   │            ▼                  │
   │       build candidate         │
   │            │                  │
   │       success?                │
   │        │      │               │
   │       yes     no              │
   │        │      │               │
   │        ▼      └─> retain LKG  │
   │   atomic publish              │
   │        │                      │
   └────────┴──────────────────────┘
            │
            ▼
      pin generation
            │
            ▼
       build Context
            │
            ▼
        GET/POST/...
            │
      ┌─────┴─────┐
      │           │
      ▼           ▼
   HtmlNode     Response
      │           │
      ▼           │
 TSP renderer     │
      │           │
      └─────┬─────┘
            ▼
    merge runtime cookies
            │
            ▼
       HTTP response
            │
            ▼
   release request pin
```

---

# 71. Reference module lifecycle

```text
Canonical source graph
       │
       ▼
PageSlot A
  Generation 17
    A.tsp
    component.tsx
    lib.ts
       │
       │ source changes
       ▼
Candidate Generation 18
    A.tsp
    component.tsx
    lib.ts (new)
       │
       ├── parse/transpile failure -> discard candidate
       ├── evaluation failure      -> discard candidate
       ├── validation failure      -> discard candidate
       │
       └── success
             │
             ▼
       atomic publish
             │
        ┌────┴─────┐
        ▼          ▼
 old requests   new requests
 Gen 17         Gen 18
        │
        ▼
 Gen 17 becomes unreferenced
        │
        ▼
 eligible for reclamation
```

---

# 72. Reference architecture boundary

The following division is normative at the ownership level, though exact code placement is implementation-defined.

## Native/persistent runtime ownership

The runtime MUST own or coordinate:

- HTTP listener;
- route table;
- filesystem security;
- static-file serving;
- configuration;
- request lifecycle;
- session store lifecycle;
- persistent service registry lifecycle;
- dependency graph metadata;
- watcher;
- PageSlot state;
- generation publication state;
- LKG state.

## Page-generation ownership

A page generation owns:

- evaluated `.tsp` module instance;
- evaluated reloadable local dependency instances;
- generation-local top-level JavaScript state;
- handler and fragment callable values associated with that generation.

## Persistent JavaScript/native domain

The persistent execution domain MAY own:

- `tsp:*` built-ins;
- native wrapper prototypes;
- external package module instances;
- JavaScript adapters for persistent services.

No implementation is required to expose these ownership categories directly to application code.

---

# 73. Design rationale summary (non-normative)

This section is explanatory and non-normative.

The specification deliberately chooses a narrow `.tsp` contract because source format is the most expensive compatibility surface to change later.

Using ordinary TSX avoids creating a parser, formatter, language server, source-map format, and template type system unique to TSP.

Named HTTP exports make routes statically understandable and eliminate wrapper magic.

Prohibiting `.tsp` imports makes a page a clear lifecycle boundary and allows each page to own an independently replaceable generation.

Generation-scoped application modules solve a difficult hot-reload problem: new requests can use coherent new code while old requests safely finish on coherent old code.

Persistent services solve the complementary state problem: durable resources do not disappear when page code is replaced.

A custom server JSX runtime removes React from the core and allows TSP to support server-oriented features such as async components without inheriting a browser reconciliation model.

---

# 74. Minimal current contract implementation bar

A runtime MUST NOT be marketed as a conforming TSP current contract runtime unless it can demonstrate all of the following:

1. execute `.tsp` as TypeScript/TSX;
2. enforce the `.tsp` export contract;
3. route filesystem pages including dynamic parameters;
4. provide Web Request/Response handler semantics;
5. render TSP JSX without requiring React;
6. support async server components;
7. provide fragments through runtime-generated URLs;
8. provide request Context including cookies, session, services, and AbortSignal;
9. preserve persistent session/service state across page reload;
10. maintain dependency-aware page invalidation;
11. construct immutable replacement generations;
12. atomically publish successful generations;
13. pin old in-flight requests to their old generation;
14. preserve Last Known Good after candidate failure in development;
15. block application imports of `.tsp`;
16. provide source-mapped development errors;
17. enforce filesystem traversal protection;
18. serve production errors without leaking source by default;
19. provide a native startup path that does not depend on application `main.ts`;
20. pass the normative conformance test categories in this specification.

---

# 75. Canonical definition

TSP can be summarized as:

> **A native server runtime in which `.tsp` files are isolated TypeScript/TSX route modules, executed as immutable page generations in JavaScriptCore, rendered by a TSP-owned server JSX runtime, while HTTP lifecycle and durable application state are owned by the persistent native runtime.**

That sentence is the architectural compatibility target for TSP.
