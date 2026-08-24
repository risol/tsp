// 01-hello.tsp -- the minimal page.
// Maps to:  GET  /
//
// Demonstrates: freeze 1 (standard TSX), freeze 4 (named GET export),
// freeze 5 (return HtmlNode), freeze 9 (string children are escaped).
//
// Note: the slice-6 PoC 1 subprocess bridge can run this. A future
// in-process JSC bridge (slice 9+) will also run it.

import { type Context, type PageConfig } from "tsp:server";

export const config = {
  cache: "no-store",
} satisfies PageConfig;

export function GET(_ctx: Context) {
  return <h1>Hello from TSP v2</h1>;
}
