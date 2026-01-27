/**
 * 端到端测试：自定义 Response 对象测试
 * 测试页面返回自定义 Response 对象的功能
 */

import { assertEquals, assertStringIncludes } from "@std/assert";

// 启动测试服务器
async function startTestServer() {
  const port = 9105;
  const root = "./www";

  const process = new Deno.Command("deno", {
    args: [
      "run",
      "--allow-net",
      "--allow-read",
      "src/main.ts",
      "--root",
      root,
      "--port",
      port.toString(),
    ],
    stdout: "piped",
    stderr: "piped",
  });

  const child = process.spawn();

  // 等待服务器启动
  await new Promise((resolve) => setTimeout(resolve, 2000));

  return { child, port };
}

// 辅助函数：发送 HTTP 请求
async function makeRequest(
  port: number,
  path: string,
  options: RequestInit = {}
): Promise<Response> {
  const url = `http://localhost:${port}${path}`;
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

Deno.test("E2E: 自定义 Response 对象 - JSON API", async (t) => {
  const { child, port } = await startTestServer();

  try {
    // 创建一个返回 JSON 的页面
    const apiPagePath = "./www/json-api.tsx";
    await Deno.writeTextFile(
      apiPagePath,
      `
      import type { PageContext } from "../src/cache.ts";

      export default async function (context: PageContext) {
        const { method } = context;

        if (method === "GET") {
          return new Response(
            JSON.stringify({ message: "Hello", status: "ok" }),
            {
              status: 200,
              headers: { "Content-Type": "application/json" },
            }
          );
        }

        return new Response("Method not allowed", {
          status: 405,
          headers: { "Content-Type": "text/plain" },
        });
      }
      `
    );

    try {
      await t.step("应该返回 JSON 响应", async () => {
        const response = await makeRequest(port, "/json-api");
        assertEquals(response.status, 200);
        const contentType = response.headers.get("content-type");
        assertEquals(contentType, "application/json");

        const data = await response.json();
        assertEquals(data.message, "Hello");
        assertEquals(data.status, "ok");
      });

      await t.step("应该返回正确的 Content-Type", async () => {
        const response = await makeRequest(port, "/json-api");
        const contentType = response.headers.get("content-type");
        assertEquals(contentType?.includes("application/json"), true);
        // 消费响应体
        await response.text();
      });
    } finally {
      await Deno.remove(apiPagePath);
    }
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 自定义 Response 对象 - 不同的状态码", async (t) => {
  const { child, port } = await startTestServer();

  try {
    const statusPagePath = "./www/status-test.tsx";
    await Deno.writeTextFile(
      statusPagePath,
      `
      import type { PageContext } from "../src/cache.ts";

      export default async function (context: PageContext) {
        const { query } = context;
        const status = parseInt(query.status || "200", 10);

        return new Response(\`Status code: \${status}\`, {
          status,
          headers: { "Content-Type": "text/plain" },
        });
      }
      `
    );

    try {
      await t.step("应该支持 200 状态码", async () => {
        const response = await makeRequest(port, "/status-test?status=200");
        assertEquals(response.status, 200);
        const text = await response.text();
        assertStringIncludes(text, "Status code: 200");
      });

      await t.step("应该支持 201 Created 状态码", async () => {
        const response = await makeRequest(port, "/status-test?status=201");
        assertEquals(response.status, 201);
        // 消费响应体
        await response.text();
      });

      await t.step("应该支持 404 Not Found 状态码", async () => {
        const response = await makeRequest(port, "/status-test?status=404");
        assertEquals(response.status, 404);
        // 消费响应体
        await response.text();
      });

      await t.step("应该支持自定义状态码", async () => {
        const response = await makeRequest(port, "/status-test?status=418");
        assertEquals(response.status, 418); // I'm a teapot
        // 消费响应体
        await response.text();
      });
    } finally {
      await Deno.remove(statusPagePath);
    }
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 自定义 Response 对象 - 自定义 Headers", async (t) => {
  const { child, port } = await startTestServer();

  try {
    const headerPagePath = "./www/headers-test.tsx";
    await Deno.writeTextFile(
      headerPagePath,
      `
      import type { PageContext } from "../src/cache.ts";

      export default async function (_context: PageContext) {
        return new Response("Headers test", {
          status: 200,
          headers: {
            "Content-Type": "text/plain",
            "X-Custom-Header": "custom-value",
            "Cache-Control": "no-cache",
            "X-Request-Id": "12345",
          },
        });
      }
      `
    );

    try {
      await t.step("应该包含自定义 header", async () => {
        const response = await makeRequest(port, "/headers-test");
        const customHeader = response.headers.get("x-custom-header");
        assertEquals(customHeader, "custom-value");
        // 消费响应体
        await response.text();
      });

      await t.step("应该包含 Cache-Control header", async () => {
        const response = await makeRequest(port, "/headers-test");
        const cacheControl = response.headers.get("cache-control");
        assertEquals(cacheControl, "no-cache");
        // 消费响应体
        await response.text();
      });

      await t.step("应该包含多个自定义 headers", async () => {
        const response = await makeRequest(port, "/headers-test");
        const customHeader = response.headers.get("x-custom-header");
        const requestId = response.headers.get("x-request-id");
        assertEquals(customHeader, "custom-value");
        assertEquals(requestId, "12345");
        // 消费响应体
        await response.text();
      });
    } finally {
      await Deno.remove(headerPagePath);
    }
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 自定义 Response 对象 - 不同内容类型", async (t) => {
  const { child, port } = await startTestServer();

  try {
    const contentPagePath = "./www/content-test.tsx";
    await Deno.writeTextFile(
      contentPagePath,
      `
      import type { PageContext } from "../src/cache.ts";

      export default async function (context: PageContext) {
        const { query } = context;
        const type = query.type || "html";

        switch (type) {
          case "json":
            return new Response(
              JSON.stringify({ data: "test" }),
              {
                status: 200,
                headers: { "Content-Type": "application/json" },
              }
            );

          case "text":
            return new Response("Plain text response", {
              status: 200,
              headers: { "Content-Type": "text/plain" },
            });

          case "xml":
            return new Response(
              '<?xml version="1.0"?><root>test</root>',
              {
                status: 200,
                headers: { "Content-Type": "application/xml" },
              }
            );

          default:
            return new Response("Unknown type", {
              status: 400,
              headers: { "Content-Type": "text/plain" },
            });
        }
      }
      `
    );

    try {
      await t.step("应该返回 JSON 内容", async () => {
        const response = await makeRequest(port, "/content-test?type=json");
        assertEquals(response.status, 200);
        const contentType = response.headers.get("content-type");
        assertEquals(contentType, "application/json");
        // 消费响应体
        await response.text();
      });

      await t.step("应该返回纯文本内容", async () => {
        const response = await makeRequest(port, "/content-test?type=text");
        assertEquals(response.status, 200);
        const contentType = response.headers.get("content-type");
        assertEquals(contentType, "text/plain");
        const text = await response.text();
        assertStringIncludes(text, "Plain text response");
      });

      await t.step("应该返回 XML 内容", async () => {
        const response = await makeRequest(port, "/content-test?type=xml");
        assertEquals(response.status, 200);
        const contentType = response.headers.get("content-type");
        assertEquals(contentType, "application/xml");
        // 消费响应体
        await response.text();
      });

      await t.step("应该处理未知类型", async () => {
        const response = await makeRequest(port, "/content-test?type=unknown");
        assertEquals(response.status, 400);
        // 消费响应体
        await response.text();
      });
    } finally {
      await Deno.remove(contentPagePath);
    }
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 自定义 Response 对象 - 响应体", async (t) => {
  const { child, port } = await startTestServer();

  try {
    const bodyPagePath = "./www/body-test.tsx";
    await Deno.writeTextFile(
      bodyPagePath,
      `
      import type { PageContext } from "../src/cache.ts";

      export default async function (context: PageContext) {
        const { query } = context;

        if (query.empty === "true") {
          return new Response(null, {
            status: 204,
            headers: {},
          });
        }

        return new Response("Response body content", {
          status: 200,
          headers: { "Content-Type": "text/plain" },
        });
      }
      `
    );

    try {
      await t.step("应该返回有内容的响应体", async () => {
        const response = await makeRequest(port, "/body-test");
        assertEquals(response.status, 200);
        const text = await response.text();
        assertStringIncludes(text, "Response body content");
      });

      await t.step("应该支持空响应体 (204 No Content)", async () => {
        const response = await makeRequest(port, "/body-test?empty=true");
        assertEquals(response.status, 204);
        const text = await response.text();
        assertEquals(text, "");
      });
    } finally {
      await Deno.remove(bodyPagePath);
    }
  } finally {
    await cleanup(child);
  }
});
