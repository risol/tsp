/**
 * 上下文处理模块
 * 负责构建模板上下文对象
 */

// 上下文参数
interface ContextParams {
  method: string;
  url: URL;
  headers: Headers;
  query: Record<string, string>;
  body: unknown;
  cookies: Record<string, string>;
  file: string;
  root: string;
}

// 模板上下文类型
export type TemplateContext = {
  method: string;
  url: URL;
  headers: Headers;
  query: Record<string, string>;
  body: unknown;
  cookies: Record<string, string>;
  file: string;
  root: string;
};

/**
 * 构建模板上下文对象
 * @param params 上下文参数
 * @returns 模板上下文
 */
export function buildContext(params: ContextParams): TemplateContext {
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
