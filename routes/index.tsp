// Slice 16d: full Context shape -- request / url / query
// (URLSearchParams) / signal, plus a body-bearing POST that
// echoes ctx.request.text().
export function GET(ctx) {
  return `<!doctype html><h1>Hello ${ctx.method} ${ctx.path}</h1><p>url=${ctx.url}</p><p>q=${ctx.query}</p><p>host=${ctx.request.headers.get('host')}</p><p>signal=${ctx.signal.aborted ? 'aborted' : 'pending'}</p>`;
}

export async function POST(ctx) {
  const raw = await ctx.request.text();
  const sig = ctx.signal.aborted ? 'aborted' : 'pending';
  return new Response(`echo:${raw} (signal=${sig})`, {
    status: 201,
    headers: {
      "x-demo": "slice16d",
      "content-type": "text/plain",
      "x-method": ctx.request.method,
    },
  });
}