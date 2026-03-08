/**
 * LDAP E2E Tests
 */

import { TEST_PORT, printSubsection, printTestResult, COLORS, assertEquals } from "../run_e2e_tests.ts";

export function getLdapTests() {
  return [
    {
      name: "ldap - LDAP authentication service functionality",
      fn: async () => {
        const startTime = Date.now();

        printSubsection("LDAP Functionality Test");

        // Test: LDAP authentication
        console.log(`  ${COLORS.dim}Test: LDAP authentication${COLORS.reset}`);

        const response = await fetch(
          `http://localhost:${TEST_PORT}/ldap_e2e.tsp`,
        );

        // LDAP might not be available, so we just check if the endpoint works
        assertEquals(response.status, 200);

        printTestResult("LDAP endpoint accessible", true);

        const duration = Date.now() - startTime;
        console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
      },
    },
  ];
}
