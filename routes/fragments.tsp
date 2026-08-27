// Phase 9 — Fragments (plan §14 / FREEZE item 7)
//
// The `fragment()` helper is the v2 protocol for a page
// subtree reachable via an internal URL. The application
// declares fragments as named exports, then asks the host
// for the URL through `ctx.fragment("name")`. The host
// owns the URL shape -- the application MUST NOT hard-code
// any path (plan §14.3).
//
// The v1 contract is `fragment(handler)` with default
// method GET. The `{ method, handler }` form lands in a
// follow-up slice; the FREEZE Amendment 4 narrows the
// surface until the host has a real fragment-method
// validation step (so a POST to a GET fragment is rejected
// by the host, not by the page).
//
// This demo exposes two fragments:
//
//   userList     returns a JSON snapshot of three users.
//                No query params; the page that owns the
//                fragment embeds the URL in HTML and the
//                client-side runtime (htmx, fetch, etc.)
//                splices the response in.
//
//   echo         takes a `?msg=...` query param and
//                reflects it. The parent page passes the
//                `msg` arg to `ctx.fragment("echo", { msg: "hi" })`;
//                the wrap bakes that into the URL. The
//                fragment handler then reads it back via
//                `ctx.query.get("msg")` -- proving that
//                extra params survive the URL builder
//                unchanged.
//
// The parent GET returns JSON with both fragment URLs
// inlined so the e2e can assert the URL builder, the
// host dispatch, and the per-request capability check
// (wrong token -> 404) end-to-end.

import { fragment } from "tsp:server";

export const userList = fragment(async (_ctx) => {
  return new Response(
    JSON.stringify({
      ok: true,
      fragment: "userList",
      users: ["alice", "bob", "carol"],
    }),
    {
      status: 200,
      headers: {
        "content-type": "application/json",
        "x-fragment": "userList",
        "x-demo": "phase9-fragments",
      },
    }
  );
});

export const echo = fragment(async (ctx) => {
  // `ctx.query` is the full request URL's searchParams
  // (plan §13.5). The wrap puts the parent page's
  // `ctx.fragment("echo", { msg: "hi" })` arg into the
  // URL, so the fragment handler can read it back as a
  // normal `?msg=...` query value. A separate `client`
  // param (added by the test, or by a client-side
  // caller) shows up alongside the parent's baked arg
  // because URLSearchParams preserves all keys.
  const msg = ctx.query.get("msg") || "";
  const client = ctx.query.get("client") || "";
  return new Response(
    JSON.stringify({ ok: true, fragment: "echo", msg, client }),
    {
      status: 200,
      headers: {
        "content-type": "application/json",
        "x-fragment": "echo",
        "x-demo": "phase9-fragments",
      },
    }
  );
});

export function GET(ctx) {
  // `ctx.fragment(name, params?)` returns the opaque URL
  // the host assigned. The shape is internal -- the
  // application passes it through to the client (e.g.
  // as an `hx-get` attribute) without reformatting.
  const listUrl = ctx.fragment("userList");
  const echoUrl = ctx.fragment("echo", { msg: "hi" });
  return new Response(
    JSON.stringify({
      ok: true,
      page: ctx.url.pathname,
      fragmentUrls: { userList: listUrl, echo: echoUrl },
    }),
    {
      status: 200,
      headers: {
        "content-type": "application/json",
        "x-demo": "phase9-fragments",
      },
    }
  );
}
