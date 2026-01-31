#!/bin/bash
#
# Docker 服务停止脚本
# 用于停止并清理 MySQL 和 Redis 容器
#

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     停止 Docker 测试服务                    ║${NC}"
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

# 检查容器是否运行
if $DOCKER_COMPOSE ps | grep -q "Up"; then
    echo -e "${YELLOW}⏹ 停止服务...${NC}"
    $DOCKER_COMPOSE down
    echo -e "${GREEN}✅ 服务已停止${NC}"
else
    echo -e "${YELLOW}⚠️  没有运行中的服务${NC}"
fi

# 询问是否删除数据
echo ""
echo -e "${YELLOW}🗑️  是否要删除数据卷？（所有数据将丢失）${NC}"
echo -n "请输入 'yes' 确认删除，或按 Enter 取消: "
read -r answer

if [ "$answer" = "yes" ]; then
    echo ""
    echo -e "${YELLOW}删除数据卷...${NC}"
    $DOCKER_COMPOSE down -v
    echo -e "${GREEN}✅ 数据卷已删除${NC}"
else
    echo -e "${GREEN}✓ 数据卷已保留${NC}"
fi

echo ""
echo -e "${GREEN}✓ 操作完成${NC}"
