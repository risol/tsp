import { createResponseHelper } from "../../src/response.ts";
import { buildContext } from "../../src/context.ts";
import { assertEquals, assertExists } from "https://deno.land/std@0.210.0/testing/asserts.ts";

// 创建测试上下文
function createMockContext() {
  return buildContext({
    method: "GET",
    url: new URL("http://localhost:9000/test"),
    headers: new Headers(),
    query: {},
    body: null,
    cookies: {},
    file: "/test.tsx",
    root: "/www",
  });
}

Deno.test("response.json() creates JSON response", async () => {
  const ctx = createMockContext();
  const response = createResponseHelper(ctx);
  const result = response.json({ message: "Hello" });

  assertEquals(result.status, 200);
  assertEquals(result.headers.get("Content-Type"), "application/json; charset=utf-8");

  const body = await result.json();
  assertEquals(body, { message: "Hello" });
});

Deno.test("response.json() with custom status", async () => {
  const ctx = createMockContext();
  const response = createResponseHelper(ctx);
  const result = response.json({ error: "Not found" }, 404);

  assertEquals(result.status, 404);
  assertEquals(result.headers.get("Content-Type"), "application/json; charset=utf-8");

  const body = await result.json();
  assertEquals(body.error, "Not found");
});

Deno.test("response.text() creates text response", async () => {
  const ctx = createMockContext();
  const response = createResponseHelper(ctx);
  const result = response.text("Hello, World!");

  assertEquals(result.status, 200);
  assertEquals(result.headers.get("Content-Type"), "text/plain; charset=utf-8");

  const body = await result.text();
  assertEquals(body, "Hello, World!");
});

Deno.test("response.html() creates HTML response", async () => {
  const ctx = createMockContext();
  const response = createResponseHelper(ctx);
  const result = response.html("<h1>Hello</h1>");

  assertEquals(result.status, 200);
  assertEquals(result.headers.get("Content-Type"), "text/html; charset=utf-8");

  const body = await result.text();
  assertEquals(body, "<h1>Hello</h1>");
});

Deno.test("response.error() creates error response", async () => {
  const ctx = createMockContext();
  const response = createResponseHelper(ctx);
  const result = response.error("Not found", 404);

  assertEquals(result.status, 404);

  const body = await result.json();
  assertEquals(body.error, "Not found");
  assertExists(body.timestamp);
});

Deno.test("response.redirect() returns RedirectResult", () => {
  const ctx = createMockContext();
  const response = createResponseHelper(ctx);
  const result = response.redirect("/login");

  assertEquals(result.redirect, "/login");
  assertEquals(result.status, 302);
});

Deno.test("response.redirect() with custom status", () => {
  const ctx = createMockContext();
  const response = createResponseHelper(ctx);
  const result = response.redirect("/dashboard", 301);

  assertEquals(result.redirect, "/dashboard");
  assertEquals(result.status, 301);
});

Deno.test("response.file() creates file download response", async () => {
  const ctx = createMockContext();
  const response = createResponseHelper(ctx);
  const result = response.file("Hello, World!", "test.txt");

  assertEquals(result.status, 200);
  assertEquals(result.headers.get("Content-Disposition"), 'attachment; filename="test.txt"');
  assertEquals(result.headers.get("Content-Type"), "text/plain; charset=utf-8");

  const body = await result.text();
  assertEquals(body, "Hello, World!");
});

Deno.test("response.file() with binary content", async () => {
  const ctx = createMockContext();
  const response = createResponseHelper(ctx);
  const content = new Uint8Array([72, 101, 108, 108, 111]); // "Hello"
  const result = response.file(content, "test.bin");

  assertEquals(result.status, 200);
  assertEquals(result.headers.get("Content-Type"), "application/octet-stream");
  assertEquals(result.headers.get("Content-Disposition"), 'attachment; filename="test.bin"');

  const body = await result.arrayBuffer();
  assertEquals(new Uint8Array(body), content);
});

Deno.test("response.noContent() returns 204", () => {
  const ctx = createMockContext();
  const response = createResponseHelper(ctx);
  const result = response.noContent();

  assertEquals(result.status, 204);
});

Deno.test("response.custom() creates custom Response", async () => {
  const ctx = createMockContext();
  const response = createResponseHelper(ctx);
  const result = response.custom("Custom body", { status: 418 });

  assertEquals(result.status, 418);
  const body = await result.text();
  assertEquals(body, "Custom body");
});

Deno.test("response.json() with custom headers", async () => {
  const ctx = createMockContext();
  const response = createResponseHelper(ctx);
  const result = response.json({ data: "test" }, 200, {
    "X-Custom-Header": "custom-value",
  });

  assertEquals(result.headers.get("X-Custom-Header"), "custom-value");
  assertEquals(result.headers.get("Content-Type"), "application/json; charset=utf-8");

  const body = await result.json();
  assertEquals(body, { data: "test" });
});
