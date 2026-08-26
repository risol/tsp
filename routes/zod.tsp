// Slice 17b: zod 3.25.76 compiled into the TSP runtime.
// The page can declare schemas / call `safeParse` by importing
// from the `tsp:server` virtual module -- per plan §16.4,
// framework API must NOT be exposed on `globalThis`. The host's
// wrap preamble inlines the pre-bundled CJS and exposes the zod
// namespace on the frozen `__tspServer.zod` that the rewriter
// destructures against.
//
// Two endpoints that exercise the most common zod shapes:
//
//   GET  /zod              -> 200 with a default parsed object
//                             (proves the namespace, the schema
//                             builder, the coercion chain, and
//                             the safeParse() return shape are
//                             all wired through the prelude).
//
//   POST /zod              -> echoes the body after parsing
//                             against a stricter schema:
//                               { email: zod.string().email(),
//                                 age:   zod.coerce.number().int() }
//                             On failure returns 400 with the
//                             `error.issues` array so a test
//                             can assert the validation path
//                             without scraping internals.
import { zod } from "tsp:server";

export function GET(_ctx) {
  const schema = zod.object({
    name: zod.string(),
    age: zod.coerce.number().int().min(0).max(150),
  });
  const result = schema.safeParse({ name: "alice", age: "30" });
  if (!result.success) {
    return new Response(JSON.stringify({ ok: false, issues: result.error.issues }), {
      status: 400,
      headers: { "content-type": "application/json" },
    });
  }
  return new Response(
    JSON.stringify({ ok: true, name: result.data.name, age: result.data.age }),
    {
      status: 200,
      headers: { "content-type": "application/json", "x-demo": "slice17b" },
    }
  );
}

export async function POST(ctx) {
  const schema = zod.object({
    email: zod.string().email(),
    age: zod.coerce.number().int().min(0).max(150),
  });
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
  const result = schema.safeParse(parsed);
  if (!result.success) {
    return new Response(
      JSON.stringify({ ok: false, issues: result.error.issues.map((i) => i.message) }),
      { status: 400, headers: { "content-type": "application/json" } }
    );
  }
  return new Response(
    JSON.stringify({ ok: true, email: result.data.email, age: result.data.age }),
    { status: 200, headers: { "content-type": "application/json", "x-demo": "slice17b" } }
  );
}
