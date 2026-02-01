/**
 * 压缩文件服务模块
 * 封装 @deno-library/compress 库，提供解压和压缩功能
 */

import { zip, tar, tgz } from "@deno-library/compress";
import { join, basename, dirname } from "std/path";
import type { ArchiveType } from "./types.ts";

/**
 * 解压压缩文件
 * @param archivePath 压缩文件路径
 * @param targetDir 目标目录
 * @param type 压缩文件类型
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
      throw new Error(`不支持的压缩格式: ${type}`);
  }
}

/**
 * 压缩为 ZIP 文件
 * @param sourcePaths 源文件路径列表
 * @param targetPath 目标 ZIP 文件路径
 * @param options 选项
 */
export async function compressToZip(
  sourcePaths: string[],
  targetPath: string,
  options?: { includeSrc?: boolean },
): Promise<void> {
  if (sourcePaths.length === 0) {
    throw new Error("至少需要一个源文件");
  }

  if (sourcePaths.length === 1) {
    // 单文件/目录压缩
    await zip.compress(sourcePaths[0], targetPath, {
      excludeSrc: !options?.includeSrc,
    });
  } else {
    // 多文件压缩：创建临时目录
    const tempDir = join(dirname(targetPath), ".temp_" + Date.now());
    await Deno.mkdir(tempDir, { recursive: true });

    try {
      // 复制所有文件到临时目录
      for (const sourcePath of sourcePaths) {
        const fileName = basename(sourcePath);
        const destPath = join(tempDir, fileName);

        // 检查源路径是文件还是目录
        const stat = await Deno.stat(sourcePath);
        if (stat.isDirectory) {
          // 递归复制目录
          await copyDirectory(sourcePath, destPath);
        } else {
          // 复制文件
          await Deno.copyFile(sourcePath, destPath);
        }
      }

      // 压缩临时目录
      await zip.compress(tempDir, targetPath, { excludeSrc: true });
    } finally {
      // 清理临时目录
      await Deno.remove(tempDir, { recursive: true });
    }
  }
}

/**
 * 递归复制目录
 * @param src 源目录路径
 * @param dest 目标目录路径
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
 * 获取压缩文件的预估大小
 * 注意：这是一个估算值，用于 ZIP 炸弹防护
 * @param archivePath 压缩文件路径
 * @returns 压缩文件大小（字节）
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
 * 获取目录中所有文件的总大小
 * @param dirPath 目录路径
 * @returns 总大小（字节）
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
    // 忽略错误，返回当前统计的大小
  }

  return totalSize;
}

/**
 * 计算多个文件/目录的总大小
 * @param paths 路径列表
 * @returns 总大小（字节）
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
      // 忽略无法访问的文件
    }
  }

  return totalSize;
}
