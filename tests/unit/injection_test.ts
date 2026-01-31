/**
 * Injection 模块单元测试
 * 测试 src/injection-typed.ts 中的依赖注入功能
 */

import {
  assertEquals,
  assertExists,
} from "https://deno.land/std@0.210.0/testing/asserts.ts";
import { type PageContext } from "../../src/context.ts";
import {
  getRegisteredDeps,
  Page,
  registerDep,
  unregisterDep,
} from "../../src/injection-typed.ts";

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

Deno.test("injection - registerDep: 注册单个依赖", () => {
  const testFn = () => "test";
  registerDep("testFunc" as never, testFn as never);

  const deps = getRegisteredDeps();
  assertEquals(deps.includes("testFunc"), true);

  // 清理
  unregisterDep("testFunc" as never);
});

Deno.test("injection - registerDep: 注册多个依赖", () => {
  registerDep("func1" as never, () => "func1");
  registerDep("func2" as never, () => "func2");
  registerDep("func3" as never, () => "func3");

  const deps = getRegisteredDeps();
  assertEquals(deps.length >= 3, true);
  assertEquals(deps.includes("func1"), true);
  assertEquals(deps.includes("func2"), true);
  assertEquals(deps.includes("func3"), true);

  // 清理
  unregisterDep("func1" as never);
  unregisterDep("func2" as never);
  unregisterDep("func3" as never);
});

Deno.test("injection - unregisterDep: 取消注册依赖", () => {
  registerDep("tempFunc" as never, () => "temp");

  let deps = getRegisteredDeps();
  assertEquals(deps.includes("tempFunc"), true);

  unregisterDep("tempFunc" as never);

  deps = getRegisteredDeps();
  assertEquals(deps.includes("tempFunc"), false);
});

Deno.test("injection - getRegisteredDeps: 获取已注册的依赖列表", () => {
  // 获取当前的依赖数量
  const beforeCount = getRegisteredDeps().length;

  registerDep("dep1" as never, () => "1");
  registerDep("dep2" as never, () => "2");

  const deps = getRegisteredDeps();
  assertEquals(deps.length, beforeCount + 2);
  assertEquals(deps.includes("dep1"), true);
  assertEquals(deps.includes("dep2"), true);

  // 清理
  unregisterDep("dep1" as never);
  unregisterDep("dep2" as never);
});

Deno.test("injection - Page: 单个依赖注入", async () => {
  const testFunc = () => "testFunc called";
  registerDep("testFunc" as never, (ctx) => testFunc);

  const wrapper = Page((ctx: PageContext, deps: AppDeps) => {
    const fn = deps.testFunc as () => string;
    return fn();
  });

  const context = createMockContext();
  const result = await wrapper(context);

  assertEquals(result, "testFunc called");

  // 清理
  unregisterDep("testFunc" as never);
});

Deno.test("injection - Page: 多个依赖注入", async () => {
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

  // 清理
  unregisterDep("func1" as never);
  unregisterDep("func2" as never);
  unregisterDep("func3" as never);
});

Deno.test("injection - Page: 异步依赖构建", async () => {
  const asyncFunc = async () => {
    await new Promise((resolve) => setTimeout(resolve, 10));
    return "async result";
  };

  registerDep("asyncFunc" as never, async (ctx) => asyncFunc);

  const wrapper = Page(async (ctx: PageContext, deps: AppDeps) => {
    // 异步依赖需要 await
    const asyncFuncPromise = deps.asyncFunc as Promise<() => Promise<string>>;
    const fn = await asyncFuncPromise;
    return await fn();
  });

  const context = createMockContext();
  const result = await wrapper(context);

  assertEquals(result, "async result");

  // 清理
  unregisterDep("asyncFunc" as never);
});

Deno.test("injection - Page: 依赖可以访问 context", async () => {
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

  // 清理
  unregisterDep("contextReader" as never);
});

Deno.test("injection - Page: 页面函数返回 JSX", async () => {
  registerDep("logger" as never, (ctx) => console.log);

  const wrapper = Page((ctx: PageContext, deps: AppDeps) => {
    const log = deps.logger as typeof console.log;
    log("页面函数执行中");

    return {
      type: "div",
      props: { children: "Hello World" },
    };
  });

  const context = createMockContext();
  const result = await wrapper(context);

  assertExists(result);
  assertEquals((result as Record<string, unknown>).type, "div");

  // 清理
  unregisterDep("logger" as never);
});

Deno.test("injection - Page: 全局 Page 函数可用", async () => {
  // 验证全局 Page 函数存在
  assertExists((globalThis as any).Page);

  // 使用全局 Page（不需要依赖时，函数参数为空）
  const wrapper = (globalThis as any).Page((ctx: PageContext) => {
    return "test";
  });

  const context = createMockContext();
  const result = await wrapper(context);

  assertEquals(result, "test");
});

console.log("\n✓ Injection 模块测试完成");
