import { type Context, type PageConfig } from "tsp:server";

export const config = {
  cache: "no-store",
} satisfies PageConfig;

export function GET(_ctx: Context) {
  return <h1>Hello from TSP</h1>;
}
