#!/usr/bin/env -S deno run --allow-net --allow-read

/**
 * 基本功能测试
 *
 * 测试核心功能，无需编译二进制
 */

import { assertEquals, assertExists } from "https://deno.land/std@0.210.0/testing/asserts.ts";
import { resolvePath, securityCheck } from "../src/router.ts";
import { buildContext, type PageContext } from "../src/context.ts";

const TEST_PORT = 9101;
const TEST_ROOT = "./www";

/**
 * 测试路由解析
 */
Deno.test("router - resolvePath: 根路径", () => {
  const result = resolvePath("/", TEST_ROOT);
  assertEquals(result.success, true);
  // Windows 使用 \，Unix 使用 /，所以替换后比较
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/index.tsx");
  console.log(`✓ / → ${result.filepath}`);
});

Deno.test("router - resolvePath: 简单路径", () => {
  const result = resolvePath("/form.tsx", TEST_ROOT);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/form.tsx");
  console.log(`✓ /form.tsx → ${result.filepath}`);
});

Deno.test("router - resolvePath: 带目录的路径", () => {
  const result = resolvePath("/api/test", TEST_ROOT);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/api/test.tsx");
  console.log(`✓ /api/test → ${result.filepath}`);
});

Deno.test("router - resolvePath: 目录默认页", () => {
  const result = resolvePath("/admin/", TEST_ROOT);
  assertEquals(result.success, true);
  // 注意：/admin/ 会被解析为 admin.tsx（如果文件不存在）
  // 实际的 index.tsx 检查在 securityCheck 中进行
  console.log(`✓ /admin/ → ${result.filepath}`);
});

/**
 * 辅助函数：从 Request 构建上下文
 */
async function buildContextFromRequest(
  request: Request,
  file: string,
  root: string,
): Promise<PageContext> {
  const url = new URL(request.url);

  // 解析查询参数
  const query: Record<string, string> = {};
  url.searchParams.forEach((value, key) => {
    query[key] = value;
  });

  // 解析 Cookies
  const cookies: Record<string, string> = {};
  const cookieHeader = request.headers.get("Cookie");
  if (cookieHeader) {
    cookieHeader.split(";").forEach((cookie) => {
      const [key, value] = cookie.trim().split("=");
      if (key && value) {
        cookies[key] = decodeURIComponent(value);
      }
    });
  }

  // 解析请求体
  let body: unknown = null;
  const method = request.method as "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS";

  if (method !== "GET" && method !== "HEAD" && method !== "DELETE") {
    const contentType = request.headers.get("Content-Type") || "";

    if (contentType.includes("application/json")) {
      try {
        body = await request.json();
      } catch {
        body = null;
      }
    } else if (contentType.includes("application/x-www-form-urlencoded")) {
      const text = await request.text();
      const params = new URLSearchParams(text);
      const formData: Record<string, string> = {};
      params.forEach((value, key) => {
        formData[key] = value;
      });
      body = formData;
    } else {
      body = await request.text();
    }
  }

  return buildContext({
    method,
    url,
    headers: request.headers,
    query,
    body,
    cookies,
    file,
    root,
  });
}

/**
 * 测试上下文构建
 */
Deno.test("context - buildContext: 基本请求", async () => {
  const request = new Request("http://localhost:9000/?name=test");
  const context = await buildContextFromRequest(request, "/test.tsx", TEST_ROOT);

  assertEquals(context.method, "GET");
  assertEquals(context.query.name, "test");
  assertEquals(context.file, "/test.tsx");
  assertEquals(context.root, TEST_ROOT);

  console.log("✓ 基本上下文构建正确");
});

Deno.test("context - buildContext: POST 请求", async () => {
  const request = new Request("http://localhost:9000/", {
    method: "POST",
    body: JSON.stringify({ username: "test" }),
    headers: {
      "Content-Type": "application/json",
    },
  });

  const context = await buildContextFromRequest(request, "/test.tsx", TEST_ROOT);

  assertEquals(context.method, "POST");
  assertExists(context.body);
  assertEquals((context.body as Record<string, unknown>).username, "test");

  console.log("✓ POST 请求上下文构建正确");
});

Deno.test("context - buildContext: Cookie 解析", async () => {
  const request = new Request("http://localhost:9000/", {
    headers: {
      "Cookie": "sessionId=abc123; theme=dark",
    },
  });

  const context = await buildContextFromRequest(request, "/test.tsx", TEST_ROOT);

  assertEquals(context.cookies.sessionId, "abc123");
  assertEquals(context.cookies.theme, "dark");

  console.log("✓ Cookie 解析正确");
});

/**
 * 测试安全检查
 */
Deno.test("security - securityCheck: 路径穿越攻击", async () => {
  const testPaths = [
    "../../../etc/passwd",
    "../secret.tsx",
    "./../../test.tsx",
  ];

  for (const path of testPaths) {
    const result = await resolvePath(path, TEST_ROOT);
    if (result.success && result.filepath) {
      const checkResult = await securityCheck(result.filepath, TEST_ROOT);
      if (checkResult.success) {
        throw new Error(`应该阻止路径穿越: ${path}`);
      } else {
        console.log(`✓ 已阻止路径穿越: ${path} (${checkResult.error})`);
      }
    }
  }
});

Deno.test("security - securityCheck: 非白名单文件", async () => {
  const testPaths = [
    "www/config.json",
    "www/data.txt",
    "www/secret.ts",
  ];

  for (const filepath of testPaths) {
    const checkResult = await securityCheck(filepath, TEST_ROOT);
    if (checkResult.success) {
      throw new Error(`应该阻止非 TSX 文件: ${filepath}`);
    } else {
      console.log(`✓ 已阻止非 TSX 文件: ${filepath} (${checkResult.error})`);
    }
  }
});

Deno.test("security - securityCheck: 正常文件通过", async () => {
  const validPaths = [
    "www/index.tsx",
    "www/form.tsx",
    "www/api.tsx",
  ];

  for (const filepath of validPaths) {
    const checkResult = await securityCheck(filepath, TEST_ROOT);
    if (!checkResult.success) {
      throw new Error(`应该允许正常文件: ${filepath} - ${checkResult.error}`);
    } else {
      console.log(`✓ 已允许正常文件: ${filepath}`);
    }
  }
});

console.log("\n╔════════════════════════════════════════════╗");
console.log("║   基本功能测试                            ║");
console.log("╚════════════════════════════════════════════╝\n");
