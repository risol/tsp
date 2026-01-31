#!/bin/bash
#
# Docker 服务连接测试脚本
# 验证 MySQL 和 Redis 服务是否正常运行
#

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     测试 Docker 服务连接                     ║${NC}"
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

# 检查容器状态
echo -e "${YELLOW}📊 检查容器状态...${NC}"
if ! $DOCKER_COMPOSE ps | grep -q "Up"; then
    echo -e "${RED}❌ 错误: 容器未运行${NC}"
    echo "请先运行: ./docker-start.sh"
    exit 1
fi
echo -e "${GREEN}✓ 容器运行中${NC}"
echo ""

# 测试 MySQL 连接
echo -e "${YELLOW}🔍 测试 MySQL 连接...${NC}"
if docker exec tsp-mysql mysql -uroot -proot123456 -e "SELECT 1;" &> /dev/null; then
    echo -e "${GREEN}✓ MySQL 连接成功${NC}"
    echo -e "  数据库: test_db"
    echo -e "  用户: root, test_user"
else
    echo -e "${RED}❌ MySQL 连接失败${NC}"
fi
echo ""

# 测试 Redis 连接
echo -e "${YELLOW}🔍 测试 Redis 连接...${NC}"
if docker exec tsp-redis redis-cli ping &> /dev/null; then
    echo -e "${GREEN}✓ Redis 连接成功${NC}"
    echo -e "  版本: $(docker exec tsp-redis redis-cli INFO server | grep 'redis_version' | cut -d: -f2)"
else
    echo -e "${RED}❌ Redis 连接失败${NC}"
fi
echo ""

# 显示测试数据
echo -e "${YELLOW}📊 数据库信息...${NC}"
echo ""
echo "  MySQL 表:"
docker exec tsp-mysql mysql -uroot -proot123456 test_db -e "SHOW TABLES;" | tail -n +2
echo ""

echo "  用户数量:"
docker exec tsp-mysql mysql -uroot -proot123456 test_db -e "SELECT COUNT(*) as count FROM users;"
echo ""

echo -e "${GREEN}✓ 测试完成！${NC}"
echo ""
echo -e "${YELLOW}💡 连接字符串：${NC}"
echo ""
echo "  MySQL:"
echo "    Host: 127.0.0.1"
echo "    Port: 3306"
echo "    User: root"
echo "    Password: root123456"
echo "    Database: test_db"
echo ""
echo "  Redis:"
echo "    Host: 127.0.0.1"
echo "    Port: 6379"
echo "    Password: (无)"
