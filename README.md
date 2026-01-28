# TSP

TypeScript Server Page - 使用 Deno + TSX + Preact 实现

## 简介

TSP (TypeScript Server Page) 是一个模板执行引擎，让 `.tsx` 文件能够直接执行并输出 HTML。

### 特点

- 🚀 简单易用，像 PHP 一样无需预编译
- ⚡ 基于文件修改时间的智能模块缓存
- 🔒 安全的路径检查和权限控制
- 📦 支持查询参数、POST 数据、Cookies
- 🎨 使用 TSX + Preact，支持组件化开发
- 💎 完整的 TypeScript 类型支持

## 快速开始

### 1. 安装 Deno

如果还没有安装 Deno，请访问 [deno.land](https://deno.land/) 安装。

### 2. 启动服务器

```bash
# 使用 Deno 直接运行
deno run --allow-net --allow-read src/main.ts --root ./www --port 9000

# 或使用开发模式（显示错误详情）
deno run --allow-net --allow-read src/main.ts --root ./www --port 9000 --dev
```

### 3. 访问应用

打开浏览器访问 `http://localhost:9000`

## 编译为可执行文件

```bash
# 编译为独立可执行文件
deno compile --allow-net --allow-read --output tspserver src/main.ts

# 运行编译后的程序
./tspserver --root ./www --port 9000
```

## 使用说明

### 命令行参数

```
--root, -r <path>   文档根目录 (默认: ./www)
--port, -p <port>   监听端口 (默认: 9000)
--dev, -d           开发模式 (显示错误详情)
--help, -h          显示帮助信息
```

### URL 路由规则

```
/                → index.tsx
/a               → a.tsx | a/index.tsx
/a/b             → a/b.tsx
```

### 页面语法

`.tsx` 文件使用 TSX 语法，导出默认函数返回 JSX：

```tsx
import type { PageContext } from "../src/cache.ts";

export default async function (context: PageContext) {
  const { method, query } = context;
  const name = query.name ?? "World";

  return (
    <html>
      <head><title>示例</title></head>
      <body>
        <h1>Hello {name}</h1>
        <p>请求方法: {method}</p>
      </body>
    </html>
  );
}
```

## 上下文对象

页面函数通过 `context` 参数访问以下信息：

```typescript
type PageContext = {
  method: string;      // HTTP 方法 (GET, POST 等)
  url: URL;           // 请求 URL 对象
  headers: Headers;   // 请求头
  query: Record<string, string>;    // 查询参数
  body: unknown;      // 请求体 (POST 数据)
  cookies: Record<string, string>;  // Cookies
  file: string;       // 当前页面文件路径
  root: string;       // 文档根目录路径
}
```

## 项目结构

```
tsp/
├── src/
│   ├── main.ts       # 主程序入口
│   ├── cache.ts      # 模板缓存模块
│   ├── context.ts    # 上下文处理
│   └── router.ts     # 路由和文件映射
├── www/
│   ├── index.tsx     # 示例首页
│   ├── form.tsx      # 表单示例
│   ├── api.tsx       # API 示例
│   ├── redirect.tsx  # 重定向示例
│   └── components/   # 组件目录
├── docs/
│   └── 开发文档.md    # 详细开发文档
├── README.md
└── 架构.md
```

## 安全特性

- ✅ 路径归一化 + root containment 校验
- ✅ 文件扩展名白名单（仅允许 .tsx）
- ✅ 防止路径穿越攻击
- ✅ 生产模式隐藏错误堆栈信息

## 性能优化

- 基于 `mtime` 的模板编译缓存
- 仅在文件修改时重新编译
- 避免每次请求都解析模板

## 示例代码

### GET 请求处理

```tsx
import type { PageContext } from "../src/cache.ts";

export default async function (context: PageContext) {
  const { query } = context;
  const name = query.name ?? "游客";

  return (
    <div>
      <p>用户名: {name}</p>
      <a href="?name=张三">设置名字为张三</a>
    </div>
  );
}
```

### POST 请求处理

```tsx
import type { PageContext } from "../src/cache.ts";

export default async function (context: PageContext) {
  const { method, body } = context;

  if (method === "POST" && typeof body === "object" && body?.username) {
    return <p>欢迎, {body.username}!</p>;
  }

  return (
    <form method="POST">
      <input type="text" name="username" />
      <button type="submit">提交</button>
    </form>
  );
}
```

### Cookie 读取

```tsx
import type { PageContext } from "../src/cache.ts";

export default async function (context: PageContext) {
  const { cookies } = context;
  const sessionId = cookies.session || "未登录";

  return <p>Session ID: {sessionId}</p>;
}
```

## 技术栈

- **Runtime**: Deno
- **语言**: TypeScript/TSX
- **渲染引擎**: Preact
- **HTTP 服务**: Deno std/http
- **模块系统**: ES Modules (动态导入)
- **类型加载**: @deno/loader (支持编译后的二进制动态加载 TSX)

## 开发与生产模式

### 开发模式 (`--dev`)

- 显示详细错误信息和堆栈跟踪
- 便于调试和开发

### 生产模式（默认）

- 隐藏敏感错误信息
- 显示友好的错误页面
- 更安全的部署方式

## 许可证

MIT

## 贡献

欢迎提交 Issue 和 Pull Request！
