// 06-async-component.tsp -- async component composition.
// Maps to:  GET  /dashboard
//
// Demonstrates: freeze 10 (async components), freeze 5 (handler
// returns HtmlNode), freeze 6 (ctx.services + ctx.session).

import { type Context, type PageConfig } from "tsp:server";

export const config = {
  auth: "required",
  cache: "no-store",
} satisfies PageConfig;

async function UserBadge({ id }: { id: string }) {
  const user = await fetchUser(id);
  return <span class="badge">{user.name}</span>;
}

async function RecentOrders({ userId }: { userId: string }) {
  const orders = await fetchOrders(userId, { limit: 5 });
  return (
    <ul>
      {orders.map((order) => (
        <li>#{order.id} - {order.total}</li>
      ))}
    </ul>
  );
}

export async function GET(ctx: Context) {
  const user = ctx.session.get<{ id: string }>("user");
  if (!user) {
    return <p>not signed in</p>;
  }
  return (
    <html>
      <head><title>Dashboard</title></head>
      <body>
        <h1>
          Hello <UserBadge id={user.id} />
        </h1>
        <h2>Recent orders</h2>
        <RecentOrders userId={user.id} />
      </body>
    </html>
  );
}

// Stubs for the example; in real code these are imported from
// services or lib/. They are kept here so the example is
// self-contained.
async function fetchUser(_id: string) {
  return { name: "Ada" };
}
async function fetchOrders(_userId: string, _opts: { limit: number }) {
  return [{ id: 1, total: 42 }];
}
