#!/bin/bash
#
# Docker 服务重启脚本
# 快速重启 MySQL 和 Redis 容器
#

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     重启 Docker 测试服务                    ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
echo ""

# 检查 docker-compose
if command -v docker-compose &> /dev/null; then
    DOCKER_COMPOSE="docker-compose"
elif docker compose version > /dev/null 2>&1; then
    DOCKER_COMPOSE="docker compose"
else
    echo -e "${RED}❌ 错误: 未找到 docker-compose${NC}"
    exit 1
fi

echo -e "${YELLOW}🔄 重启服务...${NC}"
echo ""

# 重启服务
$DOCKER_COMPOSE restart

echo ""
echo -e "${GREEN}✅ 服务已重启！${NC}"
echo ""
echo -e "${YELLOW}📊 服务状态：${NC}"
$DOCKER_COMPOSE ps
