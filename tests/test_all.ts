#!/usr/bin/env -S deno run --allow-net --allow-read --allow-run

/**
 * 测试运行器
 * 运行所有端到端测试
 */

const testFiles = [
  "tests/e2e/basic_test.ts",
  "tests/e2e/routing_test.ts",
  "tests/e2e/redirect_test.ts",
  "tests/e2e/error_test.ts",
  "tests/e2e/custom_response_test.ts",
];

console.log(`
╔════════════════════════════════════════╗
║       TSP-FPM E2E Test Runner         ║
╚════════════════════════════════════════╝
`);

let passedCount = 0;
let failedCount = 0;

for (const testFile of testFiles) {
  console.log(`\n🧪 Running: ${testFile}`);
  console.log("─".repeat(60));

  const process = new Deno.Command("deno", {
    args: [
      "test",
      "--allow-net",
      "--allow-read",
      "--allow-run",
      testFile,
    ],
    stdout: "piped",
    stderr: "piped",
  });

  const { code, stdout, stderr } = await process.output();

  const output = new TextDecoder().decode(stdout);
  const errorOutput = new TextDecoder().decode(stderr);

  if (output) {
    console.log(output);
  }

  if (errorOutput) {
    console.error(errorOutput);
  }

  if (code === 0) {
    passedCount++;
    console.log(`✅ ${testFile} - PASSED`);
  } else {
    failedCount++;
    console.log(`❌ ${testFile} - FAILED`);
  }
}

console.log(`
╔════════════════════════════════════════╗
║           Test Summary                ║
╠════════════════════════════════════════╣
║  Passed: ${passedCount.toString().padStart(2)}                              ║
║  Failed: ${failedCount.toString().padStart(2)}                              ║
╚════════════════════════════════════════╝
`);

Deno.exit(failedCount > 0 ? 1 : 0);
