#!/usr/bin/env bash
set -euo pipefail

# TSP Bun workflow. Page modules are intentionally not bundled into the
# executable: the TSP-enabled Bun runtime loads ./www from the real filesystem.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

command_exists() {
  command -v "$1" >/dev/null 2>&1
}

require_bun() {
  if ! command_exists bun; then
    echo "Error: Bun is required. Install Bun 1.x or the TSP-enabled Bun fork." >&2
    exit 1
  fi
}

run_server() {
  require_bun
  exec bun run src/main.ts "$@"
}

build_server() {
  require_bun
  local mode="${1:-release}"
  local output_dir="$ROOT_DIR/dist/bun"
  if [[ "$mode" == "dev" ]]; then
    mode="debug"
    shift
  elif [[ "$mode" == "release" ]]; then
    shift
  fi

  mkdir -p "$output_dir"
  echo "Building TSP server with Bun ($mode)..."
  bun build src/main.ts --compile --outfile "$output_dir/tspserver" "$@"
  if [[ -d "$ROOT_DIR/www" ]]; then
    rm -rf "$output_dir/www"
    cp -R "$ROOT_DIR/www" "$output_dir/www"
  fi
  if [[ -f "$ROOT_DIR/config.jsonc" ]]; then
    cp "$ROOT_DIR/config.jsonc" "$output_dir/config.jsonc"
  fi
  echo "Built $output_dir/tspserver with external www/ source tree"
}

case "${1:-help}" in
  dev)
    shift
    run_server --dev "$@"
    ;;
  start)
    shift
    run_server "$@"
    ;;
  build:tspserver|compile|build)
    shift
    build_server release "$@"
    ;;
  build:tspserver:dev|compile:dev)
    shift
    build_server dev "$@"
    ;;
  build:tspserver:rel|compile:rel)
    shift
    build_server release "$@"
    ;;
  test|test:unit|test:e2e)
    require_bun
    command_name="$1"
    shift
    case "$command_name" in
      test:unit) bun test tests/unit "$@" ;;
      test:e2e) bun test tests/e2e "$@" ;;
      *) bun test "$@" ;;
    esac
    ;;
  check)
    require_bun
    bunx tsc --noEmit
    ;;
  fmt)
    require_bun
    bunx prettier --write src tests types.d.ts
    ;;
  lint)
    require_bun
    bunx eslint src tests
    ;;
  clean)
    rm -rf "$ROOT_DIR/dist/bun" "$ROOT_DIR/tspserver" "$ROOT_DIR/tspserver.exe"
    ;;
  help|-h|--help)
    cat <<'EOF'
Usage: ./tsp.sh <command>

  dev                         Run Bun development server
  start                       Run Bun server
  build:tspserver             Compile tspserver with Bun
  build:tspserver:dev         Compile debug tspserver with Bun
  test / test:unit / test:e2e Run Bun tests
  check                       Run TypeScript validation
  fmt                         Format source files with Prettier
  lint                        Lint source files with ESLint
  clean                       Remove Bun build output
EOF
    ;;
  *)
    echo "Unknown command: $1" >&2
    exit 2
    ;;
esac
