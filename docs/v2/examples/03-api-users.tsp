// 03-api-users.tsp -- JSON API endpoint.
// Maps to:  GET, POST  /api/users
//
// Demonstrates: freeze 4 (GET + POST), freeze 5 (json helper for
// non-HTML response), freeze 6 (ctx.services), freeze 9 (numbers
// and booleans in JSX render via toString, not as raw HTML).

import {
  type Context,
  type PageConfig,
  json,
} from "tsp:server";

export const config = {
  auth: "required",
  cache: "no-store",
} satisfies PageConfig;

export async function GET(ctx: Context) {
  const limit = Number(ctx.query.get("limit") ?? "20");
  const users = await ctx.services.users.list({ limit });
  return json({ users, limit });
}

export async function POST(ctx: Context) {
  const body = await ctx.request.json<{ name: string; email: string }>();
  if (!body.name || !body.email) {
    return json({ error: "name and email are required" }, { status: 400 });
  }
  const user = await ctx.services.users.create(body);
  return json({ user }, { status: 201 });
}
