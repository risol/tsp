/**
 * 文件管理器配置验证和默认值
 */

import type { FileManagerConfig } from "./types.ts";

/**
 * 默认配置
 */
export const DEFAULT_FILE_MANAGER_CONFIG: Required<Omit<
  FileManagerConfig,
  "password" | "allowedPaths" | "deniedPaths" | "allowedExtensions" | "deniedExtensions"
>> & {
  password: string;
  allowedPaths: string[];
  deniedPaths: string[];
  allowedExtensions: string[];
  deniedExtensions: string[];
} = {
  enabled: false,
  path: "/__filemanager",
  password: "",
  allowOutsideRoot: false,
  allowedPaths: [],
  deniedPaths: [".git", ".deno", "node_modules", ".cache"],
  maxUploadSize: 100 * 1024 * 1024, // 100MB
  allowedExtensions: [],
  deniedExtensions: [".exe", ".sh", ".bat", ".cmd", ".scr", ".pif"],
  allowDelete: true,
  allowRename: true,
  allowMkdir: true,
  allowMove: false, // 移动功能默认关闭，更安全
};

/**
 * 验证文件管理器配置
 * @param config 用户提供的配置
 * @returns 验证后的完整配置
 * @throws 如果配置无效则抛出错误
 */
export function validateFileManagerConfig(
  config: FileManagerConfig,
): Required<FileManagerConfig> {
  // 检查是否启用了文件管理器
  if (config.enabled === false || config.enabled === undefined) {
    // 未启用，返回默认配置
    return { ...DEFAULT_FILE_MANAGER_CONFIG, password: "" };
  }

  // 启用了，检查必需字段
  if (!config.password || config.password.trim() === "") {
    throw new Error(
      "文件管理器启用时必须配置密码。请在配置文件中设置 fileManager.password",
    );
  }

  // 密码强度检查（至少 6 个字符）
  if (config.password.length < 6) {
    throw new Error("文件管理器密码长度至少为 6 个字符");
  }

  // 路径必须以 / 开头
  if (config.path && !config.path.startsWith("/")) {
    throw new Error("文件管理器路径必须以 / 开头");
  }

  // 路径不能是根路径
  if (config.path === "/") {
    throw new Error("文件管理器路径不能是根路径 /");
  }

  // 合并默认配置和用户配置
  const validated: Required<FileManagerConfig> = {
    enabled: true,
    path: config.path || DEFAULT_FILE_MANAGER_CONFIG.path,
    password: config.password,
    allowOutsideRoot: config.allowOutsideRoot ?? DEFAULT_FILE_MANAGER_CONFIG.allowOutsideRoot,
    allowedPaths: config.allowedPaths || DEFAULT_FILE_MANAGER_CONFIG.allowedPaths,
    deniedPaths: config.deniedPaths || DEFAULT_FILE_MANAGER_CONFIG.deniedPaths,
    maxUploadSize: config.maxUploadSize ?? DEFAULT_FILE_MANAGER_CONFIG.maxUploadSize,
    allowedExtensions: config.allowedExtensions || DEFAULT_FILE_MANAGER_CONFIG.allowedExtensions,
    deniedExtensions: config.deniedExtensions || DEFAULT_FILE_MANAGER_CONFIG.deniedExtensions,
    allowDelete: config.allowDelete ?? DEFAULT_FILE_MANAGER_CONFIG.allowDelete,
    allowRename: config.allowRename ?? DEFAULT_FILE_MANAGER_CONFIG.allowRename,
    allowMkdir: config.allowMkdir ?? DEFAULT_FILE_MANAGER_CONFIG.allowMkdir,
    allowMove: config.allowMove ?? DEFAULT_FILE_MANAGER_CONFIG.allowMove,
  };

  // 检查路径是否与标准路由冲突
  const conflictingPaths = ["/api", "/static", "/favicon.ico"];
  if (conflictingPaths.includes(validated.path)) {
    throw new Error(`文件管理器路径 ${validated.path} 与系统路由冲突`);
  }

  return validated;
}

/**
 * 检查扩展名是否被允许
 * @param extension 文件扩展名（包含点号，如 ".jpg"）
 * @param config 文件管理器配置
 * @returns 是否允许
 */
export function isExtensionAllowed(
  extension: string,
  config: Required<FileManagerConfig>,
): boolean {
  const ext = extension.toLowerCase();

  // 检查黑名单
  if (config.deniedExtensions.length > 0) {
    if (config.deniedExtensions.includes(ext)) {
      return false;
    }
  }

  // 检查白名单（如果配置了）
  if (config.allowedExtensions.length > 0) {
    return config.allowedExtensions.includes(ext);
  }

  // 没有配置白名单，默认允许
  return true;
}

/**
 * 格式化文件大小
 * @param bytes 字节数
 * @returns 格式化后的字符串（如 "1.5 MB"）
 */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return "0 B";

  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

/**
 * 格式化日期时间
 * @param date 日期对象
 * @returns 格式化后的字符串（如 "2024-01-15 14:30:45"）
 */
export function formatDateTime(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  const hours = String(date.getHours()).padStart(2, "0");
  const minutes = String(date.getMinutes()).padStart(2, "0");
  const seconds = String(date.getSeconds()).padStart(2, "0");

  return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`;
}
