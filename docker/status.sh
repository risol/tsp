#!/bin/bash
#
# Docker service status and log viewing script
# Display status and logs for MySQL, Redis and LDAP services
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
echo -e "${GREEN}║     Docker service status and logs         ║${NC}"
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

# Show container status
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}📊 Service status${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
$DOCKER_COMPOSE ps
echo ""

# Show LDAP container status
if docker ps -a | grep -q $LDAP_CONTAINER; then
    docker ps --filter "name=$LDAP_CONTAINER" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
fi
echo ""

# Show resource usage
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}💻 Resource usage${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Function: show service status
show_service_status() {
    local service=$1
    local container=$2

    if docker ps | grep -q "$container"; then
        # CPU usage
        local cpu=$(docker stats --no-stream --format "{{.CPUPerc}}" $container 2>/dev/null || echo "N/A")
        # Memory usage
        local mem=$(docker stats --no-stream --format "{{.MemUsage}}" $container 2>/dev/null || echo "N/A")
        # Network IO
        local net=$(docker stats --no-stream --format "{{.NetIO}}" $container 2>/dev/null || echo "N/A")
        # Block IO
        local io=$(docker stats --no-stream --format "{{.BlockIO}}" $container 2>/dev/null || echo "N/A")

        echo -e "${YELLOW}${service}:${NC}"
        echo -e "  Status: ${GREEN}Running${NC}"
        echo -e "  CPU:    ${cpu}"
        echo -e "  Memory: ${mem}"
        echo -e "  Network: ${net}"
        echo -e "  Disk:   ${io}"
    else
        echo -e "${YELLOW}${service}:${NC}"
        echo -e "  Status: ${RED}Not running${NC}"
    fi
    echo ""
}

show_service_status "🐬 MySQL" "tsp-mysql"
show_service_status "🔴 Redis" "tsp-redis"
show_service_status "🔐 LDAP" "$LDAP_CONTAINER"

# Show recent logs
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}📝 Recent logs (last 20 lines)${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# Show MySQL logs
if docker ps | grep -q "tsp-mysql"; then
    echo -e "${BLUE}🐬 MySQL logs:${NC}"
    $DOCKER_COMPOSE logs --tail=5 mysql 2>&1 || echo "  No logs available"
    echo ""
fi

# Show Redis logs
if docker ps | grep -q "tsp-redis"; then
    echo -e "${RED}🔴 Redis logs:${NC}"
    $DOCKER_COMPOSE logs --tail=5 redis 2>&1
    echo ""
fi

# Show LDAP logs
if docker ps | grep -q $LDAP_CONTAINER; then
    echo -e "${GREEN}🔐 LDAP logs:${NC}"
    docker logs --tail=5 $LDAP_CONTAINER 2>&1
    echo ""
fi

echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${YELLOW}💡 Tips:${NC}"
echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "  View logs in real time:"
echo "    ${YELLOW}sh ./docker/status.sh | grep -A 1000 📝${NC}"
echo ""
echo "  View specific service logs:"
echo "    ${YELLOW}$DOCKER_COMPOSE logs -f mysql${NC}"
echo "    ${YELLOW}$DOCKER_COMPOSE logs -f redis${NC}"
echo "    ${YELLOW}docker logs -f $LDAP_CONTAINER${NC}"
echo ""
echo "  Restart services:"
echo "    ${YELLOW}sh ./docker/restart.sh${NC}"
echo ""
echo "  Test connections:"
echo "    ${YELLOW}sh ./docker/test-connection.sh${NC}"
echo ""
