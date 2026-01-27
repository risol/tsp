/**
 * 模块缓存模块
 * 实现基于文件修改时间的 TSX 模块缓存
 */

import { join } from "std/path";
import { render } from "preact-render-to-string";
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
    return cached.module;
  }

  // 缓存无效或不存在，动态导入模块
  const moduleUrl = `file://${join(Deno.cwd(), filepath)}`;
  const module = await import(moduleUrl);

  // 检查模块是否导出默认函数
  if (typeof module.default !== 'function') {
    throw new Error(`Module ${filepath} must export a default function`);
  }

  // 更新缓存
  moduleCache.set(filepath, {
    mtimeMs: currentMtimeMs,
    module: module.default as PageFunction,
  });

  return module.default as ModuleFunction;
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
  const html = render(jsx);
  // 添加 DOCTYPE 声明
  return "<!DOCTYPE html>\n" + html;
}
