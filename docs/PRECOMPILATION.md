# TSP 预编译功能文档

## 概述

TSP 现在支持预编译功能，将 `www/**/*.tsx` 文件编译成 `.cache/tsp/**/*.js` 文件，提高运行时性能。

## 主要功能

### 1. 预编译 (Precompilation)

在**生产模式**下启动服务器时，会自动编译所有 TSX 文件：

```bash
deno task start  # 自动预编译
```

编译后的文件存储在 `.cache/tsp/` 目录，保持与 `www/` 相同的目录结构。

### 2. 智能缓存 (Smart Caching)

运行时使用智能缓存机制：

- **首次访问**: 编译 TSX 文件为 JS，缓存到内存
- **后续访问**: 直接使用内存缓存
- **文件变更**: 自动检测文件修改，重新编译
- **依赖变更**: 自动检测依赖文件变更，重新编译

### 3. 依赖追踪 (Dependency Tracking)

系统会分析每个 TSX 文件的本地导入依赖：

```typescript
// 自动追踪这些依赖
import { Component } from "./components/Component";
import { helper } from "../utils/helper";
```

当任何依赖文件被修改时，会自动重新编译相关文件。

### 4. 安全检查 (Security)

**禁止远程导入** - 编译时会检查并阻止以下导入方式：

```typescript
// ❌ 不允许
import { something } from "https://example.com/module.ts";
import { other } from "npm:package";
import { jsr } from "jsr:@std/assert";

// ✅ 允许
import { local } from "./local.tsx";
import { util } from "../utils/helper.ts";
```

### 5. 开发模式 (Development Mode)

开发模式下跳过预编译，按需编译：

```bash
deno task dev  # --dev 标志启用开发模式
```

## 手动预编译命令

### 编译所有文件

```bash
deno task precompile
```

### 清理缓存

```bash
deno task precompile:clean
```

### 清理所有生成文件

```bash
deno task clean
```

## 配置

### deno.json 任务

```json
{
  "tasks": {
    "precompile": "deno run --allow-net --allow-read --allow-write --allow-env src/precompile_tool.ts",
    "precompile:clean": "deno run --allow-net --allow-read --allow-write --allow-env --allow-run src/precompile_tool.ts clean"
  }
}
```

## 技术实现

### 核心模块

- **src/precompiler_lib.ts**: 预编译核心库
  - `compileFile()`: 编译单个 TSX 文件
  - `compileAll()`: 编译所有 TSX 文件
  - `checkRemoteImports()`: 检查远程导入
  - `analyzeDependencies()`: 分析文件依赖
  - `getCachePath()`: 获取缓存路径

- **src/cache.ts**: 缓存管理
  - `getPage()`: 加载页面（支持缓存失效）
  - `needsRecompilation()`: 检查是否需要重新编译
  - `moduleCache`: 内存缓存
  - `dependencyGraph`: 依赖关系图
  - `compiledMtimes`: 编译文件修改时间

- **src/main.ts**: 服务器启动
  - 生产模式：启动时预编译所有文件
  - 开发模式：跳过预编译

### 编译流程

```
TSX 文件 → @deno/loader 转译 → JS 文件 → 缓存到 .cache/tsp/
                                                    ↓
                                           内存缓存（模块 + mtime + 依赖）
```

### 缓存失效检查

每次请求页面时：

1. 检查主文件修改时间
2. 检查所有依赖文件修改时间
3. 如果任何文件变更 → 重新编译
4. 如果无变更 → 使用缓存

## 验证测试

运行预编译功能验证测试：

```bash
deno run --allow-all tests/verify_precompile.ts
```

测试包括：

- ✓ 远程导入检测
- ✓ 本地导入识别
- ✓ 依赖分析
- ✓ 缓存路径生成
- ✓ JSR 导入检测

## 文件结构

```
tsp/
├── .cache/tsp/              # 编译缓存目录
│   ├── api.js              # 编译后的 JS 文件
│   ├── index.js
│   ├── components/         # 保持目录结构
│   │   ├── Footer.js
│   │   ├── Header.js
│   │   └── Layout.js
│   └── features/
│       └── ...
├── src/
│   ├── precompiler_lib.ts  # 预编译库
│   ├── cache.ts            # 缓存管理
│   └── main.ts             # 服务器入口
└── www/
    ├── index.tsx           # 源 TSX 文件
    └── ...
```

## 性能优势

### 生产模式

- **启动时**: 一次性编译所有文件（~3-5秒）
- **运行时**: 加载预编译的 JS 文件，无需转译
- **内存占用**: 仅缓存模块函数，占用极小

### 开发模式

- **启动时**: 跳过预编译（快速启动）
- **运行时**: 按需编译，支持文件修改自动重新编译
- **开发体验**: 热重载，即时反馈

## 注意事项

1. **缓存目录**: `.cache/tsp/` 应该加入 `.gitignore`
2. **远程导入**: TSX 文件中禁止使用远程导入
3. **依赖分析**: 仅追踪本地 `.tsx`, `.ts`, `.js` 文件
4. **缓存失效**: 修改文件后自动重新编译，无需手动清理
5. **兼容性**: 同时支持 `deno run` 和编译二进制模式

## 更新日志

### 2025-01-29

- ✅ 实现预编译功能
- ✅ 添加依赖追踪
- ✅ 添加远程导入检查
- ✅ 实现智能缓存失效
- ✅ 支持开发/生产双模式
- ✅ 添加手动预编译命令
- ✅ 完成验证测试
