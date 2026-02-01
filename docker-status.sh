#!/bin/bash
#
# Docker 服务状态和日志查看脚本
# 显示 MySQL、Redis 和 LDAP 服务的状态和日志
#

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

LDAP_CONTAINER="tsp-openldap"

echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     Docker 服务状态和日志                     ║${NC}"
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

# 显示容器状态
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}📊 服务状态${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
$DOCKER_COMPOSE ps
echo ""

# 显示 LDAP 容器状态
if docker ps -a | grep -q $LDAP_CONTAINER; then
    docker ps --filter "name=$LDAP_CONTAINER" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
fi
echo ""

# 显示资源使用情况
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}💻 资源使用${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# 函数：显示服务状态
show_service_status() {
    local service=$1
    local container=$2

    if docker ps | grep -q "$container"; then
        # CPU 使用率
        local cpu=$(docker stats --no-stream --format "{{.CPUPerc}}" $container 2>/dev/null || echo "N/A")
        # 内存使用
        local mem=$(docker stats --no-stream --format "{{.MemUsage}}" $container 2>/dev/null || echo "N/A")
        # 网络IO
        local net=$(docker stats --no-stream --format "{{.NetIO}}" $container 2>/dev/null || echo "N/A")
        # 块IO
        local io=$(docker stats --no-stream --format "{{.BlockIO}}" $container 2>/dev/null || echo "N/A")

        echo -e "${YELLOW}${service}:${NC}"
        echo -e "  状态: ${GREEN}运行中${NC}"
        echo -e "  CPU:    ${cpu}"
        echo -e "  内存:   ${mem}"
        echo -e "  网络:   ${net}"
        echo -e "  磁盘:   ${io}"
    else
        echo -e "${YELLOW}${service}:${NC}"
        echo -e "  状态: ${RED}未运行${NC}"
    fi
    echo ""
}

show_service_status "🐬 MySQL" "tsp-mysql"
show_service_status "🔴 Redis" "tsp-redis"
show_service_status "🔐 LDAP" "$LDAP_CONTAINER"

# 显示最近的日志
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}📝 最近日志（最后20行）${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# 显示 MySQL 日志
if docker ps | grep -q "tsp-mysql"; then
    echo -e "${BLUE}🐬 MySQL 日志:${NC}"
    $DOCKER_COMPOSE logs --tail=5 mysql 2>&1 | docker exec tsp-mysql tail -n 5 2>/dev/null || echo "  暂无日志"
    echo ""
fi

# 显示 Redis 日志
if docker ps | grep -q "tsp-redis"; then
    echo -e "${RED}🔴 Redis 日志:${NC}"
    $DOCKER_COMPOSE logs --tail=5 redis 2>&1
    echo ""
fi

# 显示 LDAP 日志
if docker ps | grep -q $LDAP_CONTAINER; then
    echo -e "${GREEN}🔐 LDAP 日志:${NC}"
    docker logs --tail=5 $LDAP_CONTAINER 2>&1
    echo ""
fi

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}💡 提示：${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "  实时查看日志:"
echo "    ${YELLOW}./docker-status.sh | grep -A 1000 📝${NC}"
echo ""
echo "  查看特定服务日志:"
echo "    ${YELLOW}$DOCKER_COMPOSE logs -f mysql${NC}"
echo "    ${YELLOW}$DOCKER_COMPOSE logs -f redis${NC}"
echo "    ${YELLOW}docker logs -f $LDAP_CONTAINER${NC}"
echo ""
echo "  重启服务:"
echo "    ${YELLOW}./docker-restart.sh${NC}"
echo ""
echo "  测试连接:"
echo "    ${YELLOW}./docker-test-connection.sh${NC}"
echo ""
