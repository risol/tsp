export async function GET(ctx) {
// Slice 16i: a slow handler that should hit the host's
// per-request timeout (TSP_TIMEOUT_MS, default 30s; the
// E2E run used TSP_TIMEOUT_MS=3000). The handler awaits
// a 3-second promise racing a 3s timer; the host
// watchdog writes the ABORT_MARKER to the bun stdin at
// timeout_ms, waits a 1s grace period, then hard-kills
// the subprocess (bun exits non-zero -> host returns
// 500 [TSP3012]). Note: the wrap-side abort listener
// that would let `ctx.signal` fire inside the page is
// deferred post-16i; in this fixture the catch path is
// what runs if the 3s timer loses the race.
// TSP_TIMEOUT_MS=0 disables the watchdog -> hangs for
// the full 3s and returns the catch-path 200.
let signalFired = false;
ctx.signal.addEventListener('abort', () => { signalFired = true; });
try {
  await new Promise((_, reject) => {
    const t = setTimeout(() => reject(new Error('timeout-expected-abort')), 3000);
    ctx.signal.addEventListener('abort', () => { clearTimeout(t); reject(new Error('aborted')); });
  });
  return new Response(`signalFired=${signalFired} (reached-end)`, {
    status: 200,
    headers: { "content-type": "text/plain" },
  });
} catch (e) {
  return new Response(`signalFired=${signalFired}; err=${e.message}`, {
    status: 200,
    headers: { "content-type": "text/plain" },
  });
}
}
