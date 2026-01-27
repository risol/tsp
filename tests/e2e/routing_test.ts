/**
 * 端到端测试：路由和安全测试
 * 测试 URL 路由映射和安全防护功能
 */

import { assertEquals, assertStringIncludes } from "@std/assert";

const TEST_ROOT = "./tests/tmp";
const TEST_PORT = 9101;

// 确保测试目录存在
async function ensureTestDir() {
  await Deno.mkdir(TEST_ROOT, { recursive: true });
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

Deno.test("E2E: 路由和安全测试", async (t) => {
  await ensureTestDir();

  const { child } = await startTestServer();

  try {
    // 创建一个简单的 index.tsx
    await Deno.writeTextFile(
      `${TEST_ROOT}/index.tsx`,
      `
import type { PageContext } from "../../src/cache.ts";
export default async function (_context: PageContext) {
  return <html><body><h1>Index Page</h1></body></html>;
}
      `
    );

    await t.step("根路径应该映射到 index.tsx", async () => {
      const response = await makeRequest("/");
      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "Index Page");
    });

    await t.step("显式指定 .tsx 扩展名应该工作", async () => {
      const response = await makeRequest("/index.tsx");
      assertEquals(response.status, 200);
      await response.text();
    });

    await t.step("不指定扩展名应该自动添加 .tsx", async () => {
      const response = await makeRequest("/index");
      assertEquals(response.status, 200);
      await response.text();
    });

    // 清理 index.tsx
    await Deno.remove(`${TEST_ROOT}/index.tsx`);

    await t.step("不存在的页面应该返回 404", async () => {
      const response = await makeRequest("/nonexistent-page");
      assertEquals(response.status, 404);
      await response.text();
    });

    await t.step("不存在的 .tsx 文件应该返回 404", async () => {
      const response = await makeRequest("/notfound.tsx");
      assertEquals(response.status, 404);
      await response.text();
    });

    await t.step("应该拒绝路径穿越攻击 (../)", async () => {
      const response = await makeRequest("/../../etc/passwd");
      assertEquals(response.status, 403);
      await response.text();
    });

    await t.step("应该拒绝编码的路径穿越攻击", async () => {
      const response = await makeRequest("/%2e%2e/%2e%2e/etc/passwd");
      // 应该返回 403 或 404
      const status = response.status;
      assertEquals([403, 404].includes(status), true);
      await response.text();
    });

    await t.step("应该拒绝访问非 .tsx 文件", async () => {
      // 创建一个 .txt 文件
      await Deno.writeTextFile(`${TEST_ROOT}/test.txt`, "test content");

      const response = await makeRequest("/test.txt");
      assertEquals(response.status, 403);
      await response.text();

      await Deno.remove(`${TEST_ROOT}/test.txt`);
    });

    await t.step("应该处理特殊字符在路径中", async () => {
      const response = await makeRequest("/test%20page");
      assertEquals(response.status, 404); // 文件不存在
      await response.text();
    });

    await t.step("应该处理空路径", async () => {
      // 临时创建 index.tsx
      await Deno.writeTextFile(
        `${TEST_ROOT}/index.tsx`,
        `
import type { PageContext } from "../../src/cache.ts";
export default async function (_context: PageContext) {
  return <html><body><h1>Test</h1></body></html>;
}
        `
      );

      const response = await makeRequest("/");
      assertEquals(response.status, 200);
      await response.text();

      await Deno.remove(`${TEST_ROOT}/index.tsx`);
    });

    await t.step("应该处理非常长的路径", async () => {
      const longPath = "/" + "a".repeat(1000);
      const response = await makeRequest(longPath);
      assertEquals(response.status, 404);
      await response.text();
    });

    await t.step("应该拒绝绝对路径访问", async () => {
      const response = await makeRequest("/C:/Windows/System32/config");
      // 应该被安全检查拦截
      const status = response.status;
      assertEquals([403, 404].includes(status), true);
      await response.text();
    });
  } finally {
    await cleanup(child);
  }
});
