#!/bin/bash
#
# Docker service stop script
# Stop and cleanup MySQL, Redis and LDAP containers
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     Stopping Docker test services          ║${NC}"
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

# Stop all services
if $DOCKER_COMPOSE ps | grep -q "Up"; then
    echo -e "${YELLOW}⏹ Stopping all services...${NC}"
    $DOCKER_COMPOSE down
    echo -e "${GREEN}✅ Services stopped${NC}"
else
    echo -e "${YELLOW}⚠️  No running services${NC}"
fi

# Ask whether to delete data
echo ""
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}🗑️  Delete data volumes? (All data will be lost)${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "  ${RED}Warning: This operation will delete:${NC}"
echo -e "  - MySQL database data"
echo -e "  - Redis cache data"
echo -e "  - LDAP directory data"
echo ""
echo -n "Enter 'yes' to confirm deletion, or press Enter to cancel: "
read -r answer

if [ "$answer" = "yes" ]; then
    echo ""
    echo -e "${YELLOW}Deleting data volumes...${NC}"
    $DOCKER_COMPOSE down -v
    echo -e "${GREEN}✅ Data volumes deleted${NC}"
else
    echo -e "${GREEN}✓ Data volumes preserved${NC}"
fi

echo ""
echo -e "${GREEN}✓ Operation complete${NC}"
echo ""
