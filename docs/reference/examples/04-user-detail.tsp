// 04-user-detail.tsp -- dynamic segment.
// Maps to:  GET  /users/:id
//
// Demonstrates: freeze 3 (filename `[id].tsp` -> :id), freeze 6
// (ctx.params.id is a string), freeze 5 (notFound helper for 404),
// freeze 9 (string from ctx.services is escaped in the echo).

import {
  type Context,
  type PageConfig,
  notFound,
} from "tsp:server";

export const config = {
  auth: "required",
  cache: "no-store",
} satisfies PageConfig;

export async function GET(ctx: Context) {
  const id = ctx.params.id; // always a string
  const user = await ctx.services.users.get(id);
  if (!user) {
    return notFound();
  }
  return (
    <html>
      <head><title>{user.name}</title></head>
      <body>
        <h1>{user.name}</h1>
        <p>id: {user.id}</p>
        <p>email: {user.email}</p>
      </body>
    </html>
  );
}
