/**
 * 上下文处理模块
 * 负责构建页面上下文对象和管理请求上下文
 */

import { createSessionManager, SessionStore } from "./session.ts";
import { createCookieManager } from "./cookies.ts";

/**
 * HTTP 请求方法类型
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
 * 页面上下文参数（用于构建上下文的输入参数）
 */
interface ContextParams {
  method: HttpMethod;
  url: URL;
  headers: Headers;
  query: Record<string, string>;
  body: unknown;
  cookies: Record<string, string>;
  file: string;
  root: string;
}

/**
 * 页面上下文类型
 * 传递给每个 TSX 页面函数的上下文对象
 */
export type PageContext_ = Readonly<{
  /** HTTP 请求方法 (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS) */
  method: HttpMethod;

  /** 完整的请求 URL 对象 */
  url: URL;

  /** 请求头对象 */
  headers: Headers;

  /** URL 查询参数 (已解析为键值对) */
  query: Record<string, string>;

  /** 请求体数据 (POST/PUT/PATCH)
   * - application/json → 解析为对象
   * - application/x-www-form-urlencoded → 解析为键值对
   * - 其他 → 原始文本字符串
   * - GET/HEAD/DELETE 等 → null
   */
  body: unknown;

  /** Cookie 数据 (已解析为键值对) */
  cookies: Record<string, string>;

  /** 当前 TSX 页面文件的完整路径 (相对于项目根目录) */
  file: string;

  /** 文档根目录路径 */
  root: string;
}>;

/**
 * 请求上下文存储
 * 使用 WeakMap 避免内存泄漏，请求结束后自动清理
 */
const requestContextMap = new WeakMap<
  Readonly<{
    method: HttpMethod;
    url: URL;
    headers: Headers;
    query: Record<string, string>;
    body: unknown;
    cookies: Record<string, string>;
    file: string;
    root: string;
  }>,
  {
    session: import("./session.ts").SessionManager;
    cookies: import("./cookies.ts").CookieManager;
  }
>();

/**
 * 构建页面上下文对象
 * @param params 上下文参数
 * @returns 页面上下文
 */
export function buildContext(params: ContextParams): PageContext {
  return {
    method: params.method,
    url: params.url,
    headers: params.headers,
    query: params.query,
    body: params.body,
    cookies: params.cookies,
    file: params.file,
    root: params.root,
  };
}

/**
 * 设置当前请求的上下文
 * 创建 session 和 cookies 管理器实例，并存储到请求上下文
 *
 * @param ctx 页面上下文
 * @param sessionStore Session 存储实例
 */
export function setupRequestContext(
  ctx: Readonly<{
    method: HttpMethod;
    url: URL;
    headers: Headers;
    query: Record<string, string>;
    body: unknown;
    cookies: Record<string, string>;
    file: string;
    root: string;
  }>,
  sessionStore: SessionStore,
): void {
  const cookieManager = createCookieManager(ctx);
  const sessionManager = createSessionManager(ctx, sessionStore, cookieManager);

  requestContextMap.set(ctx, {
    session: sessionManager,
    cookies: cookieManager,
  });
}

/**
 * 获取当前请求的上下文
 *
 * @param ctx 页面上下文
 * @returns 请求上下文（包含 session 和 cookies）
 */
export function getRequestContext(
  ctx: Readonly<{
    method: HttpMethod;
    url: URL;
    headers: Headers;
    query: Record<string, string>;
    body: unknown;
    cookies: Record<string, string>;
    file: string;
    root: string;
  }>,
) {
  return requestContextMap.get(ctx);
}

/**
 * 清理当前请求的上下文
 * 请求结束后调用，避免内存泄漏
 *
 * @param ctx 页面上下文
 */
export function cleanupRequestContext(
  ctx: Readonly<{
    method: HttpMethod;
    url: URL;
    headers: Headers;
    query: Record<string, string>;
    body: unknown;
    cookies: Record<string, string>;
    file: string;
    root: string;
  }>,
): void {
  requestContextMap.delete(ctx);
}

// 导出 PageContext 类型
export type PageContext = Readonly<{
  /** HTTP 请求方法 (GET, POST, PUT, PATCH, DELETE, HEAD, OPTIONS) */
  method: HttpMethod;

  /** 完整的请求 URL 对象 */
  url: URL;

  /** 请求头对象 */
  headers: Headers;

  /** URL 查询参数 (已解析为键值对) */
  query: Record<string, string>;

  /** 请求体数据 (POST/PUT/PATCH)
   * - application/json → 解析为对象
   * - application/x-www-form-urlencoded → 解析为键值对
   * - 其他 → 原始文本字符串
   * - GET/HEAD/DELETE 等 → null
   */
  body: unknown;

  /** Cookie 数据 (已解析为键值对) */
  cookies: Record<string, string>;

  /** 当前 TSX 页面文件的完整路径 (相对于项目根目录) */
  file: string;

  /** 文档根目录路径 */
  root: string;
}>;
