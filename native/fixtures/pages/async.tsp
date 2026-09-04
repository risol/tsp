import { text } from "tsp:server";

export async function GET(ctx) {
  const value = await Promise.resolve(`async:${ctx.url.pathname}`);
  return text(value, 201, { "x-tsp": "native" });
}
