#!/usr/bin/env -S deno run --allow-all

/**
 * 运行所有测试（单元测试 + E2E测试）
 */

import { assertEquals } from "https://deno.land/std@0.210.0/testing/asserts.ts";

let totalTests = 0;
let passedTests = 0;
let failedTests = 0;

async function runTestSuite(name: string, command: string): Promise<boolean> {
  console.log(`\n▶ 运行测试套件: ${name}`);
  console.log("═".repeat(50));

  const parts = command.split(" ");
  const denoCommand = new Deno.Command(parts[0], {
    args: parts.slice(1),
    stdout: "piped",
    stderr: "piped",
  });

  const { code, stdout, stderr } = await denoCommand.output();

  const output = new TextDecoder().decode(stdout);
  const errorOutput = new TextDecoder().decode(stderr);

  if (output) console.log(output);
  if (errorOutput) console.error(errorOutput);

  const passed = code === 0;
  if (passed) {
    console.log(`✓ ${name} 通过`);
  } else {
    console.log(`✗ ${name} 失败`);
  }

  return passed;
}

async function main() {
  console.log("╔════════════════════════════════════════════╗");
  console.log("║   TSP-FPM 完整测试套件                    ║");
  console.log("╚════════════════════════════════════════════╝");

  const startTime = Date.now();

  // 运行单元测试
  const unitPassed = await runTestSuite(
    "单元测试",
    "deno run --allow-all tests/run_unit_tests.ts"
  );

  totalTests++;
  if (unitPassed) passedTests++; else failedTests++;

  // 运行E2E测试
  const e2ePassed = await runTestSuite(
    "E2E测试",
    "deno run --allow-all tests/run_e2e_tests.ts"
  );

  totalTests++;
  if (e2ePassed) passedTests++; else failedTests++;

  const endTime = Date.now();
  const duration = ((endTime - startTime) / 1000).toFixed(2);

  console.log("\n╔════════════════════════════════════════════╗");
  console.log("║   测试总结                                 ║");
  console.log("╚════════════════════════════════════════════╝");
  console.log(`总测试套件: ${totalTests}`);
  console.log(`✓ 通过: ${passedTests}`);
  console.log(`✗ 失败: ${failedTests}`);
  console.log(`⏱ 总耗时: ${duration} 秒`);

  if (failedTests === 0) {
    console.log("\n🎉 所有测试套件通过！");
    Deno.exit(0);
  } else {
    console.log("\n❌ 部分测试套件失败！");
    Deno.exit(1);
  }
}

if (import.meta.main) {
  await main();
}
