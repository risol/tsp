export function GET(ctx) {
  return `<!doctype html><h1>User ${ctx.params.id}</h1><p>query=${ctx.query.get("q")}</p><p>path=${ctx.url.pathname}</p>`;
}
