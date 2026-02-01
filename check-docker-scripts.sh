#!/bin/bash
#
# Docker 脚本验证脚本
# 检查所有 Docker 管理脚本的完整性
#

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}🔍 Docker 脚本完整性检查${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

PASS=0
FAIL=0

# 检查文件是否存在
check_file() {
    if [ -f "$1" ]; then
        echo -e "${GREEN}✓${NC} $1"
        ((PASS++))
    else
        echo -e "${RED}✗${NC} $1 (文件不存在)"
        ((FAIL++))
    fi
}

# 检查文件中的内容
check_content() {
    local file=$1
    local pattern=$2
    local desc=$3

    if grep -q "$pattern" "$file" 2>/dev/null; then
        echo -e "${GREEN}✓${NC} $desc"
        ((PASS++))
    else
        echo -e "${RED}✗${NC} $desc"
        ((FAIL++))
    fi
}

echo -e "${YELLOW}📁 检查文件存在性:${NC}"
echo ""
check_file "docker-compose.yml"
check_file "docker-start.sh"
check_file "docker-start.bat"
check_file "docker-stop.sh"
check_file "docker-stop.bat"
check_file "docker-restart.sh"
check_file "docker-restart.bat"
check_file "docker-status.sh"
check_file "docker-test-connection.sh"
check_file "docker-cleanup.sh"
check_file "docker-cleanup.bat"
check_file "docker/import-ldap-users.sh"
check_file "docker/import-ldap-users.bat"
check_file "docker/ldap/test-users.ldif"

echo ""
echo -e "${YELLOW}🔧 检查脚本功能:${NC}"
echo ""

# 检查 docker-start.sh
echo -e "${CYAN}docker-start.sh:${NC}"
check_content "docker-start.sh" "docker-test-openldap" "引用正确的 LDAP 容器名"
check_content "docker-start.sh" "import-ldap-users" "包含用户导入逻辑"
check_content "docker-start.sh" "test-users.ldif" "引用测试用户文件"

echo ""
echo -e "${CYAN}docker-stop.sh:${NC}"
check_content "docker-stop.sh" "docker-test-openldap" "引用正确的 LDAP 容器名"
check_content "docker-stop.sh" "docker rm -f" "包含删除 LDAP 容器的逻辑"

echo ""
echo -e "${CYAN}docker-restart.sh:${NC}"
check_content "docker-restart.sh" "docker-test-openldap" "引用正确的 LDAP 容器名"
check_content "docker-restart.sh" "ldapsearch" "包含 LDAP 健康检查"

echo ""
echo -e "${CYAN}docker-status.sh:${NC}"
check_content "docker-status.sh" "docker-test-openldap" "引用正确的 LDAP 容器名"

echo ""
echo -e "${CYAN}docker-test-connection.sh:${NC}"
check_content "docker-test-connection.sh" "docker-test-openldap" "引用正确的 LDAP 容器名"
check_content "docker-test-connection.sh" "ldapsearch" "包含 LDAP 连接测试"

echo ""
echo -e "${CYAN}docker-cleanup.sh:${NC}"
check_content "docker-cleanup.sh" "docker-test-openldap" "引用正确的 LDAP 容器名"

echo ""
echo -e "${CYAN}docker-compose.yml:${NC}"
check_content "docker-compose.yml" "mysql:" "包含 MySQL 服务"
check_content "docker-compose.yml" "redis:" "包含 Redis 服务"

# 检查不应该存在的内容
echo ""
echo -e "${CYAN}docker-compose.yml (不应包含):${NC}"
if ! grep -q "openldap:" "docker-compose.yml"; then
    echo -e "${GREEN}✓${NC} 不包含 openldap 服务（正确）"
    ((PASS++))
else
    echo -e "${RED}✗${NC} 包含 openldap 服务（应该删除）"
    ((FAIL++))
fi

if ! grep -q "phpldapadmin:" "docker-compose.yml"; then
    echo -e "${GREEN}✓${NC} 不包含 phpldapadmin 服务（正确）"
    ((PASS++))
else
    echo -e "${RED}✗${NC} 包含 phpldapadmin 服务（应该删除）"
    ((FAIL++))
fi

# 检查文件权限
echo ""
echo -e "${YELLOW}🔐 检查脚本权限:${NC}"
echo ""
for script in docker-*.sh docker/import-ldap-users.sh; do
    if [ -x "$script" ]; then
        echo -e "${GREEN}✓${NC} $script (可执行)"
        ((PASS++))
    else
        echo -e "${YELLOW}⚠️  $script (不可执行)${NC}"
    fi
done

echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}📊 检查结果:${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  ${GREEN}通过: ${PASS}${NC}  |  ${RED}失败: ${FAIL}${NC}"
echo ""

if [ $FAIL -eq 0 ]; then
    echo -e "${GREEN}✅ 所有检查通过！${NC}"
    exit 0
else
    echo -e "${RED}❌ 发现 ${FAIL} 个问题，请修复${NC}"
    exit 1
fi
