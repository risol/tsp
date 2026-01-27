#!/usr/bin/env -S deno run --allow-all

/**
 * 运行所有测试
 *
 * 使用方法：
 * deno run --allow-all tests/run_all_tests.ts
 */

import { assertEquals } from "https://deno.land/std@0.210.0/testing/asserts.ts";

const TEST_FILES = [
  "tests/basic_test.ts",
  "tests/binary_build_test.ts",
];

// 测试结果统计
let totalTests = 0;
let passedTests = 0;
let failedTests = 0;

/**
 * 运行单个测试文件
 */
async function runTestFile(testFile: string): Promise<boolean> {
  console.log(`\n▶ 运行测试: ${testFile}`);
  console.log("─".repeat(50));

  const command = new Deno.Command("deno", {
    args: [
      "test",
      "--allow-all",
      testFile,
    ],
    stdout: "piped",
    stderr: "piped",
  });

  const { code, stdout, stderr } = await command.output();

  const output = new TextDecoder().decode(stdout);
  const errorOutput = new TextDecoder().decode(stderr);

  // 输出测试结果
  if (output) {
    console.log(output);
  }
  if (errorOutput) {
    console.error(errorOutput);
  }

  const passed = code === 0;
  if (passed) {
    console.log(`✓ ${testFile} 通过`);
  } else {
    console.log(`✗ ${testFile} 失败 (退出码: ${code})`);
  }

  return passed;
}

/**
 * 主函数
 */
async function main() {
  console.log("╔════════════════════════════════════════════╗");
  console.log("║   TSP-FPM 测试套件                         ║");
  console.log("╚════════════════════════════════════════════╝");

  const startTime = Date.now();
  const results: { file: string; passed: boolean }[] = [];

  for (const testFile of TEST_FILES) {
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

  // 打印总结
  console.log("\n╔════════════════════════════════════════════╗");
  console.log("║   测试总结                                 ║");
  console.log("╚════════════════════════════════════════════╝");
  console.log(`总测试数: ${totalTests}`);
  console.log(`✓ 通过: ${passedTests}`);
  console.log(`✗ 失败: ${failedTests}`);
  console.log(`⏱ 耗时: ${duration} 秒`);

  if (failedTests === 0) {
    console.log("\n🎉 所有测试通过！");
    Deno.exit(0);
  } else {
    console.log("\n❌ 部分测试失败！");
    console.log("\n失败的测试:");
    results
      .filter((r) => !r.passed)
      .forEach((r) => console.log(`  ✗ ${r.file}`));
    Deno.exit(1);
  }
}

// 运行测试
if (import.meta.main) {
  await main();
}
