/**
 * Cookies 依赖注入测试
 * 验证 cookies 依赖能正确注入并使用
 */

import {
  assertEquals,
  assertExists,
} from "https://deno.land/std@0.210.0/testing/asserts.ts";
import { type PageContext } from "../../src/context.ts";
import { createCookieManager } from "../../src/cookies.ts";
import { Page, registerDep, unregisterDep } from "../../src/injection-typed.ts";

// 创建一个简单的 PageContext 用于测试
function createMockContext(pathname: string = "/"): PageContext {
  return {
    method: "GET",
    url: new URL(`http://localhost:9000${pathname}`),
    headers: new Headers(),
    query: {},
    body: null,
    cookies: {},
    files: {},
    file: "/test.tsx",
    root: "./www",
  };
}

Deno.test("Cookies 依赖注入 - set 方法可用", async () => {
  // 注册 cookies 依赖
  registerDep("cookies", (ctx: PageContext) => {
    return createCookieManager(ctx);
  });

  // 使用 Page wrapper
  const pageFn = Page(async function (ctx: PageContext, { cookies }) {
    // 调用 set 方法
    cookies.set("testKey", "testValue", { maxAge: 3600 });

    return {
      success: true,
    };
  });

  // 创建 mock context
  const ctx = createMockContext("/test");

  // 执行页面函数
  const result = await pageFn(ctx);

  assertExists(result);
  assertEquals(result.success, true);

  // 清理
  unregisterDep("cookies" as never);
});

Deno.test("Cookies 依赖注入 - setMultiple 方法可用", async () => {
  // 注册 cookies 依赖
  registerDep("cookies", (ctx: PageContext) => {
    return createCookieManager(ctx);
  });

  const pageFn = Page(async function (ctx: PageContext, { cookies }) {
    // 调用 setMultiple 方法
    cookies.setMultiple({
      "theme": { value: "dark", options: { maxAge: 3600 } },
      "lang": { value: "zh", options: { maxAge: 3600 } },
    });

    return {
      success: true,
      cookiesSet: 2,
    };
  });

  const ctx = createMockContext("/test");
  const result = await pageFn(ctx);

  assertExists(result);
  assertEquals(result.cookiesSet, 2);

  // 清理
  unregisterDep("cookies" as never);
});

Deno.test("Cookies 依赖注入 - delete 方法可用", async () => {
  // 注册 cookies 依赖
  registerDep("cookies", (ctx: PageContext) => {
    return createCookieManager(ctx);
  });

  const pageFn = Page(async function (ctx: PageContext, { cookies }) {
    // 调用 delete 方法
    cookies.delete("testKey");

    return {
      success: true,
      deleted: "testKey",
    };
  });

  const ctx = createMockContext("/test");
  const result = await pageFn(ctx);

  assertExists(result);
  assertEquals(result.deleted, "testKey");

  // 清理
  unregisterDep("cookies" as never);
});
