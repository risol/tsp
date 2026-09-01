# Built-in client runtime

TSP includes a small browser runtime for updating part of a page from a
fragment response. It is embedded in `tspserver` and is available at
`/__tsp/runtime.js`; applications do not need to install HTMX.

The server automatically adds the runtime script to a successful HTML page
that contains a `data-tsp-*` or `hx-*` request attribute. Fragment responses
are never modified, so they can be inserted directly into the target element.

## Basic example

```tsx
import { type Context, fragment } from "tsp:server";

export const list = fragment(async () => (
  <ul>
    <li>Alice</li>
    <li>Bob</li>
  </ul>
));

export function GET(ctx: Context) {
  return (
    <main>
      <button
        data-tsp-get={ctx.fragment("list")}
        data-tsp-target="#user-list"
      >
        Refresh
      </button>
      <section id="user-list" />
    </main>
  );
}
```

The default swap mode is `innerHTML`. Use `data-tsp-swap` for the small set of
supported alternatives: `outerHTML`, `beforebegin`, `afterbegin`, `beforeend`,
`afterend`, `append`, `prepend`, and `none`.

`data-tsp-post`, `data-tsp-put`, `data-tsp-patch`, and `data-tsp-delete` are
also supported. When the request element is inside a form, non-GET requests
send the form as `FormData`; GET requests append the form fields to the URL.

For gradual migration, the runtime also understands the matching HTMX names:
`hx-get`, `hx-post`, `hx-target`, and `hx-swap`.

The runtime dispatches `tsp:before-request`, `tsp:after-request`, and
`tsp:request-error` events. A `tsp:before-request` listener can cancel a
request with `event.preventDefault()`.
