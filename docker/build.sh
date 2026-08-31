#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
IMAGE_NAME="${1:-tspserver}"
IMAGE_TAG="${2:-latest}"

command -v docker >/dev/null 2>&1 || {
  echo "Error: Docker is required" >&2
  exit 1
}

echo "Building the TSP runtime package..."
bash "$SCRIPT_DIR/build-linux.sh" "$PROJECT_ROOT/dist/tspserver"

echo "Building Docker image ${IMAGE_NAME}:${IMAGE_TAG}..."
docker build -f "$SCRIPT_DIR/Dockerfile" \
  -t "${IMAGE_NAME}:${IMAGE_TAG}" \
  "$PROJECT_ROOT"

echo "Built ${IMAGE_NAME}:${IMAGE_TAG}"
