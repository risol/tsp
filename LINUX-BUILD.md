# TSP Server Linux 二进制构建指南

## 推荐方法：在 Linux 上直接编译

### 为什么推荐在 Linux 上编译？

**优势**：
- ✅ **避免跨平台编译问题**：Linux 上编译 Linux 目标，无兼容性问题
- ✅ **更简单可靠**：不需要 Docker 或复杂的构建脚本
- ✅ **配置正确**：Deno 的 JSX 配置正常工作，npm 依赖正确打包
- ✅ **性能更好**：本地编译不需要 Docker 开销，编译速度更快
- ✅ **调试方便**：可以直接运行测试，遇到问题更容易排查

**跨平台编译的问题**：
在 Windows 上跨平台编译 Linux 二进制时，`@deno/loader` 的行为不一致，导致 JSX 运行时配置（`jsxImportSource: "preact"`）无法正确传递，生成的代码使用 `React.createElement` 而不是 Preact 的 `h` 函数，造成运行时错误 "React is not defined"。

### 编译步骤

#### 1. 安装 Deno

```bash
curl -fsSL https://deno.land/install.sh | sh
```

安装后，将 Deno 添加到 PATH（如果需要）：
```bash
export PATH="$HOME/.deno/bin:$PATH"
```

#### 2. 获取源码

**方法 A：使用 Git 克隆**
```bash
git clone <repository-url> /opt/tspserver-source
cd /opt/tspserver-source
```

**方法 B：上传源码压缩包**
```bash
# 在 Windows 上打包源码（排除 node_modules、.cache 等）
tar -czf tspserver-source.tar.gz --exclude='.cache' --exclude='node_modules' --exclude='tspserver' --exclude='tspserver.exe' .

# 上传到 Linux 服务器
scp tspserver-source.tar.gz user@server:/opt/

# 在 Linux 服务器上解压
cd /opt
tar -xzf tspserver-source.tar.gz
cd tspserver-source
```

#### 3. 编译 Linux 二进制

**方法 A：使用 deno task（推荐）**
```bash
deno task compile:linux
```

**方法 B：直接使用 deno compile**
```bash
deno compile --allow-net --allow-read --allow-write --allow-sys --allow-env \
  --target x86_64-unknown-linux-gnu \
  --output tspserver \
  src/main.ts
```

#### 4. 运行测试

```bash
# 启动服务器
./tspserver --root ./www --port 9000

# 在另一个终端测试
curl http://localhost:9000/

# 或使用浏览器访问
# http://localhost:9000/
```

如果看到正常的 HTML 页面，说明编译成功！

#### 5. 准备部署包

```bash
# 创建输出目录
mkdir -p dist

# 复制二进制文件
cp tspserver dist/

# 复制网站文件
cp -r www dist/

# 复制配置文件（如果存在）
cp config.jsonc dist/ 2>/dev/null || true
cp .env.example dist/ 2>/dev/null || true

# 打包
cd dist
tar -czf tspserver-linux-x64.tar.gz *
cd ..
```

### 系统要求

- **操作系统**：Linux x86_64（如 Ubuntu 20.04+, CentOS 7+, Debian 10+）
- **glibc**：2.17 或更高版本
- **内存**：至少 512MB 可用内存
- **磁盘空间**：至少 200MB 可用空间

---

## 备用方法：在 Windows 上构建（需要 Docker）

> ⚠️ **注意**：此方法用于在 Windows 上跨平台编译 Linux 二进制，但可能遇到 JSX 运行时问题。**强烈推荐使用上面的"在 Linux 上直接编译"方法**。

### 前置条件

如果需要在 Windows 上构建 Linux 二进制，请确保：

1. **已安装 Docker Desktop** 并正在运行
2. **网络连接正常**（需要拉取 Deno Docker 镜像）
3. **有足够的磁盘空间**（至少 2GB 可用空间）

### 构建步骤

```powershell
# 在项目根目录运行
.\build-linux-binary.ps1
```

### 构建脚本说明

`build-linux-binary.ps1` 脚本会自动执行以下步骤：

1. ✅ 检查 Docker 是否运行
2. ✅ 检测本地 Deno 版本
3. ✅ 清理旧的构建产物
4. ✅ 拉取 Deno Docker 镜像（如果需要）
5. ✅ 在 Docker 容器中编译 Linux 二进制
6. ✅ 打包部署文件（www、配置文件、systemd 服务等）
7. ✅ 生成 `output/tspserver-linux-x64.tar.gz`

### 已知问题和解决方案

**问题**：运行时出现 "React is not defined" 错误

**原因**：跨平台编译时，`@deno/loader` 可能无法正确传递 JSX 配置（`jsxImportSource: "preact"`），导致生成的代码使用 `React.createElement` 而不是 Preact 的 `h` 函数。

**解决方案**：
- ✅ **最佳方案**：在 Linux 服务器上直接编译（见上面的"推荐方法"）
- ⚠️ **注意**：此脚本无法解决跨平台编译的 JSX 问题，仅用于特殊情况

---

## 构建产物

## 部署方法

> 以下部署方法假设你已经在 Linux 上编译好了二进制文件，并准备好了 `tspserver-linux-x64.tar.gz` 部署包。

### 部署包内容

`dist/tspserver-linux-x64.tar.gz` 包含以下文件：

#### 核心文件
- `tspserver` - Linux x64 可执行文件
- `www/` - 网站文件目录
- `config.jsonc` - 配置文件（支持注释）

#### 服务管理文件
- `tspserver.service` - systemd 服务文件
- `install.sh` - 自动安装脚本
- `run.sh` - 快速启动脚本
- `stop.sh` - 快速停止脚本

#### 其他文件
- `.env.example` - 环境变量示例

### 方法 1：使用安装脚本（推荐）

此方法假设你的部署包中包含了 `install.sh` 脚本（`build-linux-binary.ps1` 会自动生成）。

1. 上传部署包到 Linux 服务器：
```bash
scp dist/tspserver-linux-x64.tar.gz user@server:/tmp/
```

2. SSH 登录服务器并解压：
```bash
cd /tmp
tar -xzf tspserver-linux-x64.tar.gz
cd dist
```

**注意**：如果你在 Linux 上编译时使用了不同的输出目录，请相应调整路径。

3. 运行安装脚本：
```bash
sudo ./install.sh
```

4. 启动服务：
```bash
sudo systemctl start tspserver
sudo systemctl enable tspserver
```

### 方法 2：手动安装

此方法适用于部署包中没有安装脚本的情况，或者你需要自定义安装流程。

1. 解压文件：
```bash
tar -xzf tspserver-linux-x64.tar.gz
cd dist
```

2. 创建目录和用户：
```bash
sudo mkdir -p /opt/tspserver
sudo useradd -r -s /sbin/nologin tspserver 2>/dev/null || true
```

3. 复制文件：
```bash
sudo cp -r tspserver www /opt/tspserver/
sudo cp config.jsonc /opt/tspserver/ 2>/dev/null || true
```

4. 创建 systemd 服务文件：
```bash
sudo tee /etc/systemd/system/tspserver.service > /dev/null <<'EOF'
[Unit]
Description=TSP Server
After=network.target

[Service]
Type=simple
User=tspserver
Group=tspserver
WorkingDirectory=/opt/tspserver
ExecStart=/opt/tspserver/tspserver --root /opt/tspserver/www --port 9000
Restart=always
RestartSec=10

Environment="TSP_PORT=9000"
Environment="TSP_ROOT=/opt/tspserver/www"
Environment="TSP_MODE=production"

NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/tspserver

MemoryMax=512M
CPUQuota=50%

StandardOutput=journal
StandardError=journal
SyslogIdentifier=tspserver

[Install]
WantedBy=multi-user.target
EOF
```

5. 设置权限：
```bash
sudo chown -R tspserver:tspserver /opt/tspserver
sudo chmod +x /opt/tspserver/tspserver
sudo chmod -R 755 /opt/tspserver/www
```

6. 重载 systemd 并启动服务：
```bash
sudo systemctl daemon-reload
sudo systemctl start tspserver
sudo systemctl enable tspserver
```

---

## 快速启动（开发/测试）

如果只是想快速测试编译后的二进制文件，不需要安装为系统服务：

```bash
# 直接运行
./tspserver --root ./www --port 9000

# 或使用配置文件
./tspserver --config config.jsonc

# 开发模式（支持热重载）
./tspserver --root ./www --port 9000 --dev
```

---

## 生产环境部署（系统服务）

### 服务管理

```bash
# 查看服务状态
systemctl status tspserver

# 启动服务
sudo systemctl start tspserver

# 停止服务
sudo systemctl stop tspserver

# 重启服务
sudo systemctl restart tspserver

# 查看日志
journalctl -u tspserver -f

# 开机自启
sudo systemctl enable tspserver

# 禁用自启
sudo systemctl disable tspserver
```

## 配置

配置文件位置：`/opt/tspserver/config.jsonc`

```jsonc
{
  // 服务器端口（默认：9000）
  "port": 9000,

  // 网站根目录
  "root": "./www",

  // 开发模式（生产环境设为 false）
  "dev": false,

  // 文件管理器配置
  "fileManager": {
    "enabled": true,
    "password": "your_secure_password",
    "path": "/__filemanager"
  }
}
```

修改配置后重启服务：
```bash
sudo systemctl restart tspserver
```

## 日志

日志文件位置：`/var/log/tspserver/`

查看实时日志：
```bash
sudo journalctl -u tspserver -f
```

查看最近的日志：
```bash
sudo journalctl -u tspserver -n 50
```

## 防火墙配置

如果使用了防火墙，需要开放 9000 端口：

```bash
# Ubuntu/Debian
sudo ufw allow 9000/tcp

# CentOS/RHEL
sudo firewall-cmd --permanent --add-port=9000/tcp
sudo firewall-cmd --reload
```

## 故障排查

### 服务无法启动

1. 检查服务状态：
```bash
sudo systemctl status tspserver
```

2. 查看详细错误：
```bash
sudo journalctl -u tspserver -n 50
```

3. 检查配置文件语法：
```bash
/opt/tspserver/tspserver --config /opt/tspserver/config.jsonc --check
```

### 无法访问网站

1. 检查服务是否运行：
```bash
sudo systemctl status tspserver
```

2. 检查端口是否监听：
```bash
sudo ss -tlnp | grep 9000
```

3. 检查防火墙：
```bash
sudo ufw status
```

### 权限问题

确保文件权限正确：
```bash
sudo chown -R tspserver:tspserver /opt/tspserver
sudo chmod +x /opt/tspserver/tspserver
sudo chmod -R 755 /opt/tspserver/www
```

## 卸载

### 停止并禁用服务：
```bash
sudo systemctl stop tspserver
sudo systemctl disable tspserver
```

### 完全卸载：
```bash
# 停止服务
sudo systemctl stop tspserver
sudo systemctl disable tspserver

# 删除文件
sudo rm -rf /opt/tspserver
sudo rm /etc/systemd/system/tspserver.service

# 重新加载 systemd
sudo systemctl daemon-reload

# 删除用户（可选）
sudo userdel tspserver
sudo groupdel tspserver
```

## 注意事项

1. **安全建议**
   - 修改默认密码（文件管理器）
   - 使用 HTTPS（需要配置反向代理，如 Nginx）
   - 定期更新二进制文件
   - 限制文件管理器的访问权限

2. **性能优化**
   - 根据服务器资源调整内存限制（修改 systemd 服务中的 `MemoryMax`）
   - 配置反向代理（如 Nginx）以处理静态文件和 SSL
   - 启用 gzip 压缩（在反向代理层）
   - 考虑使用多进程部署（如 PM2 或 systemd 实例）

3. **监控**
   - 设置日志轮转（systemd 默认会管理日志）
   - 配置监控告警（如 Prometheus + Grafana）
   - 定期检查服务状态和磁盘空间
   - 监控内存和 CPU 使用情况

4. **备份**
   - 定期备份 `www/` 目录
   - 备份配置文件 `config.jsonc`
   - 如果使用数据库，定期备份数据库

5. **更新**
   - 更新前先停止服务：`sudo systemctl stop tspserver`
   - 备份当前版本
   - 替换二进制文件和 `www/` 目录
   - 重启服务：`sudo systemctl start tspserver`
   - 检查日志确保没有错误：`sudo journalctl -u tspserver -n 50`
