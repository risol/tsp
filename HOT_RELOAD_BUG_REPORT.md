# 热重载功能失效问题报告

## 问题描述

当修改组件文件（如 `components/Navigation.tsx`）时，使用该组件的页面（如 `index.tsx`）不会自动更新，需要重启服务器才能看到变化。

## 问题复现

1. 启动开发服务器：`deno task dev`
2. 访问 `http://localhost:9000/` - 显示正常内容
3. 修改 `www/components/Navigation.tsx`
4. 刷新页面 - **内容没有更新**
5. 必须重启服务器才能看到变化

## 根本原因分析

### 缓存机制问题

当前 `src/cache.ts` 的缓存逻辑：

```typescript
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

  // 检查依赖文件是否修改
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
        return true;
      }
    } catch (error) {
      // 文件不存在，可能被删除了
      console.log(`[WARN] Dependency not found: ${dep}`);
      return true;
    }
  }

  return false; // ❌ 问题：即使依赖修改了，也返回 false！
}
```

### 问题详解

**问题 1：主文件的缓存没有被清除**
- 当 `Navigation.tsx` 修改时
- `index.tsx` 的 mtime 没有变化
- `index.tsx` 的缓存被认为是有效的
- `index.tsx` 不会重新加载 `Navigation` 模块

**问题 2：Deno 的模块缓存**
- 即使使用了 `?t=${currentMtime}` 来绕过缓存
- 这只影响**主文件**
- **依赖模块**仍然使用旧的缓存版本

**问题 3：编译后的文件没有被重新编译**
- `Navigation.tsx` 被修改了
- 但 `.cache/tsp/www/components/Navigation.tsx.js` 没有被更新
- 因为 `index.tsx` 没有触发重新编译链

## 测试验证

### 测试 1：修改主页面文件
```bash
# 修改 www/index.tsx
# 刷新页面
# 结果：✅ 正常更新
```

### 测试 2：修改组件文件
```bash
# 修改 www/components/Navigation.tsx
# 刷新页面
# 结果：❌ 不更新（BUG）
```

### 测试 3：Header.tsx 为什么"不工作"？
- `Header.tsx` 组件存在
- 但**没有任何页面导入它**
- 所以修改它不会影响任何页面
- 这是**预期行为**，不是 bug

## 影响范围

- ❌ 开发模式下组件修改不生效
- ❌ 工具函数修改不生效
- ❌ 需要频繁重启服务器
- ✅ 主页面文件修改正常工作

## 建议修复方案

### 方案 1：基于依赖图的缓存失效（推荐）

修改 `needsRecompilation` 函数：

```typescript
async function needsRecompilation(
  filepath: string,
  currentMtime: number
): Promise<boolean> {
  const cached = moduleCache.get(filepath);

  if (!cached) {
    return true;
  }

  // 检查主文件是否修改
  const compiledMtime = compiledMtimes.get(filepath);
  if (!compiledMtime || compiledMtime !== currentMtime) {
    return true;
  }

  // ⭐ 修复：检查所有依赖文件
  const dependencies = cached.dependencies || [];
  for (const dep of dependencies) {
    try {
      const depStat = await Deno.stat(dep);
      const depMtime = depStat.mtime?.getTime() || 0;

      // ⭐ 获取依赖文件的编译时间
      const depCompiledMtime = compiledMtimes.get(dep);

      // ⭐ 如果依赖文件在编译后被修改了
      if (!depCompiledMtime || depMtime > depCompiledMtime) {
        console.log(`[INFO] Dependency ${dep} modified after compilation`);
        console.log(`[INFO] Triggering recompilation of ${filepath}`);

        // ⭐ 清除主文件的缓存，强制重新编译
        moduleCache.delete(filepath);
        compiledMtimes.delete(filepath);

        return true;
      }
    } catch (error) {
      console.log(`[WARN] Dependency not found: ${dep}`);
      return true;
    }
  }

  return false;
}
```

### 方案 2：全局依赖追踪

维护一个反向依赖图：

```typescript
// 反向依赖图：记录每个文件被哪些文件导入
const reverseDeps = new Map<string, Set<string>>();

// 当编译文件时记录反向依赖
function trackDependencies(filepath: string, dependencies: string[]) {
  for (const dep of dependencies) {
    if (!reverseDeps.has(dep)) {
      reverseDeps.set(dep, new Set());
    }
    reverseDeps.get(dep)!.add(filepath);
  }
}

// 当文件修改时，通知所有依赖它的文件
function invalidateDependents(filepath: string) {
  const dependents = reverseDeps.get(filepath);
  if (dependents) {
    for (const dependent of dependents) {
      moduleCache.delete(dependent);
      compiledMtimes.delete(dependent);
      console.log(`[INFO] Invalidating cache for ${dependent}`);
    }
  }
}
```

### 方案 3：开发模式禁用缓存（最简单）

在开发模式下完全禁用缓存：

```typescript
// 在 main.ts 中
if (config.dev) {
  clearCache(); // 每次请求都清除缓存
}
```

## E2E 测试问题

当前 `tests/hot_reload_test.ts` 使用端口 9600，但该测试可能：
1. 没有真正测试组件修改的场景
2. 测试的是主文件修改，不是依赖修改
3. 测试可能通过了，但实际场景失败

## 推荐行动

1. ✅ **立即修复**：实施方案 1（基于依赖图的缓存失效）
2. ✅ **增强测试**：更新 `hot_reload_test.ts` 包含组件修改测试
3. ✅ **文档更新**：记录热重载的工作原理和限制
4. ✅ **用户反馈**：在文档中说明当前热重载的限制

## 临时解决方案

在热重载修复之前，用户可以：
1. 重启服务器来查看组件修改
2. 直接修改页面文件而不是组件
3. 使用 `--no-cache` 模式（如果实现）

## 相关文件

- `src/cache.ts` - 缓存和热重载逻辑
- `src/precompiler_lib.ts` - 依赖分析
- `tests/hot_reload_test.ts` - 热重载测试
- `docs/HOT_RELOAD.md` - 热重载文档（需创建）

---

**报告日期**: 2026-01-29
**严重程度**: 高（影响开发体验）
**优先级**: P0（应立即修复）
