#!/bin/bash

# TSP Server Docker image build script (Linux/Mac)
# Automatically builds and tags TSP Server Docker image

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Color definitions
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Configuration
IMAGE_NAME="tspserver"
IMAGE_TAG="${1:-latest}"
REGISTRY="${2:-}"
FULL_IMAGE="${REGISTRY}${IMAGE_NAME}:${IMAGE_TAG}"

echo ""
echo "╔═══════════════════════════════════════════════════════╗"
echo "║     TSP Server Docker Image Build                  ║"
echo "╚═══════════════════════════════════════════════════════╝"
echo ""

# Check if Docker is running
echo "🔍 Checking Docker environment..."
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}Error: Docker is not running, please start Docker first${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Docker is running${NC}"
echo ""

# Show build information
echo "📋 Build information:"
echo "   Image name: ${IMAGE_NAME}"
echo "   Image tag: ${IMAGE_TAG}"
if [ -n "$REGISTRY" ]; then
    echo "   Registry: ${REGISTRY}"
fi
echo ""

# Clean old images (optional)
read -p "Delete old build cache? This will free up disk space (y/N): " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo ""
    echo "🧹 Cleaning old images..."
    docker image prune -f
    echo -e "${GREEN}✓ Cleanup complete${NC}"
    echo ""
fi

# Build image
echo "🔨 Starting Docker image build..."
echo "═════════════════════════════════════════════════════════"
echo ""

# Execute build
if [ -n "$REGISTRY" ]; then
    # If registry specified, build and push
    docker buildx build --platform linux/amd64,linux/arm64 -f "$SCRIPT_DIR/Dockerfile" -t ${FULL_IMAGE} --push "$PROJECT_ROOT"
else
    # Build local image only
    docker build -f "$SCRIPT_DIR/Dockerfile" -t ${FULL_IMAGE} "$PROJECT_ROOT"
fi

BUILD_STATUS=$?

echo ""
echo "═════════════════════════════════════════════════════════"
echo ""

if [ $BUILD_STATUS -eq 0 ]; then
    echo -e "${GREEN}╔══════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║     🎉 Build successful!                 ║${NC}"
    echo -e "${GREEN}╚══════════════════════════════════════════╝${NC}"
    echo ""
    echo "📦 Image information:"
    echo "   Local image: ${FULL_IMAGE}"
    echo ""
    echo "💡 Usage:"
    echo ""
    echo "   Run container:"
    echo "     docker run -d --name tspserver -p 9000:9000 ${FULL_IMAGE}"
    echo ""
    echo "   Run with environment variables:"
    echo "     docker run -d --name tspserver -p 9000:9000 \\"
    echo "       -e TSP_PORT=8080 \\"
    echo "       ${FULL_IMAGE}"
    echo ""
    echo "   Mount custom config:"
    echo "     docker run -d --name tspserver -p 9000:9000 \\"
    echo "       -v /path/to/config:/app/config \\"
    echo "       ${FULL_IMAGE} -- --root /app/www --port 9000"
    echo ""

    # Show image size
    IMAGE_SIZE=$(docker images ${IMAGE_NAME}:${IMAGE_TAG} --format "{{.Size}}")
    echo "   Image size: ${IMAGE_SIZE}"
    echo ""
else
    echo -e "${RED}╔══════════════════════════════════════════╗${NC}"
    echo -e "${RED}║     ❌ Build failed!                    ║${NC}"
    echo -e "${RED}╚══════════════════════════════════════════╝${NC}"
    echo ""
    exit 1
fi

# Save build info
echo "$(date)" > "$PROJECT_ROOT/.docker_build_info"
echo "Image: ${FULL_IMAGE}" >> "$PROJECT_ROOT/.docker_build_info"
echo "Size: ${IMAGE_SIZE}" >> "$PROJECT_ROOT/.docker_build_info"
