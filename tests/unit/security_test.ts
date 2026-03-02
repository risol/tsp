/**
 * Security module unit tests
 * Tests security checks in src/router.ts
 */

import { assertEquals } from "https://deno.land/std@0.210.0/testing/asserts.ts";
import { resolvePath, securityCheck } from "../../src/router.ts";

// Use paths relative to project root
const TEST_ROOT = "./www";

Deno.test("security - securityCheck: path traversal attack protection", async () => {
  const testPaths = [
    "../../../etc/passwd",
    "../secret.tsx",
    "./../../test.tsx",
    "..\\..\\..\\windows\\system32\\config\\sam",
  ];

  for (const path of testPaths) {
    const result = await resolvePath(path, TEST_ROOT);
    if (result.success && result.filepath) {
      const checkResult = await securityCheck(result.filepath, TEST_ROOT);
      assertEquals(checkResult.success, false, `Should block path traversal: ${path}`);
    }
  }
});

Deno.test("security - securityCheck: non-whitelist files", async () => {
  const testPaths = [
    "www/config.json",
    "www/data.txt",
    "www/secret.ts",
    "www/.env",
    "www/package.json",
    // .tsx files cannot be accessed directly either now
    "www/components/Header.tsx",
    "www/utils/helpers.ts",
  ];

  for (const filepath of testPaths) {
    const checkResult = await securityCheck(filepath, TEST_ROOT);
    assertEquals(
      checkResult.success,
      false,
      `Should block non-TSP files: ${filepath}`,
    );
  }
});

Deno.test("security - securityCheck: normal files pass", async () => {
  // Note: Only .tsp files pass security check
  const validPaths = [
    "www/index.tsp",
    "www/form.tsp",
    "www/api.tsp",
  ];

  for (const filepath of validPaths) {
    // Check if file actually exists
    try {
      await Deno.stat(filepath);
    } catch {
      console.log(`Skip non-existent file: ${filepath}`);
      continue;
    }

    const checkResult = await securityCheck(filepath, TEST_ROOT);
    assertEquals(
      checkResult.success,
      true,
      `Should allow normal file: ${filepath} - ${checkResult.error}`,
    );
  }
});

Deno.test("security - securityCheck: relative path traversal", async () => {
  const attackPaths = [
    "./../../../etc/passwd",
    "..././etc/passwd",
    "..\\..\\..\\config.json",
  ];

  for (const path of attackPaths) {
    const result = await resolvePath(path, TEST_ROOT);
    if (result.success && result.filepath) {
      const checkResult = await securityCheck(result.filepath, TEST_ROOT);
      assertEquals(checkResult.success, false, `Should block relative path traversal: ${path}`);
    }
  }
});

Deno.test("security - securityCheck: URL encoding bypass", async () => {
  const encodedPaths = [
    "%2e%2e%2fetc/passwd", // ../etc/passwd
    "..%252f..%252f..%252fetc/passwd", // Double encoding
  ];

  for (const path of encodedPaths) {
    const result = await resolvePath(path, TEST_ROOT);
    if (result.success && result.filepath) {
      const checkResult = await securityCheck(result.filepath, TEST_ROOT);
      // URL encoded paths should be handled securely
      if (checkResult.success) {
        // If passed check, ensure file path is within root directory
        const normalizedPath = result.filepath.replace(/\\/g, "/");
        const isInRoot = normalizedPath.startsWith(TEST_ROOT) ||
          normalizedPath.startsWith("www");
        assertEquals(isInRoot, true, `URL encoded paths should be within root directory: ${path}`);
      }
    }
  }
});

console.log("\n✓ Security module tests completed");
