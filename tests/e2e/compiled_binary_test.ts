/**
 * 编译后的二进制文件测试
 * 测试 deno compile 生成的可执行文件能否正常工作
 */

import { assertEquals, assertStringIncludes } from "@std/assert";
import { join, dirname } from "std/path";

const TEST_ROOT = "./tests/tmp";
const BINARY_NAME = "tsp-fpm.exe";
const BINARY_PATH = `./${BINARY_NAME}`;

// 测试页面内容
const testPages = {
  "compiled_index.tsx": `
export default async function(context: Record<string, unknown>) {
  return (
    <html>
      <head><title>编译测试首页</title></head>
      <body>
        <h1>编译后的二进制测试</h1>
        <p>这是编译后的二进制文件访问的页面</p>
        <p>当前路径: {String((context.url as Record<string, unknown>).pathname)}</p>
      </body>
    </html>
  );
}
`,
  "compiled_form.tsx": `
export default function(_context: Record<string, unknown>) {
  return (
    <html>
      <head><title>编译测试表单</title></head>
      <body>
        <h1>编译测试表单</h1>
        <form method="POST" action="/compiled_submit">
          <div>
            <label>用户名:</label>
            <input type="text" name="username" />
          </div>
          <button type="submit">提交</button>
        </form>
      </body>
    </html>
  );
}
`,
  "compiled_redirect.tsx": `
export default function(_context: Record<string, unknown>) {
  return {
    redirect: "/compiled_index.tsx",
    status: 302
  };
}
`,
};

/**
 * 设置测试页面
 */
async function setupTestPages(): Promise<void> {
  // 清理并重建测试目录
  try {
    await Deno.remove(TEST_ROOT, { recursive: true });
  } catch (_error) {
    // 忽略
  }
  await Deno.mkdir(TEST_ROOT, { recursive: true });

  // 创建所有测试页面
  for (const [filepath, content] of Object.entries(testPages)) {
    const fullPath = join(TEST_ROOT, filepath);
    const dir = dirname(fullPath);

    // 创建目录（如果不是根目录）
    if (dir !== TEST_ROOT) {
      await Deno.mkdir(dir, { recursive: true });
    }

    // 写入文件
    await Deno.writeTextFile(fullPath, content);
  }
}

/**
 * 清理测试页面
 */
async function cleanupTestPages(): Promise<void> {
  try {
    await Deno.remove(TEST_ROOT, { recursive: true });
  } catch (_error) {
    // 忽略删除失败
  }
}

/**
 * 检查二进制文件是否存在，如果不存在则编译
 */
async function ensureBinary(): Promise<void> {
  try {
    await Deno.stat(BINARY_PATH);
    console.log("✓ 二进制文件已存在");
  } catch (_error) {
    console.log("⚠ 二进制文件不存在，开始编译...");
    const compileProcess = new Deno.Command("deno", {
      args: [
        "compile",
        "--allow-net",
        "--allow-read",
        "--allow-env",
        "--output",
        BINARY_PATH,
        "src/main.ts",
      ],
      stdout: "inherit",
      stderr: "inherit",
    });

    const { code } = await compileProcess.output();
    if (code !== 0) {
      throw new Error("编译失败");
    }
    console.log("✓ 编译成功");
  }
}

/**
 * 启动编译后的服务器
 */
async function startServer(): Promise<Deno.ChildProcess> {
  // 创建 DENO_DIR 目录
  const denoDir = join(Deno.cwd(), ".deno");
  await Deno.mkdir(denoDir, { recursive: true });

  // 调试：列出创建的文件
  console.log("测试根目录:", TEST_ROOT);
  for await (const entry of Deno.readDir(TEST_ROOT)) {
    console.log("  文件:", entry.name);
  }

  // 启动服务器（使用 DENO_DIR 环境变量）
  const serverProcess = new Deno.Command(BINARY_PATH, {
    args: ["-r", TEST_ROOT, "-p", "9001", "--dev"],
    env: {
      "DENO_DIR": denoDir,
    },
    stdout: "inherit",
    stderr: "inherit",
  });

  const child = serverProcess.spawn();

  // 等待服务器启动
  await new Promise((resolve) => setTimeout(resolve, 3000));

  // 验证服务器是否真的在运行
  try {
    const response = await fetch("http://127.0.0.1:9001/");
    console.log("服务器启动成功，状态:", response.status);
    await response.body?.cancel();
  } catch (error) {
    // 如果连接失败，说明服务器没有正常启动
    console.error("服务器连接失败:", error);
    try {
      child.kill("SIGTERM");
    } catch (_e) {
      // 忽略
    }
    throw new Error(`服务器无法启动: ${(error as Error).message}`);
  }

  return child;
}

/**
 * 停止服务器
 */
async function stopServer(process: Deno.ChildProcess): Promise<void> {
  try {
    process.kill("SIGTERM");
    await process.status;
  } catch (_error) {
    // 忽略错误
  }
}

/**
 * 发送 HTTP 请求
 */
async function fetchPage(path: string): Promise<Response> {
  const url = `http://127.0.0.1:9001${path}`;
  const response = await fetch(url);
  return response;
}

/**
 * 测试编译后的二进制文件
 */
Deno.test("编译后的二进制 - 基本功能", async (t) => {
  await setupTestPages();
  let server: Deno.ChildProcess | null = null;

  try {
    server = await startServer();

    await t.step("应该成功访问首页", async () => {
      const response = await fetchPage("/compiled_index.tsx");
      assertEquals(response.status, 200);

      const html = await response.text();
      assertStringIncludes(html, "编译后的二进制测试");
      assertStringIncludes(html, "当前路径:");
    });

    await t.step("应该返回正确的 Content-Type", async () => {
      const response = await fetchPage("/compiled_index.tsx");
      assertEquals(response.headers.get("content-type"), "text/html; charset=utf-8");
      await response.body?.cancel();
    });

    await t.step("应该成功访问表单页面", async () => {
      const response = await fetchPage("/compiled_form.tsx");
      assertEquals(response.status, 200);

      const html = await response.text();
      assertStringIncludes(html, "编译测试表单");
      assertStringIncludes(html, "用户名:");
      assertStringIncludes(html, '<form method="POST"');
    });

    await t.step("应该正确处理重定向", async () => {
      // fetch 默认跟随重定向，所以最终会得到 200
      const response = await fetchPage("/compiled_redirect.tsx");
      assertEquals(response.status, 200);

      const html = await response.text();
      assertStringIncludes(html, "编译后的二进制测试");
    });
  } finally {
    if (server) {
      await stopServer(server);
    }
    await cleanupTestPages();
  }
});

Deno.test("编译后的二进制 - 自动编译测试", async (t) => {
  await t.step("应该能够编译二进制文件", async () => {
    // 如果二进制文件已存在，先删除
    try {
      await Deno.remove(BINARY_PATH);
    } catch (_error) {
      // 忽略
    }

    await ensureBinary();

    // 验证二进制文件存在
    const stat = await Deno.stat(BINARY_PATH);
    assertEquals(stat.isFile, true);
    console.log(`✓ 二进制文件大小: ${(stat.size / 1024 / 1024).toFixed(2)} MB`);
  });
});
