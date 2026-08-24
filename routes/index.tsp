// Slice 16a + 16b + 16c: Context + Response ABI + headers.
export function GET(ctx) {
  return `<!doctype html><h1>Hello ${ctx.method} ${ctx.path}</h1><p>q=${ctx.query}</p>`;
}

export function POST() {
  return new Response("created", {
    status: 201,
    headers: {
      "x-demo": "slice16c",
      "content-type": "application/json",
      "x-comma": "a,b,c",
    },
  });
}