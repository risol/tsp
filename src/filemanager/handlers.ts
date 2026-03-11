/**
 * File manager API handlers
 * Implements all file operation API endpoints
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
  ExtractRequest,
  CompressRequest,
  BatchMoveRequest,
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
import { formatFileSize, formatDateTime, getArchiveType } from "./config.ts";
import { extractArchive, compressToZip, getArchiveSize, getTotalSize } from "./archive.ts";

// ============== Helper Functions ==============

/**
 * Create JSON response
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
 * Create error response
 */
function createErrorResponse(message: string, status: number = 400): Response {
  return createJSONResponse({ success: false, error: message }, status);
}

/**
 * Read file info from path
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

// ============== API Handlers ==============

/**
 * Handle login request
 * @param req Request object
 * @param config File manager configuration
 * @param rootPath Root directory path
 * @param sessionSecure Session secure config
 */
export async function handleLoginAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
  sessionSecure: boolean = true,
): Promise<Response> {
  try {
    // Parse request body
    const body = await req.json() as LoginRequest;

    // Note: Using simplified plaintext password comparison here
    // In production environment, hashed password should be stored in config file
    if (body.password !== config.password) {
      return createErrorResponse(
        "Incorrect password. If you just modified the config file, please restart the server.",
        401
      );
    }

    // Create session
    const sessionId = createSession();

    // Get CSRF Token
    const csrfToken = getCSRFToken(sessionId);

    // Create response
    const responseData: LoginResponse = {
      csrfToken: csrfToken!,
    };

    const response = createJSONResponse({ success: true, data: responseData });

    // Set cookie (using session's secure config)
    response.headers.set(
      "Set-Cookie",
      createSessionCookieHeader(sessionId, sessionSecure),
    );

    return response;
  } catch (error) {
    console.log(`[DEBUG] Login exception:`, error);
    return createErrorResponse("Login failed", 500);
  }
}

/**
 * Handle logout request
 * @param req Request object
 * @param config File manager configuration
 * @param sessionSecure Session secure config
 */
export async function handleLogoutAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  sessionSecure: boolean = true,
): Promise<Response> {
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

  // Clear cookie (using session's secure config, must match when setting)
  response.headers.set(
    "Set-Cookie",
    createClearSessionCookieHeader(sessionSecure),
  );

  return response;
}

/**
 * Handle directory browse request
 */
export async function handleBrowseAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    const url = new URL(req.url);
    const requestedPath = url.searchParams.get("path") || rootPath;

    // Validate path
    const validation = validatePath(requestedPath, rootPath, config);
    if (!validation.success) {
      return createErrorResponse(validation.error!, 403);
    }

    const targetPath = validation.normalizedPath!;

    // Check if it's a directory
    const stat = await Deno.stat(targetPath);
    if (!stat.isDirectory) {
      return createErrorResponse("Path is not a directory", 400);
    }

    // Read directory contents
    const entries: Deno.DirEntry[] = [];
    for await (const entry of Deno.readDir(targetPath)) {
      entries.push(entry);
    }

    // Build file info list
    const files: FileInfo[] = [];
    for (const entry of entries) {
      const entryPath = join(targetPath, entry.name);
      const entryStat = await Deno.stat(entryPath);

      // Skip hidden files (starting with dot)
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

    // Sort: directories first, then files, sorted by name
    files.sort((a, b) => {
      if (a.isDirectory && !b.isDirectory) return -1;
      if (!a.isDirectory && b.isDirectory) return 1;
      return a.name.localeCompare(b.name);
    });

    // Calculate parent path
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
      `Browse directory failed: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}

/**
 * Handle file upload request
 */
export async function handleUploadAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    const url = new URL(req.url);
    const targetDir = url.searchParams.get("path") || rootPath;

    // Validate target directory
    const validation = validatePath(targetDir, rootPath, config);
    if (!validation.success) {
      return createErrorResponse(validation.error!, 403);
    }

    // Check if it's a directory
    const stat = await Deno.stat(validation.normalizedPath!);
    if (!stat.isDirectory) {
      return createErrorResponse("Target path is not a directory", 400);
    }

    // Parse multipart form data
    const formData = await req.formData();
    const file = formData.get("file") as File;

    if (!file) {
      return createErrorResponse("No uploaded file found", 400);
    }

    // Validate filename
    if (!isSafeFilename(file.name)) {
      return createErrorResponse("Filename contains illegal characters", 400);
    }

    // Validate extension
    if (!isAllowedExtension(file.name, config)) {
      return createErrorResponse("This file type is not allowed to upload", 403);
    }

    // Validate file size
    if (!isAllowedFileSize(file.size, config)) {
      return createErrorResponse(
        `File size exceeds limit (${formatFileSize(config.maxUploadSize)})`,
        403,
      );
    }

    // Save file
    const targetPath = join(validation.normalizedPath!, file.name);
    const arrayBuffer = await file.arrayBuffer();
    const uint8Array = new Uint8Array(arrayBuffer);
    await Deno.writeFile(targetPath, uint8Array);

    return createJSONResponse({
      success: true,
      data: { message: "File uploaded successfully" },
    });
  } catch (error) {
    return createErrorResponse(
      `File upload failed: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}

/**
 * Handle file download request
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
      return createErrorResponse("File path not specified", 400);
    }

    // Validate path
    const validation = validatePath(filePath, rootPath, config);
    if (!validation.success) {
      return createErrorResponse(validation.error!, 403);
    }

    const targetPath = validation.normalizedPath!;

    // Check if it's a file
    const stat = await Deno.stat(targetPath);
    if (stat.isDirectory) {
      return createErrorResponse("Cannot download directory", 400);
    }

    // Read file
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
      `File download failed: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}

/**
 * Handle delete request
 */
export async function handleDeleteAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    // Check permission
    if (!checkOperationPermission("delete", config)) {
      return createErrorResponse("Delete operation has been disabled", 403);
    }

    const body = await req.json() as DeleteRequest;

    // Validate path
    const validation = validatePath(body.path, rootPath, config);
    if (!validation.success) {
      return createErrorResponse(validation.error!, 403);
    }

    const targetPath = validation.normalizedPath!;

    // Check if it's root directory (prevent deleting root)
    if (targetPath === rootPath) {
      return createErrorResponse("Cannot delete root directory", 403);
    }

    // Delete file or directory
    const stat = await Deno.stat(targetPath);
    if (stat.isDirectory) {
      await Deno.remove(targetPath, { recursive: true });
    } else {
      await Deno.remove(targetPath);
    }

    return createJSONResponse({
      success: true,
      data: { message: "Deleted successfully" },
    });
  } catch (error) {
    return createErrorResponse(
      `Delete failed: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}

/**
 * Handle rename request
 */
export async function handleRenameAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    // Check permission
    if (!checkOperationPermission("rename", config)) {
      return createErrorResponse("Rename operation has been disabled", 403);
    }

    const body = await req.json() as RenameRequest;

    // Validate new name
    if (!isSafeFilename(body.newName)) {
      return createErrorResponse("Filename contains illegal characters", 400);
    }

    // Validate source path
    const sourceValidation = validatePath(body.oldPath, rootPath, config);
    if (!sourceValidation.success) {
      return createErrorResponse(sourceValidation.error!, 403);
    }

    // Build target path
    const parentDir = dirname(body.oldPath);
    const targetPath = join(parentDir, body.newName);

    // Validate target path
    const targetValidation = validatePath(targetPath, rootPath, config);
    if (targetValidation.success) {
      // Target already exists
      return createErrorResponse("Target file already exists", 409);
    }

    // Rename
    await Deno.rename(body.oldPath, targetPath);

    return createJSONResponse({
      success: true,
      data: { message: "Renamed successfully" },
    });
  } catch (error) {
    return createErrorResponse(
      `Rename failed: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}

/**
 * Handle create directory request
 */
export async function handleMkdirAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    // Check permission
    if (!checkOperationPermission("mkdir", config)) {
      return createErrorResponse("Create directory operation has been disabled", 403);
    }

    const body = await req.json() as MkdirRequest;

    // Validate directory name
    if (!isSafeFilename(body.dirName)) {
      return createErrorResponse("Directory name contains illegal characters", 400);
    }

    // Build complete path
    const targetPath = join(body.parentPath, body.dirName);

    // Validate parent directory
    const parentValidation = validatePath(body.parentPath, rootPath, config);
    if (!parentValidation.success) {
      return createErrorResponse(parentValidation.error!, 403);
    }

    // Check if parent path is a directory
    const parentStat = await Deno.stat(parentValidation.normalizedPath!);
    if (!parentStat.isDirectory) {
      return createErrorResponse("Parent path is not a directory", 400);
    }

    // Create directory
    await Deno.mkdir(targetPath);

    return createJSONResponse({
      success: true,
      data: { message: "Directory created successfully" },
    });
  } catch (error) {
    if (error instanceof Deno.errors.AlreadyExists) {
      return createErrorResponse("Directory already exists", 409);
    }
    return createErrorResponse(
      `Create directory failed: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}

/**
 * Check if request is authenticated
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

/**
 * Handle extract request
 */
export async function handleExtractAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    // Check permission
    if (!checkOperationPermission("extract", config)) {
      return createErrorResponse("Extract operation has been disabled", 403);
    }

    const body = await req.json() as ExtractRequest;

    // Validate archive path
    const archiveValidation = validatePath(body.archivePath, rootPath, config);
    if (!archiveValidation.success) {
      return createErrorResponse(archiveValidation.error!, 403);
    }

    const archivePath = archiveValidation.normalizedPath!;

    // Check if file exists
    const archiveStat = await Deno.stat(archivePath);
    if (archiveStat.isDirectory) {
      return createErrorResponse("Cannot extract directory", 400);
    }

    // Check file size
    if (archiveStat.size > config.maxExtractSize) {
      return createErrorResponse(
        `File size exceeds limit (${formatFileSize(config.maxExtractSize)})`,
        403,
      );
    }

    // Check if it's a supported archive format
    const archiveType = getArchiveType(archivePath);
    if (!archiveType) {
      return createErrorResponse("Unsupported archive format", 400);
    }

    if (!config.allowedArchiveExtensions?.includes(archiveType)) {
      return createErrorResponse(
        archiveType === "zip"
          ? "ZIP format is not supported in binary version. Please use .tar.gz or .tgz format instead."
          : `Not allowed to extract ${archiveType.toUpperCase()} format files`,
        403
      );
    }

    // Determine target directory (if user didn't specify or it's empty, use the directory where the archive is located)
    const targetDir = (body.targetDir && body.targetDir.trim() !== "")
      ? body.targetDir
      : dirname(archivePath);

    // Validate target directory path
    const targetValidation = validatePath(targetDir, rootPath, config);
    if (!targetValidation.success) {
      return createErrorResponse(targetValidation.error!, 403);
    }

    const normalizedTargetDir = targetValidation.normalizedPath!;

    // Ensure target directory exists
    try {
      await Deno.mkdir(normalizedTargetDir, { recursive: true });
    } catch {
      // Ignore already exists error
    }

    // Execute extraction
    await extractArchive(archivePath, normalizedTargetDir, archiveType);

    return createJSONResponse({
      success: true,
      data: { message: "Extracted successfully" },
    });
  } catch (error) {
    return createErrorResponse(
      `Extract failed: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}

/**
 * Handle compress request
 */
export async function handleCompressAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    // Check permission
    if (!checkOperationPermission("compress", config)) {
      return createErrorResponse("Compress operation has been disabled", 403);
    }

    const body = await req.json() as CompressRequest;

    // Validate source path
    if (!body.sourcePaths || body.sourcePaths.length === 0) {
      return createErrorResponse("You must select at least one file or directory", 400);
    }

    const validatedSourcePaths: string[] = [];
    for (const sourcePath of body.sourcePaths) {
      const validation = validatePath(sourcePath, rootPath, config);
      if (!validation.success) {
        return createErrorResponse(validation.error!, 403);
      }
      validatedSourcePaths.push(validation.normalizedPath!);
    }

    // Validate target filename
    const targetFileName = basename(body.targetPath);
    if (!isSafeFilename(targetFileName)) {
      return createErrorResponse("Filename contains illegal characters", 400);
    }

    // Check target file extension
    if (!targetFileName.toLowerCase().endsWith(".zip")) {
      return createErrorResponse("Can only create ZIP format archive files", 400);
    }

    // Determine target path
    const targetDir = dirname(body.targetPath);
    const targetDirValidation = validatePath(targetDir, rootPath, config);
    if (!targetDirValidation.success) {
      return createErrorResponse(targetDirValidation.error!, 403);
    }

    const targetPath = join(targetDirValidation.normalizedPath!, targetFileName);

    // Check if target file already exists
    try {
      await Deno.stat(targetPath);
      return createErrorResponse("Target file already exists", 409);
    } catch {
      // File doesn't exist, can continue
    }

    // Calculate total source file size
    const totalSize = await getTotalSize(validatedSourcePaths);
    if (totalSize > config.maxCompressSize) {
      return createErrorResponse(
        `Total source file size exceeds limit (${formatFileSize(config.maxCompressSize)})`,
        403,
      );
    }

    // Execute compression
    await compressToZip(validatedSourcePaths, targetPath, {
      includeSrc: body.includeSrc ?? false,
    });

    return createJSONResponse({
      success: true,
      data: { message: "Compressed successfully" },
    });
  } catch (error) {
    return createErrorResponse(
      `Compress failed: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}

/**
 * Handle batch move request
 */
export async function handleBatchMoveAPI(
  req: Request,
  config: Required<FileManagerConfig>,
  rootPath: string,
): Promise<Response> {
  try {
    // Check permission
    if (!checkOperationPermission("move", config)) {
      return createErrorResponse("Move operation has been disabled", 403);
    }

    const body = await req.json() as BatchMoveRequest;

    // Validate source path
    if (!body.sourcePaths || body.sourcePaths.length === 0) {
      return createErrorResponse("You must select at least one file or directory", 400);
    }

    const validatedSourcePaths: string[] = [];
    for (const sourcePath of body.sourcePaths) {
      const validation = validatePath(sourcePath, rootPath, config);
      if (!validation.success) {
        return createErrorResponse(validation.error!, 403);
      }
      validatedSourcePaths.push(validation.normalizedPath!);
    }

    // Validate target directory
    const targetValidation = validatePath(body.targetDir, rootPath, config);
    if (!targetValidation.success) {
      return createErrorResponse(targetValidation.error!, 403);
    }

    const normalizedTargetDir = targetValidation.normalizedPath!;

    // Check if target directory exists and is a directory
    try {
      const targetStat = await Deno.stat(normalizedTargetDir);
      if (!targetStat.isDirectory) {
        return createErrorResponse("Target path is not a directory", 400);
      }
    } catch {
      return createErrorResponse("Target directory does not exist", 404);
    }

    // Execute batch move
    const results = {
      success: [] as string[],
      failed: [] as { path: string; error: string }[],
    };

    for (const sourcePath of validatedSourcePaths) {
      try {
        const fileName = basename(sourcePath);
        const targetPath = join(normalizedTargetDir, fileName);

        // Check if target already exists
        try {
          await Deno.stat(targetPath);
          results.failed.push({
            path: sourcePath,
            error: "Target already exists",
          });
          continue;
        } catch {
          // Target doesn't exist, can move
        }

        // Move file or directory
        await Deno.rename(sourcePath, targetPath);
        results.success.push(sourcePath);
      } catch (error) {
        results.failed.push({
          path: sourcePath,
          error: error instanceof Error ? error.message : String(error),
        });
      }
    }

    // Return result
    const message = `Move completed: ${results.success.length} succeeded, ${results.failed.length} failed`;
    return createJSONResponse({
      success: true,
      data: {
        message,
        successCount: results.success.length,
        failedCount: results.failed.length,
        details: results,
      },
    });
  } catch (error) {
    return createErrorResponse(
      `Batch move failed: ${error instanceof Error ? error.message : String(error)}`,
      500,
    );
  }
}
