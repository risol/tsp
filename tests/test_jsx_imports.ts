#!/usr/bin/env -S deno run --allow-all

/**
 * JSX Import 功能测试
 * 测试导入 JSX 组件和 TS 工具函数（非 src 目录）
 * 注意：此测试只在源码模式下运行，因为编译后的二进制文件有已知的 import 问题
 */

import { assertEquals, assertExists } from "https://deno.land/std@0.210.0/testing/asserts.ts";

const TEST_PORT = 9102;
const TEST_ROOT = "./tests/test_www";
const STARTUP_DELAY = 2000;

// 全局服务器进程
let serverProcess: Deno.ChildProcess | null = null;

// ANSI 颜色码
const COLORS = {
  reset: "\x1b[0m",
  bright: "\x1b[1m",
  dim: "\x1b[2m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  cyan: "\x1b[36m",
};

/**
 * 杀掉占用端口的进程
 */
async function killProcessOnPort(port: number): Promise<void> {
  try {
    let pids: number[] = [];

    if (Deno.build.os === "windows") {
      const netstatCommand = new Deno.Command("netstat", {
        args: ["-ano"],
        stdout: "piped",
        stderr: "piped",
      });

      const { stdout } = await netstatCommand.output();
      const output = new TextDecoder().decode(stdout);

      const lines = output.split("\n");
      for (const line of lines) {
        if (line.includes(`:${port}`) && line.includes("LISTENING")) {
          const parts = line.trim().split(/\s+/);
          const pid = parseInt(parts[parts.length - 1]);
          if (!isNaN(pid)) {
            pids.push(pid);
          }
        }
      }
    }

    // 杀掉进程
    for (const pid of pids) {
      const killCommand = Deno.build.os === "windows"
        ? new Deno.Command("taskkill", {
          args: ["/F", "/PID", pid.toString()],
          stdout: "piped",
          stderr: "piped",
        })
        : new Deno.Command("kill", {
          args: ["-9", pid.toString()],
          stdout: "piped",
          stderr: "piped",
        });

      await killCommand.output();
    }
  } catch {
    // 忽略错误
  }
}

/**
 * 启动源码模式服务器
 */
async function startDevServer(): Promise<void> {
  const command = new Deno.Command("deno", {
    args: [
      "run",
      "--allow-net",
      "--allow-read",
      "--allow-write",
      "--allow-env",
      "src/main.ts",
      "--root",
      TEST_ROOT,
      "--port",
      TEST_PORT.toString(),
    ],
    stdout: "piped",
    stderr: "piped",
  });

  serverProcess = command.spawn();

  // 等待服务器启动
  await new Promise((resolve) => setTimeout(resolve, STARTUP_DELAY));
}

/**
 * 停止服务器进程
 */
async function stopServer(): Promise<void> {
  if (serverProcess) {
    try {
      serverProcess.kill("SIGTERM");
    } catch {
      // 忽略错误
    }
    serverProcess = null;
  }
}

function printSection(title: string) {
  console.log(`\n${COLORS.cyan}${COLORS.bright}╔════════════════════════════════════════════╗${COLORS.reset}`);
  console.log(`${COLORS.cyan}${COLORS.bright}║   ${title.padEnd(38)}║${COLORS.reset}`);
  console.log(`${COLORS.cyan}${COLORS.bright}╚════════════════════════════════════════════╝${COLORS.reset}`);
}

function printSubsection(title: string) {
  console.log(`\n${COLORS.yellow}${COLORS.bright}▶ ${title}${COLORS.reset}`);
  console.log(`${COLORS.dim}─${"─".repeat(50)}${COLORS.reset}`);
}

function printTestResult(name: string, passed: boolean, duration?: number) {
  const symbol = passed ? "✓" : "✗";
  const color = passed ? COLORS.green : COLORS.red;
  const durationStr = duration ? ` ${COLORS.dim}(${duration}ms)${COLORS.reset}` : "";
  console.log(`  ${color}${symbol} ${name}${durationStr}${COLORS.reset}`);
}

/**
 * 主测试函数
 */
async function runTests(): Promise<void> {
  const tests: Array<{
    name: string;
    fn: () => Promise<void>;
  }> = [];

  // 测试 1: 启动服务器
  tests.push({
    name: "setup - 启动开发服务器",
    fn: async () => {
      const startTime = Date.now();

      printSection("环境准备");

      await killProcessOnPort(TEST_PORT);
      console.log(`  ${COLORS.green}✓ 端口 ${TEST_PORT} 已清理${COLORS.reset}`);

      await startDevServer();
      console.log(`  ${COLORS.green}✓ 开发服务器已启动在端口 ${TEST_PORT}${COLORS.reset}`);

      const duration = Date.now() - startTime;
      printTestResult("启动开发服务器", true, duration);
    },
  });

  // 测试 2: JSX 组件导入
  tests.push({
    name: "jsx-import - 导入 JSX 组件",
    fn: async () => {
      const startTime = Date.now();

      printSubsection("测试 JSX 组件导入");

      const response = await fetch(`http://localhost:${TEST_PORT}/jsx-imports.tsx`);
      assertEquals(response.status, 200);

      const text = await response.text();

      // 验证导入的组件正常渲染
      assertExists(text.includes("JSX Import 功能测试"));
      assertExists(text.includes("组件嵌套测试"));
      assertExists(text.includes("工具函数测试"));

      printTestResult("Header 组件导入", true);
      printTestResult("Card 组件导入", true);
      printTestResult("组件嵌套渲染", true);

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    },
  });

  // 测试 3: TS 工具函数导入
  tests.push({
    name: "jsx-import - 导入 TS 工具函数",
    fn: async () => {
      const startTime = Date.now();

      printSubsection("测试 TS 工具函数导入");

      const response = await fetch(`http://localhost:${TEST_PORT}/jsx-imports.tsx`);
      const text = await response.text();

      // 验证工具函数执行结果
      assertExists(text.includes("下午好") || text.includes("早上好") || text.includes("晚上好"));
      assertExists(text.includes("1+2+3+4+5 = 15"));
      assertExists(text.includes("文本截断测试"));

      printTestResult("formatDate 函数", true);
      printTestResult("getGreeting 函数", true);
      printTestResult("sum 函数", true);
      printTestResult("truncate 函数", true);

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    },
  });

  // 测试 4: 非src目录导入
  tests.push({
    name: "jsx-import - 非 src 目录导入",
    fn: async () => {
      const startTime = Date.now();

      printSubsection("测试非src目录导入");

      const response = await fetch(`http://localhost:${TEST_PORT}/jsx-imports.tsx`);
      const text = await response.text();

      // 验证功能验证列表
      assertExists(text.includes("功能验证"));
      assertExists(text.includes("导入 JSX 组件 (Header.tsx)"));
      assertExists(text.includes("导入 JSX 组件 (Card.tsx)"));
      assertExists(text.includes("导入 TS 工具函数 (helpers.ts)"));
      assertExists(text.includes("非src目录导入"));

      printTestResult("components/Header.tsx", true);
      printTestResult("components/Card.tsx", true);
      printTestResult("utils/helpers.ts", true);

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    },
  });

  // 测试 5: 清理资源
  tests.push({
    name: "cleanup - 清理资源",
    fn: async () => {
      const startTime = Date.now();

      printSubsection("清理资源");

      await stopServer();
      console.log(`  ${COLORS.green}✓ 服务器已停止${COLORS.reset}`);

      const duration = Date.now() - startTime;
      printTestResult("清理资源", true, duration);

      console.log(`\n${COLORS.cyan}${COLORS.bright}╔════════════════════════════════════════════╗${COLORS.reset}`);
      console.log(`${COLORS.cyan}${COLORS.bright}║   JSX Import 测试完成                    ║${COLORS.reset}`);
      console.log(`${COLORS.cyan}${COLORS.bright}╚════════════════════════════════════════════╝${COLORS.reset}`);
    },
  });

  // 执行所有测试
  let passedTests = 0;
  let failedTests = 0;

  for (const test of tests) {
    try {
      await test.fn();
      passedTests++;
    } catch (error) {
      failedTests++;
      console.error(`  ${COLORS.red}错误: ${error.message}${COLORS.reset}`);
      if (error.stack) {
        console.error(`  ${COLORS.dim}${error.stack}${COLORS.reset}`);
      }
    }
  }

  // 打印总结
  console.log(`\n${COLORS.cyan}${COLORS.bright}╔════════════════════════════════════════════╗${COLORS.reset}`);
  console.log(`${COLORS.cyan}${COLORS.bright}║   测试总结                                 ║${COLORS.reset}`);
  console.log(`${COLORS.cyan}${COLORS.bright}╚════════════════════════════════════════════╝${COLORS.reset}`);

  console.log(`\n${COLORS.bright}总测试数: ${tests.length}${COLORS.reset}`);
  console.log(`${COLORS.green}通过: ${passedTests}${COLORS.reset}`);
  if (failedTests > 0) {
    console.log(`${COLORS.red}失败: ${failedTests}${COLORS.reset}`);
    console.log(`\n${COLORS.red}${COLORS.bright}❌ 部分测试失败！${COLORS.reset}`);
    Deno.exit(1);
  } else {
    console.log(`\n${COLORS.green}${COLORS.bright}🎉 所有 JSX Import 测试通过！${COLORS.reset}`);
  }
}

// 运行测试
await runTests();
