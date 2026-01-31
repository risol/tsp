#!/usr/bin/env -S deno run --allow-net --allow-read --allow-run

/**
 * 运行单元测试
 */

import { assertEquals } from "https://deno.land/std@0.210.0/testing/asserts.ts";

const UNIT_TEST_FILES = [
  "tests/unit/router_test.ts",
  "tests/unit/context_test.ts",
  "tests/unit/security_test.ts",
  "tests/unit/injection_test.ts",
  "tests/unit/access_log_test.ts",
  "tests/unit/static_test.ts",
  "tests/unit/cache_reverse_deps_test.ts",
];

let totalTests = 0;
let passedTests = 0;
let failedTests = 0;

async function runTestFile(testFile: string): Promise<boolean> {
  console.log(`\n▶ 运行: ${testFile}`);
  console.log("─".repeat(50));

  // 切换到项目根目录，确保相对路径正确
  const command = new Deno.Command("deno", {
    args: [
      "test",
      "--allow-all",
      testFile,
    ],
    cwd: ".", // 明确指定工作目录为项目根目录
    stdout: "piped",
    stderr: "piped",
  });

  const { code, stdout, stderr } = await command.output();

  const output = new TextDecoder().decode(stdout);
  const errorOutput = new TextDecoder().decode(stderr);

  if (output) console.log(output);
  if (errorOutput) console.error(errorOutput);

  const passed = code === 0;
  if (passed) {
    console.log(`✓ ${testFile} 通过`);
  } else {
    console.log(`✗ ${testFile} 失败`);
  }

  return passed;
}

async function main() {
  console.log("╔════════════════════════════════════════════╗");
  console.log("║   TSP 单元测试                              ║");
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
  console.log("║   测试总结                                 ║");
  console.log("╚════════════════════════════════════════════╝");
  console.log(`总测试数: ${totalTests}`);
  console.log(`✓ 通过: ${passedTests}`);
  console.log(`✗ 失败: ${failedTests}`);
  console.log(`⏱ 耗时: ${duration} 秒`);

  if (failedTests === 0) {
    console.log("\n🎉 所有单元测试通过！");
    Deno.exit(0);
  } else {
    console.log("\n❌ 部分测试失败！");
    results.filter((r) => !r.passed).forEach((r) =>
      console.log(`  ✗ ${r.file}`)
    );
    Deno.exit(1);
  }
}

if (import.meta.main) {
  await main();
}
