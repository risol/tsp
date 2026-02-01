/**
 * 文件管理器模块单元测试
 */

import {
  assertEquals,
  assertExists,
} from "https://deno.land/std@0.210.0/testing/asserts.ts";
import {
  validateFileManagerConfig,
  isExtensionAllowed,
  formatFileSize,
  formatDateTime,
} from "../../src/filemanager/config.ts";
import type { FileManagerConfig } from "../../src/filemanager/types.ts";
import {
  validatePath,
  isSafeFilename,
  isAllowedExtension,
  isAllowedFileSize,
  checkOperationPermission,
} from "../../src/filemanager/security.ts";
import {
  hashPassword,
  verifyPassword,
  createSession,
  validateSession,
  destroySession,
  getSessionIdFromCookies,
} from "../../src/filemanager/auth.ts";

Deno.test("文件管理器 - config: 验证配置 - 启用状态", () => {
  const config: FileManagerConfig = {
    enabled: true,
    password: "test123456",
  };

  const validated = validateFileManagerConfig(config);

  assertEquals(validated.enabled, true);
  assertEquals(validated.password, "test123456");
  assertEquals(validated.path, "/__filemanager");
  assertEquals(validated.allowOutsideRoot, false);
});

Deno.test("文件管理器 - config: 验证配置 - 未启用", () => {
  const config: FileManagerConfig = {
    enabled: false,
    password: "",
  };

  const validated = validateFileManagerConfig(config);

  assertEquals(validated.enabled, false);
});

Deno.test("文件管理器 - config: 密码太短", () => {
  const config: FileManagerConfig = {
    enabled: true,
    password: "12345", // 少于 6 个字符
  };

  try {
    validateFileManagerConfig(config);
    throw new Error("应该抛出错误");
  } catch (error) {
    assertExists(error);
  }
});

Deno.test("文件管理器 - config: 缺少密码", () => {
  const config: FileManagerConfig = {
    enabled: true,
    password: "",
  };

  try {
    validateFileManagerConfig(config);
    throw new Error("应该抛出错误");
  } catch (error) {
    assertExists(error);
  }
});

Deno.test("文件管理器 - config: 格式化文件大小", () => {
  assertEquals(formatFileSize(0), "0 B");
  assertEquals(formatFileSize(1024), "1 KB");
  assertEquals(formatFileSize(1024 * 1024), "1 MB");
  assertEquals(formatFileSize(1536), "1.5 KB");
});

Deno.test("文件管理器 - security: 验证路径 - 合法路径", () => {
  const config: FileManagerConfig = {
    enabled: true,
    password: "test123456",
  };
  const validated = validateFileManagerConfig(config);
  const rootPath = Deno.cwd();

  const result = validatePath(rootPath, rootPath, validated);

  assertEquals(result.success, true);
  assertExists(result.normalizedPath);
});

Deno.test("文件管理器 - security: 验证路径 - 路径穿越", () => {
  const config: FileManagerConfig = {
    enabled: true,
    password: "test123456",
    allowOutsideRoot: false,
  };
  const validated = validateFileManagerConfig(config);
  const rootPath = Deno.cwd();

  // 尝试访问 root 外的路径
  const outsidePath = rootPath + "/../outside";
  const result = validatePath(outsidePath, rootPath, validated);

  assertEquals(result.success, false);
});

Deno.test("文件管理器 - security: 安全文件名", () => {
  assertEquals(isSafeFilename("test.txt"), true);
  assertEquals(isSafeFilename("test-file.txt"), true);
  assertEquals(isSafeFilename("test_file.txt"), true);
  assertEquals(isSafeFilename("test file.txt"), true); // 空格是允许的
  assertEquals(isSafeFilename("test/file.txt"), false); // 包含路径分隔符
  assertEquals(isSafeFilename(".hidden"), false); // 以点开头
  assertEquals(isSafeFilename("CON"), false); // Windows 保留名称
});

Deno.test("文件管理器 - security: 检查操作权限", () => {
  const config: FileManagerConfig = {
    enabled: true,
    password: "test123456",
    allowDelete: true,
    allowRename: false,
    allowMkdir: true,
    allowMove: false,
  };
  const validated = validateFileManagerConfig(config);

  assertEquals(checkOperationPermission("delete", validated), true);
  assertEquals(checkOperationPermission("rename", validated), false);
  assertEquals(checkOperationPermission("mkdir", validated), true);
  assertEquals(checkOperationPermission("move", validated), false);
});

Deno.test("文件管理器 - auth: 密码哈希和验证", async () => {
  const password = "test123456";

  // 哈希密码
  const { hash, salt } = await hashPassword(password);

  assertExists(hash);
  assertExists(salt);

  // 验证密码
  const isValid = await verifyPassword(password, hash, salt);
  assertEquals(isValid, true);

  // 验证错误密码
  const isInvalid = await verifyPassword("wrongpassword", hash, salt);
  assertEquals(isInvalid, false);
});

Deno.test("文件管理器 - auth: Session 管理", () => {
  // 创建 session
  const sessionId = createSession();
  assertExists(sessionId);

  // 验证 session
  const isValid = validateSession(sessionId);
  assertEquals(isValid, true);

  // 销毁 session
  destroySession(sessionId);

  // 再次验证应该失败
  const isStillValid = validateSession(sessionId);
  assertEquals(isStillValid, false);
});

Deno.test("文件管理器 - auth: 从 cookies 获取 session ID", () => {
  const cookies: Record<string, string> = {
    fm_session: "test-session-id",
    other_cookie: "value",
  };

  const sessionId = getSessionIdFromCookies(cookies);
  assertEquals(sessionId, "test-session-id");
});

Deno.test("文件管理器 - config: 扩展名白名单检查", () => {
  const config: FileManagerConfig = {
    enabled: true,
    password: "test123456",
    allowedExtensions: [".jpg", ".png", ".gif"],
    deniedExtensions: [".exe"],
  };
  const validated = validateFileManagerConfig(config);

  assertEquals(isExtensionAllowed(".jpg", validated), true);
  assertEquals(isExtensionAllowed(".png", validated), true);
  assertEquals(isExtensionAllowed(".exe", validated), false); // 黑名单
  assertEquals(isExtensionAllowed(".txt", validated), false); // 不在白名单
});

Deno.test("文件管理器 - security: 文件大小检查", () => {
  const config: FileManagerConfig = {
    enabled: true,
    password: "test123456",
    maxUploadSize: 1024 * 1024, // 1MB
  };
  const validated = validateFileManagerConfig(config);

  assertEquals(isAllowedFileSize(1024, validated), true);
  assertEquals(isAllowedFileSize(1024 * 1024, validated), true);
  assertEquals(isAllowedFileSize(1024 * 1024 + 1, validated), false);
});

Deno.test("文件管理器 - security: 扩展名检查", () => {
  const config: FileManagerConfig = {
    enabled: true,
    password: "test123456",
    allowedExtensions: [".jpg", ".png"],
    deniedExtensions: [".exe", ".sh"],
  };
  const validated = validateFileManagerConfig(config);

  assertEquals(isAllowedExtension("photo.jpg", validated), true);
  assertEquals(isAllowedExtension("image.png", validated), true);
  assertEquals(isAllowedExtension("program.exe", validated), false);
  assertEquals(isAllowedExtension("script.sh", validated), false);
  assertEquals(isAllowedExtension("document.txt", validated), false); // 不在白名单
});

console.log("\n✓ 文件管理器模块测试完成");
