# TSP-FPM

类 PHP-FPM 模板执行引擎 - 使用 Deno + Eta 实现

## 简介

TSP-FPM 是一个类似 PHP-FPM 的模板执行引擎，让 `.tsp/.jsp` 文件能够像 PHP 一样直接执行并输出 HTML。

### 特点

- 🚀 简单易用，像 PHP 一样无需预编译
- ⚡ 基于文件修改时间的智能缓存
- 🔒 安全的路径检查和权限控制
- 📦 支持查询参数、POST 数据、Cookies
- 🎨 使用 Eta 模板语法，功能强大
- 📦 使用 Deno 原生 API，无第三方框架依赖

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
deno compile --allow-net --allow-read --output tsp-fpm src/main.ts

# 运行编译后的程序
./tsp-fpm --root ./www --port 9000
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
/                → index.tsp | index.jsp
/a               → a.tsp | a/index.tsp
/a/b             → a/b.tsp
```

### 模板语法

`.tsp` 文件使用 Eta 模板语法：

```eta
<!-- 输出变量 -->
<h1>Hello <%= it.query.name ?? "World" %></h1>

<!-- JavaScript 代码 -->
<% if (it.method === "POST") { %>
  <pre><%= JSON.stringify(it.body) %></pre>
<% } %>

<!-- 循环 -->
<% for (const item of it.items) { %>
  <div><%= item.name %></div>
<% } %>
```

## 上下文对象

模板中可以通过 `it` 访问以下上下文：

```typescript
{
  method: string      // HTTP 方法 (GET, POST 等)
  url: URL           // 请求 URL 对象
  headers: Headers   // 请求头
  query: Record<string, string>    // 查询参数
  body: unknown      // 请求体 (POST 数据)
  cookies: Record<string, string>  // Cookies
  file: string       // 当前模板文件路径
  root: string       // 文档根目录路径
}
```

## 项目结构

```
tsp/
├── src/
│   ├── main.ts       # 主程序入口
│   ├── cache.ts      # 模板缓存模块
│   ├── router.ts     # 路由和文件映射
│   └── context.ts    # 上下文处理
├── www/
│   └── index.tsp     # 示例文件
├── README.md
└── 架构.md
```

## 安全特性

- ✅ 路径归一化 + root containment 校验
- ✅ 文件扩展名白名单（仅允许 .tsp/.jsp）
- ✅ 防止路径穿越攻击
- ✅ 生产模式隐藏错误堆栈信息

## 性能优化

- 基于 `mtime` 的模板编译缓存
- 仅在文件修改时重新编译
- 避免每次请求都解析模板

## 示例代码

### GET 请求处理

```eta
<p>用户名: <%= it.query.name ?? "游客" %></p>
<a href="?name=张三">设置名字为张三</a>
```

### POST 请求处理

```eta
<% if (it.method === "POST") { %>
  <% if (it.body && it.body.username) { %>
    <p>欢迎, <%= it.body.username %>!</p>
  <% } %>
<% } %>

<form method="POST">
  <input type="text" name="username">
  <button type="submit">提交</button>
</form>
```

### Cookie 读取

```eta
<% if (it.cookies.session) { %>
  <p>Session ID: <%= it.cookies.session %></p>
<% } else { %>
  <p>未登录</p>
<% } %>
```

## 技术栈

- **Runtime**: Deno
- **模板引擎**: Eta
- **语言**: TypeScript
- **HTTP 服务**: Deno std/http

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
