/**
 * 测试Page函数是否需要async
 */

// 测试1: 不使用Page包装器，直接async（不推荐，但应该工作）
export async function test1_direct_async(context: PageContext) {
  return <div>Direct async - no dependencies</div>;
}

// 测试2: 不使用Page包装器，非async（不推荐）
export function test2_direct_sync(context: PageContext) {
  return <div>Direct sync - no dependencies</div>;
}

// 测试3: 使用全局函数，async，使用异步依赖（推荐）
export default async function (ctx) {
  // 调用异步方法，所以需要async
  const user = await session.getUser();
  return <div>Global function async with async dependency: {user?.name || "guest"}</div>;
}

// 测试4: 使用Page包装器，async，但不使用await（可以工作但不推荐）
export const test4 = Page(async function (ctx, { session }) {
  // 虽然有async但没有await
  return <div>Page async without await</div>;
});

// 测试5: 使用Page包装器，非async，不使用依赖（最简单）
export const test5 = Page(function (ctx) {
  return <div>Page sync no dependencies</div>;
});

// 测试6: 使用Page包装器，非async，但返回JSX（正确）
export const test6 = Page(function (ctx) {
  return <div>Page sync returning JSX</div>;
});
