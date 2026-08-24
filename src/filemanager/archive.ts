/**
 * Archive file service module
 * Wraps the npm `compressing` library for archive operations.
 */

import compressing from "compressing";
import { join, basename, dirname } from "node:path";
import { runtime } from "../runtime/platform.ts";
import type { ArchiveType } from "./types.ts";

/**
 * Extract archive file
 * @param archivePath Archive file path
 * @param targetDir Target directory
 * @param type Archive file type
 */
export async function extractArchive(
  archivePath: string,
  targetDir: string,
  type: ArchiveType,
): Promise<void> {
  switch (type) {
    case "zip":
      await compressing.zip.uncompress(archivePath, targetDir);
      break;
    case "tar":
      await compressing.tar.uncompress(archivePath, targetDir);
      break;
    case "tgz":
      await compressing.tgz.uncompress(archivePath, targetDir);
      break;
    default:
      throw new Error(`Unsupported archive format: ${type}`);
  }
}

/**
 * Compress to ZIP file
 * @param sourcePaths Source file path list
 * @param targetPath Target ZIP file path
 * @param options Options
 */
export async function compressToZip(
  sourcePaths: string[],
  targetPath: string,
  options?: { includeSrc?: boolean },
): Promise<void> {
  if (sourcePaths.length === 0) {
    throw new Error("At least one source file is required");
  }

  if (sourcePaths.length === 1) {
    // Single file/directory compression
    const sourceStat = await runtime.stat(sourcePaths[0]);
    await (sourceStat.isDirectory
      ? compressing.zip.compressDir(sourcePaths[0], targetPath, {
          includeParentDir: options?.includeSrc ?? false,
        })
      : compressing.zip.compressFile(sourcePaths[0], targetPath));
  } else {
    // Multiple file compression: create temp directory
    const tempDir = join(dirname(targetPath), ".temp_" + Date.now());
    await runtime.mkdir(tempDir, { recursive: true });

    try {
      // Copy all files to temp directory
      for (const sourcePath of sourcePaths) {
        const fileName = basename(sourcePath);
        const destPath = join(tempDir, fileName);

        // Check if source path is file or directory
        const stat = await runtime.stat(sourcePath);
        if (stat.isDirectory) {
          // Recursively copy directory
          await copyDirectory(sourcePath, destPath);
        } else {
          // Copy file
          await runtime.copyFile(sourcePath, destPath);
        }
      }

      // Compress temp directory
      await compressing.zip.compressDir(tempDir, targetPath, { includeParentDir: false });
    } finally {
      // Clean up temp directory
      await runtime.remove(tempDir, { recursive: true });
    }
  }
}

/**
 * Recursively copy directory
 * @param src Source directory path
 * @param dest Target directory path
 */
async function copyDirectory(src: string, dest: string): Promise<void> {
  await runtime.mkdir(dest, { recursive: true });

  for await (const entry of runtime.readDir(src)) {
    const srcPath = join(src, entry.name);
    const destPath = join(dest, entry.name);

    if (entry.isDirectory) {
      await copyDirectory(srcPath, destPath);
    } else {
      await runtime.copyFile(srcPath, destPath);
    }
  }
}

/**
 * Get estimated size of archive file
 * Note: This is an estimate, used for ZIP bomb protection
 * @param archivePath Archive file path
 * @returns Archive file size (bytes)
 */
export async function getArchiveSize(archivePath: string): Promise<number> {
  try {
    const stat = await runtime.stat(archivePath);
    return stat.size;
  } catch {
    return 0;
  }
}

/**
 * Get total size of all files in directory
 * @param dirPath Directory path
 * @returns Total size (bytes)
 */
export async function getDirectorySize(dirPath: string): Promise<number> {
  let totalSize = 0;

  try {
    for await (const entry of runtime.readDir(dirPath)) {
      const entryPath = join(dirPath, entry.name);

      if (entry.isDirectory) {
        totalSize += await getDirectorySize(entryPath);
      } else {
        const stat = await runtime.stat(entryPath);
        totalSize += stat.size;
      }
    }
  } catch {
    // Ignore error, return current counted size
  }

  return totalSize;
}

/**
 * Calculate total size of multiple files/directories
 * @param paths Path list
 * @returns Total size (bytes)
 */
export async function getTotalSize(paths: string[]): Promise<number> {
  let totalSize = 0;

  for (const path of paths) {
    try {
      const stat = await runtime.stat(path);

      if (stat.isDirectory) {
        totalSize += await getDirectorySize(path);
      } else {
        totalSize += stat.size;
      }
    } catch {
      // Ignore inaccessible files
    }
  }

  return totalSize;
}
