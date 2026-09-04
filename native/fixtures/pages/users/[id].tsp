import { json } from "tsp:server";

export function GET(ctx) {
  return json({ ok: true, id: ctx.params.id, path: ctx.url.pathname });
}
