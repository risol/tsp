#!/bin/bash
#
# Docker 测试数据清理脚本
# 删除所有容器和数据卷，完全清理测试数据
#

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${RED}╔════════════════════════════════════════╗${NC}"
echo -e "${RED}║     ⚠️  警告：删除所有测试数据 ⚠️        ║${NC}"
echo -e "${RED}╚════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}此操作将会：${NC}"
echo -e "${RED}  ✓ 停止所有容器${NC}"
echo -e "${RED}  ✓ 删除所有容器${NC}"
echo -e "${RED}  ✓ 删除所有数据卷（MySQL和Redis数据）${NC}"
echo ""
echo -e "${YELLOW}以下数据将会丢失：${NC}"
echo -e "  • MySQL 数据库（test_db）及所有表和数据"
echo -e "  • Redis 缓存中的所有数据"
echo -e "  • 所有已创建的用户、会话、文章等"
echo ""
echo -e "${BLUE}提示：数据删除后，重新启动服务将重新初始化测试数据${NC}"
echo ""

# 确认操作
echo -e "${YELLOW}确认要删除所有测试数据吗？${NC}"
read -p "请输入 'yes' 确认删除: " confirm

if [ "$confirm" != "yes" ]; then
    echo -e "${GREEN}✓ 操作已取消${NC}"
    exit 0
fi

# 检查 docker-compose
if command -v docker-compose &> /dev/null; then
    DOCKER_COMPOSE="docker-compose"
elif docker compose version > /dev/null 2>&1; then
    DOCKER_COMPOSE="docker compose"
else
    echo -e "${RED}❌ 错误: 未找到 docker-compose${NC}"
    exit 1
fi

echo ""
echo -e "${YELLOW}正在清理...${NC}"
echo ""

# 删除容器和数据卷
echo -e "${YELLOW}📦 停止并删除容器...${NC}"
$DOCKER_COMPOSE down -v

echo ""
echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     ✓ 清理完成！                        ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
echo ""
echo -e "${GREEN}所有测试数据已删除${NC}"
echo ""
echo -e "${YELLOW}💡 后续操作：${NC}"
echo ""
echo -e "${BLUE}1. 重新启动服务（将重新初始化测试数据）：${NC}"
echo "   ./docker-start.sh"
echo ""
echo -e "${BLUE}2. 或完全不使用 Docker：${NC}"
echo "   无需操作"
echo ""
