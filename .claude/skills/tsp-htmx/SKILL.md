---
name: tsp-htmx
description: Technical guidance for wiring htmx into a TSP page using fragment routing. Use when Claude needs to add `hx-get` / `hx-post` / `hx-trigger` / `hx-swap` attributes, write a `fragments` named export, render `<HtmxScript />` or `<HtmxFragment>` in a `.tsp` file, or expose a server-rendered sub-region that htmx refreshes in place. Do not use for non-htmx AJAX, WebSocket / SSE flows, or general frontend framework decisions.
---

# TSP htmx Integration

This skill covers the htmx ↔ TSP integration that ships in the project:

- A `.tsp` file can declare a `fragments` named export. Each entry is a
  `Page()`-wrapped function returning a VNode.
- Any URL of the form `<page>/__fragment/<name>` invokes that named fragment and
  returns its VNode as bare HTML (no `<!DOCTYPE>`).
- Three globals — `hxUrl`, `HtmxScript`, `HtmxFragment` — provide a type-safe
  surface for the common htmx patterns.
- A JSX pre-walk in `main.ts` auto-resolves `<HtmxFragment>` initial content
  from the same file's `fragments[name]`. No need to call the fragment function
  twice.

## Use this skill when

- A `.tsp` page should swap a region in place on a trigger (polling, click,
  hover, form submit, etc.).
- The user asks for a live dashboard tile, a refresh-on-interval widget, an
  "edit inline" form, or any "load HTML over the wire" pattern.
- The user asks how to declare multiple sub-renders inside a single `.tsp` file.
- The user asks why their `hx-get="/x.tsp/__fragment/..."` is not matching, or
  how to render an `hx-*` div with the right initial content.

## Do not use this skill when

- The task is plain form submission without htmx; the `tsp-coding` skill covers
  that.
- The user wants full-page reloads or HTML streaming.
- The task involves WebSocket / SSE (htmx extensions exist but are out of scope
  here — flag and ask the user).

## Core workflow

1. Decide the page that owns the fragments. Fragments are per-page, identified
   by `name`, and accessible only via `<that-page>/__fragment/<name>`.
2. In the owning `.tsp` file, declare
   `export const fragments:
   FragmentMap = { ... }` with one `Fragment(...)`
   per sub-region.
3. In the same file's default page, render `<HtmxScript />` in `<head>` and
   `<HtmxFragment name="..." page="..." />` where the swap region belongs.
   Initial content is auto-fetched.
4. If the page needs to know about a fragment on a _different_ page, use
   `hxUrl(otherPage, "name")` for the URL and
   `<HtmxFragment page={otherPage} name="name" />` for the wrapper.
5. If the default page is the only consumer, you may call `fragments.x(ctx)`
   directly to render initial content; the pre-walk will avoid double work.

## htmx globals

### `hxUrl(page, name)`

Builds the fragment URL. Accepts `/users`, `/users.tsp`, or `users` for the page
argument and always normalizes to `/users.tsp/__fragment/<name>`. Throws on an
empty name so typos surface at build time.

```tsx
hxUrl("/users", "table");
// -> "/users.tsp/__fragment/table"
```

Use this anywhere you would otherwise hand-write the URL.

### `<HtmxScript />`

Renders the vendored htmx client (served at `/__static/htmx.js` by the server)
and, when any config prop is provided, a `<meta name="htmx-config">` tag right
after it.

```tsx
<head>
  <HtmxScript
    defaultSwap="outerHTML"
    timeout={5000}
    historyCacheSize={20}
  />
</head>;
```

Recognised props are the common htmx options:

- `defaultSwap`, `defaultSwapDelay`, `defaultSettleDelay`
- `timeout` (ms)
- `historyCacheSize`
- `withCredentials`
- `indicatorClass`
- `inlineScriptNonce`

The server already serves htmx at `/__static/htmx.js`. Drop `<HtmxScript />` in
`<head>` and you do not need a CDN or a vendored copy in `www/`.

### `<HtmxFragment page name trigger swap target include confirm />`

Renders the hx-* wrapper div. **Initial content is auto-fetched** from the same
page's `fragments[name]` during SSR — no need to pass `children`.

```tsx
<HtmxFragment
  page="/users"
  name="table"
  trigger="every 5s"
  swap="outerHTML"
/>;
// renders
// <div hx-get="/users.tsp/__fragment/table"
//      hx-trigger="every 5s"
//      hx-swap="outerHTML">
//   <!-- fragments.table(ctx) was called server-side and the
//        result is injected here. -->
// </div>
```

If the user passes `children` explicitly, the framework does not look up the
fragment. That is the escape hatch for cases where the initial content should
differ from the swapped content.

The `trigger` value is the raw htmx trigger spec (`every 5s`, `click from:#btn`,
`load delay:200ms`, `intersect`, etc.). The default `swap` is `outerHTML`. Pass
`target` to swap into a sibling, `include` to send additional inputs with the
request, and `confirm` to require user confirmation.

## Fragment routing on the server

- Fragments live as `export const fragments: FragmentMap = { ... }` on the same
  `.tsp` file as the page that owns them.
- The URL convention is `<page>/__fragment/<name>`. The router strips the
  marker, resolves the page, imports the module, and invokes the named export
  with the request context.
- The response is `text/html` and contains **only** the VNode produced by the
  fragment — no `<!DOCTYPE>`, no `<html>`. That is what makes htmx
  `hx-swap="outerHTML"` work as a drop-in.
- A fragment that returns a `Response` (e.g. JSON via `response.json(...)`) is
  passed through; this is useful for endpoints that need to act as both htmx
  fragments and JSON APIs.
- A 404 is returned for unknown fragment names.

## Quality bar

Before finishing, check that:

- The `fragments` map is declared on the same file that owns the page (or
  imported and re-exported by it).
- Fragment names are valid identifiers (`[a-zA-Z_][a-zA-Z0-9_-]*`). Empty or
  non-matching names cause a 404.
- `HtmxScript` is rendered in `<head>`, not in the body.
- `HtmxFragment` is used only inside a page that exports a matching
  `fragments[name]`. A typo logs a single `console.warn` and renders nothing for
  that region.
- The fragment function is not called twice (once for initial content, once for
  the swap) — the swap goes through the `/__fragment/<name>` URL only.
- Forms that need to mutate state should `POST` to a separate fragment rather
  than mutating inside the GET, so the `hx-trigger="every Ns"` polling does not
  race the user's edit.

## Examples

See `assets/htmx-page-patterns.md` for full skeletons:

- Polling tile (auto-fetched initial)
- Click-to-load region with query params
- Inline edit form that refreshes the parent region
- Mixing fragments with explicit children (no auto-fetch)
