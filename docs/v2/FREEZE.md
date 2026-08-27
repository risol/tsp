# TSP v2 — Frozen Contract (Phase 0 deliverable)

> Status: **FROZEN — TSP v2.0 contract, signed off by Sol 2026-08-24**
> Date: 2026-08-24
> Owner: Mavis, on behalf of Sol

This document is the v2.0 application-facing contract. Any later
spec change that contradicts a frozen item must come with an
ADR (plan §69).

This document is the consolidation of the 12 contract items `tsp-v2-plan.md`
§60 says must be frozen before any non-trivial user code lands. The answers
are extracted from `tsp-v2-specification.md` (the 1817-line normative
document) and `tsp-v2-plan.md` §3-§60. Once Sol signs off, **the items
below are the API surface application code is allowed to rely on**; later
slices (in-process JSC bridge, watcher + atomic reload, watcher-free
reload via filesystem diff, etc.) build on these, they do not renegotiate
them.

The four supporting topic documents — `spec.md`, `tsp-module.md`,
`jsx-runtime.md`, `context.md` — are the deeper rationales. Each freeze
item below points at the relevant topic doc and at the source spec / plan
section so a reader can verify the answer is not a local invention.

---

## The 12 frozen items

### 1. `.tsp` is standard TSX

`.tsp` files MUST parse with a standard TypeScript / JSX parser. No custom
template language, no `<template>`, no `<script>`, no `{% %}`, no
`@page` / `@fragment` decorators (plan §3.1).

What this freezes for application code:
- Standard TSX syntax is always valid.
- A `.tsp` file can be opened in any editor with TSX support and the
  syntax is highlighted / typed / linted correctly.
- A `.tsp` file can be parsed, statically analysed, and transpiled by
  the host's bundler / transpiler / IDE without custom frontends.

Evidence: `tsp-v2-specification.md` §3.1, `tsp-v2-plan.md` §3.1.

### 2. `.tsp` modules cannot be imported

`.tsp` files MUST NOT appear in any `import` statement. The host reports
`TSP2003: .tsp modules are route entry modules and cannot be imported.
Move reusable code to .ts or .tsx.` and refuses to start (plan §5.1).

What this freezes for application code:
- Reusable code lives in `.ts` / `.tsx` / `.js` / `.jsx` and is imported
  normally.
- `.tsp` is a route entry point, not a library module. Two routes that
  need to share a component put the component in `components/*.tsx`.
- No two route entries can share a `.tsp` instance -- they are
  generation-root, not module-graph interior.

Evidence: `tsp-v2-specification.md` §5.1, `tsp-v2-plan.md` §5.1.

### 3. Route file system mapping rules

The route table is built from the `routes/` directory (configurable via
`tsp.toml [routes] dir`). The mapping is:

```text
routes/index.tsp             /
routes/login.tsp             /login
routes/users/index.tsp       /users
routes/users/new.tsp         /users/new
routes/users/[id].tsp        /users/:id
routes/posts/[slug].tsp      /posts/:slug
```

Segment name pattern: `[A-Za-z_][A-Za-z0-9_]*`. Dynamic segments surface
as `ctx.params.<name>`. Catch-all is `[...path]` only; optional catch-all
is **not** in v2.0. Priority is static > dynamic > catch-all, and
ambiguous pairings (e.g. `routes/users/[id].tsp` next to
`routes/users/[name].tsp`) make the host refuse to start with
`TSP1004: ambiguous routes ...`.

What this freezes for application code:
- Route paths are derived from filenames; there is no separate route
  table to maintain.
- Renaming a file moves its URL; deleting a file removes its route.
- Dynamic segment values are accessible as `ctx.params.<name>`.

Evidence: `tsp-v2-specification.md` §6, `tsp-v2-plan.md` §6.

### 4. Named HTTP method exports

A `.tsp` file exports one or more of the standard HTTP verbs as named
function exports:

```ts
export const config = ...;

export function GET(ctx) {}
export function POST(ctx) {}
export function PUT(ctx) {}
export function PATCH(ctx) {}
export function DELETE(ctx) {}
```

`HEAD` and `OPTIONS` are NOT standard exports: when a route exports `GET`
but no `HEAD`, the host synthesises a body-less `HEAD` from the `GET`
response; when a route omits `OPTIONS`, the host synthesises a 204
`Allow: <methods>` (plan §42). Default exports are NOT interpreted as
page handlers (plan §4.2.1) -- they are a type error.

What this freezes for application code:
- Page handler discovery is by named export of the standard verb; the
  method name is the contract, no registry / decorator.
- A `.tsp` file's URL is the same regardless of which verb is called;
  the host dispatches to the right export.
- A 405 from the host carries a real `Allow:` header listing the verbs
  the file actually exports.

Evidence: `tsp-v2-specification.md` §4.2, §42; `tsp-v2-plan.md` §4.2,
§42.

### 5. `HandlerResult = HtmlNode | Response`

A handler's return value MUST be one of:

- an `HtmlNode` (the TSP JSX runtime's opaque element), or
- a standard `Response` object.

Implicit shapes are rejected:

- `return { redirect: "/x" }`     -> `TSP3001` (use `redirect(...)`)
- `return { status: 404, body: "x" }` -> `TSP3001` (return a `Response`)
- `return "string"` / `return 42` -> `TSP3001`

What this freezes for application code:
- No shape magic. The handler's return is always an explicit value.
- `redirect(...)`, `json(...)`, `text(...)`, `html(...)`, `notFound()`
  helpers from `tsp:server` are the idiomatic way to build a `Response`
  without `new Response(...)`.

Evidence: `tsp-v2-specification.md` §4.3, §10; `tsp-v2-plan.md` §4.3,
§10.4.

### 6. `Context` minimal API

The `Context` argument passed to every handler has the following shape:

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

`Context` is per-request (not per-page), owned by the native runtime, and
becomes invalid after the handler returns (subsequent access raises
`TSP RuntimeError: Request context is no longer active`). Async services
SHOULD accept `ctx.signal` and propagate cancellation.

What this freezes for application code:
- The above surface is the entire context. Application code does not get
  a richer Context via subclass / monkey-patch.
- `ctx.params` is `Readonly<Record<string, string>>` -- always strings,
  even for path-shaped values; convert at the call site.
- `ctx.fragment(name)` returns the URL string the host assigned the
  fragment; the host owns the internal route (NOT a hard-coded
  `/__tsp/fragment/...` path).

Evidence: `tsp-v2-specification.md` §8, `tsp-v2-plan.md` §8.

### 7. `fragment()` API

A fragment is a reusable page subtree reachable via an internal URL:

```ts
import { fragment } from "tsp:server";

export const list = fragment({
  method: "GET",
  async handler(ctx) {
    const users = await ctx.services.users.list();
    return <UserList users={users} />;
  },
});

// Inside a handler:
ctx.fragment("list");
// -> "/_tsp/fragment/<route-id>/list" (or equivalent -- shape is
//    host-defined; the application MUST NOT rely on the path layout)
```

Default method is `GET` (plan §14.4). The v2.0 shape is `fragment(handler)`
or `fragment({ method, handler })`; the latter is the recommended form
for explicit declaration.

What this freezes for application code:
- Fragments are declared via the `fragment()` helper; they live as named
  exports of the `.tsp` file.
- The fragment URL is opaque to the application -- never hard-code a
  path, always go through `ctx.fragment("name")` or the future
  `fragmentUrl(name)` helper.

Evidence: `tsp-v2-specification.md` §14, `tsp-v2-plan.md` §14.

### 8. `tsp:*` builtin module naming

The host exposes three builtin module **names** (`tsp:server`,
`tsp:html`, `tsp:runtime`). No more, no less. Subsequent slices may
add named exports to an existing module; adding a fourth module name
would be a contract break (see Amendment log at the end of this
document).

The current **importable** surface (the names a page may
`import { ... } from "tsp:server"`):

```text
tsp:server (Amendment 1 surface, 2026-08-27)
  Response builders (v2.0)
    fragment, json, redirect, text, html, notFound, HttpError
  HTML escape (v2.0, also re-exposed under tsp:html)
    raw
  ID generation (Amendment 1)
    nanoid, customAlphabet, customRandom, random
  Validation library (Amendment 1)
    zod                              (zod 4.4.3, embedded)
  Database factory (Amendment 1)
    sql                              (Bun.SQL factory; per-worker pool
                                      via bun:sql)
  Bun builtin helper namespace (Amendment 1, password merged in
  Amendment 2)
    util                             (18 surfaces: randomUUIDv7, hash,
                                      CryptoHasher, Glob, TOML, YAML,
                                      markdown, escapeHTML, gzipSync,
                                      gunzipSync, file, write, which,
                                      peek, deepEquals, deepMatch,
                                      nanoseconds, env, password)
    -- password                      (`Bun.password` native --
                                      bcrypt / argon2id / scrypt;
                                      page reaches it as
                                      `util.password.hashSync(...)`)

tsp:html (v2.0 + Amendment 1)
  raw                                (HTML escape; same `__tspRaw__`
                                      as on tsp:server, so a page that
                                      already imports `raw` from one
                                      module does not need to re-import
                                      from the other)

tsp:runtime (v2.0)
  runtime.version, runtime.env, runtime.development
```

What this freezes for application code:
- `import { ... } from "tsp:*"` is the only framework import surface.
- There is no `globalThis.Page`, no `globalThis.Fragment`, no
  `globalThis.__tspBuiltins`. (plan §16.4) Explicit imports only.
- The host does NOT expose internal APIs through the runtime module
  (no service registry handles, no JSC value inspectors).
- The `Context` and `PageConfig` types referenced in earlier drafts of
  this item are **handler-signature types**, not importable names.
  The handler signature `export function GET(ctx: Context, cfg:
  PageConfig)` is the only way to reach them; there is no
  `import { Context, PageConfig } from "tsp:server"`.
- JSX runtime pieces (`HtmlNode`, `TrustedHtml`, `escape`) are
  applied by the transpiler at JSX-expansion time; they are not
  importable names. Pages do not need to import anything to use JSX.
- The bun builtin helpers in `util` are surfaced read-only and as
  individual properties — the wrapper deliberately omits
  `Bun.env.toJSON()` to prevent pages from dumping all env vars
  (would leak DB_PW / API_KEY per plan §17.1).
- High-risk bun builtins are intentionally NOT exposed: `Bun.serve`,
  `Bun.spawn`, `Bun.FFI`, `Bun.S3Client`, `Bun.connect`, `Bun.mmap`,
  `Bun.Cookie`, `Bun.Transpiler`. Pages needing them belong in the
  host layer, not in the import surface.

In addition to the `tsp:*` importable names above, the
`Context` argument the host injects into every page handler
exposes a per-request view of the cross-request state the
host itself owns. These are **not** `tsp:server` exports;
they ride on the handler signature. The contract for them is:

```text
ctx.request       Web `Request` (read body, json, formData, etc.)
                  - body is delivered as a `Blob` so
                    `await ctx.request.formData()` works for
                    both `multipart/form-data` and
                    `application/x-www-form-urlencoded`;
                    binary file parts keep byte fidelity
                    (Bun's multipart parser sees the raw
                    body bytes; no UTF-8 lossy decode)
                  - `formData()` throws on non-parseable
                    bodies (e.g. plain text or missing
                    boundary); the page is expected to
                    `try/catch` and surface the error
ctx.url           URL parsed from the request path + query
ctx.params        Route parameters (spec sect.11.3 / 11.4)
                  - `Record<string, string>` for one-segment
                    dynamic segments (`[id].tsp` -> `:id`)
                  - catch-all (`[...path].tsp`) binds the
                    remaining segments joined by `/`; an
                    empty path binds `""` (the catch-all
                    matches zero or more segments)
                  - the host serializes the matched `params`
                    into the `Context` JSON; the wrap
                    preamble leaves it as `ctx.params` for
                    the page handler (no per-field hydration
                    needed because the host already does
                    percent-decode and segment splitting)
                  - priority rule (spec sect.11.6): static
                    > dynamic > catch-all; if two routes
                    collide, the host refuses to start with
                    `TSP1004: ambiguous routes ...`
ctx.cookies       (Amendment 1)  get/has/set/delete
                  - get returns `undefined` for missing keys
                    (JS `Map.get` convention; coalesce with
                    `?? null` if the page wants JSON-stable
                    output, since JSON.stringify drops
                    `undefined` values)
                  - set writes to a per-request buffer the
                    async IIFE merges into the response
                    `headers` as separate `Set-Cookie` lines
                    (multi-value merge, no header flatten)
                  - delete emits `Set-Cookie: <name>=; Max-Age=0`
                    by default
ctx.session       (Amendment 1)  id / get / has / set / delete
                  / clear / regenerate / destroy
                  - `id` is the host's `SessionView.id`; on a
                    first request this is a freshly-minted id,
                    on subsequent requests it is the value of
                    the `tsp_sid` cookie
                  - writes buffer into the envelope's
                    `session_writes` array; the host applies
                    them to its process-lifetime SessionService
                    and plants `Set-Cookie: tsp_sid=...` on the
                    response when the id changes (new /
                    regenerate / destroy)
                  - backends: in-memory (default, dev) or
                    Redis (production); plan §18
ctx.services      host-owned service descriptors, snapshot
                  per request (plan §17.5)
ctx.fragment      fragment URL builder (plan §14)
```

The host's `SessionService` (with `MemoryBackend` and
`RedisBackend` implementations) and the response builder
that merges cookie / session writes are part of the
contract; tests pin both at the unit level
(`session_backend.rs` + `host.rs` Set-Cookie emission
tests) and at the e2e level (the
`session_runtime_mints_regenerates_and_destroys_session_id`
and
`cookies_runtime_parses_request_and_emits_set_cookie_on_write`
tests in `tests/start_order.rs`).

Evidence: `tsp-v2-specification.md` §16, `tsp-v2-plan.md` §16, §17.3, §18.

### 9. JSX child / attribute escaping semantics

The TSP JSX runtime escapes HTML by default. The child-rendering rules
are:

| child value            | output                              |
|------------------------|-------------------------------------|
| `null` / `undefined`   | empty                               |
| `true` / `false`       | empty                               |
| number                 | its string form (e.g. `42`)         |
| string                 | HTML-escaped (`<script>` -> `&lt;script&gt;`) |
| array                  | recursively flattened, each item per this table |
| `HtmlNode`             | rendered as its element              |
| anything else (object) | `TSP3102: object cannot be rendered as an HTML child` |

Attributes: string values are HTML-escaped; booleans are rendered as
the bare attribute name when `true` and dropped when `false` /
`null` / `undefined`; numbers are stringified. Function-valued
attributes (e.g. `onClick={fn}`) are a compile-time / runtime error
(`TSP3105: function-valued HTML attributes are not serializable`).
Raw HTML goes through `raw(trustedHtml)` from `tsp:html`, never through
`dangerouslySetInnerHTML`-style magic.

What this freezes for application code:
- XSS-by-default is impossible: any string child is escaped.
- Components that want unescaped HTML must explicitly opt in via
  `raw(...)`; the resulting value carries a `TrustedHtml` brand so the
  renderer can audit the call.
- Event handlers on the server are rejected -- client interactivity
  arrives through fragment URLs + form posts, not onClick.

Evidence: `tsp-v2-specification.md` §11.4-§11.6, `tsp-v2-plan.md`
§11.4-§11.6.

### 10. Async components

Async function components are first-class:

```tsx
async function UserName({ id }: { id: number }) {
  const user = await db.users.get(id);
  return <span>{user.name}</span>;
}
```

The TSP JSX renderer accepts `Promise<HtmlNode>` in any child position
and awaits it before rendering. Page handlers can be `async` too; the
return type is `Promise<HtmlNode | Response>`.

What this freezes for application code:
- `async function` components are the idiomatic way to fetch data; no
  `useEffect` / `useState` hooks, no React-isms.
- The renderer flattens nested promises, so a component can `await`
  multiple sources before returning.

Evidence: `tsp-v2-specification.md` §12.2, `tsp-v2-plan.md` §12.2.

### 11. PageConfig fields

`export const config = { ... } satisfies PageConfig;` accepts:

```ts
interface PageConfig {
  auth?: "none" | "optional" | "required";
  cache?: "no-store" | "private" | "public";
  bodyLimit?: number;       // bytes; cannot exceed global hard limit
  timeoutMs?: number;       // request-level timeout
  methods?: readonly HttpMethod[];  // declared, validated against exports
}
```

`config` is statically analysable. `await fetch(...)` at the top of a
file to compute `config` is rejected (`config` MUST be a plain object,
not the result of an async initialiser).

What this freezes for application code:
- The five fields above are the entire config surface for v2.0.
- `config` is read at module evaluate time; runtime-mutated `config` is
  undefined behaviour.
- `bodyLimit` is per-page; the global hard limit is in `tsp.toml` and
  applies first (page config cannot raise the ceiling).

Evidence: `tsp-v2-specification.md` §7, `tsp-v2-plan.md` §7.

### 12. Generation / LKG request-visible semantics

A "generation" is one immutable instance of a page's module namespace.
When a watched file changes, the host builds a candidate generation N+1
while generation N continues serving in-flight requests. The atomic
publish happens when:

1. candidate N+1 reads the file,
2. resolves imports,
3. transpiles TS/TSX,
4. instantiates the JSC ESM module,
5. evaluates the module,
6. validates the exports, and
7. passes the export validation.

On success, N+1 becomes current; old requests retain N until they
finish; N is then retired. On failure, current stays N; in development
the error is rendered, in production the LKG (Last Known Good)
generation serves and a metrics counter increments. The build of
candidate N+1 is deduped: a dirty PageSlot only has one in-flight
build, other concurrent requests await the same future (dev) or stay
on LKG (production, the recommended default).

What this freezes for application code:
- In-flight requests always see the generation they started on -- the
  handler does not change mid-execution.
- Compile failure never breaks live traffic in production.
- A page that has never successfully loaded returns 500 even in
  production (no LKG yet).
- A page can be served by LKG for an arbitrary amount of time after a
  change is detected; there is no deadline.

Evidence: `tsp-v2-specification.md` §21-§24, `tsp-v2-plan.md` §21-§24.

---

## What is NOT frozen (explicitly out of Phase 0 scope)

These are intentionally deferred to slice 9+ and are NOT part of the
contract application code can rely on:

- 404 / 500 custom error pages (`routes/_404.tsp` etc.) -- the host's
  built-in error pages are the v2.0 default; user-customisable pages
  land when the context bridge is in place (slice 9+).
- Middleware / global hooks -- plan §44 explicitly defers the JS
  middleware chain. v2.0 uses PageConfig + auth service hooks, not
  Express-style `(req, res, next)`.
- Streaming / partial responses -- plan §13 reserves for v2.1. v2.0
  renders the full body before sending.
- Client hydration / React-isms -- the HtmlNode ABI is independent of
  React. `@tsp/react` (plan §66) is a future opt-in compatibility
  package, not v2.0.
- WebSocket framework / server actions / ORM / page cache / permission
  sandbox / cluster-wide HMR (plan §67 explicit deferrals).
- Nested layouts as a directory convention (plan §15) -- v2.0 uses
  plain component composition.

### Spec §67 items NOT frozen by Phase 0 (deliberate scoping)

`tsp-v2-specification.md` §67 lists 25 "frozen protocol decisions"
that should be treated as protocol freeze candidates before v2.0
is declared stable. Phase 0 froze 12 of them (the ones matching
plan §60's 12-item freeze list). The remaining 13 are **not**
frozen by this document; they are guidance, not contract. If any
of them needs to become a contract for v2.0 application code,
re-open Phase 0 with an ADR (plan §69) and add the item here.

Not frozen in Phase 0 (from spec §67, items 4, 6, 8, 13, 14, 19,
21, 22, 23, 24, 25, plus fragments of items already partially
covered by the 12 frozen entries):

- **No default export from `.tsp` modules** (spec §67.4) -- the 12
  frozen items cover named HTTP method exports but not the
  negative contract "no `export default`". Open question: is a
  default export a hard error, or a warning? Currently the host
  silently ignores it.
- **Unknown runtime exports are invalid** (spec §67.6) -- the
  slice 5 detector only looks for `GET/POST/...` exports; an
  unknown `export function FOO()` is silently ignored. The spec
  intends this to be a hard error. Defer to a future slice that
  adds export validation per plan §48.
- **No arbitrary object response magic** (spec §67.8) -- covered
  by freeze item 5 (`HandlerResult = HtmlNode | Response`), but
  the negative side ("`return { redirect: '/x' }` MUST NOT
  redirect") is not enforced; the host currently 500s. Freezing
  the negative side means turning the 500 into a typed error
  with code `TSP3001` and a clearer message. Already exemplified
  in `docs/v2/examples/10-shape-magic.tsp`; the host enforcement
  is the missing piece.
- **`ctx.request` and explicit `Response` follow Web API
  semantics** (spec §67.13) -- the Context bridge is Phase 7;
  PoC 1 fixture uses a zero-arg `GET()` signature. Web `Request`
  / `Response` adoption is part of the Phase 7 work.
- **Durable state lives in persistent services, not page
  modules** (spec §67.14) -- the service registry (plan §17) is
  not built yet. v2.0 will not have a service contract until
  Phase 8.
- **Shared application-local modules are not guaranteed
  process-wide singleton across PageSlots** (spec §67.19) -- the
  in-process JSC bridge is what would make this observable; with
  the subprocess bridge each PageSlot already has its own
  bun.exe process, so the question is moot until slice 14+.
- **Framework globals are not required** (spec §67.21) -- v2.0
  has no global registry of services; imports from `tsp:*` are
  the only "global" surface. Confirming this is contract will
  come with the `tsp:server` builtin module (Phase 7).
- **Core native configuration does not require evaluating app
  JavaScript** (spec §67.22) -- the current `bin/tspserver_v2.rs`
  scans the routes dir at boot and the slice 11 watcher tracks
  files; if a `.tsp` file fails to parse (slice 5's static
  detector), the slot is not registered. This is "configuration
  without JS evaluation" by construction; freezing the negative
  side is mostly a reminder for future slices not to add a
  "validate by running JS at boot" step.
- **Fragment URL layout is internal and obtained through runtime
  APIs** (spec §67.23) -- fragments (plan §14) are not built
  yet; this lands with the fragment slice.
- **React / browser hydration is outside the core v2.0
  contract** (spec §67.24) -- already implied by the no-default-
  export / no-globals decisions, but worth a separate freeze so
  the future `@tsp/react` opt-in (plan §66) is unambiguously
  "outside the contract".
- **Layout / middleware magic is intentionally deferred**
  (spec §67.25) -- same as the "Middleware" deferral above; the
  spec explicitly states this is not v2.0 scope.

These 13 are tracked as the **Phase 0.5** candidate list. Each
one's promotion to "frozen" requires a separate Sol sign-off, at
which point it moves from this section into the numbered list
above and gets a commit hash.

---

## Sign-off

Phase 0 closed 2026-08-24 when Sol confirmed the 12 items above as
the v2.0 contract. Mavis updated `tsp-v2-specification.md` to mark
the corresponding sections as "frozen" and any later spec change
that contradicts a frozen item must come with an ADR (plan §69).

The 12 frozen items are now the surface application code is
allowed to rely on. Subsequent slices (in-process JSC bridge,
watcher + atomic reload, full Context bridge, ...) build on
these, they do not renegotiate them.

---

## Amendment log

Additive changes to the contract that do **not** contradict a frozen
item. Each entry records what was added, the slice that added it,
and the rationale. The original v2.0 sign-off above is not
overturned; the amendments are strictly add-on.

### Amendment 1 (2026-08-27) — `tsp:server` namespace expansion (slices 17 + 18)

**What changed.** Item 8's importable surface grew from 9 names to 17
(plus 1 hidden wrapper detail: `Bun.env` re-wrapped to omit
`toJSON()`). The three module names are unchanged.

**Names added.**
- `nanoid`, `customAlphabet`, `customRandom`, `random`
  (slice 17e / BUG-0001 follow-up, commit `e821af4bca`)
- `zod` (slice 17b, commit `336d3d522f`; upgraded to zod 4.4.3 in
  commit `d62ac69c94`)
- `password` (slice 17c, originally bcryptjs in `336d3d522f`, then
  migrated to native `Bun.password` in `c0b802c340`; later
  **merged into `util` via Amendment 2** so the bun-builtins
  surface stays unified)
- `sql` (slice 17d, commit `336d3d522f`; `Bun.SQL` factory)
- `util` (slice 18, commit `756108d694`; namespace of 17 bun
  builtins + `password` after Amendment 2; see the item 8 surface
  listing)

**Why this does not contradict a frozen item.** Item 8 froze the
*module names* (three of them, no more) and the *import-only*
discipline (no `globalThis`, no leaked service registry handles).
Adding named exports to an existing module preserves both. The
discipline that "imports are the only framework surface" is
strengthened, not weakened: a page now has fewer reasons to reach
for a global.

**Naming discipline enforced.** Every new export follows the
PHP-FPM-style per-request fresh state rule (plan §17.1): the
imported value is a stateless library or a factory (`sql`,
`password`), never a singleton holding cross-request mutable state.
Pages that need cross-request state must go through `ctx.session`
(plan §18) or `ctx.services` (plan §17.5), not through `tsp:server`.

**Companion `ctx.*` surfaces.** Slices 16f (`ctx.cookies`) and
16k/l (`ctx.session`) ride on the auto-injected `Context`
argument, not on `tsp:server` imports. They are host-owned
state with a per-request view: the host's `SessionService`
holds the live session map (in-memory or Redis), the wrap
preamble buffers `ctx.cookies.set` / `ctx.session.set` writes,
and the host merges them into the response (Set-Cookie for
cookies, session_writes for session, Set-Cookie: tsp_sid
on id change). Both are pinned by the e2e tests in
`tests/start_order.rs`:
`cookies_runtime_parses_request_and_emits_set_cookie_on_write`
and
`session_runtime_mints_regenerates_and_destroys_session_id`.

**Dynamic route segments (slice 16e).** Slice 16e shipped the
radix-tree matcher (`bun/src/runtime/tsp/router.rs::lookup`)
with static / dynamic / catch-all patterns. The earlier v2.0
freeze text said `ctx.params` was "empty until dynamic
segments land"; that is now lifted — `ctx.params` carries
the matched segment values (one entry per `[name]` segment,
plus one entry for the catch-all `[...name]` binding the
remaining path joined by `/`). The e2e test
`dynamic_segments_and_catch_all_route_to_pages_with_params`
in `tests/start_order.rs` exercises 8 scenarios (single
dynamic, hyphenated value, static-wins-over-dynamic,
multi-segment, catch-all with 3 segments, catch-all with
zero segments, 404s for both unknown path and
trailing-slash-on-no-index). Spec sect.11.6 priority rule
(static > dynamic > catch-all) and the `TSP1004`
ambiguous-route refusal are part of the contract.

**Multipart / form data (slice 16g).** Slice 16g shipped
the raw-body-bytes transport: the host serializes the
request body as base64, the wrap preamble atob-decodes
back to a `Uint8Array` and feeds it to Bun's native
`Request` constructor as a `Blob` (not a bare `Uint8Array`
— Bun's multipart parser needs the Blob's duplex half to
read the body). The e2e test
`multipart_form_data_round_trips_through_real_binary`
in `tests/start_order.rs` exercises 5 scenarios: text-only
multipart, multipart with a text field + a text/plain
file (size + content-type + filename surface intact),
UTF-8 file content (emoji + CJK survive byte-fidelity at
26 bytes for "你好,世界! 🚀 café\n"), url-encoded
form bodies, and the failure shape (`formData()` throws
`ERR_FORMDATA_PARSE_ERROR` on non-parseable bodies; the
production `routes/upload.tsp` page wraps the call in
`try/catch` and returns a 500 with the
`formData-error: <message>` body so the e2e can assert on
the failure shape rather than timing out on a hang).

**Config-driven custom services (slice 22 prototype, plan
§17.5 / §21).** The host reads a JSON file pointed at by
`TSP_CONFIG` (default: `tsp.config.json`) at boot. Each
`services.<name>` entry the file declares is registered
as a host-owned singleton on the `ServiceRegistry`; pages
read `ctx.services.<name>` exactly the same way they
read the built-in `logger` / `session` / `time`. Only one
service kind is supported in the prototype:

```text
{
  "services": {
    "hits":  { "kind": "counter", "initial": 0 },
    "views": { "kind": "counter", "initial": 100 }
  }
}
```

`CounterService` holds a per-name `AtomicU64` that
post-increments on every `describe_json()` call (i.e. on
every request that snapshots the registry). The wire
shape the page reads is
`{ "kind": "counter", "name": "<n>", "value": <u64> }`
and `value` is frozen on the page side. A typo'd `kind`
is a hard error at boot. A missing file is fine (the
host logs `no config at <path>` and registers only the
three built-ins). The e2e test
`config_driven_counter_service_increments_across_requests`
in `tests/start_order.rs` writes a temp config declaring
`hits` (initial 0) and `views` (initial 100), spawns the
master with `TSP_CONFIG=<temp>`, and asserts the
counters increment by exactly 1 per request (1→2→3 for
hits, 101→102→103 for views). A second round in the same
test spawns a fresh master without `TSP_CONFIG` and
asserts the custom-service names report `null` (the page
falls back to the built-ins only). The host's boot log
also uses `ServiceRegistry::iter_names` (NOT
`snapshot`) so the summary line does not bump a
counter just by printing it.

**Body size cap + 413 (spec sect.14.2).** The host
reads `TSP_MAX_BODY_BYTES` (default 1 MiB) at boot
and rejects any request whose `Content-Length` header
exceeds the cap with a 413 Payload Too Large
response, without buffering the body. The error code
is `TSP2002` and the body carries `request body
exceeds limit` so a misconfigured client (e.g. a
test fixture that forgot to set the cap) fails fast
at the wire boundary rather than at the page. The
e2e test `body_size_cap_rejects_oversized_requests_with_413`
in `tests/start_order.rs` runs the real binary with
`TSP_MAX_BODY_BYTES=200` and pins four scenarios:
50-byte body -> 200 (echo len=50), 200-byte body
(at the cap) -> 200 (== cap is allowed), 201-byte
body -> 413 + TSP2002, and 1 KiB body (5x the cap) ->
413 + TSP2002. The cap check happens before any body
bytes are buffered, so a 50 MiB multipart upload
attempt is rejected with 413 + a small error body
instead of allocating the full 50 MiB in the host
process. Note: chunked transfer encoding (no
`Content-Length`) is not supported today -- such
requests are treated as empty bodies; a future
slice should add a streaming cap for that path.

**High-risk bun builtins explicitly excluded.** `Bun.serve`,
`Bun.spawn`, `Bun.FFI`, `Bun.S3Client`, `Bun.connect`, `Bun.mmap`,
`Bun.Cookie`, `Bun.Transpiler` are deliberately **not** exposed.
These would either give a page a sub-server on a different port,
a subprocess channel (RCE surface), a raw native call interface,
cloud-credential access, or a host-pipeline escape — none of which
fit the per-request, host-mediated model. If a future use case
needs one, it belongs in the host layer (`tsp.config.ts` services,
plan §17.5), not in the page import surface.

**Correction to the original item 8 listing.** The freeze's
v2.0 line listed `Context` and `PageConfig` as `tsp:server` exports.
The actual runtime has never exposed them as importable names — they
are handler-signature types (the `ctx: Context, cfg: PageConfig`
parameters of a `GET` / `POST` / `PUT` / `DELETE` export). The
amended item 8 above clarifies that the importable surface is
*values*, not types. JSX runtime pieces (`HtmlNode`, `TrustedHtml`,
`escape`) similarly are not importable; the transpiler wires them
in at JSX-expansion time. No code that shipped on or before the
v2.0 sign-off imported these names, so this is a documentation
correction rather than a behaviour change.

**Verification.** 234 tests green as of this amendment
(202 lib + 4 worker_integration + 15 process_model + 13 start_order
e2e, including `util_namespace_surfaces_bun_builtins_for_pages` for
the new `util` namespace, `zod_runtime_compiled_into_wrap_serves_validated_schemas`
for `zod`, `password_runtime_through_bun_password_serves_hashed_passwords`
for `password`, `sql_runtime_uses_bun_native_pool_for_page_local_datasource`
for `sql`, `nanoid_runtime_compiled_into_wrap_serves_distinct_ids`
for the nanoid family, and — added with this amendment —
`cookies_runtime_parses_request_and_emits_set_cookie_on_write` for
`ctx.cookies` (slice 16f),
`session_runtime_mints_regenerates_and_destroys_session_id` for
`ctx.session` (slice 16k/l),
`dynamic_segments_and_catch_all_route_to_pages_with_params` for
dynamic route segments + catch-all (slice 16e),
`multipart_form_data_round_trips_through_real_binary` for
`ctx.request.formData()` (slice 16g),
`config_driven_counter_service_increments_across_requests` for
config-driven custom services (slice 22 prototype), and
`body_size_cap_rejects_oversized_requests_with_413` for the
`TSP_MAX_BODY_BYTES` cap and 413 path (spec sect.14.2)).

### Amendment 2 (2026-08-27) — `password` merged into `util`

**What changed.** The `password` top-level export that
Amendment 1 added (slice 17c, native `Bun.password`
bridge) was moved under the `util` namespace. Pages now
reach the same `Bun.password` object as
`util.password.hashSync(...)` / `util.password.verifySync(...)`
instead of `password.hashSync(...)`. The top-level
`__tspServer.password` slot is gone; `password: Bun.password`
is a field inside the `__tspUtilNs__` freeze.

The rewriter's allow-list dropped `"password"` (the
`import { password }` shape now fails fast at transpile
time with `unsupported tsp:server named import`), and
`password_prelude()` was removed from the wrap builder
(the password bridge is now a single line inside the
util builder).

**Why this does not contradict a frozen item.** The
freeze allows adding named exports to an existing
module; this amendment does the inverse for an existing
export. The `util` namespace already grouped the other
17 bun builtins; `password` was structurally identical
(`Bun.X` reference, 0 embed, 0 per-request state) and
the only thing distinguishing it was the topic ("security
vs. utility"). Topic-level grouping was a leaky design --
the page-side code does not care which "topic" a builtin
belongs to, only that it can reach it through a single
import. The discipline that "imports are the only
framework surface" is preserved; the import surface
shrinks from 16 names to 15 (one fewer top-level slot
to maintain).

**What changed in code.**
- `bun/src/runtime/tsp/jsx.rs`:
  - `__tspUtilNs__` gains `password: Bun.password` as
    its 18th field (the `env` wrapper is the only other
    field with a one-line host-side wrapper; `password`
    is just a direct `Bun.password` reference).
  - The `password: __tspPasswordNs__` slot is removed
    from the `__tspServer` freeze.
  - `password_prelude()` and its caller are removed.
  - The rewriter's allow-list drops the `"password"`
    arm.
- `bun/src/runtime/tsp/tests/start_order.rs`:
  - The inline `PASSWORD_TSP` fixture now uses
    `import { util } from "tsp:server"` and
    `util.password.hashSync(...)` /
    `util.password.verifySync(...)`.
  - The `password_runtime_through_bun_password_serves_hashed_passwords`
    e2e keeps its name (the test still pins bun's
    native password API) but the body it asserts against
    is the new `util.password` shape.
  - The module-level doc comment for the password
    e2e is updated to point at `util` instead of
    the top-level `password` slot.
- `routes/password.tsp` is rewritten to use
  `import { util }` + `util.password.*`. The HTTP
  shape (`GET /password`, `POST /password`) is
  unchanged; only the JS surface inside the page
  changes.
- The 3 password unit tests in `jsx.rs`:
  - `wrap_for_bun_cli_surfaces_bun_password_for_pages`
    now pins `password: Bun.password,` inside the
    `__tspUtilNs__` block (instead of the old
    `const __tspPasswordNs__ = Bun.password;` +
    `password: __tspPasswordNs__` two-line pattern).
  - `rewrite_tsp_server_imports_accepts_password_named_export`
    is renamed to
    `rewrite_tsp_server_imports_rejects_password_after_merge_into_util`
    and now asserts the rewriter **rejects**
    `import { password }` (the old positive
    contract is now covered by the `util` test).
  - `password_pipeline_generates_runable_module_under_real_bun`
    now uses `import { util }` + `util.password.*`
    in the source fixture, and asserts the rewriter
    emits `const { util } = __tspServer;` (not
    `const { password } = __tspServer;`).

**What was NOT changed in this amendment.**
- `routes/counter.tsp` and the slice 22
  `CounterService` plumbing are untouched (the
  config-driven `custom services` are a host-side
  service registry concept, not a `tsp:server` import).
- No `serde` dep was added; the config parser
  hand-rolls the same minimal JSON shape it did
  before.
- Production behaviour is unchanged: a page that
  hashed a bcrypt / argon2id string and verified
  it back gets the same boolean answer, in the same
  e2e test, just routed through `util.password`
  instead of the top-level `password`.

**Verification.** Same 234 tests green (202 lib +
4 worker_integration + 15 process_model + 13 start_order
e2e). The 3 password unit tests in `jsx.rs` change
contract (one now asserts a negative -- the rewriter
rejects the old shape) but they still cover the same
end-to-end behaviour: bun's native `Bun.password` is
reachable from a page through the namespace and the
result is a real bcrypt / argon2id hash. The
`password_runtime_through_bun_password_serves_hashed_passwords`
e2e is unchanged in name and intent.
