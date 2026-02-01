#!/bin/bash
#
# Docker 服务连接测试脚本
# 验证 MySQL、Redis 和 LDAP 服务是否正常运行
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

# 测试计数器
pass_count=0
fail_count=0

# 测试 MySQL 连接
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}🐬 测试 MySQL 连接...${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
if docker exec tsp-mysql mysql -uroot -proot123456 -e "SELECT 1;" &> /dev/null; then
    echo -e "${GREEN}✓ MySQL 连接成功${NC}"
    echo -e "  数据库: test_db"
    echo -e "  用户: root, test_user"
    ((pass_count++))
else
    echo -e "${RED}❌ MySQL 连接失败${NC}"
    ((fail_count++))
fi
echo ""

# 测试 Redis 连接
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${RED}🔴 测试 Redis 连接...${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
if docker exec tsp-redis redis-cli ping &> /dev/null; then
    echo -e "${GREEN}✓ Redis 连接成功${NC}"
    REDIS_VERSION=$(docker exec tsp-redis redis-cli INFO server | grep 'redis_version' | cut -d: -f2)
    echo -e "  版本: ${REDIS_VERSION}"
    echo -e "  数据库数: $(docker exec tsp-redis redis-cli DBSIZE)"
    ((pass_count++))
else
    echo -e "${RED}❌ Redis 连接失败${NC}"
    ((fail_count++))
fi
echo ""

# 测试 LDAP 连接
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}🔐 测试 LDAP 连接...${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
if docker exec $LDAP_CONTAINER ldapsearch -x -H ldap://localhost:389 -b dc=example,dc=org -D "cn=admin,dc=example,dc=org" -w admin123456 "(objectClass=*)" &> /dev/null; then
    echo -e "${GREEN}✓ LDAP 连接成功${NC}"
    echo -e "  Base DN: dc=example,dc=org"
    echo -e "  管理员: cn=admin,dc=example,dc=org"

    # 统计用户数量
    USER_COUNT=$(docker exec $LDAP_CONTAINER ldapsearch -x -H ldap://localhost:389 -b ou=developers,dc=example,dc=org -D "cn=admin,dc=example,dc=org" -w admin123456 "(objectClass=person)" 2>/dev/null | grep "^dn:" | wc -l)
    echo -e "  用户数量: ${USER_COUNT}"
    ((pass_count++))
else
    echo -e "${RED}❌ LDAP 连接失败${NC}"
    ((fail_count++))
fi
echo ""

# 显示测试数据
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}📊 数据库信息...${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

echo "  MySQL 表:"
docker exec tsp-mysql mysql -uroot -proot123456 test_db -e "SHOW TABLES;" 2>/dev/null | tail -n +2
echo ""

echo "  MySQL 用户:"
docker exec tsp-mysql mysql -uroot -proot123456 test_db -e "SELECT id, username, email FROM users LIMIT 5;" 2>/dev/null | column -t
echo ""

echo "  Redis 键:"
docker exec tsp-redis redis-cli KEYS '*' 2>/dev/null | head -5
echo ""

if docker ps | grep -q $LDAP_CONTAINER; then
    echo "  LDAP 用户（前5个）:"
    docker exec $LDAP_CONTAINER ldapsearch -x -H ldap://localhost:389 -b ou=developers,dc=example,dc=org -D "cn=admin,dc=example,dc=org" -w admin123456 "(objectClass=person)" cn mail 2>/dev/null | grep "^dn:" | head -5
    echo ""
fi

# 测试总结
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}📋 测试总结${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  ${GREEN}通过: ${pass_count}${NC}  |  ${RED}失败: ${fail_count}${NC}"
echo ""

if [ $fail_count -eq 0 ]; then
    echo -e "${GREEN}✓ 所有服务连接正常！${NC}"
else
    echo -e "${RED}⚠️  部分服务连接失败，请检查日志${NC}"
    echo ""
    echo "  查看日志:"
    echo "    ./docker-status.sh"
fi

echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}💡 连接字符串：${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  ${BLUE}MySQL:${NC}"
echo -e "    Host: 127.0.0.1"
echo -e "    Port: 3306"
echo -e "    User: root"
echo -e "    Password: root123456"
echo -e "    Database: test_db"
echo ""
echo -e "  ${RED}Redis:${NC}"
echo -e "    Host: 127.0.0.1"
echo -e "    Port: 6379"
echo -e "    Password: (无)"
echo ""
echo -e "  ${GREEN}LDAP:${NC}"
echo -e "    URL: ldap://localhost:1389"
echo -e "    Base DN: dc=example,dc=org"
echo -e "    Admin DN: cn=admin,dc=example,dc=org"
echo -e "    Admin Password: admin123456"
echo ""
