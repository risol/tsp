# Linux 二进制文件构建指南

本指南说明如何使用 Docker 构建适用于 Linux 的 TSP Server 二进制文件。

## 前置要求

- Docker Desktop（Windows/Mac）或 Docker Engine（Linux）
- Git

## 构建方法

### Windows

双击运行 `build-linux-binary.bat` 脚本，或在命令行中执行：

```cmd
build-linux-binary.bat
```

### Linux / macOS

确保脚本有执行权限，然后运行：

```bash
chmod +x build-linux-binary.sh
./build-linux-binary.sh
```

## 构建过程

构建脚本会自动执行以下步骤：

1. ✅ 检查 Docker 是否运行
2. ✅ 清理旧的构建产物
3. ✅ 构建 Docker 镜像（基于 Deno 官方镜像）
4. ✅ 在 Docker 容器中编译 Linux x64 二进制文件
5. ✅ 创建发布包（包含二进制文件和 www 目录）
6. ✅ 打包为 `tspserver-linux-x64.tar.gz`
7. ✅ 输出到 `output/` 目录

## 构建产物

成功构建后，会在 `output/` 目录下生成：

```
output/
└── tspserver-linux-x64.tar.gz    # Linux 二进制发布包
```

## 在 Linux 服务器上使用

### 1. 传输文件到 Linux 服务器

使用 scp、sftp 或其他方式将 `tspserver-linux-x64.tar.gz` 传输到目标 Linux 服务器。

```bash
# 示例：使用 scp 传输
scp output/tspserver-linux-x64.tar.gz user@linux-server:/opt/
```

### 2. 解压文件

```bash
# 解压到当前目录
tar -xzf tspserver-linux-x64.tar.gz

# 或解压到指定目录
tar -xzf tspserver-linux-x64.tar.gz -C /opt/
```

### 3. 运行服务器

```bash
# 进入目录
cd tspserver

# 运行服务器（基本用法）
./tspserver --root ./www --port 9000

# 运行服务器（开发模式，启用热重载）
./tspserver --root ./www --port 9000 --dev

# 使用配置文件运行
./tspserver --config config.json
```

### 4. 配置为系统服务（可选）

创建 systemd 服务文件 `/etc/systemd/system/tspserver.service`：

```ini
[Unit]
Description=TSP Server
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/opt/tspserver
ExecStart=/opt/tspserver/tspserver --root ./www --port 9000
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

启用并启动服务：

```bash
sudo systemctl enable tspserver
sudo systemctl start tspserver
sudo systemctl status tspserver
```

## 发布包内容

解压后的目录结构：

```
tspserver/
├── tspserver          # Linux x64 二进制文件（可执行）
├── www/               # 网站根目录
│   ├── index.tsx
│   ├── components/
│   └── ...
├── README.md          # 项目说明
├── BUILD_INFO.txt     # 构建信息（生成时自动添加）
└── deno.json          # Deno 配置文件（如果存在）
```

## 系统要求

- **操作系统**：Linux x86_64（64位）
- **最低内核版本**：Linux 3.2+（glibc 2.17+）
- **已测试的发行版**：
  - Ubuntu 18.04+
  - Debian 10+
  - CentOS 7+
  - Fedora 30+
  - Arch Linux

## 性能优化建议

### 1. 使用生产模式

在生产环境中，不要使用 `--dev` 标志以获得最佳性能。

### 2. 配置反向代理（推荐）

使用 Nginx 作为反向代理：

```nginx
server {
    listen 80;
    server_name your-domain.com;

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

### 3. 使用进程管理器

使用 PM2 或 systemd 保持服务运行并自动重启。

## 故障排除

### 问题：无法执行二进制文件

**错误信息**：`Permission denied` 或 `cannot execute binary file`

**解决方案**：
```bash
chmod +x tspserver
```

### 问题：缺少依赖库

**错误信息**：`error while loading shared libraries: libxyz.so.xxx`

**解决方案**：
- 确保系统是最新的：`sudo apt update && sudo apt upgrade`（Ubuntu/Debian）
- 安装缺少的库：`sudo apt install libc6`（Ubuntu/Debian）

### 问题：端口已被占用

**错误信息**：`AddrInUse` 或 `address already in use`

**解决方案**：
- 更换端口：`./tspserver --root ./www --port 9001`
- 或停止占用端口的进程

## 手动构建（不使用 Docker）

如果你不想使用 Docker，也可以在有 Deno 环境的 Linux 系统上直接编译：

```bash
# 安装 Deno
curl -fsSL https://deno.land/install.sh | sh

# 克隆项目
git clone <your-repo-url>
cd tsp

# 编译二进制文件
deno compile --allow-all --output tspserver --target x86_64-unknown-linux-gnu src/main.ts

# 运行
./tspserver --root ./www --port 9000
```

## 构建信息

构建产物中包含 `BUILD_INFO.txt` 文件，记录了以下信息：

- 构建日期和时间
- Deno 版本
- 目标平台
- Git 提交哈希（如果可用）

## 许可证

请参考项目根目录的 LICENSE 文件。

## 支持

如有问题，请提交 Issue 或 Pull Request。
