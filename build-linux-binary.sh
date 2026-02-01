#!/bin/bash

# Linux 二进制文件构建脚本
# 使用 Docker 构建跨平台的 Linux 二进制文件

set -e

echo "╔════════════════════════════════════════╗"
echo "║   构建 Linux x64 二进制文件            ║"
echo "╚════════════════════════════════════════╝"
echo ""

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# 检查 Docker 是否运行
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}错误: Docker 未运行，请先启动 Docker${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Docker 运行中${NC}"
echo ""

# 清理旧的构建产物
echo -e "${YELLOW}清理旧的构建产物...${NC}"
rm -f tspserver-linux-x64.tar.gz
echo -e "${GREEN}✓ 清理完成${NC}"
echo ""

# 构建 Docker 镜像
echo -e "${YELLOW}构建 Docker 镜像...${NC}"
docker build -t tspserver-linux-builder .
echo -e "${GREEN}✓ Docker 镜像构建完成${NC}"
echo ""

# 运行容器并复制构建产物
echo -e "${YELLOW}生成二进制文件并打包...${NC}"
docker create --name temp-build tspserver-linux-builder >/dev/null 2>&1
docker cp temp-build:/output/tspserver-linux-x64.tar.gz output/tspserver-linux-x64.tar.gz
docker rm temp-build >/dev/null 2>&1
echo -e "${GREEN}✓ 打包完成${NC}"
echo ""

# 检查构建产物
if [ -f "output/tspserver-linux-x64.tar.gz" ]; then
    FILE_SIZE=$(du -h output/tspserver-linux-x64.tar.gz | cut -f1)
    echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║   构建成功！                            ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "文件位置: ${YELLOW}output/tspserver-linux-x64.tar.gz${NC}"
    echo -e "文件大小: ${YELLOW}${FILE_SIZE}${NC}"
    echo ""
    echo "使用方法:"
    echo "  1. 解压: tar -xzf output/tspserver-linux-x64.tar.gz"
    echo "  2. 进入目录: cd tspserver"
    echo "  3. 运行: ./tspserver --root ./www --port 9000"
    echo ""
else
    echo -e "${RED}错误: 构建失败，找不到输出文件${NC}"
    exit 1
fi
