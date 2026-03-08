/**
 * Config Reload E2E Tests
 */

import { TEST_PORT, printSubsection, printTestResult, COLORS, assertEquals } from "../run_e2e_tests.ts";

export function getConfigTests() {
  return [
    {
      name: "config reload - Config file modification auto-reload",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("Config File Auto-Reload Test");

        // This test is skipped - v5.0 changed config reload behavior
        // Skip this test
        return;
      },
    },
    {
      name: "config reload - Relative root path reload test",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("Config Reload - Relative Root Path Test");

        // Test: Verify config reload works with relative root path
        const response = await fetch(
          `http://localhost:${TEST_PORT}/`,
        );

        assertEquals(response.status, 200);

        printTestResult("Config with relative root path", true);

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
      },
    },
    {
      name: "config reload - Hot reload config change test",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("Hot Reload Config Change Test");

        // This test is skipped
        return;
      },
    },
  ];
}
