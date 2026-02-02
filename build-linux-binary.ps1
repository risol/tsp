# Linux Binary Build Script (Windows)
#
# 使用 Docker 容器在 Windows 上构建 Linux 二进制文件
#
# 使用方法：
#   1. 确保 Docker Desktop 正在运行
#   2. 运行此脚本：.\build-linux.ps1
#   3. 生成的文件在 output\tspserver-linux-x64.tar.gz

$ErrorActionPreference = "Stop"

# 显示标题
Write-Host ""
Write-Host "============================================"
Write-Host "  TSP Server - Linux Build Tool"
Write-Host "============================================"
Write-Host ""

# 检查 Docker 是否运行
Write-Host "Checking Docker..."
try {
    $null = docker info 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Docker not running"
    }
    Write-Host "[OK] Docker is running" -ForegroundColor Green
}
catch {
    Write-Host "[ERROR] Docker is not running. Please start Docker Desktop" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}
Write-Host ""

# 获取 Deno 版本
Write-Host "Detecting Deno version..."
$denoVersion = "2.6.6"
try {
    $denoVersionOutput = deno --version 2>&1
    if ($LASTEXITCODE -eq 0) {
        $lines = $denoVersionOutput -split "`n"
        $firstLine = $lines[0].Trim()
        if ($firstLine -match 'deno (\d+\.\d+\.\d+)') {
            $denoVersion = $matches[1]
            Write-Host "[OK] Detected Deno version: $denoVersion" -ForegroundColor Green
        }
    }
} catch {
    Write-Host "[WARN] Using default version: $denoVersion" -ForegroundColor Yellow
}
Write-Host ""

# 清理旧的构建产物
Write-Host "Cleaning old build artifacts..."
if (Test-Path "output") {
    Remove-Item "output" -Recurse -Force -ErrorAction SilentlyContinue
}
if (Test-Path "dist") {
    Remove-Item "dist" -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -ItemType Directory -Path "output" -Force | Out-Null
Write-Host "[OK] Clean completed" -ForegroundColor Green
Write-Host ""

# 拉取 Deno 镜像
Write-Host "Pulling Deno Docker image..."
docker pull denoland/deno:$denoVersion | Out-Host
Write-Host "[OK] Image ready" -ForegroundColor Green
Write-Host ""

# 步骤 1: 在容器中运行 build_tool.ts 构建完整的 dist 目录
Write-Host "Step 1: Building dist directory..."
Write-Host "----------------------------------------"
Write-Host "Running: deno run build"
Write-Host "This may take a few minutes..."
Write-Host ""

$env:MSYS_NO_PATHCONV = "1"
try {
    docker run --rm `
        -v "${PWD}:/workspace" `
        -w /workspace `
        -e DENO_DIR=/workspace/.deno `
        denoland/deno:$denoVersion `
        run --allow-all src/build_tool.ts
} finally {
    Remove-Item Env:\MSYS_NO_PATHCONV -ErrorAction SilentlyContinue
}

if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "[ERROR] Build failed (exit code: $LASTEXITCODE)" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

Write-Host ""
Write-Host "[OK] dist directory built" -ForegroundColor Green
Write-Host ""

# 验证 dist 目录
if (-not (Test-Path "dist\tspserver")) {
    Write-Host "[ERROR] tspserver binary not found in dist directory" -ForegroundColor Red
    Read-Host "Press Enter to exit"
    exit 1
}

$binarySizeBytes = (Get-Item "dist\tspserver").Length
$binarySizeMB = [math]::Round($binarySizeBytes / 1MB, 2)
Write-Host "Binary size: $binarySizeMB MB"
Write-Host ""

# 步骤 2: 创建服务管理脚本
Write-Host "Step 2: Creating service management scripts..."
Write-Host "----------------------------------------"

# 创建 systemd 服务文件
$serviceContent = '[Unit]
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
WantedBy=multi-user.target'
$serviceContent | Out-File -FilePath "dist\tspserver.service" -Encoding UTF8

# 创建 install.sh
$installScriptContent = '#!/bin/bash
set -e

echo "============================================"
echo "  TSP Server Installation"
echo "============================================"
echo ""

if [ "$EUID" -ne 0 ]; then
    echo "Error: This script must be run as root"
    exit 1
fi

INSTALL_DIR="/opt/tspserver"
echo "Installation directory: $INSTALL_DIR"

# Create user
if ! id tspserver >/dev/null 2>&1; then
    echo "Creating user and group..."
    groupadd -r tspserver 2>/dev/null || true
    useradd -r -g tspserver -s /sbin/nologin tspserver 2>/dev/null || true
    echo "[OK] User created"
fi

# Create directories
echo "Creating directories..."
mkdir -p "$INSTALL_DIR"
mkdir -p "$INSTALL_DIR/www"
mkdir -p "/var/log/tspserver"

# Copy files
echo "Copying files..."
cp tspserver "$INSTALL_DIR/"
cp -r www "$INSTALL_DIR/"

if [ -f config.jsonc ]; then
    cp config.jsonc "$INSTALL_DIR/"
    echo "[OK] Config copied"
fi

if [ -f deno.json ]; then
    cp deno.json "$INSTALL_DIR/"
    echo "[OK] Deno config copied"
fi

if [ -f tspserver.service ]; then
    cp tspserver.service /etc/systemd/system/
    systemctl daemon-reload
    echo "[OK] systemd service installed"
fi

# Set permissions
echo "Setting permissions..."
chown -R tspserver:tspserver "$INSTALL_DIR"
chmod +x "$INSTALL_DIR/tspserver"
chmod -R 755 "$INSTALL_DIR/www"
echo "[OK] Permissions set"

echo ""
echo "============================================"
echo "  Installation Complete!"
echo "============================================"
echo ""
echo "Service management:"
echo "  Start:   systemctl start tspserver"
echo "  Stop:    systemctl stop tspserver"
echo "  Restart: systemctl restart tspserver"
echo "  Status:  systemctl status tspserver"
echo "  Enable:  systemctl enable tspserver"
echo ""
echo "Configuration:"
echo "  Config file: $INSTALL_DIR/config.jsonc"
echo "  Logs: /var/log/tspserver/"
echo "  Website: $INSTALL_DIR/www"
echo ""
echo "Default port: 9000"
echo "User: tspserver"
echo ""'
$installScriptContent | Out-File -FilePath "dist\install.sh" -Encoding UTF8

# 创建 uninstall.sh
$uninstallScriptContent = '#!/bin/bash
set -e

echo "============================================"
echo "  TSP Server Uninstallation"
echo "============================================"
echo ""

if [ "$EUID" -ne 0 ]; then
    echo "Error: This script must be run as root"
    exit 1
fi

INSTALL_DIR="/opt/tspserver"
echo "Installation directory: $INSTALL_DIR"
echo ""

# Confirm uninstall
read -p "Are you sure you want to uninstall TSP Server? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Uninstall cancelled"
    exit 0
fi

# Stop service
echo "Stopping service..."
if systemctl is-active --quiet tspserver; then
    systemctl stop tspserver
    echo "[OK] Service stopped"
fi

# Disable service
if systemctl is-enabled --quiet tspserver; then
    systemctl disable tspserver
    echo "[OK] Autostart disabled"
fi

# Remove systemd service
if [ -f /etc/systemd/system/tspserver.service ]; then
    rm -f /etc/systemd/system/tspserver.service
    systemctl daemon-reload
    echo "[OK] systemd service removed"
fi

# Remove installation directory
if [ -d "$INSTALL_DIR" ]; then
    rm -rf "$INSTALL_DIR"
    echo "[OK] Installation directory removed"
fi

# Remove log directory
if [ -d "/var/log/tspserver" ]; then
    rm -rf "/var/log/tspserver"
    echo "[OK] Log directory removed"
fi

# Ask to remove user
read -p "Remove tspserver user? (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    userdel tspserver 2>/dev/null || true
    groupdel tspserver 2>/dev/null || true
    echo "[OK] User removed"
fi

echo ""
echo "============================================"
echo "  Uninstallation Complete!"
echo "============================================"
echo ""'
$uninstallScriptContent | Out-File -FilePath "dist\uninstall.sh" -Encoding UTF8

# 创建 start.sh
$startScriptContent = '#!/bin/bash
cd "$(dirname "$0")"
./tspserver "$@"'
$startScriptContent | Out-File -FilePath "dist\start.sh" -Encoding UTF8

# 创建 stop.sh
$stopScriptContent = '#!/bin/bash
echo "Stopping TSP Server..."
if systemctl is-active --quiet tspserver; then
    systemctl stop tspserver
    echo "[OK] TSP Server stopped"
else
    echo "[INFO] TSP Server is not running or not installed as a system service"
fi'
$stopScriptContent | Out-File -FilePath "dist\stop.sh" -Encoding UTF8

Write-Host "[OK] Service management scripts created" -ForegroundColor Green
Write-Host ""

# 步骤 3: 手动测试说明
Write-Host "Step 3: Testing instructions..."
Write-Host "----------------------------------------"
Write-Host "[INFO] Binary successfully built"
Write-Host ""
Write-Host "To test the binary on Linux:"
Write-Host "  1. Transfer output\tspserver-linux-x64.tar.gz to Linux server"
Write-Host "  2. Extract: tar -xzf tspserver-linux-x64.tar.gz"
Write-Host "  3. Run: DENO_DIR=./.deno ./tspserver --root ./www --port 9000"
Write-Host "  4. Wait 20-30 seconds (TSX compilation)"
Write-Host "  5. Test: curl http://localhost:9000/"
Write-Host ""

# 步骤 4: 打包 dist 目录为 tar.gz
Write-Host "Step 4: Creating tar.gz package..."
Write-Host "----------------------------------------"
Write-Host "Creating Linux-compatible tar.gz using Docker..."
Write-Host ""

$env:MSYS_NO_PATHCONV = "1"
try {
    docker run --rm `
        -v "${PWD}:/workspace" `
        -w /workspace `
        denoland/deno:$denoVersion `
        sh -c "cd dist && tar -czf ../output/tspserver-linux-x64.tar.gz *"
} finally {
    Remove-Item Env:\MSYS_NO_PATHCONV -ErrorAction SilentlyContinue
}

Write-Host "[OK] Package created" -ForegroundColor Green
Write-Host ""

# 步骤 5: 清理临时文件
Write-Host "Step 5: Cleaning temporary files..."
Write-Host "----------------------------------------"
Remove-Item -Path "dist" -Recurse -Force -ErrorAction SilentlyContinue
Write-Host "[OK] Cleanup completed" -ForegroundColor Green
Write-Host ""

# 检查构建产物
Write-Host "============================================"
Write-Host "  Build Successful!"
Write-Host "============================================"
Write-Host ""

if (Test-Path "output\tspserver-linux-x64.tar.gz") {
    $fileInfo = Get-Item "output\tspserver-linux-x64.tar.gz"
    $fileSizeMB = [math]::Round($fileInfo.Length / 1MB, 2)

    Write-Host "Package Info:"
    Write-Host "  File: output\tspserver-linux-x64.tar.gz"
    Write-Host "  Size: $fileSizeMB MB"
    Write-Host ""

    Write-Host "Package Contents:"
    Write-Host "  - tspserver              (Linux x64 binary)"
    Write-Host "  - www/                   (website files)"
    Write-Host "  - .deno/                 (Deno cache directory)"
    Write-Host "  - config.jsonc           (configuration file)"
    Write-Host "  - README.md              (usage documentation)"
    Write-Host "  - tspserver.service      (systemd service file)"
    Write-Host "  - install.sh             (installation script)"
    Write-Host "  - uninstall.sh           (uninstallation script)"
    Write-Host "  - start.sh               (start script)"
    Write-Host "  - stop.sh                (stop script)"
    Write-Host ""

    Write-Host "Linux Deployment:"
    Write-Host ""
    Write-Host "Method 1: Using install script (Recommended)"
    Write-Host "  1. Transfer output\tspserver-linux-x64.tar.gz to Linux server"
    Write-Host "  2. Extract: tar -xzf tspserver-linux-x64.tar.gz"
    Write-Host "  3. Run install: sudo ./install.sh"
    Write-Host "  4. Start service: systemctl start tspserver"
    Write-Host "  5. Enable autostart: systemctl enable tspserver"
    Write-Host ""

    Write-Host "Method 2: Manual installation"
    Write-Host "  1. Extract tar.gz file"
    Write-Host "  2. Copy to /opt/tspserver:"
    Write-Host "     sudo mkdir -p /opt/tspserver"
    Write-Host "     sudo cp -r tspserver www .deno /opt/tspserver/"
    Write-Host "     sudo cp tspserver.service /etc/systemd/system/"
    Write-Host "  3. Reload systemd: sudo systemctl daemon-reload"
    Write-Host "  4. Start service: sudo systemctl start tspserver"
    Write-Host "     sudo systemctl enable tspserver"
    Write-Host ""

    Write-Host "Service Management:"
    Write-Host "  Status:  systemctl status tspserver"
    Write-Host "  Start:   systemctl start tspserver"
    Write-Host "  Stop:    systemctl stop tspserver"
    Write-Host "  Restart: systemctl restart tspserver"
    Write-Host "  Logs:    journalctl -u tspserver -f"
    Write-Host ""

    Write-Host "Configuration:"
    Write-Host "  Config file:   /opt/tspserver/config.jsonc"
    Write-Host "  Website:       /opt/tspserver/www"
    Write-Host "  Log directory: /var/log/tspserver/"
    Write-Host "  Default port:  9000"
    Write-Host "  User:          tspserver"
    Write-Host ""

    Write-Host "Quick Start (without systemd):"
    Write-Host "  cd /opt/tspserver"
    Write-Host "  ./start.sh"
    Write-Host ""
}
else {
    Write-Host "[ERROR] Build failed, output file not found" -ForegroundColor Red
    Write-Host ""
    Write-Host "The file 'output\tspserver-linux-x64.tar.gz' was not created"
    Write-Host ""
    Read-Host "Press Enter to exit"
    exit 1
}

Write-Host "============================================"
