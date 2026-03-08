/**
 * MySQL E2E Tests
 */

import { TEST_PORT, printSubsection, printTestResult, COLORS, assertEquals } from "../run_e2e_tests.ts";

export function getMysqlTests() {
  return [
    {
      name: "mysql - MySQL database functionality",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("MySQL Functionality Test");

        // Check if MySQL container is running
        console.log(`  ${COLORS.dim}Checking MySQL container status...${COLORS.reset}`);

        let mysqlRunning = false;
        try {
          const checkCommand = new Deno.Command("docker", {
            args: ["ps", "--filter", "name=tsp-mysql", "--format", "{{.Status}}"],
            stdout: "piped",
            stderr: "piped",
          });

          const { stdout } = await checkCommand.output();
          const status = new TextDecoder().decode(stdout).trim();

          if (status.includes("Up")) {
            mysqlRunning = true;
            console.log(`  ${COLORS.green}✓ MySQL container is running${COLORS.reset}`);
          }
        } catch (error) {
          console.log(
            `  ${COLORS.yellow}⚠ Cannot check MySQL status, skipping test${COLORS.reset}`,
          );
          console.log(`  ${COLORS.dim}Error: ${(error as Error).message}${COLORS.reset}`);
        }

        if (!mysqlRunning) {
          console.log(
            `  ${COLORS.yellow}⚠ MySQL container not running, skipping test${COLORS.reset}`,
          );
          return;
        }

        // Test 1: Basic query
        console.log(`  ${COLORS.dim}Test 1: Basic query${COLORS.reset}`);

        const queryResponse = await fetch(
          `http://localhost:${TEST_PORT}/mysql_e2e.tsp?action=query`,
        );

        assertEquals(queryResponse.status, 200);

        const queryResult = await queryResponse.json();
        if (queryResult.error) {
          throw new Error(`Query failed: ${queryResult.error}`);
        }
        console.log(
          `  ${COLORS.dim}Query result: ${queryResult.count} users${COLORS.reset}`,
        );
        printTestResult("Basic query", true);

        // Test 2: Parameterized query
        console.log(`  ${COLORS.dim}Test 2: Parameterized query${COLORS.reset}`);

        const paramQueryResponse = await fetch(
          `http://localhost:${TEST_PORT}/mysql_e2e.tsp?action=param-query&id=1`,
        );

        assertEquals(paramQueryResponse.status, 200);

        const paramQueryResult = await paramQueryResponse.json();
        if (paramQueryResult.error) {
          throw new Error(`Parameterized query failed: ${paramQueryResult.error}`);
        }
        console.log(
          `  ${COLORS.dim}Parameterized query result: ${paramQueryResult.user.length} users${COLORS.reset}`,
        );
        printTestResult("Parameterized query", true);

        // Test 3: Insert data
        console.log(`  ${COLORS.dim}Test 3: Insert data${COLORS.reset}`);

        const insertResponse = await fetch(
          `http://localhost:${TEST_PORT}/mysql_e2e.tsp?action=insert&name=TestUser&email=test@example.com`,
        );

        assertEquals(insertResponse.status, 200);

        const insertResult = await insertResponse.json();
        if (insertResult.error) {
          throw new Error(`Insert failed: ${insertResult.error}`);
        }
        console.log(
          `  ${COLORS.dim}Inserted record ID: ${insertResult.insertId}${COLORS.reset}`,
        );
        printTestResult("Insert data", true);

        // Test 4: Update data
        console.log(`  ${COLORS.dim}Test 4: Update data${COLORS.reset}`);

        const updateResponse = await fetch(
          `http://localhost:${TEST_PORT}/mysql_e2e.tsp?action=update&id=1&name=UpdatedUser`,
        );

        assertEquals(updateResponse.status, 200);

        const updateResult = await updateResponse.json();
        if (updateResult.error) {
          throw new Error(`Update failed: ${updateResult.error}`);
        }
        printTestResult("Update data", true);

        // Test 5: Delete data
        console.log(`  ${COLORS.dim}Test 5: Delete data${COLORS.reset}`);

        const deleteResponse = await fetch(
          `http://localhost:${TEST_PORT}/mysql_e2e.tsp?action=delete&id=1`,
        );

        assertEquals(deleteResponse.status, 200);

        const deleteResult = await deleteResponse.json();
        if (deleteResult.error) {
          throw new Error(`Delete failed: ${deleteResult.error}`);
        }
        printTestResult("Delete data", true);

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
      },
    },
  ];
}
