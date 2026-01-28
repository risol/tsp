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

console.log("\n✓ Router 模块测试完成");
