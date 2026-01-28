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
 * // 使用 Page 包装页面函数
 * import { Page } from "../src/injection-typed.ts";
 *
 * export default Page(async function(ctx, { testFunc, db }) {
 *   const result = testFunc();  // ✅ 有完整类型提示
 *   return <div>{result}</div>;
 * });
 *
 * // Page 已在全局作用域，无需 import
 * export default Page(async function(ctx, { testFunc, db }) {
 *   const result = testFunc();
 *   return <div>{result}</div>;
 * });
 * ```
 */
export function createPage() {
  /**
   * 包装页面函数，自动注入依赖
   */
  function Page<T>(
    fn: (ctx: PageContext, deps: AppDeps) => Promise<T> | T
  ): (ctx: PageContext) => Promise<T> {
    return async (ctx: PageContext) => {
      // 获取函数签名中期望的依赖名称
      const fnStr = fn.toString();
      const depsMatch = fnStr.match(/\([^)]*,\s*{([^}]+)\}\s*\)/);
      const expectedDeps = depsMatch ? depsMatch[1].split(',').map(d => d.trim()) : [];

      // 构建所有已注册的依赖
      const deps: Record<string, unknown> = {};

      for (const [name, builder] of depBuilders) {
        deps[name] = await builder(ctx);
      }

      // 检查所有期望的依赖是否都已注册
      for (const depName of expectedDeps) {
        if (!(depName in deps)) {
          throw new Error(
            `Dependency "${depName}" is used but not registered. ` +
            `Please register it using registerDep('${depName}', builder) in main.ts.`
          );
        }
      }

      // 调用原始函数，注入依赖
      return fn(ctx, deps as AppDeps);
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
