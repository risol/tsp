/**
 * Validation E2E Tests
 */

import { TEST_PORT, printSubsection, printTestResult, COLORS, assertEquals } from "../run_e2e_tests.ts";

export function getValidationTests() {
  return [
    {
      name: "validation - Zod validator functionality",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("Zod Validator Functionality Test");

        // Test: Zod validation
        const response = await fetch(
          `http://localhost:${TEST_PORT}/validation_e2e.tsp`,
        );

        assertEquals(response.status, 200);

        const result = await response.json();
        if (result.error) {
          throw new Error(`Validation failed: ${result.error}`);
        }

        printTestResult("Zod validator functionality", true);

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
      },
    },
  ];
}
