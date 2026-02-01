/**
 * 文件管理器 API 处理器
 * 实现所有文件操作的 API 端点
 */

import { join, dirname, basename } from "std/path";
import type {
  FileManagerConfig,
  FileInfo,
  BrowseResult,
  APIResponse,
  LoginRequest,
  LoginResponse,
  RenameRequest,
  MoveRequest,
  MkdirRequest,
  DeleteRequest,
} from "./types.ts";
import {
  validatePath,
  isSafeFilename,
  isAllowedExtension,
  isAllowedFileSize,
  validateTargetPath,
  checkOperationPermission,
} from "./security.ts";
import {
  verifyPassword,
  createSession,
  destroySession,
  getCSRFToken,
  verifyCSRFToken,
  getSessionIdFromCookies,
  createSessionCookieHeader,
  createClearSessionCookieHeader,
} from "./auth.ts";
import { formatFileSize, formatDateTime } from "./config.ts";

// ============== 辅助函数 ==============

/**
 * 创建 JSON 响应
 */
function createJSONResponse<T>(
  data: APIResponse<T>,
  status: number = 200,
): Response {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      "Content-Type": "application/json; charset=utf-8",
    },
  });
}

/**
 * 创建错误响应
 */
function createErrorResponse(message: string, status: number = 400): Response {
  return createJSONResponse({ success: false, error: message }, status);
}

/**
 * 从路径读取文件信息
 */
async function getFileInfo(path: string): Promise<FileInfo> {
  const stat = await Deno.stat(path);
  const name = basename(path);
  const lastDotIndex = name.lastIndexOf(".");

  return {
    name,
    isDirectory: stat.isDirectory,
    size: stat.size,
    modifiedTime: stat.mtime || new Date(),
    extension: lastDotIndex !== -1 ? name.slice(lastDotIndex) : undefined,
  };
}

// ============== API 处理器 ==============

/**
 * 处理登录请求
 */
export async function handleLoginAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    // 解析请求体
    const body = await req.json() as LoginRequest;

    // 验证密码
    const isValid = await verifyPassword(
      body.password,
      config.password,
      config.password, // 这里假设配置中的密码已经是哈希值，实际应该分离存储
    );

    // 注意：由于简化实现，这里直接比较明文密码
    // 在生产环境中，应该在配置文件中存储哈希后的密码
    if (body.password !== config.password) {
      return createErrorResponse("密码错误", 401);
    }

    // 创建 session
    const sessionId = createSession();

    // 获取 CSRF Token
    const csrfToken = getCSRFToken(sessionId);

    // 创建响应
    const responseData: LoginResponse = {
      csrfToken: csrfToken!,
    };

    const response = createJSONResponse({ success: true, data: responseData });

    // 设置 cookie
    response.headers.set(
      "Set-Cookie",
      createSessionCookieHeader(sessionId),
    );

    return response;
  } catch (error) {
    return createErrorResponse("登录失败", 500);
  }
}

/**
 * 处理登出请求
 */
export async function handleLogoutAPI(req: Request): Promise<Response> {
  const cookies: Record<string, string> = {};
  const cookieHeader = req.headers.get("cookie");
  if (cookieHeader) {
    for (const pair of cookieHeader.split(";")) {
      const [key, value] = pair.trim().split("=");
      if (key && value) {
        cookies[key] = decodeURIComponent(value);
      }
    }
  }

  const sessionId = getSessionIdFromCookies(cookies);
  if (sessionId) {
    destroySession(sessionId);
  }

  const response = createJSONResponse({ success: true });

  // 清除 cookie
  response.headers.set(
    "Set-Cookie",
    createClearSessionCookieHeader(),
  );

  return response;
}

/**
 * 处理目录浏览请求
 */
export async function handleBrowseAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    const url = new URL(req.url);
    const requestedPath = url.searchParams.get("path") || rootPath;

    // 验证路径
    const validation = validatePath(requestedPath, rootPath, config);
    if (!validation.success) {
      return createErrorResponse(validation.error!, 403);
    }

    const targetPath = validation.normalizedPath!;

    // 检查是否是目录
    const stat = await Deno.stat(targetPath);
    if (!stat.isDirectory) {
      return createErrorResponse("路径不是目录", 400);
    }

    // 读取目录内容
    const entries: Deno.DirEntry[] = [];
    for await (const entry of Deno.readDir(targetPath)) {
      entries.push(entry);
    }

    // 构建文件信息列表
    const files: FileInfo[] = [];
    for (const entry of entries) {
      const entryPath = join(targetPath, entry.name);
      const entryStat = await Deno.stat(entryPath);

      // 跳过隐藏文件（以点开头）
      if (entry.name.startsWith(".")) {
        continue;
      }

      const lastDotIndex = entry.name.lastIndexOf(".");
      files.push({
        name: entry.name,
        isDirectory: entry.isDirectory,
        size: entryStat.size,
        modifiedTime: entryStat.mtime || new Date(),
        extension: !entry.isDirectory && lastDotIndex !== -1
          ? entry.name.slice(lastDotIndex)
          : undefined,
      });
    }

    // 排序：目录在前，文件在后，按名称排序
    files.sort((a, b) => {
      if (a.isDirectory && !b.isDirectory) return -1;
      if (!a.isDirectory && b.isDirectory) return 1;
      return a.name.localeCompare(b.name);
    });

    // 计算父级路径
    const parentPath = dirname(targetPath) !== targetPath
      ? dirname(targetPath)
      : null;

    const result: BrowseResult = {
      path: targetPath,
      parentPath,
      files,
    };

    return createJSONResponse({ success: true, data: result });
  } catch (error) {
    return createErrorResponse(
      `浏览目录失败: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}

/**
 * 处理文件上传请求
 */
export async function handleUploadAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    const url = new URL(req.url);
    const targetDir = url.searchParams.get("path") || rootPath;

    // 验证目标目录
    const validation = validatePath(targetDir, rootPath, config);
    if (!validation.success) {
      return createErrorResponse(validation.error!, 403);
    }

    // 检查是否是目录
    const stat = await Deno.stat(validation.normalizedPath!);
    if (!stat.isDirectory) {
      return createErrorResponse("目标路径不是目录", 400);
    }

    // 解析 multipart 表单数据
    const formData = await req.formData();
    const file = formData.get("file") as File;

    if (!file) {
      return createErrorResponse("未找到上传的文件", 400);
    }

    // 验证文件名
    if (!isSafeFilename(file.name)) {
      return createErrorResponse("文件名包含非法字符", 400);
    }

    // 验证扩展名
    if (!isAllowedExtension(file.name, config)) {
      return createErrorResponse("不允许上传此类型的文件", 403);
    }

    // 验证文件大小
    if (!isAllowedFileSize(file.size, config)) {
      return createErrorResponse(
        `文件大小超过限制 (${formatFileSize(config.maxUploadSize)})`,
        403,
      );
    }

    // 保存文件
    const targetPath = join(validation.normalizedPath!, file.name);
    const arrayBuffer = await file.arrayBuffer();
    const uint8Array = new Uint8Array(arrayBuffer);
    await Deno.writeFile(targetPath, uint8Array);

    return createJSONResponse({
      success: true,
      data: { message: "文件上传成功" },
    });
  } catch (error) {
    return createErrorResponse(
      `文件上传失败: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}

/**
 * 处理文件下载请求
 */
export async function handleDownloadAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    const url = new URL(req.url);
    const filePath = url.searchParams.get("path");

    if (!filePath) {
      return createErrorResponse("未指定文件路径", 400);
    }

    // 验证路径
    const validation = validatePath(filePath, rootPath, config);
    if (!validation.success) {
      return createErrorResponse(validation.error!, 403);
    }

    const targetPath = validation.normalizedPath!;

    // 检查是否是文件
    const stat = await Deno.stat(targetPath);
    if (stat.isDirectory) {
      return createErrorResponse("不能下载目录", 400);
    }

    // 读取文件
    const fileContent = await Deno.readFile(targetPath);
    const filename = basename(targetPath);

    return new Response(fileContent, {
      status: 200,
      headers: {
        "Content-Type": "application/octet-stream",
        "Content-Disposition": `attachment; filename="${encodeURIComponent(filename)}"`,
        "Content-Length": String(fileContent.length),
      },
    });
  } catch (error) {
    return createErrorResponse(
      `文件下载失败: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}

/**
 * 处理删除请求
 */
export async function handleDeleteAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    // 检查权限
    if (!checkOperationPermission("delete", config)) {
      return createErrorResponse("删除操作已被禁用", 403);
    }

    const body = await req.json() as DeleteRequest;

    // 验证路径
    const validation = validatePath(body.path, rootPath, config);
    if (!validation.success) {
      return createErrorResponse(validation.error!, 403);
    }

    const targetPath = validation.normalizedPath!;

    // 检查是否在根目录（防止删除根目录）
    if (targetPath === rootPath) {
      return createErrorResponse("不能删除根目录", 403);
    }

    // 删除文件或目录
    const stat = await Deno.stat(targetPath);
    if (stat.isDirectory) {
      await Deno.remove(targetPath, { recursive: true });
    } else {
      await Deno.remove(targetPath);
    }

    return createJSONResponse({
      success: true,
      data: { message: "删除成功" },
    });
  } catch (error) {
    return createErrorResponse(
      `删除失败: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}

/**
 * 处理重命名请求
 */
export async function handleRenameAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    // 检查权限
    if (!checkOperationPermission("rename", config)) {
      return createErrorResponse("重命名操作已被禁用", 403);
    }

    const body = await req.json() as RenameRequest;

    // 验证新名称
    if (!isSafeFilename(body.newName)) {
      return createErrorResponse("文件名包含非法字符", 400);
    }

    // 验证源路径
    const sourceValidation = validatePath(body.oldPath, rootPath, config);
    if (!sourceValidation.success) {
      return createErrorResponse(sourceValidation.error!, 403);
    }

    // 构建目标路径
    const parentDir = dirname(body.oldPath);
    const targetPath = join(parentDir, body.newName);

    // 验证目标路径
    const targetValidation = validatePath(targetPath, rootPath, config);
    if (targetValidation.success) {
      // 目标已存在
      return createErrorResponse("目标文件已存在", 409);
    }

    // 重命名
    await Deno.rename(body.oldPath, targetPath);

    return createJSONResponse({
      success: true,
      data: { message: "重命名成功" },
    });
  } catch (error) {
    return createErrorResponse(
      `重命名失败: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}

/**
 * 处理创建目录请求
 */
export async function handleMkdirAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    // 检查权限
    if (!checkOperationPermission("mkdir", config)) {
      return createErrorResponse("创建目录操作已被禁用", 403);
    }

    const body = await req.json() as MkdirRequest;

    // 验证目录名
    if (!isSafeFilename(body.dirName)) {
      return createErrorResponse("目录名包含非法字符", 400);
    }

    // 构建完整路径
    const targetPath = join(body.parentPath, body.dirName);

    // 验证父目录
    const parentValidation = validatePath(body.parentPath, rootPath, config);
    if (!parentValidation.success) {
      return createErrorResponse(parentValidation.error!, 403);
    }

    // 检查父目录是否是目录
    const parentStat = await Deno.stat(parentValidation.normalizedPath!);
    if (!parentStat.isDirectory) {
      return createErrorResponse("父路径不是目录", 400);
    }

    // 创建目录
    await Deno.mkdir(targetPath);

    return createJSONResponse({
      success: true,
      data: { message: "目录创建成功" },
    });
  } catch (error) {
    if (error instanceof Deno.errors.AlreadyExists) {
      return createErrorResponse("目录已存在", 409);
    }
    return createErrorResponse(
      `创建目录失败: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}

/**
 * 检查请求是否已认证
 */
export async function isAuthenticated(
  req: Request,
): Promise<boolean> {
  const cookies: Record<string, string> = {};
  const cookieHeader = req.headers.get("cookie");
  if (cookieHeader) {
    for (const pair of cookieHeader.split(";")) {
      const [key, value] = pair.trim().split("=");
      if (key && value) {
        cookies[key] = decodeURIComponent(value);
      }
    }
  }

  const sessionId = getSessionIdFromCookies(cookies);
  if (!sessionId) {
    return false;
  }

  const { validateSession } = await import("./auth.ts");
  return validateSession(sessionId);
}
