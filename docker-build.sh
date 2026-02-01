#!/bin/bash

# TSP Server Docker 镜像构建脚本 (Linux/Mac)
# 自动构建并标记 TSP Server Docker 镜像

set -e

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 配置
IMAGE_NAME="tspserver"
IMAGE_TAG="${1:-latest}"
REGISTRY="${2:-}"
FULL_IMAGE="${REGISTRY}${IMAGE_NAME}:${IMAGE_TAG}"

echo ""
echo "╔═══════════════════════════════════════════════════════╗"
echo "║     TSP Server Docker 镜像构建                        ║"
echo "╚═══════════════════════════════════════════════════════╝"
echo ""

# 检查 Docker 是否运行
echo "🔍 检查 Docker 环境..."
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}错误: Docker 未运行，请先启动 Docker${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Docker 运行中${NC}"
echo ""

# 显示构建信息
echo "📋 构建信息:"
echo "   镜像名称: ${IMAGE_NAME}"
echo "   镜像标签: ${IMAGE_TAG}"
if [ -n "$REGISTRY" ]; then
    echo "   仓库: ${REGISTRY}"
fi
echo ""

# 清理旧镜像（可选）
read -p "是否删除旧的构建缓存？这将释放磁盘空间 (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    echo "🧹 清理旧镜像..."
    docker image prune -f
    echo -e "${GREEN}✓ 清理完成${NC}"
    echo ""
fi

# 构建镜像
echo "🔨 开始构建 Docker 镜像..."
echo "═════════════════════════════════════════════════════════"
echo ""

# 执行构建
if [ -n "$REGISTRY" ]; then
    # 如果指定了仓库，构建并推送
    docker buildx build --platform linux/amd64,linux/arm64 -t ${FULL_IMAGE} --push .
else
    # 只构建本地镜像
    docker build -t ${FULL_IMAGE} .
fi

BUILD_STATUS=$?

echo ""
echo "═════════════════════════════════════════════════════════"
echo ""

if [ $BUILD_STATUS -eq 0 ]; then
    echo -e "${GREEN}╔══════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║     🎉 构建成功！                            ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════╝${NC}"
    echo ""
    echo "📦 镜像信息:"
    echo "   本地镜像: ${FULL_IMAGE}"
    echo ""
    echo "💡 使用方法:"
    echo ""
    echo "   运行容器:"
    echo "     docker run -d --name tspserver -p 9000:9000 ${FULL_IMAGE}"
    echo ""
    echo "   带环境变量运行:"
    echo "     docker run -d --name tspserver -p 9000:9000 \\"
    echo "       -e TSP_PORT=8080 \\"
    echo "       ${FULL_IMAGE}"
    echo ""
    echo "   挂载自定义配置:"
    echo "     docker run -d --name tspserver -p 9000:9000 \\"
    echo "       -v /path/to/config:/app/config \\"
    echo "       ${FULL_IMAGE} -- --root /app/www --port 9000"
    echo ""

    # 显示镜像大小
    IMAGE_SIZE=$(docker images ${IMAGE_NAME}:${IMAGE_TAG} --format "{{.Size}}")
    echo "   镜像大小: ${IMAGE_SIZE}"
    echo ""
else
    echo -e "${RED}╔══════════════════════════════════════════╗${NC}"
    echo -e "${RED}║     ❌ 构建失败！                            ║${NC}"
    echo -e "${RED}╚══════════════════════════════════════════╝${NC}"
    echo ""
    exit 1
fi
# 保存构建信息
echo "$(date)" > .docker_build_info
echo "Image: ${FULL_IMAGE}" >> .docker_build_info
echo "Size: ${IMAGE_SIZE}" >> .docker_build_info
