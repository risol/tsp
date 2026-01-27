/**
 * 路由和文件映射模块
 * 负责 URL 到文件系统的映射
 */

import { join, normalize, resolve } from "jsr:@std/path@1.0.0";

// 路径解析结果
interface PathResult {
  success: boolean;
  filepath?: string;
  error?: string;
}

// 安全检查结果
interface SecurityResult {
  success: boolean;
  error?: string;
}

// 允许的文件扩展名
const ALLOWED_EXTENSIONS = [".tsx"];

/**
 * 解析 URL 路径到文件系统路径
 * @param pathname URL 路径
 * @param root 文档根目录
 * @returns 解析结果
 */
export function resolvePath(pathname: string, root: string): PathResult {
  try {
    // 移除开头的斜杠并解码
    const decoded = decodeURIComponent(pathname.slice(1));

    // 如果路径为空，尝试 index.tsx
    if (!decoded) {
      return {
        success: true,
        filepath: join(root, "index.tsx"),
      };
    }

    // 检查路径是否已经有允许的扩展名
    const hasExtension = ALLOWED_EXTENSIONS.some(ext => decoded.endsWith(ext));

    if (hasExtension) {
      // 路径已有扩展名，直接使用
      return {
        success: true,
        filepath: join(root, decoded),
      };
    } else {
      // 添加 .tsx 扩展名
      return {
        success: true,
        filepath: join(root, decoded + ".tsx"),
      };
    }
  } catch (error) {
    return {
      success: false,
      error: "Invalid path",
    };
  }
}

/**
 * 尝试查找文件（处理目录/index.tsp 情况）
 * @param basePath 基础路径
 * @returns 文件路径或 null
 */
async function findFile(basePath: string): Promise<string | null> {
  // 检查是否存在且是文件
  try {
    const stat = await Deno.stat(basePath);
    if (stat.isFile) {
      return basePath;
    }
  } catch {
    // 文件不存在，继续尝试其他路径
  }

  // 尝试目录下的 index.tsx
  const indexPath = join(basePath, "index.tsx");
  try {
    const stat = await Deno.stat(indexPath);
    if (stat.isFile) {
      return indexPath;
    }
  } catch {
    // 不存在
  }

  return null;
}

/**
 * 检查文件扩展名是否允许
 * @param filepath 文件路径
 * @returns 是否允许
 */
function checkExtension(filepath: string): boolean {
  return ALLOWED_EXTENSIONS.some((ext) => filepath.endsWith(ext));
}

/**
 * 安全检查
 * @param filepath 文件路径
 * @param root 文档根目录
 * @returns 安全检查结果
 */
export async function securityCheck(
  filepath: string,
  root: string
): Promise<SecurityResult> {
  try {
    // 1. 检查文件扩展名
    if (!checkExtension(filepath)) {
      return {
        success: false,
        error: "File type not allowed",
      };
    }

    // 2. 规范化路径并解析为绝对路径
    const normalizedFilepath = normalize(filepath);
    const normalizedRoot = normalize(root);
    const absoluteFilepath = resolve(normalizedFilepath);
    const absoluteRoot = resolve(normalizedRoot);

    // 3. 检查文件是否存在
    let finalFilepath = absoluteFilepath;

    try {
      const stat = await Deno.stat(absoluteFilepath);

      // 如果路径是目录，尝试查找 index.tsp
      if (stat.isDirectory) {
        const indexFile = await findFile(absoluteFilepath);
        if (!indexFile) {
          return {
            success: false,
            error: "Directory index not found",
          };
        }
        finalFilepath = indexFile;
      }
      // 如果是文件，直接使用
    } catch (statError) {
      // 文件或目录不存在
      return {
        success: false,
        error: "File not found",
      };
    }

    // 4. 检查路径穿越攻击（确保文件在 root 目录内）
    const computedPath = relativePath(absoluteRoot, finalFilepath);
    if (computedPath.startsWith("..") || isAbsolutePathOutside(computedPath)) {
      return {
        success: false,
        error: "Access denied",
      };
    }

    return { success: true };
  } catch (error) {
    return {
      success: false,
      error: "File not found",
    };
  }
}

/**
 * 计算相对路径
 * @param from 起始路径
 * @param to 目标路径
 * @returns 相对路径
 */
function relativePath(from: string, to: string): string {
  const fromParts = from.split(/[/\\]/);
  const toParts = to.split(/[/\\]/);

  // 找到公共前缀
  let commonLength = 0;
  for (let i = 0; i < Math.min(fromParts.length, toParts.length); i++) {
    if (fromParts[i] === toParts[i]) {
      commonLength++;
    } else {
      break;
    }
  }

  // 构建相对路径
  const upLevels = fromParts.length - commonLength - 1;
  const downParts = toParts.slice(commonLength);

  const relativeParts = [];
  for (let i = 0; i < upLevels; i++) {
    relativeParts.push("..");
  }
  relativeParts.push(...downParts);

  return relativeParts.join("/");
}

/**
 * 检查路径是否在根目录外
 * @param relativePath 相对路径
 * @returns 是否在根目录外
 */
function isAbsolutePathOutside(relativePath: string): boolean {
  return relativePath.startsWith("/") || /^[a-zA-Z]:/.test(relativePath);
}
