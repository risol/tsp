#!/usr/bin/env bun

/**
 * Run all tests (unit + E2E)
 */

let totalTests = 0;
let passedTests = 0;
let failedTests = 0;

async function runTestSuite(name: string, args: string[]): Promise<boolean> {
  console.log(`\n▶ Running test suite: ${name}`);
  console.log("═".repeat(50));

  const child = Bun.spawn([process.execPath, ...args], {
    stdio: ["ignore", "pipe", "pipe"],
  });

  const [code, output, errorOutput] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);

  if (output) console.log(output);
  if (errorOutput) console.error(errorOutput);

  const passed = code === 0;
  if (passed) {
    console.log(`✓ ${name} passed`);
  } else {
    console.log(`✗ ${name} failed`);
  }

  return passed;
}

async function main() {
  console.log("╔════════════════════════════════════════════╗");
  console.log("║   TSP Full Test Suite                     ║");
  console.log("╚════════════════════════════════════════════╝");

  const startTime = Date.now();

  // Run unit tests
  const unitPassed = await runTestSuite(
    "Unit Tests",
    ["run", "tests/run_unit_tests.ts"],
  );

  totalTests++;
  if (unitPassed) passedTests++;
  else failedTests++;

  // Run E2E tests
  const e2ePassed = await runTestSuite(
    "E2E Tests",
    ["run", "tests/run_e2e_tests.ts"],
  );

  totalTests++;
  if (e2ePassed) passedTests++;
  else failedTests++;

  const endTime = Date.now();
  const duration = ((endTime - startTime) / 1000).toFixed(2);

  console.log("\n╔════════════════════════════════════════════╗");
  console.log("║   Test Summary                             ║");
  console.log("╚════════════════════════════════════════════╝");
  console.log(`\nTotal test suites: ${totalTests}`);
  console.log(`✓ Passed: ${passedTests}`);
  if (failedTests > 0) {
    console.log(`✗ Failed: ${failedTests}`);
  }
  console.log(`⏱ Total duration: ${duration} seconds`);

  if (failedTests === 0) {
    console.log("\n🎉 All test suites passed!");
    process.exit(0);
  } else {
    console.log("\n❌ Some test suites failed!");
    process.exit(1);
  }
}

if (import.meta.main) {
  await main();
}
