/**
 * Injection 模块单元测试
 * 测试 src/injection.ts 中的依赖注入功能
 */

import { assertEquals, assertExists } from "https://deno.land/std@0.210.0/testing/asserts.ts";
import { type PageContext } from "../../src/context.ts";
import {
  registerDepBuilder,
  withDeps,
  unregisterDepBuilder,
  getRegisteredDeps,
  type Deps,
} from "../../src/injection.ts";

// 创建一个简单的 PageContext 用于测试
function createMockContext(pathname: string = "/"): PageContext {
  return {
    method: "GET",
    url: new URL(`http://localhost:9000${pathname}`),
    headers: new Headers(),
    query: {},
    body: null,
    cookies: {},
    file: "/test.tsx",
    root: "./www",
  };
}

Deno.test("injection - registerDepBuilder: 注册单个依赖", () => {
  const testFn = () => "test";
  registerDepBuilder("testFunc", testFn);

  const deps = getRegisteredDeps();
  assertEquals(deps.includes("testFunc"), true);

  // 清理
  unregisterDepBuilder("testFunc");
});

Deno.test("injection - registerDepBuilder: 注册多个依赖", () => {
  registerDepBuilder("func1", () => "func1");
  registerDepBuilder("func2", () => "func2");
  registerDepBuilder("func3", () => "func3");

  const deps = getRegisteredDeps();
  assertEquals(deps.length, 3);
  assertEquals(deps.includes("func1"), true);
  assertEquals(deps.includes("func2"), true);
  assertEquals(deps.includes("func3"), true);

  // 清理
  unregisterDepBuilder("func1");
  unregisterDepBuilder("func2");
  unregisterDepBuilder("func3");
});

Deno.test("injection - unregisterDepBuilder: 取消注册依赖", () => {
  registerDepBuilder("tempFunc", () => "temp");

  let deps = getRegisteredDeps();
  assertEquals(deps.includes("tempFunc"), true);

  unregisterDepBuilder("tempFunc");

  deps = getRegisteredDeps();
  assertEquals(deps.includes("tempFunc"), false);
});

Deno.test("injection - getRegisteredDeps: 获取已注册的依赖列表", () => {
  // 清空所有依赖（确保测试隔离）
  const existingDeps = getRegisteredDeps();
  for (const dep of existingDeps) {
    unregisterDepBuilder(dep);
  }

  registerDepBuilder("dep1", () => "1");
  registerDepBuilder("dep2", () => "2");

  const deps = getRegisteredDeps();
  assertEquals(deps.length, 2);
  assertEquals(deps.sort(), ["dep1", "dep2"]);

  // 清理
  unregisterDepBuilder("dep1");
  unregisterDepBuilder("dep2");
});

Deno.test("injection - withDeps: 单个依赖注入", async () => {
  const testFunc = () => "testFunc called";
  registerDepBuilder("testFunc", (ctx) => testFunc);

  const wrapper = withDeps((ctx: PageContext, deps: Deps) => {
    const fn = deps.testFunc as () => string;
    return fn();
  });

  const context = createMockContext();
  const result = await wrapper(context);

  assertEquals(result, "testFunc called");

  // 清理
  unregisterDepBuilder("testFunc");
});

Deno.test("injection - withDeps: 多个依赖注入", async () => {
  const func1 = () => "func1";
  const func2 = () => "func2";
  const func3 = () => "func3";

  registerDepBuilder("func1", () => func1);
  registerDepBuilder("func2", () => func2);
  registerDepBuilder("func3", () => func3);

  const wrapper = withDeps((ctx: PageContext, deps: Deps) => {
    const f1 = deps.func1 as () => string;
    const f2 = deps.func2 as () => string;
    const f3 = deps.func3 as () => string;
    return [f1(), f2(), f3()];
  });

  const context = createMockContext();
  const result = await wrapper(context);

  assertEquals(result, ["func1", "func2", "func3"]);

  // 清理
  unregisterDepBuilder("func1");
  unregisterDepBuilder("func2");
  unregisterDepBuilder("func3");
});

Deno.test("injection - withDeps: 异步依赖构建", async () => {
  const asyncFunc = async () => {
    await new Promise((resolve) => setTimeout(resolve, 10));
    return "async result";
  };

  registerDepBuilder("asyncFunc", async (ctx) => asyncFunc);

  const wrapper = withDeps(async (ctx: PageContext, deps: Deps) => {
    const fn = deps.asyncFunc as () => Promise<string>;
    return await fn();
  });

  const context = createMockContext();
  const result = await wrapper(context);

  assertEquals(result, "async result");

  // 清理
  unregisterDepBuilder("asyncFunc");
});

Deno.test("injection - withDeps: 依赖可以访问 context", async () => {
  registerDepBuilder("contextReader", (ctx) => {
    return () => ctx.url.pathname;
  });

  const wrapper = withDeps((ctx: PageContext, deps: Deps) => {
    const reader = deps.contextReader as () => string;
    return reader();
  });

  const context = createMockContext("/test/path");
  const result = await wrapper(context);

  assertEquals(result, "/test/path");

  // 清理
  unregisterDepBuilder("contextReader");
});

Deno.test("injection - withDeps: 未注册的依赖返回 undefined", async () => {
  // 不注册任何依赖
  const wrapper = withDeps((ctx: PageContext, deps: Deps) => {
    return deps.nonExistent;
  });

  const context = createMockContext();
  const result = await wrapper(context);

  assertEquals(result, undefined);
});

Deno.test("injection - withDeps: 页面函数返回 JSX", async () => {
  registerDepBuilder("logger", (ctx) => console.log);

  const wrapper = withDeps((ctx: PageContext, deps: Deps) => {
    const log = deps.logger as typeof console.log;
    log("页面函数执行中");

    return {
      type: "div",
      props: { children: "Hello World" }
    };
  });

  const context = createMockContext();
  const result = await wrapper(context);

  assertExists(result);
  assertEquals((result as Record<string, unknown>).type, "div");

  // 清理
  unregisterDepBuilder("logger");
});

console.log("\n✓ Injection 模块测试完成");
