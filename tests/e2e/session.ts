/**
 * Session E2E Tests
 */

import { TEST_PORT, printSubsection, printTestResult, COLORS, assertEquals } from "../run_e2e_tests.ts";

export function getSessionTests() {
  return [
    {
      name: "session - Session functionality in compiled binary",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("Session Functionality Test");

        // Test 1: Access session E2E page, check method availability
        const response = await fetch(
          `http://localhost:${TEST_PORT}/session_e2e.tsp`,
        );

        // Output status code
        console.log(`  ${COLORS.dim}Response status: ${response.status}${COLORS.reset}`);

        // If not 200, session functionality has issues
        if (response.status !== 200) {
          const text = await response.text();
          console.log(`  ${COLORS.red}Error response content:${COLORS.reset}`);
          console.log(`  ${COLORS.dim}${text.substring(0, 500)}${COLORS.reset}`);
          throw new Error("session_e2e.tsp page returned non-200 status");
        }

        // Parse JSON response
        const text = await response.text();
        // Use [\\s\\S]*? to match content including newlines
        const jsonMatch = text.match(/<div>([\s\S]*?)<\/div>/);

        if (!jsonMatch) {
          throw new Error("Cannot extract JSON data from response");
        }

        // HTML entity decode (convert &quot; back to ")
        const jsonString = jsonMatch[1]
          .replace(/&quot;/g, '"')
          .replace(/&amp;/g, "&")
          .replace(/&lt;/g, "<")
          .replace(/&gt;/g, ">");

        const data = JSON.parse(jsonString);

        // Verify session method checks
        console.log(`  ${COLORS.dim}Method checks:${COLORS.reset}`);
        console.log(`    hasSession: ${data.methodChecks.hasSession}`);
        console.log(`    hasInit: ${data.methodChecks.hasInit}`);
        console.log(`    hasDestroy: ${data.methodChecks.hasDestroy}`);
        console.log(`    hasSet: ${data.methodChecks.hasSet}`);
        console.log(`    hasGet: ${data.methodChecks.hasGet}`);
        console.log(`    hasDelete: ${data.methodChecks.hasDelete}`);
        console.log(`    hasClear: ${data.methodChecks.hasClear}`);
        console.log(`    hasAll: ${data.methodChecks.hasAll}`);

        // If any method check fails, session has issues
        if (!data.methodChecks.hasInit || !data.methodChecks.hasSet) {
          throw new Error(
            `Session method unavailable: hasInit=${data.methodChecks.hasInit}, ` +
              `hasSet=${data.methodChecks.hasSet}`,
          );
        }

        printTestResult("Session method existence check", true);

        // Test 2: Initialize session and store data
        const initResponse = await fetch(
          `http://localhost:${TEST_PORT}/session_e2e.tsp`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ action: "init" }),
          },
        );

        assertEquals(initResponse.status, 200);

        const initText = await initResponse.text();
        const initMatch = initText.match(/<div>([\s\S]*?)<\/div>/);
        if (!initMatch) {
          throw new Error("Invalid response format: no JSON found");
        }
        // HTML entity decode
        const initJsonString = initMatch[1]
          .replace(/&quot;/g, '"')
          .replace(/&amp;/g, "&")
          .replace(/&lt;/g, "<")
          .replace(/&gt;/g, ">");
        const initData = JSON.parse(initJsonString);

        if (initData.init.error) {
          throw new Error(`Init call failed: ${initData.init.error}`);
        }

        printTestResult("session.init call", true);

        // Test 3: Verify stored data
        if (
          initData.getData.userId !== "e2e-test-user" ||
          initData.getData.userName !== "E2E Test User"
        ) {
          throw new Error(
            `Incorrect get data: userId=${initData.getData.userId}, userName=${initData.getData.userName}`,
          );
        }

        printTestResult("session.get call", true);

        // Test 4: Test getAll functionality
        if (
          !initData.getAll.result || typeof initData.getAll.result !== "object"
        ) {
          throw new Error(`getAll functionality failed: ${initData.getAll.result}`);
        }

        if (initData.getAll.result.userId !== "e2e-test-user") {
          throw new Error(`getAll returned incorrect data`);
        }

        printTestResult("session.all call", true);

        // Test 5: Test delete functionality
        if (initData.delete.result !== "success") {
          throw new Error(`delete functionality failed: ${initData.delete.result}`);
        }

        printTestResult("session.delete call", true);

        // Test 6: Test clear functionality
        const clearResponse = await fetch(
          `http://localhost:${TEST_PORT}/session_e2e.tsp`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ action: "clear" }),
          },
        );

        assertEquals(clearResponse.status, 200);

        const clearText = await clearResponse.text();
        const clearMatch = clearText.match(/<div>([\s\S]*?)<\/div>/);
        if (!clearMatch) {
          throw new Error("Invalid response format: no JSON found");
        }
        const clearJsonString = clearMatch[1]
          .replace(/&quot;/g, '"')
          .replace(/&amp;/g, "&")
          .replace(/&lt;/g, "<")
          .replace(/&gt;/g, ">");
        const clearData = JSON.parse(clearJsonString);

        if (clearData.clear.result !== "success") {
          throw new Error(`clear functionality failed: ${clearData.clear.result}`);
        }

        printTestResult("session.clear call", true);

        // Test 7: Test destroy functionality
        const destroyResponse = await fetch(
          `http://localhost:${TEST_PORT}/session_e2e.tsp`,
          {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ action: "destroy" }),
          },
        );

        assertEquals(destroyResponse.status, 200);

        const destroyText = await destroyResponse.text();
        const destroyMatch = destroyText.match(/<div>([\s\S]*?)<\/div>/);
        if (!destroyMatch) {
          throw new Error("Invalid response format: no JSON found");
        }
        const destroyJsonString = destroyMatch[1]
          .replace(/&quot;/g, '"')
          .replace(/&amp;/g, "&")
          .replace(/&lt;/g, "<")
          .replace(/&gt;/g, ">");
        const destroyData = JSON.parse(destroyJsonString);

        if (destroyData.destroy.result !== "success") {
          throw new Error(`destroy functionality failed: ${destroyData.destroy.result}`);
        }

        printTestResult("session.destroy call", true);

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
      },
    },
  ];
}
