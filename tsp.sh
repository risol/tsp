#!/usr/bin/env bash
set -euo pipefail

# TSP's standalone compiler, native host, and process-worker workflow.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BUILD_DIR="${TSP_BUILD_DIR:-$ROOT_DIR/.tsp-build}"
ROUTES_DIR="${TSP_ROUTES_DIR:-$ROOT_DIR/pages}"
PUBLIC_DIR="${TSP_PUBLIC_DIR:-$ROOT_DIR/public}"
CONFIG_FILE="${TSP_CONFIG_FILE:-$ROOT_DIR/tsp.config.json}"

die() { echo "$*" >&2; exit 1; }
require_command() { command -v "$1" >/dev/null 2>&1 || die "$1 is required"; }

resolve_native_cli() {
  for candidate in \
    "$ROOT_DIR/native/target/release/tsp-cli.exe" \
    "$ROOT_DIR/native/target/release/tsp-cli" \
    "$ROOT_DIR/native/target/debug/tsp-cli.exe" \
    "$ROOT_DIR/native/target/debug/tsp-cli"; do
    if [[ -f "$candidate" ]]; then printf '%s\n' "$candidate"; return; fi
  done
  die "native TSP CLI not found; run './tsp.sh build:native'"
}

resolve_native_worker() {
  for candidate in \
    "$ROOT_DIR/native/target/release/tsp-worker.exe" \
    "$ROOT_DIR/native/target/release/tsp-worker" \
    "$ROOT_DIR/native/target/debug/tsp-worker.exe" \
    "$ROOT_DIR/native/target/debug/tsp-worker"; do
    if [[ -f "$candidate" ]]; then printf '%s\n' "$candidate"; return; fi
  done
  die "native TSP worker not found; run './tsp.sh build:native'"
}

require_sdk() {
  require_command cargo
  require_command node
  [[ -n "${TSP_JSC_SDK_ROOT:-}" ]] || die "TSP_JSC_SDK_ROOT must point to the TSP JSC SDK"
  [[ -d "$TSP_JSC_SDK_ROOT/include" && -d "$TSP_JSC_SDK_ROOT/lib" ]] ||
    die "TSP_JSC_SDK_ROOT must contain include and lib directories"
}

compile_routes() {
  require_command node
  mkdir -p "$BUILD_DIR"
  node "$ROOT_DIR/tools/tspc.mjs" compile --root "$ROUTES_DIR" --out "$BUILD_DIR"
}

build_native() {
  require_sdk
  compile_routes
  cargo build --manifest-path "$ROOT_DIR/native/Cargo.toml" --release -p tsp-cli -p tsp-worker
}

run_native() {
  local cli
  cli="$(resolve_native_cli)"
  [[ -s "$BUILD_DIR/manifest.json" ]] || compile_routes
  TSP_PORT="${TSP_PORT:-9000}" "$cli" \
    --manifest "$BUILD_DIR/manifest.json" \
    --listen "${TSP_LISTEN:-127.0.0.1:${TSP_PORT:-9000}}" \
    --workers "${TSP_WORKER_COUNT:-2}" "$@"
}

package_native() {
  local cli worker
  cli="$(resolve_native_cli)"
  worker="$(resolve_native_worker)"
  [[ -s "$BUILD_DIR/manifest.json" ]] || compile_routes
  mkdir -p "$ROOT_DIR/dist"
  bash "$ROOT_DIR/scripts/package-native-tspserver.sh" \
    "$cli" "$worker" "$ROOT_DIR/dist/tspserver" "$BUILD_DIR" "$PUBLIC_DIR" "$CONFIG_FILE"
}

run_tests() {
  require_command cargo
  cargo test --manifest-path "$ROOT_DIR/native/Cargo.toml" --workspace
}

run_check() {
  require_command cargo
  cargo check --manifest-path "$ROOT_DIR/native/Cargo.toml" --workspace
}

run_native_tests() {
  require_sdk
  (cd "$ROOT_DIR/tools" && npm test)
  npm test --prefix "$ROOT_DIR/native/runtime-js"
  run_tests
  TSP_NATIVE_E2E_ROOT="${TSP_NATIVE_E2E_ROOT:-$ROOT_DIR/native/fixtures/pages}" \
    node "$ROOT_DIR/scripts/native-e2e.mjs"
}

case "${1:-help}" in
  build|build:native|build:tspserver|build:tspserver:rel)
    build_native
    package_native
    ;;
  start|start:native|dev)
    shift
    require_sdk
    if [[ ! -s "$BUILD_DIR/manifest.json" ]] || ! resolve_native_cli >/dev/null 2>&1; then
      build_native
    fi
    run_native "$@"
    ;;
  test|test:native)
    run_native_tests
    ;;
  test:rust)
    run_tests
    ;;
  check)
    run_check
    ;;
  check:app)
    compile_routes
    ;;
  package)
    package_native
    ;;
  clean)
    rm -rf "$ROOT_DIR/dist/tspserver" "$BUILD_DIR"
    ;;
  help|-h|--help)
    cat <<'EOF'
Usage: ./tsp.sh <command>

  build                  Compile routes, build, and package the native server
  build:native           Compile routes and build the native host and worker
  start                  Start the standalone native server
  dev                    Start the standalone native server for development
  test                   Run compiler, runtime-js, Rust, and application E2E tests
  test:rust              Run the native Rust workspace tests
  check                  Run cargo check for the native workspace
  check:app              Validate and compile the application routes
  package                Package the native host, worker, routes, and assets
  clean                  Remove generated native build and package output

Environment:
  TSP_JSC_SDK_ROOT       TSP JSC SDK root containing include/ and lib/
  TSP_ROUTES_DIR         Application route source directory (default: ./pages)
  TSP_PUBLIC_DIR         Static asset directory (default: ./public)
  TSP_BUILD_DIR          Compiled route artifact directory (default: ./.tsp-build)
  TSP_PORT               Native server port (default: 9000)
  TSP_LISTEN             Native server listen address
  TSP_WORKER_COUNT       Process worker count (default: 2)
EOF
    ;;
  *) die "unknown command: $1" ;;
esac
