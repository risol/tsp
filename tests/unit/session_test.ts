/**
 * Session Module Unit Tests
 */

import { assertEquals, assertExists } from "@std/assert";
import {
  createSessionManager,
  getDefaultOptions,
  SessionStore,
} from "../../src/session.ts";

// Mock PageContext for testing
function createMockContext(sessionCookie = "") {
  const cookiesMap: Record<string, string> = {};

  const ctx = {
    method: "GET" as const,
    url: new URL("http://localhost:9000/test"),
    headers: new Headers(),
    query: {},
    body: null,
    cookies: {
      "tsp_session": sessionCookie,
    } as Record<string, string>,
    file: "/test.tsx",
    root: "./www",
  };

  // Create mock cookie manager
  const cookieManager = {
    set: (name: string, value: string) => {
      cookiesMap[name] = value;
    },
    delete: (name: string) => {
      delete cookiesMap[name];
    },
    getCookies: () => cookiesMap,
  };

  return { ctx, cookieManager };
}

// Test helper to create store with auto-cleanup
function createTestStore() {
  const options = {
    ...getDefaultOptions(),
    secret: new TextEncoder().encode(
      "test-secret-key-for-testing-min-32-chars",
    ),
    cleanupInterval: 1000, // Shorter interval for tests
  };
  const store = new SessionStore(options);

  // Return store with cleanup wrapper
  return {
    store,
    async cleanup() {
      store.cleanupStore();
      await new Promise((resolve) => setTimeout(resolve, 10)); // Small delay to ensure cleanup
    },
  };
}

Deno.test("SessionStore: create and get session", async () => {
  const options = {
    ...getDefaultOptions(),
    secret: new TextEncoder().encode(
      "test-secret-key-for-testing-min-32-chars",
    ),
  };
  const store = new SessionStore(options);

  try {
    // Create session
    const session = await store.create("user-123", { name: "Test User" });
    assertExists(session);
    assertEquals(session.rawId.length, 36); // UUID length
    assertEquals(session.data.get("user"), {
      id: "user-123",
      name: "Test User",
    });

    // Get session
    const retrieved = await store.get(session.id);
    assertExists(retrieved);
    assertEquals(retrieved?.rawId, session.rawId);
    assertEquals(retrieved?.data.get("user"), session.data.get("user"));
  } finally {
    store.cleanupStore();
  }
});

Deno.test("SessionStore: signature verification", async () => {
  const { store, cleanup } = createTestStore();

  try {
    // Create session with valid signature
    const session = await store.create("user-123");
    const retrieved = await store.get(session.id);
    assertExists(retrieved);

    // Try to access with tampered ID (wrong signature)
    const tamperedId = session.id.slice(0, -5) + "xxxxx";
    const tamperedResult = await store.get(tamperedId);
    assertEquals(tamperedResult, null);

    // Try invalid format (no signature)
    const noSignatureResult = await store.get("invalid-id");
    assertEquals(noSignatureResult, null);
  } finally {
    await cleanup();
  }
});

Deno.test("SessionStore: destroy session", async () => {
  const { store, cleanup } = createTestStore();

  try {
    const session = await store.create("user-123");

    // Verify session exists
    let retrieved = await store.get(session.id);
    assertExists(retrieved);

    // Destroy session
    await store.destroy(session.id);

    // Verify session is gone
    retrieved = await store.get(session.id);
    assertEquals(retrieved, null);
  } finally {
    await cleanup();
  }
});

Deno.test("SessionStore: touch refreshes expiration", async () => {
  const options = {
    ...getDefaultOptions(),
    secret: new TextEncoder().encode(
      "test-secret-key-for-testing-min-32-chars",
    ),
    maxAge: 2, // 2 seconds
    cleanupInterval: 1000,
  };
  const store = new SessionStore(options);

  try {
    const session = await store.create("user-123");
    const originalExpiresAt = session.expiresAt;

    // Wait a bit
    await new Promise((resolve) => setTimeout(resolve, 1000));

    // Touch session
    await store.touch(session.id);

    // Get updated session
    const retrieved = await store.get(session.id);
    assertExists(retrieved);
    if (retrieved) {
      assertEquals(retrieved.expiresAt > originalExpiresAt, true);
    }
  } finally {
    store.cleanupStore();
  }
});

Deno.test("SessionStore: session expiration", async () => {
  const options = {
    ...getDefaultOptions(),
    secret: new TextEncoder().encode(
      "test-secret-key-for-testing-min-32-chars",
    ),
    maxAge: 1, // 1 second
    cleanupInterval: 1000,
  };
  const store = new SessionStore(options);

  try {
    const session = await store.create("user-123");

    // Session should be valid immediately
    let retrieved = await store.get(session.id);
    assertExists(retrieved);

    // Wait for expiration
    await new Promise((resolve) => setTimeout(resolve, 1500));

    // Session should be expired
    retrieved = await store.get(session.id);
    assertEquals(retrieved, null);
  } finally {
    store.cleanupStore();
  }
});

Deno.test("SessionStore: regenerateId", async () => {
  const { store, cleanup } = createTestStore();

  try {
    const session = await store.create("user-123");
    const oldId = session.id;

    // Regenerate ID
    const newId = await store.regenerateId(oldId);

    // IDs should be different
    assertEquals(newId !== oldId, true);

    // Old ID should not work
    const oldRetrieved = await store.get(oldId);
    assertEquals(oldRetrieved, null);

    // New ID should work and have same data
    const newRetrieved = await store.get(newId);
    assertExists(newRetrieved);
    if (newRetrieved) {
      assertEquals(newRetrieved.data.get("user"), session.data.get("user"));
    }
  } finally {
    await cleanup();
  }
});

Deno.test("SessionManager: getUser", async () => {
  const { store, cleanup } = createTestStore();

  try {
    // Create session
    const session = await store.create("user-456", {
      name: "John Doe",
      email: "john@example.com",
    });

    // Create manager with session cookie
    const { ctx, cookieManager } = createMockContext(session.id);
    const manager = await createSessionManager(ctx, store, cookieManager);

    // Get user
    const user = await manager.getUser();
    assertExists(user);
    assertEquals(user?.id, "user-456");
    assertEquals(user?.name, "John Doe");
    assertEquals(user?.email, "john@example.com");
  } finally {
    await cleanup();
  }
});

Deno.test("SessionManager: getUser without session", async () => {
  const { store, cleanup } = createTestStore();

  try {
    // Create manager without session cookie
    const { ctx, cookieManager } = createMockContext();
    const manager = await createSessionManager(ctx, store, cookieManager);

    // Get user should return null
    const user = await manager.getUser();
    assertEquals(user, null);
  } finally {
    await cleanup();
  }
});

Deno.test("SessionManager: login", async () => {
  const { store, cleanup } = createTestStore();

  try {
    // Create manager without session
    const { ctx, cookieManager } = createMockContext();
    const manager = await createSessionManager(ctx, store, cookieManager);

    // Login
    await manager.login("user-789", {
      name: "Jane Doe",
      email: "jane@example.com",
    });

    // Check user is logged in
    const user = await manager.getUser();
    assertExists(user);
    assertEquals(user?.id, "user-789");
    assertEquals(user?.name, "Jane Doe");

    // Check cookie was set
    const setCookies = cookieManager.getCookies();
    assertExists(setCookies["tsp_session"]);
  } finally {
    await cleanup();
  }
});

Deno.test("SessionManager: logout", async () => {
  const { store, cleanup } = createTestStore();

  try {
    // Create session
    const session = await store.create("user-999");

    // Create manager
    const { ctx, cookieManager } = createMockContext(session.id);
    const manager = await createSessionManager(ctx, store, cookieManager);

    // Verify user exists
    let user = await manager.getUser();
    assertExists(user);

    // Logout
    await manager.logout();

    // Verify user is gone
    user = await manager.getUser();
    assertEquals(user, null);

    // Verify session is destroyed
    const retrieved = await store.get(session.id);
    assertEquals(retrieved, null);
  } finally {
    await cleanup();
  }
});

Deno.test("SessionManager: set and get data", async () => {
  const { store, cleanup } = createTestStore();

  try {
    // Create session
    const session = await store.create("user-111");

    // Create manager
    const { ctx, cookieManager } = createMockContext(session.id);
    const manager = await createSessionManager(ctx, store, cookieManager);

    // Set data
    await manager.set("visits", 5);
    await manager.set("cart", ["item1", "item2"]);
    await manager.set("preferences", { theme: "dark" });

    // Get data
    const visits = await manager.get<number>("visits");
    assertEquals(visits, 5);

    const cart = await manager.get<string[]>("cart");
    assertEquals(cart, ["item1", "item2"]);

    const prefs = await manager.get<{ theme: string }>("preferences");
    assertEquals(prefs?.theme, "dark");

    // Get non-existent key
    const nothing = await manager.get("nonexistent");
    assertEquals(nothing, null);
  } finally {
    await cleanup();
  }
});

Deno.test("SessionManager: delete data", async () => {
  const { store, cleanup } = createTestStore();

  try {
    // Create session
    const session = await store.create("user-222");

    // Create manager
    const { ctx, cookieManager } = createMockContext(session.id);
    const manager = await createSessionManager(ctx, store, cookieManager);

    // Set and delete data
    await manager.set("temp", "value");
    let temp = await manager.get("temp");
    assertEquals(temp, "value");

    await manager.delete("temp");
    temp = await manager.get("temp");
    assertEquals(temp, null);
  } finally {
    await cleanup();
  }
});

Deno.test("SessionManager: regenerateId", async () => {
  const { store, cleanup } = createTestStore();

  try {
    // Create session
    const session = await store.create("user-333");

    // Create manager
    const { ctx, cookieManager } = createMockContext(session.id);
    const manager = await createSessionManager(ctx, store, cookieManager);

    const oldId = manager.getId();
    assertEquals(oldId, session.id);

    // Regenerate
    await manager.regenerateId();

    const newId = manager.getId();
    assertEquals(newId !== oldId, true);

    // Old ID should not work
    const oldRetrieved = await store.get(oldId);
    assertEquals(oldRetrieved, null);

    // New ID should work
    const newRetrieved = await store.get(newId);
    assertExists(newRetrieved);
  } finally {
    await cleanup();
  }
});

Deno.test("SessionManager: touch", async () => {
  const options = {
    ...getDefaultOptions(),
    secret: new TextEncoder().encode(
      "test-secret-key-for-testing-min-32-chars",
    ),
    maxAge: 2,
    cleanupInterval: 1000,
  };
  const store = new SessionStore(options);

  try {
    // Create session
    const session = await store.create("user-444");
    const originalExpiresAt = session.expiresAt;

    // Create manager
    const { ctx, cookieManager } = createMockContext(session.id);
    const manager = await createSessionManager(ctx, store, cookieManager);

    // Wait a bit
    await new Promise((resolve) => setTimeout(resolve, 1000));

    // Touch
    await manager.touch();

    // Get updated session
    const retrieved = await store.get(manager.getId());
    assertExists(retrieved);
    if (retrieved) {
      assertEquals(retrieved.expiresAt > originalExpiresAt, true);
    }
  } finally {
    store.cleanupStore();
  }
});

Deno.test("SessionManager: isValid", async () => {
  const { store, cleanup } = createTestStore();

  try {
    // Test with valid session
    const session = await store.create("user-555");
    const { ctx, cookieManager } = createMockContext(session.id);
    const manager = await createSessionManager(ctx, store, cookieManager);

    let valid = await manager.isValid();
    assertEquals(valid, true);

    // Test without session
    const { ctx: ctx2, cookieManager: cookieManager2 } = createMockContext();
    const manager2 = await createSessionManager(ctx2, store, cookieManager2);

    valid = await manager2.isValid();
    assertEquals(valid, false);
  } finally {
    await cleanup();
  }
});

Deno.test("SessionManager: set without session throws error", async () => {
  const { store, cleanup } = createTestStore();

  try {
    // Create manager without session
    const { ctx, cookieManager } = createMockContext();
    const manager = await createSessionManager(ctx, store, cookieManager);

    // Try to set data without session
    let errorThrown = false;
    try {
      await manager.set("key", "value");
    } catch (error) {
      errorThrown = true;
      assertEquals(error instanceof Error, true);
      if (error instanceof Error) {
        assertEquals(error.message, "No active session");
      }
    }

    assertEquals(errorThrown, true);
  } finally {
    await cleanup();
  }
});

Deno.test("SessionManager: login with existing session updates user", async () => {
  const { store, cleanup } = createTestStore();

  try {
    // Create session as user1
    const session = await store.create("user-1", { name: "User One" });

    // Create manager
    const { ctx, cookieManager } = createMockContext(session.id);
    const manager = await createSessionManager(ctx, store, cookieManager);

    // Verify initial user
    let user = await manager.getUser();
    assertEquals(user?.id, "user-1");
    assertEquals(user?.name, "User One");

    // Login as different user
    await manager.login("user-2", { name: "User Two" });

    // Verify user updated
    user = await manager.getUser();
    assertEquals(user?.id, "user-2");
    assertEquals(user?.name, "User Two");
  } finally {
    await cleanup();
  }
});
