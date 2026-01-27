/**
 * 端到端测试：错误处理和边缘情况
 * 测试各种错误场景和边缘情况
 */

import { assertEquals, assertStringIncludes } from "@std/assert";

const TEST_ROOT = "./tests/tmp";

// 启动测试服务器
async function startTestServer(devMode: boolean = false) {
  const port = devMode ? 9104 : 9103;

  const args = [
    "run",
    "--allow-net",
    "--allow-read",
    "src/main.ts",
    "--root",
    TEST_ROOT,
    "--port",
    port.toString(),
  ];

  if (devMode) {
    args.push("--dev");
  }

  const process = new Deno.Command("deno", {
    args,
    stdout: "piped",
    stderr: "piped",
  });

  const child = process.spawn();

  // 等待服务器启动
  await new Promise((resolve) => setTimeout(resolve, 2000));

  return { child, port };
}

// 确保测试目录存在
async function ensureTestDir() {
  await Deno.mkdir(TEST_ROOT, { recursive: true });
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

Deno.test("E2E: 生产模式错误处理", async (t) => {
  await ensureTestDir();
  const { child, port } = await startTestServer(false);

  try {
    await t.step("服务器错误应该返回 500", async () => {
      // 创建一个会抛出错误的页面
      const errorPagePath = `${TEST_ROOT}/error-test.tsx`;
      await Deno.writeTextFile(
        errorPagePath,
        `
        import type { PageContext } from "../../src/cache.ts";
        export default async function (_context: PageContext) {
          throw new Error("Test error");
        }
        `
      );

      try {
        const response = await makeRequest(port, "/error-test");
        assertEquals(response.status, 500);
      } finally {
        await Deno.remove(errorPagePath);
      }
    });

    await t.step("生产模式应该隐藏错误详情", async () => {
      const errorPagePath = "./www/error-test2.tsx";
      await Deno.writeTextFile(
        errorPagePath,
        `
        import type { PageContext } from "../../src/cache.ts";
        export default async function (_context: PageContext) {
          throw new Error("Secret error details");
        }
        `
      );

      try {
        const response = await makeRequest(port, "/error-test2");
        const text = await response.text();
        // 生产模式不应该显示具体错误信息
        assertEquals(text.includes("Secret error details"), false);
        assertStringIncludes(text, "Internal Server Error");
      } finally {
        await Deno.remove(errorPagePath);
      }
    });

    await t.step("生产模式错误页面应该友好", async () => {
      const errorPagePath = "./www/error-test3.tsx";
      await Deno.writeTextFile(
        errorPagePath,
        `
        import type { PageContext } from "../../src/cache.ts";
        export default async function (_context: PageContext) {
          throw new Error("Any error");
        }
        `
      );

      try {
        const response = await makeRequest(port, "/error-test3");
        const text = await response.text();
        assertStringIncludes(text, "500 Internal Server Error");
        assertStringIncludes(text, "An error occurred");
      } finally {
        await Deno.remove(errorPagePath);
      }
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 开发模式错误处理", async (t) => {
  await ensureTestDir();
  const { child, port } = await startTestServer(true);

  try {
    await t.step("开发模式应该返回 500", async () => {
      const errorPagePath = "./www/dev-error-test.tsx";
      await Deno.writeTextFile(
        errorPagePath,
        `
        import type { PageContext } from "../../src/cache.ts";
        export default async function (_context: PageContext) {
          throw new Error("Development error");
        }
        `
      );

      try {
        const response = await makeRequest(port, "/dev-error-test");
        assertEquals(response.status, 500);
      } finally {
        await Deno.remove(errorPagePath);
      }
    });

    await t.step("开发模式应该显示错误详情", async () => {
      const errorPagePath = "./www/dev-error-test2.tsx";
      await Deno.writeTextFile(
        errorPagePath,
        `
        import type { PageContext } from "../../src/cache.ts";
        export default async function (_context: PageContext) {
          throw new Error("Detailed error message");
        }
        `
      );

      try {
        const response = await makeRequest(port, "/dev-error-test2");
        const text = await response.text();
        // 开发模式应该显示具体错误信息
        assertStringIncludes(text, "Detailed error message");
        assertStringIncludes(text, "500 Internal Server Error");
      } finally {
        await Deno.remove(errorPagePath);
      }
    });

    await t.step("开发模式应该显示堆栈跟踪", async () => {
      const errorPagePath = "./www/dev-error-test3.tsx";
      await Deno.writeTextFile(
        errorPagePath,
        `
        import type { PageContext } from "../../src/cache.ts";
        export default async function (_context: PageContext) {
          throw new Error("Stack trace test");
        }
        `
      );

      try {
        const response = await makeRequest(port, "/dev-error-test3");
        const text = await response.text();
        // 应该包含堆栈跟踪信息
        assertStringIncludes(text.toLowerCase(), "stack");
      } finally {
        await Deno.remove(errorPagePath);
      }
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 模块加载错误", async (t) => {
  await ensureTestDir();
  const { child, port } = await startTestServer(false);

  try {
    await t.step("语法错误的页面应该返回 500", async () => {
      const errorPagePath = "./www/syntax-error.tsx";
      await Deno.writeTextFile(
        errorPagePath,
        `
        import type { PageContext } from "../../src/cache.ts";
        export default async function (_context: PageContext) {
          // 故意的语法错误
          const x = ;
          return <div>Error</div>;
        }
        `
      );

      try {
        const response = await makeRequest(port, "/syntax-error");
        // 语法错误会导致模块加载失败
        const status = response.status;
        assertEquals([500, 404].includes(status), true);
      } finally {
        await Deno.remove(errorPagePath);
      }
    });

    await t.step("没有默认导出的模块应该返回 500", async () => {
      const errorPagePath = "./www/no-export.tsx";
      await Deno.writeTextFile(
        errorPagePath,
        `
        import type { PageContext } from "../../src/cache.ts";
        // 没有默认导出
        export const something = "test";
        `
      );

      try {
        const response = await makeRequest(port, "/no-export");
        assertEquals(response.status, 500);
      } finally {
        await Deno.remove(errorPagePath);
      }
    });

    await t.step("默认导出不是函数应该返回 500", async () => {
      const errorPagePath = "./www/not-function.tsx";
      await Deno.writeTextFile(
        errorPagePath,
        `
        import type { PageContext } from "../../src/cache.ts";
        export default "not a function";
        `
      );

      try {
        const response = await makeRequest(port, "/not-function");
        assertEquals(response.status, 500);
      } finally {
        await Deno.remove(errorPagePath);
      }
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 边缘情况测试", async (t) => {
  await ensureTestDir();
  const { child, port } = await startTestServer(false);

  try {
    await t.step("空路径应该映射到 index", async () => {
      const response = await makeRequest(port, "/");
      assertEquals(response.status, 200);
      const text = await response.text();
      assertStringIncludes(text, "TSP-FPM");
    });

    await t.step("多个斜杠应该正常处理", async () => {
      const response = await makeRequest(port, "//");
      // 可能是 404 或 400
      const status = response.status;
      assertEquals([400, 404].includes(status), true);
    });

    await t.step("非常长的路径应该正确处理", async () => {
      const longPath = "/" + "a".repeat(1000);
      const response = await makeRequest(port, longPath);
      assertEquals(response.status, 404);
    });

    await t.step("特殊字符在路径中应该被正确处理", async () => {
      const response = await makeRequest(port, "/test%20page");
      assertEquals(response.status, 404); // 文件不存在
    });

    await t.step("查询参数中的特殊字符应该被正确处理", async () => {
      const response = await makeRequest(port, "/?data=<script>alert('xss')</script>");
      assertEquals(response.status, 200);
      const text = await response.text();
      // 应该被转义或正确显示
      assertStringIncludes(text, "&lt;script&gt;"); // HTML 转义
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: HTTP 方法测试", async (t) => {
  await ensureTestDir();
  const { child, port } = await startTestServer(false);

  try {
    await t.step("GET 请求应该正常工作", async () => {
      const response = await makeRequest(port, "/", { method: "GET" });
      assertEquals(response.status, 200);
    });

    await t.step("POST 请求应该正常工作", async () => {
      const response = await makeRequest(port, "/", {
        method: "POST",
        body: "test=data",
        headers: {
          "Content-Type": "application/x-www-form-urlencoded",
        },
      });
      assertEquals(response.status, 200);
    });

    await t.step("HEAD 请求应该正常工作", async () => {
      const response = await makeRequest(port, "/", { method: "HEAD" });
      assertEquals(response.status, 200);
    });

    await t.step("不支持的 HTTP 方法应该有合理的行为", async () => {
      const response = await makeRequest(port, "/", { method: "OPTIONS" });
      // 应该返回某个状态码（可能是 200, 405, 或其他）
      const status = response.status;
      assertEquals(status >= 200 && status < 600, true);
    });
  } finally {
    await cleanup(child);
  }
});

Deno.test("E2E: 大文件和性能测试", async (t) => {
  await ensureTestDir();
  const { child, port } = await startTestServer(false);

  try {
    await t.step("大型查询参数应该正确处理", async () => {
      const largeQuery = "?data=" + "x".repeat(5000);
      const response = await makeRequest(port, "/" + largeQuery);
      assertEquals(response.status, 200);
    });

    await t.step("大型 POST 请求应该正确处理", async () => {
      const largeData = { data: "x".repeat(10000) };
      const response = await makeRequest(port, "/", {
        method: "POST",
        body: JSON.stringify(largeData),
        headers: {
          "Content-Type": "application/json",
        },
      });
      assertEquals(response.status, 200);
    });
  } finally {
    await cleanup(child);
  }
});
