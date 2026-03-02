/**
 * Router and file mapping module
 * Responsible for URL to filesystem mapping
 */

import { join, normalize, resolve } from "std/path";

// Path resolution result
interface PathResult {
  success: boolean;
  filepath?: string;
  error?: string;
}

// Security check result
interface SecurityResult {
  success: boolean;
  error?: string;
}

// Allowed file extensions (TSP pages)
const ALLOWED_EXTENSIONS = [".tsp"];

/**
 * Resolve URL path to filesystem path
 * @param pathname URL path
 * @param root Document root directory
 * @param additionalExtensions Additional allowed extensions (e.g., static files)
 * @returns Resolution result
 */
export function resolvePath(
  pathname: string,
  root: string,
  additionalExtensions: string[] = [],
): PathResult {
  try {
    // Remove leading slash and decode
    const decoded = decodeURIComponent(pathname.slice(1));

    // If path is empty, return index.tsp
    if (!decoded) {
      return {
        success: true,
        filepath: join(root, "index.tsp"),
      };
    }

    // Merge all allowed extensions
    const allAllowedExtensions = [
      ...ALLOWED_EXTENSIONS,
      ...additionalExtensions,
    ];

    // If path ends with /, it's a directory, try to find index.tsp inside
    if (decoded.endsWith("/")) {
      const indexPath = join(root, decoded, "index.tsp");
      return {
        success: true,
        filepath: indexPath,
      };
    }

    // Check if path already has an allowed extension
    const hasExtension = allAllowedExtensions.some((ext) =>
      decoded.endsWith(ext)
    );

    if (hasExtension) {
      // Path already has extension, use directly
      return {
        success: true,
        filepath: join(root, decoded),
      };
    } else {
      // Add .tsp extension (default behavior)
      return {
        success: true,
        filepath: join(root, decoded + ".tsp"),
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
 * Try to find file (handles directory/index.tsp case)
 * @param basePath Base path
 * @returns File path or null
 */
async function findFile(basePath: string): Promise<string | null> {
  // Check if exists and is a file
  try {
    const stat = await Deno.stat(basePath);
    if (stat.isFile) {
      return basePath;
    }
  } catch {
    // File does not exist, continue trying other paths
  }

  // Try index.tsp in directory
  const indexPath = join(basePath, "index.tsp");
  try {
    const stat = await Deno.stat(indexPath);
    if (stat.isFile) {
      return indexPath;
    }
  } catch {
    // Does not exist
  }

  return null;
}

/**
 * Check if file extension is allowed
 * @param filepath File path
 * @param additionalExtensions Additional allowed extensions
 * @returns Whether allowed
 */
function checkExtension(
  filepath: string,
  additionalExtensions: string[] = [],
): boolean {
  const allAllowedExtensions = [...ALLOWED_EXTENSIONS, ...additionalExtensions];
  return allAllowedExtensions.some((ext) => filepath.endsWith(ext));
}

/**
 * Security check
 * @param filepath File path
 * @param root Document root directory
 * @param additionalExtensions Additional allowed extensions (e.g., static files)
 * @returns Security check result
 */
export async function securityCheck(
  filepath: string,
  root: string,
  additionalExtensions: string[] = [],
): Promise<SecurityResult> {
  try {
    // 1. Check file extension
    if (!checkExtension(filepath, additionalExtensions)) {
      return {
        success: false,
        error: "File type not allowed",
      };
    }

    // 2. Check if it's an internal file (starting with __) or cache file (.mjs)
    // Extract filename (without directory)
    const filename = filepath.split(/[/\\]/).pop() || "";
    if (filename.startsWith("__") || filename.endsWith(".mjs")) {
      return {
        success: false,
        error: "Internal file (not accessible via HTTP)",
      };
    }

    // 3. Normalize path and resolve to absolute path
    const normalizedFilepath = normalize(filepath);
    const normalizedRoot = normalize(root);
    const absoluteFilepath = resolve(normalizedFilepath);
    const absoluteRoot = resolve(normalizedRoot);

    // 4. Check if file exists
    let finalFilepath = absoluteFilepath;

    try {
      const stat = await Deno.stat(absoluteFilepath);

      // If path is a directory, try to find index.tsp
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
      // If it's a file, use directly
    } catch (statError) {
      // File or directory does not exist
      return {
        success: false,
        error: "File not found",
      };
    }

    // 5. Check for path traversal attack (ensure file is within root directory)
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
 * Calculate relative path
 * @param from From path
 * @param to To path
 * @returns Relative path
 */
function relativePath(from: string, to: string): string {
  const fromParts = from.split(/[/\\]/);
  const toParts = to.split(/[/\\]/);

  // Find common prefix
  let commonLength = 0;
  for (let i = 0; i < Math.min(fromParts.length, toParts.length); i++) {
    if (fromParts[i] === toParts[i]) {
      commonLength++;
    } else {
      break;
    }
  }

  // Build relative path
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
 * Check if path is outside root directory
 * @param relativePath Relative path
 * @returns Whether outside root directory
 */
function isAbsolutePathOutside(relativePath: string): boolean {
  return relativePath.startsWith("/") || /^[a-zA-Z]:/.test(relativePath);
}
