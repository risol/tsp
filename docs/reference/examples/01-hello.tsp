// 01-hello.tsp -- the minimal page.
// Maps to:  GET  /
//
// Demonstrates: freeze 1 (standard TSX), freeze 4 (named GET export),
// freeze 5 (return HtmlNode), freeze 9 (string children are escaped).
//
// This fixture is also used by the runtime smoke tests.

import { type Context, type PageConfig } from "tsp:server";

export const config = {
  cache: "no-store",
} satisfies PageConfig;

export function GET(_ctx: Context) {
  return <h1>Hello from TSP</h1>;
}
