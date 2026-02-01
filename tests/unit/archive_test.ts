/**
 * 文件管理器压缩功能单元测试
 */

import { assertEquals, assertExists } from "@std/assert";
import { join } from "std/path";
import { extractArchive, compressToZip, getDirectorySize, getTotalSize } from "../../src/filemanager/archive.ts";
import { isArchiveFile, getArchiveType } from "../../src/filemanager/config.ts";
import { getSupportedArchiveType } from "../../src/filemanager/security.ts";
import { validateFileManagerConfig } from "../../src/filemanager/config.ts";

const TEST_DIR = join(Deno.cwd(), ".test_archive");
const TEST_FILE = join(TEST_DIR, "test.txt");
const TEST_ARCHIVE = join(TEST_DIR, "test.zip");
const TEST_TAR = join(TEST_DIR, "test.tar");
const TEST_TGZ = join(TEST_DIR, "test.tar.gz");

// 创建测试目录和文件
async function setupTestEnvironment() {
  try {
    await Deno.mkdir(TEST_DIR, { recursive: true });
    await Deno.writeTextFile(TEST_FILE, "Hello, World!");
  } catch {
    // 目录可能已存在
  }
}

// 清理测试环境
async function cleanupTestEnvironment() {
  try {
    await Deno.remove(TEST_DIR, { recursive: true });
  } catch {
    // 目录可能不存在
  }
}

Deno.test("文件管理器配置 - 验证默认配置包含解压缩选项", () => {
  const config = validateFileManagerConfig({
    enabled: true,
    password: "test123",
  });

  assertEquals(config.allowExtract, true);
  assertEquals(config.allowCompress, true);
  assertEquals(config.allowedArchiveExtensions?.length, 3);
  assertEquals(config.maxExtractSize, 1024 * 1024 * 1024); // 1GB
  assertEquals(config.maxCompressSize, 500 * 1024 * 1024); // 500MB
  assertEquals(config.maxExtractFileCount, 10000);
});

Deno.test("压缩文件检测 - 识别 ZIP 文件", () => {
  const config = validateFileManagerConfig({
    enabled: true,
    password: "test123",
  });

  assertEquals(isArchiveFile("test.zip"), true);
  assertEquals(isArchiveFile("test.ZIP"), true);
  assertEquals(isArchiveFile("archive.zip"), true);
});

Deno.test("压缩文件检测 - 识别 TAR 文件", () => {
  assertEquals(isArchiveFile("test.tar"), true);
  assertEquals(isArchiveFile("test.TAR"), true);
});

Deno.test("压缩文件检测 - 识别 TAR.GZ 文件", () => {
  assertEquals(isArchiveFile("test.tar.gz"), true);
  assertEquals(isArchiveFile("test.TAR.GZ"), true);
  assertEquals(isArchiveFile("test.tgz"), true);
  assertEquals(isArchiveFile("test.TGZ"), true);
});

Deno.test("压缩文件检测 - 拒绝非压缩文件", () => {
  assertEquals(isArchiveFile("test.txt"), false);
  assertEquals(isArchiveFile("test.jpg"), false);
  assertEquals(isArchiveFile("test"), false);
});

Deno.test("压缩文件类型 - 获取 ZIP 类型", () => {
  const type = getArchiveType("test.zip");
  assertEquals(type, "zip");
});

Deno.test("压缩文件类型 - 获取 TAR 类型", () => {
  const type = getArchiveType("test.tar");
  assertEquals(type, "tar");
});

Deno.test("压缩文件类型 - 获取 TAR.GZ 类型", () => {
  assertEquals(getArchiveType("test.tar.gz"), "tgz");
  assertEquals(getArchiveType("test.tgz"), "tgz");
});

Deno.test("压缩文件类型 - 不支持的格式返回 null", () => {
  assertEquals(getArchiveType("test.txt"), null);
  assertEquals(getArchiveType("test.rar"), null);
});

Deno.test("安全验证 - 检查支持的压缩格式", () => {
  const config = validateFileManagerConfig({
    enabled: true,
    password: "test123",
  });

  assertEquals(getSupportedArchiveType("test.zip", config), "zip");
  assertEquals(getSupportedArchiveType("test.tar", config), "tar");
  assertEquals(getSupportedArchiveType("test.tar.gz", config), "tgz");
  assertEquals(getSupportedArchiveType("test.rar", config), null);
});

Deno.test("目录大小计算 - 空目录", async () => {
  await setupTestEnvironment();

  try {
    const emptyDir = join(TEST_DIR, "empty");
    await Deno.mkdir(emptyDir);

    const size = await getDirectorySize(emptyDir);
    assertEquals(size, 0);

    await Deno.remove(emptyDir);
  } finally {
    await cleanupTestEnvironment();
  }
});

Deno.test("总大小计算 - 多个文件", async () => {
  await setupTestEnvironment();

  try {
    const file1 = join(TEST_DIR, "file1.txt");
    const file2 = join(TEST_DIR, "file2.txt");

    await Deno.writeTextFile(file1, "Hello");
    await Deno.writeTextFile(file2, "World");

    const totalSize = await getTotalSize([file1, file2]);
    assertEquals(totalSize, 10); // "Hello" (5) + "World" (5)
  } finally {
    await cleanupTestEnvironment();
  }
});

Deno.test("ZIP 压缩 - 压缩单个文件", async () => {
  await setupTestEnvironment();

  try {
    await compressToZip([TEST_FILE], TEST_ARCHIVE);

    // 检查压缩文件是否存在
    const stat = await Deno.stat(TEST_ARCHIVE);
    assertExists(stat);
    assertEquals(stat.isFile, true);
  } finally {
    await cleanupTestEnvironment();
  }
});

Deno.test("ZIP 压缩 - 压缩不存在的文件应抛出错误", async () => {
  await setupTestEnvironment();

  try {
    const nonExistentFile = join(TEST_DIR, "nonexistent.txt");

    let threwError = false;
    try {
      await compressToZip([nonExistentFile], TEST_ARCHIVE);
    } catch {
      threwError = true;
    }

    assertEquals(threwError, true);
  } finally {
    await cleanupTestEnvironment();
  }
});

// 清理测试
Deno.test("清理测试环境", async () => {
  await cleanupTestEnvironment();
  const exists = await Deno.stat(TEST_DIR).then(() => true).catch(() => false);
  assertEquals(exists, false);
});
