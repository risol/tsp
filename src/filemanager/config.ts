/**
 * File manager configuration validation and defaults
 */

import type { FileManagerConfig, ArchiveType } from "./types.ts";

/**
 * Default configuration
 */
export const DEFAULT_FILE_MANAGER_CONFIG: Required<Omit<
  FileManagerConfig,
  "password" | "allowedPaths" | "deniedPaths" | "allowedExtensions" | "deniedExtensions" | "allowedArchiveExtensions"
>> & {
  password: string;
  allowedPaths: string[];
  deniedPaths: string[];
  allowedExtensions: string[];
  deniedExtensions: string[];
  allowedArchiveExtensions: NonNullable<FileManagerConfig["allowedArchiveExtensions"]>;
} = {
  enabled: false,
  path: "/__filemanager",
  password: "",
  allowOutsideRoot: false,
  allowedPaths: [],
  deniedPaths: [".git", ".deno", "node_modules"],
  maxUploadSize: 100 * 1024 * 1024, // 100MB
  allowedExtensions: [],
  deniedExtensions: [".exe", ".sh", ".bat", ".cmd", ".scr", ".pif"],
  allowDelete: true,
  allowRename: true,
  allowMkdir: true,
  allowMove: true, // Move feature enabled, supports batch move
  // Extract/compress related configuration
  allowExtract: true,
  allowCompress: true,
  allowedArchiveExtensions: ["zip", "tar", "tgz"],
  maxExtractSize: 1024 * 1024 * 1024, // 1GB
  maxCompressSize: 500 * 1024 * 1024, // 500MB
  maxExtractFileCount: 10000,
};

/**
 * Validate file manager configuration
 * @param config User provided configuration
 * @returns Validated complete configuration
 * @throws Throws error if configuration is invalid
 */
export function validateFileManagerConfig(
  config: FileManagerConfig,
): Required<FileManagerConfig> {
  // Check if file manager is enabled
  if (config.enabled === false || config.enabled === undefined) {
    // Not enabled, return default configuration
    return { ...DEFAULT_FILE_MANAGER_CONFIG, password: "" };
  }

  // Enabled, check required fields
  if (!config.password || config.password.trim() === "") {
    throw new Error(
      "Password must be configured when file manager is enabled. Please set fileManager.password in config file",
    );
  }

  // Password strength check (at least 6 characters)
  if (config.password.length < 6) {
    throw new Error("File manager password must be at least 6 characters");
  }

  // Path must start with /
  if (config.path && !config.path.startsWith("/")) {
    throw new Error("File manager path must start with /");
  }

  // Path cannot be root path
  if (config.path === "/") {
    throw new Error("File manager path cannot be root path /");
  }

  // Merge default configuration and user configuration
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
    // Extract/compress related configuration
    allowExtract: config.allowExtract ?? DEFAULT_FILE_MANAGER_CONFIG.allowExtract,
    allowCompress: config.allowCompress ?? DEFAULT_FILE_MANAGER_CONFIG.allowCompress,
    allowedArchiveExtensions: config.allowedArchiveExtensions ?? DEFAULT_FILE_MANAGER_CONFIG.allowedArchiveExtensions,
    maxExtractSize: config.maxExtractSize ?? DEFAULT_FILE_MANAGER_CONFIG.maxExtractSize,
    maxCompressSize: config.maxCompressSize ?? DEFAULT_FILE_MANAGER_CONFIG.maxCompressSize,
    maxExtractFileCount: config.maxExtractFileCount ?? DEFAULT_FILE_MANAGER_CONFIG.maxExtractFileCount,
  };

  // Check if path conflicts with standard routes
  const conflictingPaths = ["/api", "/static", "/favicon.ico"];
  if (conflictingPaths.includes(validated.path)) {
    throw new Error(`File manager path ${validated.path} conflicts with system route`);
  }

  return validated;
}

/**
 * Check if extension is allowed
 * @param extension File extension (including dot, e.g., ".jpg")
 * @param config File manager configuration
 * @returns Whether allowed
 */
export function isExtensionAllowed(
  extension: string,
  config: Required<FileManagerConfig>,
): boolean {
  const ext = extension.toLowerCase();

  // Check blacklist
  if (config.deniedExtensions.length > 0) {
    if (config.deniedExtensions.includes(ext)) {
      return false;
    }
  }

  // Check whitelist (if configured)
  if (config.allowedExtensions.length > 0) {
    return config.allowedExtensions.includes(ext);
  }

  // No whitelist configured, allow by default
  return true;
}

/**
 * Format file size
 * @param bytes Number of bytes
 * @returns Formatted string (e.g., "1.5 MB")
 */
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return "0 B";

  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));

  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(2))} ${sizes[i]}`;
}

/**
 * Format datetime
 * @param date Date object
 * @returns Formatted string (e.g., "2024-01-15 14:30:45")
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

/**
 * Get archive file type from filename
 * @param filename Filename
 * @returns Archive file type, or null if not a supported archive file
 */
export function getArchiveType(filename: string): ArchiveType | null {
  const lowerName = filename.toLowerCase();

  if (lowerName.endsWith(".tar.gz") || lowerName.endsWith(".tgz")) {
    return "tgz";
  }

  if (lowerName.endsWith(".tar")) {
    return "tar";
  }

  if (lowerName.endsWith(".zip")) {
    return "zip";
  }

  return null;
}

/**
 * Check if it's a supported archive file format
 * @param filename Filename
 * @param allowedExtensions List of allowed archive formats
 * @returns Whether it's a supported archive file
 */
export function isArchiveFile(
  filename: string,
  allowedExtensions: ArchiveType[] = ["zip", "tar", "tgz"],
): boolean {
  const archiveType = getArchiveType(filename);
  return archiveType !== null && allowedExtensions.includes(archiveType);
}
