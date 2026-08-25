// Slice 16m: call-capable read-only `time` service (spec sect.17).
// The host captures a fresh snapshot per request and
// embeds it in `ctx.services.time`; the wrap preamble
// freezes the descriptor so the page sees a normal JS
// object it can read but cannot call back into the host.
// Read fields:
//   - iso:        "YYYY-MM-DDTHH:MM:SS.sssZ"
//   - epoch_ms:   ms since the Unix epoch
//   - uptime_ms:  ms since the host process started
//   - kind/scope: stable strings ("time" / "runtime")
// The page just renders them; no further state, no
// state mutations, no service RPC. The next slice
// (16n+) lands the persistent JS adapter realm for
// call-capable services with a real IPC.
export async function GET(ctx) {
  const t = ctx.services.time;
  if (!t) {
    return new Response('time service not registered', {
      status: 500,
      headers: { 'content-type': 'text/plain' },
    });
  }
  return new Response(
    `iso=${t.iso} epoch_ms=${t.epoch_ms} uptime_ms=${t.uptime_ms} kind=${t.kind}`,
    {
      status: 200,
      headers: { 'content-type': 'text/plain' },
    },
  );
}