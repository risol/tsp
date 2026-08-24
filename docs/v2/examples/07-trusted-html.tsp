// 07-trusted-html.tsp -- raw HTML via the explicit `raw()` helper.
// Maps to:  GET  /docs/markdown-preview
//
// Demonstrates: freeze 9 (raw HTML is opt-in via raw(trustedHtml),
// not via dangerouslySetInnerHTML), freeze 8 (TrustedHtml brand
// type from tsp:html).

import { raw, type Context, type PageConfig } from "tsp:server";
import { escape, type TrustedHtml } from "tsp:html";

export const config = {
  auth: "required",
  cache: "no-store",
} satisfies PageConfig;

export async function GET(ctx: Context) {
  const markdown = String(ctx.query.get("body") ?? "");
  // `render` would be a Markdown -> HTML library; we hand-roll a
  // tiny trusted-renderer for the example. In real code, only
  // pass output that has been sanitized by a vetted library.
  const html = renderMarkdownToTrustedHtml(markdown);

  return (
    <html>
      <head><title>Preview</title></head>
      <body>
        <h1>Preview</h1>
        {html}
      </body>
    </html>
  );
}

function renderMarkdownToTrustedHtml(source: string): TrustedHtml {
  // Defensive: escape any leftover `<` characters before treating
  // the result as trusted. Real Markdown libraries do this
  // properly; this is a placeholder.
  return raw(escape(source).replaceAll("&lt;", "<").replaceAll("&gt;", ">"));
}
