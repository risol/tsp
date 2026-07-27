# TSP htmx Page Patterns

Use these skeletons when Claude needs to add a swap region to a `.tsp` page.
Adjust injected deps to match the current project's `types.d.ts`.

All patterns assume the project exports `HtmxScript`, `HtmxFragment`, `hxUrl`,
`Fragment`, and `FragmentMap` as globals (added in the same release as the
fragment router).

## 1. Minimal polling tile

```tsx
export const fragments: FragmentMap = {
  table: Fragment(async (_ctx) => {
    return (
      <table>
        <tr>
          <th>id</th>
          <th>name</th>
        </tr>
        <tr>
          <td>1</td>
          <td>apple</td>
        </tr>
      </table>
    );
  }),
};

export default Page(async function () {
  return (
    <html>
      <head>
        <title>Users</title>
        <HtmxScript />
      </head>
      <body>
        <h1>Users</h1>
        <HtmxFragment
          page="/users"
          name="table"
          trigger="every 5s"
        />
      </body>
    </html>
  );
});
```

What the browser sees on first render:

```html
<html>
  <head>
    <title>Users</title>
    <script src="/__static/htmx.js"></script>
  </head>
  <body>
    <h1>Users</h1>
    <div hx-get="/users.tsp/__fragment/table" hx-trigger="every 5s">
      <table>
        <tr>
          <th>id</th>
          <th>name</th>
        </tr>
        <tr>
          <td>1</td>
          <td>apple</td>
        </tr>
      </table>
    </div>
  </body>
</html>
```

The framework calls `fragments.table(ctx)` once during SSR to populate the div.
After that, htmx polls every 5s and replaces the div with whatever the server
returns.

## 2. Click-to-load with query params

```tsx
export const fragments: FragmentMap = {
  search: Fragment(async (_ctx, { query, createZod }) => {
    const z = await createZod();
    const { q } = query(z.object({
      q: z.string().default(""),
    }));
    return (
      <ul id="results">
        {q ? <li>Searching for: {q}</li> : <li>Type to search</li>}
      </ul>
    );
  }),
};

export default Page(async function () {
  return (
    <html>
      <head>
        <title>Search</title>
        <HtmxScript />
      </head>
      <body>
        <input
          type="search"
          name="q"
          placeholder="Type a query"
        />
        <HtmxFragment
          page="/search"
          name="search"
          trigger="keyup changed delay:300ms from:input[name=q]"
          target="#results"
        />
        <ul id="results">
          <li>Type to search</li>
        </ul>
      </body>
    </html>
  );
});
```

The fragment reads `q` from the query params htmx sends. The `target` points at
the static `<ul id="results">` below the input — the input stays put, only the
result list swaps.

## 3. Inline edit form that refreshes the parent tile

```tsx
export const fragments: FragmentMap = {
  row: Fragment(async (ctx, { db, query }) => {
    const { id } = query({ id: Number });
    return (
      <tr id={`row-${id}`}>
        <td>{id}</td>
        <td>active</td>
      </tr>
    );
  }),

  // Fragment that mutates state and returns the updated tile.
  // The default page never calls this directly — it is the
  // form's POST target.
  saveRow: Fragment(async (ctx, { body, db, createZod, response }) => {
    const z = await createZod();
    const data = body(z.object({ id: z.coerce.number(), name: z.string() }));
    await db.execute(
      { affectedRows: z.number(), insertId: z.number() },
      "UPDATE users SET name = ? WHERE id = ?",
      [data.name, data.id],
    );
    // Re-render the updated row and return it. htmx will swap
    // the surrounding <tr> because the form's hx-target is the
    // closest <tr>.
    return (
      <tr id={`row-${data.id}`}>
        <td>{data.id}</td>
        <td>{data.name}</td>
      </tr>
    );
  }),
};

export default Page(async function () {
  return (
    <html>
      <head>
        <title>Users</title>
        <HtmxScript />
      </head>
      <body>
        <table>
          <tbody>
            <HtmxFragment page="/users" name="row" />
            <tr>
              <td>2</td>
              <td>
                <form
                  hx-post={hxUrl("/users", "saveRow")}
                  hx-trigger="submit"
                  hx-target="closest tr"
                  hx-swap="outerHTML"
                >
                  <input type="hidden" name="id" value="2" />
                  <input type="text" name="name" value="bob" />
                  <button type="submit">Save</button>
                </form>
              </td>
            </tr>
          </tbody>
        </table>
      </body>
    </html>
  );
});
```

The form posts to the `saveRow` fragment, which mutates the row and returns the
updated `<tr>`. htmx swaps it into the table. The `row` fragment is the static
tile; it is re-fetched on the next `every 5s` poll.

## 4. Explicit children instead of auto-fetch

```tsx
export const fragments: FragmentMap = {
  counter: Fragment(async () => <span>0</span>),
};

export default Page(async function () {
  // Skip the auto-fetch and ship a placeholder. Useful when the
  // initial content is something the fragment does not produce
  // (e.g. a spinner, a default that is not a server render).
  return (
    <html>
      <head>
        <HtmxScript />
      </head>
      <body>
        <HtmxFragment
          page="/dashboard"
          name="counter"
          trigger="load delay:200ms"
        >
          <em>loading...</em>
        </HtmxFragment>
      </body>
    </html>
  );
});
```

When the user passes children, the pre-walk does not call
`fragments[name](ctx)`. The supplied children are used verbatim.

## 5. Cross-page fragment

```tsx
// /shared/notifications.tsp
export const fragments: FragmentMap = {
  bell: Fragment(async (_ctx, { db }) => {
    const count = await db.scalar(
      z.number(),
      "SELECT COUNT(*) FROM notifications WHERE read = 0",
    );
    return <span id="bell" hx-swap-oob="true">{count}</span>;
  }),
};

// /dashboard.tsp — pulls in a fragment from a different page.
export default Page(async function () {
  return (
    <html>
      <head>
        <HtmxScript />
      </head>
      <body>
        <h1>Dashboard</h1>
        <HtmxFragment
          page="/shared/notifications"
          name="bell"
          trigger="every 10s"
        />
      </body>
    </html>
  );
});
```

`page` accepts the other page's path; the framework builds the `/__fragment/`
URL via `hxUrl` and the htmx request goes to the other page's module. The
initial content comes from the other page's `fragments.bell`; if it cannot be
found, the framework prints a single `console.warn` and renders nothing.

## 6. Manual control via `hxUrl`

When the user does not want the `<HtmxFragment>` wrapper (for example, to attach
htmx attributes to a custom element), build the URL with `hxUrl` and write the
attributes by hand:

```tsx
<a
  href={hxUrl("/users", "row")}
  hx-get={hxUrl("/users", "row")}
  hx-trigger="mouseenter once"
  hx-target="#preview"
>
  Preview row
</a>;
```

`hxUrl` is a pure function — it has no side effects and never throws on a valid
page/name pair.
