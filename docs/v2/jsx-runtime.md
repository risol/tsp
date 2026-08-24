# TSP v2 JSX Runtime Contract

> Phase 0 topic doc. Source of truth: `tsp-v2-specification.md` §11-§12
> and `tsp-v2-plan.md` §11-§12, §60 freeze items 9, 10.

The TSP JSX runtime is the framework's server-side element tree. It is
**not React**. The element tree is the server-render model; the host
walks it to produce HTML bytes that ship in the response body.

## What the runtime is NOT

- It is not React. There is no reconciliation, no `useState`, no
  `useEffect`, no `useLayoutEffect`, no hooks at all (plan §12.3).
- It is not a client runtime. There is no client hydration, no
  virtual DOM diff, no event handler attachment on the server.
- It is not a JSX-as-React-Element surface. JSX compiles to calls
  to the TSP JSX runtime, not to `React.createElement`.

The element tree is a server-render tree. The host walks it once,
synchronously (or with `await` for async components, see below) to
produce the byte stream.

## JSX compilation

TypeScript / TSX configuration:

```json
{
  "compilerOptions": {
    "jsx": "react-jsx",
    "jsxImportSource": "tsp"
  }
}
```

JSX is compiled to:

```ts
import { jsx, jsxs, Fragment } from "tsp/jsx-runtime";

const tree = jsx("h1", { children: "Hello" });
const tree2 = jsxs("ul", { children: [a, b, c] });
const tree3 = jsx(Fragment, { children: [a, b] });
```

`tsp/jsx-runtime` is a `tsp:*` builtin module (see `context.md` /
freeze 8). The `jsx` / `jsxs` / `Fragment` functions are
host-implemented; they return an `HtmlNode` opaque to the
application.

## The child-rendering rules (freeze 9)

The renderer walks a node's `children` slot. Each child is processed
by this table:

| child value            | output                                                |
|------------------------|-------------------------------------------------------|
| `null` / `undefined`   | empty                                                 |
| `true` / `false`       | empty                                                 |
| number                 | its string form (e.g. `42`)                           |
| string                 | HTML-escaped (`<script>` -> `&lt;script&gt;`)         |
| array                  | recursively flattened, each item per this table       |
| `HtmlNode`             | rendered as its element                                |
| anything else (object) | `TSP3102: object cannot be rendered as an HTML child` |

These rules are the XSS-by-default contract. Any string child is
escaped; the only way to ship unescaped HTML is the explicit
`raw(trustedHtml)` from `tsp:html`.

## Attribute rules (freeze 9)

| attribute value      | output                                                |
|----------------------|-------------------------------------------------------|
| string               | HTML-escaped                                          |
| number               | stringified                                           |
| `true`               | the bare attribute name (e.g. `<input disabled>`)     |
| `false` / `null` / `undefined` | attribute dropped                           |
| function             | `TSP3105: function-valued HTML attributes are not serializable` (runtime error) |
| `HtmlNode`           | not allowed as an attribute value                     |

Function-valued attributes are a hard error on the server. There is
no `onClick={fn}` model; client interactivity arrives through
fragment URLs + form posts.

## Raw HTML (freeze 9)

Raw HTML is opt-in via the `raw(...)` helper from `tsp:html`:

```tsx
import { raw } from "tsp:html";

const trusted: TrustedHtml = raw("<b>pre-escaped content</b>");
return <div>{trusted}</div>;
```

`raw(...)` returns a `TrustedHtml` brand type. The renderer audits
the brand before splicing the bytes into the output; unbranded
strings cannot reach the `raw(...)` path. The name `raw` is the
warning.

## Components (function + async, freeze 10)

A function component is a `function` or `const` that takes a `props`
object and returns an `HtmlNode`:

```tsx
function Greeting({ name }: { name: string }) {
  return <strong>Hello {name}</strong>;
}
```

An async function component is first-class (freeze 10):

```tsx
async function UserName({ id }: { id: number }) {
  const user = await db.users.get(id);
  return <span>{user.name}</span>;
}
```

The renderer accepts `Promise<HtmlNode>` in any child position and
awaits it before continuing. Nested promises flatten.

Components MUST be pure with respect to their props. Side effects
(DB writes, network requests beyond `await`, etc.) are allowed
inside async components but the component MUST still return an
`HtmlNode` (not `void`).

## Fragment

`<>...</>` and `<Fragment>...</Fragment>` are equivalent. The
runtime flattens fragments at render time; they do not appear in
the output HTML.

## Class names

For v2.0, both `class` and `className` are accepted; the canonical
output uses the HTML-native `class`. The same applies to `for` /
`htmlFor`. The runtime does not invent a different naming
convention. See `tsp-v2-plan.md` §11.5.

## What is NOT in v2.0

These are explicitly deferred:

- Streaming / partial responses. v2.0 renders the full body
  before sending (plan §13).
- Error boundaries per component. v2.0 lets the handler-level
  error path take over; component-level `<Boundary>` arrives in
  v2.1 (plan §12.4).
- React compatibility. The optional `@tsp/react` package (plan
  §66) lets a single handler return a React render result; it is
  not v2.0 core.
- Server Components / RSC. Not in scope.
- Web Components / custom elements. Plain HTML elements only in
  v2.0; custom elements arrive when the JSX type-check widens
  (slice 9+).
