// PoC 1 fixture for plan §70 step 7.
// Slice 5 will transpile + evaluate this file in JSC, find the `GET`
// export, call it, and render the result into a 200 text/html response.
//
// Spec: this is a standard TSX module (plan §3.1). The `tsp:server`
// import will land in a later slice; PoC 1 only validates the "find
// GET, call it" path so the handler is argument-less for now.
export function GET() {
  return <h1>Hello from TSP v2</h1>;
}
