import { json } from "tsp:server";

export async function POST(ctx) {
  const value = await ctx.request.json();
  return json({ received: value.message, method: ctx.request.method });
}
