import { json } from "tsp:server";

export function GET(ctx) {
  return json({
    ok: true,
    method: ctx.method,
    path: ctx.url.pathname,
    query: ctx.query.get("q"),
  });
}

export async function POST(ctx) {
  const body = await ctx.request.text();
  return new Response(`echo:${body}`, {
    status: 201,
    headers: { "content-type": "text/plain", "x-tsp-case": "post" },
  });
}
