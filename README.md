# TSP

TypeScript Server Page - 使用 Deno + TSX + Preact 实现的模板服务器

## ✨ 特点

- 🚀 **简单易用** - 像 PHP 一样无需预编译，直接执行 `.tsx` 文件
- ⚡ **智能缓存** - 基于文件修改时间的模块缓存，性能优异
- 🔒 **安全可靠** - 完善的路径检查和权限控制
- 📦 **功能完整** - 支持查询参数、POST 数据、Cookies、重定向等
- 🎨 **组件化** - 使用 TSX + Preact，支持现代前端组件开发
- 💎 **类型安全** - 完整的 TypeScript 类型支持

## 🚀 快速开始

### 1. 安装 Deno

访问 [deno.land](https://deno.land/) 安装 Deno。

### 2. 启动服务器

```bash
# 克隆仓库
git clone https://github.com/your-repo/tsp.git
cd tsp

# 使用 Deno 直接运行
deno run --allow-net --allow-read src/main.ts --root ./www --port 9000

# 或使用开发模式
deno run --allow-net --allow-read src/main.ts --root ./www --port 9000 --dev
```

### 3. 访问应用

打开浏览器访问 `http://localhost:9000`

## 🐳 Docker 测试服务

项目包含 Docker Compose 配置，用于快速启动测试所需的 MySQL 和 Redis 服务。

详见 [DOCKER.md](./DOCKER.md)

### Linux / macOS

```bash
# 启动服务
./docker-start.sh

# 测试连接
./docker-test-connection.sh

# 停止服务
./docker-stop.sh

# 删除所有测试数据
./docker-cleanup.sh
```

### Windows (PowerShell)

```powershell
# 启动服务
.\docker-start.ps1

# 测试连接
.\docker-test-connection.sh

# 停止服务
.\docker-stop.ps1

# 删除所有测试数据
.\docker-cleanup.ps1
```

## 📦 编译为可执行文件

```bash
# 编译
deno compile --allow-net --allow-read --allow-write --allow-env --output tspserver src/main.ts

# 运行
./tspserver --root ./www --port 9000
```

## 📖 文档

完整文档请访问 [docs/](./docs/)

- [快速开始](./docs/getting-started.md) - 5 分钟上手指南
- [配置说明](./docs/configuration.md) - 配置文件使用方法
- [架构设计](./docs/architecture.md) - 系统架构和原理
- [开发指南](./docs/development.md) - 开发环境配置
- [开发路线图](./docs/ROADMAP.md) - 功能规划和未来计划

## 🛠️ 常用命令

```bash
# 开发模式（热重载）
deno task dev

# 生产模式
deno task start

# 编译二进制
deno task compile

# 运行测试
deno task test

# 清理生成文件
deno task clean
```

## 💡 页面示例

创建 `index.tsx` 文件：

```tsx
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

访问 `http://localhost:9000/?name=TSP` 即可看到效果。

## 📄 许可证

MIT License

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

---

**查看完整文档**: [docs/](./docs/)
