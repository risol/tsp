/**
 * 模块缓存模块
 * 实现基于预编译的模块缓存
 * TSX 文件会被编译到 .cache/tsp/ 目录，getPage() 加载编译后的 JS 文件
 */

import { join, toFileUrl } from "std/path";
import { render } from "preact-render-to-string";
import { ensureDir } from "https://deno.land/std@0.224.0/fs/ensure_dir.ts";
import { compileFile, analyzeDependencies, getCachePath } from "./precompiler_lib.ts";
import type { PageContext } from "./context.ts";

// 重新导出 PageContext 类型，供页面使用
export type { PageContext };

// JSX 返回类型
export type JSXResult = unknown;

/**
 * 页面返回值类型
 * 可以是 JSX 元素、重定向对象或 Response 对象
 */
export type PageResult = unknown;

/**
 * 重定向结果
 * 页面返回此对象将触发 HTTP 重定向
 */
export interface RedirectResult {
  /** 重定向的目标 URL */
  redirect: string;
  /** 重定向状态码，默认 302 */
  status?: 301 | 302 | 303 | 307 | 308;
}

/**
 * 页面函数类型
 * 每个 TSX 页面应该导出符合此类型的默认函数
 * 返回值可以是：
 * - JSX 元素：渲染为 HTML
 * - RedirectResult 对象：触发重定向
 * - Response 对象：直接返回
 */
export type PageFunction = (
  context: PageContext
) => Promise<PageResult> | PageResult;

// 缓存条目类型
interface CacheEntry {
  mtimeMs: number;
  module: PageFunction;
  dependencies: string[]; // 依赖的文件列表
}

// 模块缓存 Map
const moduleCache = new Map<string, CacheEntry>();

// 依赖图：记录每个 TSX 文件及其依赖的文件
const dependencyGraph = new Map<string, string[]>();

// ⭐ 反向依赖图：记录每个文件被哪些文件依赖
const reverseDeps = new Map<string, Set<string>>();

// 编译文件修改时间
const compiledMtimes = new Map<string, number>();

/**
 * 获取页面函数
 * 使用预编译的 JS 文件，支持自动重新编译
 * @param filepath 页面文件路径（相对于 www 目录）
 * @returns 页面函数
 */
export async function getPage(
  filepath: string
): Promise<PageFunction> {
  const absPath = join(Deno.cwd(), filepath);

  // 获取文件修改时间
  const stat = await Deno.stat(absPath);
  const currentMtime = stat.mtime?.getTime() || 0;

  // 检查缓存
  const cached = moduleCache.get(filepath);

  // 构建依赖关系并检查是否需要重新编译
  const needsRecompile = await needsRecompilation(filepath, currentMtime);

  if (cached && !needsRecompile) {
    // 缓存有效，使用缓存的模块
    console.log(`[CACHE HIT] ${filepath} (mtime: ${currentMtime})`);
    return cached.module;
  }

  // 需要重新编译
  console.log(`[CACHE MISS] ${filepath} - recompiling...`);

  // 1. 检查远程导入
  const { checkRemoteImports } = await import("./precompiler_lib.ts");
  const remoteImports = await checkRemoteImports(absPath);
  if (remoteImports.length > 0) {
    throw new Error(
      `Remote imports are not allowed: ${remoteImports.join(", ")}`
    );
  }

  // 2. 分析依赖
  const { analyzeDependencies } = await import("./precompiler_lib.ts");
  const dependencies = await analyzeDependencies(absPath);
  console.log(`[INFO] Dependencies: ${dependencies.join(", ")}`);

  // 3. 编译当前文件
  const { compileFile } = await import("./precompiler_lib.ts");
  await compileFile(absPath);

  // 4. 编译所有依赖的 TSX 文件
  for (const dep of dependencies) {
    if (dep.endsWith(".tsx")) {
      console.log(`[INFO] Compiling dependency: ${dep}`);
      await compileFile(dep);

      // 更新依赖文件的编译时间戳
      const depStat = await Deno.stat(dep);
      const depMtime = depStat.mtime?.getTime() || 0;
      compiledMtimes.set(dep, depMtime);
    }
  }

  // 5. 更新依赖图
  dependencyGraph.set(filepath, dependencies);

  // ⭐ 记录反向依赖关系
  trackReverseDependencies(filepath, dependencies);

  // 6. 加载编译后的 JS 文件
  const { getCachePath } = await import("./precompiler_lib.ts");
  const cachePath = getCachePath(filepath);
  const cacheUrl = toFileUrl(cachePath).href;

  // 添加时间戳查询参数，强制 Deno 重新加载模块（解决二进制模式下 import() 缓存问题）
  const moduleUrl = `${cacheUrl}?t=${currentMtime}`;

  // 加载模块
  const module = await import(moduleUrl);

  // 检查模块是否导出默认函数
  if (typeof module.default !== 'function') {
    throw new Error(`Module ${filepath} must export a default function`);
  }

  // 7. 更新缓存
  moduleCache.set(filepath, {
    mtimeMs: currentMtime,
    module: module.default as PageFunction,
    dependencies: dependencies,
  });

  // 8. 更新编译文件修改时间
  compiledMtimes.set(filepath, currentMtime);

  return module.default as PageFunction;
}

/**
 * 检查是否需要重新编译
 */
async function needsRecompilation(
  filepath: string,
  currentMtime: number
): Promise<boolean> {
  const cached = moduleCache.get(filepath);

  if (!cached) {
    return true; // 没有缓存，需要编译
  }

  // 检查主文件是否修改
  const compiledMtime = compiledMtimes.get(filepath);
  if (!compiledMtime || compiledMtime !== currentMtime) {
    return true;
  }

  // ⭐ 检查依赖文件是否修改，并批量失效
  const dependencies = cached.dependencies || [];
  for (const dep of dependencies) {
    try {
      const depStat = await Deno.stat(dep);
      const depMtime = depStat.mtime?.getTime() || 0;

      // 获取这个依赖文件的编译时间
      const depCompiledMtime = compiledMtimes.get(dep);

      // 如果依赖文件修改了，或者还没有编译过
      if (!depCompiledMtime || depMtime > depCompiledMtime) {
        console.log(`[INFO] Dependency modified: ${dep}`);

        // ⭐ 主动通知所有依赖此文件的缓存失效
        const invalidated = invalidateDependents(dep);

        console.log(`[INFO] Batch invalidated ${invalidated.length} file(s) due to ${dep} change`);

        // 返回 true，当前文件需要重新编译
        return true;
      }
    } catch (error) {
      // 文件不存在，可能被删除了
      console.log(`[WARN] Dependency not found: ${dep}`);
      return true;
    }
  }

  return false;
}

/**
 * 清除所有模块缓存
 */
export function clearCache(): void {
  moduleCache.clear();
  dependencyGraph.clear();
  reverseDeps.clear();  // ⭐ 清除反向依赖图
  compiledMtimes.clear();
}

/**
 * ⭐ 记录反向依赖关系
 * @param filepath 当前文件路径
 * @param dependencies 当前文件的依赖列表
 */
function trackReverseDependencies(filepath: string, dependencies: string[]): void {
  for (const dep of dependencies) {
    if (!reverseDeps.has(dep)) {
      reverseDeps.set(dep, new Set());
    }
    reverseDeps.get(dep)!.add(filepath);

    console.log(`[REVERSE_DEPS] ${dep} → [${Array.from(reverseDeps.get(dep)!).join(", ")}]`);
  }
}

/**
 * ⭐ 使依赖某个文件的所有文件失效
 * @param dependencyFile 被修改的文件路径
 * @returns 被失效的文件列表
 */
export function invalidateDependents(dependencyFile: string): string[] {
  const dependents = reverseDeps.get(dependencyFile);

  if (!dependents || dependents.size === 0) {
    console.log(`[REVERSE_DEPS] No files depend on ${dependencyFile}`);
    return [];
  }

  console.log(`[REVERSE_DEPS] File modified: ${dependencyFile}`);
  console.log(`[REVERSE_DEPS] Invalidating ${dependents.size} dependent(s):`, Array.from(dependents));

  const invalidated: string[] = [];

  for (const dependent of dependents) {
    // 清除缓存
    moduleCache.delete(dependent);
    compiledMtimes.delete(dependent);
    invalidated.push(dependent);

    console.log(`[CACHE] Invalidated: ${dependent}`);
  }

  return invalidated;
}

/**
 * 获取缓存统计信息
 * @returns 缓存大小
 */
export function getCacheSize(): number {
  return moduleCache.size;
}

/**
 * 渲染 JSX 到字符串
 * @param jsx JSX 元素
 * @returns HTML 字符串
 */
export function renderJSX(jsx: JSXResult): string {
  const html = render(jsx as unknown as Parameters<typeof render>[0]);
  // 添加 DOCTYPE 声明
  return "<!DOCTYPE html>\n" + html;
}
