/**
 * 端到端测试：基础功能测试
 * 测试 TSP-FPM 的核心功能
 */

import { assertEquals, assertStringIncludes } from "@std/assert";

const TEST_ROOT = "./tests/tmp";
const TEST_PORT = 9100;

// 辅助函数：创建测试页面
async function setupTestPages() {
  // 确保 tmp 目录存在
  await Deno.mkdir(TEST_ROOT, { recursive: true });

  // 创建首页测试页面
  await Deno.writeTextFile(
    `${TEST_ROOT}/index.tsx`,
    `
import type { PageContext } from "../../src/cache.ts";

export default async function (context: PageContext) {
  const { query } = context;
  const name = query.name ?? "World";

  return (
    <>
      <html lang="zh-CN">
      <head>
        <meta charset="UTF-8" />
        <title>Test Page</title>
      </head>
      <body>
        <h1>TSP-FPM Test</h1>
        <p>Hello <strong>{name}</strong>!</p>
        <p>Query: {JSON.stringify(query)}</p>
      </body>
      </html>
    </>
  );
}
    `
  );

  // 创建表单测试页面
  await Deno.writeTextFile(
    `${TEST_ROOT}/form.tsx`,
    `
import type { PageContext } from "../../src/cache.ts";

export default async function (context: PageContext) {
  const { method, body } = context;

  const result = method === "POST" && body
    ? <div class="result">Received: {JSON.stringify(body)}</div>
    : null;

  return (
    <html>
      <head>
        <meta charset="UTF-8" />
        <title>Form Test</title>
      </head>
      <body>
        <h1>Form Test</h1>
        {result}
        <form method="POST">
          <input type="text" name="username" />
          <button type="submit">Submit</button>
        </form>
      </body>
    </html>
  );
}
    `
  );

  // 创建 API 测试页面
  await Deno.writeTextFile(
    `${TEST_ROOT}/api.tsx`,
    `
import type { PageContext } from "../../src/cache.ts";

export default async function (context: PageContext) {
  const { headers } = context;

  const headersObj: Record<string, string> = {};
  for (const [key, value] of headers.entries()) {
    headersObj[key] = value;
  }

  return new Response(
    JSON.stringify({ headers: headersObj }),
    {
      status: 200,
      headers: { "Content-Type": "application/json" },
    }
  );
}
    `
  );
}

// 辅助函数：清理测试页面
async function cleanupTestPages() {
  try {
    await Deno.remove(`${TEST_ROOT}/index.tsx`);
    await Deno.remove(`${TEST_ROOT}/form.tsx`);
    await Deno.remove(`${TEST_ROOT}/api.tsx`);
  } catch (_error) {
    // 忽略错误
  }
}

// 启动测试服务器
async function startTestServer() {
  const process = new Deno.Command("deno", {
    args: [
      "run",
      "--allow-net",
      "--allow-read",
      "src/main.ts",
      "--root",
      TEST_ROOT,
      "--port",
      TEST_PORT.toString(),
    ],
    stdout: "piped",
    stderr: "piped",
  });

  const child = process.spawn();

  // 等待服务器启动
  await new Promise((resolve) => setTimeout(resolve, 2000));

  return { child, port: TEST_PORT };
}

// 辅助函数：发送 HTTP 请求
async function makeRequest(
  path: string,
  options: RequestInit = {}
): Promise<Response> {
  const url = `http://localhost:${TEST_PORT}${path}`;
  return await fetch(url, options);
}

// 清理函数
async function cleanup(child: Deno.ChildProcess) {
  child.kill("SIGTERM");
  try {
    void child.stdout.cancel();
    void child.stderr.cancel();
    await child.status;
  } catch (_error) {
    // 忽略错误
  }
}

Deno.test("E2E: 基础功能测试", async (t) => {
  // 设置测试页面
  await setupTestPages();

  const { child } = await startTestServer();

  try {
    await t.step("应该返回 200 状态码", async () => {
      const response = await makeRequest("/");
      assertEquals(response.status, 200);
      await response.text();
    });

    await t.step("应该返回 HTML 内容", async () => {
      const response = await makeRequest("/");
      const contentType = response.headers.get("content-type");
      assertEquals(contentType?.includes("text/html"), true);
      await response.text();
    });

    await t.step("应该包含标题文本", async () => {
      const response = await makeRequest("/");
      const text = await response.text();
      assertStringIncludes(text, "TSP-FPM Test");
    });

    await t.step("应该显示默认的问候语", async () => {
      const response = await makeRequest("/");
      const text = await response.text();
      assertStringIncludes(text, "Hello");
      assertStringIncludes(text, "World");
    });

    await t.step("应该正确解析查询参数", async () => {
      const response = await makeRequest("/?name=Claude");
      const text = await response.text();
      assertStringIncludes(text, "Claude");
    });

    await t.step("应该处理 form-urlencoded POST 请求", async () => {
      const formData = new URLSearchParams();
      formData.append("username", "testuser");

      const response = await makeRequest("/form", {
        method: "POST",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
        },
        body: formData,
      });

      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "testuser");
    });

    await t.step("应该处理 JSON POST 请求", async () => {
      const response = await makeRequest("/form", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ username: "jsonuser" }),
      });

      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "jsonuser");
    });

    await t.step("应该解析 Cookies", async () => {
      const response = await makeRequest("/", {
        headers: {
          "Cookie": "sessionId=abc123; userId=user1",
        },
      });

      assertEquals(response.status, 200);
      await response.text();
    });

    await t.step("应该返回 JSON API 响应", async () => {
      const response = await makeRequest("/api");
      assertEquals(response.status, 200);

      const contentType = response.headers.get("content-type");
      assertEquals(contentType, "application/json");

      const data = await response.json();
      assertEquals(typeof data.headers, "object");
    });
  } finally {
    await cleanup(child);
    await cleanupTestPages();
  }
});
