// 10-shape-magic.tsp -- the wrong way to return a response.
// This file is INTENTIONALLY INVALID; it is kept here as a
// reference for the host's TSP3001 error.
//
// Demonstrates: freeze 5 -- the only legal HandlerResult shapes
// are HtmlNode and Response. Returning a plain object that "looks
// like" a redirect is rejected at the host's runtime boundary.

import type { Context, PageConfig } from "tsp:server";

export const config = { auth: "none" } satisfies PageConfig;

export function GET(_ctx: Context) {
  // The host will reject this with TSP3001: handler returned
  // unsupported value Object. Expected HtmlNode or Response.
  return { redirect: "/login" };
}
