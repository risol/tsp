#!/bin/bash
#
# Docker 服务启动脚本
# 用于启动测试所需的 MySQL 和 Redis 容器
#

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     启动 Docker 测试服务                    ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
echo ""

# 检查 Docker 是否运行
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}❌ 错误: Docker 未运行${NC}"
    echo "请先启动 Docker Desktop 或执行: sudo systemctl start docker"
    exit 1
fi

# 检查 docker-compose 是否安装
if ! command -v docker-compose &> /dev/null; then
    echo -e "${YELLOW}⚠️  未找到 docker-compose${NC}"
    echo "尝试使用 docker compose (Docker Desktop 最新版本)..."

    if docker compose version > /dev/null 2>&1; then
        echo -e "${GREEN}✓ 使用 docker compose${NC}"
        DOCKER_COMPOSE="docker compose"
    else
        echo -e "${RED}❌ 错误: 未找到 docker-compose 或 docker compose${NC}"
        echo "请安装 Docker Desktop 或 docker-compose"
        exit 1
    fi
else
    echo -e "${GREEN}✓ 使用 docker-compose${NC}"
    DOCKER_COMPOSE="docker-compose"
fi

echo ""
echo -e "${YELLOW}📋 启动服务...${NC}"
echo ""

# 启动服务
$DOCKER_COMPOSE up -d

echo ""
echo -e "${GREEN}✅ 服务启动成功！${NC}"
echo ""
echo -e "${YELLOW}📊 服务信息：${NC}"
echo ""
echo -e "  MySQL 数据库:"
echo -e "    Host: 127.0.0.1"
echo -e "    Port: 3306"
echo -e "    Root Password: root123456"
echo -e "    Database: test_db"
echo -e "    User: test_user / test123456"
echo ""
echo -e "  Redis 缓存:"
echo -e "    Host: 127.0.0.1"
echo -e "    Port: 6379"
echo -e "    No password (默认无密码)"
echo ""
echo -e "  管理工具:"
echo -e "    phpMyAdmin: http://localhost:8080"
echo -e "    Redis Commander: http://localhost:8081"
echo ""
echo -e "${YELLOW}🔧 常用命令：${NC}"
echo ""
echo -e "  查看日志:"
echo -e "    $DOCKER_COMPOSE logs -f mysql"
echo -e "    $DOCKER_COMPOSE logs -f redis"
echo ""
echo -e "  进入 MySQL:"
echo -e "    $DOCKER_COMPOSE exec mysql mysql -uroot -proot123456"
echo ""
echo -e "  进入 Redis:"
echo -e "    $DOCKER_COMPOSE exec redis redis-cli"
echo ""
echo -e "  停止服务:"
echo -e "    ./docker-stop.sh"
echo "    或: $DOCKER_COMPOSE down"
echo ""
echo -e "${GREEN}✓ 服务已在后台运行${NC}"
