// Slice 17a: nanoid 5.1.6 compiled into the TSP runtime.
// The page can call `nanoid()` (and `customAlphabet`,
// `customRandom`, `random`) by importing from the `tsp:server`
// virtual module -- per plan §16.4, framework API must NOT be
// exposed on `globalThis`. The host's wrap preamble inlines the
// runtime and exposes it on the frozen `__tspServer` object that
// the rewriter destructures against.
//
// Two endpoints:
//   GET  /nanoid            -> 200 with a fresh 21-char id (default size)
//   POST /nanoid  body={"size": 10} -> 200 with a 10-char id
//
// Both responses are text/plain so the smoke + regression
// tests can assert on the body fingerprint directly.
import { nanoid } from "tsp:server";

export function GET(_ctx) {
  return new Response(nanoid(), {
    status: 200,
    headers: { "content-type": "text/plain" },
  });
}

export async function POST(ctx) {
  const body = await ctx.request.text();
  let size = 21;
  if (body) {
    try {
      const parsed = JSON.parse(body);
      if (typeof parsed.size === "number" && parsed.size > 0) {
        size = parsed.size;
      }
    } catch {
      // body wasn't JSON, ignore and use default size
    }
  }
  return new Response(nanoid(size), {
    status: 200,
    headers: { "content-type": "text/plain", "x-demo": "slice17a" },
  });
}
