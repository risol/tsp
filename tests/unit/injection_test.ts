/**
 * Injection module unit tests
 * Tests dependency injection in src/injection-typed.ts
 */

import {
  assertEquals,
  assertExists,
} from "https://deno.land/std@0.210.0/testing/asserts.ts";
import { type InternalPageContext, type PageContext } from "../../src/context.ts";
import {
  getRegisteredDeps,
  Page,
  registerDep,
  unregisterDep,
} from "../../src/injection-typed.ts";

// Create a simple InternalPageContext for testing
function createMockContext(pathname: string = "/"): InternalPageContext {
  return {
    method: "GET",
    url: new URL(`http://localhost:9000${pathname}`),
    headers: new Headers(),
    cookies: {},
    files: {},
    file: "/test.tsx",
    root: "./www",
    // Internal fields
    _query: {},
    _body: null,
  };
}

Deno.test("injection - registerDep: register single dependency", () => {
  const testFn = () => "test";
  registerDep("testFunc" as never, testFn as never);

  const deps = getRegisteredDeps();
  assertEquals(deps.includes("testFunc"), true);

  // Cleanup
  unregisterDep("testFunc" as never);
});

Deno.test("injection - registerDep: register multiple dependencies", () => {
  registerDep("func1" as never, () => "func1");
  registerDep("func2" as never, () => "func2");
  registerDep("func3" as never, () => "func3");

  const deps = getRegisteredDeps();
  assertEquals(deps.length >= 3, true);
  assertEquals(deps.includes("func1"), true);
  assertEquals(deps.includes("func2"), true);
  assertEquals(deps.includes("func3"), true);

  // Cleanup
  unregisterDep("func1" as never);
  unregisterDep("func2" as never);
  unregisterDep("func3" as never);
});

Deno.test("injection - unregisterDep: unregister dependency", () => {
  registerDep("tempFunc" as never, () => "temp");

  let deps = getRegisteredDeps();
  assertEquals(deps.includes("tempFunc"), true);

  unregisterDep("tempFunc" as never);

  deps = getRegisteredDeps();
  assertEquals(deps.includes("tempFunc"), false);
});

Deno.test("injection - getRegisteredDeps: get registered dependency list", () => {
  // Get current dependency count
  const beforeCount = getRegisteredDeps().length;

  registerDep("dep1" as never, () => "1");
  registerDep("dep2" as never, () => "2");

  const deps = getRegisteredDeps();
  assertEquals(deps.length, beforeCount + 2);
  assertEquals(deps.includes("dep1"), true);
  assertEquals(deps.includes("dep2"), true);

  // Cleanup
  unregisterDep("dep1" as never);
  unregisterDep("dep2" as never);
});

Deno.test("injection - Page: single dependency injection", async () => {
  const testFunc = () => "testFunc called";
  registerDep("testFunc" as never, (ctx) => testFunc);

  const wrapper = Page((ctx: PageContext, deps: AppDeps) => {
    const fn = deps.testFunc as () => string;
    return fn();
  });

  const context = createMockContext();
  const result = await wrapper(context);

  assertEquals(result, "testFunc called");

  // Cleanup
  unregisterDep("testFunc" as never);
});

Deno.test("injection - Page: multiple dependency injection", async () => {
  const func1 = () => "func1";
  const func2 = () => "func2";
  const func3 = () => "func3";

  registerDep("func1" as never, () => func1);
  registerDep("func2" as never, () => func2);
  registerDep("func3" as never, () => func3);

  const wrapper = Page((ctx: PageContext, deps: AppDeps) => {
    const f1 = deps.func1 as () => string;
    const f2 = deps.func2 as () => string;
    const f3 = deps.func3 as () => string;
    return [f1(), f2(), f3()];
  });

  const context = createMockContext();
  const result = await wrapper(context);

  assertEquals(result, ["func1", "func2", "func3"]);

  // Cleanup
  unregisterDep("func1" as never);
  unregisterDep("func2" as never);
  unregisterDep("func3" as never);
});

Deno.test("injection - Page: async dependency building", async () => {
  const asyncFunc = async () => {
    await new Promise((resolve) => setTimeout(resolve, 10));
    return "async result";
  };

  registerDep("asyncFunc" as never, async (ctx) => asyncFunc);

  const wrapper = Page(async (ctx: PageContext, deps: AppDeps) => {
    // Async dependencies need await
    const asyncFuncPromise = deps.asyncFunc as Promise<() => Promise<string>>;
    const fn = await asyncFuncPromise;
    return await fn();
  });

  const context = createMockContext();
  const result = await wrapper(context);

  assertEquals(result, "async result");

  // Cleanup
  unregisterDep("asyncFunc" as never);
});

Deno.test("injection - Page: dependency can access context", async () => {
  registerDep("contextReader" as never, (ctx) => {
    return () => ctx.url.pathname;
  });

  const wrapper = Page((ctx: PageContext, deps: AppDeps) => {
    const reader = deps.contextReader as () => string;
    return reader();
  });

  const context = createMockContext("/test/path");
  const result = await wrapper(context);

  assertEquals(result, "/test/path");

  // Cleanup
  unregisterDep("contextReader" as never);
});

Deno.test("injection - Page: page function returns JSX", async () => {
  registerDep("logger" as never, () => ({
    debug: () => {},
    info: () => {},
    warn: () => {},
    error: () => {},
  }));

  const wrapper = Page((ctx: PageContext, deps: AppDeps) => {
    const log = deps.logger;
    log?.info("Page function executing");

    return {
      type: "div",
      props: { children: "Hello World" },
    };
  });

  const context = createMockContext();
  const result = await wrapper(context);

  assertExists(result);
  assertEquals((result as Record<string, unknown>).type, "div");

  // Cleanup
  unregisterDep("logger" as never);
});

Deno.test("injection - Page: global Page function available", async () => {
  // Verify global Page function exists
  assertExists((globalThis as any).Page);

  // Use global Page (when no dependencies needed, function parameter is empty)
  const wrapper = (globalThis as any).Page((ctx: PageContext) => {
    return "test";
  });

  const context = createMockContext();
  const result = await wrapper(context);

  assertEquals(result, "test");
});

console.log("\n✓ Injection module tests completed");
