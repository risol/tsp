# Deno Compile 与动态导入 TSX 的问题

## 🔴 问题根源

### 位置
`src/cache.ts:76-77`

```typescript
const moduleUrl = `file://${join(Deno.cwd(), filepath)}`;
const module = await import(moduleUrl);
```

### 原因

**编译后的二进制在运行时动态导入 `.tsx` 文件时，不会进行 TypeScript/JSX 转换**

#### `deno run` ✅ 能工作
- 运行时有完整的 TypeScript/JSX 编译器
- 可以动态导入并转译 `.tsx` 文件

#### `deno compile` ❌ 不能工作
- 编译时只处理静态依赖（那些直接 `import` 的）
- 动态导入（使用 `import()` 函数）的文件在运行时作为纯 JavaScript 加载
- TypeScript 的 `import type { ... }` 语法不是有效的 JavaScript
- JSX 语法也不是有效的 JavaScript

### 错误信息

```
Unexpected token '{' at file:///D:/GitHub/tsp/www/form.tsx:1:13
```

这是因为 `import type { PageContext } from "../src/cache.ts";` 在 JavaScript 中是无效语法。

## 🔍 问题分析

### 当前架构

```
┌─────────────┐
│  main.ts    │ (静态依赖，被编译进二进制)
├─────────────┤
│  cache.ts   │ (被编译进二进制)
│  getPage()  │
│     ↓       │
│  import()   │ ← 动态导入
│     ↓       │
│  *.tsx      │ ← 运行时加载，不会转译 ❌
└─────────────┘
```

### 为什么会这样

1. **Deno compile 的工作原理**:
   - 静态分析 `import` 语句
   - 将依赖打包进二进制
   - 移除 TypeScript 编译器以减小体积

2. **动态导入的问题**:
   - `import()` 是运行时操作
   - 编译器无法预知要导入什么
   - 运行时没有 TS/JSX 编译器

3. **`.tsx` 文件的问题**:
   - 包含 TypeScript 语法 (`import type`, `interface`, 类型注解等)
   - 包含 JSX 语法 (`<div>`, `{variable}` 等)
   - 这些需要转译才能在 JavaScript 运行时执行

## 💡 解决方案

### 方案 1: 预编译所有 `.tsx` 为 `.js` (推荐)

在编译前先将所有 `.tsx` 转换为 `.js`:

```bash
# 1. 构建步骤：转译所有 .tsx 文件
deno run --allow-read --allow-write --allow-net https://deno.land/std@0.210.0/cli/denoprompt.tsx build

# 2. 修改 cache.ts 使用 .js 而不是 .tsx
# 3. 编译
deno compile --allow-net --allow-read --output tsp-fpm src/main.ts
```

**优点**: 简单直接
**缺点**: 需要构建步骤

### 方案 2: 使用 `deno emit` 运行时转换

在 `cache.ts` 中使用 `deno.emit` 动态转译:

```typescript
// 修改 src/cache.ts
import { emit } from "jsr:@deno/emit";

export async function getPage(filepath: string): Promise<PageFunction> {
  // ... 缓存检查 ...

  // 读取源代码
  const source = await Deno.readTextFile(filepath);

  // 转译为 JavaScript
  const { url } = await emit(filepath, {
    source,
    compilerOptions: {
      jsx: "react",
      jsxImportSource: "preact",
    },
  });

  // 动态导入转译后的代码
  const module = await import(url);

  // ...
}
```

**优点**: 保持开发体验
**缺点**: 运行时开销，需要 `@deno/emit` 依赖

### 方案 3: 启动时预加载所有模板 (不推荐编译)

在启动时扫描并预加载所有 `.tsx` 文件:

```typescript
// 修改 src/main.ts
async function preloadTemplates(root: string) {
  const files = Deno.readDirSync(root, { recursive: true });
  for (const file of files) {
    if (file.name.endsWith('.tsx')) {
      const filepath = join(root, file.name);
      await getPage(filepath); // 预加载
    }
  }
}

async function main() {
  // ...
  await preloadTemplates(config.root);
  // ...
}
```

**优点**: 简单
**缺点**:
- 启动时间增加
- 编译后仍无法工作（静态导入也需要编译时已知）

### 方案 4: 使用 ESM 模板 + 模板引擎

改用纯 JavaScript 的模板引擎:

```typescript
// 使用 etas, handlebars 等
import etas from "npm:etas";

export default async function (context: PageContext) {
  return etas.render(await Deno.readTextFile("./templates/form.eta"), context);
}
```

**优点**: 编译友好
**缺点**: 丢失 TSX 类型安全和 Preact 组件

## 🎯 推荐方案

### 短期：添加构建步骤

创建 `build.ts`:

```typescript
#!/usr/bin/env -S deno run --allow-read --allow-write --allow-net
import { build } from "jsr:@deno/emit";

await build({
  entrypoints: ["www/**/*.tsx"],
  outDir: "./dist",
  compilerOptions: {
    jsx: "react",
    jsxImportSource: "preact",
  },
});
```

### 长期：重新设计架构

将 `cache.ts` 改为：

1. **构建时预渲染** (SSG)
2. **使用模板引擎** (etas, handlebars)
3. **使用 API 架构** (前后端分离)

## 📝 临时解决方案

### 当前可以工作的方式

```bash
# 开发阶段 - 使用 deno run
deno run --allow-net --allow-read src/main.ts

# 生产阶段 - 不使用 deno compile，直接用 deno run
# 或者配置进程管理器 (pm2, systemd 等)
```

### 不要使用 `deno compile` 的场景

- ❌ 使用动态导入 `.tsx` 文件
- ❌ 使用动态导入 `.ts` 文件（如果有 TypeScript 特性）
- ❌ 依赖运行时类型检查或转译

### 可以使用 `deno compile` 的场景

- ✅ 所有依赖都是静态导入的
- ✅ 所有 TypeScript 代码在编译时已知
- ✅ 不需要运行时动态加载代码

## 🔄 对比其他方案

### 与 Node.js 的对比

**Node.js**:
```bash
# 需要预先转译
tsc src/**/*.ts --outDir dist
node dist/main.js
```

**Deno**:
```bash
# 无需转译
deno run src/main.ts

# 但 compile 仍有限制
deno compile src/main.ts  # 仅适用于纯 JS/静态依赖
```

### 与 Bundler 的对比

**Webpack/Vite**:
- 预打包所有模块
- 可以处理动态导入（有条件地）
- 生成纯 JavaScript bundle

**Deno compile**:
- 打包静态依赖
- 不能处理运行时的 TypeScript/JSX

## ⚠️ 重要说明

### TSP-FPM 的当前状态

**使用 `deno run`**: ✅ 完全正常
```bash
deno run --allow-net --allow-read src/main.ts -r ./www -p 9000
```

**使用 `deno compile`**: ❌ 不能正常工作
```bash
deno compile --allow-net --allow-read --output tsp-fpm src/main.ts
./tsp-fpm  # 访问 .tsx 文件会报错
```

### 推荐使用方式

对于 TSP-FPM 这种模板引擎：
- ✅ 使用 `deno run` (开发)
- ✅ 使用进程管理器 (生产)
- ❌ 不推荐使用 `deno compile` (除非修改架构)

## 📚 相关资源

- [Deno Compile Limitations](https://deno.com/manual/deploying/compile)
- [Dynamic Imports](https://deno.com/manual/basics/modules/dynamic_imports)
- [Deno Emit](https://jsr.io/@deno/emit)

## 🎯 结论

**问题**: `deno compile` 后动态导入的 `.tsx` 文件无法被转译
**原因**: 编译后的二进制没有 TypeScript/JSX 运行时
**解决**: 添加构建步骤预编译 `.tsx`，或使用 `deno run` 代替编译

对于 TSP-FPM 当前架构，**推荐使用 `deno run`** 而不是 `deno compile`。

---

**文档更新**: 2026-01-27
**问题确认**: ✅ 编译后无法动态加载 .tsx
**推荐方案**: 使用 `deno run` 或添加构建步骤
