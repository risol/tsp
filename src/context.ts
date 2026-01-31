/**
 * 上下文处理模块
 * 负责构建页面上下文对象
 */

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
