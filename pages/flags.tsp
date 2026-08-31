// Slice 22 follow-up: `kind: feature_flag` demo. The
// host reads a per-service boolean flag set from
// `tsp.config.json` and surfaces it to the page as
// `ctx.services.<name>.flags.<flag>`. The flag set is a
// frozen snapshot (read-only on the page side) -- the
// source of truth is the host's config file; changes ship
// on master restart.
//
//   GET /flags              -> echoes the entire flags map
//   GET /flags?check=X      -> echoes whether the named
//                             flag is enabled (true / false /
//                             null if not declared). The page
//                             uses this to gate code paths
//                             (new checkout flow, beta UI,
//                             A/B test bucket assignment, ...).
//
// The page mirrors `pages/kv.tsp` shape: a single GET
// that reports a `x-demo` header so the e2e can assert
// the page actually ran (200) versus the host rejecting
// (4xx with TSP error code).
export function GET(ctx) {
  const flags = ctx.services.flags;
  const flagMap = flags && flags.flags ? flags.flags : {};
  const check = ctx.url.searchParams.get("check");
  if (check) {
    const value = Object.prototype.hasOwnProperty.call(flagMap, check)
      ? flagMap[check]
      : null;
    return new Response(
      JSON.stringify({ ok: true, flag: check, value }),
      {
        status: 200,
        headers: { "content-type": "application/json", "x-demo": "slice22-flags" },
      }
    );
  }
  return new Response(
    JSON.stringify({ ok: true, flags: flagMap }),
    {
      status: 200,
      headers: { "content-type": "application/json", "x-demo": "slice22-flags" },
    }
  );
}
