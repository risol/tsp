#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
OUTPUT_DIR="${1:-$PROJECT_ROOT/dist/tsp-v2-linux-x64}"
BUILDER_IMAGE="${TSP_BUILDER_IMAGE:-tspserver-v2-build-env:latest}"
TSP_BUILD_REVISION="${GITHUB_SHA:-${GIT_SHA:-}}"

if [ -z "$TSP_BUILD_REVISION" ]; then
  TSP_BUILD_REVISION="$(git -C "$PROJECT_ROOT" rev-parse HEAD 2>/dev/null || true)"
fi

if [ -z "$TSP_BUILD_REVISION" ]; then
  TSP_BUILD_REVISION="unknown"
fi

command -v docker >/dev/null 2>&1 || {
  echo "Error: Docker is required" >&2
  exit 1
}

mkdir -p "$OUTPUT_DIR"
rm -f "$OUTPUT_DIR/tspserver_v2"

if ! docker image inspect "$BUILDER_IMAGE" >/dev/null 2>&1; then
  echo "Build environment ${BUILDER_IMAGE} was not found; creating it..."
  bash "$SCRIPT_DIR/build-builder-image.sh" "${BUILDER_IMAGE%:*}" "${BUILDER_IMAGE##*:}"
fi

echo "Building tspserver_v2 for Linux in Docker..."
docker buildx build \
  --progress=plain \
  --platform linux/amd64 \
  --build-arg "BUILDER_IMAGE=$BUILDER_IMAGE" \
  --build-arg "GIT_SHA=$TSP_BUILD_REVISION" \
  --target artifact \
  --output "type=local,dest=$OUTPUT_DIR" \
  -f "$SCRIPT_DIR/Dockerfile.build-linux" \
  "$PROJECT_ROOT/bun"

test -x "$OUTPUT_DIR/tspserver_v2" || {
  echo "Error: Docker build did not produce $OUTPUT_DIR/tspserver_v2" >&2
  exit 1
}

echo "Built Linux tspserver_v2: $OUTPUT_DIR/tspserver_v2"
if command -v file >/dev/null 2>&1; then
  file "$OUTPUT_DIR/tspserver_v2"
fi
