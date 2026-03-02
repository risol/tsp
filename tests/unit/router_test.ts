/**
 * Router module unit tests
 * Tests routing in src/router.ts
 */

import { assertEquals } from "https://deno.land/std@0.210.0/testing/asserts.ts";
import { resolvePath, securityCheck } from "../../src/router.ts";

// Use paths relative to project root
const TEST_ROOT = "./www";

Deno.test("router - resolvePath: root path", () => {
  const result = resolvePath("/", TEST_ROOT);
  assertEquals(result.success, true);
  // Windows uses \, Unix uses /, so replace before comparing
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/index.tsp");
});

Deno.test("router - resolvePath: simple path", () => {
  const result = resolvePath("/form.tsp", TEST_ROOT);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/form.tsp");
});

Deno.test("router - resolvePath: path with directory", () => {
  const result = resolvePath("/api/test", TEST_ROOT);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/api/test.tsp");
});

Deno.test("router - resolvePath: path with extension", () => {
  const result = resolvePath("/admin/user.tsp", TEST_ROOT);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/admin/user.tsp");
});

Deno.test("router - resolvePath: nested directory", () => {
  const result = resolvePath("/api/v1/users/list", TEST_ROOT);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/api/v1/users/list.tsp");
});

// Static file tests
Deno.test("router - resolvePath: static CSS file", () => {
  const result = resolvePath("/static/style.css", TEST_ROOT, [".css"]);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/static/style.css");
});

Deno.test("router - resolvePath: static JS file", () => {
  const result = resolvePath("/js/app.js", TEST_ROOT, [".js"]);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/js/app.js");
});

Deno.test("router - resolvePath: static image file", () => {
  const result = resolvePath("/images/logo.png", TEST_ROOT, [".png"]);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/images/logo.png");
});

Deno.test("router - resolvePath: mixed extensions (TSP and static files)", () => {
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
Deno.test("router - securityCheck: internal files (starting with __) cannot be accessed via HTTP", async () => {
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

Deno.test("router - securityCheck: internal files in nested directories cannot be accessed", async () => {
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

Deno.test("router - resolvePath: internal file paths can still be resolved", () => {
  const testRoot = "./tests/test_www";

  // resolvePath should still be able to resolve internal file paths
  // (because import needs to be able to find these files)
  const result = resolvePath("/__internal_component.tsp", testRoot);

  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "tests/test_www/__internal_component.tsp");
});

// TSP file security tests
Deno.test("router - securityCheck: .ts files cannot be accessed via HTTP", async () => {
  const testRoot = "./tests/test_www";

  // Testing .ts files
  const result = await securityCheck(
    `${testRoot}/utils/helpers.ts`,
    testRoot,
  );

  assertEquals(result.success, false);
  assertEquals(result.error, "File type not allowed");
});

Deno.test("router - securityCheck: .tsx files cannot be accessed via HTTP", async () => {
  const testRoot = "./tests/test_www";

  // Testing .tsx files
  const result = await securityCheck(
    `${testRoot}/components/Header.tsx`,
    testRoot,
  );

  assertEquals(result.success, false);
  assertEquals(result.error, "File type not allowed");
});
