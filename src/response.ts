/**
 * Response 辅助器模块
 * 提供便捷的 HTTP 响应创建方法
 */

import type { PageContext } from "./context.ts";
import type { RedirectResult } from "./cache.ts";

/**
 * Response 辅助器接口
 */
export interface ResponseHelper {
  json<T = unknown>(data: T, status?: number, headers?: HeadersInit): Response;
  text(content: string, status?: number, headers?: HeadersInit): Response;
  html(content: string, status?: number, headers?: HeadersInit): Response;
  redirect(url: string, status?: 301 | 302 | 303 | 307 | 308): RedirectResult;
  error(message: string, status?: number, headers?: HeadersInit): Response;
  file(content: string | Uint8Array, filename: string, headers?: HeadersInit): Response;
  noContent(): Response;
  custom(body?: BodyInit | null, init?: ResponseInit): Response;
}

/**
 * 创建 Response 辅助器实例
 */
export function createResponseHelper(_ctx: PageContext): ResponseHelper {
  return {
    json<T = unknown>(data: T, status = 200, headers: HeadersInit = {}): Response {
      return new Response(JSON.stringify(data), {
        status,
        headers: {
          "Content-Type": "application/json; charset=utf-8",
          ...headers,
        },
      });
    },

    text(content: string, status = 200, headers: HeadersInit = {}): Response {
      return new Response(content, {
        status,
        headers: {
          "Content-Type": "text/plain; charset=utf-8",
          ...headers,
        },
      });
    },

    html(content: string, status = 200, headers: HeadersInit = {}): Response {
      return new Response(content, {
        status,
        headers: {
          "Content-Type": "text/html; charset=utf-8",
          ...headers,
        },
      });
    },

    redirect(url: string, status: 301 | 302 | 303 | 307 | 308 = 302): RedirectResult {
      return { redirect: url, status };
    },

    error(message: string, status = 500, headers: HeadersInit = {}): Response {
      return new Response(
        JSON.stringify({
          error: message,
          status,
          timestamp: new Date().toISOString(),
        }),
        {
          status,
          headers: {
            "Content-Type": "application/json; charset=utf-8",
            ...headers,
          },
        }
      );
    },

    file(content: string | Uint8Array, filename: string, headers: HeadersInit = {}): Response {
      const ext = filename.split(".").pop()?.toLowerCase() || "";
      const mimeTypes: Record<string, string> = {
        txt: "text/plain; charset=utf-8",
        json: "application/json",
        csv: "text/csv",
        xml: "application/xml",
        html: "text/html; charset=utf-8",
        pdf: "application/pdf",
        zip: "application/zip",
        png: "image/png",
        jpg: "image/jpeg",
        jpeg: "image/jpeg",
        gif: "image/gif",
        svg: "image/svg+xml",
      };

      const contentType = mimeTypes[ext] || "application/octet-stream";

      return new Response(content as BodyInit, {
        status: 200,
        headers: {
          "Content-Type": contentType,
          "Content-Disposition": `attachment; filename="${filename}"`,
          ...headers,
        },
      });
    },

    noContent(): Response {
      return new Response(null, { status: 204 });
    },

    custom(body?: BodyInit | null, init?: ResponseInit): Response {
      return new Response(body, init);
    },
  };
}
