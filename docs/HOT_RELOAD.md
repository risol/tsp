# TSP 热重载功能文档

## 概述

TSP 支持智能热重载（Hot Reload）功能，在开发模式下自动检测文件变更并重新编译受影响的模块，无需重启服务器。

## 工作原理

### 缓存失效机制

TSP 使用以下策略检测文件变更：

1. **主文件修改时间检查**：每次请求时检查 TSX 文件的修改时间（mtime）
2. **依赖文件修改时间检查**：递归检查所有本地依赖（TSX/TS）的修改时间
3. **自动重新编译**：如果任何文件被修改，自动重新编译受影响的主文件和依赖

### 依赖追踪

系统会自动分析每个 TSX 文件的本地导入：

```typescript
// 自动追踪这些依赖
import { Component } from "./components/Component";  // TSX 组件
import { helper } from "../utils/helper";           // TS 工具函数
import { config } from "./config";                  // JS 配置
```

### 缓存层

```
请求 → getPage() → 检查 mtime → 缓存有效？
                          ↓ YES          ↓ NO
                    使用内存缓存    检查依赖 mtime
                                        ↓
                                   依赖被修改？
                                        ↓ YES           ↓ NO
                              重新编译依赖链    仅编译主文件
```

## 使用场景

### 开发模式

在开发模式下，热重载自动启用：

```bash
deno task dev
# 或
deno run --allow-net --allow-read --allow-write --allow-env --allow-run src/main.ts --dev
```

### 生产模式

生产模式下启动时预编译所有文件，运行时也会检查文件变更：

```bash
deno task start
# 或
deno run --allow-net --allow-read --allow-write --allow-env src/main.ts
```

## 热重载场景

### 场景1：主文件修改

修改 `www/index.tsx`：

```typescript
// 修改前
export default async function() {
  return <h1>Version 1</h1>;
}

// 修改后
export default async function() {
  return <h1>Version 2</h1>;
}
```

**结果**：✅ 自动重新编译 `index.tsx`，刷新页面立即生效

### 场景2：组件文件修改

修改 `www/components/Header.tsx`：

```typescript
// 修改前
export function Header() {
  return <header>Old Header</header>;
}

// 修改后
export function Header() {
  return <header>New Header</header>;
}
```

**结果**：✅ 自动重新编译 `Header.tsx` 和所有导入它的文件（如 `index.tsx`）

### 场景3：工具函数修改

修改 `www/utils/format.ts`：

```typescript
// 修改前
export function formatDate(date: Date) {
  return date.toISOString();
}

// 修改后
export function formatDate(date: Date) {
  return date.toLocaleDateString();
}
```

**结果**：✅ 自动重新编译 `format.ts` 和所有使用它的组件

### 场景4：级联依赖修改

```
index.tsx → components/Layout.tsx → components/Navigation.tsx
```

修改 `Navigation.tsx`：

**结果**：✅ 自动重新编译整个依赖链：
- `Navigation.tsx`
- `Layout.tsx`（依赖 Navigation）
- `index.tsx`（依赖 Layout）

## 日志输出

### 缓存命中（CACHE HIT）

```
[CACHE HIT] www/index.tsx (mtime: 1706534400000)
```

说明文件未修改，使用内存缓存。

### 缓存未命中（CACHE MISS）

```
[CACHE MISS] www/index.tsx - recompiling...
[INFO] Dependencies: D:\GitHub\tsp\www\components\Layout.tsx
[COMPILED] D:\GitHub\tsp\www\index.tsx -> .cache\tsp\index.js
```

说明文件已修改，正在重新编译。

### 依赖文件修改

```
[INFO] Dependency modified: D:\GitHub\tsp\www\components\Layout.tsx
[CACHE MISS] www/index.tsx - recompiling...
[INFO] Compiling dependency: D:\GitHub\tsp\www\components\Layout.tsx
```

说明依赖文件被修改，正在重新编译依赖链。

## 测试

运行热重载测试：

```bash
deno task test:hot-reload
```

测试覆盖：

1. ✅ **主文件热重载**：修改主 TSX 文件，验证自动重载
2. ✅ **组件热重载**：修改依赖的 TSX 组件，验证主文件自动重载
3. ✅ **工具文件热重载**：修改依赖的 TS 工具文件，验证主文件自动重载
4. ✅ **级联热重载**：修改多层依赖的底层文件，验证整个链路自动重载

## 性能考虑

### 开发模式

- **启动速度**：⚡ 快速（跳过预编译）
- **首次请求**：按需编译（稍慢）
- **后续请求**：使用缓存（快速）
- **文件变更**：自动重新编译（透明）

### 生产模式

- **启动速度**：⏳ 较慢（预编译所有文件）
- **首次请求**：使用缓存（快速）
- **后续请求**：使用缓存（快速）
- **文件变更**：自动重新编译（透明）

### 内存占用

- **模块缓存**：仅缓存编译后的函数（轻量级）
- **依赖图**：存储文件路径关系（小内存占用）
- **mtime 追踪**：仅存储时间戳数字（极小）

## 限制

### 不追踪的变更

1. **远程依赖**：HTTP/HTTPS/npm/jsr 导入不检查变更
2. **配置文件**：修改 `config.json` 需要重启服务器
3. **源代码之外**：静态资源（CSS、图片等）变更不会触发重载

### 安全限制

- **禁止远程导入**：编译时会检查并阻止远程文件导入
- **路径穿越防护**：自动阻止恶意路径访问
- **文件类型白名单**：仅处理 `.tsx`、`.ts`、`.js` 文件

## 最佳实践

### 1. 开发工作流

```bash
# 终端1：启动开发服务器
deno task dev

# 终端2：运行测试（可选）
deno task test:watch
```

### 2. 文件组织

建议的文件组织结构：

```
www/
├── pages/           # 页面文件
│   ├── index.tsx
│   └── about.tsx
├── components/      # 可复用组件
│   ├── Header.tsx
│   └── Footer.tsx
└── utils/           # 工具函数
    ├── format.ts
    └── validate.ts
```

### 3. 性能优化

- **避免过度导入**：只导入必要的文件
- **共享组件**：将通用功能提取到独立组件
- **工具函数**：将复杂逻辑移到工具文件

## 故障排查

### 问题：修改文件后没有生效

**解决方案**：
1. 检查是否在开发模式（`--dev` 标志）
2. 查看服务器日志，确认没有编译错误
3. 确认文件保存成功（检查文件修改时间）

### 问题：依赖文件修改后没有触发重载

**解决方案**：
1. 确认使用本地导入（`./` 或 `../`）
2. 检查导入路径是否正确
3. 查看服务器日志，确认依赖被正确识别

### 问题：缓存未命中导致频繁重编译

**解决方案**：
1. 这是正常行为，系统会自动处理
2. 频繁重编译可能是因为文件系统监控问题
3. 可以使用生产模式（预编译）来避免启动时的编译

## 相关文档

- [预编译功能文档](./PRECOMPILATION.md)
- [构建和部署文档](./BUILD.md)
- [开发规范](./CODING_STANDARDS.md)
