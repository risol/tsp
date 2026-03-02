#!/bin/bash
#
# Docker service connection test script
# Verify MySQL, Redis and LDAP services are running correctly
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

LDAP_CONTAINER="tsp-samba-ad"

echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     Testing Docker service connections      ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
echo ""

# Check docker-compose
if command -v docker-compose &> /dev/null; then
    DOCKER_COMPOSE="docker-compose"
elif docker compose version > /dev/null 2>&1; then
    DOCKER_COMPOSE="docker compose"
else
    echo -e "${RED}❌ Error: docker-compose not found${NC}"
    exit 1
fi

# Check container status
echo -e "${YELLOW}📊 Checking container status...${NC}"
if ! $DOCKER_COMPOSE ps | grep -q "Up"; then
    echo -e "${RED}❌ Error: Containers are not running${NC}"
    echo "Please run: sh ./docker/start.sh"
    exit 1
fi
echo -e "${GREEN}✓ Containers are running${NC}"
echo ""

# Test counters
pass_count=0
fail_count=0

# Test MySQL connection
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}🐬 Testing MySQL connection...${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
if docker exec tsp-mysql mysql -uroot -proot123456 -e "SELECT 1;" &> /dev/null; then
    echo -e "${GREEN}✓ MySQL connection successful${NC}"
    echo -e "  Database: test_db"
    echo -e "  Users: root, test_user"
    ((pass_count++))
else
    echo -e "${RED}❌ MySQL connection failed${NC}"
    ((fail_count++))
fi
echo ""

# Test Redis connection
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${RED}🔴 Testing Redis connection...${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
if docker exec tsp-redis redis-cli ping &> /dev/null; then
    echo -e "${GREEN}✓ Redis connection successful${NC}"
    REDIS_VERSION=$(docker exec tsp-redis redis-cli INFO server | grep 'redis_version' | cut -d: -f2)
    echo -e "  Version: ${REDIS_VERSION}"
    echo -e "  Number of databases: $(docker exec tsp-redis redis-cli DBSIZE)"
    ((pass_count++))
else
    echo -e "${RED}❌ Redis connection failed${NC}"
    ((fail_count++))
fi
echo ""

# Test Samba AD connection
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}🔐 Testing Samba AD connection...${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
if docker exec $LDAP_CONTAINER ldapsearch -x -H ldap://localhost:389 -b DC=example,DC=com -D "CN=Administrator,CN=Users,DC=example,DC=com" -w P@ssw0rd123 "(objectClass=*)" &> /dev/null; then
    echo -e "${GREEN}✓ Samba AD connection successful${NC}"
    echo -e "  Domain: EXAMPLE.COM"
    echo -e "  Base DN: DC=example,DC=com"
    echo -e "  Admin: CN=Administrator,CN=Users,DC=example,DC=com"

    # Count users
    USER_COUNT=$(docker exec $LDAP_CONTAINER ldapsearch -x -H ldap://localhost:389 -b CN=Users,DC=example,DC=com -D "CN=Administrator,CN=Users,DC=example,DC=com" -w P@ssw0rd123 "(objectClass=user)" 2>/dev/null | grep "^dn:" | wc -l)
    echo -e "  Number of users: ${USER_COUNT}"
    ((pass_count++))
else
    echo -e "${RED}❌ Samba AD connection failed${NC}"
    ((fail_count++))
fi
echo ""

# Show test data
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}📊 Database information...${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

echo "  MySQL tables:"
docker exec tsp-mysql mysql -uroot -proot123456 test_db -e "SHOW TABLES;" 2>/dev/null | tail -n +2
echo ""

echo "  MySQL users:"
docker exec tsp-mysql mysql -uroot -proot123456 test_db -e "SELECT id, username, email FROM users LIMIT 5;" 2>/dev/null | column -t
echo ""

echo "  Redis keys:"
docker exec tsp-redis redis-cli KEYS '*' 2>/dev/null | head -5
echo ""

if docker ps | grep -q $LDAP_CONTAINER; then
    echo "  Samba AD users (first 5):"
    docker exec $LDAP_CONTAINER ldapsearch -x -H ldap://localhost:389 -b CN=Users,DC=example,DC=com -D "CN=Administrator,CN=Users,DC=example,DC=com" -w P@ssw0rd123 "(objectClass=user)" cn 2>/dev/null | grep "^dn:" | head -5
    echo ""
fi

# Test summary
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}📋 Test summary${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  ${GREEN}Passed: ${pass_count}${NC}  |  ${RED}Failed: ${fail_count}${NC}"
echo ""

if [ $fail_count -eq 0 ]; then
    echo -e "${GREEN}✓ All service connections are normal!${NC}"
else
    echo -e "${RED}⚠️  Some service connections failed, please check logs${NC}"
    echo ""
    echo "  View logs:"
    echo "    sh ./docker/status.sh"
fi

echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}💡 Connection strings:${NC}"
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
echo -e "    Password: (none)"
echo ""
echo -e "  ${GREEN}Samba AD:${NC}"
echo -e "    URL: ldap://localhost:389"
echo -e "    Base DN: DC=example,DC=com"
echo -e "    Admin: CN=Administrator,CN=Users,DC=example,DC=com"
echo -e "    Admin Password: P@ssw0rd123"
echo ""
