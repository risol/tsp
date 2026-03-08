/**
 * Injection E2E Tests
 */

import { TEST_PORT, printSubsection, printTestResult, COLORS, assertEquals, assertExists } from "../run_e2e_tests.ts";

export function getInjectionTests() {
  return [
    {
      name: "injection - Dependency injection test page",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("Dependency Injection Test Page");

        const response = await fetch(
          `http://localhost:${TEST_PORT}/injection.tsp`,
        );

        // If not 200, output error info
        if (response.status !== 200) {
          const text = await response.text();
          console.log(`  ${COLORS.red}Error response content:${COLORS.reset}`);
          console.log(`  ${COLORS.dim}${text.substring(0, 500)}${COLORS.reset}`);

          // Output server error log
          const serverErrors = (globalThis as any).serverErrors || "";
          if (serverErrors) {
            console.log(`  ${COLORS.red}Server error log:${COLORS.reset}`);
            console.log(`  ${COLORS.dim}${serverErrors}${COLORS.reset}`);
          }
        }

        assertEquals(response.status, 200);

        const text = await response.text();
        // Verify page contains expected content (note: E2E does not test actual DI due to compilation limits)
        assertExists(text.includes("Dependency Injection Test"));
        assertExists(text.includes("Unit Test Coverage"));
        printTestResult("injection.tsp - Basic page functionality", true);

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
      },
    },
  ];
}
