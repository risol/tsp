# TSP 构建和部署文档

## 概述

TSP 提供了完整的构建工具链，可以将项目打包为独立的可分发版本。

## 构建命令

### 完整构建

使用 `build` 任务创建完整的发布包：

```bash
deno task build
```

这个命令会：

1. ✅ 清理旧的 `dist/` 目录
2. ✅ 编译二进制文件到 `dist/tspserver` (或 `dist/tspserver.exe` Windows)
3. ✅ 复制 `www/` 目录到 `dist/www/`
4. ✅ 复制配置文件（如果存在）到 `dist/`
5. ✅ 生成 `dist/README.md` 使用说明
6. ✅ 创建必要的目录结构

### 其他编译命令

```bash
# 仅编译二进制文件到当前目录
deno task compile

# 仅编译到指定路径
deno compile --allow-net --allow-read --allow-write --allow-env --output myserver src/main.ts
```

## 构建产物

构建完成后，`dist/` 目录结构如下：

```
dist/
├── tspserver.exe          # Windows 二进制文件
│   或 tspserver           # Linux/Mac 二进制文件
├── www/                   # 网站文件
│   ├── index.tsx
│   ├── form.tsx
│   ├── api.tsx
│   ├── components/
│   │   ├── Layout.tsx
│   │   ├── Header.tsx
│   │   └── ...
│   └── features/
│       └── ...
├── config.json            # 可选配置文件
├── config.jsonc           # 可选配置文件
└── README.md              # 使用说明
```

## 部署步骤

### 1. 构建项目

```bash
deno task build
```

### 2. 测试构建结果

```bash
# 进入 dist 目录
cd dist

# 启动服务器测试
# Windows
DENO_DIR=./.deno .\tspserver.exe

# Linux/Mac
DENO_DIR=./.deno ./tspserver
```

### 3. 打包分发

#### Windows

```bash
# 使用 PowerShell
Compress-Archive -Path dist\* -DestinationPath tspserver-windows.zip

# 或使用 7-Zip / WinRAR
```

#### Linux/Mac

```bash
# 创建 tar.gz 压缩包
tar -czf tspserver-linux.tar.gz -C dist .

# 或创建 zip
cd dist
zip -r ../tspserver-linux.zip .
cd ..
```

### 4. 部署到服务器

将打包后的文件上传到目标服务器：

```bash
# 使用 scp 上传
scp tspserver-linux.tar.gz user@server:/path/to/deploy/

# 在服务器上解压
ssh user@server
cd /path/to/deploy
tar -xzf tspserver-linux.tar.gz
cd dist
DENO_DIR=./.deno ./tspserver
```

## 服务器配置

### 使用配置文件

在 `dist/` 目录创建 `config.json` 或 `config.jsonc`:

```json
{
  "root": "./www",
  "port": 9000,
  "dev": false
}
```

### 使用命令行参数

```bash
# 指定端口
./tspserver --port 8080

# 指定文档根目录
./tspserver --root /var/www

# 开发模式
./tspserver --dev

# 组合使用
./tspserver --root ./www --port 9000 --dev
```

## 环境变量

### DENO_DIR

编译后的二进制文件需要设置 `DENO_DIR` 环境变量：

```bash
# 设置 DENO_DIR 指向当前目录
export DENO_DIR=./.deno  # Linux/Mac
set DENO_DIR=./.deno     # Windows CMD
$env:DENO_DIR="./.deno"  # Windows PowerShell

# 然后运行服务器
./tspserver
```

### 一次性设置

```bash
# Linux/Mac
DENO_DIR=./.deno ./tspserver

# Windows PowerShell
$env:DENO_DIR="./.deno"; .\tspserver.exe

# Windows CMD
set DENO_DIR=./.deno&& tspserver.exe
```

## 生产环境部署建议

### 1. 使用进程管理器

#### systemd (Linux)

创建 `/etc/systemd/system/tsp.service`:

```ini
[Unit]
Description=TSP Server
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/opt/tsp
Environment="DENO_DIR=./.deno"
ExecStart=/opt/tsp/tspserver --root ./www --port 9000
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

启动服务：

```bash
sudo systemctl daemon-reload
sudo systemctl enable tsp
sudo systemctl start tsp
```

#### PM2

```bash
# 安装 PM2
npm install -g pm2

# 启动服务
pm2 start tspserver --name tsp -- --root ./www --port 9000

# 设置开机自启
pm2 startup
pm2 save
```

### 2. 使用反向代理

#### Nginx

```nginx
server {
    listen 80;
    server_name example.com;

    location / {
        proxy_pass http://localhost:9000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
        proxy_set_header Host $host;
        proxy_cache_bypass $http_upgrade;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

#### Caddy

```
example.com {
    reverse_proxy localhost:9000
}
```

### 3. 使用 Docker

创建 `Dockerfile`:

```dockerfile
FROM debian:bookworm-slim

# 安装依赖
RUN apt-get update && apt-get install -y \
    curl \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# 复制构建产物
COPY dist/ /app/

# 暴露端口
EXPOSE 9000

# 设置环境变量并启动
ENV DENO_DIR=./.deno
CMD ["./tspserver", "--root", "./www", "--port", "9000"]
```

构建和运行：

```bash
# 构建镜像
docker build -t tsp-server .

# 运行容器
docker run -d -p 9000:9000 --name tsp tsp-server
```

### 4. 安全建议

1. **文件权限**
   ```bash
   # 设置适当的文件权限
   chmod 750 tspserver
   chmod -R 640 www/
   chmod +X www/  # 设置目录执行权限
   ```

2. **防火墙配置**
   ```bash
   # UFW (Ubuntu)
   sudo ufw allow 9000/tcp

   # firewalld (CentOS)
   sudo firewall-cmd --add-port=9000/tcp --permanent
   sudo firewall-cmd --reload
   ```

3. **HTTPS 配置**
   - 使用 Let's Encrypt 获取免费 SSL 证书
   - 配置 Nginx/Caddy 处理 HTTPS
   - 设置自动证书续期

## 清理构建产物

```bash
# 清理所有生成文件
deno task clean

# 仅删除 dist 目录
rm -rf dist/
```

## 常见问题

### Q: 为什么需要 DENO_DIR 环境变量？

A: 编译后的二进制文件需要 Deno 运行时来处理模块缓存和动态导入。`DENO_DIR` 指定缓存目录位置。

### Q: 可以在没有 Deno 的机器上运行吗？

A: 需要安装 Deno 运行时。二进制文件包含了您的应用代码，但依赖 Deno 运行时环境。

### Q: 如何减少二进制文件大小？

A: 可以使用 `--no-remote` 选项避免嵌入远程模块：

```bash
deno compile --no-remote --allow-net --allow-read --allow-write --allow-env --output tspserver src/main.ts
```

注意：这需要目标机器上有网络连接来下载依赖。

### Q: 如何调试编译后的二进制？

A: 可以使用 `--log` 级别或启用开发模式：

```bash
./tspserver --dev
```

## 相关文档

- [预编译功能文档](./PRECOMPILATION.md)
- [开发规范](./CODING_STANDARDS.md)
- [README](../README.md)
