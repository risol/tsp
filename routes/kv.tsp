// Slice 22 follow-up: `kind: kv` demo. The host reads a
// per-service KV map from `tsp.config.json` and surfaces
// it to the page as `ctx.services.<name>.entries.<key>`.
// The map is a frozen snapshot (read-only on the page
// side) -- the source of truth is the host's config file;
// changes ship on master restart. A future slice may add
// a write-back path through `ctx.session` or a
// dedicated mutation envelope field.
//
//   GET /kv            -> echoes the entire entries map
//   GET /kv?key=X      -> echoes one entry (null if missing)
//
// The page mirrors `routes/counter.tsp` / `routes/flags.tsp`
// shape: a single GET that reports a `x-demo` header so
// the e2e can assert the page actually ran (200) versus
// the host rejecting (4xx with TSP error code).
export function GET(ctx) {
  const config = ctx.services.config;
  const key = ctx.url.searchParams.get("key");
  const entries = config && config.entries ? config.entries : {};
  if (key) {
    return new Response(
      JSON.stringify({ ok: true, key, value: entries[key] ?? null }),
      {
        status: 200,
        headers: { "content-type": "application/json", "x-demo": "slice22-kv" },
      }
    );
  }
  return new Response(
    JSON.stringify({ ok: true, entries }),
    {
      status: 200,
      headers: { "content-type": "application/json", "x-demo": "slice22-kv" },
    }
  );
}
