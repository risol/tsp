/**
 * 预编译模块
 * 将 TSX 文件编译为缓存目录中的 JS 文件
 */

import { dirname, join, relative, resolve, toFileUrl } from "std/path";
import { ensureDir } from "https://deno.land/std@0.224.0/fs/ensure_dir.ts";
import type { Logger } from "./logger.ts";

const WWW_DIR = "./www";
const CACHE_DIR = ".cache/tsp";

// 全局缓存基础目录 - 从 main.ts 设置
let cacheBaseDir: string | null = null;

/**
 * 设置缓存计算的基础目录
 * 应该传入 www 文件夹的父目录调用
 */
export function setCacheBaseDir(dir: string): void {
  cacheBaseDir = dir;
}

// 模块级 logger 实例（可通过 setPrecompilerLogger 设置）
let precompilerLogger: Logger | null = null;

/**
 * 设置预编译模块使用的 logger
 * @param logger Logger 实例
 */
export function setPrecompilerLogger(logger: Logger | null): void {
  precompilerLogger = logger;
}

/**
 * 获取预编译日志输出函数（如果没有 logger 则使用 console）
 */
function getLogOutput() {
  if (!precompilerLogger) {
    return {
      debug: console.log.bind(console),
      info: console.log.bind(console),
      warn: console.warn.bind(console),
      error: console.error.bind(console),
    };
  }
  return {
    debug: precompilerLogger.debug.bind(precompilerLogger),
    info: precompilerLogger.info.bind(precompilerLogger),
    warn: precompilerLogger.warn.bind(precompilerLogger),
    error: precompilerLogger.error.bind(precompilerLogger),
  };
}

/**
 * 获取缓存基础目录
 * 返回 .cache 应该位于的目录
 * 如果调用了 setCacheBaseDir，则使用该目录；否则使用 Deno.cwd()
 */
function getCacheBaseDir(): string {
  if (cacheBaseDir) {
    return cacheBaseDir;
  }
  // 默认为当前工作目录
  return Deno.cwd();
}

/** 获取相对于当前工作目录或 www 目录的文件路径 */
function getRelativePath(filepath: string): string {
  const cacheBase = getCacheBaseDir();

  // 🔧 修复：将相对路径转换为绝对路径
  // filepath 可能是相对路径（如 "./www/index.tsx"）或绝对路径
  let absPath = filepath;
  if (!filepath.startsWith("/") && !filepath.match(/^[a-zA-Z]:/)) {
    // 不是绝对路径，转换为相对于 cache base 的绝对路径
    absPath = resolve(cacheBase, filepath);
  }

  // 尝试相对于 www 目录计算路径
  const wwwPath = join(cacheBase, WWW_DIR);
  if (absPath.startsWith(wwwPath)) {
    return relative(wwwPath, absPath);
  }

  // 如果不在 www 目录下，相对于 cache base 计算路径
  return relative(cacheBase, absPath);
}

/**
 * 获取 cache 目录中的目标路径
 * @param filepath 文件路径
 * @param version 版本号（可选，用于生成版本化文件名）
 */
export function getCachePath(filepath: string, version?: number): string {
  const relativePath = getRelativePath(filepath);
  const jsPath = relativePath.replace(/\.tsx$/, ".js");

  const cacheBase = getCacheBaseDir();

  // 如果提供了版本号，在文件名中嵌入版本号
  if (version !== undefined) {
    const versionedPath = jsPath.replace(/\.js$/, `.v${version}.js`);
    return join(cacheBase, CACHE_DIR, versionedPath);
  }

  return join(cacheBase, CACHE_DIR, jsPath);
}

/**
 * 使用 @deno/loader 转译 TSX 为 JS
 * @param filepath 文件路径
 * @param version 版本号（用于缓存破坏，绕过 Deno import 缓存）
 */
async function transpileTSX(
  filepath: string,
  version?: number,
): Promise<string> {
  const { Workspace, RequestedModuleType } = await import("@deno/loader");

  // filepath 已经是绝对路径
  const fileUrl = toFileUrl(filepath).href;

  const workspace = new Workspace();
  const loader = await workspace.createLoader();

  // 添加入口点
  const diagnostics = await loader.addEntrypoints([fileUrl]);
  if (diagnostics.length > 0) {
    throw new Error(diagnostics[0].message);
  }

  // 加载模块
  const response = await loader.load(fileUrl, RequestedModuleType.Default);

  if (response.kind !== "module") {
    throw new Error(`Failed to load module: ${filepath}`);
  }

  // 将 Uint8Array 转换为字符串
  let code = new TextDecoder().decode(response.code);

  // ⭐ 移除 source map 注释，避免路径解析问题
  // source map 中的相对路径会导致 Deno 解析错误的文件位置
  code = code.replace(/\/\/# sourceMappingURL=.+$/gm, "");

  // 重写导入路径：将本地导入中的 .tsx 替换为 .js，并添加版本号
  // 注意：.ts 文件不需要编译，Deno 原生支持 TypeScript，所以保留 .ts 扩展名
  // 支持多种格式：
  // - import { X } from "./path.tsx"
  // - import { X } from "./path.tsx";
  // - export { X } from "./path.tsx"
  code = code.replace(
    /((?:import|export)\s+(?:(?:\*\s+as\s+\w+)|(?:\w+)|(?:\{[^}]*\}))\s+from\s+['"])((?:\.\/|\.\.\/)[^'"]+)\.tsx(['"][;\s]*)/g,
    (match, prefix, importPath, suffix) => {
      // 替换 .tsx 为 .js
      const jsPath = importPath + ".js";

      // ⭐ 如果提供了版本号，在文件名中嵌入版本号以绕过 Deno 的 import 缓存
      // 例如：./Component.js 变成 ./ Component.v2.js
      // 每次版本号变化时，文件名不同，Deno 会重新加载
      if (version !== undefined) {
        // 在扩展名前插入版本号
        const versionedPath = jsPath.replace(/\.js$/, `.v${version}.js`);
        return `${prefix}${versionedPath}${suffix}`;
      }

      return `${prefix}${jsPath}${suffix}`;
    },
  );

  return code;
}

/**
 * 检查 TSX 文件是否包含远程导入
 */
export async function checkRemoteImports(filepath: string): Promise<string[]> {
  const code = await Deno.readTextFile(filepath);

  // 匹配所有 import 语句
  const importRegex =
    /import\s+(?:\*\s+as\s+\w+|\w+|\{[^}]*\})\s+from\s+['"]([^'"]+)['"]|import\s+['"]([^'"]+)['"]/g;

  const remoteImports: string[] = [];
  let match;

  while ((match = importRegex.exec(code)) !== null) {
    const importPath = match[1] || match[2];

    // 检查是否是远程导入
    if (
      importPath && (importPath.startsWith("http://") ||
        importPath.startsWith("https://") ||
        importPath.startsWith("npm:") ||
        importPath.startsWith("jsr:"))
    ) {
      remoteImports.push(importPath);
    }
  }

  return remoteImports;
}

/**
 * 分析 TSX 文件的依赖
 */
export async function analyzeDependencies(filepath: string): Promise<string[]> {
  const code = await Deno.readTextFile(filepath);

  // 匹配所有本地导入（可能包含扩展名）
  const localImports: string[] = [];
  const importRegex =
    /import\s+(?:\*\s+as\s+\w+|\w+|\{[^}]*\})\s+from\s+['"]((?:\.\/|\.\.\/)[^'"]+)['"]/g;

  let match;
  while ((match = importRegex.exec(code)) !== null) {
    let importPath = match[1];

    // 移除可能的扩展名 (.tsx, .ts, .js)
    importPath = importPath.replace(/\.tsx?$/, "").replace(/\.js$/, "");

    localImports.push(importPath);
  }

  // 解析为绝对路径
  const fileDir = dirname(filepath);
  const dependencies: string[] = [];

  for (const importPath of localImports) {
    const absPath = join(fileDir, importPath);

    // 尝试 .tsx、.ts、.js 扩展名
    const extensions = [".tsx", ".ts", ".js"];
    let found = false;

    for (const ext of extensions) {
      const testPath = absPath + ext;
      try {
        const stat = await Deno.stat(testPath);
        if (stat.isFile) {
          dependencies.push(testPath);
          found = true;
          break;
        }
      } catch {
        // 文件不存在，继续尝试
      }
    }

    if (!found) {
      const log = getLogOutput();
      log.warn(`[WARN] Dependency not found: ${importPath} in ${filepath}`);
    }
  }

  return dependencies;
}

/**
 * 编译单个 TSX 文件
 * @param filepath 文件路径
 * @param version 版本号（用于缓存破坏，绕过 Deno import 缓存）
 */
export async function compileFile(
  filepath: string,
  version?: number,
): Promise<void> {
  const log = getLogOutput();

  // 检查远程导入
  const remoteImports = await checkRemoteImports(filepath);
  if (remoteImports.length > 0) {
    throw new Error(
      `Remote imports are not allowed in ${filepath}:\n  ${
        remoteImports.join("\n  ")
      }`,
    );
  }

  // 分析依赖
  const dependencies = await analyzeDependencies(filepath);

  // 转译为 JS（传递版本号）
  const jsCode = await transpileTSX(filepath, version);

  // 获取目标路径（传递版本号）
  const cachePath = getCachePath(filepath, version);

  // 确保目录存在
  await ensureDir(dirname(cachePath));

  // 写入编译后的文件
  await Deno.writeTextFile(cachePath, jsCode);

  // 复制 .ts 依赖文件到缓存目录（不编译，Deno 原生支持 TypeScript）
  for (const dep of dependencies) {
    if (dep.endsWith(".ts")) {
      const depCachePath = getCachePath(dep, version);
      await ensureDir(dirname(depCachePath));
      await Deno.copyFile(dep, depCachePath);
      // ⭐ 等待文件系统完成写入
      await new Promise((resolve) => setTimeout(resolve, 10));
      log.debug(`[COPIED] ${dep} -> ${relative(Deno.cwd(), depCachePath)}`);
    }
  }

  const versionSuffix = version !== undefined ? ` (v=${version})` : "";
  log.debug(
    `[COMPILED] ${filepath} -> ${
      relative(Deno.cwd(), cachePath)
    }${versionSuffix}`,
  );
}

/**
 * 编译所有 TSX 文件
 * @param rootDir - 根目录（相对于CWD，默认为 "./www"）
 * @returns 编译的文件列表（相对于根目录）
 */
export async function compileAll(rootDir: string = WWW_DIR): Promise<string[]> {
  const log = getLogOutput();
  log.info("🚀 Starting TSX compilation...\n");

  // 确保 cache 目录存在
  await ensureDir(join(Deno.cwd(), CACHE_DIR));

  // 递归查找所有 TSX 文件
  const tsxFiles: string[] = [];

  async function findTSXFiles(dir: string) {
    const entries = Deno.readDir(dir);

    for await (const entry of entries) {
      const path = join(dir, entry.name);

      if (entry.isDirectory && !entry.name.startsWith(".")) {
        await findTSXFiles(path);
      } else if (entry.isFile && entry.name.endsWith(".tsx")) {
        tsxFiles.push(path);
      }
    }
  }

  await findTSXFiles(join(Deno.cwd(), rootDir));

  log.info(`[INFO] Found ${tsxFiles.length} TSX files`);

  // 编译所有文件
  for (const filepath of tsxFiles) {
    try {
      await compileFile(filepath);
    } catch (error) {
      const err = error as Error;
      log.error(`[ERROR] Failed to compile ${filepath}:`, err.message);
      throw error;
    }
  }

  log.info(`\n✅ Compiled ${tsxFiles.length} files to cache/`);

  // 返回编译的文件列表（转换为相对于根目录的路径）
  return tsxFiles.map((f) => relative(Deno.cwd(), f));
}

/**
 * 清理 cache 目录
 */
export async function cleanCache(): Promise<void> {
  const log = getLogOutput();
  try {
    await Deno.remove(join(Deno.cwd(), CACHE_DIR), { recursive: true });
    log.debug(`[CLEAN] Removed cache/ directory`);
  } catch (error) {
    if (!(error instanceof Deno.errors.NotFound)) {
      throw error;
    }
  }
}
