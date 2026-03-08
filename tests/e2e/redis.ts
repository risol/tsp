/**
 * Redis E2E Tests
 */

import { TEST_PORT, printSubsection, printTestResult, COLORS, assertEquals } from "../run_e2e_tests.ts";

export function getRedisTests() {
  return [
    {
      name: "redis - Redis cache functionality",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("Redis Functionality Test");

        // Check if Redis container is running
        console.log(`  ${COLORS.dim}Checking Redis container status...${COLORS.reset}`);

        let redisRunning = false;
        try {
          const checkCommand = new Deno.Command("docker", {
            args: ["ps", "--filter", "name=tsp-redis", "--format", "{{.Status}}"],
            stdout: "piped",
            stderr: "piped",
          });

          const { stdout } = await checkCommand.output();
          const status = new TextDecoder().decode(stdout).trim();

          if (status.includes("Up")) {
            redisRunning = true;
            console.log(`  ${COLORS.green}✓ Redis container is running${COLORS.reset}`);
          }
        } catch (error) {
          console.log(
            `  ${COLORS.yellow}⚠ Cannot check Redis status, skipping test${COLORS.reset}`,
          );
          console.log(`  ${COLORS.dim}Error: ${(error as Error).message}${COLORS.reset}`);
        }

        if (!redisRunning) {
          console.log(
            `  ${COLORS.yellow}⚠ Redis container not running, skipping test${COLORS.reset}`,
          );
          return;
        }

        // Test 1: Basic operation
        console.log(`  ${COLORS.dim}Test 1: Basic operation (set/get/del)${COLORS.reset}`);

        const response = await fetch(
          `http://localhost:${TEST_PORT}/redis_e2e.tsp?action=basic`,
        );

        assertEquals(response.status, 200);

        const result = await response.json();
        if (result.error) {
          throw new Error(`Redis basic test failed: ${result.error}`);
        }
        if (!result.success) {
          throw new Error(`Redis test failed: ${result.error || "unknown error"}`);
        }
        printTestResult("Basic operation", true);

        // Test 2: List operations
        console.log(`  ${COLORS.dim}Test 2: List operations${COLORS.reset}`);

        const listResponse = await fetch(
          `http://localhost:${TEST_PORT}/redis_e2e.tsp?action=list`,
        );

        assertEquals(listResponse.status, 200);

        const listResult = await listResponse.json();
        if (listResult.error) {
          throw new Error(`Redis list test failed: ${listResult.error}`);
        }
        printTestResult("List operations", true);

        // Test 3: Hash operations
        console.log(`  ${COLORS.dim}Test 3: Hash operations${COLORS.reset}`);

        const hashResponse = await fetch(
          `http://localhost:${TEST_PORT}/redis_e2e.tsp?action=hash`,
        );

        assertEquals(hashResponse.status, 200);

        const hashResult = await hashResponse.json();
        if (hashResult.error) {
          throw new Error(`Redis hash test failed: ${hashResult.error}`);
        }
        printTestResult("Hash operations", true);

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
      },
    },
  ];
}
