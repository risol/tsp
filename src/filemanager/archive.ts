/**
 * Archive file service module
 * Wraps @deno-library/compress library, provides extract and compress functionality
 */

import { zip, tar, tgz } from "@deno-library/compress";
import { join, basename, dirname } from "std/path";
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
      await zip.uncompress(archivePath, targetDir);
      break;
    case "tar":
      await tar.uncompress(archivePath, targetDir);
      break;
    case "tgz":
      await tgz.uncompress(archivePath, targetDir);
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
    await zip.compress(sourcePaths[0], targetPath, {
      excludeSrc: !options?.includeSrc,
    });
  } else {
    // Multiple file compression: create temp directory
    const tempDir = join(dirname(targetPath), ".temp_" + Date.now());
    await Deno.mkdir(tempDir, { recursive: true });

    try {
      // Copy all files to temp directory
      for (const sourcePath of sourcePaths) {
        const fileName = basename(sourcePath);
        const destPath = join(tempDir, fileName);

        // Check if source path is file or directory
        const stat = await Deno.stat(sourcePath);
        if (stat.isDirectory) {
          // Recursively copy directory
          await copyDirectory(sourcePath, destPath);
        } else {
          // Copy file
          await Deno.copyFile(sourcePath, destPath);
        }
      }

      // Compress temp directory
      await zip.compress(tempDir, targetPath, { excludeSrc: true });
    } finally {
      // Clean up temp directory
      await Deno.remove(tempDir, { recursive: true });
    }
  }
}

/**
 * Recursively copy directory
 * @param src Source directory path
 * @param dest Target directory path
 */
async function copyDirectory(src: string, dest: string): Promise<void> {
  await Deno.mkdir(dest, { recursive: true });

  for await (const entry of Deno.readDir(src)) {
    const srcPath = join(src, entry.name);
    const destPath = join(dest, entry.name);

    if (entry.isDirectory) {
      await copyDirectory(srcPath, destPath);
    } else {
      await Deno.copyFile(srcPath, destPath);
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
    const stat = await Deno.stat(archivePath);
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
    for await (const entry of Deno.readDir(dirPath)) {
      const entryPath = join(dirPath, entry.name);

      if (entry.isDirectory) {
        totalSize += await getDirectorySize(entryPath);
      } else {
        const stat = await Deno.stat(entryPath);
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
      const stat = await Deno.stat(path);

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
