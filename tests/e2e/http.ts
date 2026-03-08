/**
 * HTTP E2E Tests
 */

import { testHttpRequest, TEST_PORT, printSubsection, printTestResult, COLORS } from "../run_e2e_tests.ts";

export function getHttpTests() {
  return [
    {
      name: "http - Basic HTTP functionality",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("Basic HTTP Test");

        await testHttpRequest(`http://localhost:${TEST_PORT}/`, 200, {
          expectHtml: true,
        });
        printTestResult("Root path /", true);

        await testHttpRequest(`http://localhost:${TEST_PORT}/index.tsp`, 200, {
          expectHtml: true,
        });
        printTestResult("index.tsp", true);

        await testHttpRequest(`http://localhost:${TEST_PORT}/form.tsp`, 200, {
          expectHtml: true,
        });
        printTestResult("form.tsp", true);

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
      },
    },
    {
      name: "http - API test",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("API Test");

        await testHttpRequest(`http://localhost:${TEST_PORT}/api.tsp`, 200, {
          expectHtml: true,
        });
        printTestResult("api.tsp", true);

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms}${COLORS.reset}`);
      },
    },
    {
      name: "http - Error handling",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("Error Handling Test");

        await testHttpRequest(`http://localhost:${TEST_PORT}/error.tsp`, 500);
        printTestResult("500 Server Error", true);

        await testHttpRequest(
          `http://localhost:${TEST_PORT}/nonexistent.tsp`,
          404,
        );
        printTestResult("404 Not Found", true);

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
      },
    },
  ];
}
