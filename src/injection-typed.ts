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
const depBuilders = new Map<
  keyof AppDeps,
  (ctx: PageContext) => Promise<unknown> | unknown
>();

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
  builder: (ctx: PageContext) => Promise<AppDeps[K]> | AppDeps[K],
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
export function registerDeps<
  T extends Record<string, (ctx: PageContext) => unknown>,
>(
  deps: T,
): void {
  for (const [name, builder] of Object.entries(deps)) {
    depBuilders.set(
      name as keyof AppDeps,
      builder as (ctx: PageContext) => unknown,
    );
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
 * // 简化用法（推荐）- 自动懒加载依赖
 * export default Page(async function(ctx, { session, db }) {
 *   const user = await session.getUser();
 *   const data = await db.query('SELECT * FROM users');
 *   return <div>{data}</div>;
 * });
 *
 * // 只有在访问依赖时才构建，未访问的依赖不会构建
 * export default Page(async function(ctx, { logger }) {
 *   // 只有 logger 被构建，其他依赖（如 db）不会被构建
 *   logger('Page loaded');
 *   return <div>Hello</div>;
 * });
 * ```
 */
export function createPage() {
  /**
   * 创建懒加载依赖的 Proxy
   *
   * 只有在访问依赖属性时才构建依赖
   * 已构建的依赖会被缓存
   *
   * @param ctx - 页面上下文
   * @returns Proxy 对象
   */
  function createLazyDeps(ctx: PageContext): AppDeps {
    const cache = new Map<string, unknown>();
    const resolvedDeps: Partial<AppDeps> = {};

    return new Proxy(resolvedDeps as AppDeps, {
      get(target, prop: string) {
        // 如果是 Symbol 或特殊属性，直接返回
        if (typeof prop === "symbol") {
          return (target as Record<string, unknown>)[prop];
        }

        // 返回已缓存的依赖
        if (cache.has(prop)) {
          return cache.get(prop);
        }

        // 检查依赖是否已注册
        const builder = depBuilders.get(prop as keyof AppDeps);
        if (!builder) {
          throw new Error(
            `Dependency "${prop}" is requested but not registered. ` +
              `Please register it using registerDep('${prop}', builder) in main.ts. ` +
              `Available deps: ${Array.from(depBuilders.keys()).join(", ")}`,
          );
        }

        // 构建依赖
        const dep = builder(ctx);

        // 🔧 修复：如果是 Promise，直接返回 Promise
        // 这样 user code 中 await session.getUser() 仍然有效
        if (dep instanceof Promise) {
          // 先缓存 Promise，避免重复构建
          cache.set(prop, dep);

          // 直接返回 Promise
          // user code 可以这样使用：const session = await deps.session
          return dep as unknown;
        }

        // 同步依赖直接缓存
        cache.set(prop, dep);
        (target as Record<string, unknown>)[prop] = dep;

        return dep;
      },

      // 支持 'in' 操作符
      has(target, prop: string) {
        if (typeof prop === "symbol") {
          return prop in target;
        }
        return depBuilders.has(prop as keyof AppDeps);
      },

      // 支持 Object.keys() 等操作
      ownKeys() {
        return Array.from(depBuilders.keys()).map(String);
      },

      // 支持 getOwnPropertyDescriptor
      getOwnPropertyDescriptor(target, prop: string) {
        if (typeof prop === "symbol") {
          return Object.getOwnPropertyDescriptor(target, prop);
        }
        if (depBuilders.has(prop as keyof AppDeps)) {
          return {
            enumerable: true,
            configurable: true,
            value: (target as Record<string, unknown>)[prop],
          };
        }
        return undefined;
      },
    });
  }

  /**
   * 包装页面函数，自动注入依赖（懒加载版本）
   *
   * @param fn - 页面函数
   * @returns 包装后的页面函数
   */
  function Page<T>(
    fn: (ctx: PageContext, deps: AppDeps) => Promise<T> | T,
  ): (ctx: PageContext) => Promise<T> {
    return async (ctx: PageContext) => {
      // 创建懒加载依赖 Proxy
      const deps = createLazyDeps(ctx);

      // 调用原始函数，注入懒加载依赖
      return fn(ctx, deps);
    };
  }

  return Page;
}

/**
 * 初始化全局 Page
 * 在模块加载时就自动执行，无需手动调用
 */
export function initGlobalPage(): void {
  if (typeof (globalThis as any).Page === "undefined") {
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
