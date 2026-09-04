import { json } from "tsp:server";

export function GET(ctx) {
  return json({
    ok: true,
    method: ctx.method,
    path: ctx.url.pathname,
    query: ctx.query.get("q"),
  });
}
