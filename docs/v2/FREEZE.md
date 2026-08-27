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
  Password hashing (Amendment 1)
    password                         (Bun.password native — bcrypt /
                                      argon2id / scrypt)
  Database factory (Amendment 1)
    sql                              (Bun.SQL factory; per-worker pool
                                      via bun:sql)
  Bun builtin helper namespace (Amendment 1)
    util                             (17 surfaces: randomUUIDv7, hash,
                                      CryptoHasher, Glob, TOML, YAML,
                                      markdown, escapeHTML, gzipSync,
                                      gunzipSync, file, write, which,
                                      peek, deepEquals, deepMatch,
                                      nanoseconds, env)

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

Evidence: `tsp-v2-specification.md` §16, `tsp-v2-plan.md` §16, §17.3.

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
  migrated to native `Bun.password` in `c0b802c340`)
- `sql` (slice 17d, commit `336d3d522f`; `Bun.SQL` factory)
- `util` (slice 18, commit `756108d694`; namespace of 17 bun
  builtins, see the item 8 surface listing)

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

**Verification.** 228 tests green as of this amendment
(202 lib + 4 worker_integration + 15 process_model + 7 start_order
e2e, including `util_namespace_surfaces_bun_builtins_for_pages` for
the new `util` namespace, `zod_runtime_compiled_into_wrap_serves_validated_schemas`
for `zod`, `password_runtime_through_bun_password_serves_hashed_passwords`
for `password`, `sql_runtime_uses_bun_native_pool_for_page_local_datasource`
for `sql`, and `nanoid_runtime_compiled_into_wrap_serves_distinct_ids`
for the nanoid family).
