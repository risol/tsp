export function GET() {
  return new Response("Hello from TSP v2", {
    status: 200,
    headers: { "content-type": "text/plain" },
  });
}
