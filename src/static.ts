/**
 * Static file service module
 * Responsible for serving static files
 */

import { extname } from "std/path";

/**
 * MIME type mapping table
 */
const MIME_TYPES: Record<string, string> = {
  // Text types
  ".html": "text/html; charset=utf-8",
  ".htm": "text/html; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".js": "application/javascript; charset=utf-8",
  ".json": "application/json; charset=utf-8",
  ".txt": "text/plain; charset=utf-8",
  ".md": "text/markdown; charset=utf-8",
  ".xml": "application/xml; charset=utf-8",

  // Image types
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".gif": "image/gif",
  ".svg": "image/svg+xml",
  ".ico": "image/x-icon",
  ".webp": "image/webp",

  // Font types
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".ttf": "font/ttf",
  ".eot": "application/vnd.ms-fontobject",

  // Media types
  ".mp3": "audio/mpeg",
  ".mp4": "video/mp4",
  ".webm": "video/webm",
};

/**
 * Get MIME type for file
 * @param filepath File path
 * @returns MIME type string
 */
export function getMimeType(filepath: string): string {
  const ext = extname(filepath).toLowerCase();
  return MIME_TYPES[ext] || "application/octet-stream";
}

/**
 * Check if file extension is in allowed list
 * @param filepath File path
 * @param allowedExtensions List of allowed extensions
 * @returns Whether access is allowed
 */
export function isStaticFileAllowed(
  filepath: string,
  allowedExtensions: string[],
): boolean {
  const ext = extname(filepath).toLowerCase();
  return allowedExtensions.includes(ext);
}

/**
 * Generate ETag
 * @param content File content
 * @param mtime File modification time
 * @returns ETag string
 */
async function generateETag(
  content: ArrayBuffer,
  mtime: number,
): Promise<string> {
  const hash = await crypto.subtle.digest("SHA-256", content);
  const hashSlice = hash.slice(0, 16);
  const hex = Array.from(new Uint8Array(hashSlice))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
  return `"${hex}-${mtime}"`;
}

/**
 * Serve static file
 * @param filepath File path
 * @param allowedExtensions List of allowed extensions
 * @param isDev Whether in development mode
 * @returns Response object, or null if file is not allowed to access
 */
export async function serveStaticFile(
  filepath: string,
  allowedExtensions: string[],
  isDev: boolean = false,
): Promise<Response | null> {
  // Check if file extension is in allowed list
  if (!isStaticFileAllowed(filepath, allowedExtensions)) {
    return null;
  }

  try {
    // Read file info
    const stat = await Deno.stat(filepath);

    // Reject directories
    if (stat.isDirectory) {
      return null;
    }

    // Read file content
    const content = await Deno.readFile(filepath);

    // Get MIME type
    const mimeType = getMimeType(filepath);

    // Build response headers
    const headers: HeadersInit = {
      "Content-Type": mimeType,
    };

    // Development mode: disable browser caching
    if (isDev) {
      headers["Cache-Control"] = "no-cache, no-store, must-revalidate";
      headers["Pragma"] = "no-cache";
      headers["Expires"] = "0";
    } else {
      // Production mode: use caching strategy
      const mtime = stat.mtime?.getTime() || 0;
      const etag = await generateETag(content.buffer, mtime);

      headers["ETag"] = etag;
      headers["Last-Modified"] = new Date(mtime).toUTCString();
      headers["Cache-Control"] = "public, max-age=86400"; // 1 day
    }

    return new Response(content, {
      status: 200,
      headers,
    });
  } catch (error) {
    // File does not exist or read error
    if (error instanceof Deno.errors.NotFound) {
      return null;
    }
    throw error;
  }
}

/**
 * Handle static file request with ETag validation
 * @param filepath File path
 * @param allowedExtensions List of allowed extensions
 * @param requestHeaders Request headers
 * @param isDev Whether in development mode
 * @returns Response object, or null if file is not allowed to access
 */
export async function serveStaticFileWithCache(
  filepath: string,
  allowedExtensions: string[],
  requestHeaders: Headers,
  isDev: boolean = false,
): Promise<Response | null> {
  // Check if file extension is in allowed list
  if (!isStaticFileAllowed(filepath, allowedExtensions)) {
    return null;
  }

  try {
    // Read file info
    const stat = await Deno.stat(filepath);

    // Reject directories
    if (stat.isDirectory) {
      return null;
    }

    // Read file content
    const content = await Deno.readFile(filepath);

    // Get MIME type
    const mimeType = getMimeType(filepath);

    // Development mode: disable caching
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

    // Production mode: use ETag and Last-Modified
    const mtime = stat.mtime?.getTime() || 0;
    const etag = await generateETag(content.buffer, mtime);
    const lastModified = new Date(mtime).toUTCString();

    // Check If-None-Match (ETag)
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

    // Check If-Modified-Since
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

    // Return file content
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
    // File does not exist or read error
    if (error instanceof Deno.errors.NotFound) {
      return null;
    }
    throw error;
  }
}
