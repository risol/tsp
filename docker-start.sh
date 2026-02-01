#!/bin/bash
#
# Docker 服务启动脚本
# 用于启动测试所需的 MySQL、Redis 和 LDAP 容器
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

# 启动所有服务
$DOCKER_COMPOSE up -d

echo ""
echo -e "${YELLOW}⏳ 等待 LDAP 服务启动...${NC}"
sleep 5

# 检查并导入测试用户
echo ""
echo -e "${YELLOW}🔍 检查测试用户...${NC}"
USER_COUNT=$(docker exec $LDAP_CONTAINER ldapsearch -x -H ldap://localhost:389 -b ou=developers,dc=example,dc=org -D "cn=admin,dc=example,dc=org" -w admin123456 "(objectClass=person)" 2>/dev/null | grep "^dn:" | wc -l)

if [ "$USER_COUNT" -lt 6 ]; then
    echo -e "${YELLOW}📥 导入测试用户...${NC}"
    if [ -f "docker/ldap/test-users.ldif" ]; then
        docker exec -i $LDAP_CONTAINER ldapadd -x -H ldap://localhost:389 -D "cn=admin,dc=example,dc=org" -w admin123456 < docker/ldap/test-users.ldif > /dev/null 2>&1
        if [ $? -eq 0 ]; then
            echo -e "${GREEN}✓ 测试用户导入成功 (6个用户)${NC}"
        else
            echo -e "${YELLOW}⚠️  测试用户导入失败${NC}"
        fi
    fi
else
    echo -e "${GREEN}✓ 测试用户已存在 ($USER_COUNT 个用户)${NC}"
fi

echo ""
echo -e "${GREEN}✅ 服务启动成功！${NC}"
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}📊 服务信息：${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}🐬 MySQL 数据库:${NC}"
echo -e "    Host: 127.0.0.1"
echo -e "    Port: 3306"
echo -e "    Root Password: root123456"
echo -e "    Database: test_db"
echo -e "    User: test_user / test123456"
echo ""
echo -e "${RED}🔴 Redis 缓存:${NC}"
echo -e "    Host: 127.0.0.1"
echo -e "    Port: 6379"
echo -e "    No password (默认无密码)"
echo ""
echo -e "${GREEN}🔐 LDAP 认证服务:${NC}"
echo -e "    URL: ldap://localhost:1389"
echo -e "    Base DN: dc=example,dc=org"
echo -e "    Admin DN: cn=admin,dc=example,dc=org"
echo -e "    Admin Password: admin123456"
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}🌐 管理工具：${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  phpMyAdmin (MySQL):    ${GREEN}http://localhost:8080${NC}"
echo -e "  Redis Commander:       ${GREEN}http://localhost:8081${NC}"
echo -e "  phpLDAPadmin (LDAP):   ${GREEN}http://localhost:8082${NC}"
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}🔧 常用命令：${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  查看日志:"
echo -e "    ${YELLOW}$DOCKER_COMPOSE logs -f mysql${NC}"
echo -e "    ${YELLOW}$DOCKER_COMPOSE logs -f redis${NC}"
echo -e "    ${YELLOW}$DOCKER_COMPOSE logs -f openldap${NC}"
echo ""
echo -e "  进入 MySQL:"
echo -e "    ${YELLOW}$DOCKER_COMPOSE exec mysql mysql -uroot -proot123456${NC}"
echo ""
echo -e "  进入 Redis:"
echo -e "    ${YELLOW}$DOCKER_COMPOSE exec redis redis-cli${NC}"
echo ""
echo -e "  测试 LDAP 连接:"
echo -e "    ${YELLOW}deno run --allow-all tests/unit/ldap_docker_test.ts${NC}"
echo ""
echo -e "  重启服务:"
echo -e "    ${YELLOW}./docker-restart.sh${NC}"
echo ""
echo -e "  停止服务:"
echo -e "    ${YELLOW}./docker-stop.sh${NC}"
echo "    或: $DOCKER_COMPOSE down"
echo ""
echo -e "${GREEN}✓ 服务已在后台运行${NC}"
echo ""
