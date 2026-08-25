export async function GET(ctx) {
// Slice 16j (Phase 8): ctx.services demo (spec sect.17).
// The runtime-scoped `logger` lives in the host registry;
// `total_lines` counts every log line flushed from PREVIOUS
// requests (each request's envelope carries this request's
// log back; the host flushes before responding, so the next
// request observes it).
// Because the logger reports is_request_varying(), every
// render bypasses the generation cache -- the page actually
// runs on each request (see the 16j ledger note).
// Expected sequence (E2E):
//   GET /svc        -> svc lines=0   (page logs 1 line; flushed after)
//   GET /svc        -> svc lines=1   (state survived request 1)
//   edit this file  -> watcher reloads the generation
//   GET /svc        -> svc lines=2   (service NOT rebuilt -- plan
//                                     sect.61 Phase 8 acceptance)
const logger = ctx.services.logger;
const before = typeof logger.total_lines === 'number' ? logger.total_lines : -1;
logger.info(`svc-hit from ${ctx.url.pathname}`);
return new Response(`svc lines=${before}`, {
  status: 200,
  headers: { "content-type": "text/plain" },
});
}
