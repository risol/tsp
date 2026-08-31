// Slice 16f: ctx.cookies (spec sect.15) fixture.
//
// The wrap preamble parses the request's Cookie header
// into ctx.cookies (read methods) and pushes Set-Cookie
// lines the page writes via ctx.cookies.set / .delete
// into a buffer that the async IIFE merges into the
// outgoing response headers. Each write becomes a
// separate Set-Cookie entry, so multiple cookies on one
// request do not collapse via the response's flatten loop.
//
//   GET  /cookies           -> echoes parsed cookies + 200 body
//   GET  /cookies/value?k=X -> reads one cookie, returns JSON
//   POST /cookies           body {name, value, options}
//                          -> ctx.cookies.set(...) ; 200 echo
//   DELETE /cookies?k=X     -> ctx.cookies.delete(...) ; 200
//
// All endpoints return JSON for grep-ability in tests.

function jsonResponse(value, status = 200, extraHeaders) {
  const headers = { "content-type": "application/json" };
  if (extraHeaders) for (const k in extraHeaders) headers[k] = extraHeaders[k];
  return new Response(JSON.stringify(value), { status, headers });
}

export function GET(ctx) {
  // Surface all the cookies the request carried. A real
  // page would only ask for the ones it cares about; we
  // dump them so the e2e test can verify the parser.
  // `?? null` because ctx.cookies.get returns `undefined`
  // for missing keys (Map.get convention) and
  // JSON.stringify drops undefined values.
  const sess = ctx.cookies.get("sess") ?? null;
  const theme = ctx.cookies.get("theme") ?? null;
  const lang = ctx.cookies.get("lang") ?? null;
  const all = {};
  for (const k of ["theme", "lang", "sess"]) {
    if (ctx.cookies.has(k)) all[k] = ctx.cookies.get(k);
  }
  return jsonResponse({
    ok: true,
    has_sess: ctx.cookies.has("sess"),
    has_theme: ctx.cookies.has("theme"),
    sess,
    theme,
    lang,
    all,
  });
}

export async function POST(ctx) {
  let body;
  try { body = await ctx.request.json(); }
  catch { return jsonResponse({ ok: false, error: "body must be JSON" }, 400); }
  if (typeof body.name !== "string" || !body.name) {
    return jsonResponse({ ok: false, error: "name must be a non-empty string" }, 400);
  }
  const value = String(body.value ?? "");
  const options = body.options || {};
  ctx.cookies.set(body.name, value, options);
  return jsonResponse({
    ok: true,
    name: body.name,
    value,
    options,
    written: true,
  });
}

export function DELETE(ctx) {
  const k = ctx.url.searchParams.get("k");
  if (!k) return jsonResponse({ ok: false, error: "missing k query param" }, 400);
  // .delete emits a Set-Cookie line with Max-Age=0 by
  // default; the test asserts on that line.
  ctx.cookies.delete(k);
  return jsonResponse({ ok: true, deleted: k });
}
