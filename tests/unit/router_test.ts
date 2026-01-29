/**
 * Router 模块单元测试
 * 测试 src/router.ts 中的路由解析功能
 */

import { assertEquals } from "https://deno.land/std@0.210.0/testing/asserts.ts";
import { resolvePath } from "../../src/router.ts";

// 使用相对于项目根目录的路径
const TEST_ROOT = "./www";

Deno.test("router - resolvePath: 根路径", () => {
  const result = resolvePath("/", TEST_ROOT);
  assertEquals(result.success, true);
  // Windows 使用 \，Unix 使用 /，所以替换后比较
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/index.tsx");
});

Deno.test("router - resolvePath: 简单路径", () => {
  const result = resolvePath("/form.tsx", TEST_ROOT);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/form.tsx");
});

Deno.test("router - resolvePath: 带目录的路径", () => {
  const result = resolvePath("/api/test", TEST_ROOT);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/api/test.tsx");
});

Deno.test("router - resolvePath: 带扩展名的路径", () => {
  const result = resolvePath("/admin/user.tsx", TEST_ROOT);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/admin/user.tsx");
});

Deno.test("router - resolvePath: 嵌套目录", () => {
  const result = resolvePath("/api/v1/users/list", TEST_ROOT);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/api/v1/users/list.tsx");
});

// 静态文件测试
Deno.test("router - resolvePath: 静态 CSS 文件", () => {
  const result = resolvePath("/static/style.css", TEST_ROOT, [".css"]);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/static/style.css");
});

Deno.test("router - resolvePath: 静态 JS 文件", () => {
  const result = resolvePath("/js/app.js", TEST_ROOT, [".js"]);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/js/app.js");
});

Deno.test("router - resolvePath: 静态图片文件", () => {
  const result = resolvePath("/images/logo.png", TEST_ROOT, [".png"]);
  assertEquals(result.success, true);
  const normalizedPath = result.filepath!.replace(/\\/g, "/");
  assertEquals(normalizedPath, "www/images/logo.png");
});

Deno.test("router - resolvePath: 混合扩展名（TSX 和静态文件）", () => {
  // TSX 文件（默认支持）
  const tsxResult = resolvePath("/page", TEST_ROOT, [".css", ".js"]);
  assertEquals(tsxResult.success, true);
  const tsxPath = tsxResult.filepath!.replace(/\\/g, "/");
  assertEquals(tsxPath, "www/page.tsx");

  // 静态 CSS 文件
  const cssResult = resolvePath("/style.css", TEST_ROOT, [".css"]);
  assertEquals(cssResult.success, true);
  const cssPath = cssResult.filepath!.replace(/\\/g, "/");
  assertEquals(cssPath, "www/style.css");
});

console.log("\n✓ Router 模块测试完成");
