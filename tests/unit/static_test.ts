/**
 * Static file module unit tests
 * Tests static file service in src/static.ts
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

// Temporary file paths for testing
const TEST_DIR = "./test_static_files";
const TEST_FILES = {
  css: `${TEST_DIR}/test.css`,
  js: `${TEST_DIR}/test.js`,
  png: `${TEST_DIR}/test.png`,
  html: `${TEST_DIR}/test.html`,
  txt: `${TEST_DIR}/test.txt`,
};

// Default allowed extensions list
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
 * Create test files
 */
async function setupTestFiles() {
  // Create test directory
  await Deno.mkdir(TEST_DIR, { recursive: true });

  // Create various test files
  await Deno.writeTextFile(TEST_FILES.css, "body { color: red; }");
  await Deno.writeTextFile(TEST_FILES.js, "console.log('test');");
  await Deno.writeTextFile(TEST_FILES.txt, "test content");
  await Deno.writeTextFile(TEST_FILES.html, "<html></html>");

  // Create a fake image file (binary)
  const pngData = new Uint8Array([0x89, 0x50, 0x4e, 0x47]); // PNG signature
  await Deno.writeFile(TEST_FILES.png, pngData);
}

/**
 * Cleanup test files
 */
async function cleanupTestFiles() {
  try {
    await Deno.remove(TEST_DIR, { recursive: true });
  } catch {
    // Directory does not exist, ignore
  }
}

Deno.test("static - getMimeType: CSS file", () => {
  const mimeType = getMimeType("style.css");
  assertEquals(mimeType, "text/css; charset=utf-8");
});

Deno.test("static - getMimeType: JavaScript file", () => {
  const mimeType = getMimeType("app.js");
  assertEquals(mimeType, "application/javascript; charset=utf-8");
});

Deno.test("static - getMimeType: JSON file", () => {
  const mimeType = getMimeType("data.json");
  assertEquals(mimeType, "application/json; charset=utf-8");
});

Deno.test("static - getMimeType: PNG image", () => {
  const mimeType = getMimeType("image.png");
  assertEquals(mimeType, "image/png");
});

Deno.test("static - getMimeType: JPG image", () => {
  const mimeType = getMimeType("photo.jpg");
  assertEquals(mimeType, "image/jpeg");
});

Deno.test("static - getMimeType: JPEG image", () => {
  const mimeType = getMimeType("photo.jpeg");
  assertEquals(mimeType, "image/jpeg");
});

Deno.test("static - getMimeType: SVG image", () => {
  const mimeType = getMimeType("icon.svg");
  assertEquals(mimeType, "image/svg+xml");
});

Deno.test("static - getMimeType: ICO icon", () => {
  const mimeType = getMimeType("favicon.ico");
  assertEquals(mimeType, "image/x-icon");
});

Deno.test("static - getMimeType: WOFF font", () => {
  const mimeType = getMimeType("font.woff");
  assertEquals(mimeType, "font/woff");
});

Deno.test("static - getMimeType: WOFF2 font", () => {
  const mimeType = getMimeType("font.woff2");
  assertEquals(mimeType, "font/woff2");
});

Deno.test("static - getMimeType: text file", () => {
  const mimeType = getMimeType("readme.txt");
  assertEquals(mimeType, "text/plain; charset=utf-8");
});

Deno.test("static - getMimeType: Markdown file", () => {
  const mimeType = getMimeType("doc.md");
  assertEquals(mimeType, "text/markdown; charset=utf-8");
});

Deno.test("static - getMimeType: unknown type", () => {
  const mimeType = getMimeType("file.unknown");
  assertEquals(mimeType, "application/octet-stream");
});

Deno.test("static - isStaticFileAllowed: allowed extensions", () => {
  const allowed = isStaticFileAllowed("style.css", DEFAULT_EXTENSIONS);
  assertEquals(allowed, true);
});

Deno.test("static - isStaticFileAllowed: disallowed extensions", () => {
  const allowed = isStaticFileAllowed("page.tsx", DEFAULT_EXTENSIONS);
  assertEquals(allowed, false);
});

Deno.test("static - isStaticFileAllowed: empty list", () => {
  const allowed = isStaticFileAllowed("style.css", []);
  assertEquals(allowed, false);
});

Deno.test("static - isStaticFileAllowed: case insensitive", () => {
  const allowed = isStaticFileAllowed("IMAGE.PNG", DEFAULT_EXTENSIONS);
  assertEquals(allowed, true);
});

Deno.test("static - serveStaticFile: read CSS file", async () => {
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

    // Verify ETag exists (production mode)
    assertExists(response?.headers.get("ETag"));
    assertExists(response?.headers.get("Last-Modified"));
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFile: read JS file", async () => {
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

Deno.test("static - serveStaticFile: read text file", async () => {
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

    // Verify content
    const text = await response?.text();
    assertEquals(text, "test content");
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFile: dev mode no cache", async () => {
  await setupTestFiles();

  try {
    const response = await serveStaticFile(
      TEST_FILES.css,
      DEFAULT_EXTENSIONS,
      true, // dev mode
    );

    assertExists(response);
    assertEquals(response?.status, 200);

    // Dev mode should disable cache
    assertEquals(
      response?.headers.get("Cache-Control"),
      "no-cache, no-store, must-revalidate",
    );
    assertEquals(response?.headers.get("Pragma"), "no-cache");
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFile: disallowed file type", async () => {
  await setupTestFiles();

  try {
    const response = await serveStaticFile(
      TEST_FILES.html, // HTML not in default list
      DEFAULT_EXTENSIONS,
      false,
    );

    // Should return null (not handled)
    assertEquals(response, null);
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFile: file not found", async () => {
  await setupTestFiles();

  try {
    const response = await serveStaticFile(
      `${TEST_DIR}/notexist.css`,
      DEFAULT_EXTENSIONS,
      false,
    );

    // File not found should return null
    assertEquals(response, null);
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFileWithCache: ETag validation", async () => {
  await setupTestFiles();

  try {
    // First request
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

    // Second request with If-None-Match
    const headers = new Headers();
    headers.set("If-None-Match", etag!);

    const response2 = await serveStaticFileWithCache(
      TEST_FILES.css,
      DEFAULT_EXTENSIONS,
      headers,
      false,
    );

    // Should return 304 Not Modified
    assertEquals(response2?.status, 304);
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFileWithCache: If-Modified-Since validation", async () => {
  await setupTestFiles();

  try {
    // First request
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

    // Second request with If-Modified-Since (using future time)
    const headers = new Headers();
    const futureDate = new Date();
    futureDate.setTime(futureDate.getTime() + 10000); // Future 10 seconds
    headers.set("If-Modified-Since", futureDate.toUTCString());

    const response2 = await serveStaticFileWithCache(
      TEST_FILES.css,
      DEFAULT_EXTENSIONS,
      headers,
      false,
    );

    // Should return 304 Not Modified
    assertEquals(response2?.status, 304);
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - serveStaticFileWithCache: dev mode does not use cache", async () => {
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

    // Dev mode should return full content, not using cache
    const text = await response?.text();
    assertEquals(text, "body { color: red; }");
  } finally {
    await cleanupTestFiles();
  }
});

Deno.test("static - MIME type: all default supported types", () => {
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

console.log("\n✓ Static File module tests completed");
