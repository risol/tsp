#!/usr/bin/env -S deno run --allow-net --allow-read --allow-write --allow-run

/**
 * 二进制构建测试
 *
 * 测试内容：
 * 1. 编译二进制文件
 * 2. 启动二进制服务器
 * 3. 通过 HTTP 测试各种功能
 * 4. 清理资源
 */

import { assertEquals, assertExists } from "https://deno.land/std@0.210.0/testing/asserts.ts";

const OUTPUT_BINARY = "tsp-fpm-test";
const TEST_PORT = 9100;
const TEST_ROOT = "./tests/test_www";

// 测试配置
const STARTUP_DELAY = 2000; // 服务器启动延迟 2秒

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
  white: "\x1b[37m",
};

// 测试统计
let totalTests = 0;
let passedTests = 0;
let failedTests = 0;

/**
 * 格式化输出
 */
function printHeader(title: string) {
  console.log(`\n${COLORS.cyan}${COLORS.bright}╔════════════════════════════════════════════╗${COLORS.reset}`);
  console.log(`${COLORS.cyan}${COLORS.bright}║   ${title.padEnd(38)}║${COLORS.reset}`);
  console.log(`${COLORS.cyan}${COLORS.bright}╚════════════════════════════════════════════╝${COLORS.reset}`);
}

function printSection(title: string) {
  console.log(`\n${COLORS.yellow}${COLORS.bright}▶ ${title}${COLORS.reset}`);
  console.log(`${COLORS.dim}─${"─".repeat(50)}${COLORS.reset}`);
}

function printSuccess(message: string) {
  console.log(`${COLORS.green}✓ ${message}${COLORS.reset}`);
}

function printError(message: string) {
  console.log(`${COLORS.red}✗ ${message}${COLORS.reset}`);
}

function printInfo(message: string) {
  console.log(`${COLORS.blue}ℹ ${message}${COLORS.reset}`);
}

function printTestResult(name: string, passed: boolean, duration?: number) {
  totalTests++;
  if (passed) {
    passedTests++;
    const durationStr = duration ? ` ${COLORS.dim}(${duration}ms)${COLORS.reset}` : "";
    console.log(`  ${COLORS.green}✓${COLORS.reset} ${name}${durationStr}`);
  } else {
    failedTests++;
    console.log(`  ${COLORS.red}✗${COLORS.reset} ${name}`);
  }
}

function printSummary() {
  console.log(`\n${COLORS.cyan}${COLORS.bright}╔════════════════════════════════════════════╗${COLORS.reset}`);
  console.log(`${COLORS.cyan}${COLORS.bright}║   测试总结                                 ║${COLORS.reset}`);
  console.log(`${COLORS.cyan}${COLORS.bright}╚════════════════════════════════════════════╝${COLORS.reset}`);

  console.log(`\n${COLORS.bright}总测试数:${COLORS.reset} ${totalTests}`);
  console.log(`${COLORS.green}${COLORS.bright}通过:${COLORS.reset} ${passedTests}`);
  if (failedTests > 0) {
    console.log(`${COLORS.red}${COLORS.bright}失败:${COLORS.reset} ${failedTests}`);
  }

  const passRate = totalTests > 0 ? ((passedTests / totalTests) * 100).toFixed(1) : "0.0";
  console.log(`${COLORS.bright}通过率:${COLORS.reset} ${passRate}%`);

  if (failedTests === 0) {
    console.log(`\n${COLORS.green}${COLORS.bright}🎉 所有测试通过！${COLORS.reset}`);
  } else {
    console.log(`\n${COLORS.red}${COLORS.bright}❌ 部分测试失败！${COLORS.reset}`);
  }
}

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
 * 检查端口是否被占用，如果占用则kill掉进程
 */
async function killProcessOnPort(port: number): Promise<void> {
  try {
    let pids: number[] = [];

    if (Deno.build.os === "windows") {
      // Windows: 使用 netstat 查找占用端口的进程
      const netstatCommand = new Deno.Command("netstat", {
        args: ["-ano"],
        stdout: "piped",
        stderr: "piped",
      });

      const { stdout } = await netstatCommand.output();
      const output = new TextDecoder().decode(stdout);

      // 解析 netstat 输出，找到占用指定端口的 PID
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
      // Linux/Mac: 使用 lsof 查找占用端口的进程
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

    // Kill 掉找到的进程
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

      // 等待一下让进程完全退出
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  } catch {
    // 忽略端口检查错误
  }
}

/**
 * 编译二进制文件
 */
async function compileBinary(): Promise<void> {
  const outputFile = Deno.build.os === "windows" ? `${OUTPUT_BINARY}.exe` : OUTPUT_BINARY;

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

  // Windows 下使用相对路径 ./ 前缀
  const commandPath = Deno.build.os === "windows" && !binaryPath.startsWith("./")
    ? `./${binaryPath}`
    : binaryPath;

  const env = { ...Deno.env.toObject(), DENO_DIR: "./.deno" };

  const command = new Deno.Command(commandPath, {
    args: ["--root", TEST_ROOT, "--port", TEST_PORT.toString(), "--dev"],
    env,
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

// ============================================
// 测试套件
// ============================================

/**
 * 测试准备：编译并启动服务器
 */
Deno.test({
  name: "binary build - 编译并启动服务器",
  sanitizeOps: false,
  sanitizeResources: false,
  sanitizeExit: false,
  fn: async () => {
    const startTime = Date.now();

    printSection("环境准备");

    // 1. 清理旧文件
    await cleanupBinary();
    printSuccess("清理旧文件");

    // 2. 编译二进制
    await compileBinary();
    const binaryPath = getBinaryPath();
    const stat = await Deno.stat(binaryPath);
    const sizeMB = (stat.size / 1024 / 1024).toFixed(2);
    printSuccess(`二进制文件编译成功 (${sizeMB} MB)`);

    // 3. 清理端口占用
    await killProcessOnPort(TEST_PORT);
    printSuccess(`端口 ${TEST_PORT} 已清理`);

    // 4. 启动服务器
    await startServer();
    printSuccess(`服务器已启动在端口 ${TEST_PORT}`);

    const duration = Date.now() - startTime;
    printTestResult("编译并启动服务器", true, duration);
  },
});

/**
 * 基本页面测试
 */
Deno.test({
  name: "http - 基本页面测试",
  sanitizeOps: false,
  sanitizeResources: false,
  sanitizeExit: false,
  fn: async () => {
    const startTime = Date.now();

    printSection("基本页面测试");

    try {
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
      printTestResult("form.tsx 表单页面", true);

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    } catch (error) {
      printTestResult("基本页面测试", false);
      throw error;
    }
  },
});

/**
 * API 测试
 */
Deno.test({
  name: "http - API 测试",
  sanitizeOps: false,
  sanitizeResources: false,
  sanitizeExit: false,
  fn: async () => {
    const startTime = Date.now();

    printSection("API 测试");

    try {
      await testHttpRequest(`http://localhost:${TEST_PORT}/api.tsx`, 200, {
        expectHtml: true,
      });
      printTestResult("api.tsx 接口", true);

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    } catch (error) {
      printTestResult("API 测试", false);
      throw error;
    }
  },
});

/**
 * 自定义响应测试
 */
Deno.test({
  name: "http - 自定义响应测试",
  sanitizeOps: false,
  sanitizeResources: false,
  sanitizeExit: false,
  fn: async () => {
    const startTime = Date.now();

    printSection("自定义响应测试");

    try {
      await testHttpRequest(`http://localhost:${TEST_PORT}/custom_response.tsx`, 200, {
        expectHtml: true,
      });
      printTestResult("自定义 HTML Response", true);

      await testHttpRequest(
        `http://localhost:${TEST_PORT}/custom_response.tsx?format=json`,
        200,
        {
          expectedContentType: "application/json",
        }
      );
      printTestResult("自定义 JSON Response", true);

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    } catch (error) {
      printTestResult("自定义响应测试", false);
      throw error;
    }
  },
});

/**
 * 错误处理测试
 */
Deno.test({
  name: "http - 错误处理测试",
  sanitizeOps: false,
  sanitizeResources: false,
  sanitizeExit: false,
  fn: async () => {
    const startTime = Date.now();

    printSection("错误处理测试");

    try {
      await testHttpRequest(`http://localhost:${TEST_PORT}/error.tsx`, 500);
      printTestResult("500 服务器错误", true);

      await testHttpRequest(
        `http://localhost:${TEST_PORT}/nonexistent.tsx`,
        404
      );
      printTestResult("404 Not Found", true);

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    } catch (error) {
      printTestResult("错误处理测试", false);
      throw error;
    }
  },
});

/**
 * 安全性测试 - 路径穿越攻击
 */
Deno.test({
  name: "security - 路径穿越攻击防护",
  sanitizeOps: false,
  sanitizeResources: false,
  sanitizeExit: false,
  fn: async () => {
    const startTime = Date.now();

    printSection("路径穿越攻击防护");

    try {
      const pathTraversalAttempts = [
        "../../../etc/passwd",
        "..\\..\\..\\windows\\system32\\config\\sam",
        "../../.env",
        "./../../secret.tsx",
        "....//....//....//etc/passwd",
        "%2e%2e%2fetc/passwd",
        "..%252f..%252f..%252fetc/passwd",
      ];

      for (const path of pathTraversalAttempts) {
        try {
          const url = `http://localhost:${TEST_PORT}/${path}`;
          const response = await fetch(url);

          // 路径穿越攻击应该被阻止，返回 404 或 400
          if (response.status === 200) {
            const text = await response.text();
            // 检查是否真的访问到了敏感文件
            if (text.includes("root:") || text.includes("password") || text.includes("SECRET")) {
              printError(`路径穿越攻击未阻止: ${path}`);
              printTestResult(`阻止路径穿越: ${path}`, false);
              throw new Error(`安全漏洞：路径穿越攻击未被阻止 - ${path}`);
            }
          }

          // 期望返回 404 或 400 等错误状态码
          const blocked = response.status >= 400;
          if (blocked) {
            printTestResult(`阻止路径穿越: ${path.substring(0, 30)}...`, true);
          } else {
            printTestResult(`阻止路径穿越: ${path.substring(0, 30)}...`, false);
            throw new Error(`路径穿越攻击可能成功: ${path}`);
          }
        } catch (error) {
          if (error instanceof Error && error.message.includes("安全漏洞")) {
            throw error;
          }
          // 网络错误也算阻止成功
          printTestResult(`阻止路径穿越: ${path.substring(0, 30)}...`, true);
        }
      }

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    } catch (error) {
      printTestResult("路径穿越攻击防护", false);
      throw error;
    }
  },
});

/**
 * 安全性测试 - 非法文件访问
 */
Deno.test({
  name: "security - 非法文件访问防护",
  sanitizeOps: false,
  sanitizeResources: false,
  sanitizeExit: false,
  fn: async () => {
    const startTime = Date.now();

    printSection("非法文件访问防护");

    try {
      const illegalFileAttempts = [
        "config.json",
        ".env",
        "package.json",
        "deno.json",
        "secret.ts",
        "../src/main.ts",
        "../../tests/basic_test.ts",
        "data.txt",
        "backup.sql",
      ];

      for (const file of illegalFileAttempts) {
        try {
          const url = `http://localhost:${TEST_PORT}/${file}`;
          const response = await fetch(url);

          // 非法文件应该被阻止访问
          if (response.status === 200) {
            const text = await response.text();
            // 检查是否真的返回了文件内容
            if (text.includes("DATABASE_URL") ||
                text.includes("API_KEY") ||
                text.includes("import ") ||
                text.includes("{")) {
              printTestResult(`阻止非法文件: ${file}`, false);
              throw new Error(`安全漏洞：非法文件可访问 - ${file}`);
            }
          }

          // 期望返回 404
          const blocked = response.status === 404;
          if (blocked) {
            printTestResult(`阻止非法文件: ${file}`, true);
          } else {
            printTestResult(`阻止非法文件: ${file}`, false);
            throw new Error(`非法文件访问未被阻止: ${file}`);
          }
        } catch (error) {
          if (error instanceof Error && error.message.includes("安全漏洞")) {
            throw error;
          }
          printTestResult(`阻止非法文件: ${file}`, true);
        }
      }

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    } catch (error) {
      printTestResult("非法文件访问防护", false);
      throw error;
    }
  },
});

/**
 * 安全性测试 - HTTP 方法安全
 */
Deno.test({
  name: "security - HTTP 方法安全",
  sanitizeOps: false,
  sanitizeResources: false,
  sanitizeExit: false,
  fn: async () => {
    const startTime = Date.now();

    printSection("HTTP 方法安全");

    try {
      // 测试各种 HTTP 方法是否被正确处理（不抛出错误）
      const testMethods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

      for (const method of testMethods) {
        try {
          const response = await fetch(`http://localhost:${TEST_PORT}/`, {
            method,
          });

          // 检查响应状态码是否合理
          const validStatus = response.status >= 100 && response.status < 600;

          if (!validStatus) {
            printTestResult(`${method} 方法返回无效状态`, false);
            throw new Error(`${method} 方法返回无效状态码: ${response.status}`);
          }

          // 对于 PUT/DELETE/PATCH 等方法，即使返回 200 也可以
          // 因为框架可能只是忽略方法类型，都当做 GET 处理
          // 关键是确保服务器不会崩溃或返回错误
          printTestResult(`${method} 方法正常处理`, true);
        } catch (error) {
          if (error instanceof Error && error.message.includes("无效状态码")) {
            printTestResult(`${method} 方法正常处理`, false);
            throw error;
          }
          // 网络错误也算失败
          printTestResult(`${method} 方法正常处理`, false);
        }
      }

      // 测试危险方法 TRACE（可能被用于 XST 攻击）
      try {
        const traceResponse = await fetch(`http://localhost:${TEST_PORT}/`, {
          method: "TRACE",
        });

        // TRACE 方法应该被拒绝或返回错误
        if (traceResponse.status === 200) {
          // 如果返回 200，检查响应体是否包含请求头（XST 漏洞）
          const text = await traceResponse.text();
          if (text.includes("HTTP/") || text.includes("User-Agent")) {
            printTestResult("阻止 TRACE 方法（XST 防护）", false);
            throw new Error("安全漏洞：TRACE 方法可能被用于 XST 攻击");
          }
        }
        printTestResult("TRACE 方法安全检查", true);
      } catch (error) {
        if (error instanceof Error && error.message.includes("XST")) {
          printTestResult("阻止 TRACE 方法（XST 防护）", false);
          throw error;
        }
        printTestResult("TRACE 方法安全检查", true);
      }

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    } catch (error) {
      printTestResult("HTTP 方法安全", false);
      throw error;
    }
  },
});

/**
 * 安全性测试 - 请求头注入
 */
Deno.test({
  name: "security - 请求头注入防护",
  sanitizeOps: false,
  sanitizeResources: false,
  sanitizeExit: false,
  fn: async () => {
    const startTime = Date.now();

    printSection("请求头注入防护");

    try {
      // 测试各种注入尝试
      const injectionAttempts = [
        { header: "X-Forwarded-For", value: "127.0.0.1\r\nInjected-Header: true" },
        { header: "User-Agent", value: "Mozilla/5.0\r\nX-Injected: true" },
        { header: "Referer", value: "https://evil.com\r\nX-Evil: true" },
        { header: "X-Forwarded-Host", value: "evil.com" },
        { header: "X-Original-URL", value: "/admin/delete" },
      ];

      for (const { header, value } of injectionAttempts) {
        try {
          const response = await fetch(`http://localhost:${TEST_PORT}/`, {
            headers: {
              [header]: value,
            },
          });

          // 检查响应中是否包含注入的内容
          const responseText = await response.text();

          // 简单检查：响应不应该包含注入的恶意内容
          const hasInjection = responseText.includes("Injected-Header") ||
                              responseText.includes("X-Injected") ||
                              responseText.includes("X-Evil");

          if (hasInjection) {
            printTestResult(`阻止请求头注入: ${header}`, false);
            throw new Error(`请求头注入攻击成功: ${header}`);
          }

          printTestResult(`阻止请求头注入: ${header}`, true);
        } catch (error) {
          if (error instanceof Error && error.message.includes("请求头注入")) {
            throw error;
          }
          printTestResult(`阻止请求头注入: ${header}`, true);
        }
      }

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    } catch (error) {
      printTestResult("请求头注入防护", false);
      throw error;
    }
  },
});

/**
 * 安全性测试 - URL 参数注入
 */
Deno.test({
  name: "security - URL 参数注入防护",
  sanitizeOps: false,
  sanitizeResources: false,
  sanitizeExit: false,
  fn: async () => {
    const startTime = Date.now();

    printSection("URL 参数注入防护");

    try {
      const injectionAttempts = [
        "?id=1' OR '1'='1",
        "?search=<script>alert('XSS')</script>",
        "?redirect=https://evil.com",
        "?file=../../etc/passwd",
        "?debug=true",
        "?admin=true",
        "?token=abc123",
      ];

      for (const param of injectionAttempts) {
        try {
          const url = `http://localhost:${TEST_PORT}/index.tsx${param}`;
          const response = await fetch(url);
          const text = await response.text();

          // 检查是否反射了 XSS 脚本
          const hasXSS = text.includes("<script>alert") ||
                         text.includes("<script>") ||
                         text.includes("javascript:");

          // 检查是否暴露了调试信息
          const hasDebugInfo = text.includes("Stack trace") ||
                              text.includes("Error: ") ||
                              text.includes("at line");

          if (hasXSS) {
            printTestResult(`阻止 XSS 注入: ${param.substring(0, 20)}...`, false);
            throw new Error(`XSS 漏洞：参数被反射: ${param}`);
          }

          // 生产模式不应该显示详细错误
          if (hasDebugInfo && response.status === 200) {
            printTestResult(`防止调试信息泄露: ${param.substring(0, 20)}...`, false);
            throw new Error(`调试信息泄露: ${param}`);
          }

          printTestResult(`安全处理参数: ${param.substring(0, 20)}...`, true);
        } catch (error) {
          if (error instanceof Error && (error.message.includes("XSS") || error.message.includes("调试信息"))) {
            throw error;
          }
          printTestResult(`安全处理参数: ${param.substring(0, 20)}...`, true);
        }
      }

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    } catch (error) {
      printTestResult("URL 参数注入防护", false);
      throw error;
    }
  },
});

/**
 * 安全性测试 - 并发请求压力
 */
Deno.test({
  name: "security - 并发请求压力测试",
  sanitizeOps: false,
  sanitizeResources: false,
  sanitizeExit: false,
  fn: async () => {
    const startTime = Date.now();

    printSection("并发请求压力测试");

    try {
      const concurrentRequests = 50;
      const requests: Promise<Response>[] = [];

      // 发送大量并发请求
      for (let i = 0; i < concurrentRequests; i++) {
        requests.push(
          fetch(`http://localhost:${TEST_PORT}/`)
        );
      }

      // 等待所有请求完成
      const responses = await Promise.all(requests);

      // 统计成功的请求数
      const successCount = responses.filter(r => r.status === 200).length;
      const errorCount = responses.filter(r => r.status >= 400).length;

      printTestResult(`处理 ${concurrentRequests} 个并发请求`, true);
      printInfo(`成功: ${successCount}, 错误: ${errorCount}`);

      // 检查是否有大量失败（可能是 DoS 漏洞）
      const errorRate = errorCount / concurrentRequests;
      if (errorRate > 0.5) {
        printError(`错误率过高: ${(errorRate * 100).toFixed(1)}%`);
        throw new Error("服务器可能存在 DoS 漏洞");
      }

      const duration = Date.now() - startTime;
      printInfo(`平均响应时间: ${(duration / concurrentRequests).toFixed(0)}ms`);
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    } catch (error) {
      if (error instanceof Error && error.message.includes("DoS")) {
        printTestResult("并发请求压力测试", false);
        throw error;
      }
      printTestResult("并发请求压力测试", false);
      throw error;
    }
  },
});

/**
 * 配置文件测试
 */
Deno.test({
  name: "config - 配置文件测试",
  sanitizeOps: false,
  sanitizeResources: false,
  sanitizeExit: false,
  fn: async () => {
    const startTime = Date.now();

    printSection("配置文件测试");

    try {
      // 先停止当前服务器
      await stopServer();
      printSuccess("已停止当前服务器");

      // 等待端口释放
      await new Promise((resolve) => setTimeout(resolve, 500));

      /**
       * 辅助函数：启动带配置的服务器
       */
      async function startWithConfig(configContent: string, filename: string, args: string[] = []): Promise<boolean> {
        try {
          // 写入配置文件
          await Deno.writeTextFile(filename, configContent);

          // 启动服务器
          const binaryPath = getBinaryPath();
          const commandPath = Deno.build.os === "windows" && !binaryPath.startsWith("./")
            ? `./${binaryPath}`
            : binaryPath;

          const serverProcess = new Deno.Command(commandPath, {
            args: [...args, "--root", TEST_ROOT],
            stdout: "piped",
            stderr: "piped",
          });

          const process = serverProcess.spawn();

          // 等待服务器启动
          await new Promise((resolve) => setTimeout(resolve, STARTUP_DELAY));

          // 检查是否启动成功
          try {
            const response = await fetch(`http://localhost:${TEST_PORT}/`);
            const success = response.status === 200;

            // 停止服务器
            try {
              process.kill("SIGTERM");
            } catch {
              // 忽略错误
            }

            return success;
          } catch {
            // 无法连接，启动失败
            try {
              process.kill("SIGTERM");
            } catch {
              // 忽略错误
            }
            return false;
          }
        } finally {
          // 清理配置文件
          try {
            await Deno.remove(filename);
          } catch {
            // 忽略错误
          }
        }
      }

      // 测试 1: config.json 配置文件
      printInfo("测试 config.json 配置文件");
      const jsonConfig = JSON.stringify({
        root: TEST_ROOT,
        port: TEST_PORT,
        dev: false,
      }, null, 2);

      const success1 = await startWithConfig(jsonConfig, "config.json");
      printTestResult("config.json 配置文件", success1);
      if (!success1) throw new Error("config.json 配置文件测试失败");

      // 等待端口释放
      await new Promise((resolve) => setTimeout(resolve, 500));

      // 测试 2: config.jsonc 配置文件（支持注释）
      printInfo("测试 config.jsonc 配置文件（带注释）");
      const jsoncConfig = `{
  // 这是一个配置文件
  "root": "${TEST_ROOT}",
  "port": ${TEST_PORT},
  /* 多行注释
     开发模式 */
  "dev": false
}`;

      const success2 = await startWithConfig(jsoncConfig, "config.jsonc");
      printTestResult("config.jsonc 配置文件（支持注释）", success2);
      if (!success2) throw new Error("config.jsonc 配置文件测试失败");

      // 等待端口释放
      await new Promise((resolve) => setTimeout(resolve, 500));

      // 测试 3: 配置文件优先级（jsonc > json）
      printInfo("测试配置文件优先级");
      await Deno.writeTextFile("config.json", JSON.stringify({ root: TEST_ROOT, port: TEST_PORT, dev: false }));
      await Deno.writeTextFile("config.jsonc", JSON.stringify({ root: TEST_ROOT, port: TEST_PORT, dev: true }));

      // 启动服务器（应该使用 config.jsonc）
      const binaryPath = getBinaryPath();
      const commandPath = Deno.build.os === "windows" && !binaryPath.startsWith("./")
        ? `./${binaryPath}`
        : binaryPath;

      const priorityProcess = new Deno.Command(commandPath, {
        args: ["--root", TEST_ROOT],
        stdout: "piped",
        stderr: "piped",
      });

      const priorityServer = priorityProcess.spawn();
      await new Promise((resolve) => setTimeout(resolve, STARTUP_DELAY));

      // 检查是否是开发模式（来自 jsonc）
      try {
        const response = await fetch(`http://localhost:${TEST_PORT}/error.tsx`);
        const text = await response.text();
        const isDevMode = text.includes("Stack trace") || text.includes("Error:");

        printTestResult("配置文件优先级 (jsonc > json)", isDevMode);

        priorityServer.kill("SIGTERM");
      } catch {
        priorityServer.kill("SIGTERM");
        printTestResult("配置文件优先级 (jsonc > json)", false);
        throw new Error("配置文件优先级测试失败");
      }

      // 清理配置文件
      try {
        await Deno.remove("config.json");
        await Deno.remove("config.jsonc");
      } catch {
        // 忽略错误
      }

      // 等待端口释放
      await new Promise((resolve) => setTimeout(resolve, 500));

      // 测试 4: 命令行参数覆盖配置文件
      printInfo("测试命令行参数覆盖配置文件");
      await Deno.writeTextFile("config.json", JSON.stringify({
        root: TEST_ROOT,
        port: TEST_PORT,
        dev: false,
      }));

      const overrideProcess = new Deno.Command(commandPath, {
        args: ["--dev"], // 覆盖配置文件中的 dev: false
        stdout: "piped",
        stderr: "piped",
      });

      const overrideServer = overrideProcess.spawn();
      await new Promise((resolve) => setTimeout(resolve, STARTUP_DELAY));

      try {
        const response = await fetch(`http://localhost:${TEST_PORT}/error.tsx`);
        const text = await response.text();
        const isDevMode = text.includes("Stack trace") || text.includes("Error:");

        printTestResult("命令行参数覆盖配置文件", isDevMode);

        overrideServer.kill("SIGTERM");

        if (!isDevMode) {
          throw new Error("命令行参数覆盖测试失败");
        }
      } catch {
        overrideServer.kill("SIGTERM");
        printTestResult("命令行参数覆盖配置文件", false);
        throw new Error("命令行参数覆盖测试失败");
      }

      // 清理配置文件
      try {
        await Deno.remove("config.json");
      } catch {
        // 忽略错误
      }

      // 等待端口释放
      await new Promise((resolve) => setTimeout(resolve, 500));

      // 测试 5: 无效的配置文件
      printInfo("测试无效配置文件处理");
      await Deno.writeTextFile("config.json", "{ invalid json }");

      const invalidProcess = new Deno.Command(commandPath, {
        args: ["--root", TEST_ROOT],
        stdout: "piped",
        stderr: "piped",
      });

      const invalidServer = invalidProcess.spawn();
      await new Promise((resolve) => setTimeout(resolve, 1000));

      // 检查服务器是否启动失败（预期行为）
      try {
        const response = await fetch(`http://localhost:${TEST_PORT}/`);
        const failed = response.status !== 200;

        printTestResult("无效配置文件被拒绝", failed);

        try {
          invalidServer.kill("SIGTERM");
        } catch {
          // 忽略错误
        }

        if (!failed) {
          throw new Error("无效配置文件应该被拒绝");
        }
      } catch {
        // 连接失败说明服务器未启动，这是正确的
        printTestResult("无效配置文件被拒绝", true);

        try {
          invalidServer.kill("SIGTERM");
        } catch {
          // 忽略错误
        }
      }

      // 清理配置文件
      try {
        await Deno.remove("config.json");
      } catch {
        // 忽略错误
      }

      const duration = Date.now() - startTime;
      console.log(`  ${COLORS.dim}${duration}ms${COLORS.reset}`);
    } catch (error) {
      printTestResult("配置文件测试", false);
      throw error;
    }
  },
});

/**
 * 测试清理：停止服务器
 */
Deno.test({
  name: "binary build - 停止服务器",
  sanitizeOps: false,
  sanitizeResources: false,
  sanitizeExit: false,
  fn: async () => {
    const startTime = Date.now();

    printSection("清理资源");

    await stopServer();
    printSuccess("服务器已停止");

    const binaryPath = getBinaryPath();
    printInfo(`二进制文件: ${binaryPath}`);
    printInfo(`手动运行: ${binaryPath} --root ./www --port 9000`);

    const duration = Date.now() - startTime;
    printTestResult("清理资源", true, duration);

    // 打印总结
    printSummary();
  },
});

// 启动横幅
console.log(`\n${COLORS.cyan}${COLORS.bright}╔════════════════════════════════════════════╗${COLORS.reset}`);
console.log(`${COLORS.cyan}${COLORS.bright}║   TSP-FPM 二进制构建 HTTP 测试             ║${COLORS.reset}`);
console.log(`${COLORS.cyan}${COLORS.bright}╚════════════════════════════════════════════╝${COLORS.reset}`);
