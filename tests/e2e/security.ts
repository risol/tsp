/**
 * Security E2E Tests
 */

import { testHttpRequest, TEST_PORT, printSubsection, printTestResult, COLORS } from "../run_e2e_tests.ts";

export function getSecurityTests() {
  return [
    {
      name: "security - Path traversal protection",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("Security Test");

        // Test 1: Path traversal attempts for non-existent files return 404
        await testHttpRequest(
          `http://localhost:${TEST_PORT}/../config.json`,
          404,  // Non-existent file returns 404
        );
        printTestResult(`Path traversal returns 404 for non-existent files`, true);

        // Test 2: Path traversal with encoded characters (non-existent file)
        await testHttpRequest(
          `http://localhost:${TEST_PORT}/..%2F..%2Fconfig.json`,
          404,
        );
        printTestResult(`Encoded path traversal returns 404`, true);

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
      },
    },
    {
      name: "security - Internal files (starting with __) cannot be accessed directly",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("Internal File Access Control Test");

        // Test 1: Direct access to internal file (whether it exists or not) returns 403
        // The internal file check happens before file existence check
        await testHttpRequest(
          `http://localhost:${TEST_PORT}/__test_utils.ts`,
          403,  // Internal file check happens before file existence check
        );
        printTestResult("Internal files are blocked (403)", true);

        // Test 2: Access existing internal .tsp file - should also be blocked with 403
        await testHttpRequest(
          `http://localhost:${TEST_PORT}/__internal_component.tsp`,
          403,  // Internal file that exists should be blocked
        );
        printTestResult("Existing internal files are blocked (403)", true);

        // Test 3: Access a normal page should work
        await testHttpRequest(
          `http://localhost:${TEST_PORT}/index.tsp`,
          200,
        );
        printTestResult("Can normally access regular pages", true);

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
      },
    },
  ];
}
