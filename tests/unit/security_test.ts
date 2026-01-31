/**
 * Security 模块单元测试
 * 测试 src/router.ts 中的安全检查功能
 */

import { assertEquals } from "https://deno.land/std@0.210.0/testing/asserts.ts";
import { resolvePath, securityCheck } from "../../src/router.ts";

// 使用相对于项目根目录的路径
const TEST_ROOT = "./www";

Deno.test("security - securityCheck: 路径穿越攻击防护", async () => {
  const testPaths = [
    "../../../etc/passwd",
    "../secret.tsx",
    "./../../test.tsx",
    "..\\..\\..\\windows\\system32\\config\\sam",
  ];

  for (const path of testPaths) {
    const result = await resolvePath(path, TEST_ROOT);
    if (result.success && result.filepath) {
      const checkResult = await securityCheck(result.filepath, TEST_ROOT);
      assertEquals(checkResult.success, false, `应该阻止路径穿越: ${path}`);
    }
  }
});

Deno.test("security - securityCheck: 非白名单文件", async () => {
  const testPaths = [
    "www/config.json",
    "www/data.txt",
    "www/secret.ts",
    "www/.env",
    "www/package.json",
  ];

  for (const filepath of testPaths) {
    const checkResult = await securityCheck(filepath, TEST_ROOT);
    assertEquals(
      checkResult.success,
      false,
      `应该阻止非 TSX 文件: ${filepath}`,
    );
  }
});

Deno.test("security - securityCheck: 正常文件通过", async () => {
  const validPaths = [
    "www/index.tsx",
    "www/form.tsx",
    "www/api.tsx",
  ];

  for (const filepath of validPaths) {
    // 检查文件是否真的存在
    try {
      await Deno.stat(filepath);
    } catch {
      console.log(`跳过不存在的文件: ${filepath}`);
      continue;
    }

    const checkResult = await securityCheck(filepath, TEST_ROOT);
    assertEquals(
      checkResult.success,
      true,
      `应该允许正常文件: ${filepath} - ${checkResult.error}`,
    );
  }
});

Deno.test("security - securityCheck: 相对路径穿越", async () => {
  const attackPaths = [
    "./../../../etc/passwd",
    "..././etc/passwd",
    "..\\..\\..\\config.json",
  ];

  for (const path of attackPaths) {
    const result = await resolvePath(path, TEST_ROOT);
    if (result.success && result.filepath) {
      const checkResult = await securityCheck(result.filepath, TEST_ROOT);
      assertEquals(checkResult.success, false, `应该阻止相对路径穿越: ${path}`);
    }
  }
});

Deno.test("security - securityCheck: URL 编码绕过", async () => {
  const encodedPaths = [
    "%2e%2e%2fetc/passwd", // ../etc/passwd
    "..%252f..%252f..%252fetc/passwd", // 双重编码
  ];

  for (const path of encodedPaths) {
    const result = await resolvePath(path, TEST_ROOT);
    if (result.success && result.filepath) {
      const checkResult = await securityCheck(result.filepath, TEST_ROOT);
      // URL 编码的路径应该被安全处理
      if (checkResult.success) {
        // 如果通过了检查，确保文件路径在根目录内
        const normalizedPath = result.filepath.replace(/\\/g, "/");
        const isInRoot = normalizedPath.startsWith(TEST_ROOT) ||
          normalizedPath.startsWith("www");
        assertEquals(isInRoot, true, `URL编码的路径应该在根目录内: ${path}`);
      }
    }
  }
});

console.log("\n✓ Security 模块测试完成");
