/**
 * 端到端测试：路由和安全测试
 * 测试 URL 路由映射和安全防护功能
 */

import { assertEquals, assertStringIncludes } from "@std/assert";

// 启动测试服务器
async function startTestServer() {
  const port = 9101;
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
  const url = `http://localhost:9101${path}`;
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

Deno.test("E2E: 路由映射测试", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("根路径应该映射到 index.tsx", async () => {
      const response = await makeRequest("/");
      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "TSP-FPM 运行成功");
    });

    await t.step("显式指定 .tsx 扩展名应该工作", async () => {
      const response = await makeRequest("/index.tsx");
      assertEquals(response.status, 200);
    });

    await t.step("不指定扩展名应该自动添加 .tsx", async () => {
      const response = await makeRequest("/form");
      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "表单提交示例");
    });

    await t.step("API 路由应该工作", async () => {
      const response = await makeRequest("/api");
      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "API 请求信息");
    });

    await t.step("重定向页面路由应该工作", async () => {
      const response = await makeRequest("/redirect");
      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "重定向示例");
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 404 错误处理", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("不存在的页面应该返回 404", async () => {
      const response = await makeRequest("/nonexistent-page");
      assertEquals(response.status, 404);
    });

    await t.step("不存在的 .tsx 文件应该返回 404", async () => {
      const response = await makeRequest("/notfound.tsx");
      assertEquals(response.status, 404);
    });

    await t.step("404 页面应该有文本内容", async () => {
      const response = await makeRequest("/this-page-does-not-exist");
      const text = await response.text();
      assertStringIncludes(text.toLowerCase(), "not found");
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 安全防护测试", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("应该拒绝路径穿越攻击 (../)", async () => {
      const response = await makeRequest("/../../etc/passwd");
      assertEquals(response.status, 403);
    });

    await t.step("应该拒绝编码的路径穿越攻击", async () => {
      const response = await makeRequest("/%2e%2e/%2e%2e/etc/passwd");
      // 应该返回 404 或 403
      const status = response.status;
      assertEquals([403, 404].includes(status), true);
    });

    await t.step("应该拒绝访问非 .tsx 文件", async () => {
      const response = await makeRequest("/deno.json");
      assertEquals(response.status, 403);
    });

    await t.step("应该拒绝访问系统文件", async () => {
      const response = await makeRequest("/../../../deno.json");
      assertEquals(response.status, 403);
    });

    await t.step("应该拒绝绝对路径访问", async () => {
      const response = await makeRequest("/C:/Windows/System32/config");
      // 应该被安全检查拦截
      const status = response.status;
      assertEquals([403, 404].includes(status), true);
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: URL 编码处理", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("应该正确解码 URL 编码的查询参数", async () => {
      const response = await makeRequest("/?name=%E4%B8%AD%E6%96%87%E5%90%8D%E5%AD%97");
      const text = await response.text();
      assertStringIncludes(text, "中文字名");
    });

    await t.step("应该处理特殊字符", async () => {
      const response = await makeRequest("/?name=Test%20User%40Email");
      const text = await response.text();
      assertStringIncludes(text, "Test User@Email");
    });

    await t.step("应该处理空格编码", async () => {
      const response = await makeRequest("/?name=hello+world");
      const text = await response.text();
      assertStringIncludes(text, "hello world");
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 大小写敏感性测试", async (t) => {
  const { child } = await startTestServer();

  try {
    await t.step("路径应该保持大小写（但在 Windows 上可能不敏感）", async () => {
      const response = await makeRequest("/Form");
      // 在 Windows 上文件系统不区分大小写
      const status = response.status;
      assertEquals([200, 404].includes(status), true);
    });

    await t.step("查询参数应该区分大小写", async () => {
      const response = await makeRequest("/?Name=Test");
      const text = await response.text();
      // 查询参数区分大小写，所以 Name 不等于 name
      assertStringIncludes(text, "Hello World");
    });
  } finally {
    await cleanup(child);
  }
});
