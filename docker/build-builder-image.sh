#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGE_NAME="${1:-tspserver-v2-build-env}"
IMAGE_TAG="${2:-latest}"
IMAGE="${IMAGE_NAME}:${IMAGE_TAG}"

command -v docker >/dev/null 2>&1 || {
  echo "Error: Docker is required" >&2
  exit 1
}

echo "Building the reusable TSP Linux build environment ${IMAGE}..."
docker buildx build \
  --progress=plain \
  --platform linux/amd64 \
  --load \
  -t "$IMAGE" \
  -f "$SCRIPT_DIR/Dockerfile.build-env" \
  "$SCRIPT_DIR"

echo "Built reusable build environment ${IMAGE}"
