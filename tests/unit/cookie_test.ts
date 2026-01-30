/**
 * Unit tests for cookie management module
 */

import { assertEquals, assertExists } from "https://deno.land/std@0.210.0/testing/asserts.ts";
import {
  serializeCookie,
  createCookieManager,
  extractSetCookieHeaders,
  type CookieManager,
  type CookieOptions,
} from "../../src/cookies.ts";

// Mock PageContext for testing
function createMockContext(): PageContext {
  return {
    method: "GET",
    url: new URL("http://example.com"),
    headers: new Headers(),
    query: {},
    body: null,
    cookies: {},
    file: "/test.tsx",
    root: "/www",
  };
}

// Helper to get headers with type safety
function getHeaders(ctx: PageContext): string[] {
  const headers = extractSetCookieHeaders(ctx);
  if (!headers) throw new Error("Expected headers to exist");
  return headers;
}

Deno.test("serializeCookie - basic cookie", () => {
  const result = serializeCookie("name", "value");
  assertEquals(result, "name=value");
});

Deno.test("serializeCookie - with URL encoding", () => {
  const result = serializeCookie("user name", "john doe");
  assertEquals(result, "user%20name=john%20doe");
});

Deno.test("serializeCookie - with maxAge", () => {
  const options: CookieOptions = { maxAge: 3600 };
  const result = serializeCookie("session", "abc123", options);
  assertEquals(result, "session=abc123; Max-Age=3600");
});

Deno.test("serializeCookie - with expires Date", () => {
  const date = new Date("2025-12-31T23:59:59Z");
  const options: CookieOptions = { expires: date };
  const result = serializeCookie("session", "abc123", options);
  assertEquals(result, `session=abc123; Expires=${date.toUTCString()}`);
});

Deno.test("serializeCookie - with expires string", () => {
  const expiresStr = "Fri, 31 Dec 2025 23:59:59 GMT";
  const options: CookieOptions = { expires: expiresStr };
  const result = serializeCookie("session", "abc123", options);
  assertEquals(result, `session=abc123; Expires=${expiresStr}`);
});

Deno.test("serializeCookie - with domain", () => {
  const options: CookieOptions = { domain: ".example.com" };
  const result = serializeCookie("session", "abc123", options);
  assertEquals(result, "session=abc123; Domain=.example.com");
});

Deno.test("serializeCookie - with path", () => {
  const options: CookieOptions = { path: "/" };
  const result = serializeCookie("session", "abc123", options);
  assertEquals(result, "session=abc123; Path=/");
});

Deno.test("serializeCookie - with secure", () => {
  const options: CookieOptions = { secure: true };
  const result = serializeCookie("session", "abc123", options);
  assertEquals(result, "session=abc123; Secure");
});

Deno.test("serializeCookie - with httpOnly", () => {
  const options: CookieOptions = { httpOnly: true };
  const result = serializeCookie("session", "abc123", options);
  assertEquals(result, "session=abc123; HttpOnly");
});

Deno.test("serializeCookie - with sameSite Strict", () => {
  const options: CookieOptions = { sameSite: "Strict" };
  const result = serializeCookie("session", "abc123", options);
  assertEquals(result, "session=abc123; SameSite=Strict");
});

Deno.test("serializeCookie - with sameSite Lax", () => {
  const options: CookieOptions = { sameSite: "Lax" };
  const result = serializeCookie("session", "abc123", options);
  assertEquals(result, "session=abc123; SameSite=Lax");
});

Deno.test("serializeCookie - with sameSite None", () => {
  const options: CookieOptions = { sameSite: "None" };
  const result = serializeCookie("session", "abc123", options);
  assertEquals(result, "session=abc123; SameSite=None");
});

Deno.test("serializeCookie - with all options", () => {
  const date = new Date("2025-12-31T23:59:59Z");
  const options: CookieOptions = {
    expires: date,
    domain: ".example.com",
    path: "/",
    secure: true,
    httpOnly: true,
    sameSite: "Strict",
  };
  const result = serializeCookie("session", "abc123", options);

  // Verify all parts are present (order may vary)
  assertEquals(result.includes("session=abc123"), true);
  assertEquals(result.includes(`Expires=${date.toUTCString()}`), true);
  assertEquals(result.includes("Domain=.example.com"), true);
  assertEquals(result.includes("Path=/"), true);
  assertEquals(result.includes("Secure"), true);
  assertEquals(result.includes("HttpOnly"), true);
  assertEquals(result.includes("SameSite=Strict"), true);
});

Deno.test("serializeCookie - maxAge takes precedence over expires", () => {
  const date = new Date("2025-12-31T23:59:59Z");
  const options: CookieOptions = {
    maxAge: 3600,
    expires: date,
  };
  const result = serializeCookie("session", "abc123", options);
  assertEquals(result.includes("Max-Age=3600"), true);
  assertEquals(result.includes("Expires="), false);
});

Deno.test("createCookieManager - set single cookie", () => {
  const ctx = createMockContext();
  const manager = createCookieManager(ctx);

  manager.set("username", "john_doe");

  const headers = getHeaders(ctx);
  if (!headers) throw new Error("headers should exist");
  assertEquals(headers.length, 1);
  assertEquals(headers[0], "username=john_doe");
});

Deno.test("createCookieManager - set multiple cookies", () => {
  const ctx = createMockContext();
  const manager = createCookieManager(ctx);

  manager.set("username", "john_doe");
  manager.set("session", "abc123");

  const headers = getHeaders(ctx);
  assertEquals(headers.length, 2);
  assertEquals(headers[0], "username=john_doe");
  assertEquals(headers[1], "session=abc123");
});

Deno.test("createCookieManager - set cookie with options", () => {
  const ctx = createMockContext();
  const manager = createCookieManager(ctx);

  manager.set("session", "abc123", {
    httpOnly: true,
    secure: true,
    sameSite: "Strict",
    maxAge: 3600,
  });

  const headers = getHeaders(ctx);
  assertEquals(headers.length, 1);
  // Check that all required parts are present (order may vary)
  const header = headers[0];
  assertEquals(header.includes("session=abc123"), true);
  assertEquals(header.includes("Max-Age=3600"), true);
  assertEquals(header.includes("HttpOnly"), true);
  assertEquals(header.includes("Secure"), true);
  assertEquals(header.includes("SameSite=Strict"), true);
});

Deno.test("createCookieManager - delete cookie", () => {
  const ctx = createMockContext();
  const manager = createCookieManager(ctx);

  manager.delete("session");

  const headers = getHeaders(ctx);
  assertEquals(headers.length, 1);

  // Should have maxAge=0 and (optionally) expires in the past
  // Note: when maxAge=0, it takes precedence, so Expires may or may not be present
  const header = headers[0];
  assertEquals(header.includes("session="), true);
  assertEquals(header.includes("Max-Age=0"), true);
});

Deno.test("createCookieManager - delete cookie with path and domain", () => {
  const ctx = createMockContext();
  const manager = createCookieManager(ctx);

  manager.delete("session", { path: "/", domain: ".example.com" });

  const headers = getHeaders(ctx);
  assertEquals(headers.length, 1);

  const header = headers[0];
  assertEquals(header.includes("Path=/"), true);
  assertEquals(header.includes("Domain=.example.com"), true);
  assertEquals(header.includes("Max-Age=0"), true);
});

Deno.test("createCookieManager - setMultiple", () => {
  const ctx = createMockContext();
  const manager = createCookieManager(ctx);

  manager.setMultiple({
    theme: { value: "dark", options: { maxAge: 31536000 } },
    language: { value: "zh-CN", options: { maxAge: 31536000 } },
  });

  const headers = getHeaders(ctx);
  assertEquals(headers.length, 2);
  assertEquals(headers[0], "theme=dark; Max-Age=31536000");
  assertEquals(headers[1], "language=zh-CN; Max-Age=31536000");
});

Deno.test("createCookieManager - deleteMultiple", () => {
  const ctx = createMockContext();
  const manager = createCookieManager(ctx);

  manager.deleteMultiple(["session1", "session2", "session3"]);

  const headers = getHeaders(ctx);
  assertEquals(headers.length, 3);

  // All should have Max-Age=0
  for (const header of headers) {
    assertEquals(header.includes("Max-Age=0"), true);
  }
});

Deno.test("createCookieManager - mixed operations", () => {
  const ctx = createMockContext();
  const manager = createCookieManager(ctx);

  manager.set("username", "john");
  manager.set("theme", "dark");
  manager.delete("old_session");
  manager.setMultiple({
    pref1: { value: "value1" },
    pref2: { value: "value2" },
  });
  manager.deleteMultiple(["temp1", "temp2"]);

  const headers = getHeaders(ctx);
  // 2 set calls + 1 delete + 2 from setMultiple + 2 from deleteMultiple = 7
  assertEquals(headers.length, 7);
});

Deno.test("extractSetCookieHeaders - returns undefined for no cookies", () => {
  const ctx = createMockContext();
  const headers = extractSetCookieHeaders(ctx);
  assertEquals(headers, undefined);
});

Deno.test("createCookieManager - special characters in name and value", () => {
  const ctx = createMockContext();
  const manager = createCookieManager(ctx);

  manager.set("user name", "john doe");
  manager.set("email", "test@example.com");

  const headers = getHeaders(ctx);
  assertEquals(headers.length, 2);
  assertEquals(headers[0], "user%20name=john%20doe");
  assertEquals(headers[1], "email=test%40example.com");
});

Deno.test("createCookieManager - unicode characters", () => {
  const ctx = createMockContext();
  const manager = createCookieManager(ctx);

  manager.set("name", "张三");
  manager.set("emoji", "😀");

  const headers = getHeaders(ctx);
  assertEquals(headers.length, 2);

  // Unicode should be percent-encoded
  assertEquals(headers[0].includes("%"), true);
  assertEquals(headers[1].includes("%"), true);
});

Deno.test("createCookieManager - empty value", () => {
  const ctx = createMockContext();
  const manager = createCookieManager(ctx);

  manager.set("empty", "");

  const headers = getHeaders(ctx);
  assertEquals(headers.length, 1);
  assertEquals(headers[0], "empty=");
});

Deno.test("createCookieManager - multiple contexts are isolated", () => {
  const ctx1 = createMockContext();
  const ctx2 = createMockContext();

  const manager1 = createCookieManager(ctx1);
  const manager2 = createCookieManager(ctx2);

  manager1.set("cookie1", "value1");
  manager2.set("cookie2", "value2");

  const headers1 = getHeaders(ctx1);
  const headers2 = getHeaders(ctx2);

  assertEquals(headers1.length, 1);
  assertEquals(headers2.length, 1);
  assertEquals(headers1[0], "cookie1=value1");
  assertEquals(headers2[0], "cookie2=value2");
});
