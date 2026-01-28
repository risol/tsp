# TSP 架构文档

## 概述

TSP 是一个使用 Deno + TSX + Preact 实现的模板执行引擎。

## 核心特性

- 使用 `.tsx` 文件作为页面
- 基于 Preact 的 JSX 渲染
- 智能的文件修改时间缓存
- 支持编译后的二进制动态加载 TSX
- 完整的 TypeScript 类型支持

## 技术栈

- **Runtime**: Deno
- **语言**: TypeScript/TSX
- **渲染引擎**: Preact (10.25.4)
- **模块加载**: @deno/loader (编译后支持动态 TSX 加载)
- **HTTP 服务**: Deno std/http

## 架构设计

### 文件结构

```
tsp/
├── src/
│   ├── main.ts       # 主程序入口，HTTP 服务器
│   ├── cache.ts      # 模板缓存和加载逻辑
│   ├── context.ts    # 请求上下文构建
│   └── router.ts     # URL 到文件的映射
├── www/              # 页面根目录
│   ├── index.tsx     # 首页
│   ├── form.tsx      # 表单示例
│   ├── api.tsx       # API 示例
│   ├── redirect.tsx  # 重定向示例
│   └── components/   # 可复用组件
└── docs/
    └── 开发文档.md    # 详细开发文档
```

### 工作流程

```
HTTP 请求
    ↓
路由解析 (router.ts)
    ↓
安全检查
    ↓
构建上下文 (context.ts)
    ↓
获取页面函数 (cache.ts)
    ├─ 缓存命中 → 直接使用
    └─ 缓存未命中 → 动态加载 TSX
        ├─ deno run: 直接 import
        └─ deno compile: @deno/loader 转译
    ↓
执行页面函数 → 返回 JSX
    ↓
渲染 JSX → HTML (Preact)
    ↓
返回 HTTP 响应
```

### 核心模块

#### 1. main.ts

- 启动 HTTP 服务器
- 解析命令行参数
- 请求分发和错误处理

**关键功能:**
- 解析 `--root`, `--port`, `--dev` 参数
- 监听指定端口
- 处理每个请求，调用相应模块

#### 2. cache.ts

**最核心的模块**，负责:

- 页面函数缓存（基于文件修改时间）
- 动态加载 TSX 文件
- 支持 deno compile 后的动态加载

**缓存策略:**
```typescript
type CacheEntry = {
  mtimeMs: number;      // 文件修改时间
  pageFunction: PageFunction;  // 页面函数
};

const cache = new Map<string, CacheEntry>();
```

**动态加载逻辑:**
```typescript
// deno run 模式
const module = await import(fileUrl);

// deno compile 模式
const loader = await getGlobalLoader();
const response = await loader.load(fileUrl, RequestedModuleType.Default);
const code = new TextDecoder().decode(response.code);
const dataUrl = `data:application/javascript,${encodeURIComponent(code)}`;
const module = await import(dataUrl);
```

#### 3. context.ts

构建请求上下文对象:

```typescript
type PageContext = {
  method: HttpMethod;
  url: URL;
  headers: Headers;
  query: Record<string, string>;
  body: unknown;
  cookies: Record<string, string>;
  file: string;
  root: string;
};
```

**处理内容:**
- URL 查询参数解析
- 请求体解析 (JSON / form / text)
- Cookie 解析
- 请求头处理

#### 4. router.ts

URL 到文件的映射规则:

| URL 路径 | 文件路径 |
|---------|---------|
| `/` | `www/index.tsx` |
| `/about` | `www/about.tsx` |
| `/user/profile` | `www/user/profile.tsx` |
| `/user/` | `www/user/index.tsx` |

**安全特性:**
- 路径归一化
- 禁止路径穿越 (`../`)
- 只允许 `.tsx` 文件
- 必须在 root 目录内

## 页面开发

### 页面结构

每个 `.tsx` 文件导出默认函数:

```tsx
export default async function (context: PageContext) {
  return <div>页面内容</div>;
}
```

### 支持的返回值

1. **JSX 元素** - 渲染为 HTML
2. **重定向对象** - 触发 HTTP 重定向
   ```tsx
   return { redirect: "/target" };
   ```
3. **Response 对象** - 自定义响应
   ```tsx
   return new Response("...", { status: 200, headers: {...} });
   ```

### 组件开发

在 `www/components/` 创建可复用组件:

```tsx
// www/components/Header.tsx
interface HeaderProps {
  title: string;
}

export function Header({ title }: HeaderProps) {
  return <header><h1>{title}</h1></header>;
}
```

## 编译部署

### 开发模式

```bash
deno run --allow-net --allow-read src/main.ts --dev
```

- 实时看到代码修改
- 显示详细错误信息

### 生产部署

**方法 1: 使用 deno run**
```bash
deno run --allow-net --allow-read --allow-env src/main.ts
```

**方法 2: 编译为二进制**
```bash
# 编译
deno compile --allow-net --allow-read --allow-env --output tspserver src/main.ts

# 运行 (需要设置 DENO_DIR)
DENO_DIR=/path/to/.deno ./tspserver --root ./www --port 9000
```

**编译后的优势:**
- 无需安装 Deno
- 更快的启动速度
- 单文件分发

**注意事项:**
- 需要 `--allow-env` 权限
- 需要设置 `DENO_DIR` 环境变量
- 使用 `@deno/loader` 支持动态加载 TSX

## 性能优化

### 缓存机制

1. **文件缓存**: 基于 `mtime` 的智能缓存
   - 文件未修改 → 复用缓存
   - 文件已修改 → 重新加载

2. **模块缓存**: Deno 的模块缓存
   - 自动缓存远程依赖
   - 加速模块加载

### 性能指标

| 操作 | 耗时 |
|------|-----|
| 首次加载 TSX (deno run) | ~20ms |
| 首次加载 TSX (compiled) | ~50ms |
| 缓存后加载 | ~5ms |
| Preact 渲染 | ~1-5ms |

## 安全设计

### 路径安全

```typescript
// 路径归一化
const normalized = normalize(path);
const absolute = resolve(root, normalized);

// 检查是否在 root 内
if (!absolute.startsWith(root)) {
  throw new Error("Path traversal detected");
}
```

### 文件类型白名单

```typescript
if (!filepath.endsWith(".tsx")) {
  throw new Error("Only .tsx files are allowed");
}
```

### 错误处理

- **开发模式**: 显示详细错误和堆栈
- **生产模式**: 隐藏敏感信息，显示通用错误页面

## 扩展性

### 添加中间件

在 `main.ts` 的请求处理流程中添加:

```typescript
async function handleRequest(req: Request) {
  // 1. 日志中间件
  console.log(`${req.method} ${req.url}`);

  // 2. 认证中间件
  const authHeader = req.headers.get("Authorization");
  if (!authHeader) {
    return new Response("Unauthorized", { status: 401 });
  }

  // 3. 路由处理
  // ...
}
```

### 自定义类型

扩展 `PageContext`:

```typescript
// src/context.ts
export interface ExtendedPageContext extends PageContext {
  user?: User;
  session?: Session;
}

export function buildExtendedContext(req: Request, file: string, root: string): ExtendedPageContext {
  const base = buildContext(req, file, root);

  return {
    ...base,
    user: getCurrentUser(req),
  };
}
```

## 对比其他方案

### vs PHP-FPM

| 特性 | PHP-FPM | TSP |
|------|---------|---------|
| 语言 | PHP | TypeScript |
| 模板语法 | PHP | TSX (JSX) |
| 类型安全 | ❌ | ✅ |
| 组件化 | ❌ | ✅ |
| 现代工具链 | ❌ | ✅ |

### vs Next.js

| 特性 | Next.js | TSP |
|------|---------|---------|
| 构建步骤 | ✅ 需要 | ❌ 不需要 |
| 服务端渲染 | ✅ | ✅ |
| 简单性 | ❌ 复杂 | ✅ 简单 |
| 学习曲线 | 陡峭 | 平缓 |

## 最佳实践

1. **类型安全**: 始终使用 `PageContext` 类型
2. **组件复用**: 提取公共部分为组件
3. **错误处理**: 使用 try-catch 包裹异步操作
4. **性能**: 利用缓存机制，避免重复计算
5. **安全**: 验证所有用户输入

## 常见问题

### Q: 为什么选择 TSX 而不是模板引擎？

A: TSX 提供:
- 完整的类型安全
- 组件化能力
- 现代的开发体验
- Preact 生态支持

### Q: 如何支持热重载？

A: 使用 `--dev` 模式，文件修改后会自动重新加载

### Q: 可以部署到哪些平台？

A:
- 任何支持 Deno 的平台
- 编译后的二进制可部署到任何 Linux/Windows/macOS 服务器

## 未来规划

- [ ] 支持更多渲染模式 (SSG, ISR)
- [ ] 添加数据库 ORM 集成
- [ ] WebSocket 支持
- [ ] 插件系统
- [ ] CLI 工具增强

## 贡献指南

欢迎贡献代码、报告问题或提出建议！

## 许可证

MIT License

## 相关文档

- [开发指南](./development.md) - 开发环境配置和最佳实践
- [功能特性](./features/README.md) - TSP 的功能说明
- [快速开始](./getting-started.md) - 5 分钟快速上手
- [测试文档](./testing/README.md) - 测试相关文档

---

[← 返回文档中心](./README.md)
