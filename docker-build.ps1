# TSP Server Docker 镜像构建脚本 (Windows)
# 自动构建并标记 TSP Server Docker 镜像

$ErrorActionPreference = "Stop"

# 配置
$IMAGE_NAME = "tspserver"
$IMAGE_TAG = if ($args.Count -gt 0) { $args[0] } else { "latest" }
$REGISTRY = if ($args.Count -gt 1) { $args[1] } else { "" }

if ($REGISTRY -ne "") {
    $FULL_IMAGE = "$REGISTRY${IMAGE_NAME}:${IMAGE_TAG}"
} else {
    $FULL_IMAGE = "${IMAGE_NAME}:${IMAGE_TAG}"
}

Write-Host ""
Write-Host "==============================================="
Write-Host "  TSP Server Docker 镜像构建"
Write-Host "==============================================="
Write-Host ""

# 检查 Docker 是否运行
Write-Host "检查 Docker 环境..."
try {
    docker info | Out-Null
    Write-Host "[OK] Docker 运行中" -ForegroundColor Green
} catch {
    Write-Host "错误: Docker 未运行，请先启动 Docker" -ForegroundColor Red
    Read-Host "按 Enter 键退出"
    exit 1
}
Write-Host ""

# 显示构建信息
Write-Host "构建信息:"
Write-Host "  镜像名称: $IMAGE_NAME"
Write-Host "  镜像标签: $IMAGE_TAG"
if ($REGISTRY -ne "") {
    Write-Host "  仓库: $REGISTRY"
}
Write-Host ""

# 询问是否清理
$cleanup = Read-Host "是否删除旧的构建缓存？这将释放磁盘空间 (y/N)"
Write-Host ""
if ($cleanup -eq "y") {
    Write-Host ""
    Write-Host "清理旧镜像..."
    docker image prune -f | Out-Null
    Write-Host "[OK] 清理完成" -ForegroundColor Green
    Write-Host ""
}

# 构建镜像
Write-Host "开始构建 Docker 镜像..."
Write-Host "==============================================="
Write-Host ""

# 执行构建
if ($REGISTRY -ne "") {
    # 如果指定了仓库，构建并推送
    docker buildx build --platform linux/amd64,linux/arm64 -t $FULL_IMAGE --push .
    $BUILD_STATUS = $LASTEXITCODE
} else {
    # 只构建本地镜像
    docker build -t $FULL_IMAGE .
    $BUILD_STATUS = $LASTEXITCODE
}

Write-Host ""
Write-Host "==============================================="
Write-Host ""

if ($BUILD_STATUS -eq 0) {
    # 获取镜像大小
    $imageSize = docker images $FULL_IMAGE --format "{{.Size}}"

    Write-Host "==============================================="
    Write-Host "  构建成功！"
    Write-Host "==============================================="
    Write-Host ""
    Write-Host "镜像信息:"
    Write-Host "  本地镜像: $FULL_IMAGE"
    Write-Host "  镜像大小: $imageSize"
    Write-Host ""
    Write-Host "使用方法:"
    Write-Host ""
    Write-Host "  运行容器:"
    Write-Host "    docker run -d --name tspserver -p 9000:9000 $FULL_IMAGE"
    Write-Host ""
    Write-Host "  带环境变量运行:"
    Write-Host "    docker run -d --name tspserver -p 9000:9000 `"
    Write-Host "      -e TSP_PORT=8080 `"
    Write-Host "      $FULL_IMAGE"
    Write-Host ""
    Write-Host "  挂载自定义配置:"
    Write-Host "    docker run -d --name tspserver -p 9000:9000 `"
    Write-Host "      -v C:/path/to/config:/app/config `"
    Write-Host "      $FULL_IMAGE --root /app/www --port 9000"
    Write-Host ""
} else {
    Write-Host "==============================================="
    Write-Host "  构建失败！"
    Write-Host "==============================================="
    Write-Host ""
    Read-Host "按 Enter 键退出"
    exit 1
}

Read-Host "按 Enter 键退出"
