/**
 * Web Crypto API E2E Tests
 */

import { TEST_PORT, printSubsection, printTestResult, COLORS, assertEquals } from "./helpers.ts";

export function getCryptoTests() {
  return [
    {
      name: "crypto - Web Crypto API functionality",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("Web Crypto API Test");

        const response = await fetch(`http://localhost:${TEST_PORT}/crypto_e2e.tsp`);
        assertEquals(response.status, 200);

        const result = await response.json();
        if (result.failed !== 0 || result.passed !== result.total) {
          throw new Error(`Crypto tests failed: ${JSON.stringify(result)}`);
        }

        printTestResult(`${result.passed}/${result.total} crypto tests`, true);
        console.log(`  ${COLORS.dim}${Date.now() - startTime}ms${COLORS.reset}`);
      },
    },
  ];
}
