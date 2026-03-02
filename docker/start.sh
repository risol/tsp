#!/bin/bash
#
# Docker service start script
# Start MySQL, Redis and LDAP containers required for testing
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
echo -e "${GREEN}║     Starting Docker test services          ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
echo ""

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}❌ Error: Docker is not running${NC}"
    echo "Please start Docker Desktop or run: sudo systemctl start docker"
    exit 1
fi

# Check if docker-compose is installed
if ! command -v docker-compose &> /dev/null; then
    echo -e "${YELLOW}⚠️  docker-compose not found${NC}"
    echo "Trying to use docker compose (Docker Desktop latest)..."

    if docker compose version > /dev/null 2>&1; then
        echo -e "${GREEN}✓ Using docker compose${NC}"
        DOCKER_COMPOSE="docker compose"
    else
        echo -e "${RED}❌ Error: docker-compose or docker compose not found${NC}"
        echo "Please install Docker Desktop or docker-compose"
        exit 1
    fi
else
    echo -e "${GREEN}✓ Using docker-compose${NC}"
    DOCKER_COMPOSE="docker-compose"
fi

echo ""
echo -e "${YELLOW}📋 Starting services...${NC}"
echo ""

# Start all services
$DOCKER_COMPOSE up -d

echo ""
echo -e "${YELLOW}⏳ Waiting for Samba AD service to initialize...${NC}"
echo -e "${CYAN}ℹ️  This may take 30-40 seconds...${NC}"
sleep 40

# Clear and create test users
echo ""
echo -e "${YELLOW}⏳ Clearing and creating test users...${NC}"
sleep 5

# Get all users list
ALL_USERS=$(docker exec tsp-samba-ad samba-tool user list -H ldap://localhost -U "Administrator%P@ssw0rd123" 2>/dev/null | grep -v "Administrator" || true)

# Delete all users (except Administrator)
if [ -n "$ALL_USERS" ]; then
    echo -e "${CYAN}ℹ️  Found existing users, deleting...${NC}"
    for user in $ALL_USERS; do
        echo "  Deleting user: $user"
        docker exec tsp-samba-ad samba-tool user delete "$user" -H ldap://localhost -U "Administrator%P@ssw0rd123" 2>/dev/null || true
    done
    echo -e "${GREEN}✓ Old users cleared${NC}"
fi

# Create 10 test accounts (simulate real structure)
echo ""
echo -e "${CYAN}ℹ️  Creating 10 test accounts...${NC}"

# User data array (using pinyin to avoid encoding issues)
declare -A USERS
USERS=(
    ["ZhangWei"]="t_zhangw|13800138001|80213801"
    ["LiNa"]="t_lina|13800138002|80213802"
    ["WangQiang"]="t_wangq|13800138003|80213803"
    ["LiuYang"]="t_liuy|13800138004|80213804"
    ["ChenJing"]="t_chenj|13800138005|80213805"
    ["YangLei"]="t_yangl|13800138006|80213806"
    ["ZhaoMin"]="t_zhaom|13800138007|80213807"
    ["SunJie"]="t_sunj|13800138008|80213808"
    ["ZhouTing"]="t_zhout|13800138009|80213809"
    ["WuGang"]="t_wug|13800138010|80213810"
)

# Create users
for CN in "${!USERS[@]}"; do
    IFS='|' read -r SAMACCOUNT MOBILE EMPID <<< "${USERS[$CN]}"
    MAIL="${SAMACCOUNT}@cnnp.com.cn"

    echo "  Creating: $CN ($SAMACCOUNT)"

    docker exec tsp-samba-ad samba-tool user create "$SAMACCOUNT" "Test@123" \
        --use-username-as-cn \
        --given-name="$CN" \
        --surname="User" \
        --mail-address="$MAIL" \
        --telephone-number="$MOBILE" \
        --job-title="Employee" \
        --department="Business" \
        --company="Beijing CNRP Info Tech" \
        --description="Employee ID: $EMPID" \
        -H ldap://localhost \
        -U "Administrator%P@ssw0rd123" 2>/dev/null

    # Use ldapmodify to set employeeID attribute (not supported by samba-tool)
    docker exec tsp-samba-ad ldapmodify -x \
        -H ldap://localhost:389 \
        -D "CN=Administrator,CN=Users,DC=example,DC=com" \
        -w "P@ssw0rd123" \
        <<LDIF 2>/dev/null
dn: CN=$SAMACCOUNT,CN=Users,DC=example,DC=com
changetype: modify
replace: employeeID
employeeID: $EMPID
-
replace: displayName
displayName: $CN
-
replace: userPrincipalName
userPrincipalName: $SAMACCOUNT
LDIF
done

echo -e "${GREEN}✓ Test accounts created${NC}"

# Create test group
if ! docker exec tsp-samba-ad samba-tool group list 2>/dev/null | grep -q "TestGroup"; then
    docker exec tsp-samba-ad samba-tool group add TestGroup \
        -H ldap://localhost \
        -U "Administrator%P@ssw0rd123" 2>/dev/null

    # Add first 5 users to test group
    docker exec tsp-samba-ad samba-tool group addmembers TestGroup t_zhangw,t_lina,t_wangq,t_liuy,t_chenj \
        -H ldap://localhost \
        -U "Administrator%P@ssw0rd123" 2>/dev/null

    echo -e "${GREEN}✓ Test group created${NC}"
else
    echo -e "${CYAN}ℹ️  Test group already exists${NC}"
fi

echo ""
echo -e "${GREEN}✓ Samba AD initialization complete!${NC}"
echo -e "${CYAN}ℹ️  Test users have been manually created${NC}"

# Fix Samba AD network binding (allow external connections)
echo ""
echo -e "${YELLOW}⏳ Fixing Samba AD network binding...${NC}"
sleep 2
docker exec tsp-samba-ad sh -c "sed -i 's/bind interfaces only = Yes/bind interfaces only = No/' /usr/local/samba/etc/smb.conf" 2>/dev/null
docker exec tsp-samba-ad sh -c "killall samba || true" 2>/dev/null
sleep 3
echo -e "${GREEN}✓ Samba AD network binding fixed${NC}"

# Configure LAM for Samba AD
echo ""
echo -e "${YELLOW}⏳ Configuring LAM for Samba AD...${NC}"
sleep 3

# Update LAM config to use Windows modules
docker exec tsp-ldap-admin sh -c "sed -i 's/types: modules_user: inetOrgPerson.*/types: modules_user: windowsUser/' /var/lib/ldap-account-manager/config/lam.conf" 2>/dev/null
docker exec tsp-ldap-admin sh -c "sed -i 's/types: modules_group: posixGroup.*/types: modules_group: windowsGroup/' /var/lib/ldap-account-manager/config/lam.conf" 2>/dev/null
docker exec tsp-ldap-admin sh -c "sed -i 's/types: attr_user:.*/types: attr_user: #cn;#givenName;#sn;#displayName;#mail/' /var/lib/ldap-account-manager/config/lam.conf" 2>/dev/null
docker exec tsp-ldap-admin sh -c "sed -i 's/types: attr_group:.*/types: attr_group: #cn;#member;#description/' /var/lib/ldap-account-manager/config/lam.conf" 2>/dev/null

echo -e "${GREEN}✓ LAM configuration complete${NC}"

echo ""
echo -e "${GREEN}✅ Services started successfully!${NC}"
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}📊 Service information:${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${BLUE}🐬 MySQL database:${NC}"
echo -e "    Host: 127.0.0.1"
echo -e "    Port: 3306"
echo -e "    Root Password: root123456"
echo -e "    Database: test_db"
echo -e "    User: test_user / test123456"
echo ""
echo -e "${RED}🔴 Redis cache:${NC}"
echo -e "    Host: 127.0.0.1"
echo -e "    Port: 6379"
echo -e "    No password (default no password)"
echo ""
echo -e "${GREEN}🔐 Samba AD authentication service:${NC}"
echo -e "    URL: ldap://localhost:389"
echo -e "    Domain: EXAMPLE.COM"
echo -e "    Base DN: DC=example,DC=com"
echo -e "    Admin: CN=Administrator,CN=Users,DC=example,DC=com"
echo -e "    Admin Password: P@ssw0rd123"
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}🌐 Admin tools:${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  phpMyAdmin (MySQL):    ${GREEN}http://localhost:8080${NC}"
echo -e "  Redis Commander:       ${GREEN}http://localhost:8081${NC}"
echo -e "  phpLDAPadmin (LDAP):   ${GREEN}http://localhost:8082${NC}"
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}🔧 Common commands:${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  View logs:"
echo -e "    ${YELLOW}$DOCKER_COMPOSE logs -f mysql${NC}"
echo -e "    ${YELLOW}$DOCKER_COMPOSE logs -f redis${NC}"
echo -e "    ${YELLOW}$DOCKER_COMPOSE logs -f samba-ad${NC}"
echo ""
echo -e "  Enter MySQL:"
echo -e "    ${YELLOW}$DOCKER_COMPOSE exec mysql mysql -uroot -proot123456${NC}"
echo ""
echo -e "  Enter Redis:"
echo -e "    ${YELLOW}$DOCKER_COMPOSE exec redis redis-cli${NC}"
echo ""
echo -e "  Test LDAP connection:"
echo -e "    ${YELLOW}deno run --allow-all tests/unit/ldap_docker_test.ts${NC}"
echo ""
echo -e "  Restart services:"
echo -e "    ${YELLOW}./restart.sh${NC}"
echo ""
echo -e "  Stop services:"
echo -e "    ${YELLOW}./stop.sh${NC}"
echo "    or: $DOCKER_COMPOSE down"
echo ""
echo -e "${GREEN}✓ Services are running in background${NC}"
echo ""
