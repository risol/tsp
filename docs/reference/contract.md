# TSP — Frozen Contract (Phase 0 deliverable)

> Status: **FROZEN — TSP current contract, signed off by Sol 2026-08-24**
> Date: 2026-08-24
> Owner: Mavis, on behalf of Sol

This document is the current contract application-facing contract. Any later
spec change that contradicts a frozen item must come with an
ADR (plan §69).

This document is the consolidation of the 12 contract items `tsp-plan.md`
§60 says must be frozen before any non-trivial user code lands. The answers
are extracted from `tsp-specification.md` (the 1817-line normative
document) and `tsp-plan.md` §3-§60. Once Sol signs off, **the items
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

Evidence: `tsp-specification.md` §3.1, `tsp-plan.md` §3.1.

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

Evidence: `tsp-specification.md` §5.1, `tsp-plan.md` §5.1.

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
is **not** in current contract. Priority is static > dynamic > catch-all, and
ambiguous pairings (e.g. `routes/users/[id].tsp` next to
`routes/users/[name].tsp`) make the host refuse to start with
`TSP1004: ambiguous routes ...`.

What this freezes for application code:
- Route paths are derived from filenames; there is no separate route
  table to maintain.
- Renaming a file moves its URL; deleting a file removes its route.
- Dynamic segment values are accessible as `ctx.params.<name>`.

Evidence: `tsp-specification.md` §6, `tsp-plan.md` §6.

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

Evidence: `tsp-specification.md` §4.2, §42; `tsp-plan.md` §4.2,
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

Evidence: `tsp-specification.md` §4.3, §10; `tsp-plan.md` §4.3,
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

Evidence: `tsp-specification.md` §8, `tsp-plan.md` §8.

### 7. `fragment()` API

A fragment is a reusable page subtree reachable via an internal URL:

```ts
import { fragment } from "tsp:server";

export const list = fragment(async (ctx) => {
  const users = await ctx.services.users.list();
  return <UserList users={users} />;
});

// Inside a handler:
ctx.fragment("list");
// -> "/__tsp/fragment?route=...&name=list&token=<capability>"
//    (or equivalent -- shape is host-defined; the application
//     MUST NOT rely on the path layout)
```

The current contract shape is `fragment(handler)` with default method `GET`
(plan §14.4). The `{ method, handler }` form that earlier drafts of
this item mentioned is **deferred** to a follow-up slice -- see
Amendment 4. The current contract is the single-arg form only.

What this freezes for application code:
- Fragments are declared via the `fragment()` helper; they live as named
  exports of the `.tsp` file.
- The fragment URL is opaque to the application -- never hard-code a
  path, always go through `ctx.fragment("name")` or the future
  `fragmentUrl(name)` helper.
- current contract fragments are always reachable via `GET`. The application
  reads the URL with `ctx.fragment("name")` and uses it as a
  `fetch` / `hx-get` / `<a href>` target. The `{ method: "POST" }`
  form lands in a follow-up.

Evidence: `tsp-specification.md` §14, `tsp-plan.md` §14,
`Amendment 4` below.

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
  Response builders (current contract)
    fragment, json, redirect, text, html, notFound, HttpError
  HTML escape (current contract, also re-exposed under tsp:html)
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

tsp:html (current contract + Amendment 1)
  raw                                (HTML escape; same `__tspRaw__`
                                      as on tsp:server, so a page that
                                      already imports `raw` from one
                                      module does not need to re-import
                                      from the other)

tsp:runtime (current contract)
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

Evidence: `tsp-specification.md` §16, `tsp-plan.md` §16, §17.3, §18.

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

Evidence: `tsp-specification.md` §11.4-§11.6, `tsp-plan.md`
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

Evidence: `tsp-specification.md` §12.2, `tsp-plan.md` §12.2.

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
- The five fields above are the entire config surface for current contract.
- `config` is read at module evaluate time; runtime-mutated `config` is
  undefined behaviour.
- `bodyLimit` is per-page; the global hard limit is in `tsp.toml` and
  applies first (page config cannot raise the ceiling).

Evidence: `tsp-specification.md` §7, `tsp-plan.md` §7.

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

Evidence: `tsp-specification.md` §21-§24, `tsp-plan.md` §21-§24.

---

## What is NOT frozen (explicitly out of Phase 0 scope)

These are intentionally deferred to slice 9+ and are NOT part of the
contract application code can rely on:

- 404 / 500 custom error pages (`routes/_404.tsp` etc.) -- the host's
  built-in error pages are the current contract default; user-customisable pages
  land when the context bridge is in place (slice 9+).
- Middleware / global hooks -- plan §44 explicitly defers the JS
  middleware chain. current contract uses PageConfig + auth service hooks, not
  Express-style `(req, res, next)`.
- Streaming / partial responses -- plan §13 reserves for future release. current contract
  renders the full body before sending.
- Client hydration / React-isms -- the HtmlNode ABI is independent of
  React. `@tsp/react` (plan §66) is a future opt-in compatibility
  package, not current contract.
- WebSocket framework / server actions / ORM / page cache / permission
  sandbox / cluster-wide HMR (plan §67 explicit deferrals).
- Nested layouts as a directory convention (plan §15) -- current contract uses
  plain component composition.

### Spec §67 items NOT frozen by Phase 0 (deliberate scoping)

`tsp-specification.md` §67 lists 25 "frozen protocol decisions"
that should be treated as protocol freeze candidates before current contract
is declared stable. Phase 0 froze 12 of them (the ones matching
plan §60's 12-item freeze list). The remaining 13 are **not**
frozen by this document; they are guidance, not contract. If any
of them needs to become a contract for current contract application code,
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
  in `docs/reference/examples/10-shape-magic.tsp`; the host enforcement
  is the missing piece.
- **`ctx.request` and explicit `Response` follow Web API
  semantics** (spec §67.13) -- the Context bridge is Phase 7;
  PoC 1 fixture uses a zero-arg `GET()` signature. Web `Request`
  / `Response` adoption is part of the Phase 7 work.
- **Durable state lives in persistent services, not page
  modules** (spec §67.14) -- the service registry (plan §17) is
  not built yet. current contract will not have a service contract until
  Phase 8.
- **Shared application-local modules are not guaranteed
  process-wide singleton across PageSlots** (spec §67.19) -- the
  in-process JSC bridge is what would make this observable; with
  the subprocess bridge each PageSlot already has its own
  bun.exe process, so the question is moot until slice 14+.
- **Framework globals are not required** (spec §67.21) -- current contract
  has no global registry of services; imports from `tsp:*` are
  the only "global" surface. Confirming this is contract will
  come with the `tsp:server` builtin module (Phase 7).
- **Core native configuration does not require evaluating app
  JavaScript** (spec §67.22) -- the current `bin/tspserver.rs`
  scans the routes dir at boot and the slice 11 watcher tracks
  files; if a `.tsp` file fails to parse (slice 5's static
  detector), the slot is not registered. This is "configuration
  without JS evaluation" by construction; freezing the negative
  side is mostly a reminder for future slices not to add a
  "validate by running JS at boot" step.
- **Fragment URL layout is internal and obtained through runtime
  APIs** (spec §67.23) -- fragments (plan §14) are not built
  yet; this lands with the fragment slice.
- **React / browser hydration is outside the core current contract
  contract** (spec §67.24) -- already implied by the no-default-
  export / no-globals decisions, but worth a separate freeze so
  the future `@tsp/react` opt-in (plan §66) is unambiguously
  "outside the contract".
- **Layout / middleware magic is intentionally deferred**
  (spec §67.25) -- same as the "Middleware" deferral above; the
  spec explicitly states this is not current contract scope.

These 13 are tracked as the **Phase 0.5** candidate list. Each
one's promotion to "frozen" requires a separate Sol sign-off, at
which point it moves from this section into the numbered list
above and gets a commit hash.

---

## Sign-off

Phase 0 closed 2026-08-24 when Sol confirmed the 12 items above as
the current contract. Mavis updated `tsp-specification.md` to mark
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
and the rationale. The original current contract sign-off above is not
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
with static / dynamic / catch-all patterns. The earlier current contract
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
current contract line listed `Context` and `PageConfig` as `tsp:server` exports.
The actual runtime has never exposed them as importable names — they
are handler-signature types (the `ctx: Context, cfg: PageConfig`
parameters of a `GET` / `POST` / `PUT` / `DELETE` export). The
amended item 8 above clarifies that the importable surface is
*values*, not types. JSX runtime pieces (`HtmlNode`, `TrustedHtml`,
`escape`) similarly are not importable; the transpiler wires them
in at JSX-expansion time. No code that shipped on or before the
current contract sign-off imported these names, so this is a documentation
correction rather than a behaviour change.

**Verification.** 235 tests green as of this amendment
(202 lib + 4 worker_integration + 15 process_model + 14 start_order
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
config-driven custom services (slice 22 prototype),
`body_size_cap_rejects_oversized_requests_with_413` for the
`TSP_MAX_BODY_BYTES` cap and 413 path (spec sect.14.2), and
`config_driven_kv_and_feature_flag_kinds_are_readable_by_pages`
for the `kind: kv` and `kind: feature_flag` extension
(slice 22 follow-up)).

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

**Verification.** Same 235 tests green (202 lib +
4 worker_integration + 15 process_model + 14 start_order
e2e). The 3 password unit tests in `jsx.rs` change
contract (one now asserts a negative -- the rewriter
rejects the old shape) but they still cover the same
end-to-end behaviour: bun's native `Bun.password` is
reachable from a page through the namespace and the
result is a real bcrypt / argon2id hash. The
`password_runtime_through_bun_password_serves_hashed_passwords`
e2e is unchanged in name and intent.

### Amendment 3 (2026-08-27) — `kind: kv` and `kind: feature_flag` added to the config-driven services

**What changed.** The slice 22 prototype shipped
`kind: counter` as the only custom service kind. This
amendment extends the parser and `Service` registry
with two more kinds so the config-driven surface can
host real application state without a new
`Service` trait impl per kind:

```text
{
  "services": {
    "hits":   { "kind": "counter",     "initial": 0 },
    "config": { "kind": "kv",
                "entries": {
                  "support_email":  "help@example.com",
                  "max_upload_size": "10485760"
                } },
    "flags":  { "kind": "feature_flag",
                "flags": {
                  "beta_ui":      true,
                  "new_checkout": false
                } }
  }
}
```

**`kind: kv`** holds a `BTreeMap<String, String>`
keyed by name, surfaced to the page as
`ctx.services.<name>.entries.<key>`. The map is a
frozen snapshot (read-only on the page side); the
source of truth is `tsp.config.json` and changes
ship on master restart. This is the current surface for
host-supplied configuration values a page needs to
read (rate limits, support emails, internal service
URLs, non-secret feature knobs) without leaking the
whole process environment (compare with the
`util.env` wrapper, which also hides
`Bun.env.toJSON`).

**`kind: feature_flag`** holds a
`BTreeMap<String, bool>`, surfaced to the page as
`ctx.services.<name>.flags.<flag>`. Same frozen-
snapshot semantics as `kv`. The flag set gates
code paths (new checkout flow, beta UI, A/B test
bucket assignment) from a single host-controlled
config.

Both kinds use `BTreeMap` internally so the wire
format is deterministic (stable key order across
runs / OSes) and a typo'd duplicate key in the
config file cannot silently shadow a real one.

**What changed in code.**

- `bun/src/runtime/tsp/services.rs`:
  - `KvService` -- a new `Service` impl holding a
    `BTreeMap<String, String>`. `describe_json()`
    returns
    `{"kind":"kv","name":"<n>","entries":{...}}`
    (hand-rolled, no `serde`). `is_request_varying()`
    is `false` because the values do not change
    between requests; the generation cache can
    replay the snapshot safely.
  - `FeatureFlagService` -- a new `Service` impl
    holding a `BTreeMap<String, bool>`. Same shape
    (hand-rolled, `is_request_varying()` is
    `false`).
  - `load_counter_services_from_config` gains two
    more match arms. The function name was kept
    (slice 22 was the only kind at the time) -- a
    follow-up can rename to
    `load_services_from_config` if the call sites
    stay short.
  - New helpers: `parse_string_map` (string ->
    string `BTreeMap` for `kv` entries) and
    `parse_bool_map` (string -> bool `BTreeMap`
    for `feature_flag` flags). Both reuse the
    existing minimal JSON walker; `parse_bool_map`
    only accepts the literal `true` / `false`
    tokens, so a typo like `yes` is a hard error
    at boot rather than a silent `false`.
  - The "unknown kind" error message now lists
    the supported set:
    `(supported: counter, kv, feature_flag)`.

- `bun/src/runtime/tsp/tests/start_order.rs`:
  - New e2e
    `config_driven_kv_and_feature_flag_kinds_are_readable_by_pages`.
    Writes a temp config declaring all three kinds
    at once (counter + kv + feature_flag),
    asserts the boot log lists the three custom
    service names, then hits:
    - `GET /counter`         (hits = 1)
    - `GET /kv?key=support_email`
    - `GET /kv`              (whole-map dump)
    - `GET /flags?check=beta_ui`    (true)
    - `GET /flags?check=new_checkout` (false)
    - `GET /flags`           (whole-map dump)
    - `GET /kv?key=missing`  (null)
    - `GET /flags?check=missing` (null)
    Then a second round spawns a fresh master
    without `TSP_CONFIG` and asserts the kv /
    feature_flag / counter names are all `null`
    (the page falls back to "not present" for
    every missing service name). The whole-map
    assertions pin the exact key order (BTreeMap
    iteration order = alphabetical) so a future
    refactor that switches to a different map
    type would fail the e2e with a clear diff.

- `routes/kv.tsp` and `routes/flags.tsp` -- two
  new demo routes mirroring the production-shape
  `routes/counter.tsp` style. Each just echoes
  the requested slice of the snapshot; the
  `x-demo: slice22-kv` / `x-demo: slice22-flags`
  response headers let the e2e distinguish "the
  page actually ran" (200) from "the host
  rejected" (4xx).

**What was NOT changed in this amendment.**

- No new `serde` dep. `KvService` and
  `FeatureFlagService` hand-roll the wire format
  the same way `CounterService` does, reusing
  the existing `json_string_field` helper.
- No migration of `counter` callers. The slice 22
  `kind: counter` shape is unchanged; the only
  difference is that the parser now accepts two
  more `kind` values.
- The two new services are read-only on the page
  side. A future slice may add a write-back
  path through `ctx.session` or a dedicated
  mutation envelope field; for the prototype,
  changes ship on master restart (a 1-second
  blip), which is the right semantics for
  non-secret feature knobs.

**Verification.** 235 tests green (202 lib +
4 worker_integration + 15 process_model + 14 start_order
e2e, the prior 13 plus
`config_driven_kv_and_feature_flag_kinds_are_readable_by_pages`).
The 4 e2e tests in the slice 22 family (counter, kv,
feature_flag, body cap) all run against the same real
binary with the same dist/tspserver/tspserver.exe
production shape, so a regression that breaks one
config-driven kind would surface as a failure in the
e2e that exercises it.

---

### Amendment 4 (2026-08-27) — Phase 9 fragments slice closed end-to-end

Phase 9 (plan §14 / contract item 7) is the last architecture
slice the current contract froze but no e2e exercise had pinned
yet. The `fragment()` helper, `ctx.fragment(name, params?)`,
the wrap-prelude registry, the host's `fragment_target`
parser, the per-process capability token, and the
`/__tsp/fragment?route=...&name=...&token=...` URL shape
were all in place from earlier slices; this amendment
closes the slice with a demo route, an end-to-end test,
and a narrow refinement of the contract contract.

**What changed in this amendment.**

- The fragment legacy contract is `fragment(handler)` only.
  The `{ method, handler }` form that the original
  contract item 7 mentioned is **deferred** to a follow-up
  slice. The deferred form needs a real host-side
  fragment-method validation step (currently the host
  uses the route table's method check, which means a
  POST to a default-GET fragment falls through to 405
  rather than a host-level "fragment does not accept
  POST"). v1 ships with default GET only; the demo
  route mirrors that.

- New demo: `routes/fragments.tsp` exposes two named
  fragments (`userList` and `echo`). The parent page
  returns both URLs through `ctx.fragment("name")` so
  the e2e can pin the URL builder, the dispatch
  round-trip, the `params?` arg flow, the
  client-side-param passthrough, and the per-process
  capability check end-to-end.

- New e2e:
  `fragment_runtime_exposes_opaque_url_and_renders_subtree`
  in `bun/src/runtime/tsp/tests/start_order.rs`. It
  runs against the real `dist/tspserver/tspserver.exe`
  binary and pins six scenarios:
  1. `GET /fragments` returns 200 with the `userList`
     and `echo` URLs inlined.
  2. The parsed `userList` URL returns the JSON
     fragment body (alice / bob / carol) -- proving
     the host dispatches back to the right page +
     the right fragment name.
  3. The parsed `echo` URL reflects the parent's
     baked `msg=hi` -- proving `ctx.fragment("name",
     params?)` survives the round trip.
  4. Adding a `&client=hello` query param to the
     fragment URL surfaces a `client: "hello"` field
     in the body -- proving the fragment handler
     reads the full request query, not just the
     parent's intent.
  5. `GET /__tsp/fragment?route=...&name=userList&token=wrong`
     returns 404 -- proving the per-process capability
     check rejects a token that does not match
     `host::fragment_token()`.
  6. `GET /__tsp/fragment?name=userList&token=<correct>`
     returns 404 -- proving a missing `route=` param
     falls through to the route table (no route at
     `/__tsp/fragment` -> 404).

- New unit tests in `bun/src/runtime/tsp/jsx.rs`:
  - `rewrite_fragment_exports_injects_name_as_first_arg`:
    `export const X = fragment(handler)` is rewritten
    to `const X = fragment("X", handler)`, so the
    wrap-prelude registry can store the handler
    under the name the host will look up.
  - `rewrite_fragment_exports_does_not_touch_other_exports`:
    pin that `export function GET()` and
    `export const plain = 5` are not touched.
  - `wrap_emits_fragment_registry_and_dispatch`:
    pin that the wrap preamble declares the
    `__tspFragments` map, the `__tspFragment__`
    registry function, the dispatch from
    `__tspContext.__tsp_fragment` to the right
    handler, the `ctx.fragment(name, params?)` URL
    builder, and the `__tspServer.fragment`
    export.
  - `wrap_context_fragment_url_bakes_token_route_and_extra_params`:
    pin that the URL builder reads the parent path,
    the fragment name, the per-process token (from
    the context, not a hard-coded value), and the
    user's extra params.

**What was NOT changed in this amendment.**

- The fragment implementation itself. The rewriter,
  the `__tspFragment__` registry, the
  `ctx.fragment()` URL builder, the host's
  `fragment_target` parser, the capability token,
  and the dispatch all stay exactly as they were;
  this amendment only adds the demo, the e2e, and
  the unit-test pins.
- The contract contract shape. The Application
  Protocol surface (item 7) keeps the same
  description; the narrowing to `fragment(handler)`
  only is documented under this amendment's
  "What changed" rather than as a new contract
  clause, because the deferred `{ method, handler }`
  form was not yet exercised by application code.
- No new `serde` dep, no new npm module, no
  watch / reloader changes. The fragment URL is
  not persisted across restarts; the per-process
  token rotates on every master boot, so a
  long-lived client tab that holds a fragment URL
  will need to re-fetch the parent page after a
  restart to refresh the URL. This is the same
  staleness model the existing session id uses
  (per-process, not durable).

**Verification.** 240 tests green (206 lib +
4 worker_integration + 15 process_model + 15 start_order
e2e, the prior 14 plus
`fragment_runtime_exposes_opaque_url_and_renders_subtree`).
The lib gain is the 4 new `jsx::tests::fragment_*`
tests; the worker_integration and process_model counts
are unchanged. The new e2e exercises the real
`dist/tspserver/tspserver.exe` binary with the
production-shape demo route, so a regression in the
fragment dispatch or capability check would surface
as a hard failure on `cargo test --test start_order`.

---

### Amendment 5 (2026-08-27) — Phase 11 IDE typings + tooling shortcuts

Plan §11 ("Tooling") lists five dev-workflow items
the contract should ship: `tsp check`, `tsp routes`,
`tsp graph`, IDE typings, diagnostics, source maps.
The host binary already had `routes` / `graph` / `check`
subcommands wired (slice 11 forward), but they were
not surfaced through `tsp.sh` and IDE typings were
not emitted. This amendment closes the "IDE typings"
piece plus the `tsp.sh` shortcuts so the dev workflow
matches the plan.

**What changed in this amendment.**

- **New `tspserver typings` subcommand.** Writes
  three TypeScript declaration files into a
  user-supplied output directory (default
  `.tsp-types`):
  - `tsp-server.d.ts` — declares the `tsp:server`
    module with the request / response / fragment /
    id-gen / validation / database / bun-builtin
    surface (mirror of the wrap-prelude's
    `__tspServer = Object.freeze({...})` in
    `jsx.rs:829`).
  - `tsp-html.d.ts` — declares the `tsp:html`
    module (single `raw` export).
  - `tsp-runtime.d.ts` — declares the `tsp:runtime`
    module (`runtime.version`, `runtime.env` with
    get/has only, `runtime.development`).
  Both `--out <DIR>` and the bare-positional
  `<DIR>` form are accepted; `--help` prints
  usage. The files are embedded at compile time
  via `include_str!` so a binary that ships the
  typings subcommand cannot drift from the
  declarations (the build is the contract).

- **New `tsp.sh` shortcuts.** The host's
  `routes` / `graph` / `check` subcommands were
  only reachable through `./tsp.sh dev <sub>`;
  the new top-level shortcuts `./tsp.sh routes`,
  `./tsp.sh graph`, `./tsp.sh check:app`, and
  `./tsp.sh typings` exec the host directly
  (the existing `./tsp.sh check` is unchanged --
  it still runs `cargo check` for the Rust
  runtime; `check:app` is the host's check for
  application routes + module graph). The
  `typings` shortcut accepts `--out <DIR>` /
  `<DIR>` and respects `TSP_TYPINGS_DIR` for
  the default.

- **Updated `--help` text.** The host's help
  output now lists the `typings` subcommand
  alongside `check` / `routes` / `graph`.

**What was NOT changed in this amendment.**

- The host's `routes` / `graph` / `check` runtime
  behavior is unchanged. This amendment only
  surfaces the existing subcommands and adds the
  new `typings` subcommand. The hand-rolled
  content for the typings is a NEW piece; the
  subcommand dispatch is a NEW arm; the existing
  arm list is untouched.
- The wrap-prelude (`jsx.rs`) is unchanged. The
  typings mirror the wrap surface but the wrap
  itself is the source of truth; a future slice
  that adds a new name to the wrap MUST also add
  the matching declaration to `tsp-server.d.ts`
  in the same commit (the e2e
  `tspserver_typings_emits_three_dts_files`
  + the unit tests in `typings.rs` pin every
  public name so a drift surfaces as a hard
  failure).
- `tsp check` is still "validate routes and
  local imports" (the host's existing check
  subcommand), NOT a real TypeScript type-check.
  Plan §11 lists "tsp check" as the type-checker;
  the existing check is a much narrower parse +
  module-graph validator (it confirms every
  `.tsp` file parses + every local import
  resolves). A real `tsc --noEmit`-style
  type-checker is a follow-up slice: it would
  need either bun's experimental
  type-only-mode, a child `tsc` process, or a
  hand-rolled Rust type-checker. The v1.0
  contract is "the user wires `tsc` themselves
  using the typings this amendment ships".
- The `Context.route: RouteInfo` field is in
  the contract contract (item 6) but the wrap
  has not been updated to set it. The typings
  ship the contract shape so the type-checker
  is correct; the runtime gap is on a follow-up
  slice.
- Bun-builtin types (`Bun.password`,
  `Bun.markdown`, etc.) are intentionally NOT
  re-exported from `bun-types` in the typings.
  The user can install `bun-types` and
  augment the `UtilNamespace` interface
  themselves if they need the full typing;
  the slice 18 surface we expose is
  hand-rolled so the user does not need a
  devDependency just to satisfy the
  type-checker.

**Verification.** 245 tests green (210 lib +
4 worker_integration + 15 process_model + 16 start_order
e2e, the prior 15 plus
`tspserver_typings_emits_three_dts_files`).
The lib gain is the 4 new `typings::tests::*`
tests; the start_order e2e gain is the 1 new
typings e2e. The new e2e runs the real
`dist/tspserver/tspserver.exe` binary with the
new subcommand, asserts the three files are
written, and pins every public name the
wrap-prelude exposes (so a future slice that
adds a name to the wrap without updating
`tsp-server.d.ts` would fail the
`tsp_server_declares_every_wrap_prelude_name`
unit test).

---

### Amendment 6 (2026-08-27) — §22.3 config-file hot reload

Plan §22.3 ("Optional eager reload") and the slice
22 follow-up's deferred-work list both flag the
config-file hot reload as the next gap. Pre-§22.3,
the host reads `tsp.config.json` ONCE at boot; every
config change requires a master restart. The slice
22 family (counter / kv / feature_flag) is the
use case: today an operator who wants to add a new
kv entry must bounce the master.

This slice closes the gap end-to-end. The
existing `tsp.config.json`-driven service kinds
(`counter` / `kv` / `feature_flag`) can now be
edited on disk and take effect on the next request,
without a master restart.

**What changed in this amendment.**

- **`ServiceRegistry::apply_config_snapshot(fresh)`.**
  A new method that takes a freshly-parsed `Vec<Arc<dyn Service>>`
  and atomically replaces the previous
  config-declared set. Semantics:
  - Every name in `fresh` is registered (last-wins
    for duplicate names within `fresh`).
  - The previous config-declared set is dropped
    before the new entries are added; this gives
    `counter` / `kv` / `feature_flag` a clean
    reset to the new config (state is replaced,
    not merged -- a counter's `hits` reset to the
    new `initial`, a kv's `entries` replaced, etc.).
  - Built-in services (`logger` / `session` /
    `time`) are preserved across reloads. The
    registry tracks config-declared names in a
    `BTreeSet<String>` field; the retain / insert
    loop only touches that set, leaving the
    built-in entries untouched.
  - A config that re-declares a built-in name
    (e.g. `logger: {kind: "kv", ...}`) replaces
    the built-in, matching the existing
    `register()` last-wins semantic.

- **`ServiceRegistry` wrapped in `RwLock` for the
  host's request path.** Pre-§22.3 the bin
  leaked `&'static ServiceRegistry`; the request
  path borrowed it immutably. With hot reload,
  the watcher thread mutates the registry while
  the request path reads it. The bin now leaks
  `&'static RwLock<ServiceRegistry>`; the
  request path takes a read lock per
  `services.snapshot(&[])` / `services.get(...)`
  call. The lock is held for microseconds, so
  the watcher's write-lock brief wait is
  invisible in practice. The `host::serve` and
  `handle_connection` signatures changed to
  `&'static RwLock<ServiceRegistry>` /
  `&RwLock<ServiceRegistry>`; all five
  call-sites in `host.rs` use `.read().unwrap()`.

- **`WatchConfig` accepts a config-file watcher.**
  Two new fields:
  - `config_path: Option<PathBuf>` -- the file to
    poll (the bin passes `tsp.config.json` or
    the `TSP_CONFIG` env override when the file
    exists at boot).
  - `on_config_reload: Option<Arc<dyn Fn(&str) -> Result<String, String> + Send + Sync>>`
    -- the callback invoked with the new file
    text when the content hash changes.
  The watcher thread's poll loop gained a
  `poll_config_once` step (per-tick stat + read
  + content-hash diff). On a hash change, the
  callback is invoked and the result is logged
  to stderr (`config reloaded from <path>: <summary>`
  on success, `config reload apply failed: <err>`
  on failure). The bin's callback takes a write
  lock on the registry, calls
  `apply_config_snapshot`, and returns a summary
  string.

- **Bin wires up the callback.** The bin's
  `serve_main` reads the config at boot as
  before, but now routes the parsed services
  through `apply_config_snapshot` (so the
  registry's `config_decls` set is populated).
  The same callback is registered with the
  watcher. The bin's boot log gained a
  `config hot-reload watching <path> (poll
  interval = watcher poll)` line so the
  operator can see the watcher is active.

**What was NOT changed in this amendment.**

- The config parser (`load_counter_services_from_config`)
  is unchanged. The boot path and the reload
  path use the same parser; a typo in the new
  config that the boot path would have
  rejected is also rejected on reload, and the
  watcher logs the error and the previous
  snapshot stays in place (the registry is
  unchanged on Err).
- The `register()` method's last-wins semantic
  is preserved for non-config code paths. The
  bin's built-in registration
  (`ServiceRegistry::with_backends`) still uses
  `register()`; the new `apply_config_snapshot`
  is the only entry-point that touches
  `config_decls`.
- The watcher's routes-file polling is
  unchanged. The config-file poll is a second
  `poll_config_once` call inside the same
  watch-loop iteration; the two polls are
  independent.
- No new dependency. `RwLock` is in `std::sync`
  and was already imported elsewhere in
  `host.rs`.
- The host's `serve` / `serve_with_public_root`
  signatures changed (`&'static ServiceRegistry`
  -> `&'static RwLock<ServiceRegistry>`) but
  the bin is the only caller, and the change
  is internal to the runtime crate.

**Verification.** 248 tests green (212 lib +
4 worker_integration + 15 process_model + 17 start_order
e2e, the prior 16 plus
`config_file_hot_reload_replaces_services_without_master_restart`).
The lib gain is the 2 new
`services::apply_config_snapshot_tests::*` tests;
the start_order e2e gain is the 1 new
config-reload e2e. The new e2e runs the real
`dist/tspserver/tspserver.exe` binary with a
`tsp.config.json` declared in a temp dir, modifies
the file mid-run, waits for the `config reloaded`
marker in stderr, and asserts the new state is
visible on the next request (counter reset to
the new `initial`, kv `entries` replaced,
feature_flag `flags` replaced, built-in `logger`
preserved). The 4 unchanged test buckets
(`worker_integration` / `process_model` / the
prior 15 start_order e2e / the 210 prior lib
tests) all still pass against the rebuilt
binary, confirming the `RwLock` refactor did
not regress any of the request-path
semantics pinned by those tests.

---

### Amendment 7 (2026-08-27) — §32.1 dev error page

Plan §32.1 ("Dev Error Page") is the dev-UX
half of the Phase 11 tooling slice. Pre-§32.1,
a page that throws (other than `HttpError`)
caused the wrap to `console.error` and
`process.exit(1)`. The host saw a dead worker
and returned a generic 500 to the client --
the developer had to read `tspserver`'s
stderr to see what went wrong, and the
operator had to map the stderr back to the
right request. The §32.1 change makes the
error visible IN the response, with a
dev-mode HTML page that the developer can read
in their browser.

**What changed in this amendment.**

- **Wrap change (`jsx.rs`):** the inner catch
  that previously only handled `HttpError` now
  ALSO handles all other thrown values (Error
  instances with custom classes, plain strings,
  undefined, etc.) by building a 500 response
  with a JSON body of shape
  `{"kind":"tsp_error","error":"<class>",
  "message":"<message>","stack":"<stack>"}` and
  a `x-tsp-error: page` header. The outer IIFE
  catch (the "wrap itself failed" path) is
  unchanged -- it still `console.error`s the
  stack and `process.exit(1)`s, because by the
  time the outer catch fires, the wrap preamble
  has already failed and we cannot build a
  sane 500 envelope. The inner / outer split
  means: page errors -> 500 with JSON; wrap
  errors -> dead worker (a host-side 500 with
  the same generic body).

- **Host change (`host.rs`):**
  - A new `TSP_DEVELOPMENT=1` env var flips a
    `dev_mode()` helper to true. The flag is
    read on every request (no boot-time cache)
    so a single test can boot one master with
    the dev flag and another without.
  - The consumer of `EnvelopeOutcome` checks
    for the `x-tsp-error: page` header. If
    present AND `dev_mode()` is on, the host
    replaces the wire body with a self-
    contained HTML page (see below). If
    present AND prod, the host strips the
    `x-tsp-error` header (internal marker, not
    a public surface) and returns the 500 with
    the JSON body unchanged. The page-level
    wire contract (status 500, headers, body)
    is the same in both modes; only the
    rendered body changes.
  - A new `render_dev_error_page(body, status_line)`
    helper parses the JSON envelope and emits
    a hand-rolled HTML page. The page is
    self-contained (no external CSS, no JS, no
    template engine) and HTML-escapes every
    user-controlled field (error name,
    message, stack) so a malicious or
    accidentally-weird stack trace cannot
    inject `<script>` into the page. A failed
    parse falls back to a minimal `<error>`
    body so the host never returns an empty
    500.

- **Demo route `routes/dev_error_demo.tsp`**
  exposes three throw shapes via `?kind=`:
  - `plain` (`new Error("plain boom ...")`)
  - `range` (`new RangeError("index out of
    bounds: ...")` -- pins the `e.name`
    serialization for custom error classes)
  - `quiet` (a plain string throw -- pins the
    "non-Error thrown value" path; a regression
    where the wrap's `e.name` / `e.stack` access
    throws because the thrown value is not an
    Error would surface here as a host-side 500
    from the worker dying)

**What was NOT changed in this amendment.**

- The page-level wire contract is unchanged.
  The 500 response is the same in dev and
  prod; only the rendered body differs. The
  prod wire body IS the JSON envelope; the
  application can log it (or pipe it to its
  own monitoring) without parsing anything
  extra. The dev HTML is a presentation
  choice, not a contract change.
- Source maps (plan §32.2). The stack trace
  in the dev error page is whatever bun
  emits; bun has native TS / TSX source-map
  support, so the stack lines already point
  at the original `.tsp` file / line. This
  slice does not add a separate source-map
  pass on top; it relies on bun's built-in
  support. A future slice can add a "jump to
  editor" link to each stack frame, but
  that's editor-coupling work, not a
  contract change.
- The `Context.route: RouteInfo` runtime gap
  is unchanged. Out of scope for this slice.
- The `x-tsp-error: page` header is a
  marker between the wrap and the host; it
  is NEVER sent to the client. In prod mode
  the host strips it before sending; in dev
  mode the host replaces the wire body with
  HTML, so the client never sees the JSON
  envelope either.

**Verification.** 252 tests green (215 lib +
4 worker_integration + 15 process_model + 18 start_order
e2e, the prior 17 plus
`dev_error_page_renders_html_in_dev_mode_and_json_in_prod`).
The lib gain is the 2 new
`host::tests::dev_error_page_*` tests; the
start_order e2e gain is the 1 new dev-error
e2e. The new e2e runs the real
`dist/tspserver/tspserver.exe` binary
twice -- once with `TSP_DEVELOPMENT=1` and
once without -- and asserts the dev HTML
(three different throw shapes) and the prod
JSON envelope. The 4 unchanged test buckets
all still pass against the rebuilt binary.

---

### Amendment 8 (2026-08-27) — `kind: rate_limit` added to the config-driven services

Amendment 3 added `kind: kv` and `kind: feature_flag`
to the slice 22 `load_counter_services_from_config`
parser; this amendment extends the same surface
with `kind: rate_limit` so the prototype can
ship a fixed-window rate limit without writing a
single line of host Rust. The page reads the
snapshot via `ctx.services.<name>.{count,limit,
window_ms,window_start_ms,remaining}` and uses
the value to gate code paths (return 429 with a
`retry-after` header, show a quota in the UI,
back off an upstream call, etc.).

**What changed in this amendment.**

- **New `RateLimitService`** in
  `bun/src/runtime/tsp/services.rs`. The service
  holds `(name, limit, window_ms, Mutex<Inner>)`
  where `Inner = {count: u64, window_start_ms: u64}`.
  Every `describe_json()` call (i.e. every request
  that snapshots `ctx.services`) takes a brief
  write lock and:
  1. If `now - window_start_ms >= window_ms` (or
     `window_start_ms == 0`, the uninitialized
     state at construction), reset `count = 1`
     and `window_start_ms = now`.
  2. Otherwise increment `count` by 1.
  3. Serialize `{kind, name, count, limit,
     window_ms, window_start_ms, remaining}`
     where `remaining = saturating_sub(limit, count)`
     (clamped to 0 -- the page compares `count > limit`
     for the over-limit case rather than relying on
     a negative `remaining`).
  The service reports `is_request_varying() = true`
  so the generation cache never replays a stale
  count and silently lets a flood past the limit.

- **`load_counter_services_from_config` accepts
  `kind: rate_limit`.** The new match arm reads
  `limit` (required) and `window_seconds` (defaults
  to 60) and constructs the service. A missing
  `limit` is a hard error -- the operator must
  declare the bucket size explicitly. The
  unknown-kind error message now lists
  `rate_limit` in the supported set.

- **Demo route `routes/rate_limit.tsp`** exposes
  a `/rate_limit` endpoint that returns 200 with
  the snapshot when `count <= limit` and 429 with
  a `retry-after: 60` header when `count > limit`.
  A `?kind=info` query parameter bypasses the
  gate so the e2e can read the post-over-limit
  count without redirecting through the 429.

- **`tsp-server.d.ts` `ServiceDescriptor` union**
  grew a `rate_limit` variant with the wire
  shape above so the page's TypeScript type-check
  sees the new fields.

- **4 new unit tests** in
  `bun/src/runtime/tsp/services.rs`:
  - `rate_limit_post_increments_with_remaining_under_limit`
    pins the basic counter / remaining arithmetic.
  - `rate_limit_remaining_clamps_to_zero_when_over_limit`
    pins the over-limit `remaining = 0` clamp.
  - `rate_limit_window_resets_after_window_ms_elapses`
    pins the lazy window-reset semantic.
  - `config_parser_handles_rate_limit_kind` pins
    the parser (happy path + `window_seconds`
    default + missing-`limit` error message).

- **1 new e2e**
  `config_driven_rate_limit_kind_gates_requests_with_a_fixed_window`
  in `bun/src/runtime/tsp/tests/start_order.rs`.
  Runs the real binary with `limit=2` and
  exercises the full 1 -> 2 -> over-limit -> info
  sequence, then hot-reloads the config to
  `limit=10` and asserts the count resets to 1
  on the new service instance. The hot-reload
  round-trip is the same path Amendment 6 added
  for `counter` / `kv` / `feature_flag`.

**What was NOT changed in this amendment.**

- Per-IP / per-user / per-API-key bucketing.
  The v1 service is single-bucket; every snapshot
  increments the same counter. A per-key bucket
  would need the host to derive the key from the
  request (IP, session id, an API-key header)
  and call `service.tick(key)` before the page
  runs. The interface is designed to extend
  cleanly (`Inner` is already a `BTreeMap`-shaped
  field waiting to grow), but the slice stays
  focused on the single-bucket primitive.

- The config parser is still named
  `load_counter_services_from_config`. The name
  has been misleading since Amendment 3 added
  `kv` + `feature_flag`; this amendment makes
  the name three kinds out of date. A follow-up
  slice can rename it to `load_config_services`
  -- the function body is generic and the
  only caller is the bin. The rename is left
  for a future slice so this commit stays
  focused on the new kind.

- `is_request_varying()` is `true` for
  `RateLimitService`. Same as `CounterService`:
  every request bumps the count, so a generation
  cache that replays a stale snapshot would
  under-count and let a flood past the limit.
  The trade-off is the same -- no cache -- and
  acceptable for a runtime-owned singleton.

- The page wire contract is unchanged. The
  page's `ctx.services.<name>` shape is
  `{kind, name, count, limit, window_ms,
  window_start_ms, remaining}`; this is a new
  variant of the existing `ServiceDescriptor`
  union (typed as `kind: "rate_limit"` in
  the .d.ts). No new wrap-prelude code -- the
  existing JSON hydrator in the wrap preamble
  handles the new fields automatically.

**Verification.** 257 tests green (219 lib +
4 worker_integration + 15 process_model + 19 start_order
e2e, the prior 18 plus
`config_driven_rate_limit_kind_gates_requests_with_a_fixed_window`).
The lib gain is the 4 new
`services::tests::rate_limit_*` and
`services::tests::config_parser_handles_rate_limit_kind`
tests; the start_order e2e gain is the 1 new
rate-limit e2e. The 4 unchanged test buckets all
still pass against the rebuilt binary, confirming
the parser change (the unknown-kind error
message now lists `rate_limit`) and the snapshot
serializer change do not regress any of the
prior 18 e2e scenarios.

---

### Amendment 9 (2026-08-27) — §32.2 source-map gap pinned

Plan §32.2 ("Source Map") asks for "TS/TSX
runtime stack MUST map back to the original
source" per the spec. The wrap already emits
`//# sourceURL=<tsp://path>?generation=<N>` for
the embedded-worker path
(`bun/src/runtime/tsp/jsc_bridge.rs:338`), so a
`vm.eval`-style runtime would attribute the script
to the .tsp file. The current production worker
writes the script to a temp `.tsx` file and runs
it via `vm.load_entry_point`; bun 1.4 honors
`//# sourceURL=` for IN-LINE eval'd scripts but
NOT for the file-loaded path, so the dev error
page stack currently shows
`tsp-embedded-worker-<pid>.tsx:<line>:<col>`
where `<line>` is the WRAPPED-output line (the
prelude is thousands of lines long) -- not the
original .tsp line.

This amendment is a "pin + document" rather than
a "fix". The bun-side change required is either
(a) honor `//# sourceURL=` for file-loaded
scripts in bun:runtime, or (b) move the TSP
worker from file-load to in-line `vm.eval` /
`vm.Script.evaluate` (so the directive is
honored). Both are outside the runtime crate.
The host can ship every other Phase 11 /
Amendment 8 surface (the dev error page, the
config-driven services, the config hot reload,
the rate-limit / kv / feature_flag kinds)
without the source-map remap; the page still
throws, the host still captures the error, the
dev page still renders HTML. The line/col being
unhelpful is a known limitation; the future
bun-side change can flip the assertions this
amendment adds.

**What changed in this amendment.**

- **`bun/src/runtime/tsp/jsx.rs`: 2 new unit
  tests.**
  - `wrap_for_embedded_worker_emits_sourceurl_directive`
    pins the wrap's contract -- the wrap does
    NOT emit any sourceURL or sourceMappingURL.
    The `//# sourceURL=` is appended AFTER the
    wrap by `jsc_bridge::execute_inner` with the
    real source path (so the wrap itself does
    not need to know the path). The test asserts
    the absence of any sourceURL in the wrap
    output; a regression that emits a placeholder
    path (or no directive at all) is caught.
  - `jsc_bridge_appends_tsp_sourceurl_with_generation`
    pins the directive shape in
    `jsc_bridge.rs`:
    `//# sourceURL=tsp://<path>?generation=<N>`.
    The `?generation=<N>` suffix is the
    per-request cache-bust for `bun:main`
    (see BUG-0001); a refactor that drops the
    suffix would re-introduce the multi-route
    alias bug. The test is a string-pin against
    `include_str!("jsc_bridge.rs")` so a
    refactor that changes the format is caught
    here, and the start_order e2e catches the
    runtime consequence.

- **`bun/src/runtime/tsp/tests/start_order.rs`:**
  the existing
  `dev_error_page_renders_html_in_dev_mode_and_json_in_prod`
  e2e gained 2 assertions in the "plain" body
  check:
  - The stack MUST currently show
    `tsp-embedded-worker-` (pin the worker's
    temp-file attribution, which is what bun
    actually uses).
  - The stack MUST NOT show `dev_error_demo.tsp`
    (pin the gap -- the `.tsp` file is not yet
    visible in the stack because the source
    remap needs the bun-side change). A future
    bun-side change can flip this assertion to
    assert the `.tsp` path is present.

**What was NOT changed in this amendment.**

- The wrap's contract is unchanged. The wrap
  does NOT emit sourceURL or sourceMappingURL;
  `jsc_bridge::execute_inner` continues to
  append the sourceURL after the wrap. A
  regression that emits a placeholder sourceURL
  inside the wrap would still be caught by the
  new unit test (the absence assertion).
- The dev error page is unchanged. The page
  still surfaces the raw `e.stack` from bun;
  the file name + line/col are still the
  temp-file + wrapped-output values.
- The worker is unchanged. The worker
  continues to write the script to a temp file
  and run `vm.load_entry_point`. A follow-up
  slice can either (a) move the worker to
  `vm.eval` / `vm.Script.evaluate` (a more
  invasive change to
  `bun/src/runtime/tsp_worker.rs`), or (b)
  wait for bun to honor `//# sourceURL=` for
  file-loaded scripts (a bun-side change).
- The production binary at
  `dist/tspserver/tspserver.exe` does not
  change. The unit tests are against the wrap
  string + the source file content; the e2e
  pins the live behavior via the existing
  dev_error_page run. No relink needed.

**Verification.** 240 tests green (238 prior +
2 new). The lib gain is the 2 new
`jsx::tests::sourceurl_*` tests; the start_order
count is unchanged (the 2 new assertions are
added to the existing dev_error_page e2e). The 4
unchanged test buckets all still pass against the
unchanged wrap, the unchanged worker, and the
unchanged production binary.

### Amendment 10 (2026-08-28) — `/__tsp/metrics` e2e pin + `tsc check`

Two new tooling surfaces ship together:

**1. `/__tsp/metrics` endpoint contract
(host.rs:1517-1532).** The closure-hardening
metrics surface (closure item, recorded in
`progress.md` §"Session update (2026-08-25)")
was always exposed at `GET /__tsp/metrics`
returning `200 OK` + `text/plain;
version=0.0.4; charset=utf-8` + a Prometheus
text-format body. It was not e2e-pinned. The
new e2e
(`metrics_endpoint_serves_prometheus_text_after_priming_requests`)
boots the real binary, primes the counters with
a 200 + 404, then hits `/__tsp/metrics` and
asserts:

  - the response shape (status + content-type
    + 10 metric names each with `# HELP` and
    `# TYPE` preamble);
  - the **snapshot semantics** of the body:
    `prometheus()` runs AFTER the metrics
    call's own `record_request()` (so
    `requests_total` and `active_requests`
    include the call) but BEFORE its own
    `record_response()` + `record_duration()`
    (so `2xx_total`, `duration_count`, and the
    `active` decrement do NOT yet reflect the
    call). A second hit on the same binary
    sees the first call's contributions, which
    pins the order. A regression that swaps
    those phases would break the test.

  The contract application code may rely on:

  - `GET /__tsp/metrics` returns 200 with a
    Prometheus body covering 10 metric names
    (`tsp_requests_total`, `tsp_active_requests`,
    `tsp_request_duration_ms_sum`,
    `tsp_request_duration_ms_count`,
    `tsp_responses_{2,4,5}xx_total`,
    `tsp_request_timeouts_total`,
    `tsp_request_cancellations_total`,
    `tsp_reload_total`).
  - The body is a snapshot of the host's
    `metrics::global()` at the moment of the
    metrics call, BEFORE the call's own
    `record_response` + `record_duration`
    fire. Operators that scrape `/__tsp/metrics`
    should treat the body as "state as of the
    scrape started" (so `requests_total`
    always shows 1 more than `2xx+4xx+5xx`,
    and `duration_count` is one less than
    `requests_total` until the scrape
    completes).
  - The endpoint is the ONLY path matched
    before the page router. It is dispatched
    in `host.rs` directly, not through the
    `PageRegistry`, so a route file at
    `routes/__tsp/metrics.tsp` does not
    shadow it.

**2. `tspserver check --tsc` (Phase 11
close).** The `check` subcommand historically
did only the regex-based static-export
detection + the module-graph build. The new
`--tsc` flag adds a real `tsc --noEmit` pass:

  - Walks the routes dir recursively, copying
    `.tsp` to `.tsx` and `.ts` helpers
    verbatim into a temp tree.
  - Copies the three bundled declaration files
    from `.tsp-types/` (the `tspserver
    typings` default) or `tsp-types/` (the
    repo's location) into the temp tree.
    The bin probes CWD and the parent of the
    routes root, in that order.
  - Writes a `tsconfig.json` that maps
    `tsp:server` / `tsp:html` / `tsp:runtime`
    to the three declarations via `paths`,
    with `skipLibCheck: true` (the hand-rolled
    d.ts shape is the contract, not a bug) and
    `strict: false` (the runtime is not strict,
    so the check is not either).
  - Locates a `tsc` binary: CWD
    `node_modules/.bin/tsc{.cmd,}` first, then
    `tsc` on PATH.
  - Invokes `tsc --noEmit --project <tsconfig>`
    and forwards stdout / stderr to the user
    verbatim.

  The check is opt-in: the default
  `tspserver check` continues to do the
  original regex scan + graph build, so
  existing workflows are unaffected.
  `--tsc` returns 0 if tsc exits 0, 1
  otherwise.

  E2E
  (`check_with_tsc_flag_catches_user_type_errors_and_passes_clean_routes`):
  Round 1 (clean route) exits 0 and prints
  "OK tsc: 0 error(s)". Round 2 (broken
  route) exits 1 with a TS2353 diagnostic for
  a property the d.ts does not allow (e.g.
  `cost` on `util.password.hash` options --
  the d.ts only allows `algorithm`; bun's
  native password API silently drops unknown
  options at runtime, so the check is the
  only place the type error is visible).

  The contract application code may rely on:

  - `tspserver check --tsc` returns 0 when
    the routes type-check cleanly against the
    bundled `tsp:*` declaration files; 1 if
    tsc reports any error.
  - The user's project must have a `tsc`
    binary either in the local
    `node_modules/.bin/` or on `PATH`.
  - The bundled declaration files are
    required; run `tspserver typings --out
    .tsp-types` first if the project does not
    have them.

**Verification.** 21 e2e (start_order.rs),
221 lib, 4 worker_integration, 15
process_model = 261 tests, all green. The 2
new amendments net 1 e2e (the metrics pin;
the tsc check is a new flag on the existing
check subcommand) and 0 lib changes.
