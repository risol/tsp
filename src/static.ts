/**
 * 静态文件服务模块
 * 负责提供静态文件的读取和响应
 */

import { extname } from "std/path";

/**
 * MIME 类型映射表
 */
const MIME_TYPES: Record<string, string> = {
  // 文本类型
  ".html": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
  ".md": "text/markdown; charset=utf-8",
  ".xml": "application/xml; charset=utf-8",

  // 图片类型
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".svg": "image/svg+xml",
  ".ico": "image/x-icon",
  ".webp": "image/webp",

  // 字体类型
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".ttf": "font/ttf",
  ".eot": "application/vnd.ms-fontobject",

  // 媒体类型
  ".mp3": "audio/mpeg",
  ".mp4": "video/mp4",
  ".webm": "video/webm",
};

/**
 * 获取文件的 MIME 类型
 * @param filepath 文件路径
 * @returns MIME 类型字符串
 */
export function getMimeType(filepath: string): string {
  const ext = extname(filepath).toLowerCase();
  return MIME_TYPES[ext] || "application/octet-stream";
}

/**
 * 检查文件扩展名是否在允许列表中
 * @param filepath 文件路径
 * @param allowedExtensions 允许的扩展名列表
 * @returns 是否允许访问
 */
export function isStaticFileAllowed(
  filepath: string,
  allowedExtensions: string[]
): boolean {
  const ext = extname(filepath).toLowerCase();
  return allowedExtensions.includes(ext);
}

/**
 * 生成 ETag
 * @param content 文件内容
 * @param mtime 文件修改时间
 * @returns ETag 字符串
 */
async function generateETag(content: ArrayBuffer, mtime: number): Promise<string> {
  const hash = await crypto.subtle.digest("SHA-256", content);
  const hashSlice = hash.slice(0, 16);
  const hex = Array.from(new Uint8Array(hashSlice))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
  return `"${hex}-${mtime}"`;
}

/**
 * 服务静态文件
 * @param filepath 文件路径
 * @param allowedExtensions 允许的扩展名列表
 * @param isDev 是否开发模式
 * @returns Response 对象，如果文件不允许访问则返回 null
 */
export async function serveStaticFile(
  filepath: string,
  allowedExtensions: string[],
  isDev: boolean = false
): Promise<Response | null> {
  // 检查文件扩展名是否在允许列表中
  if (!isStaticFileAllowed(filepath, allowedExtensions)) {
    return null;
  }

  try {
    // 读取文件信息
    const stat = await Deno.stat(filepath);

    // 拒绝目录
    if (stat.isDirectory) {
      return null;
    }

    // 读取文件内容
    const content = await Deno.readFile(filepath);

    // 获取 MIME 类型
    const mimeType = getMimeType(filepath);

    // 构建响应头
    const headers: HeadersInit = {
      "Content-Type": mimeType,
    };

    // 开发模式：禁用浏览器缓存
    if (isDev) {
      headers["Cache-Control"] = "no-cache, no-store, must-revalidate";
      headers["Pragma"] = "no-cache";
      headers["Expires"] = "0";
    } else {
      // 生产模式：使用缓存策略
      const mtime = stat.mtime?.getTime() || 0;
      const etag = await generateETag(content.buffer, mtime);

      headers["ETag"] = etag;
      headers["Last-Modified"] = new Date(mtime).toUTCString();
      headers["Cache-Control"] = "public, max-age=86400"; // 1天
    }

    return new Response(content, {
      status: 200,
      headers,
    });
  } catch (error) {
    // 文件不存在或读取错误
    if (error instanceof Deno.errors.NotFound) {
      return null;
    }
    throw error;
  }
}

/**
 * 处理带 ETag 验证的静态文件请求
 * @param filepath 文件路径
 * @param allowedExtensions 允许的扩展名列表
 * @param requestHeaders 请求头
 * @param isDev 是否开发模式
 * @returns Response 对象，如果文件不允许访问则返回 null
 */
export async function serveStaticFileWithCache(
  filepath: string,
  allowedExtensions: string[],
  requestHeaders: Headers,
  isDev: boolean = false
): Promise<Response | null> {
  // 检查文件扩展名是否在允许列表中
  if (!isStaticFileAllowed(filepath, allowedExtensions)) {
    return null;
  }

  try {
    // 读取文件信息
    const stat = await Deno.stat(filepath);

    // 拒绝目录
    if (stat.isDirectory) {
      return null;
    }

    // 读取文件内容
    const content = await Deno.readFile(filepath);

    // 获取 MIME 类型
    const mimeType = getMimeType(filepath);

    // 开发模式：禁用缓存
    if (isDev) {
      return new Response(content, {
        status: 200,
        headers: {
          "Content-Type": mimeType,
          "Cache-Control": "no-cache, no-store, must-revalidate",
          "Pragma": "no-cache",
          "Expires": "0",
        },
      });
    }

    // 生产模式：使用 ETag 和 Last-Modified
    const mtime = stat.mtime?.getTime() || 0;
    const etag = await generateETag(content.buffer, mtime);
    const lastModified = new Date(mtime).toUTCString();

    // 检查 If-None-Match (ETag)
    const ifNoneMatch = requestHeaders.get("If-None-Match");
    if (ifNoneMatch === etag) {
      return new Response(null, {
        status: 304, // Not Modified
        headers: {
          "ETag": etag,
          "Last-Modified": lastModified,
          "Cache-Control": "public, max-age=86400",
        },
      });
    }

    // 检查 If-Modified-Since
    const ifModifiedSince = requestHeaders.get("If-Modified-Since");
    if (ifModifiedSince) {
      const ifModifiedSinceTime = new Date(ifModifiedSince).getTime();
      if (ifModifiedSinceTime >= mtime) {
        return new Response(null, {
          status: 304, // Not Modified
          headers: {
            "ETag": etag,
            "Last-Modified": lastModified,
            "Cache-Control": "public, max-age=86400",
          },
        });
      }
    }

    // 返回文件内容
    return new Response(content, {
      status: 200,
      headers: {
        "Content-Type": mimeType,
        "ETag": etag,
        "Last-Modified": lastModified,
        "Cache-Control": "public, max-age=86400",
      },
    });
  } catch (error) {
    // 文件不存在或读取错误
    if (error instanceof Deno.errors.NotFound) {
      return null;
    }
    throw error;
  }
}
