/**
 * Router module unit tests
 * Tests routing in src/router.ts
 */

import { assertEquals } from "./asserts.ts";
import { test } from "bun:test";
import {
  parseFragmentPath,
  resolvePath,
  securityCheck,
} from "../../src/router.ts";

// Use paths relative to project root
const TEST_ROOT = "./www";

test("router - resolvePath: root path", () => {
  const result = resolvePath("/", TEST_ROOT);
  assertEquals(result.success, true);
  // Windows uses \, Unix uses /, so replace before comparing
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/index.tsp");
});

test("router - resolvePath: simple path", () => {
  const result = resolvePath("/form.tsp", TEST_ROOT);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/form.tsp");
});

test("router - resolvePath: path with directory", () => {
  const result = resolvePath("/api/test", TEST_ROOT);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/api/test.tsp");
});

test("router - resolvePath: path with extension", () => {
  const result = resolvePath("/admin/user.tsp", TEST_ROOT);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/admin/user.tsp");
});

test("router - resolvePath: nested directory", () => {
  const result = resolvePath("/api/v1/users/list", TEST_ROOT);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/api/v1/users/list.tsp");
});

// Static file tests
test("router - resolvePath: static CSS file", () => {
  const result = resolvePath("/static/style.css", TEST_ROOT, [".css"]);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/static/style.css");
});

test("router - resolvePath: static JS file", () => {
  const result = resolvePath("/js/app.js", TEST_ROOT, [".js"]);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/js/app.js");
});

test("router - resolvePath: static image file", () => {
  const result = resolvePath("/images/logo.png", TEST_ROOT, [".png"]);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/images/logo.png");
});

test("router - resolvePath: mixed extensions (TSP and static files)", () => {
  // TSP files (default support)
  const tspResult = resolvePath("/page", TEST_ROOT, [".css", ".js"]);
  assertEquals(tspResult.success, true);
  const tspPath = tspResult.filepath!.replace(/\\/g, "/");
  assertEquals(tspPath, "www/page.tsp");

  // Static CSS files
  const cssResult = resolvePath("/style.css", TEST_ROOT, [".css"]);
  assertEquals(cssResult.success, true);
  const cssPath = cssResult.filepath!.replace(/\\/g, "/");
  assertEquals(cssPath, "www/style.css");
});

console.log("\n✓ Router module tests completed");

// Internal file access control tests
test("router - securityCheck: internal files (starting with __) cannot be accessed via HTTP", async () => {
  const testRoot = "./tests/test_www";

  // Note: .tsx files are now intercepted by extension check first, returning "File type not allowed"
  // Only .tsp files are allowed direct access
  const result = await securityCheck(
    `${testRoot}/__internal_component.tsx`,
    testRoot,
  );

  // Now returns "File type not allowed" because .tsx is not in the allowed list
  assertEquals(result.success, false);
  assertEquals(result.error, "File type not allowed");
});

test("router - securityCheck: internal files in nested directories cannot be accessed", async () => {
  const testRoot = "./tests/test_www";

  // Testing files starting with __ in nested directories
  // Note: .tsx files are now intercepted by extension check first
  const result = await securityCheck(
    `${testRoot}/components/__private.tsx`,
    testRoot,
  );

  // Now returns "File type not allowed" because .tsx is not in the allowed list
  assertEquals(result.success, false);
  assertEquals(result.error, "File type not allowed");
});

test("router - resolvePath: internal file paths can still be resolved", () => {
  const testRoot = "./tests/test_www";

  // resolvePath should still be able to resolve internal file paths
  // (because import needs to be able to find these files)
  const result = resolvePath("/__internal_component.tsp", testRoot);

  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "tests/test_www/__internal_component.tsp");
});

// TSP file security tests
test("router - securityCheck: .ts files cannot be accessed via HTTP", async () => {
  const testRoot = "./tests/test_www";

  // Testing .ts files
  const result = await securityCheck(
    `${testRoot}/utils/helpers.ts`,
    testRoot,
  );

  assertEquals(result.success, false);
  assertEquals(result.error, "File type not allowed");
});

test("router - securityCheck: .tsx files cannot be accessed via HTTP", async () => {
  const testRoot = "./tests/test_www";

  // Testing .tsx files
  const result = await securityCheck(
    `${testRoot}/components/Header.tsx`,
    testRoot,
  );

  assertEquals(result.success, false);
  assertEquals(result.error, "File type not allowed");
});

// ============================================
// Fragment routing tests
// ============================================

test("router - parseFragmentPath: simple fragment", () => {
  assertEquals(parseFragmentPath("/users/__fragment/table"), {
    pagePath: "/users",
    fragmentName: "table",
  });
});

test("router - parseFragmentPath: root page fragment", () => {
  assertEquals(parseFragmentPath("/__fragment/root"), {
    pagePath: "/",
    fragmentName: "root",
  });
});

test("router - parseFragmentPath: nested directory page", () => {
  assertEquals(parseFragmentPath("/admin/users/__fragment/row"), {
    pagePath: "/admin/users",
    fragmentName: "row",
  });
});

test("router - parseFragmentPath: non-fragment URL", () => {
  assertEquals(parseFragmentPath("/users"), null);
  assertEquals(parseFragmentPath("/admin/index.tsp"), null);
  assertEquals(parseFragmentPath("/"), null);
});

test("router - parseFragmentPath: empty fragment name", () => {
  assertEquals(parseFragmentPath("/users/__fragment/"), null);
  assertEquals(parseFragmentPath("/__fragment/"), null);
});

test("router - parseFragmentPath: rejects slashes in name", () => {
  assertEquals(parseFragmentPath("/users/__fragment/foo/bar"), null);
});

test("router - parseFragmentPath: rejects names starting with digit/dash", () => {
  assertEquals(parseFragmentPath("/users/__fragment/123"), null);
  assertEquals(parseFragmentPath("/users/__fragment/-foo"), null);
  assertEquals(parseFragmentPath("/users/__fragment/.dot"), null);
});

test("router - parseFragmentPath: accepts underscores and dashes", () => {
  assertEquals(parseFragmentPath("/users/__fragment/user_table"), {
    pagePath: "/users",
    fragmentName: "user_table",
  });
  assertEquals(parseFragmentPath("/users/__fragment/row-1"), {
    pagePath: "/users",
    fragmentName: "row-1",
  });
});

test("router - resolvePath: fragment URL resolves to base page file", () => {
  // The fragment prefix must be stripped before resolvePath is called
  // (router does this in main.ts); this test guards the assumption.
  const result = resolvePath("/users", "./www");
  assertEquals(result.success, true);
  const normalized = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalized, "www/users.tsp");
});
