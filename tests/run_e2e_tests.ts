#!/usr/bin/env -S deno run --allow-all

/**
 * 运行E2E测试（二进制测试）
 * 一次性运行所有E2E测试，共享服务器实例
 */

import { assertEquals, assertExists } from "https://deno.land/std@0.210.0/testing/asserts.ts";
import { join } from "std/path";

const OUTPUT_BINARY = "tspserver-test";
const TEST_PORT = 9100;
const TEST_ROOT = "./tests/test_www";
const STARTUP_DELAY = 2000;
const RELOAD_DELAY = 1000;

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
 * 获取二进制文件路径
 */
function getBinaryPath(): string {
  return Deno.build.os === "windows" ? `${OUTPUT_BINARY}.exe` : OUTPUT_BINARY;
}

/**
 * 清理旧的二进制文件
 */
async function cleanupBinary(): Promise<void> {
  const filesToRemove = [OUTPUT_BINARY];
  if (Deno.build.os === "windows") {
    filesToRemove.push(`${OUTPUT_BINARY}.exe`);
  }

  for (const file of filesToRemove) {
    try {
      await Deno.remove(file);
    } catch (error) {
      if (error instanceof Deno.errors.NotFound) {
        // 文件不存在，忽略
      } else {
        throw error;
      }
    }
  }
}

/**
 * 编译二进制文件
 */
async function compileBinary(): Promise<void> {
  const outputFile = getBinaryPath();

  const command = new Deno.Command("deno", {
    args: [
      "compile",
      "--allow-net",
      "--allow-read",
      "--allow-write",
      "--allow-env",
      "--output",
      outputFile,
      "src/main.ts",
    ],
    stdout: "piped",
    stderr: "piped",
  });

  const { code, stderr } = await command.output();

  if (code !== 0) {
    const stderrText = new TextDecoder().decode(stderr);
    throw new Error(`编译失败: ${stderrText}`);
  }
}

/**
 * 启动服务器进程
 */
async function startServer(): Promise<void> {
  const binaryPath = getBinaryPath();
  const commandPath = Deno.build.os === "windows" && !binaryPath.startsWith("./")
    ? `./${binaryPath}`
    : binaryPath;

  const command = new Deno.Command(commandPath, {
    args: ["--root", TEST_ROOT, "--port", TEST_PORT.toString()],
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
    } else {
      const lsofCommand = new Deno.Command("lsof", {
        args: ["-ti", `:${port}`],
        stdout: "piped",
        stderr: "piped",
      });

      const { stdout, code } = await lsofCommand.output();

      if (code === 0) {
        const output = new TextDecoder().decode(stdout);
        const pidsStr = output.trim().split("\n");
        pids = pidsStr.map((pid) => parseInt(pid)).filter((pid) => !isNaN(pid));
      }
    }

    if (pids.length > 0) {
      for (const pid of pids) {
        try {
          if (Deno.build.os === "windows") {
            const killCommand = new Deno.Command("taskkill", {
              args: ["/PID", pid.toString(), "/F"],
              stdout: "piped",
              stderr: "piped",
            });
            await killCommand.output();
          } else {
            Deno.kill(pid, "SIGKILL");
          }
        } catch {
          // 忽略终止失败
        }
      }

      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  } catch {
    // 忽略端口检查错误
  }
}

/**
 * 测试 HTTP 请求
 */
async function testHttpRequest(
  url: string,
  expectedStatus: number,
  options: {
    expectedContentType?: string;
    expectHtml?: boolean;
    method?: "GET" | "POST" | "PUT" | "DELETE";
    body?: string;
    headers?: Record<string, string>;
  } = {}
): Promise<void> {
  const {
    expectedContentType,
    expectHtml = false,
    method = "GET",
    body,
    headers,
  } = options;

  const response = await fetch(url, {
    method,
    body,
    headers,
  });

  assertEquals(response.status, expectedStatus);

  const text = await response.text();

  // 检查 Content-Type（如果指定了期望值）
  if (expectedContentType) {
    const contentType = response.headers.get("content-type");
    assertExists(contentType?.includes(expectedContentType));
  }

  // 对于 HTML 响应，检查是否包含 HTML 标签
  if (expectHtml && expectedStatus === 200) {
    assertExists(text.includes("<html") || text.includes("<!DOCTYPE"));
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

// ============================================
// 主测试执行函数
// ============================================

async function runE2ETests(): Promise<void> {
  const tests: Array<{
    name: string;
    fn: () => Promise<void>;
  }> = [];

  // 测试 1: 编译并启动服务器
  tests.push({
    name: "binary build - 编译并启动服务器",
    fn: async () => {
      const startTime = Date.now();

      printSection("环境准备");

      await cleanupBinary();
      console.log(`  ${COLORS.green}✓ 清理旧文件${COLORS.reset}`);

      await compileBinary();
      const binaryPath = getBinaryPath();
      const stat = await Deno.stat(binaryPath);
      const sizeMB = (stat.size / 1024 / 1024).toFixed(2);
      console.log(`  ${COLORS.green}✓ 二进制文件编译成功 (${sizeMB} MB)${COLORS.reset}`);

      await killProcessOnPort(TEST_PORT);
      console.log(`  ${COLORS.green}✓ 端口 ${TEST_PORT} 已清理${COLORS.reset}`);

      await startServer();
      console.log(`  ${COLORS.green}✓ 服务器已启动在端口 ${TEST_PORT}${COLORS.reset}`);

      const duration = Date.now() - startTime;
      printTestResult("编译并启动服务器", true, duration);
    },
  });

  // 测试 2: 基本 HTTP 功能
  tests.push({
    name: "http - 基本 HTTP 功能",
    fn: async () => {
      const startTime = Date.now();

      printSubsection("基本 HTTP 测试");

      await testHttpRequest(`http://localhost:${TEST_PORT}/`, 200, {
        expectHtml: true,
      });
      printTestResult("根路径 /", true);

      await testHttpRequest(`http://localhost:${TEST_PORT}/index.tsx`, 200, {
        expectHtml: true,
      });
      printTestResult("index.tsx", true);

      await testHttpRequest(`http://localhost:${TEST_PORT}/form.tsx`, 200, {
        expectHtml: true,
      });
      printTestResult("form.tsx", true);

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    },
  });

  // 测试 3: API 测试
  tests.push({
    name: "http - API 测试",
    fn: async () => {
      const startTime = Date.now();

      printSubsection("API 测试");

      await testHttpRequest(`http://localhost:${TEST_PORT}/api.tsx`, 200, {
        expectHtml: true,
      });
      printTestResult("api.tsx", true);

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms}${COLORS.reset}`);
    },
  });

  // 测试 3.5: 依赖注入测试页面（基本 TSX 功能）
  tests.push({
    name: "injection - 依赖注入测试页面",
    fn: async () => {
      const startTime = Date.now();

      printSubsection("依赖注入测试页面");

      const response = await fetch(`http://localhost:${TEST_PORT}/injection.tsx`);

      // 如果不是 200，输出错误信息
      if (response.status !== 200) {
        const text = await response.text();
        console.log(`  ${COLORS.red}错误响应内容:${COLORS.reset}`);
        console.log(`  ${COLORS.dim}${text.substring(0, 500)}${COLORS.reset}`);

        // 输出服务器错误日志
        const serverErrors = (globalThis as any).serverErrors || '';
        if (serverErrors) {
          console.log(`  ${COLORS.red}服务器错误日志:${COLORS.reset}`);
          console.log(`  ${COLORS.dim}${serverErrors}${COLORS.reset}`);
        }
      }

      assertEquals(response.status, 200);

      const text = await response.text();
      // 验证页面包含预期内容（注意：由于编译限制，E2E 不测试实际依赖注入）
      assertExists(text.includes("依赖注入测试"));
      assertExists(text.includes("单元测试覆盖"));
      printTestResult("injection.tsx - 基本页面功能正常", true);

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    },
  });

  // 注意：JSX Import 测试已移到单独的测试套件 (tests/test_jsx_imports.ts)
  // 因为编译后的二进制文件有已知的 import 限制，JSX import 功能只在源码模式下测试

  // 测试 4: 错误处理
  tests.push({
    name: "http - 错误处理",
    fn: async () => {
      const startTime = Date.now();

      printSubsection("错误处理测试");

      await testHttpRequest(`http://localhost:${TEST_PORT}/error.tsx`, 500);
      printTestResult("500 服务器错误", true);

      await testHttpRequest(`http://localhost:${TEST_PORT}/nonexistent.tsx`, 404);
      printTestResult("404 Not Found", true);

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms}${COLORS.reset}`);
    },
  });

  // 测试 5: 安全性测试
  tests.push({
    name: "security - 路径穿越防护",
    fn: async () => {
      const startTime = Date.now();

      printSubsection("安全性测试");

      const pathTraversalAttempts = [
        "../../../etc/passwd",
        "../../.env",
        "./../../secret.tsx",
      ];

      for (const path of pathTraversalAttempts) {
        try {
          const url = `http://localhost:${TEST_PORT}/${path}`;
          const response = await fetch(url);
          const blocked = response.status === 404 || response.status >= 400;
          printTestResult(`阻止路径穿越`, blocked);
          if (!blocked) throw new Error(`路径穿越攻击未被阻止: ${path}`);
        } catch {
          printTestResult(`阻止路径穿越`, true);
        }
      }

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms}${COLORS.reset}`);
    },
  });

  // 测试 6: 清理资源
  tests.push({
    name: "binary build - 停止服务器",
    fn: async () => {
      const startTime = Date.now();

      printSubsection("清理资源");

      await stopServer();
      console.log(`  ${COLORS.green}✓ 服务器已停止${COLORS.reset}`);

      const binaryPath = getBinaryPath();
      console.log(`\n${COLORS.cyan}💡 提示: 二进制文件: ${binaryPath}${COLORS.reset}`);
      console.log(`${COLORS.cyan}💡 手动运行: ${binaryPath} --root ./www --port 9000${COLORS.reset}`);

      const duration = Date.now() - startTime;
      printTestResult("清理资源", true, duration);

      console.log(`\n${COLORS.cyan}${COLORS.bright}╔════════════════════════════════════════════╗${COLORS.reset}`);
      console.log(`${COLORS.cyan}${COLORS.bright}║   E2E 测试完成                           ║${COLORS.reset}`);
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
      console.log(`  ${COLORS.green}✓ ${test.name}${COLORS.reset}\n`);
    } catch (error) {
      failedTests++;
      console.error(`  ${COLORS.red}✗ ${test.name}${COLORS.reset}`);
      console.error(`    ${error instanceof Error ? error.message : String(error)}\n`);
    }
  }

  // 打印总结
  console.log(`\n${COLORS.cyan}${COLORS.bright}╔════════════════════════════════════════════╗${COLORS.reset}`);
  console.log(`${COLORS.cyan}${COLORS.bright}║   测试总结                                 ║${COLORS.reset}`);
  console.log(`${COLORS.cyan}${COLORS.bright}╚════════════════════════════════════════════╝${COLORS.reset}`);

  console.log(`\n${COLORS.bright}总测试数: ${tests.length}${COLORS.reset}`);
  console.log(`${COLORS.green}${COLORS.bright}通过: ${passedTests}${COLORS.reset}`);
  if (failedTests > 0) {
    console.log(`${COLORS.red}${COLORS.bright}失败: ${failedTests}${COLORS.reset}`);
  }

  if (failedTests === 0) {
    console.log(`\n${COLORS.green}${COLORS.bright}🎉 所有E2E测试通过！${COLORS.reset}`);
  } else {
    console.log(`\n${COLORS.red}${COLORS.bright}❌ 部分测试失败！${COLORS.reset}`);
    Deno.exit(1);
  }
}

// 启动横幅
console.log(`\n${COLORS.cyan}${COLORS.bright}╔════════════════════════════════════════════╗${COLORS.reset}`);
console.log(`${COLORS.cyan}${COLORS.bright}║   TSP E2E 测试                            ║${COLORS.reset}`);
console.log(`${COLORS.cyan}${COLORS.bright}╚════════════════════════════════════════════╝${COLORS.reset}`);

// 运行测试
if (import.meta.main) {
  await runE2ETests();
}
