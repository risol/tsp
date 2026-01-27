# 使用 @deno/loader 解决动态加载 TSX 问题

## ✅ 解决方案

使用 `@deno/loader` 在编译后的二进制中动态加载和转译 TSX 文件。

## 🔧 实现步骤

### 1. 添加依赖

```bash
deno add jsr:@deno/loader
```

### 2. 修改 `src/cache.ts`

使用 `@deno/loader` 的 API 来：
1. 解析模块路径
2. 加载并转译 TSX 为 JavaScript
3. 使用 data URL 导入执行

**关键代码**:

```typescript
import {
  Workspace,
  ResolutionMode,
  type LoadResponse,
  RequestedModuleType,
} from "@deno/loader";

// 创建全局 loader
const workspace = new Workspace();
const loader = await workspace.createLoader();

// 解析模块
const resolvedUrl = await loader.resolve(
  filepath,
  `file://${Deno.cwd()}/`,
  ResolutionMode.Import
);

// 加载并转译
const response = await loader.load(
  resolvedUrl,
  RequestedModuleType.Default
);

// 使用 data URL 导入
const code = new TextDecoder().decode(response.code);
const dataUrl = `data:application/javascript,${encodeURIComponent(code)}`;
const module = await import(dataUrl);
```

### 3. 重新编译

```bash
rm -f tsp-fpm.exe
deno compile --allow-net --allow-read --output tsp-fpm src/main.ts
```

## 📊 工作原理

```
┌─────────────────────────────────────┐
│  tsp-fpm.exe (编译后的二进制)        │
│  ├─ 内置 @deno/loader (Wasm)         │
│  └─ 包含完整的 Deno 运行时           │
├─────────────────────────────────────┤
│  运行时动态加载流程:                  │
│                                     │
│  1. getPage() 被调用                 │
│     ↓                               │
│  2. @deno/loader.resolve()           │
│     解析 www/form.tsx 路径            │
│     ↓                               │
│  3. @deno/loader.load()              │
│     读取 TSX 源码                    │
│     使用内置 Wasm 编译器转译         │
│     返回 JavaScript 代码            │
│     ↓                               │
│  4. import(data URL)                │
│     执行转译后的 JS 代码             │
│     返回模块导出                     │
│                                     │
└─────────────────────────────────────┘
```

## ✨ 优势

### 1. 完全兼容编译后的二进制
- `@deno/loader` 包含完整的 Deno 运行时
- 使用 Wasm 编译器，与 Deno CLI 相同
- 支持所有 TypeScript/TSX 特性

### 2. 无需预构建
- 不需要先转译 `.tsx` 为 `.js`
- 动态加载和编译
- 开发体验不变

### 3. 保持缓存机制
- 文件修改时间检查
- 转译结果缓存
- 性能优化

## 🔍 关键要点

### data URL 编码

```typescript
// ❌ 错误: base64 编码不适合 UTF-8
const dataUrl = `data:application/javascript;base64,${btoa(code)}`;

// ✅ 正确: 使用 encodeURIComponent
const dataUrl = `data:application/javascript,${encodeURIComponent(code)}`;
```

### Uint8Array 转换

```typescript
// response.code 是 Uint8Array
const code = new TextDecoder().decode(response.code);
```

### 全局 Loader 复用

```typescript
// 创建一次，重复使用
let globalLoader: Awaited<ReturnType<typeof Workspace.prototype.createLoader>> | null = null;

async function getGlobalLoader() {
  if (!globalLoader) {
    const workspace = new Workspace();
    globalLoader = await workspace.createLoader();
  }
  return globalLoader;
}
```

## 📝 完整的 diff

```diff
+ import {
+   Workspace,
+   ResolutionMode,
+   type LoadResponse,
+   RequestedModuleType,
+ } from "@deno/loader";

+ // 创建全局 loader 实例
+ let globalLoader: Awaited<ReturnType<typeof Workspace.prototype.createLoader>> | null = null;

+ async function getGlobalLoader() {
+   if (!globalLoader) {
+     const workspace = new Workspace();
+     globalLoader = await workspace.createLoader();
+   }
+   return globalLoader;
+ }

-   // 缓存无效或不存在，动态导入模块
-   const moduleUrl = `file://${join(Deno.cwd(), filepath)}`;
-   const module = await import(moduleUrl);

+   // 获取 loader
+   const loader = await getGlobalLoader();
+
+   // 解析模块路径
+   const resolvedUrl = await loader.resolve(
+     filepath,
+     `file://${Deno.cwd()}/`,
+     ResolutionMode.Import
+   );
+
+   // 加载模块
+   const response = await loader.load(
+     resolvedUrl,
+     RequestedModuleType.Default
+   );
+
+   if (response.kind !== "module") {
+     throw new Error(`Failed to load module: ${filepath}`);
+   }
+
+   // 将 Uint8Array 转换为字符串
+   const code = new TextDecoder().decode(response.code);
+
+   // 使用 data URL 导入转译后的代码
+   const dataUrl = `data:application/javascript,${encodeURIComponent(code)}`;
+   const module = await import(dataUrl);
```

## 🎯 验证

### 语法检查
```bash
deno check src/cache.ts
```

### 编译
```bash
deno compile --allow-net --allow-read --output tsp-fpm src/main.ts
```

### 运行
```bash
./tsp-fpm.exe -r ./www -p 9000 --dev
```

### 访问测试
```bash
# 测试动态加载 TSX
curl http://127.0.0.1:9000/form.tsx

# 应该返回正常页面，不再报错
```

## 🚀 预期结果

使用 `@deno/loader` 后：
- ✅ `deno run` 正常工作
- ✅ `deno compile` 后的二进制也能动态加载 TSX
- ✅ 不需要预构建步骤
- ✅ 保持开发体验

## 📚 相关资源

- [@deno/loader on JSR](https://jsr.io/@deno/loader)
- [deno-js-loader on GitHub](https://github.com/denoland/deno-js-loader)
- [Archiving deno_emit Issue #200](https://github.com/denoland/deno_emit/issues/200)

## ⚠️ 注意事项

1. **依赖大小**: `@deno/loader` 会增加二进制大小（包含 Wasm 编译器）
2. **首次加载**: 第一次加载时需要初始化 loader（约几十毫秒）
3. **内存使用**: Wasm 编译器会增加内存占用
4. **缓存重要**: 务必使用文件修改时间缓存，避免重复转译

## 🎉 结论

使用 `@deno/loader` 是解决编译后二进制动态加载 TSX 文件的**最佳方案**：

- ✅ 官方维护
- ✅ 与 Deno CLI 相同的编译器
- ✅ 支持所有 TS/TSX 特性
- ✅ 无需修改开发流程

---

**更新时间**: 2026-01-27
**测试状态**: ✅ 语法检查通过
**下一步**: 编译并测试运行
