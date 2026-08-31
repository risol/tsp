// Slice 16d: full Context shape -- request / url / query
// (URLSearchParams) / signal, plus a body-bearing POST that
// echoes ctx.request.text(). Slice 16f adds ctx.cookies
// (spec sect.15) with read+write API, plus the Set-Cookie
// merge path so cookie writes are reflected on the response
// even when the handler returns an HtmlNode (string).
export function GET(ctx) {
  // Read existing cookies from the request, then write a
  // session cookie. The HtmlNode return path must still
  // surface the Set-Cookie wire line (spec sect.15: "Cookie
  // writes MUST be reflected in the outgoing response even
  // when the handler returns an HtmlNode").
  const seen = ctx.cookies.get('sid') || 'none';
  ctx.cookies.set('sid', 's_' + Math.random().toString(36).slice(2, 8), { path: '/', httpOnly: true });
  ctx.cookies.set('theme', 'dark', { path: '/', maxAge: 3600 });
  return `<!doctype html><h1>Hello ${ctx.method} ${ctx.path}</h1><p>seen=${seen}</p><p>cookies.has(sid)=${ctx.cookies.has('sid')}</p>`;
}

export async function POST(ctx) {
  const raw = await ctx.request.text();
  const sig = ctx.signal.aborted ? 'aborted' : 'pending';
  // Two cookie writes on the Response return path; both
  // must reach the wire as separate Set-Cookie lines.
  const resp = new Response(`echo:${raw} (signal=${sig})`, {
    status: 201,
    headers: {
      "x-demo": "slice16f",
      "content-type": "text/plain",
      "x-method": ctx.request.method,
    },
  });
  ctx.cookies.set('a', 'v2', { path: '/' });
  ctx.cookies.set('b', 'v2', { path: '/' });
  return resp;
}
