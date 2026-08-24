// 08-catch-all.tsp -- catch-all route.
// Maps to:  GET  /files/*path
//
// Demonstrates: freeze 3 (filename `[...path].tsp` -> catch-all
// segment surfaced as `ctx.params.path` with the sub-path joined
// by `/`), freeze 6 (ctx.params.path is "a/b/c" not "/a/b/c"),
// freeze 5 (404 via notFound when the file does not exist).

import {
  type Context,
  type PageConfig,
  notFound,
  text,
} from "tsp:server";

export const config = {
  auth: "required",
  cache: "private",
  timeoutMs: 5000,
} satisfies PageConfig;

export async function GET(ctx: Context) {
  // ctx.params.path is the sub-path joined by "/". For
  // /files/a/b/c this is "a/b/c" (no leading slash).
  const sub = ctx.params.path ?? "";
  const blob = await ctx.services.files.read(sub);
  if (!blob) {
    return notFound();
  }
  // For text files we render as-is; for binaries we'd return a
  // streamed Response. The example keeps it simple.
  return text(await blob.text(), {
    headers: { "Content-Type": blob.type },
  });
}
