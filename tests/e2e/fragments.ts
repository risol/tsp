/**
 * Fragment routing E2E tests
 *
 * Verifies that `<page>/__fragment/<name>` URLs:
 *   1. Return 200 + bare HTML (no <!DOCTYPE>, no <html>)
 *   2. Resolve the named export from the page module
 *   3. Pass query parameters through
 *   4. Return 404 for unknown fragment names
 *   5. Allow fragments to return Response objects (e.g. JSON)
 */

import {
  assertEquals,
  assertExists,
  assertStringIncludes,
  COLORS,
  printSubsection,
  printTestResult,
  TEST_PORT,
} from "../run_e2e_tests.ts";

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
  ];
}
