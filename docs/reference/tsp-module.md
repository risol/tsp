# TSP `.tsp` Module Format

> Phase 0 topic doc. Source of truth: `tsp-specification.md` §3-§6
> and `tsp-plan.md` §3-§6, §60 freeze items 1, 2, 3, 4.

A `.tsp` file is the **route entry module** -- it is a standard TSX
file with a small set of framework-level named exports. This document
covers the file format only; the JSX runtime contract lives in
`jsx-runtime.md`, the Context ABI in `context.md`.

## What `.tsp` is

`.tsp` files MUST parse with a standard TypeScript / JSX parser
(freeze 1). No `<template>`, no `<script>`, no `{% %}`, no `@page` /
`@fragment` decorators. Syntax is plain TSX. (See `tsp-plan.md`
§3.1 for the explicit list of forbidden syntaxes.)

What this means in practice:

- Any editor with TSX support highlights / lints / type-checks `.tsp`
  files correctly.
- The host's bundler / transpiler / IDE sees `.tsp` as a normal
  TSX module.
- A `.tsp` file is a single file; there is no SFC-style frontmatter.

## What `.tsp` is NOT

A `.tsp` file is NOT a reusable library module. It cannot appear in
any `import` statement (freeze 2). The host refuses to start with:

```text
TSP2003: .tsp modules are route entry modules and cannot be imported.
Move reusable code to .ts or .tsx.
```

If two routes need to share code, the shared code lives in
`components/*.tsx` (or `lib/*.ts`) and is imported normally from both
`.tsp` files.

The rationale is in `tsp-plan.md` §5.1: a `.tsp` is a generation
root, and the module graph cannot let one generation root depend on
another.

## Legal exports

A `.tsp` file's framework-level exports are:

```ts
// Page configuration (optional)
export const config = {
  auth: "required",
  cache: "no-store",
  bodyLimit: 2 * 1024 * 1024,
  timeoutMs: 30000,
} satisfies PageConfig;

// One handler per HTTP method (named by the verb)
export function GET(ctx) {}
export function POST(ctx) {}
export function PUT(ctx) {}
export function PATCH(ctx) {}
export function DELETE(ctx) {}
// HEAD and OPTIONS are NOT standard exports -- see below.

// Fragments (one per named export)
export const list = fragment(async (ctx) => { ... });
export const detail = fragment({ method: "GET", handler: ... });
```

The full list of HTTP method exports is the standard verb set
(freeze 4). `HEAD` is synthesised by the host from the `GET`
response when the page exports `GET` but no explicit `HEAD` -- the
host strips the body and keeps the status / headers. `OPTIONS` is
synthesised as a 204 with `Allow: <methods>` when the page omits
it. See `tsp-plan.md` §42.

`export default function Page() {}` is **not** interpreted as a
default page handler (plan §4.2.1). It is a regular named export
from the application's point of view; the host ignores it for
routing. The motivation is in `tsp-plan.md` §4.2.1: HTTP method
semantics are clearer when the verb is the export name.

## Handler type

```ts
type PageHandler = (
  ctx: Context
) => HandlerResult | Promise<HandlerResult>;
type HandlerResult = HtmlNode | Response;
```

The return is **explicit**: an `HtmlNode` (from the JSX runtime) or a
standard `Response`. Shape magic is rejected (freeze 5):

```text
TSP3001: handler returned unsupported value Object.
Expected HtmlNode or Response.
```

This means no `return { redirect: "/x" }`, no `return "string"`, no
`return 42`. Use the helpers in `tsp:server`:

```ts
return redirect("/users");
return json({ ok: true });
return text("ok", { status: 201 });
return html(<MyPage />);
return notFound();
```

## Route mapping (file system)

The route table is built from the `pages/` directory (configurable
via `tsp.toml [routes] dir`). The mapping is purely filename-driven
(freeze 3):

```text
pages/index.tsp             ->  GET  /
pages/login.tsp             ->  GET  /login
pages/users/index.tsp       ->  GET  /users
pages/users/new.tsp         ->  GET  /users/new
pages/users/[id].tsp        ->  GET  /users/:id
pages/posts/[slug].tsp      ->  GET  /posts/:slug
pages/files/[...path].tsp   ->  GET  /files/*    (catch-all)
```

Segment name pattern: `[A-Za-z_][A-Za-z0-9_]*`. Dynamic segment values
surface in `ctx.params.<name>`. Optional catch-all is not in current contract.

Priority at runtime: **static > dynamic > catch-all**. The host's
route table is built at startup; an ambiguous pairing (e.g.
`pages/users/[id].tsp` and `pages/users/[name].tsp` at the same
level) makes the host refuse to start with:

```text
TSP1004: ambiguous routes /users/[id] and /users/[name]
```

## Importing other modules

`.tsp` files can import:

- `.ts`, `.tsx`, `.js`, `.jsx`
- `.json` (for typed data files)
- npm packages (resolved via `node_modules/`)
- `tsp:*` builtin modules (`tsp:server`, `tsp:html`, `tsp:runtime`)
- `node:*` builtins if the runtime supports them

`.tsp` files **cannot** import other `.tsp` files (freeze 2). They
also cannot import the same `.tsp` file under any alias or path
indirection; the host normalises all canonical paths before
checking.

## The two classes of modules

The host treats every imported module as one of two kinds:

```text
Reloadable Application Module
  = canonical_path under application.root
    AND supported source extension
  -> pages/, components/, lib/, services/, ... go here

Persistent External Module
  = everything else
  -> node_modules/, tsp:*, node:* go here
```

The watcher only watches Reloadable modules. Persistent External
modules do not invalidate the page graph. See
`tsp-plan.md` §5.3 for the rationale (rebuild cost, dev-loop
stability).

## What is NOT in current contract

These are explicitly deferred to future releases and are NOT part of the
`.tsp` file contract in current contract:

- Nested layouts as a directory convention (plan §15). Use component
  composition: `<AppLayout>...</AppLayout>` in the handler body.
- Custom error pages (`pages/_404.tsp`, `pages/_500.tsp`). The
  host's built-in error pages are the current contract default.
- Middleware modules (`middleware.ts`). The current contract design is
  PageConfig + auth service hooks, not Express-style middleware
  chains (plan §44).
- Decorators (`@page`, `@fragment`, etc.). Named exports are the
  contract (freeze 4 + freeze 7); there is no metadata layer.
