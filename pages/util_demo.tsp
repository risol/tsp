// Slice 18 demo: bun's built-in `__tspServer.util` namespace
// (slice 18 forward). The page exercises a few of the 17
// surfaces the wrap exposes -- enough to prove the bridge
// works end-to-end without bloating the wrap / prelude.
//
// All endpoints return JSON. The first one (`GET /util_demo`)
// is a kitchen-sink that demos hash / uuid / markdown /
// escape / config-parse; the others each demo a single
// feature so the integration test can pick and choose.
import { util } from "tsp:server";

function jsonResponse(value, status = 200) {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json", "x-demo": "slice18" },
  });
}

export function GET(_ctx) {
  // randomUUIDv7
  const id = util.randomUUIDv7();
  // hash (xxhash; numeric)
  const hash = util.hash(new TextEncoder().encode("hello, world"));
  // escapeHTML
  const safe = util.escapeHTML("<script>alert('xss')</script>");
  // markdown render
  const html = util.markdown.html("# Title\n\n**bold** _italic_");
  // YAML parse (config file)
  const config = util.YAML.parse("version: 1\nname: demo\n");
  // TOML parse
  const toml = util.TOML.parse("key = \"value\"\nn = 42\n");
  // sha256 via CryptoHasher
  const sha = new util.CryptoHasher("sha256");
  sha.update("hello");
  const digest = sha.digest("hex");
  return jsonResponse({
    ok: true,
    id,
    hash,
    sha256: digest,
    safe,
    markdown: html,
    config,
    toml,
  });
}

// POST /util_demo with body {"password": "..."} -> derive a
// stable 32-bit key via Bun.hash (xxhash) for the password.
// This is the kind of "session-derivation" use case the slice
// 18 util namespace is designed for: a page needs a fast
// non-cryptographic hash of an arbitrary string to use as a
// cache key / row id. Bun's xxhash is ~10x faster than
// crypto.createHash('sha256') for the same input.
export async function POST(ctx) {
  const body = await ctx.request.text();
  let parsed;
  try {
    parsed = JSON.parse(body);
  } catch {
    return jsonResponse({ ok: false, error: "body must be JSON" }, 400);
  }
  if (typeof parsed.password !== "string" || !parsed.password) {
    return jsonResponse({ ok: false, error: "password must be a non-empty string" }, 400);
  }
  const key = util.hash(new TextEncoder().encode(parsed.password));
  return jsonResponse({ ok: true, key });
}
