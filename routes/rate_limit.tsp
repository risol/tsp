// Slice 22 + Amendment 8: `kind: rate_limit` demo.
// The page reads the rate-limit snapshot and gates
// the response on `count > limit`. The host tracks
// the count per request (every `ctx.services` snapshot
// post-increments the counter; see services.rs).
//
// Config the e2e ships with this route:
//   "rate": { "kind": "rate_limit", "limit": 2, "window_seconds": 60 }
//
// Endpoints:
//   GET /rate_limit          -> 200 with the current
//                                snapshot (count, limit,
//                                remaining, window_start_ms)
//                                until the count exceeds
//                                the limit; then 429.
//   GET /rate_limit?kind=info
//                            -> 200 with the snapshot
//                                regardless of the
//                                over-limit state (for
//                                the e2e to read the
//                                post-over-limit count).
//
// The e2e asserts the count increments monotonically
// across requests, the `remaining` field decrements,
// and the response status flips to 429 once the
// count exceeds the limit.

export function GET(ctx) {
  const rate = ctx.services.rate;
  // `kind=info` is a debug-mode read that does NOT
  // gate the response -- the e2e uses it after the
  // count has been pushed over the limit to read the
  // current snapshot.
  const kind = ctx.query.get("kind") || "gate";
  if (kind === "info") {
    return new Response(
      JSON.stringify({
        ok: true,
        kind: "info",
        count: rate.count,
        limit: rate.limit,
        remaining: rate.remaining,
        window_ms: rate.window_ms,
        window_start_ms: rate.window_start_ms,
      }),
      {
        status: 200,
        headers: { "content-type": "application/json", "x-demo": "rate-limit" },
      }
    );
  }
  // Default: gate. The page returns 429 when the
  // post-incremented count exceeds the limit.
  if (rate.count > rate.limit) {
    return new Response(
      JSON.stringify({
        ok: false,
        over_limit: true,
        count: rate.count,
        limit: rate.limit,
      }),
      {
        status: 429,
        headers: {
          "content-type": "application/json",
          "x-demo": "rate-limit",
          "retry-after": "60",
        },
      }
    );
  }
  return new Response(
    JSON.stringify({
      ok: true,
      kind: "gate",
      count: rate.count,
      limit: rate.limit,
      remaining: rate.remaining,
    }),
    {
      status: 200,
      headers: { "content-type": "application/json", "x-demo": "rate-limit" },
    }
  );
}
