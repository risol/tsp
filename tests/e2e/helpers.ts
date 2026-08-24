import {
  assertEquals,
  assertExists,
  assertStringIncludes,
} from "../unit/asserts.ts";
import { resolve } from "node:path";
import { cwd } from "node:process";

export { assertEquals, assertExists, assertStringIncludes };

export const TEST_PORT = 9001;
export const RELOAD_DELAY = 1000;

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

export function getTestRoot(): string {
  const currentDirectory = cwd();
  if (currentDirectory.endsWith("\\tests") || currentDirectory.endsWith("/tests")) {
    return resolve(currentDirectory, "test_www");
  }
  return resolve(currentDirectory, "tests", "test_www");
}

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

  const response = await fetch(url, { method, body, headers });
  assertEquals(response.status, expectedStatus);

  const text = await response.text();
  if (expectedContentType) {
    const contentType = response.headers.get("content-type");
    assertExists(contentType?.includes(expectedContentType));
  }
  if (expectHtml && expectedStatus === 200) {
    assertExists(text.includes("<html") || text.includes("<!DOCTYPE"));
  }
}
