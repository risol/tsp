// Slice 17c demo: bun's native `Bun.password` (exposed as
// `tsp:server.password`) for password hashing + verification.
//
// Two endpoints that exercise the page-side API and a third
// that flips the algorithm to demonstrate that
// `__tspServer.password` is just `Bun.password` re-exported,
// so any future `Bun.password.*` API surface is automatically
// available to the page without a v2 release.
//
//   GET  /password                    -> bcrypt round=4 hash of
//                                        the literal "hello" + a
//                                        salt-style marker. Proves
//                                        the namespace, hashSync,
//                                        and that the result
//                                        starts with `$2b$04$`.
//
//   POST /password  body={pw,hash}     -> 200 when verifySync
//                                        returns true (the
//                                        password matches the
//                                        stored hash), 400
//                                        otherwise. This is the
//                                        actual auth path:
//                                        "given the stored
//                                        `$2b$...` and the
//                                        user's submitted
//                                        password, do they
//                                        match?".
//
//   GET  /password?algo=argon2id       -> argon2id hash of "hello".
//                                        Demonstrates that the
//                                        same namespace supports
//                                        algorithms other than
//                                        bcrypt. argon2id is
//                                        the bun default and the
//                                        OWASP 2024+
                                        recommendation for new
//                                        password storage;
//                                        bcrypt remains here
//                                        for verifying legacy
//                                        `$2b$...` hashes
//                                        already in the
//                                        database.
//
// `round=4` is the bcrypt minimum, kept low so the integration
// test runs in well under a second; production should use
// `round=10-12` for bcrypt or `argon2id` with sane parameters
// (config knob in v2.5+).
import { password } from "tsp:server";

export async function GET(ctx) {
  const algo = ctx.query.get("algo") || "bcrypt";
  const hash = password.hashSync("hello", { algorithm: algo, cost: 4 });
  return new Response(
    JSON.stringify({
      ok: true,
      algorithm: algo,
      hash,
      isArgon: hash.startsWith("$argon2id$"),
      isBcrypt: hash.startsWith("$2b$"),
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
  const matches = password.verifySync(parsed.pw, parsed.hash);
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
