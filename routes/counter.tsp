// Slice 22 prototype: config-driven custom service
// (plan §17.5 / §21). The host registers a
// `CounterService` for each `services.<name>.kind =
// "counter"` entry in the JSON config file pointed at
// by `TSP_CONFIG` (default: `tsp.config.json`). The
// wire snapshot the page reads is `{"kind":"counter",
// "name":"<n>","value":<u64>}` with the value
// post-incremented.
//
//   GET /counter             -> { "hits": N, "views": M }
//   GET /counter?only=hits   -> { "hits": N }
//
// Because `CounterService::is_request_varying()` returns
// true (the value changes per snapshot), the host
// bypasses the generation cache for this page -- the
// page actually runs on each request, the value the
// page reads is the value the host snapshotted for THIS
// request, and the next request observes the next
// value. This is the slice 22 acceptance shape for
// "config-driven service visible in the page".
export function GET(ctx) {
  // `ctx.services` is the per-request snapshot the wrap
  // preamble hydrates from the host's `ServiceRegistry`.
  // Built-in services (logger / session / time) plus
  // any custom services the config declared are
  // available by name. The descriptor is frozen on the
  // page side, so direct mutation throws in strict mode
  // (and is silently ignored in sloppy mode) -- the
  // host owns the live state.
  const hits = ctx.services.hits ? ctx.services.hits.value : null;
  const views = ctx.services.views ? ctx.services.views.value : null;
  const only = ctx.url.searchParams.get("only");
  const out = {};
  if (only === "hits") {
    out.hits = hits;
  } else if (only === "views") {
    out.views = views;
  } else {
    out.hits = hits;
    out.views = views;
  }
  return new Response(JSON.stringify(out), {
    status: 200,
    headers: { "content-type": "application/json", "x-demo": "slice22" },
  });
}
