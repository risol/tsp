/**
 * bcryptjs E2E Tests
 */

import { TEST_PORT, printSubsection, printTestResult, COLORS, assertEquals } from "./helpers.ts";

export function getBcryptTests() {
  return [
    {
      name: "bcryptjs - password hashing functionality",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("bcryptjs Test");

        const response = await fetch(`http://localhost:${TEST_PORT}/bcrypt_e2e.tsp`);
        assertEquals(response.status, 200);

        const result = await response.json();
        if (result.failed !== 0 || result.passed !== result.total) {
          throw new Error(`bcryptjs tests failed: ${JSON.stringify(result)}`);
        }

        printTestResult(`${result.passed}/${result.total} bcryptjs tests`, true);
        console.log(`  ${COLORS.dim}${Date.now() - startTime}ms${COLORS.reset}`);
      },
    },
  ];
}
