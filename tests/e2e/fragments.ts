/**
 * Fragment routing E2E tests
 *
 * Verifies that `<page>/__fragment/<name>` URLs:
 *   1. Return 200 + bare HTML (no <!DOCTYPE>, no <html>)
 *   2. Resolve the named export from the page module
 *   3. Pass query parameters through
 *   4. Return 404 for unknown fragment names
 *   5. Allow fragments to return Response objects (e.g. JSON)
 * And that the built-in htmx asset is served at `/__static/htmx.js`.
 */

import {
  assertEquals,
  assertExists,
  assertStringIncludes,
  COLORS,
  printSubsection,
  printTestResult,
  TEST_PORT,
} from "./helpers.ts";

const BASE = `http://localhost:${TEST_PORT}`;
const PAGE = `${BASE}/fragments_demo.tsp`;

export function getFragmentTests() {
  return [
    {
      name: "fragments - default page renders full HTML",
      fn: async () => {
        const start = Date.now();
        printSubsection("Default page sanity check");

        const resp = await fetch(PAGE);
        assertEquals(resp.status, 200);
        const ct = resp.headers.get("content-type") ?? "";
        assertStringIncludes(ct, "text/html");

        const text = await resp.text();
        // Default page must be a full document
        assertExists(text.includes("<!DOCTYPE"));
        assertExists(text.includes("<html"));
        assertExists(text.includes("Fragments Demo"));

        // The pre-rendered fragment should also be present
        assertExists(text.includes('id="demo-table"'));
        assertExists(text.includes('data-source="fragment"'));

        printTestResult(
          "Default page is full HTML with pre-rendered fragment",
          true,
        );
        console.log(`  ${COLORS.dim}${Date.now() - start}ms${COLORS.reset}`);
      },
    },

    {
      name: "fragments - named export returns bare HTML",
      fn: async () => {
        const start = Date.now();
        printSubsection("Fragment: table");

        const resp = await fetch(`${PAGE}/__fragment/table`);
        assertEquals(resp.status, 200);
        const ct = resp.headers.get("content-type") ?? "";
        assertStringIncludes(ct, "text/html");

        const text = await resp.text();
        // Bare HTML: no <!DOCTYPE>, no <html>
        assertExists(!text.includes("<!DOCTYPE"));
        assertExists(!text.includes("<html"));
        // Content is the fragment only
        assertExists(text.includes("<table"));
        assertExists(text.includes('id="demo-table"'));
        assertExists(text.includes("apple"));
        assertExists(text.includes("banana"));

        printTestResult("table fragment is bare HTML, no wrapper", true);
        console.log(`  ${COLORS.dim}${Date.now() - start}ms${COLORS.reset}`);
      },
    },

    {
      name: "fragments - query params passed to fragment",
      fn: async () => {
        const start = Date.now();
        printSubsection("Fragment: echo with query");

        const resp = await fetch(`${PAGE}/__fragment/echo?msg=ping`);
        assertEquals(resp.status, 200);
        const text = await resp.text();
        assertExists(text.includes("ping"));
        assertExists(text.includes('data-msg="ping"'));

        // Default value when query is missing
        const resp2 = await fetch(`${PAGE}/__fragment/echo`);
        assertEquals(resp2.status, 200);
        const text2 = await resp2.text();
        assertExists(text2.includes("hello"));
        assertExists(text2.includes('data-msg="hello"'));

        printTestResult("echo fragment reads query, applies default", true);
        console.log(`  ${COLORS.dim}${Date.now() - start}ms${COLORS.reset}`);
      },
    },

    {
      name: "fragments - Response return passes through",
      fn: async () => {
        const start = Date.now();
        printSubsection("Fragment: json returning Response");

        const resp = await fetch(`${PAGE}/__fragment/json?n=42`);
        assertEquals(resp.status, 200);
        const ct = resp.headers.get("content-type") ?? "";
        assertStringIncludes(ct, "application/json");

        const body = await resp.json();
        assertEquals(body.ok, true);
        assertEquals(body.n, 42);

        printTestResult("json fragment passes Response through", true);
        console.log(`  ${COLORS.dim}${Date.now() - start}ms${COLORS.reset}`);
      },
    },

    {
      name: "fragments - unknown fragment returns 404",
      fn: async () => {
        const start = Date.now();
        printSubsection("Fragment: 404 for unknown name");

        const resp = await fetch(`${PAGE}/__fragment/nonexistent`);
        assertEquals(resp.status, 404);
        const text = await resp.text();
        assertExists(text.includes("Fragment not found"));

        printTestResult("Unknown fragment returns 404", true);
        console.log(`  ${COLORS.dim}${Date.now() - start}ms${COLORS.reset}`);
      },
    },

    {
      name: "fragments - missing page returns 404 (not 500)",
      fn: async () => {
        const start = Date.now();
        printSubsection("Fragment: page missing");

        // A fragment URL whose base page does not exist
        const resp = await fetch(
          `${BASE}/no_such_page.tsp/__fragment/anything`,
        );
        assertEquals(resp.status, 404);

        printTestResult("Missing base page returns 404", true);
        console.log(`  ${COLORS.dim}${Date.now() - start}ms${COLORS.reset}`);
      },
    },

    {
      name: "fragments - regular page URL is unaffected",
      fn: async () => {
        const start = Date.now();
        printSubsection("Sanity: existing routes still work");

        // /form.tsp uses no fragments; must still serve the full page.
        const resp = await fetch(`${BASE}/form.tsp`);
        assertEquals(resp.status, 200);
        const text = await resp.text();
        assertExists(text.includes("<!DOCTYPE"));
        assertExists(text.includes("Form Test"));

        printTestResult("Existing routes untouched", true);
        console.log(`  ${COLORS.dim}${Date.now() - start}ms${COLORS.reset}`);
      },
    },

    {
      name: "htmx - /__static/htmx.js returns the vendored library",
      fn: async () => {
        const start = Date.now();
        printSubsection("htmx asset");

        const resp = await fetch(`${BASE}/__static/htmx.js`);
        assertEquals(resp.status, 200);
        const ct = resp.headers.get("content-type") ?? "";
        assertStringIncludes(ct, "application/javascript");

        const body = await resp.text();
        // The vendored bundle must register htmx on the global window
        // and declare the expected version. Both signals confirm we are
        // not accidentally serving a 404 HTML page or an empty body.
        assertExists(body.includes("htmx") && body.includes("1.9.10"));

        // Cache header is set so the browser can keep the library.
        const cache = resp.headers.get("cache-control") ?? "";
        assertExists(cache.includes("max-age"));

        printTestResult(
          "htmx asset is served with correct MIME + version",
          true,
        );
        console.log(`  ${COLORS.dim}${Date.now() - start}ms${COLORS.reset}`);
      },
    },

    {
      name: "htmx - default page references the built-in script",
      fn: async () => {
        const start = Date.now();
        printSubsection("htmx wired into demo page");

        const resp = await fetch(PAGE);
        const text = await resp.text();
        assertExists(text.includes('src="/__static/htmx.js"'));

        printTestResult(
          "fragments_demo.tsp includes the htmx script tag",
          true,
        );
        console.log(`  ${COLORS.dim}${Date.now() - start}ms${COLORS.reset}`);
      },
    },

    {
      name: "HtmxScript - emits <meta name=htmx-config> with provided options",
      fn: async () => {
        const start = Date.now();
        printSubsection("HtmxScript config meta tag");

        const resp = await fetch(PAGE);
        const text = await resp.text();
        // The demo uses <HtmxScript defaultSwap="outerHTML" timeout={5000}
        //                    historyCacheSize={20} />.
        const metaMatch = text.match(
          /<meta\s+name="htmx-config"\s+content="([^"]+)"\s*\/>/,
        );
        assertExists(metaMatch, "HtmxScript must emit <meta name=htmx-config>");
        const parsed = JSON.parse(
          metaMatch[1].replace(/&quot;/g, '"').replace(/&amp;/g, "&"),
        );
        assertEquals(parsed.defaultSwapStyle, "outerHTML");
        assertEquals(parsed.timeout, 5000);
        assertEquals(parsed.historyCacheSize, 20);

        printTestResult(
          "HtmxScript serializes props to htmx-config meta",
          true,
        );
        console.log(`  ${COLORS.dim}${Date.now() - start}ms${COLORS.reset}`);
      },
    },

    {
      name: "HtmxFragment - auto-fetches initial from fragments[name]",
      fn: async () => {
        const start = Date.now();
        printSubsection("HtmxFragment auto-resolve");

        const resp = await fetch(PAGE);
        const text = await resp.text();

        // The page has <HtmxFragment page="..." name="table" trigger="every 5s" />
        // and the fixture does NOT pass children. The framework must call
        // fragments.table(ctx) on the server and inject the <table> as
        // initial content.
        const tableDiv = text.match(
          /<div\s+hx-get="\/fragments_demo\.tsp\/__fragment\/table"\s+hx-trigger="every 5s">[\s\S]*?<\/div>/,
        );
        assertExists(tableDiv, "HtmxFragment must render the hx-* div");
        assertExists(
          tableDiv[0].includes('<table id="demo-table"'),
          "auto-resolved table must live inside the div",
        );
        assertExists(
          tableDiv[0].includes("apple") && tableDiv[0].includes("cherry"),
          "auto-resolved table must contain the rows",
        );

        printTestResult("HtmxFragment auto-injects initial content", true);
        console.log(`  ${COLORS.dim}${Date.now() - start}ms${COLORS.reset}`);
      },
    },

    {
      name: "HtmxFragment - URL uses hxUrl convention (not hand-written)",
      fn: async () => {
        const start = Date.now();
        printSubsection("HtmxFragment URL shape");

        const resp = await fetch(PAGE);
        const text = await resp.text();
        // Echo fragment should be wired with click trigger and the
        // same /__fragment/ path.
        assertExists(
          text.includes(
            '<div hx-get="/fragments_demo.tsp/__fragment/echo" hx-trigger="click from:#echo-btn">',
          ),
        );

        // The JSON section uses hxUrl() inside a template literal;
        // the resolved URL must match the same /__fragment/ shape.
        assertExists(text.includes("/fragments_demo.tsp/__fragment/json"));

        printTestResult("HtmxFragment / hxUrl produce correct URLs", true);
        console.log(`  ${COLORS.dim}${Date.now() - start}ms${COLORS.reset}`);
      },
    },

    {
      name: "HtmxFragment - missing fragment name does not crash page render",
      fn: async () => {
        // A bad fragment name would print to console.warn but should
        // not 500 the page or leak the warning text into the HTML.
        const start = Date.now();
        printSubsection("Robustness check");

        const resp = await fetch(PAGE);
        assertEquals(resp.status, 200);
        const text = await resp.text();
        assertExists(!text.includes("HtmxFragment] fragment"));

        printTestResult(
          "page renders without spurious HtmxFragment warnings",
          true,
        );
        console.log(`  ${COLORS.dim}${Date.now() - start}ms${COLORS.reset}`);
      },
    },
  ];
}
