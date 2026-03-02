/**
 * Context module unit tests
 * Tests context building in src/context.ts
 */

import {
  assertEquals,
  assertExists,
} from "https://deno.land/std@0.210.0/testing/asserts.ts";
import { buildContext, type InternalPageContext } from "../../src/context.ts";

// Use paths relative to project root
const TEST_ROOT = "./www";

/**
 * Helper function: Build context from Request
 */
async function buildContextFromRequest(
  request: Request,
  file: string,
  root: string,
): Promise<InternalPageContext> {
  const url = new URL(request.url);

  // Parse query parameters
  const query: Record<string, string> = {};
  url.searchParams.forEach((value, key) => {
    query[key] = value;
  });

  // Parse cookies
  const cookies: Record<string, string> = {};
  const cookieHeader = request.headers.get("Cookie");
  if (cookieHeader) {
    cookieHeader.split(";").forEach((cookie) => {
      const [key, value] = cookie.trim().split("=");
      if (key && value) {
        cookies[key] = decodeURIComponent(value);
      }
    });
  }

  // Parse request body
  let body: unknown = null;
  const method = request.method as
    | "GET"
    | "POST"
    | "PUT"
    | "PATCH"
    | "DELETE"
    | "HEAD"
    | "OPTIONS";

  if (method !== "GET" && method !== "HEAD" && method !== "DELETE") {
    const contentType = request.headers.get("Content-Type") || "";

    if (contentType.includes("application/json")) {
      try {
        body = await request.json();
      } catch {
        body = null;
      }
    } else if (contentType.includes("application/x-www-form-urlencoded")) {
      const text = await request.text();
      const params = new URLSearchParams(text);
      const formData: Record<string, string> = {};
      params.forEach((value, key) => {
        formData[key] = value;
      });
      body = formData;
    } else {
      body = await request.text();
    }
  }

  return buildContext({
    method,
    url,
    headers: request.headers,
    query,
    body,
    cookies,
    files: {},
    file,
    root,
  });
}

Deno.test("context - buildContext: basic request", async () => {
  const request = new Request("http://localhost:9000/?name=test");
  const context = await buildContextFromRequest(
    request,
    "/test.tsx",
    TEST_ROOT,
  );

  assertEquals(context.method, "GET");
  assertEquals(context._query.name, "test");
  assertEquals(context.file, "/test.tsx");
  assertEquals(context.root, TEST_ROOT);
});

Deno.test("context - buildContext: multiple query parameters", async () => {
  const request = new Request(
    "http://localhost:9000/?name=test&page=1&limit=10",
  );
  const context = await buildContextFromRequest(
    request,
    "/test.tsx",
    TEST_ROOT,
  );

  assertEquals(context._query.name, "test");
  assertEquals(context._query.page, "1");
  assertEquals(context._query.limit, "10");
});

Deno.test("context - buildContext: POST JSON request", async () => {
  const request = new Request("http://localhost:9000/", {
    method: "POST",
    body: JSON.stringify({ username: "test", age: 25 }),
    headers: {
      "Content-Type": "application/json",
    },
  });

  const context = await buildContextFromRequest(
    request,
    "/test.tsx",
    TEST_ROOT,
  );

  assertEquals(context.method, "POST");
  assertExists(context._body);
  assertEquals((context._body as Record<string, unknown>).username, "test");
  assertEquals((context._body as Record<string, unknown>).age, 25);
});

Deno.test("context - buildContext: POST form request", async () => {
  const request = new Request("http://localhost:9000/", {
    method: "POST",
    body: "username=test&password=secret",
    headers: {
      "Content-Type": "application/x-www-form-urlencoded",
    },
  });

  const context = await buildContextFromRequest(
    request,
    "/test.tsx",
    TEST_ROOT,
  );

  assertEquals(context.method, "POST");
  assertExists(context._body);
  assertEquals((context._body as Record<string, string>).username, "test");
  assertEquals((context._body as Record<string, string>).password, "secret");
});

Deno.test("context - buildContext: cookie parsing", async () => {
  const request = new Request("http://localhost:9000/", {
    headers: {
      "Cookie": "sessionId=abc123; theme=dark; lang=zh-CN",
    },
  });

  const context = await buildContextFromRequest(
    request,
    "/test.tsx",
    TEST_ROOT,
  );

  assertEquals(context.cookies.sessionId, "abc123");
  assertEquals(context.cookies.theme, "dark");
  assertEquals(context.cookies.lang, "zh-CN");
});

Deno.test("context - buildContext: multiple request headers", async () => {
  const request = new Request("http://localhost:9000/", {
    headers: {
      "User-Agent": "Mozilla/5.0",
      "Accept": "text/html",
      "Accept-Language": "zh-CN",
    },
  });

  const context = await buildContextFromRequest(
    request,
    "/test.tsx",
    TEST_ROOT,
  );

  assertEquals(context.headers.get("User-Agent"), "Mozilla/5.0");
  assertEquals(context.headers.get("Accept"), "text/html");
  assertEquals(context.headers.get("Accept-Language"), "zh-CN");
});

console.log("\n✓ Context module tests completed");
