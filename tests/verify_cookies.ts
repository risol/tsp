/**
 * Quick verification script for cookie functionality
 * This tests that the cookie module integrates correctly with the server
 */

import { assertEquals } from "https://deno.land/std@0.210.0/testing/asserts.ts";
import {
  serializeCookie,
  createCookieManager,
  extractSetCookieHeaders,
} from "../src/cookies.ts";

// Mock PageContext
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

console.log("🍪 Running cookie functionality verification...\n");

// Test 1: Basic serialization
console.log("✓ Test 1: Cookie serialization");
const basic = serializeCookie("test", "value");
assertEquals(basic, "test=value");
console.log(`  Result: ${basic}`);

// Test 2: Cookie with options
console.log("\n✓ Test 2: Cookie with options");
const withOptions = serializeCookie("session", "abc123", {
  httpOnly: true,
  secure: true,
  maxAge: 3600,
});
console.log(`  Result: ${withOptions}`);
assertEquals(withOptions.includes("session=abc123"), true);
assertEquals(withOptions.includes("HttpOnly"), true);
assertEquals(withOptions.includes("Secure"), true);
assertEquals(withOptions.includes("Max-Age=3600"), true);

// Test 3: Cookie manager
console.log("\n✓ Test 3: Cookie manager integration");
const ctx = createMockContext();
const manager = createCookieManager(ctx);

manager.set("username", "john");
manager.set("theme", "dark");
manager.delete("old_session");

const headers = extractSetCookieHeaders(ctx);
console.log(`  Cookies set: ${headers?.length || 0}`);
assertEquals(headers?.length, 3);

// Test 4: URL encoding
console.log("\n✓ Test 4: URL encoding");
const encoded = serializeCookie("user name", "John Doe");
assertEquals(encoded, "user%20name=John%20Doe");
console.log(`  Result: ${encoded}`);

// Test 5: Special characters
console.log("\n✓ Test 5: Special characters");
const special = serializeCookie("email", "test@example.com");
assertEquals(special, "email=test%40example.com");
console.log(`  Result: ${special}`);

console.log("\n✅ All cookie functionality tests passed!\n");
console.log("The cookie module is ready for integration.");
console.log("Visit http://localhost:9000/cookie_test.tsx for E2E testing.\n");
