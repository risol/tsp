// Slice 16n': wrap-side abort signalling (spec sect.13.7).
// The host's per-request watchdog writes ABORT_MARKER ("A\n") to the
// bun subprocess stdin at TSP_TIMEOUT_MS. The wrap preamble (jsx.rs)
// registers a module-scope `process.stdin` 'data' listener that calls
// `__tspAbortCtrl__.abort()`, which fires `ctx.signal`. This page
// honors the signal: it awaits a promise that rejects on abort, so a
// timeout must terminate the page cooperatively (not be hard-killed).
//
// The host still reports the timeout as 500 [TSP3012] (a timed-out
// request is a failure), but the *stderr* line `[ABORT-REACHED]`
// proves the signal reached page code in time for cooperative cancel.
// Deeper note: this fixture reads nothing and writes nothing, so a
// timeout-cancel path is purely about `ctx.signal` propagation.
export async function GET(ctx) {
  let fired = false;
  ctx.signal.addEventListener('abort', () => { fired = true; });
  try {
    await new Promise((_, reject) => {
      const t = setTimeout(() => reject(new Error('timer-expired')), 3000);
      ctx.signal.addEventListener('abort', () => { clearTimeout(t); reject(new Error('aborted-by-signal')); });
    });
    return new Response(`reached-end fired=${fired}`, {
      status: 200,
      headers: { 'content-type': 'text/plain' },
    });
  } catch (e) {
    console.error(`[ABORT-REACHED] fired=${fired} err=${e.message}`);
    return new Response(`aborted fired=${fired} err=${e.message}`, {
      status: 200,
      headers: { 'content-type': 'text/plain' },
    });
  }
}
