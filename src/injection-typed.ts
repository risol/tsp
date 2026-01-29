/**
 * 类型安全的依赖注入模块
 * 通过函数签名自动推断类型，提供完整类型提示
 *
 * 使用步骤：
 * 1. 在 types.d.ts 的 AppDeps 接口中声明依赖类型
 * 2. 在 main.ts 中使用 registerDep() 注册依赖实现
 * 3. 在 TSX 中直接使用全局 Page()（无需 import）
 */

import type { PageContext } from "./context.ts";

// ============================================
// 类型声明已在 types.d.ts 中
// ============================================

/**
 * 依赖构建器注册表（内部使用）
 */
const depBuilders = new Map<keyof AppDeps, (ctx: PageContext) => Promise<unknown> | unknown>();

/**
 * 注册单个依赖（带类型推断）
 *
 * @example
 * ```typescript
 * // main.ts
 * import { registerDep } from "./src/injection-typed.ts";
 *
 * registerDep('testFunc', () => {
 *   console.log('testFunc called');
 *   return 'testFunc called';
 * });
 * ```
 */
export function registerDep<K extends keyof AppDeps>(
  name: K,
  builder: (ctx: PageContext) => Promise<AppDeps[K]> | AppDeps[K]
): void {
  depBuilders.set(name, builder);
}

/**
 * 批量注册依赖
 *
 * @example
 * ```typescript
 * // main.ts
 * import { registerDeps } from "./src/injection-typed.ts";
 *
 * registerDeps({
 *   testFunc: () => {
 *     console.log('testFunc called');
 *     return 'testFunc called';
 *   },
 *   db: (ctx) => ({
 *     query: async (sql: string) => database.execute(sql),
 *   }),
 * });
 * ```
 */
export function registerDeps<T extends Record<string, (ctx: PageContext) => unknown>>(
  deps: T
): void {
  for (const [name, builder] of Object.entries(deps)) {
    depBuilders.set(name as keyof AppDeps, builder as (ctx: PageContext) => unknown);
  }
}

/**
 * 取消注册依赖
 */
export function unregisterDep(name: keyof AppDeps): void {
  depBuilders.delete(name);
}

/**
 * 获取已注册的依赖列表
 */
export function getRegisteredDeps(): string[] {
  return Array.from(depBuilders.keys()) as string[];
}

/**
 * Page 包装器
 * 这是一个高阶函数，返回一个包装函数，用于自动注入依赖
 *
 * @example
 * ```tsx
 * // 使用 Page 包装页面函数（显式声明依赖）
 * import { Page } from "../src/injection-typed.ts";
 *
 * // 方式1: 使用依赖键数组（推荐）
 * export default Page(['testFunc', 'db'], async (ctx, { testFunc, db }) => {
 *   const result = await testFunc();  // ✅ 有完整类型提示
 *   const data = await db.query('SELECT * FROM users');
 *   return <div>{result}</div>;
 * });
 *
 * // Page 已在全局作用域，无需 import
 * export default Page(['logger'], async function(ctx, { logger }) {
 *   await logger('Page loaded');
 *   return <div>Hello</div>;
 * });
 * ```
 */
export function createPage() {
  /**
   * 包装页面函数，自动注入依赖（显式声明版本）
   *
   * @param deps - 依赖名称数组
   * @param fn - 页面函数
   * @returns 包装后的页面函数
   */
  function Page<T>(
    deps: (keyof AppDeps)[],
    fn: (ctx: PageContext, deps: AppDeps) => Promise<T> | T
  ): (ctx: PageContext) => Promise<T> {
    return async (ctx: PageContext) => {
      // 构建请求的依赖
      const resolvedDeps: Record<string, unknown> = {};

      for (const depName of deps) {
        const builder = depBuilders.get(depName);
        if (builder) {
          resolvedDeps[depName] = await builder(ctx);
        } else {
          throw new Error(
            `Dependency "${String(depName)}" is requested but not registered. ` +
            `Please register it using registerDep('${String(depName)}', builder) in main.ts. ` +
            `Available deps: ${Array.from(depBuilders.keys()).join(', ')}`
          );
        }
      }

      // 调用原始函数，注入依赖
      return fn(ctx, resolvedDeps as AppDeps);
    };
  }

  return Page;
}

/**
 * 初始化全局 Page
 * 在模块加载时就自动执行，无需手动调用
 */
export function initGlobalPage(): void {
  if (typeof (globalThis as any).Page === 'undefined') {
    (globalThis as any).Page = createPage();
  }
}

// 自动初始化（在模块加载时执行）
initGlobalPage();

// 导出 Page
const _pageFn = createPage();
export const Page = _pageFn;

// 默认导出
export default {
  registerDep,
  registerDeps,
  unregisterDep,
  getRegisteredDeps,
  createPage,
  initGlobalPage,
  Page,
};
