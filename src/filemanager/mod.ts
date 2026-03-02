/**
 * File manager main entry module
 * Handles all file manager related requests
 */

import { TSP_VERSION } from "../version.ts";
import type { FileManagerConfig } from "./types.ts";
import {
  handleLoginAPI,
  handleLogoutAPI,
  handleBrowseAPI,
  handleUploadAPI,
  handleDownloadAPI,
  handleDeleteAPI,
  handleRenameAPI,
  handleMkdirAPI,
  handleExtractAPI,
  handleCompressAPI,
  handleBatchMoveAPI,
  isAuthenticated,
} from "./handlers.ts";
import { generateLoginPage, generateFileManagerPage, generateErrorPage } from "./template.ts";
import type { Logger } from "../logger.ts";

/**
 * Handle file manager request
 * @param req Request object
 * @param config File manager configuration
 * @param rootPath Web root directory path
 * @param logger Logger
 * @param sessionSecure Session secure config (for Cookie)
 * @returns Response object
 */
export async function handleFileManagerRequest(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
  logger?: Logger,
  sessionSecure: boolean = true,
): Promise<Response> {
  const url = new URL(req.url);
  const pathname = url.pathname;

  // Remove file manager path prefix
  const basePath = config.path.endsWith("/")
    ? config.path.slice(0, -1)
    : config.path;

  let relativePath = "";
  if (pathname === basePath) {
    relativePath = "";
  } else if (pathname.startsWith(basePath + "/")) {
    relativePath = pathname.slice(basePath.length);
  } else {
    relativePath = pathname;
  }

  logger?.debug("File manager request", {
    method: req.method,
    pathname,
    basePath,
    relativePath,
  });

  try {
    // Route dispatch
    if (relativePath === "" || relativePath === "/") {
      // Main page
      return await handleMainPage(req, config, rootPath);
    } else if (relativePath === "/api/login") {
      // Login API
      return await handleLoginAPI(req, config, rootPath, sessionSecure);
    } else if (relativePath === "/api/logout") {
      // Logout API
      return await handleLogoutAPI(req, config, sessionSecure);
    } else if (relativePath === "/api/browse") {
      // Browse directory API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleBrowseAPI(req, config, rootPath);
    } else if (relativePath === "/api/upload") {
      // Upload API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleUploadAPI(req, config, rootPath);
    } else if (relativePath === "/api/download") {
      // Download API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleDownloadAPI(req, config, rootPath);
    } else if (relativePath === "/api/delete") {
      // Delete API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleDeleteAPI(req, config, rootPath);
    } else if (relativePath === "/api/rename") {
      // Rename API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleRenameAPI(req, config, rootPath);
    } else if (relativePath === "/api/mkdir") {
      // Create directory API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleMkdirAPI(req, config, rootPath);
    } else if (relativePath === "/api/extract") {
      // Extract API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleExtractAPI(req, config, rootPath);
    } else if (relativePath === "/api/compress") {
      // Compress API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleCompressAPI(req, config, rootPath);
    } else if (relativePath === "/api/batch-move") {
      // Batch move API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleBatchMoveAPI(req, config, rootPath);
    } else {
      // 404
      return new Response("Not Found", {
        status: 404,
        headers: { "Content-Type": "text/plain; charset=utf-8" },
      });
    }
  } catch (error) {
    logger?.error("File manager processing error", {
      error: error instanceof Error ? error.message : String(error),
      pathname,
    });

    return new Response(generateErrorPage(
      `File manager error: ${error instanceof Error ? error.message : String(error)}`
    ), {
      status: 500,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }
}

/**
 * Handle main page (show login page or file manager based on authentication status)
 */
async function handleMainPage(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  const authenticated = await isAuthenticated(req);

  if (authenticated) {
    // Authenticated, show file manager
    return new Response(generateFileManagerPage(rootPath), {
      status: 200,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  } else {
    // Not authenticated, show login page
    return new Response(generateLoginPage(), {
      status: 200,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }
}

/**
 * Create unauthorized response
 */
function createUnauthorizedResponse(): Response {
  return new Response(
    JSON.stringify({ success: false, error: "Unauthorized, please login first" }),
    {
      status: 401,
      headers: { "Content-Type": "application/json; charset=utf-8" },
    },
  );
}
