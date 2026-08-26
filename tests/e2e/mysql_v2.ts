/**
 * MySQL schema-first API E2E Tests
 */

import { TEST_PORT, printSubsection, printTestResult, COLORS, assertEquals, runCommand } from "./helpers.ts";

export function getMysqlV2Tests() {
  return [
    {
      name: "mysql-v2 - schema-first queries and transactions",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("MySQL Schema-first API Test");

        let mysqlRunning = false;
        try {
          const { stdout } = await runCommand("docker", [
            "ps", "--filter", "name=tsp-mysql", "--format", "{{.Status}}",
          ]);
          mysqlRunning = stdout.trim().includes("Up");
        } catch {
          // Handled below so local runs may still skip external services.
        }

        if (!mysqlRunning) {
          if (process.env.CI) {
            throw new Error("MySQL container is required in CI but is not running");
          }
          console.log(`  ${COLORS.yellow}⚠ MySQL container not running, skipping test${COLORS.reset}`);
          return;
        }

        const response = await fetch(`http://localhost:${TEST_PORT}/mysql_v2_e2e.tsp`);
        assertEquals(response.status, 200);

        const result = await response.json();
        if (result.error) {
          throw new Error(`MySQL schema-first tests failed: ${result.error.message || result.error}`);
        }

        const failed = Object.entries(result).filter(([key, value]) => {
          if (!key.startsWith("test")) return false;
          if (typeof value === "string") return value.includes("FAILED");
          return typeof value === "object" && value !== null &&
            "status" in value && String(value.status).includes("FAILED");
        });

        if (failed.length > 0) {
          throw new Error(`MySQL schema-first tests failed: ${JSON.stringify(failed)}`);
        }

        const total = Object.keys(result).filter((key) => key.startsWith("test")).length;
        printTestResult(`${total} schema-first tests`, true);
        console.log(`  ${COLORS.dim}${Date.now() - startTime}ms${COLORS.reset}`);
      },
    },
  ];
}
