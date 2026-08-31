# `.tsp` module format

A `.tsp` file is a standard TypeScript/TSX route entry module. It has no
frontmatter, template delimiters, decorators, or special page class.

## Minimal page

```tsx
import { type Context } from "tsp:server";

export function GET(ctx: Context) {
  return <h1>Hello from {ctx.url.pathname}</h1>;
}
```

The handler may be synchronous or async:

```tsx
export async function POST(ctx: Context) {
  const payload = await ctx.request.json<{ name: string }>();
  return Response.json({ ok: true, name: payload.name });
}
```

## Exports

The runtime recognizes these page handler names:

```ts
export function GET(ctx: Context) {}
export function POST(ctx: Context) {}
export function PUT(ctx: Context) {}
export function PATCH(ctx: Context) {}
export function DELETE(ctx: Context) {}
```

Use `config.methods` when the page wants an explicit method declaration:

```tsx
export const config = {
  methods: ["GET", "POST"],
};
```

The list must exactly match the exported handlers. `HEAD` and `OPTIONS` have
host behavior and are not normally exported. A default export is not a page
handler. `tspserver check` reports default exports and unknown exported
functions so typos do not become silent routes.

## Handler results

Return JSX, a top-level HTML string, a trusted node, or `Response`:

```tsx
import { json, redirect, text } from "tsp:server";

export function GET() {
  return json({ ok: true });
}

export function POST() {
  return redirect("/done", 303);
}

export function PUT() {
  return text("updated", { status: 202 });
}
```

Do not return arbitrary objects as a response. `json(value)` is the explicit
object-to-JSON boundary.

## Filesystem mapping

```text
pages/index.tsp             -> /
pages/login.tsp              -> /login
pages/users/index.tsp        -> /users
pages/users/[id].tsp         -> /users/:id
pages/posts/[slug].tsp       -> /posts/:slug
pages/files/[...path].tsp   -> /files/*
```

Dynamic segment names follow the identifier pattern `[A-Za-z_][A-Za-z0-9_]*`.
Values are available as strings in `ctx.params`. Static routes have priority
over dynamic routes, and dynamic routes have priority over catch-all routes.
Two routes with the same shape are ambiguous and fail route discovery.

## Imports

A `.tsp` module may import:

- `.ts`, `.tsx`, `.js`, and `.jsx` application modules;
- JSON data files supported by the Bun resolver;
- supported npm packages;
- `tsp:*` built-ins; and
- supported `node:*` built-ins.

It must not import another `.tsp` route, directly or through an alias. Local
imports that escape the configured application root are rejected. The watcher
tracks supported local application modules; external packages are not treated
as reloadable page files.

## Checking a module

```text
tspserver check
tspserver check --tsc --no-color
tspserver routes --json
tspserver graph --json
```

Generate editor types with `tspserver typings --out .tsp-types` and include
that directory in the TypeScript project.
