# 热重载修复方案对比分析

## 方案对比总览

| 维度 | 方案 1：依赖图缓存失效 | 方案 2：反向依赖追踪 |
|------|----------------------|-------------------|
| **复杂度** | 低 - 简单修改现有逻辑 | 中 - 需要新数据结构 |
| **性能** | 中 - 每次请求都检查依赖 | 高 - 精确失效，无冗余检查 |
| **维护性** | 中 - 逻辑复杂度高 | 高 - 清晰的依赖关系 |
| **可靠性** | 中 - 可能遗漏边界情况 | 高 - 完整追踪依赖链 |
| **实现成本** | 低 - 修改 1 个函数 | 中 - 修改多个函数 |
| **测试难度** | 中 - 需测试多种场景 | 低 - 行为可预测 |

---

## 方案 1：依赖图缓存失效

### 工作原理

在 `needsRecompilation()` 函数中，当检测到依赖文件修改时，**主动清除父文件的缓存**：

```typescript
async function needsRecompilation(
  filepath: string,
  currentMtime: number
): Promise<boolean> {
  const cached = moduleCache.get(filepath);

  // ... 省略其他检查

  // 检查依赖文件是否修改
  const dependencies = cached.dependencies || [];
  for (const dep of dependencies) {
    const depStat = await Deno.stat(dep);
    const depMtime = depStat.mtime?.getTime() || 0;
    const depCompiledMtime = compiledMtimes.get(dep);

    // ⭐ 关键修改：依赖修改时，清除父文件缓存
    if (!depCompiledMtime || depMtime > depCompiledMtime) {
      console.log(`[INFO] Dependency ${dep} modified after compilation`);
      console.log(`[INFO] Triggering recompilation of ${filepath}`);

      // 🔥 清除父文件的缓存，强制重新编译
      moduleCache.delete(filepath);
      compiledMtimes.delete(filepath);

      return true; // 触发重新编译
    }
  }

  return false;
}
```

### 优点

1. **实现简单** - 只需修改一个函数
2. **立即可用** - 利用现有的依赖列表
3. **无额外数据结构** - 不增加内存开销
4. **向后兼容** - 不改变现有架构

### 缺点

1. **被动触发** - 只有在访问父文件时才发现依赖修改
2. **重复检查** - 每次请求都要遍历所有依赖
3. **不够精确** - 可能导致不必要的重新编译

### 性能分析

```
请求 index.tsx 时：
1. 检查 index.tsx 的 mtime ✓ (快速)
2. 遍历所有依赖（Layout, Navigation, Footer, Styles）
3. 检查每个依赖的 mtime
4. 发现 Navigation.tsx 修改
5. 清除 index.tsx 缓存，重新编译

问题：即使没有修改，每次请求都要检查所有依赖
```

### 实现代码

```typescript
// src/cache.ts

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

  // ⭐ 检查依赖文件是否修改
  const dependencies = cached.dependencies || [];
  for (const dep of dependencies) {
    try {
      const depStat = await Deno.stat(dep);
      const depMtime = depStat.mtime?.getTime() || 0;
      const depCompiledMtime = compiledMtimes.get(dep);

      // ⭐ 关键：依赖修改时，清除父文件缓存
      if (!depCompiledMtime || depMtime > depCompiledMtime) {
        console.log(`[INFO] Dependency ${dep} modified, invalidating ${filepath}`);

        // 🔥 清除父文件缓存
        moduleCache.delete(filepath);
        compiledMtimes.delete(filepath);

        return true;
      }
    } catch (error) {
      console.log(`[WARN] Dependency ${dep} not found`);
      return true;
    }
  }

  return false;
}
```

---

## 方案 2：反向依赖追踪（反向依赖图）

### 工作原理

维护一个**反向依赖图**，记录每个文件被哪些文件导入。当文件修改时，**主动通知**所有依赖它的文件：

```typescript
// ⭐ 新增：反向依赖图
const reverseDeps = new Map<string, Set<string>>();

// 1. 编译文件时，记录反向依赖
function trackDependencies(filepath: string, dependencies: string[]) {
  for (const dep of dependencies) {
    if (!reverseDeps.has(dep)) {
      reverseDeps.set(dep, new Set());
    }
    reverseDeps.get(dep)!.add(filepath);
  }
}

// 2. 文件修改时，主动通知所有依赖者
function invalidateDependents(dependencyFile: string) {
  const dependents = reverseDeps.get(dependencyFile);
  if (dependents) {
    for (const dependent of dependents) {
      console.log(`[INFO] Invalidating ${dependent} (depends on ${dependencyFile})`);

      // 清除缓存
      moduleCache.delete(dependent);
      compiledMtimes.delete(dependent);
    }
  }
}

// 3. 在 getPage() 中使用
export async function getPage(filepath: string): Promise<PageFunction> {
  const absPath = join(Deno.cwd(), filepath);
  const stat = await Deno.stat(absPath);
  const currentMtime = stat.mtime?.getTime() || 0;

  // ⭐ 检查该文件是否被其他文件依赖
  // 如果该文件是依赖（被导入），通知所有依赖者
  if (isDependencyFile(filepath)) {
    invalidateDependents(filepath);
  }

  // ... 继续原有的逻辑
}
```

### 优点

1. **主动失效** - 修改文件后立即通知所有依赖者
2. **精确失效** - 只清除真正受影响的缓存
3. **无需遍历** - 不需要在每次请求时检查所有依赖
4. **可扩展** - 容易添加更多依赖管理功能

### 缺点

1. **实现复杂** - 需要维护额外的数据结构
2. **内存开销** - 需要存储反向依赖图
3. **更多代码** - 需要多个新函数
4. **一致性** - 反向依赖图必须保持同步

### 性能分析

```
修改 Navigation.tsx 时：
1. 检测到 Navigation.tsx 修改
2. 查询反向依赖图：reverseDeps.get("Navigation.tsx")
3. 立即得到所有依赖者：["index.tsx", "features.tsx", "form.tsx"]
4. 批量清除这些文件的缓存
5. 这些文件下次被访问时会重新编译

优势：
- 无需等待请求触发
- 一次性清除所有相关缓存
- 下次访问这些页面时自动使用新组件
```

### 实现代码

```typescript
// src/cache.ts

// ⭐ 新增：反向依赖图
const reverseDeps = new Map<string, Set<string>>();

/**
 * 记录反向依赖关系
 * @param filepath 当前文件
 * @param dependencies 当前文件的依赖列表
 */
function trackReverseDependencies(filepath: string, dependencies: string[]) {
  for (const dep of dependencies) {
    if (!reverseDeps.has(dep)) {
      reverseDeps.set(dep, new Set());
    }
    reverseDeps.get(dep)!.add(filepath);

    console.log(`[DEBUG] ${filepath} depends on ${dep}`);
  }
}

/**
 * 使依赖某个文件的所有文件失效
 * @param dependencyFile 被修改的文件
 */
function invalidateDependents(dependencyFile: string) {
  const dependents = reverseDeps.get(dependencyFile);

  if (!dependents || dependents.size === 0) {
    return; // 没有文件依赖它
  }

  console.log(`[INFO] File ${dependencyFile} was modified`);
  console.log(`[INFO] Invalidating ${dependents.size} dependent(s):`,
    Array.from(dependents));

  for (const dependent of dependents) {
    // 清除缓存
    moduleCache.delete(dependent);
    compiledMtimes.delete(dependent);

    console.log(`[CACHE] Invalidated: ${dependent}`);
  }
}

/**
 * 检查文件是否是依赖（被其他文件导入）
 */
function isDependencyFile(filepath: string): boolean {
  return reverseDeps.has(filepath);
}

// ⭐ 修改 getPage() 函数
export async function getPage(filepath: string): Promise<PageFunction> {
  const absPath = join(Deno.cwd(), filepath);
  const stat = await Deno.stat(absPath);
  const currentMtime = stat.mtime?.getTime() || 0;

  // ⭐ 如果这个文件被其他文件依赖，通知它们失效
  if (isDependencyFile(filepath)) {
    invalidateDependents(filepath);
  }

  // ... 继续原有的缓存检查逻辑
}

// ⭐ 修改编译部分，记录反向依赖
async function compileAndCache(filepath: string, currentMtime: number) {
  // ... 编译逻辑

  // ⭐ 记录反向依赖
  trackReverseDependencies(filepath, dependencies);

  // 更新缓存
  moduleCache.set(filepath, {
    mtimeMs: currentMtime,
    module: module.default,
    dependencies: dependencies,
  });
}
```

---

## 关键区别对比

### 1. 触发时机

| 方案 1 | 方案 2 |
|--------|--------|
| **被动触发**<br>访问父文件时才发现依赖修改 | **主动触发**<br>依赖修改时立即通知父文件 |
| 时机：检查依赖时 | 时机：编译/导入依赖时 |

### 2. 缓存清除策略

```
方案 1（被动）:
访问 A.tsx → 检查依赖 → 发现 B.tsx 修改 → 清除 A.tsx 缓存

方案 2（主动）:
修改 B.tsx → 立即清除 A.tsx, C.tsx, D.tsx 缓存
（它们都依赖 B.tsx）
```

### 3. 数据流

#### 方案 1：单向依赖图
```
现有架构（正向依赖）:
index.tsx → [Layout.tsx, Navigation.tsx, Footer.tsx]
检查方向: 从父到子
```

#### 方案 2：双向依赖图
```
新增架构（反向依赖）:
Navigation.tsx → [index.tsx, features.tsx, form.tsx]
反向图: 从子到父
```

### 4. 内存占用

| 方案 1 | 方案 2 |
|--------|--------|
| 无额外内存 | 反向依赖图 Map |
| 复杂度：O(1) | 复杂度：O(N×M) <br> N=文件数, M=平均依赖数 |

### 5. 时间复杂度

| 操作 | 方案 1 | 方案 2 |
|------|--------|--------|
| 编译时 | O(1) | O(M) - 记录反向依赖 |
| 检查时 | O(M) - 遍历所有依赖 | O(1) - 直接查询 |
| 文件修改 | O(1) - 等待下次请求 | O(D×M) - 立即通知 D 个依赖者 |

---

## 使用场景示例

### 场景：修改 Navigation.tsx

#### 方案 1 执行流程
```
1. 用户修改 Navigation.tsx
2. 用户刷新浏览器，访问 index.tsx
3. getPage("index.tsx") 被调用
4. needsRecompilation() 检查依赖
5. 发现 Navigation.tsx 修改
6. 清除 index.tsx 缓存
7. 重新编译 index.tsx
8. 返回新页面

延迟：1 个请求周期
```

#### 方案 2 执行流程
```
1. 用户修改 Navigation.tsx
2. 下一次任何一个请求访问 index.tsx 时:
   - getPage("index.tsx") 被调用
   - 检查 Navigation.tsx 是否修改
   - 如果是，立即清除 index.tsx 缓存
   - 重新编译 index.tsx

或者更理想：在开发模式下监听文件系统事件
3. 立即清除所有依赖者的缓存
4. 后续请求直接使用新内容

延迟：几乎实时（如果有文件监听）
```

---

## 推荐选择

### 选择方案 1 的理由（推荐）
- ✅ **实现简单** - 10 行代码修改
- ✅ **低风险** - 不改变核心架构
- ✅ **向后兼容** - 与现有逻辑一致
- ✅ **立即可用** - 无需重构
- ⚠️ **性能可接受** - 检查依赖的开销很小

### 选择方案 2 的理由（长期）
- ✅ **扩展性强** - 为高级功能打基础
- ✅ **精确失效** - 无冗余重新编译
- ✅ **性能更好** - 长期看更优
- ❌ **实现复杂** - 需要大量代码
- ❌ **维护成本** - 需要保持反向依赖图同步

---

## 混合方案（最佳）

```typescript
// 阶段 1：实施方案 1（立即修复）
// 在 needsRecompilation() 中清除父文件缓存

// 阶段 2：实施方案 2（长期优化）
// 增加反向依赖图，提升性能
```

### 实现步骤

**第一步**：方案 1（立即）
- 修改 `needsRecompilation()` 函数
- 测试验证
- 部署到生产

**第二步**：方案 2（优化）
- 添加反向依赖图
- 在开发模式启用
- 逐步迁移

**第三步**：高级功能
- 文件系统监听（开发模式）
- WebSocket 通知客户端
- 增量编译

---

## 总结

| 维度 | 方案 1 | 方案 2 | 推荐 |
|------|--------|--------|------|
| **快速修复** | ✅ 10 分钟 | ❌ 2-3 小时 | 方案 1 |
| **长期维护** | ⚠️ 可接受 | ✅ 优秀 | 方案 2 |
| **性能** | ⚠️ 中等 | ✅ 优秀 | 方案 2 |
| **复杂度** | ✅ 简单 | ❌ 复杂 | 方案 1 |
| **可靠性** | ⚠️ 中等 | ✅ 高 | 方案 2 |
| **渐进式** | ✅ 支持 | ✅ 支持 | 混合 |

**最终建议**：先实施方案 1 快速修复，然后逐步迁移到方案 2。

---

**创建日期**: 2026-01-29
**分析者**: Claude Sonnet 4.5
