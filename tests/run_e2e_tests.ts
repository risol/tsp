#!/usr/bin/env bun

/**
 * Run E2E tests (binary tests)
 * Run all E2E tests at once, sharing server instance
 * Includes hot reload tests (in dev mode)
 */

import {
  assertEquals,
  assertExists,
  assertStringIncludes,
} from "./unit/asserts.ts";

// Re-export for e2e test modules
export { assertEquals, assertExists, assertStringIncludes };
import { resolve } from "node:path";
import { cwd } from "node:process";
import { spawn } from "node:child_process";
import { statSync } from "node:fs";

// Import E2E test modules
import { getHttpTests } from "./e2e/http.ts";
import { getSecurityTests } from "./e2e/security.ts";
import { getInjectionTests } from "./e2e/injection.ts";
import { getHotReloadTests } from "./e2e/hotreload.ts";
import { getSessionTests } from "./e2e/session.ts";
import { getUploadTests } from "./e2e/upload.ts";
import { getMysqlTests } from "./e2e/mysql.ts";
import { getRedisTests } from "./e2e/redis.ts";
import { getRedisSessionTests } from "./e2e/redis_session.ts";
import { getExcelJsTests } from "./e2e/exceljs.ts";
import { getLdapTests } from "./e2e/ldap.ts";
import { getConfigTests } from "./e2e/config.ts";
import { getValidationTests } from "./e2e/validation.ts";
import { getFragmentTests } from "./e2e/fragments.ts";

export const TEST_PORT = 9001;
export const RELOAD_DELAY = 1000;
// TEST_ROOT will be dynamically calculated based on current directory at runtime

/**
 * Get test website root directory path
 */
export function getTestRoot(): string {
  const currentDirectory = cwd();
  if (currentDirectory.endsWith("\\tests") || currentDirectory.endsWith("/tests")) {
    return resolve(currentDirectory, "test_www");
  } else {
    return resolve(currentDirectory, "tests", "test_www");
  }
}

// Global server process
let serverProcess: ReturnType<typeof spawn> | null = null;

// Server startup log (for analyzing compilation behavior)
let serverStartupLog: string = "";

export async function runCommand(command: string, args: string[] = []): Promise<{
  code: number;
  stdout: string;
  stderr: string;
}> {
  const child = Bun.spawn([command, ...args], {
    stdio: ["ignore", "pipe", "pipe"],
  });
  const [code, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  return { code, stdout, stderr };
}

// ANSI color codes
export const COLORS = {
  reset: "\x1b[0m",
  bright: "\x1b[1m",
  dim: "\x1b[2m",
  red: "\x1b[31m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  cyan: "\x1b[36m",
};

/** Get the Bun-compiled test server path. */
export function getBinaryPath(): string {
  const binaryName = process.platform === "win32"
    ? "tspserver.exe"
    : "tspserver";
  return resolve(cwd(), "dist", "bun", binaryName);
}

/**
 * Cleanup old server processes and verify binary exists
 */
export async function cleanupBinary(): Promise<boolean> {
  const binaryPath = getBinaryPath();

  // First kill all tspserver processes
  try {
    if (process.platform === "win32") {
      // Windows: use taskkill to kill all tspserver processes
      await runCommand("taskkill", ["/F", "/IM", "tspserver.exe"]);
      await runCommand("taskkill", ["/F", "/IM", "tspserver-test.exe"]);
    } else {
      // Linux/macOS: use pkill
      await runCommand("pkill", ["-f", "tspserver"]);
    }

    // Wait for processes to terminate
    await new Promise((resolve) => setTimeout(resolve, 500));
  } catch {
    // Ignore kill process failure
  }

  // Verify binary exists (do not delete - we use production build for testing)
  try {
    return statSync(binaryPath).isFile();
  } catch {
    return false;
  }
}

/**
 * Start server process
 * @param devMode Whether to use dev mode (default true)
 */
export async function startServer(devMode: boolean = true): Promise<void> {
  const binaryPath = getBinaryPath();
  const testRoot = getTestRoot();

  console.log(
    `  ${COLORS.dim}[DEBUG] Binary path: ${binaryPath}${COLORS.reset}`,
  );
  console.log(`  ${COLORS.dim}[DEBUG] Test root: ${testRoot}${COLORS.reset}`);
  console.log(`  ${COLORS.dim}[DEBUG] Port: ${TEST_PORT}${COLORS.reset}`);
  console.log(
    `  ${COLORS.dim}[DEBUG] Mode: ${devMode ? "development" : "production"}${COLORS.reset}`,
  );

  const args = ["--root", testRoot, "--port", TEST_PORT.toString()];
  if (devMode) {
    args.push("--dev");
  }

  // Node's child-process bridge reliably starts the compiled Bun binary on
  // Windows when its console handles are inherited.
  serverProcess = spawn(binaryPath, args, { stdio: "inherit" });
  console.log(`  ${COLORS.dim}[DEBUG] Spawned server process: ${serverProcess.pid}${COLORS.reset}`);
  serverProcess.once("error", (error) => {
    console.error(`  ${COLORS.red}[DEBUG] Server process error: ${String(error)}${COLORS.reset}`);
  });
  serverProcess.once("exit", (code, signal) => {
    console.error(
      `  ${COLORS.red}[DEBUG] Server process exited: code=${code}, signal=${signal}${COLORS.reset}`,
    );
  });

  // Async read server logs (for analyzing compilation behavior)
  serverStartupLog = "";
  const readStream = async (stream: ReadableStream<Uint8Array>) => {
    const reader = stream.getReader();
    const decoder = new TextDecoder();
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      const text = decoder.decode(value);
      serverStartupLog += text;
    }
  };

  // Start background task to read stdout and stderr
  if (serverProcess.stdout) {
    readStream(serverProcess.stdout).catch(() => {});
  }
  if (serverProcess.stderr) {
    readStream(serverProcess.stderr).catch(() => {});
  }

  // Compiled binaries may need several seconds for their first startup.
  // Poll readiness instead of relying on a fixed delay.
  const startupDeadline = Date.now() + 15_000;
  let lastStartupError: unknown = null;
  while (Date.now() < startupDeadline) {
    try {
      const testResponse = await fetch(`http://localhost:${TEST_PORT}/`);
      if (testResponse.status === 404) {
        console.log(
          `  ${COLORS.dim}[DEBUG] Server responded normally (404 expected for /)${COLORS.reset}`,
        );
      } else {
        console.log(
          `  ${COLORS.dim}[DEBUG] Server responded normally (${testResponse.status})${COLORS.reset}`,
        );
      }
      return;
    } catch (error) {
      lastStartupError = error;
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
  }

  throw new Error(
    `Server failed to start: ${String(lastStartupError)}\nStartup log:\n${serverStartupLog}`,
  );
}

/**
 * Stop server process
 */
export async function stopServer(): Promise<void> {
  if (serverProcess) {
    try {
      serverProcess.kill("SIGTERM");
    } catch {
      // Ignore error
    }
    serverProcess = null;
  }
}

/**
 * Kill process occupying the port
 */
export async function killProcessOnPort(port: number): Promise<void> {
  try {
    let pids: number[] = [];

    if (process.platform === "win32") {
      const { stdout } = await runCommand("netstat", ["-ano"]);
      const output = stdout;

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
      const { stdout, code } = await runCommand("lsof", ["-ti", `:${port}`]);

      if (code === 0) {
        const pidsStr = stdout.trim().split("\n");
        pids = pidsStr.map((pid) => parseInt(pid)).filter((pid) => !isNaN(pid));
      }
    }

    if (pids.length > 0) {
      for (const pid of pids) {
        try {
          if (process.platform === "win32") {
            await runCommand("taskkill", ["/PID", pid.toString(), "/F"]);
          } else {
            await runCommand("kill", ["-9", pid.toString()]);
          }
        } catch {
          // Ignore termination failures
        }
      }

      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  } catch {
    // Ignore port check errors
  }
}

/**
 * Test HTTP request
 */
export async function testHttpRequest(
  url: string,
  expectedStatus: number,
  options: {
    expectedContentType?: string;
    expectHtml?: boolean;
    method?: "GET" | "POST" | "PUT" | "DELETE";
    body?: string;
    headers?: Record<string, string>;
  } = {},
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

  // Check Content-Type (if expected value is specified)
  if (expectedContentType) {
    const contentType = response.headers.get("content-type");
    assertExists(contentType?.includes(expectedContentType));
  }

  // For HTML responses, check if they contain HTML tags
  if (expectHtml && expectedStatus === 200) {
    assertExists(text.includes("<html") || text.includes("<!DOCTYPE"));
  }
}

export function printSection(title: string) {
  console.log(
    `\n${COLORS.cyan}${COLORS.bright}╔════════════════════════════════════════════╗${COLORS.reset}`,
  );
  console.log(
    `${COLORS.cyan}${COLORS.bright}║   ${title.padEnd(38)}║${COLORS.reset}`,
  );
  console.log(
    `${COLORS.cyan}${COLORS.bright}╚════════════════════════════════════════════╝${COLORS.reset}`,
  );
}

export function printSubsection(title: string) {
  console.log(`\n${COLORS.yellow}${COLORS.bright}▶ ${title}${COLORS.reset}`);
  console.log(`${COLORS.dim}─${"─".repeat(50)}${COLORS.reset}`);
}

export function printTestResult(name: string, passed: boolean, duration?: number) {
  const symbol = passed ? "✓" : "✗";
  const color = passed ? COLORS.green : COLORS.red;
  const durationStr = duration
    ? ` ${COLORS.dim}(${duration}ms)${COLORS.reset}`
    : "";
  console.log(`  ${color}${symbol} ${name}${durationStr}${COLORS.reset}`);
}

// ============================================
// Main test execution function
// ============================================

async function runE2ETests(): Promise<void> {
  const tests: Array<{
    name: string;
    fn: () => Promise<void>;
  }> = [];

  // Test 1: Verify and start server (must be first!)
  tests.push({
    name: "binary build - compile and start server",
    fn: async () => {
      const startTime = Date.now();

      printSection("Environment Setup");

      // Kill old processes and verify binary exists
      const binaryExists = await cleanupBinary();
      if (!binaryExists) {
        throw new Error(
          `Binary not found at ${getBinaryPath()}. Please run './tsp.sh build:tspserver' first.`,
        );
      }
      console.log(`  ${COLORS.green}✓ Old processes cleaned up${COLORS.reset}`);
      const binaryPath = getBinaryPath();
      const stat = statSync(binaryPath);
      const sizeMB = (stat.size / 1024 / 1024).toFixed(2);
      console.log(
        `  ${COLORS.green}✓ Binary found (${sizeMB} MB)${COLORS.reset}`,
      );

      await killProcessOnPort(TEST_PORT);
      console.log(`  ${COLORS.green}✓ Port ${TEST_PORT} cleaned${COLORS.reset}`);

      await startServer();
      console.log(
        `  ${COLORS.green}✓ Server started on port ${TEST_PORT}${COLORS.reset}`,
      );

      const duration = Date.now() - startTime;
      printTestResult("Compile and start server", true, duration);
    },
  });

  // Import tests from e2e modules (run after server starts)
  tests.push(...getHttpTests());
  tests.push(...getSecurityTests());
  tests.push(...getInjectionTests());
  tests.push(...getHotReloadTests());
  tests.push(...getSessionTests());
  tests.push(...getUploadTests());
  tests.push(...getMysqlTests());
  tests.push(...getRedisTests());
  // tests.push(...getRedisSessionTests()); // Skipped - manual verification needed
  tests.push(...getExcelJsTests());
  tests.push(...getLdapTests());
  tests.push(...getConfigTests());
  tests.push(...getValidationTests());
  tests.push(...getFragmentTests());


  // Test 16: Cleanup resources
  tests.push({
    name: "binary build - Stop server",
    fn: async () => {
      const startTime = Date.now();

      printSubsection("Cleanup Resources");

      await stopServer();
      console.log(`  ${COLORS.green}✓ Server stopped${COLORS.reset}`);

      const binaryPath = getBinaryPath();
      console.log(
        `\n${COLORS.cyan}💡 Hint: Binary file: ${binaryPath}${COLORS.reset}`,
      );
      console.log(
        `${COLORS.cyan}💡 Manual run: ${binaryPath} --root ./www --port 9000${COLORS.reset}`,
      );

      const duration = Date.now() - startTime;
      printTestResult("Cleanup resources", true, duration);

      console.log(
        `\n${COLORS.cyan}${COLORS.bright}╔════════════════════════════════════════════╗${COLORS.reset}`,
      );
      console.log(
        `${COLORS.cyan}${COLORS.bright}║   E2E Tests Completed                      ║${COLORS.reset}`,
      );
      console.log(
        `${COLORS.cyan}${COLORS.bright}╚════════════════════════════════════════════╝${COLORS.reset}`,
      );
    },
  });

  // Run all tests
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
      console.error(
        `    ${error instanceof Error ? error.message : String(error)}\n`,
      );
    }
  }

  // Print summary
  console.log(
    `\n${COLORS.cyan}${COLORS.bright}╔════════════════════════════════════════════╗${COLORS.reset}`,
  );
  console.log(
    `${COLORS.cyan}${COLORS.bright}║   Test Summary                              ║${COLORS.reset}`,
  );
  console.log(
    `${COLORS.cyan}${COLORS.bright}╚════════════════════════════════════════════╝${COLORS.reset}`,
  );

  console.log(`\n${COLORS.bright}Total tests: ${tests.length}${COLORS.reset}`);
  console.log(
    `${COLORS.green}${COLORS.bright}Passed: ${passedTests}${COLORS.reset}`,
  );
  if (failedTests > 0) {
    console.log(
      `${COLORS.red}${COLORS.bright}Failed: ${failedTests}${COLORS.reset}`,
    );
  }

  if (failedTests === 0) {
    console.log(
      `\n${COLORS.green}${COLORS.bright}All E2E tests passed!${COLORS.reset}`,
    );
  } else {
    console.log(
      `\n${COLORS.red}${COLORS.bright}❌ Some tests failed!${COLORS.reset}`,
    );
    process.exit(1);
  }
}

// Startup banner
console.log(
  `\n${COLORS.cyan}${COLORS.bright}╔════════════════════════════════════════════╗${COLORS.reset}`,
);
console.log(
  `${COLORS.cyan}${COLORS.bright}║   TSP E2E Tests                            ║${COLORS.reset}`,
);
console.log(
  `${COLORS.cyan}${COLORS.bright}╚════════════════════════════════════════════╝${COLORS.reset}`,
);

// Run tests
if (import.meta.main) {
  await runE2ETests();
}
