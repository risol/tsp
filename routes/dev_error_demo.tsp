// §32.1: demo route for the dev error page slice.
// The page throws on every request; the host catches
// the error and renders the dev error page HTML in
// `TSP_DEVELOPMENT=1` mode, or returns a 500 with the
// raw JSON error body in prod (the application can
// log it; the user sees a generic 500).
//
// The throw shape is intentionally varied across two
// query params so the e2e can pin each:
//   ?kind=range   -> throws `new RangeError("index out of bounds")`
//   ?kind=plain   -> throws `new Error("plain boom")` (default)
//   ?kind=quiet   -> throws a string (no Error instance)
//
// The e2e asserts (a) the page reaches the host
// (status 500), (b) the body in dev mode is HTML
// containing the error name + message + stack, and
// (c) the body in prod mode is the JSON envelope.

export function GET(ctx) {
  const kind = ctx.query.get("kind") || "plain";
  if (kind === "range") {
    throw new RangeError("index out of bounds: " + (ctx.query.get("idx") || "0"));
  }
  if (kind === "quiet") {
    // `throw "string"` is legal JS; the wrap must
    // handle non-Error throws without crashing.
    throw "string-throw-not-an-error";
  }
  throw new Error("plain boom from dev_error_demo");
}
