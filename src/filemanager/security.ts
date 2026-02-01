/**
 * 文件管理器安全检查模块
 * 提供路径验证、权限控制、黑名单/白名单检查
 */

import { join, normalize, resolve, relative, dirname, basename } from "std/path";
import type { FileManagerConfig } from "./types.ts";

/**
 * 路径验证结果
 */
export interface PathValidationResult {
  success: boolean;
  normalizedPath?: string;
  error?: string;
}

/**
 * 验证路径是否允许访问
 * @param requestedPath 请求的路径
 * @param rootPath 根目录路径
 * @param config 文件管理器配置
 * @returns 验证结果
 */
export function validatePath(
  requestedPath: string,
  rootPath: string,
  config: Required<FileManagerConfig>,
): PathValidationResult {
  try {
    // 1. 规范化路径
    let normalizedPath: string;

    // 如果是绝对路径
    if (requestedPath.startsWith("/") || /^[A-Za-z]:/.test(requestedPath)) {
      normalizedPath = normalize(requestedPath);
    } else {
      // 相对路径，相对于当前工作目录解析
      normalizedPath = resolve(requestedPath);
    }

    // 2. 检查路径是否存在
    try {
      const stat = Deno.statSync(normalizedPath);
      // 路径存在，继续检查
    } catch (error) {
      if (error instanceof Deno.errors.NotFound) {
        return {
          success: false,
          error: "路径不存在",
        };
      }
      return {
        success: false,
        error: "无法访问路径",
      };
    }

    // 3. 检查黑名单
    const pathBasename = basename(normalizedPath);
    for (const deniedPattern of config.deniedPaths) {
      // 检查路径中是否包含黑名单模式
      if (
        normalizedPath.includes(deniedPattern) ||
        pathBasename === deniedPattern
      ) {
        return {
          success: false,
          error: `访问被拒绝：路径包含禁止访问的目录或文件 "${deniedPattern}"`,
        };
      }
    }

    // 4. 检查白名单（如果配置了）
    if (config.allowedPaths.length > 0) {
      let allowed = false;
      for (const allowedPattern of config.allowedPaths) {
        if (normalizedPath.includes(allowedPattern)) {
          allowed = true;
          break;
        }
      }

      if (!allowed) {
        return {
          success: false,
          error: "访问被拒绝：路径不在允许的列表中",
        };
      }
    }

    // 5. 检查是否允许访问 root 外
    if (!config.allowOutsideRoot) {
      // 计算相对路径
      const relPath = relative(rootPath, normalizedPath);

      // 如果相对路径以 .. 开头，说明在 root 外
      if (relPath.startsWith("..")) {
        return {
          success: false,
          error: `访问被拒绝：不允许访问网站根目录 (${rootPath}) 外的路径`,
        };
      }
    }

    // 6. 防止路径穿越攻击（再次检查）
    const resolvedPath = resolve(normalizedPath);
    if (resolvedPath !== normalizedPath) {
      // 规范化后的路径与解析后的路径不一致，可能包含 ..
      // 但如果允许访问 root 外，这是合法的
      if (!config.allowOutsideRoot) {
        return {
          success: false,
          error: "访问被拒绝：检测到路径穿越攻击",
        };
      }
    }

    return {
      success: true,
      normalizedPath,
    };
  } catch (error) {
    return {
      success: false,
      error: `路径验证失败: ${error instanceof Error ? error.message : String(error)}`,
    };
  }
}

/**
 * 验证文件名是否安全
 * @param filename 文件名
 * @returns 是否安全
 */
export function isSafeFilename(filename: string): boolean {
  // 检查是否为空
  if (!filename || filename.trim() === "") {
    return false;
  }

  // 检查是否包含路径分隔符
  if (filename.includes("/") || filename.includes("\\")) {
    return false;
  }

  // 检查是否包含非法字符
  const illegalChars = /[<>:"|?*\x00-\x1f]/;
  if (illegalChars.test(filename)) {
    return false;
  }

  // 检查是否是保留名称（Windows）
  const reservedNames = /^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$/i;
  if (reservedNames.test(filename)) {
    return false;
  }

  // 检查是否以点开头（隐藏文件）
  if (filename.startsWith(".")) {
    return false;
  }

  // 检查长度
  if (filename.length > 255) {
    return false;
  }

  return true;
}

/**
 * 验证扩展名是否允许上传
 * @param filename 文件名
 * @param config 文件管理器配置
 * @returns 是否允许
 */
export function isAllowedExtension(
  filename: string,
  config: Required<FileManagerConfig>,
): boolean {
  // 提取扩展名
  const lastDotIndex = filename.lastIndexOf(".");
  if (lastDotIndex === -1) {
    // 没有扩展名
    return config.allowedExtensions.length === 0;
  }

  const extension = filename.slice(lastDotIndex).toLowerCase();

  // 检查黑名单
  if (config.deniedExtensions.includes(extension)) {
    return false;
  }

  // 检查白名单（如果配置了）
  if (config.allowedExtensions.length > 0) {
    return config.allowedExtensions.includes(extension);
  }

  // 没有配置白名单，默认允许
  return true;
}

/**
 * 验证文件大小
 * @param fileSize 文件大小（字节）
 * @param config 文件管理器配置
 * @returns 是否允许
 */
export function isAllowedFileSize(
  fileSize: number,
  config: Required<FileManagerConfig>,
): boolean {
  return fileSize <= config.maxUploadSize;
}

/**
 * 验证并规范化目标路径（用于移动、重命名等操作）
 * @param sourcePath 源路径
 * @param targetName 目标名称（文件名或目录名）
 * @param rootPath 根目录
 * @param config 配置
 * @returns 验证结果
 */
export function validateTargetPath(
  sourcePath: string,
  targetName: string,
  rootPath: string,
  config: Required<FileManagerConfig>,
): PathValidationResult {
  // 验证目标名称
  if (!isSafeFilename(targetName)) {
    return {
      success: false,
      error: "文件名包含非法字符或不符合规范",
    };
  }

  // 构建完整路径
  const parentDir = dirname(sourcePath);
  const targetPath = join(parentDir, targetName);

  // 验证目标路径
  return validatePath(targetPath, rootPath, config);
}

/**
 * 检查操作权限
 * @param operation 操作类型
 * @param config 配置
 * @returns 是否允许
 */
export function checkOperationPermission(
  operation: "delete" | "rename" | "mkdir" | "move",
  config: Required<FileManagerConfig>,
): boolean {
  switch (operation) {
    case "delete":
      return config.allowDelete;
    case "rename":
      return config.allowRename;
    case "mkdir":
      return config.allowMkdir;
    case "move":
      return config.allowMove;
    default:
      return false;
  }
}
