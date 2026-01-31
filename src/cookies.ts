/**
 * Cookie 管理模块（用于 TSP）
 *
 * 通过依赖注入提供 cookie 管理功能。
 * 使用 WeakMap 存储每个请求上下文的 cookie 操作。
 */

/**
 * 标准 cookie 选项，与浏览器 Set-Cookie 头兼容
 */
export interface CookieOptions {
  /** Expiration date (GMT string or Date object) */
  expires?: string | Date;
  /** Max age in seconds */
  maxAge?: number;
  /** Domain restriction */
  domain?: string;
  /** Path restriction */
  path?: string;
  /** HTTPS only */
  secure?: boolean;
  /** No JavaScript access */
  httpOnly?: boolean;
  /** Same-site policy */
  sameSite?: "Strict" | "Lax" | "None";
}

/**
 * Cookie 管理器接口，用于设置和删除 cookies
 */
export interface CookieManager {
  /** Set a single cookie */
  set: (name: string, value: string, options?: CookieOptions) => void;
  /** Delete a single cookie */
  delete: (
    name: string,
    options?: Pick<CookieOptions, "domain" | "path">,
  ) => void;
  /** Set multiple cookies at once */
  setMultiple: (
    cookies: Record<string, { value: string; options?: CookieOptions }>,
  ) => void;
  /** Delete multiple cookies at once */
  deleteMultiple: (
    names: string[],
    options?: Pick<CookieOptions, "domain" | "path">,
  ) => void;
}

/**
 * 内部请求上下文，用于存储 cookie 操作
 */
interface RequestContext {
  setCookieHeaders: string[];
}

/**
 * WeakMap 用于存储每个请求的 cookie 操作
 * 使用 WeakMap 是为了让当 PageContext 被垃圾回收时，
 * 关联的 cookie 上下文也会自动清理
 */
const cookieContextMap = new WeakMap<PageContext, RequestContext>();

/**
 * 将 cookie 序列化为 Set-Cookie 头格式
 * @param name Cookie 名称
 * @param value Cookie 值
 * @param options Cookie 选项
 * @returns Set-Cookie 头值
 */
export function serializeCookie(
  name: string,
  value: string,
  options: CookieOptions = {},
): string {
  const parts: string[] = [];

  // 编码名称和值（特殊字符需要）
  parts.push(`${encodeURIComponent(name)}=${encodeURIComponent(value)}`);

  // Max-Age 优先于 Expires
  if (options.maxAge !== undefined) {
    parts.push(`Max-Age=${options.maxAge}`);
  } else if (options.expires) {
    const expires = options.expires instanceof Date
      ? options.expires.toUTCString()
      : options.expires;
    parts.push(`Expires=${expires}`);
  }

  // Domain
  if (options.domain) {
    parts.push(`Domain=${options.domain}`);
  }

  // Path
  if (options.path) {
    parts.push(`Path=${options.path}`);
  }

  // Secure
  if (options.secure) {
    parts.push("Secure");
  }

  // HttpOnly
  if (options.httpOnly) {
    parts.push("HttpOnly");
  }

  // SameSite
  if (options.sameSite) {
    parts.push(`SameSite=${options.sameSite}`);
  }

  return parts.join("; ");
}

/**
 * 为给定的请求上下文创建 cookie 管理器
 * @param ctx Page 上下文
 * @returns Cookie 管理器实例
 */
export function createCookieManager(ctx: PageContext): CookieManager {
  // 获取或创建请求上下文
  let requestContext = cookieContextMap.get(ctx);
  if (!requestContext) {
    requestContext = { setCookieHeaders: [] };
    cookieContextMap.set(ctx, requestContext);
  }

  return {
    /**
     * Set a single cookie
     */
    set(name: string, value: string, options?: CookieOptions): void {
      const header = serializeCookie(name, value, options);
      requestContext!.setCookieHeaders.push(header);
    },

    /**
     * Delete a single cookie by setting maxAge=0
     */
    delete(
      name: string,
      options?: Pick<CookieOptions, "domain" | "path">,
    ): void {
      // 要删除 cookie，设置 maxAge=0 并过期时间为过去
      // 必须匹配设置时的 domain 和 path
      const header = serializeCookie(name, "", {
        ...options,
        maxAge: 0,
        expires: new Date(0),
      });
      requestContext!.setCookieHeaders.push(header);
    },

    /**
     * Set multiple cookies at once
     */
    setMultiple(
      cookies: Record<string, { value: string; options?: CookieOptions }>,
    ): void {
      for (const [name, { value, options }] of Object.entries(cookies)) {
        this.set(name, value, options);
      }
    },

    /**
     * Delete multiple cookies at once
     */
    deleteMultiple(
      names: string[],
      options?: Pick<CookieOptions, "domain" | "path">,
    ): void {
      for (const name of names) {
        this.delete(name, options);
      }
    },
  };
}

/**
 * 为给定的请求上下文提取 Set-Cookie 头
 * 被 main.ts 调用以添加 cookies 到 HTTP 响应
 * @param ctx Page 上下文
 * @returns Set-Cookie 头值数组，如果没有则返回 undefined
 */
export function extractSetCookieHeaders(
  ctx: PageContext,
): string[] | undefined {
  const requestContext = cookieContextMap.get(ctx);
  return requestContext?.setCookieHeaders;
}
