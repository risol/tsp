// Slice 16a + 16b: use the Context and demonstrate the
// HtmlNode (string) return path. The Response path is
// exercised in unit tests; the fixture keeps just the GET
// for the smoke-test side because the slice 5 detector
// does not yet distinguish page-detected methods from the
// route's REAL method set -- adding POST here exposes the
// pre-existing DELETE/PUT/PATCH 500 fallback that needs
// its own fix.
export function GET(ctx) {
  return `<!doctype html><h1>Hello ${ctx.method} ${ctx.path}</h1><p>q=${ctx.query}</p>`;
}
