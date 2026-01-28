/**
 * 函数注入模块
 * 支持在页面函数中注入辅助函数
 * 使用方式:
 *   1. 注册依赖: registerDepBuilder('session', (ctx) => getSession(ctx));
 *   2. 使用依赖: export default withDeps(async function(ctx, {session, db}) {})
 */

import type { PageContext } from "./context.ts";

/**
 * 依赖对象类型
 * 可以是任意类型，通过 registerDepBuilder 注册
 */
export type Deps = Record<string, unknown>;

/**
 * 依赖构建器类型
 */
export type DepBuilder<T> = (ctx: PageContext) => Promise<T> | T;

/**
 * 依赖构建器注册表
 */
const depBuilders = new Map<string, DepBuilder<unknown>>();

/**
 * 注册依赖构建器
 * @param name 依赖名称
 * @param builder 依赖构建函数
 * @example
 * registerDepBuilder('db', (ctx) => new Database());
 * registerDepBuilder('session', async (ctx) => await getSession(ctx));
 */
export function registerDepBuilder<T>(
  name: string,
  builder: DepBuilder<T>
): void {
  depBuilders.set(name, builder);
}

/**
 * 取消注册依赖构建器
 * @param name 依赖名称
 */
export function unregisterDepBuilder(name: string): void {
  depBuilders.delete(name);
}

/**
 * 获取已注册的依赖列表
 * @returns 依赖名称数组
 */
export function getRegisteredDeps(): string[] {
  return Array.from(depBuilders.keys());
}

/**
 * 依赖注入包装器
 * 自动构建所有已注册的依赖并注入到页面函数中
 *
 * @example
 * ```tsx
 * // 在应用启动时注册依赖
 * registerDepBuilder('db', (ctx) => new Database());
 * registerDepBuilder('session', async (ctx) => await getSession(ctx));
 *
 * // 在页面中使用
 * export default withDeps(async function(ctx, {session, db}) {
 *   const user = await session.getUser();
 *   const data = await db.query('SELECT * FROM users');
 *   return <div>Hello {user.name}</div>;
 * });
 * ```
 */
export function withDeps<T>(
  fn: (ctx: PageContext, deps: Deps) => Promise<T> | T
): (ctx: PageContext) => Promise<T> {
  return async (ctx: PageContext) => {
    // 构建所有已注册的依赖
    const deps: Deps = {};

    for (const [name, builder] of depBuilders) {
      deps[name] = await builder(ctx);
    }

    // 调用原始函数，注入依赖
    return fn(ctx, deps);
  };
}
