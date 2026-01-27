/**
 * 端到端测试：重定向功能测试
 * 测试重定向功能
 */

import { assertEquals, assertStringIncludes } from "@std/assert";

const TEST_ROOT = "./tests/tmp";
const TEST_PORT = 9102;

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

// 辅助函数：跟随重定向
async function makeRequestFollowRedirect(
  path: string,
  options: RequestInit = {}
): Promise<Response> {
  const url = `http://localhost:${TEST_PORT}${path}`;
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
  await ensureTestDir();

  const { child } = await startTestServer();

  try {
    // 创建首页
    await Deno.writeTextFile(
      `${TEST_ROOT}/index.tsx`,
      `
import type { PageContext } from "../../src/cache.ts";
export default async function (_context: PageContext) {
  return <html><body><h1>Home Page</h1></body></html>;
}
      `
    );

    // 创建重定向页面
    await Deno.writeTextFile(
      `${TEST_ROOT}/redirect.tsx`,
      `
import type { PageContext } from "../../src/cache.ts";

export default async function (context: PageContext) {
  const { query } = context;

  // 默认 302 重定向
  if (query.to === "home") {
    return { redirect: "/" };
  }

  // 301 永久重定向
  if (query.to === "permanent") {
    return { redirect: "/", status: 301 };
  }

  // 307 临时重定向
  if (query.to === "temp") {
    return { redirect: "/", status: 307 };
  }

  return <html><body><h1>Redirect Test</h1></body></html>;
}
      `
    );

    await t.step("默认 302 重定向应该工作", async () => {
      const response = await makeRequest("/redirect?to=home");
      assertEquals(response.status, 302);
      const location = response.headers.get("location");
      assertEquals(location, "/");
    });

    await t.step("301 永久重定向应该工作", async () => {
      const response = await makeRequest("/redirect?to=permanent");
      assertEquals(response.status, 301);
      const location = response.headers.get("location");
      assertEquals(location, "/");
    });

    await t.step("307 临时重定向应该工作", async () => {
      const response = await makeRequest("/redirect?to=temp");
      assertEquals(response.status, 307);
      const location = response.headers.get("location");
      assertEquals(location, "/");
    });

    await t.step("重定向 Location 头应该存在", async () => {
      const response = await makeRequest("/redirect?to=home");
      const location = response.headers.get("location");
      assertEquals(location !== null, true);
      assertEquals(location?.length || 0, 1);
    });

    await t.step("跟随重定向应该到达目标页面", async () => {
      const response = await makeRequestFollowRedirect("/redirect?to=home");
      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "Home Page");
    });

    await t.step("没有重定向参数时应该返回正常页面", async () => {
      const response = await makeRequest("/redirect");
      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "Redirect Test");
    });

    // 清理测试文件
    await Deno.remove(`${TEST_ROOT}/index.tsx`);
    await Deno.remove(`${TEST_ROOT}/redirect.tsx`);
  } finally {
    await cleanup(child);
  }
});
