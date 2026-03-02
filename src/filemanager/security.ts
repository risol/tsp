/**
 * File manager security check module
 * Provides path validation, permission control, blacklist/whitelist checking
 */

import { join, normalize, resolve, relative, dirname, basename } from "std/path";
import type { FileManagerConfig, ArchiveType } from "./types.ts";

/**
 * Path validation result
 */
export interface PathValidationResult {
  success: boolean;
  normalizedPath?: string;
  error?: string;
}

/**
 * Validate if path is allowed to access
 * @param requestedPath Requested path
 * @param rootPath Root directory path
 * @param config File manager configuration
 * @returns Validation result
 */
export function validatePath(
  requestedPath: string,
  rootPath: string,
  config: Required<FileManagerConfig>,
): PathValidationResult {
  try {
    // 1. Normalize path
    let normalizedPath: string;

    // If absolute path
    if (requestedPath.startsWith("/") || /^[A-Za-z]:/.test(requestedPath)) {
      normalizedPath = normalize(requestedPath);
    } else {
      // Relative path, resolve relative to current working directory
      normalizedPath = resolve(requestedPath);
    }

    // 2. Check if path exists
    try {
      const stat = Deno.statSync(normalizedPath);
      // Path exists, continue checking
    } catch (error) {
      if (error instanceof Deno.errors.NotFound) {
        return {
          success: false,
          error: "Path does not exist",
        };
      }
      return {
        success: false,
        error: "Cannot access path",
      };
    }

    // 3. Check blacklist
    const pathBasename = basename(normalizedPath);
    for (const deniedPattern of config.deniedPaths) {
      // Check if path contains blacklist pattern
      if (
        normalizedPath.includes(deniedPattern) ||
        pathBasename === deniedPattern
      ) {
        return {
          success: false,
          error: `Access denied: path contains forbidden directory or file "${deniedPattern}"`,
        };
      }
    }

    // 4. Check whitelist (if configured)
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
          error: "Access denied: path not in allowed list",
        };
      }
    }

    // 5. Check if allow access outside root
    if (!config.allowOutsideRoot) {
      // Calculate relative path
      const relPath = relative(rootPath, normalizedPath);

      // If relative path starts with .., it's outside root
      if (relPath.startsWith("..")) {
        return {
          success: false,
          error: `Access denied: not allowed to access path outside web root (${rootPath})`,
        };
      }
    }

    // 6. Prevent path traversal attack (check again)
    const resolvedPath = resolve(normalizedPath);
    if (resolvedPath !== normalizedPath) {
      // Normalized path and resolved path don't match, may contain ..
      // But if allowOutsideRoot is true, this is legal
      if (!config.allowOutsideRoot) {
        return {
          success: false,
          error: "Access denied: path traversal attack detected",
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
      error: `Path validation failed: ${error instanceof Error ? error.message : String(error)}`,
    };
  }
}

/**
 * Validate if filename is safe
 * @param filename Filename
 * @returns Whether safe
 */
export function isSafeFilename(filename: string): boolean {
  // Check if empty
  if (!filename || filename.trim() === "") {
    return false;
  }

  // Check if contains path separator
  if (filename.includes("/") || filename.includes("\\")) {
    return false;
  }

  // Check if contains illegal characters
  const illegalChars = /[<>:"|?*\x00-\x1f]/;
  if (illegalChars.test(filename)) {
    return false;
  }

  // Check if it's a reserved name (Windows)
  const reservedNames = /^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$/i;
  if (reservedNames.test(filename)) {
    return false;
  }

  // Check if starts with dot (hidden file)
  if (filename.startsWith(".")) {
    return false;
  }

  // Check length
  if (filename.length > 255) {
    return false;
  }

  return true;
}

/**
 * Validate if extension is allowed for upload
 * @param filename Filename
 * @param config File manager configuration
 * @returns Whether allowed
 */
export function isAllowedExtension(
  filename: string,
  config: Required<FileManagerConfig>,
): boolean {
  // Extract extension
  const lastDotIndex = filename.lastIndexOf(".");
  if (lastDotIndex === -1) {
    // No extension
    return config.allowedExtensions.length === 0;
  }

  const extension = filename.slice(lastDotIndex).toLowerCase();

  // Check blacklist
  if (config.deniedExtensions.includes(extension)) {
    return false;
  }

  // Check whitelist (if configured)
  if (config.allowedExtensions.length > 0) {
    return config.allowedExtensions.includes(extension);
  }

  // No whitelist configured, allow by default
  return true;
}

/**
 * Validate file size
 * @param fileSize File size (bytes)
 * @param config File manager configuration
 * @returns Whether allowed
 */
export function isAllowedFileSize(
  fileSize: number,
  config: Required<FileManagerConfig>,
): boolean {
  return fileSize <= config.maxUploadSize;
}

/**
 * Validate and normalize target path (for move, rename, etc. operations)
 * @param sourcePath Source path
 * @param targetName Target name (filename or directory name)
 * @param rootPath Root directory
 * @param config Configuration
 * @returns Validation result
 */
export function validateTargetPath(
  sourcePath: string,
  targetName: string,
  rootPath: string,
  config: Required<FileManagerConfig>,
): PathValidationResult {
  // Validate target name
  if (!isSafeFilename(targetName)) {
    return {
      success: false,
      error: "Filename contains illegal characters or does not conform to specification",
    };
  }

  // Build complete path
  const parentDir = dirname(sourcePath);
  const targetPath = join(parentDir, targetName);

  // Validate target path
  return validatePath(targetPath, rootPath, config);
}

/**
 * Check operation permission
 * @param operation Operation type
 * @param config Configuration
 * @returns Whether allowed
 */
export function checkOperationPermission(
  operation: "delete" | "rename" | "mkdir" | "move" | "extract" | "compress",
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
    case "extract":
      return config.allowExtract;
    case "compress":
      return config.allowCompress;
    default:
      return false;
  }
}

/**
 * Check if it's a supported archive file format
 * @param filename Filename
 * @param config Configuration
 * @returns Archive file type, or null if not supported
 */
export function getSupportedArchiveType(
  filename: string,
  config: Required<FileManagerConfig>,
): ArchiveType | null {
  if (!config.allowedArchiveExtensions) {
    return null;
  }

  const lowerName = filename.toLowerCase();

  // Check TAR.GZ
  if (lowerName.endsWith(".tar.gz") || lowerName.endsWith(".tgz")) {
    return config.allowedArchiveExtensions.includes("tgz") ? "tgz" : null;
  }

  // Check TAR
  if (lowerName.endsWith(".tar")) {
    return config.allowedArchiveExtensions.includes("tar") ? "tar" : null;
  }

  // Check ZIP
  if (lowerName.endsWith(".zip")) {
    return config.allowedArchiveExtensions.includes("zip") ? "zip" : null;
  }

  return null;
}

/**
 * Validate if extract operation is safe (ZIP bomb protection)
 * @param archivePath Archive file path
 * @param targetDir Target directory
 * @param rootPath Root directory
 * @param config Configuration
 * @returns Validation result
 */
export async function validateExtractOperation(
  archivePath: string,
  targetDir: string,
  rootPath: string,
  config: Required<FileManagerConfig>,
): Promise<PathValidationResult> {
  // Validate archive file path
  const archiveValidation = validatePath(archivePath, rootPath, config);
  if (!archiveValidation.success) {
    return archiveValidation;
  }

  // Validate target directory path
  const targetValidation = validatePath(targetDir, rootPath, config);
  if (!targetValidation.success) {
    return targetValidation;
  }

  // Check file size
  try {
    const stat = await Deno.stat(archiveValidation.normalizedPath!);
    if (stat.size > config.maxExtractSize) {
      return {
        success: false,
        error: `Archive file size exceeds limit (${config.maxExtractSize} bytes)`,
      };
    }
  } catch {
    return {
      success: false,
      error: "Cannot read archive file information",
    };
  }

  return {
    success: true,
    normalizedPath: targetValidation.normalizedPath,
  };
}
