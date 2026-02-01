/**
 * 文件管理器主入口模块
 * 处理文件管理器相关的所有请求
 */

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
  isAuthenticated,
} from "./handlers.ts";
import { generateLoginPage, generateFileManagerPage, generateErrorPage } from "./template.ts";
import type { Logger } from "../logger.ts";

/**
 * 处理文件管理器请求
 * @param req 请求对象
 * @param config 文件管理器配置
 * @param rootPath 网站根目录路径
 * @param logger 日志记录器
 * @returns 响应对象
 */
export async function handleFileManagerRequest(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
  logger?: Logger,
): Promise<Response> {
  const url = new URL(req.url);
  const pathname = url.pathname;

  // 移除文件管理器路径前缀
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

  logger?.debug("文件管理器请求", {
    method: req.method,
    pathname,
    basePath,
    relativePath,
  });

  try {
    // 路由分发
    if (relativePath === "" || relativePath === "/") {
      // 主页面
      return await handleMainPage(req, config, rootPath);
    } else if (relativePath === "/api/login") {
      // 登录 API
      return await handleLoginAPI(req, config, rootPath);
    } else if (relativePath === "/api/logout") {
      // 登出 API
      return await handleLogoutAPI(req);
    } else if (relativePath === "/api/browse") {
      // 浏览目录 API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleBrowseAPI(req, config, rootPath);
    } else if (relativePath === "/api/upload") {
      // 上传 API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleUploadAPI(req, config, rootPath);
    } else if (relativePath === "/api/download") {
      // 下载 API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleDownloadAPI(req, config, rootPath);
    } else if (relativePath === "/api/delete") {
      // 删除 API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleDeleteAPI(req, config, rootPath);
    } else if (relativePath === "/api/rename") {
      // 重命名 API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleRenameAPI(req, config, rootPath);
    } else if (relativePath === "/api/mkdir") {
      // 创建目录 API
      if (!(await isAuthenticated(req))) {
        return createUnauthorizedResponse();
      }
      return await handleMkdirAPI(req, config, rootPath);
    } else {
      // 404
      return new Response("Not Found", {
        status: 404,
        headers: { "Content-Type": "text/plain; charset=utf-8" },
      });
    }
  } catch (error) {
    logger?.error("文件管理器处理错误", {
      error: error instanceof Error ? error.message : String(error),
      pathname,
    });

    return new Response(generateErrorPage(
      `文件管理器错误: ${error instanceof Error ? error.message : String(error)}`
    ), {
      status: 500,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }
}

/**
 * 处理主页面（根据认证状态显示登录页或文件管理器）
 */
async function handleMainPage(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  const authenticated = await isAuthenticated(req);

  if (authenticated) {
    // 已认证，显示文件管理器
    return new Response(generateFileManagerPage(rootPath), {
      status: 200,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  } else {
    // 未认证，显示登录页
    return new Response(generateLoginPage(), {
      status: 200,
      headers: { "Content-Type": "text/html; charset=utf-8" },
    });
  }
}

/**
 * 创建未认证响应
 */
function createUnauthorizedResponse(): Response {
  return new Response(
    JSON.stringify({ success: false, error: "未认证，请先登录" }),
    {
      status: 401,
      headers: { "Content-Type": "application/json; charset=utf-8" },
    },
  );
}
