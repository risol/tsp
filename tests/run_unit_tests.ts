#!/usr/bin/env -S deno run --allow-net --allow-read --allow-run

/**
 * Run unit tests
 */

import { assertEquals } from "https://deno.land/std@0.210.0/testing/asserts.ts";

const UNIT_TEST_FILES = [
  "tests/unit/router_test.ts",
  "tests/unit/context_test.ts",
  "tests/unit/security_test.ts",
  "tests/unit/injection_test.ts",
  "tests/unit/access_log_test.ts",
  "tests/unit/static_test.ts",
];

let totalTests = 0;
let passedTests = 0;
let failedTests = 0;

async function runTestFile(testFile: string): Promise<boolean> {
  console.log(`\n▶ Running: ${testFile}`);
  console.log("─".repeat(50));

  // Switch to project root directory, ensure relative paths are correct
  const command = new Deno.Command("deno", {
    args: [
      "test",
      "--allow-all",
      testFile,
    ],
    cwd: ".", // Explicitly specify working directory as project root
    stdout: "piped",
    stderr: "piped",
  });

  const child = command.spawn();

  try {
    // Get child process exit status
    const status = await child.status;

    // Get output
    let output = "";
    let errorOutput = "";
    try {
      const outputResult = await child.output;
      if (outputResult) {
        output = new TextDecoder().decode(outputResult.stdout);
        errorOutput = new TextDecoder().decode(outputResult.stderr);
      }
    } catch {
      // Output may already be unavailable
    }

    if (output) console.log(output);
    if (errorOutput) console.error(errorOutput);

    const passed = status.code === 0;
    if (passed) {
      console.log(`✓ ${testFile} passed`);
    } else {
      console.log(`✗ ${testFile} failed`);
    }

    return passed;
  } finally {
    // Wait for child process to exit naturally then clean up resources
    // Don't call child.kill(), let test process exit naturally
  }
}

async function main() {
  console.log("╔════════════════════════════════════════════╗");
  console.log("║   TSP Unit Tests                           ║");
  console.log("╚════════════════════════════════════════════╝");

  const startTime = Date.now();
  const results: { file: string; passed: boolean }[] = [];

  for (const testFile of UNIT_TEST_FILES) {
    const passed = await runTestFile(testFile);
    results.push({ file: testFile, passed });

    totalTests++;
    if (passed) {
      passedTests++;
    } else {
      failedTests++;
    }
  }

  const endTime = Date.now();
  const duration = ((endTime - startTime) / 1000).toFixed(2);

  console.log("\n╔════════════════════════════════════════════╗");
  console.log("║   Test Summary                             ║");
  console.log("╚════════════════════════════════════════════╝");
  console.log(`Total: ${totalTests}`);
  console.log(`✓ Passed: ${passedTests}`);
  console.log(`✗ Failed: ${failedTests}`);
  console.log(`⏱ Duration: ${duration} seconds`);

  if (failedTests === 0) {
    console.log("\n🎉 All unit tests passed!");
    Deno.exit(0);
  } else {
    console.log("\n❌ Some tests failed!");
    results.filter((r) => !r.passed).forEach((r) =>
      console.log(`  ✗ ${r.file}`)
    );
    Deno.exit(1);
  }
}

if (import.meta.main) {
  await main();
}
