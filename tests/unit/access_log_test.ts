/**
 * Access Log 模块单元测试
 * 测试 src/main.ts 中的日志记录功能
 */

import {
  assertEquals,
  assertExists,
  assertStringIncludes,
} from "https://deno.land/std@0.210.0/testing/asserts.ts";
import { type Config, logAccess } from "../../src/main.ts";

// 测试用的临时日志文件路径
const TEST_LOG_FILE = "./test_access.log";
const TEST_LOG_FILE_2 = "./test_access_2.log";

/**
 * 清理测试日志文件
 */
async function cleanupTestLog(filepath: string) {
  try {
    await Deno.remove(filepath);
  } catch {
    // 文件不存在，忽略
  }
}

Deno.test("access log - logAccess: 控制台输出（默认配置）", async () => {
  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    // 不设置 accessLogPath，应该输出到控制台
  };

  const request = new Request("http://localhost:9000/test/path?param=value", {
    method: "GET",
    headers: {
      "User-Agent": "TestAgent/1.0",
    },
  });

  const response = new Response("OK", { status: 200 });

  // 这个测试主要验证不会抛出错误
  await logAccess(request, response, config);

  // 如果没有配置 accessLogPath，日志应该输出到控制台
  // 我们无法直接测试控制台输出，但可以验证函数不抛出错误
  assertEquals(true, true);
});

Deno.test("access log - logAccess: 文件输出 - 基本请求", async () => {
  await cleanupTestLog(TEST_LOG_FILE);

  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    accessLogPath: TEST_LOG_FILE,
  };

  const request = new Request("http://localhost:9000/test/path?param=value", {
    method: "GET",
    headers: {
      "User-Agent": "TestAgent/1.0",
    },
  });

  const response = new Response("OK", { status: 200 });

  await logAccess(request, response, config);

  // 读取日志文件
  const logContent = await Deno.readTextFile(TEST_LOG_FILE);
  const logLines = logContent.trim().split("\n");

  // 验证日志内容
  assertEquals(logLines.length, 1);

  const logLine = logLines[0];
  assertStringIncludes(logLine, "GET");
  assertStringIncludes(logLine, "/test/path");
  assertStringIncludes(logLine, "200");
  assertStringIncludes(logLine, "TestAgent/1.0");

  // 验证 ISO 时间戳格式（包含 T 和 Z）
  assertStringIncludes(logLine, "T");
  assertStringIncludes(logLine, "Z");

  // 清理
  await cleanupTestLog(TEST_LOG_FILE);
});

Deno.test("access log - logAccess: 文件输出 - 多次追加", async () => {
  await cleanupTestLog(TEST_LOG_FILE_2);

  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    accessLogPath: TEST_LOG_FILE_2,
  };

  // 记录三条日志
  await logAccess(
    new Request("http://localhost:9000/page1", {
      method: "GET",
      headers: { "User-Agent": "Agent1" },
    }),
    new Response("OK", { status: 200 }),
    config,
  );

  await logAccess(
    new Request("http://localhost:9000/page2", {
      method: "POST",
      headers: { "User-Agent": "Agent2" },
    }),
    new Response("Created", { status: 201 }),
    config,
  );

  await logAccess(
    new Request("http://localhost:9000/page3", {
      method: "DELETE",
      headers: { "User-Agent": "Agent3" },
    }),
    new Response("Not Found", { status: 404 }),
    config,
  );

  // 读取日志文件
  const logContent = await Deno.readTextFile(TEST_LOG_FILE_2);
  const logLines = logContent.trim().split("\n");

  // 验证有三行日志
  assertEquals(logLines.length, 3);

  // 验证第一行
  assertStringIncludes(logLines[0], "GET");
  assertStringIncludes(logLines[0], "/page1");
  assertStringIncludes(logLines[0], "200");
  assertStringIncludes(logLines[0], "Agent1");

  // 验证第二行
  assertStringIncludes(logLines[1], "POST");
  assertStringIncludes(logLines[1], "/page2");
  assertStringIncludes(logLines[1], "201");
  assertStringIncludes(logLines[1], "Agent2");

  // 验证第三行
  assertStringIncludes(logLines[2], "DELETE");
  assertStringIncludes(logLines[2], "/page3");
  assertStringIncludes(logLines[2], "404");
  assertStringIncludes(logLines[2], "Agent3");

  // 清理
  await cleanupTestLog(TEST_LOG_FILE_2);
});

Deno.test("access log - logAccess: 没有 User-Agent", async () => {
  await cleanupTestLog(TEST_LOG_FILE);

  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    accessLogPath: TEST_LOG_FILE,
  };

  const request = new Request("http://localhost:9000/test", {
    method: "GET",
    // 不设置 User-Agent
  });

  const response = new Response("OK", { status: 200 });

  await logAccess(request, response, config);

  // 读取日志文件
  const logContent = await Deno.readTextFile(TEST_LOG_FILE);

  // 验证日志中包含 "-" 作为 User-Agent 占位符
  assertStringIncludes(logContent, '"-"');

  // 清理
  await cleanupTestLog(TEST_LOG_FILE);
});

Deno.test("access log - logAccess: 各种 HTTP 方法", async () => {
  await cleanupTestLog(TEST_LOG_FILE);

  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    accessLogPath: TEST_LOG_FILE,
  };

  const methods = [
    "GET",
    "POST",
    "PUT",
    "PATCH",
    "DELETE",
    "HEAD",
    "OPTIONS",
  ] as const;

  for (const method of methods) {
    await logAccess(
      new Request("http://localhost:9000/test", {
        method,
        headers: { "User-Agent": "Test" },
      }),
      new Response("OK", { status: 200 }),
      config,
    );
  }

  // 读取日志文件
  const logContent = await Deno.readTextFile(TEST_LOG_FILE);
  const logLines = logContent.trim().split("\n");

  // 验证所有方法都被记录
  assertEquals(logLines.length, methods.length);

  for (let i = 0; i < methods.length; i++) {
    assertStringIncludes(logLines[i], methods[i]);
  }

  // 清理
  await cleanupTestLog(TEST_LOG_FILE);
});

Deno.test("access log - logAccess: 各种状态码", async () => {
  await cleanupTestLog(TEST_LOG_FILE);

  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    accessLogPath: TEST_LOG_FILE,
  };

  const statusCodes = [200, 201, 301, 302, 400, 401, 403, 404, 500, 502, 503];

  for (const status of statusCodes) {
    await logAccess(
      new Request("http://localhost:9000/test", {
        method: "GET",
        headers: { "User-Agent": "Test" },
      }),
      new Response("Status", { status }),
      config,
    );
  }

  // 读取日志文件
  const logContent = await Deno.readTextFile(TEST_LOG_FILE);
  const logLines = logContent.trim().split("\n");

  // 验证所有状态码都被记录
  assertEquals(logLines.length, statusCodes.length);

  for (let i = 0; i < statusCodes.length; i++) {
    assertStringIncludes(logLines[i], statusCodes[i].toString());
  }

  // 清理
  await cleanupTestLog(TEST_LOG_FILE);
});

Deno.test("access log - logAccess: 路径包含查询参数", async () => {
  await cleanupTestLog(TEST_LOG_FILE);

  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    accessLogPath: TEST_LOG_FILE,
  };

  const request = new Request(
    "http://localhost:9000/search?q=test&page=1&sort=desc",
    {
      method: "GET",
      headers: { "User-Agent": "Test" },
    },
  );

  const response = new Response("OK", { status: 200 });

  await logAccess(request, response, config);

  // 读取日志文件
  const logContent = await Deno.readTextFile(TEST_LOG_FILE);

  // 验证路径被记录（注意：日志中只记录 pathname，不包含查询参数）
  // 根据当前实现，只记录 pathname
  assertStringIncludes(logContent, "/search");

  // 清理
  await cleanupTestLog(TEST_LOG_FILE);
});

Deno.test("access log - logAccess: 复杂 User-Agent", async () => {
  await cleanupTestLog(TEST_LOG_FILE);

  const config: Config = {
    root: "./www",
    port: 9000,
    dev: false,
    accessLogPath: TEST_LOG_FILE,
  };

  const userAgent =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

  const request = new Request("http://localhost:9000/test", {
    method: "GET",
    headers: { "User-Agent": userAgent },
  });

  const response = new Response("OK", { status: 200 });

  await logAccess(request, response, config);

  // 读取日志文件
  const logContent = await Deno.readTextFile(TEST_LOG_FILE);

  // 验证完整的 User-Agent 被记录
  assertStringIncludes(logContent, userAgent);

  // 清理
  await cleanupTestLog(TEST_LOG_FILE);
});

console.log("\n✓ Access Log 模块测试完成");
