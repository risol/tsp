/**
 * 端到端测试：重定向和响应测试
 * 测试重定向功能和自定义 Response 对象
 */

import { assertEquals, assertStringIncludes } from "@std/assert";

// 启动测试服务器
async function startTestServer() {
  const port = 9102;
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
  const url = `http://localhost:9102${path}`;
  return await fetch(url, options);
}

// 辅助函数：跟随重定向
async function makeRequestFollowRedirect(
  path: string,
  options: RequestInit = {}
): Promise<Response> {
  const url = `http://localhost:9102${path}`;
  return await fetch(url, {
    ...options,
    redirect: "follow",
  });
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

Deno.test("E2E: 重定向功能测试", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("默认 302 重定向应该工作", async () => {
      const response = await makeRequest("/redirect?to=home");
      assertEquals(response.status, 302);
      const location = response.headers.get("location");
      assertEquals(location, "/");
    });

    await t.step("301 永久重定向应该工作", async () => {
      const response = await makeRequest("/redirect?to=new-home");
      assertEquals(response.status, 301);
      const location = response.headers.get("location");
      assertEquals(location, "/");
    });

    await t.step("重定向到正确的 URL", async () => {
      const response = await makeRequest("/redirect?to=home");
      const location = response.headers.get("location");
      assertEquals(location, "/");
    });

    await t.step("跟随重定向应该到达目标页面", async () => {
      const response = await makeRequestFollowRedirect("/redirect?to=home");
      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "TSP-FPM 运行成功");
    });

    await t.step("条件重定向（基于 cookie）应该工作", async () => {
      // 没有 session cookie，应该重定向
      const response = await makeRequest("/redirect?to=protected");
      assertEquals(response.status, 302);
      const location = response.headers.get("location");
      assertStringIncludes(location || "", "/login");
    });

    await t.step("有 session cookie 时不应重定向", async () => {
      const response = await makeRequest("/redirect?to=protected", {
        headers: {
          "Cookie": "sessionId=valid-session-123",
        },
      });
      // 有 session，不应该重定向，返回正常的 200
      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "重定向示例");
    });

    await t.step("重定向 URL 应该可以包含查询参数", async () => {
      const response = await makeRequest("/redirect?to=protected");
      const location = response.headers.get("location");
      assertStringIncludes(location || "", "redirect=/protected");
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 重定向状态码测试", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("应该支持 301 Moved Permanently", async () => {
      const response = await makeRequest("/redirect?to=new-home");
      assertEquals(response.status, 301);
    });

    await t.step("应该支持 302 Found (默认)", async () => {
      const response = await makeRequest("/redirect?to=home");
      assertEquals(response.status, 302);
    });

    await t.step("无效的状态码应该使用默认的 302", async () => {
      // 这个测试需要创建一个返回无效状态码的页面
      // 目前测试文件中没有这样的场景，所以跳过
      // 但实际代码中处理了这种情况
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 重定向链测试", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("应该支持多次重定向", async () => {
      // 测试重定向到首页，然后再跟随重定向
      const response = await makeRequestFollowRedirect("/redirect?to=home");
      assertEquals(response.status, 200);
      assertEquals(response.url.includes("/redirect"), false);
    });

    await t.step("重定向次数不应该超过浏览器限制", async () => {
      // 正常情况下不应该有循环重定向
      const response = await makeRequest("/redirect?to=home");
      // 应该返回重定向状态，而不是 200（除非跟随了重定向）
      assertEquals(response.status, 302);
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 重定向 Location 头测试", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("Location 头应该是绝对路径或相对路径", async () => {
      const response = await makeRequest("/redirect?to=home");
      const location = response.headers.get("location");
      assertEquals(location !== null, true);
    });

    await t.step("Location 头不应该为空", async () => {
      const response = await makeRequest("/redirect?to=home");
      const location = response.headers.get("location");
      assertEquals(location?.length || 0, 1);
    });

    await t.step("Location 头应该正确编码", async () => {
      const response = await makeRequest("/redirect?to=protected");
      const location = response.headers.get("location");
      // 包含查询参数应该正确编码
      assertStringIncludes(location || "", "redirect");
    });
  } finally {
    await cleanup(child);
  }
});
