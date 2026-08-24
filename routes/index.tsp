// Slice 16a: use the Context object the host now injects.
export function GET(ctx) {
  return `<!doctype html><h1>Hello ${ctx.method} ${ctx.path}</h1><p>q=${ctx.query}</p>`;
}
