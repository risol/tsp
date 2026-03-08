/**
 * ExcelJS E2E Tests
 */

import { TEST_PORT, printSubsection, printTestResult, COLORS, assertEquals } from "../run_e2e_tests.ts";

export function getExcelJsTests() {
  return [
    {
      name: "exceljs - Excel file operation functionality",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("ExcelJS Functionality Test");

        // Test: Create and download Excel file
        console.log(`  ${COLORS.dim}Test: Create Excel file${COLORS.reset}`);

        const response = await fetch(
          `http://localhost:${TEST_PORT}/exceljs_e2e.tsp`,
        );

        assertEquals(response.status, 200);

        const result = await response.json();
        if (result.error) {
          throw new Error(`Excel operation failed: ${result.error}`);
        }

        printTestResult("Excel file creation", true);

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
      },
    },
  ];
}
