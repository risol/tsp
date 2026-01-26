/**
 * 模板缓存模块
 * 实现基于文件修改时间的模板编译缓存
 */

import { Eta } from "https://deno.land/x/eta@v3.2.0/src/index.ts";

// 缓存条目类型
interface CacheEntry {
  mtimeMs: number;
  template: string;
}

// 模板缓存 Map
const templateCache = new Map<string, CacheEntry>();

/**
 * 获取模板渲染函数
 * 如果缓存有效则使用缓存，否则重新编译
 * @param filepath 模板文件路径
 * @param eta Eta 实例
 * @returns 渲染函数
 */
export async function getTemplate(
  filepath: string,
  eta: Eta
): Promise<(data: unknown) => Promise<string>> {
  // 获取文件修改时间
  const stat = await Deno.stat(filepath);
  const currentMtimeMs = stat.mtime?.getTime() || 0;

  // 检查缓存
  const cached = templateCache.get(filepath);
  let templateContent: string;

  if (cached && cached.mtimeMs === currentMtimeMs) {
    // 缓存有效，使用缓存的模板内容
    templateContent = cached.template;
  } else {
    // 缓存无效或不存在，读取文件
    templateContent = await Deno.readTextFile(filepath);

    // 更新缓存
    templateCache.set(filepath, {
      mtimeMs: currentMtimeMs,
      template: templateContent,
    });
  }

  // 编译模板并返回渲染函数
  const templateFn = eta.compile(templateContent);

  return (data: unknown): Promise<string> => {
    const result = templateFn.call(eta, data as Record<string, unknown>);
    return Promise.resolve(result as string);
  };
}

/**
 * 清除所有模板缓存
 */
export function clearCache(): void {
  templateCache.clear();
}

/**
 * 获取缓存统计信息
 * @returns 缓存大小
 */
export function getCacheSize(): number {
  return templateCache.size;
}
