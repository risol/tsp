/**
 * 端到端测试：基础功能测试
 * 测试 TSP-FPM 的核心功能
 */

import { assertEquals, assertStringIncludes } from "@std/assert";

// 启动测试服务器
async function startTestServer() {
  const port = 9100;
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
  path: string,
  options: RequestInit = {}
): Promise<Response> {
  const url = `http://localhost:9100${path}`;
  return await fetch(url, options);
}

// 清理函数
async function cleanup(child: Deno.ChildProcess) {
  child.kill("SIGTERM");
  try {
    // 取消流但不等待
    void child.stdout.cancel();
    void child.stderr.cancel();
    await child.status;
  } catch (_error) {
    // 忽略错误
  }
}

Deno.test("E2E: 首页应该正确渲染", async (t) => {
  const { child, port } = await startTestServer();

  try {
    await t.step("应该返回 200 状态码", async () => {
      const response = await makeRequest("/");
      assertEquals(response.status, 200);
      // 消费响应体以避免资源泄漏
      await response.text();
    });

    await t.step("应该返回 HTML 内容", async () => {
      const response = await makeRequest("/");
      const contentType = response.headers.get("content-type");
      assertEquals(contentType?.includes("text/html"), true);
      // 消费响应体
      await response.text();
    });

    await t.step("应该包含标题文本", async () => {
      const response = await makeRequest("/");
      const text = await response.text();
      assertStringIncludes(text, "TSP-FPM");
    });

    await t.step("应该显示默认的问候语", async () => {
      const response = await makeRequest("/");
      const text = await response.text();
      // 检查 HTML 内容中是否包含 Hello World
      assertStringIncludes(text, "Hello");
      assertStringIncludes(text, "World");
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 查询参数处理", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("应该正确解析查询参数", async () => {
      const response = await makeRequest("/?name=Claude&lang=zh");
      const text = await response.text();
      // 检查是否包含 Claude
      assertStringIncludes(text, "Claude");
    });

    await t.step("应该显示查询参数信息", async () => {
      const response = await makeRequest("/?test=value&number=123");
      const text = await response.text();
      assertStringIncludes(text, "test");
      assertStringIncludes(text, "value");
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: POST 请求处理", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("应该处理 form-urlencoded POST 请求", async () => {
      const formData = new URLSearchParams();
      formData.append("username", "testuser");
      formData.append("email", "test@example.com");

      const response = await makeRequest("/", {
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
      const response = await makeRequest("/", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ username: "jsonuser", age: 25 }),
      });

      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "jsonuser");
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 表单页面测试", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("应该显示表单页面", async () => {
      const response = await makeRequest("/form");
      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "表单提交示例");
    });

    await t.step("应该处理表单提交", async () => {
      const formData = new URLSearchParams();
      formData.append("username", "formuser");
      formData.append("email", "form@example.com");
      formData.append("age", "30");
      formData.append("bio", "Test bio");

      const response = await makeRequest("/form", {
        method: "POST",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
        },
        body: formData,
      });

      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "提交成功");
      assertStringIncludes(text, "formuser");
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: API 信息页面测试", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("应该显示 API 信息", async () => {
      const response = await makeRequest("/api");
      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "API 请求信息");
    });

    await t.step("应该显示请求方法", async () => {
      const response = await makeRequest("/api");
      const text = await response.text();
      assertStringIncludes(text, "GET");
    });

    await t.step("应该显示请求头信息", async () => {
      const response = await makeRequest("/api", {
        headers: {
          "X-Custom-Header": "test-value",
        },
      });
      const text = await response.text();
      // headers 在页面中以小写显示
      assertStringIncludes(text, "x-custom-header");
      assertStringIncludes(text, "test-value");
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: Cookies 处理", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("应该解析和显示 cookies", async () => {
      const response = await makeRequest("/", {
        headers: {
          "Cookie": "sessionId=abc123; userId=user1",
        },
      });

      const text = await response.text();
      assertStringIncludes(text, "sessionId");
      assertStringIncludes(text, "abc123");
    });
  } finally {
    await cleanup(child);
  }
});
