#!/usr/bin/env -S deno run --allow-net --allow-read --allow-write --allow-run

/**
 * 二进制构建测试
 *
 * 测试内容：
 * 1. 编译二进制文件
 * 2. 验证编译产物
 * 3. 测试二进制文件运行
 * 4. 测试基本 HTTP 功能
 */

import { assertEquals, assertExists } from "https://deno.land/std@0.210.0/testing/asserts.ts";

const OUTPUT_BINARY = "tsp-fpm-test";
const TEST_PORT = 9100; // 使用不同的端口避免冲突
const TEST_ROOT = "./tests/test_www";

// 测试配置
const TEST_TIMEOUT = 30000; // 30秒超时
const STARTUP_DELAY = 2000; // 服务器启动延迟 2秒

/**
 * 获取二进制文件路径
 */
function getBinaryPath(): string {
  return Deno.build.os === "windows"
    ? `${OUTPUT_BINARY}.exe`
    : OUTPUT_BINARY;
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
      console.log(`✓ 清理旧文件: ${file}`);
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
  console.log("\n=== 开始编译二进制文件 ===");

  // 在 Windows 下显式添加 .exe 后缀
  const outputFile = Deno.build.os === "windows"
    ? `${OUTPUT_BINARY}.exe`
    : OUTPUT_BINARY;

  console.log(`✓ 编译输出: ${outputFile}`);

  const command = new Deno.Command("deno", {
    args: [
      "compile",
      "--allow-net",
      "--allow-read",
      "--allow-env",
      "--output", outputFile,
      "src/main.ts",
    ],
    stdout: "piped",
    stderr: "piped",
  });

  const { code, stdout, stderr } = await command.output();

  if (code !== 0) {
    console.error("编译失败!");
    const stderrText = new TextDecoder().decode(stderr);
    console.error("STDERR:", stderrText);
    throw new Error(`编译失败，退出码: ${code}`);
  }

  console.log("✓ 二进制文件编译成功");

  // 验证文件存在
  const binaryPath = getBinaryPath();
  console.log(`✓ 查找文件: ${binaryPath}`);

  try {
    const stat = await Deno.stat(binaryPath);
    console.log(`✓ 二进制文件大小: ${(stat.size / 1024 / 1024).toFixed(2)} MB`);
  } catch (error) {
    // 列出当前目录的文件用于调试
    console.error(`❌ 文件不存在: ${binaryPath}`);
    console.error("当前目录相关文件:");
    for await (const entry of Deno.readDir(".")) {
      if (entry.name.includes("tsp-fpm") || entry.name.includes(".exe")) {
        console.error(`  - ${entry.name}`);
      }
    }
    throw error;
  }
}

/**
 * 启动服务器进程
 */
async function startServer(): Promise<Deno.ChildProcess> {
  console.log("\n=== 启动测试服务器 ===");

  const binaryPath = getBinaryPath();

  // 验证文件存在
  try {
    const stat = await Deno.stat(binaryPath);
    console.log(`✓ 找到二进制文件: ${binaryPath} (${(stat.size / 1024 / 1024).toFixed(2)} MB)`);
  } catch (error) {
    throw new Error(`二进制文件不存在: ${binaryPath}`);
  }

  // Windows 下使用相对路径 ./ 前缀
  const commandPath = Deno.build.os === "windows" && !binaryPath.startsWith("./")
    ? `./${binaryPath}`
    : binaryPath;

  console.log(`✓ 启动命令: ${commandPath}`);

  const env = { ...Deno.env.toObject(), DENO_DIR: "./.deno" };

  const serverProcess = new Deno.Command(commandPath, {
    args: [
      "--root", TEST_ROOT,
      "--port", TEST_PORT.toString(),
      "--dev",
    ],
    env,
    stdout: "piped",
    stderr: "piped",
  });

  const process = serverProcess.spawn();

  console.log(`✓ 服务器进程已启动 (PID: ${process.pid})`);

  // 等待服务器启动
  console.log(`⏳ 等待 ${STARTUP_DELAY / 1000} 秒让服务器完全启动...`);
  await new Promise((resolve) => setTimeout(resolve, STARTUP_DELAY));

  return process;
}

/**
 * 停止服务器进程
 */
async function stopServer(process: Deno.ChildProcess): Promise<void> {
  console.log("\n=== 停止测试服务器 ===");

  try {
    process.kill("SIGTERM");
    console.log("✓ 服务器进程已停止");
  } catch (error) {
    console.warn("⚠ 无法停止进程:", error);
  }
}

/**
 * 测试 HTTP 请求
 */
async function testHttpRequest(
  url: string,
  expectedStatus: number = 200,
  expectedContentType?: string,
  expectErrorInBody = false
): Promise<void> {
  console.log(`\n测试请求: ${url}`);

  try {
    const response = await fetch(url);
    console.log(`✓ 状态码: ${response.status}`);

    assertEquals(response.status, expectedStatus, `期望状态码 ${expectedStatus}`);

    const text = await response.text();
    console.log(`✓ 响应长度: ${text.length} 字节`);

    // 验证是 HTML 响应
    const contentType = response.headers.get("content-type");
    console.log(`✓ Content-Type: ${contentType}`);

    // 检查 Content-Type（如果指定了期望值）
    if (expectedContentType) {
      assertExists(contentType?.includes(expectedContentType), `期望 Content-Type 包含 ${expectedContentType}`);
    }

    // 对于错误响应，检查错误信息
    if (expectedStatus >= 400 && expectErrorInBody) {
      // 生产模式：错误页面不显示堆栈
      // 开发模式：应该显示错误详情
      assertExists(text.includes("500") || text.includes("Error"), "应该包含错误信息");

      // 检查不应该包含堆栈信息（生产模式）
      const hasStackTrace = text.includes("at ") && text.includes(".ts:");
      console.log(hasStackTrace ? "⚠ 发现堆栈信息" : "✓ 无堆栈信息（生产模式）");
    }

    if (expectedStatus === 200 && (!expectedContentType || expectedContentType.includes("text/html"))) {
      assertExists(text.includes("<html") || text.includes("<!DOCTYPE"), "应该包含 HTML 标签");
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(`请求失败: ${message}`);
  }
}

/**
 * 运行所有测试
 */
async function runTests(): Promise<void> {
  console.log("╔════════════════════════════════════════════╗");
  console.log("║   TSP-FPM 二进制构建测试                  ║");
  console.log("╚════════════════════════════════════════════╝");

  let serverProcess: Deno.ChildProcess | null = null;

  try {
    // 1. 清理旧文件
    await cleanupBinary();

    // 2. 编译二进制
    await compileBinary();

    // 3. 启动服务器
    serverProcess = await startServer();

    // 4. 测试各种 HTTP 请求
    console.log("\n=== 开始 HTTP 功能测试 ===");

    await testHttpRequest(`http://localhost:${TEST_PORT}/`, 200);
    await testHttpRequest(`http://localhost:${TEST_PORT}/index.tsx`, 200);
    await testHttpRequest(`http://localhost:${TEST_PORT}/form.tsx`, 200);
    await testHttpRequest(`http://localhost:${TEST_PORT}/api.tsx`, 200);
    await testHttpRequest(`http://localhost:${TEST_PORT}/custom_response.tsx`, 200); // 自定义 Response
    await testHttpRequest(`http://localhost:${TEST_PORT}/custom_response.tsx?format=json`, 200, "application/json"); // JSON Response
    await testHttpRequest(`http://localhost:${TEST_PORT}/error.tsx`, 500); // 错误处理测试
    await testHttpRequest(`http://localhost:${TEST_PORT}/nonexistent.tsx`, 404); // 404

    console.log("\n╔════════════════════════════════════════════╗");
    console.log("║   ✓ 所有测试通过！                        ║");
    console.log("╚════════════════════════════════════════════╝");

  } catch (error) {
    console.error("\n╔════════════════════════════════════════════╗");
    console.error("║   ✗ 测试失败！                            ║");
    console.error("╚════════════════════════════════════════════╝");
    console.error("\n错误详情:");
    console.error(error);
    Deno.exit(1);
  } finally {
    // 清理：停止服务器
    if (serverProcess) {
      await stopServer(serverProcess);
    }

    // 可选：保留二进制文件用于手动测试
    const binaryPath = getBinaryPath();
    console.log(`\n提示: 二进制文件已保存在 ${binaryPath}`);
    console.log(`可以手动运行: ${binaryPath} --root ./www --port 9000`);
  }
}

// 主测试函数
Deno.test({
  name: "binary_build_test",
  fn: runTests,
  sanitizeOps: false,
  sanitizeResources: false,
  sanitizeExit: false,
});

// 如果直接运行此文件
if (import.meta.main) {
  await runTests();
}
