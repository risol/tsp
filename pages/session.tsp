// Slice 16k: ctx.session (spec sect.16) fixture.
//
// GET /session  -> shows id + the "counter" key, then
//                  increments it. First request creates a
//                  fresh session (no cookie); subsequent
//                  requests reuse it. Because the session
//                  service is `is_request_varying()`, the
//                  generation cache is bypassed every time
//                  so each request actually runs the page.
// POST /session {action: "regenerate"} -> regenerate()
//                  (16.3): new id, data preserved.
// POST /session {action: "destroy"}    -> destroy() (16.4):
//                  session cleared, cookie cleared on
//                  next response (Max-Age=0).
// POST /session {action: "reset"}      -> delete("counter").
//                 200 text body in either case for grep.
export async function GET(ctx) {
  const before = ctx.session.get('counter') || 0;
  const sid = ctx.session.id;
  ctx.session.set('counter', Number(before) + 1);
  return new Response(`session id=${sid} counter=${before}`, {
    status: 200,
    headers: { 'content-type': 'text/plain' },
  });
}
export async function POST(ctx) {
  let payload;
  try { payload = await ctx.request.json(); } catch (e) { payload = {}; }
  const action = (payload && payload.action) || '';
  if (action === 'regenerate') {
    await ctx.session.regenerate();
    return new Response(`regenerated new-id=${ctx.session.id} counter=${ctx.session.get('counter') || 0}`, {
      status: 200,
      headers: { 'content-type': 'text/plain' },
    });
  }
  if (action === 'destroy') {
    await ctx.session.destroy();
    return new Response('destroyed', {
      status: 200,
      headers: { 'content-type': 'text/plain' },
    });
  }
  if (action === 'reset') {
    ctx.session.delete('counter');
    return new Response(`reset counter=${ctx.session.get('counter')}`, {
      status: 200,
      headers: { 'content-type': 'text/plain' },
    });
  }
  return new Response(`unknown action=${action}`, {
    status: 400,
    headers: { 'content-type': 'text/plain' },
  });
}