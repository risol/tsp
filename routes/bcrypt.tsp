// Slice 17c: bcryptjs 3.0.3 compiled into the TSP runtime.
// The page can call `bcrypt.hashSync()` / `compareSync()` /
// `genSaltSync()` / `getRounds()` by importing from the
// `tsp:server` virtual module -- per plan §16.4, framework API
// must NOT be exposed on `globalThis`. The host's wrap preamble
// inlines the pre-bundled CJS and exposes the bcrypt namespace
// on the frozen `__tspServer.bcrypt` that the rewriter
// destructures against.
//
// Two endpoints that exercise the most common bcrypt flows:
//
//   GET  /bcrypt               -> hashes the literal "hello" with
//                                 a 4-round salt and reports
//                                 { rounds, hash, salt }. Proves
//                                 the namespace, genSaltSync,
//                                 hashSync, and getRounds all
//                                 work end-to-end.
//
//   POST /bcrypt  body={pw,hash}  -> 200 when compareSync(pw, hash)
//                                     is true, 400 otherwise. The
//                                     path is the actual user
//                                     flow: "given the stored
//                                     hash and the user's
//                                     submitted password, do they
//                                     match?".
//
// round=4 is the bcrypt minimum, kept low so the integration
// test runs in under a second; production usage should default
// to 10-12 rounds (a config knob in v2.5+).
import { bcrypt } from "tsp:server";

export function GET(_ctx) {
  const salt = bcrypt.genSaltSync(4);
  const hash = bcrypt.hashSync("hello", salt);
  return new Response(
    JSON.stringify({
      ok: true,
      rounds: bcrypt.getRounds(hash),
      hash,
      salt,
    }),
    {
      status: 200,
      headers: { "content-type": "application/json", "x-demo": "slice17c" },
    }
  );
}

export async function POST(ctx) {
  const body = await ctx.request.text();
  let parsed;
  try {
    parsed = JSON.parse(body);
  } catch {
    return new Response(JSON.stringify({ ok: false, error: "body must be JSON" }), {
      status: 400,
      headers: { "content-type": "application/json" },
    });
  }
  if (typeof parsed.pw !== "string" || typeof parsed.hash !== "string") {
    return new Response(
      JSON.stringify({ ok: false, error: "pw and hash must be strings" }),
      { status: 400, headers: { "content-type": "application/json" } }
    );
  }
  const matches = bcrypt.compareSync(parsed.pw, parsed.hash);
  if (!matches) {
    return new Response(JSON.stringify({ ok: false, matches: false }), {
      status: 400,
      headers: { "content-type": "application/json", "x-demo": "slice17c" },
    });
  }
  return new Response(JSON.stringify({ ok: true, matches: true }), {
    status: 200,
    headers: { "content-type": "application/json", "x-demo": "slice17c" },
  });
}
