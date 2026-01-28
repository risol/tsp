/**
 * 模块缓存模块
 * 实现基于文件修改时间的 TSX 模块缓存
 */

import { join, toFileUrl } from "std/path";
import { render } from "preact-render-to-string";
import {
  Workspace,
  type LoadResponse,
  RequestedModuleType,
} from "@deno/loader";
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
}

// 模块缓存 Map
const moduleCache = new Map<string, CacheEntry>();

// 动态导入缓存 - 用于清除模块缓存
const importCache = new Map<string, string>();

// 创建全局 loader 实例
let globalLoader: Awaited<ReturnType<typeof Workspace.prototype.createLoader>> | null = null;
let globalWorkspace: Workspace | null = null;

async function getGlobalLoader() {
  if (!globalLoader || !globalWorkspace) {
    globalWorkspace = new Workspace();
    globalLoader = await globalWorkspace.createLoader();
  }
  return globalLoader;
}

// 清除 loader 缓存并重建
function clearGlobalLoader() {
  globalLoader = null;
  globalWorkspace = null;
}

/**
 * 获取页面函数
 * 如果缓存有效则使用缓存，否则重新加载
 * @param filepath 页面文件路径
 * @returns 页面函数
 */
export async function getPage(
  filepath: string
): Promise<PageFunction> {
  // 获取文件修改时间
  const stat = await Deno.stat(filepath);
  const currentMtimeMs = stat.mtime?.getTime() || 0;

  // 检查缓存
  const cached = moduleCache.get(filepath);

  if (cached && cached.mtimeMs === currentMtimeMs) {
    // 缓存有效，使用缓存的模块
    console.log(`[CACHE HIT] ${filepath} (mtime: ${currentMtimeMs})`);
    return cached.module;
  }

  // 缓存失效，重新加载
  console.log(`[CACHE MISS] ${filepath} (old: ${cached?.mtimeMs || 'none'}, new: ${currentMtimeMs})`);

  // 将相对路径转换为绝对路径，然后转为 file URL
  const absolutePath = join(Deno.cwd(), filepath);
  const fileUrl = toFileUrl(absolutePath).href;

  // 清除 loader 缓存，确保获取最新代码
  clearGlobalLoader();

  // 先尝试直接 import（适用于 deno run 模式）
  try {
    const module = await import(fileUrl);

    // 检查模块是否导出默认函数
    if (typeof module.default !== 'function') {
      throw new Error(`Module ${filepath} must export a default function`);
    }

    // 更新缓存
    moduleCache.set(filepath, {
      mtimeMs: currentMtimeMs,
      module: module.default as PageFunction,
    });

    return module.default as PageFunction;
  } catch (importError) {
    // 如果直接 import 失败，使用 loader（适用于编译后的二进制文件）
    console.log(`[FALLBACK] Using @deno/loader for ${filepath}`);

    const loader = await getGlobalLoader();

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
    const code = new TextDecoder().decode(response.code);

    // 使用 data URL 导入转译后的代码
    const dataUrl = `data:application/javascript,${encodeURIComponent(code)}`;
    const module = await import(dataUrl);

    // 检查模块是否导出默认函数
    if (typeof module.default !== 'function') {
      throw new Error(`Module ${filepath} must export a default function`);
    }

    // 更新缓存
    moduleCache.set(filepath, {
      mtimeMs: currentMtimeMs,
      module: module.default as PageFunction,
    });

    return module.default as PageFunction;
  }
}

/**
 * 清除所有模块缓存
 */
export function clearCache(): void {
  moduleCache.clear();
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
