#!/bin/bash
#
# Docker test data cleanup script
# Delete all containers and data volumes, completely clean test data
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${RED}╔════════════════════════════════════════╗${NC}"
echo -e "${RED}║     ⚠️  Warning: Delete all test data ⚠️        ║${NC}"
echo -e "${RED}╚════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}This operation will:${NC}"
echo -e "${RED}  ✓ Stop all containers${NC}"
echo -e "${RED}  ✓ Delete all containers${NC}"
echo -e "${RED}  ✓ Delete all data volumes (MySQL, Redis, LDAP data)${NC}"
echo ""
echo -e "${YELLOW}The following data will be lost:${NC}"
echo -e "  • MySQL database (test_db) and all tables and data"
echo -e "  • All data in Redis cache"
echo -e "  • All data in LDAP directory service"
echo -e "  • All created users, sessions, articles, etc."
echo ""
echo -e "${BLUE}Note: After data deletion, restarting services will reinitialize test data${NC}"
echo ""

# Confirm operation
echo -e "${YELLOW}Are you sure you want to delete all test data?${NC}"
read -p "Enter 'yes' to confirm deletion: " confirm

if [ "$confirm" != "yes" ]; then
    echo -e "${GREEN}✓ Operation cancelled${NC}"
    exit 0
fi

# Check docker-compose
if command -v docker-compose &> /dev/null; then
    DOCKER_COMPOSE="docker-compose"
elif docker compose version > /dev/null 2>&1; then
    DOCKER_COMPOSE="docker compose"
else
    echo -e "${RED}❌ Error: docker-compose not found${NC}"
    exit 1
fi

echo ""
echo -e "${YELLOW}Cleaning up...${NC}"
echo ""

# Delete containers and data volumes
echo -e "${YELLOW}📦 Stopping and removing containers...${NC}"
$DOCKER_COMPOSE down -v

echo ""
echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║     ✓ Cleanup complete!                ║${NC}"
echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
echo ""
echo -e "${GREEN}All test data has been deleted${NC}"
echo ""
echo -e "${YELLOW}💡 Next steps:${NC}"
echo ""
echo -e "${BLUE}1. Restart services (will reinitialize test data):${NC}"
echo "   sh ./docker/start.sh"
echo ""
echo -e "${BLUE}2. Or stop using Docker:${NC}"
echo "   No action needed"
echo ""
