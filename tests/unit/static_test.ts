/**
 * Static File 模块单元测试
 * 测试 src/static.ts 中的静态文件服务功能
 */

import {
  assertEquals,
  assertExists,
} from "https://deno.land/std@0.210.0/testing/asserts.ts";
import {
  getMimeType,
  isStaticFileAllowed,
  serveStaticFile,
  serveStaticFileWithCache,
} from "../../src/static.ts";

// 测试用的临时文件路径
const TEST_DIR = "./test_static_files";
const TEST_FILES = {
  css: `${TEST_DIR}/test.css`,
  js: `${TEST_DIR}/test.js`,
  png: `${TEST_DIR}/test.png`,
  html: `${TEST_DIR}/test.html`,
  txt: `${TEST_DIR}/test.txt`,
};

// 默认允许的扩展名列表
const DEFAULT_EXTENSIONS = [
  ".css",
  ".js",
  ".json",
  ".png",
  ".jpg",
  ".jpeg",
  ".gif",
  ".svg",
  ".ico",
  ".webp",
  ".woff",
  ".woff2",
  ".ttf",
  ".eot",
  ".mp3",
  ".mp4",
  ".webm",
  ".txt",
  ".md",
  ".xml",
];

/**
 * 创建测试文件
 */
async function setupTestFiles() {
  // 创建测试目录
  await Deno.mkdir(TEST_DIR, { recursive: true });

  // 创建各种测试文件
  await Deno.writeTextFile(TEST_FILES.css, "body { color: red; }");
  await Deno.writeTextFile(TEST_FILES.js, "console.log('test');");
  await Deno.writeTextFile(TEST_FILES.txt, "test content");
  await Deno.writeTextFile(TEST_FILES.html, "<html></html>");

  // 创建一个假的图片文件（二进制）
  const pngData = new Uint8Array([0x89, 0x50, 0x4e, 0x47]); // PNG 签名
  await Deno.writeFile(TEST_FILES.png, pngData);
}

/**
 * 清理测试文件
 */
async function cleanupTestFiles() {
  try {
    await Deno.remove(TEST_DIR, { recursive: true });
  } catch {
    // 目录不存在，忽略
  }
}

Deno.test("static - getMimeType: CSS 文件", () => {
  const mimeType = getMimeType("style.css");
  assertEquals(mimeType, "text/css; charset=utf-8");
});

Deno.test("static - getMimeType: JavaScript 文件", () => {
  const mimeType = getMimeType("app.js");
  assertEquals(mimeType, "application/javascript; charset=utf-8");
});

Deno.test("static - getMimeType: JSON 文件", () => {
  const mimeType = getMimeType("data.json");
  assertEquals(mimeType, "application/json; charset=utf-8");
});

Deno.test("static - getMimeType: PNG 图片", () => {
  const mimeType = getMimeType("image.png");
  assertEquals(mimeType, "image/png");
});

Deno.test("static - getMimeType: JPG 图片", () => {
  const mimeType = getMimeType("photo.jpg");
  assertEquals(mimeType, "image/jpeg");
});

Deno.test("static - getMimeType: JPEG 图片", () => {
  const mimeType = getMimeType("photo.jpeg");
  assertEquals(mimeType, "image/jpeg");
});

Deno.test("static - getMimeType: SVG 图片", () => {
  const mimeType = getMimeType("icon.svg");
  assertEquals(mimeType, "image/svg+xml");
});

Deno.test("static - getMimeType: ICO 图标", () => {
  const mimeType = getMimeType("favicon.ico");
  assertEquals(mimeType, "image/x-icon");
});

Deno.test("static - getMimeType: WOFF 字体", () => {
  const mimeType = getMimeType("font.woff");
  assertEquals(mimeType, "font/woff");
});

Deno.test("static - getMimeType: WOFF2 字体", () => {
  const mimeType = getMimeType("font.woff2");
  assertEquals(mimeType, "font/woff2");
});

Deno.test("static - getMimeType: 文本文件", () => {
  const mimeType = getMimeType("readme.txt");
  assertEquals(mimeType, "text/plain; charset=utf-8");
});

Deno.test("static - getMimeType: Markdown 文件", () => {
  const mimeType = getMimeType("doc.md");
  assertEquals(mimeType, "text/markdown; charset=utf-8");
});

Deno.test("static - getMimeType: 未知类型", () => {
  const mimeType = getMimeType("file.unknown");
  assertEquals(mimeType, "application/octet-stream");
});

Deno.test("static - isStaticFileAllowed: 允许的扩展名", () => {
  const allowed = isStaticFileAllowed("style.css", DEFAULT_EXTENSIONS);
  assertEquals(allowed, true);
});

Deno.test("static - isStaticFileAllowed: 不允许的扩展名", () => {
  const allowed = isStaticFileAllowed("page.tsx", DEFAULT_EXTENSIONS);
  assertEquals(allowed, false);
});

Deno.test("static - isStaticFileAllowed: 空列表", () => {
  const allowed = isStaticFileAllowed("style.css", []);
  assertEquals(allowed, false);
});

Deno.test("static - isStaticFileAllowed: 大小写不敏感", () => {
  const allowed = isStaticFileAllowed("IMAGE.PNG", DEFAULT_EXTENSIONS);
  assertEquals(allowed, true);
});

Deno.test("static - serveStaticFile: 读取 CSS 文件", async () => {
  await setupTestFiles();

  try {
    const response = await serveStaticFile(
      TEST_FILES.css,
      DEFAULT_EXTENSIONS,
      false,
    );

    assertExists(response);
    assertEquals(response?.status, 200);
    assertEquals(
      response?.headers.get("Content-Type"),
      "text/css; charset=utf-8",
    );

    // 验证 ETag 存在（生产模式）
    assertExists(response?.headers.get("ETag"));
    assertExists(response?.headers.get("Last-Modified"));
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFile: 读取 JS 文件", async () => {
  await setupTestFiles();

  try {
    const response = await serveStaticFile(
      TEST_FILES.js,
      DEFAULT_EXTENSIONS,
      false,
    );

    assertExists(response);
    assertEquals(response?.status, 200);
    assertEquals(
      response?.headers.get("Content-Type"),
      "application/javascript; charset=utf-8",
    );
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFile: 读取文本文件", async () => {
  await setupTestFiles();

  try {
    const response = await serveStaticFile(
      TEST_FILES.txt,
      DEFAULT_EXTENSIONS,
      false,
    );

    assertExists(response);
    assertEquals(response?.status, 200);
    assertEquals(
      response?.headers.get("Content-Type"),
      "text/plain; charset=utf-8",
    );

    // 验证内容
    const text = await response?.text();
    assertEquals(text, "test content");
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFile: 开发模式无缓存", async () => {
  await setupTestFiles();

  try {
    const response = await serveStaticFile(
      TEST_FILES.css,
      DEFAULT_EXTENSIONS,
      true, // dev mode
    );

    assertExists(response);
    assertEquals(response?.status, 200);

    // 开发模式应该禁用缓存
    assertEquals(
      response?.headers.get("Cache-Control"),
      "no-cache, no-store, must-revalidate",
    );
    assertEquals(response?.headers.get("Pragma"), "no-cache");
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFile: 不允许的文件类型", async () => {
  await setupTestFiles();

  try {
    const response = await serveStaticFile(
      TEST_FILES.html, // HTML 不在默认列表中
      DEFAULT_EXTENSIONS,
      false,
    );

    // 应该返回 null（不处理）
    assertEquals(response, null);
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFile: 文件不存在", async () => {
  await setupTestFiles();

  try {
    const response = await serveStaticFile(
      `${TEST_DIR}/notexist.css`,
      DEFAULT_EXTENSIONS,
      false,
    );

    // 文件不存在应该返回 null
    assertEquals(response, null);
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFileWithCache: ETag 验证", async () => {
  await setupTestFiles();

  try {
    // 第一次请求
    const response1 = await serveStaticFileWithCache(
      TEST_FILES.css,
      DEFAULT_EXTENSIONS,
      new Headers(),
      false,
    );

    assertExists(response1);
    assertEquals(response1?.status, 200);

    const etag = response1?.headers.get("ETag");
    assertExists(etag);

    // 第二次请求，带 If-None-Match
    const headers = new Headers();
    headers.set("If-None-Match", etag!);

    const response2 = await serveStaticFileWithCache(
      TEST_FILES.css,
      DEFAULT_EXTENSIONS,
      headers,
      false,
    );

    // 应该返回 304 Not Modified
    assertEquals(response2?.status, 304);
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFileWithCache: If-Modified-Since 验证", async () => {
  await setupTestFiles();

  try {
    // 第一次请求
    const response1 = await serveStaticFileWithCache(
      TEST_FILES.css,
      DEFAULT_EXTENSIONS,
      new Headers(),
      false,
    );

    assertExists(response1);
    assertEquals(response1?.status, 200);

    const lastModified = response1?.headers.get("Last-Modified");
    assertExists(lastModified);

    // 第二次请求，带 If-Modified-Since（使用未来的时间）
    const headers = new Headers();
    const futureDate = new Date();
    futureDate.setTime(futureDate.getTime() + 10000); // 未来 10 秒
    headers.set("If-Modified-Since", futureDate.toUTCString());

    const response2 = await serveStaticFileWithCache(
      TEST_FILES.css,
      DEFAULT_EXTENSIONS,
      headers,
      false,
    );

    // 应该返回 304 Not Modified
    assertEquals(response2?.status, 304);
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFileWithCache: 开发模式不使用缓存", async () => {
  await setupTestFiles();

  try {
    const headers = new Headers();
    headers.set("If-None-Match", '"some-etag"');

    const response = await serveStaticFileWithCache(
      TEST_FILES.css,
      DEFAULT_EXTENSIONS,
      headers,
      true, // dev mode
    );

    assertExists(response);
    assertEquals(response?.status, 200);

    // 开发模式应该返回完整内容，不使用缓存
    const text = await response?.text();
    assertEquals(text, "body { color: red; }");
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - MIME 类型: 所有默认支持的类型", () => {
  const testCases = [
    ["file.html", "text/html; charset=utf-8"],
    ["file.css", "text/css; charset=utf-8"],
    ["file.js", "application/javascript; charset=utf-8"],
    ["file.json", "application/json; charset=utf-8"],
    ["file.png", "image/png"],
    ["file.jpg", "image/jpeg"],
    ["file.jpeg", "image/jpeg"],
    ["file.gif", "image/gif"],
    ["file.svg", "image/svg+xml"],
    ["file.ico", "image/x-icon"],
    ["file.webp", "image/webp"],
    ["file.woff", "font/woff"],
    ["file.woff2", "font/woff2"],
    ["file.ttf", "font/ttf"],
    ["file.eot", "application/vnd.ms-fontobject"],
    ["file.mp3", "audio/mpeg"],
    ["file.mp4", "video/mp4"],
    ["file.webm", "video/webm"],
    ["file.txt", "text/plain; charset=utf-8"],
    ["file.md", "text/markdown; charset=utf-8"],
    ["file.xml", "application/xml; charset=utf-8"],
  ];

  for (const [filename, expectedMime] of testCases) {
    const mimeType = getMimeType(filename);
    assertEquals(
      mimeType,
      expectedMime,
      `MIME type for ${filename} should be ${expectedMime}`,
    );
  }
});

console.log("\n✓ Static File 模块测试完成");
