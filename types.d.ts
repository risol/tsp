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
    /** 上传的文件（multipart/form-data）
     * - 单个文件：UploadedFile
     * - 多个同名文件：UploadedFile[]
     */
    readonly files: Record<string, UploadedFile | UploadedFile[]>;
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
   * 上传的文件接口
   * 表示通过 multipart/form-data 上传的文件
   */
  interface UploadedFile {
    /** 原始文件名 */
    readonly name: string;
    /** MIME 类型 */
    readonly type: string;
    /** 文件大小（字节） */
    readonly size: number;
    /** 文件内容（Uint8Array） */
    readonly data: Uint8Array;

    /**
     * 保存文件到指定路径
     * @param path - 目标路径（可以是相对路径或绝对路径）
     */
    save(path: string): Promise<void>;

    /**
     * 将文件内容转换为文本（适用于文本文件）
     */
    text(): Promise<string>;
  }

  /**
   * Response 辅助器接口
   * 提供便捷的 HTTP 响应创建方法
   */
  interface ResponseHelper {
    /**
     * 返回 JSON 响应
     * @param data - 要序列化的数据
     * @param status - HTTP 状态码，默认 200
     * @param headers - 额外的响应头
     */
    json<T = unknown>(data: T, status?: number, headers?: HeadersInit): Response;

    /**
     * 返回纯文本响应
     * @param content - 文本内容
     * @param status - HTTP 状态码，默认 200
     * @param headers - 额外的响应头
     */
    text(content: string, status?: number, headers?: HeadersInit): Response;

    /**
     * 返回 HTML 响应
     * @param content - HTML 内容
     * @param status - HTTP 状态码，默认 200
     * @param headers - 额外的响应头
     */
    html(content: string, status?: number, headers?: HeadersInit): Response;

    /**
     * 返回重定向
     * @param url - 重定向目标 URL
     * @param status - 重定向状态码，默认 302
     */
    redirect(url: string, status?: 301 | 302 | 303 | 307 | 308): RedirectResult;

    /**
     * 返回错误响应
     * @param message - 错误消息
     * @param status - HTTP 状态码，默认 500
     * @param headers - 额外的响应头
     */
    error(message: string, status?: number, headers?: HeadersInit): Response;

    /**
     * 返回文件下载响应
     * @param content - 文件内容（字符串或二进制）
     * @param filename - 文件名
     * @param headers - 额外的响应头
     */
    file(content: string | Uint8Array, filename: string, headers?: HeadersInit): Response;

    /**
     * 返回 204 No Content 响应
     */
    noContent(): Response;

    /**
     * 返回自定义 Response
     * @param body - 响应体
     * @param init - Response 初始化选项
     */
    custom(body?: BodyInit | null, init?: ResponseInit): Response;
  }

  /**
   * 日志记录器接口
   * 提供结构化的日志记录功能
   */
  interface Logger {
    /**
     * 调试日志
     * @param args - 要记录的消息内容
     */
    debug(...args: unknown[]): void;

    /**
     * 信息日志
     * @param args - 要记录的消息内容
     */
    info(...args: unknown[]): void;

    /**
     * 警告日志
     * @param args - 要记录的消息内容
     */
    warn(...args: unknown[]): void;

    /**
     * 错误日志
     * @param args - 要记录的消息内容
     */
    error(...args: unknown[]): void;
  }

  /**
   * 应用依赖类型
   * 在此声明所有可注入的依赖及其类型
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
     * Session 管理
     */
    session: import("./src/session.ts").SessionManager;

    /**
     * Cookie管理器
     */
    cookies: import("./src/cookies.ts").CookieManager;

    /**
     * Response 辅助器
     * 提供便捷的 HTTP 响应创建方法
     */
    response: ResponseHelper;

    /**
     * 日志记录器
     * 提供结构化的日志记录功能
     */
    logger: import("./src/logger.ts").Logger;
  }

  /**
   * 全局 Page 函数
   * TSX 文件中可以直接使用，无需 import
   *
   * @example
   * ```tsx
   * export default Page(async function(ctx, { testFunc, db }) {
   *   const result = testFunc();  // ✅ 有完整类型提示
   *   return <div>{result}</div>;
   * });
   * ```
   */
  function Page<T>(
    fn: (ctx: PageContext, deps: AppDeps) => Promise<T> | T
  ): (ctx: PageContext) => Promise<T>;
}

// 确保类型被视为全局的
export {};
