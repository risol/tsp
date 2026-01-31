/**
 * 全局类型声明
 * TSX 文件可以直接使用这些类型，无需 import
 */

declare global {
  /**
   * HTTP 方法类型
   */
  type HttpMethod =
    | "GET"
    | "POST"
    | "PUT"
    | "PATCH"
    | "DELETE"
    | "HEAD"
    | "OPTIONS";

  /**
   * 页面上下文类型
   * 每个 TSX 页面函数接收此上下文作为参数
   */
  interface PageContext {
    /** HTTP 请求方法 */
    readonly method: HttpMethod;
    /** 完整的请求 URL 对象 */
    readonly url: URL;
    /** 请求头对象 */
    readonly headers: Headers;
    /** URL 查询参数 */
    readonly query: Record<string, string>;
    /** 请求体数据 */
    readonly body: unknown;
    /** Cookie 数据 */
    readonly cookies: Record<string, string>;
    /** 当前 TSX 页面文件的路径 */
    readonly file: string;
    /** 文档根目录路径 */
    readonly root: string;
  }

  /**
   * 重定向结果
   * 页面返回此对象将触发 HTTP 重定向
   */
  interface RedirectResult {
    /** 重定向的目标 URL */
    redirect: string;
    /** 重定向状态码，默认 302 */
    status?: 301 | 302 | 303 | 307 | 308;
  }

  /**
   * 全局变量 - 获取当前请求的 Session 管理器
   * 可以直接在任何 TSX 页面中使用，无需 Page 包装器
   *
   * @example
   * ```tsx
   * export default async function(ctx) {
   *   const user = await session.getUser();
   *   return <div>欢迎, {user?.name}</div>;
   * }
   * ```
   */
  const session: import("./src/session.ts").SessionManager;

  /**
   * 全局变量 - 获取当前请求的 Cookie 管理器
   * 可以直接在任何 TSX 页面中使用，无需 Page 包装器
   *
   * @example
   * ```tsx
   * export default async function(ctx) {
   *   cookies.set('theme', 'dark');
   *   return <div>主题已设置</div>;
   * }
   * ```
   */
  const cookies: import("./src/cookies.ts").CookieManager;

  /**
   * 应用依赖类型（用于 Page 包装器）
   * Page 包装器提供可选的依赖注入方式
   *
   * @example
   * ```typescript
   * // 添加新的依赖类型
   * interface AppDeps {
   *   testFunc: () => string;
   *   db: {
   *     query: (sql: string) => Promise<unknown[]>;
   *   };
   * }
   * ```
   */
  interface AppDeps extends Record<string, unknown> {
    /**
     * 测试函数
     */
    testFunc: () => string;

    /**
     * 数据库接口（示例）
     */
    db: {
      query: (sql: string) => Promise<unknown[]>;
      insert: (table: string, data: Record<string, unknown>) => Promise<void>;
    };

    /**
     * Session 管理（通过 Page 包装器注入）
     * @deprecated 推荐使用全局函数 session()
     */
    session: import("./src/session.ts").SessionManager;

    /**
     * 日志函数（示例）
     */
    logger: typeof console.log;

    /**
     * Cookie管理器（通过 Page 包装器注入）
     * @deprecated 推荐使用全局函数 cookies()
     */
    cookies: import("./src/cookies.ts").CookieManager;
  }

  /**
   * 全局 Page 函数（可选的依赖注入包装器）
   * 推荐使用全局函数 session() 和 cookies()，Page 包装器作为备选方案
   *
   * @example
   * ```tsx
   * // 推荐方式：使用全局函数
   * export default async function(ctx) {
   *   const user = await session().getUser();
   *   return <div>{user?.name}</div>;
   * }
   *
   * // 可选方式：使用 Page 包装器
   * export default Page(async function(ctx, { session, logger }) {
   *   const user = await session.getUser();
   *   logger('用户访问');
   *   return <div>{user?.name}</div>;
   * });
   * ```
   */
  function Page<T>(
    fn: (ctx: PageContext, deps: AppDeps) => Promise<T> | T
  ): (ctx: PageContext) => Promise<T>;
}

// 确保类型被视为全局的
export {};
