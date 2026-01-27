/**
 * 模块缓存模块
 * 实现基于文件修改时间的 TSX 模块缓存
 */

import { join } from "jsr:@std/path@1.0.0";

// 上下文类型（从 context.ts 导入）
export interface TemplateContext {
  method: string;
  url: URL;
  headers: Headers;
  query: Record<string, string>;
  body: unknown;
  cookies: Record<string, string>;
  file: string;
  root: string;
}

// 模块函数类型
export type ModuleFunction = (
  context: TemplateContext
) => Promise<string> | string;

// 缓存条目类型
interface CacheEntry {
  mtimeMs: number;
  module: ModuleFunction;
}

// 模块缓存 Map
const moduleCache = new Map<string, CacheEntry>();

/**
 * 获取模块函数
 * 如果缓存有效则使用缓存，否则重新加载
 * @param filepath 模块文件路径
 * @returns 模块函数
 */
export async function getTemplate(
  filepath: string
): Promise<ModuleFunction> {
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
    module: module.default as ModuleFunction,
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
