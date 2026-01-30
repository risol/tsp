/**
 * Cookie Management Module for TSP
 *
 * Provides cookie management capabilities through dependency injection.
 * Uses WeakMap to store cookie operations per request context.
 */

/**
 * Standard cookie options compatible with browser Set-Cookie header
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
  sameSite?: 'Strict' | 'Lax' | 'None';
}

/**
 * Cookie manager interface for setting and deleting cookies
 */
export interface CookieManager {
  /** Set a single cookie */
  set: (name: string, value: string, options?: CookieOptions) => void;
  /** Delete a single cookie */
  delete: (name: string, options?: Pick<CookieOptions, 'domain' | 'path'>) => void;
  /** Set multiple cookies at once */
  setMultiple: (cookies: Record<string, { value: string; options?: CookieOptions }>) => void;
  /** Delete multiple cookies at once */
  deleteMultiple: (names: string[], options?: Pick<CookieOptions, 'domain' | 'path'>) => void;
}

/**
 * Internal request context for storing cookie operations
 */
interface RequestContext {
  setCookieHeaders: string[];
}

/**
 * WeakMap to store cookie operations per request
 * WeakMap is used so that when PageContext is garbage collected,
 * the associated cookie context is automatically cleaned up
 */
const cookieContextMap = new WeakMap<PageContext, RequestContext>();

/**
 * Serialize a cookie to Set-Cookie header format
 * @param name Cookie name
 * @param value Cookie value
 * @param options Cookie options
 * @returns Set-Cookie header value
 */
export function serializeCookie(
  name: string,
  value: string,
  options: CookieOptions = {}
): string {
  const parts: string[] = [];

  // Encode name and value (required for special characters)
  parts.push(`${encodeURIComponent(name)}=${encodeURIComponent(value)}`);

  // Max-Age takes precedence over Expires
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
    parts.push('Secure');
  }

  // HttpOnly
  if (options.httpOnly) {
    parts.push('HttpOnly');
  }

  // SameSite
  if (options.sameSite) {
    parts.push(`SameSite=${options.sameSite}`);
  }

  return parts.join('; ');
}

/**
 * Create a cookie manager for the given request context
 * @param ctx Page context
 * @returns Cookie manager instance
 */
export function createCookieManager(ctx: PageContext): CookieManager {
  // Get or create request context
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
    delete(name: string, options?: Pick<CookieOptions, 'domain' | 'path'>): void {
      // To delete a cookie, set it with maxAge=0 and expires in the past
      // Must match domain and path from when it was set
      const header = serializeCookie(name, '', {
        ...options,
        maxAge: 0,
        expires: new Date(0),
      });
      requestContext!.setCookieHeaders.push(header);
    },

    /**
     * Set multiple cookies at once
     */
    setMultiple(cookies: Record<string, { value: string; options?: CookieOptions }>): void {
      for (const [name, { value, options }] of Object.entries(cookies)) {
        this.set(name, value, options);
      }
    },

    /**
     * Delete multiple cookies at once
     */
    deleteMultiple(names: string[], options?: Pick<CookieOptions, 'domain' | 'path'>): void {
      for (const name of names) {
        this.delete(name, options);
      }
    },
  };
}

/**
 * Extract Set-Cookie headers for the given request context
 * Called by main.ts to add cookies to the HTTP response
 * @param ctx Page context
 * @returns Array of Set-Cookie header values, or undefined if none
 */
export function extractSetCookieHeaders(ctx: PageContext): string[] | undefined {
  const requestContext = cookieContextMap.get(ctx);
  return requestContext?.setCookieHeaders;
}
