// 05-list-with-fragment.tsp -- page + named fragment.
// Maps to:  GET  /users
//           GET  /...fragment route.../list
//
// Demonstrates: freeze 7 (fragment() helper, ctx.fragment(name)
// returns the URL), freeze 4 (page GET export + named fragment
// export), freeze 6 (fragment handler receives a Context too).

import {
  type Context,
  type PageConfig,
  fragment,
} from "tsp:server";

export const config = {
  auth: "required",
  cache: "no-store",
} satisfies PageConfig;

export function GET(ctx: Context) {
  return (
    <html>
      <head><title>Users</title></head>
      <body>
        <h1>Users</h1>
        <button
          hx-get={ctx.fragment("list")}
          hx-target="#user-list"
          hx-trigger="click"
        >
          Refresh
        </button>
        <section id="user-list">
          {/* fragment contents are rendered server-side at /...list */}
        </section>
      </body>
    </html>
  );
}

export const list = fragment({
  method: "GET",
  async handler(_ctx: Context) {
    const users = await _ctx.services.users.list({ limit: 50 });
    return (
      <ul>
        {users.map((user) => (
          <li data-id={user.id}>{user.name}</li>
        ))}
      </ul>
    );
  },
});
