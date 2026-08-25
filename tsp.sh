#!/usr/bin/env bash
set -euo pipefail

# TSP Bun workflow. Page modules are intentionally not bundled into the
# executable: the TSP-enabled Bun runtime loads ./www from the real filesystem.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT_DIR"

require_bun() {
  if [[ -n "${TSP_BUN_BIN:-}" ]]; then
    if [[ ! -f "$TSP_BUN_BIN" ]]; then
      echo "Error: TSP_BUN_BIN does not exist: $TSP_BUN_BIN" >&2
      exit 1
    fi
    BUN_BIN=("$TSP_BUN_BIN")
    return
  fi

  local candidate
  local candidates=(
    "$ROOT_DIR/bun/build/debug/bun-debug.exe"
    "$ROOT_DIR/bun/build/debug/bun-debug"
    "$ROOT_DIR/bun/build/release/bun.exe"
    "$ROOT_DIR/bun/build/release/bun"
  )
  for candidate in "${candidates[@]}"; do
    if [[ -f "$candidate" ]]; then
      BUN_BIN=("$candidate")
      return
    fi
  done

  if command -v bun >/dev/null 2>&1; then
    BUN_BIN=("$(command -v bun)")
    return
  fi

  {
    echo "Error: Bun is required."
    echo "Build the bundled fork or install Bun 1.x."
    echo "You can also set TSP_BUN_BIN to an explicit Bun executable."
  } >&2
  exit 1
}

run_bun() {
  "${BUN_BIN[@]}" "$@"
}

require_bun_and_report() {
  require_bun
  echo "Using Bun: ${BUN_BIN[0]}"
}

run_server() {
  require_bun_and_report
  exec "${BUN_BIN[@]}" run src/main.ts "$@"
}

build_server() {
  require_bun_and_report
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
  run_bun build src/main.ts --compile --outfile "$output_dir/tspserver" "$@"
  if [[ -d "$ROOT_DIR/www" ]]; then
    rm -rf "$output_dir/www"
    cp -R "$ROOT_DIR/www" "$output_dir/www"
  fi
  if [[ -f "$ROOT_DIR/config.jsonc" ]]; then
    cp "$ROOT_DIR/config.jsonc" "$output_dir/config.jsonc"
  fi
  echo "Built $output_dir/tspserver with external www/ source tree"
}

build_v2_server() {
  local mode="${1:-debug}"
  local output_dir="$ROOT_DIR/dist/tsp-v2"
  local cargo_args=(--manifest-path "$ROOT_DIR/bun/Cargo.toml" -p bun_runtime_tsp --bin tspserver_v2)
  if [[ "$mode" == "release" ]]; then
    cargo_args+=(--release)
  fi

  mkdir -p "$output_dir"
  echo "Building TSP v2 host ($mode)..."
  cargo build "${cargo_args[@]}"

  local binary="$ROOT_DIR/bun/target/$mode/tspserver_v2"
  if [[ -f "$binary.exe" ]]; then
    binary="$binary.exe"
  fi
  if [[ ! -f "$binary" ]]; then
    echo "Error: v2 host binary was not produced: $binary" >&2
    exit 1
  fi
  cp "$binary" "$output_dir/$(basename "$binary")"
  echo "Built $output_dir/$(basename "$binary")"
}

run_tests() {
  require_bun_and_report
  local command_name="$1"
  shift
  case "$command_name" in
    test:unit) run_bun test tests/unit "$@" ;;
    test:e2e) run_bun test tests/e2e "$@" ;;
    *) run_bun test "$@" ;;
  esac
}

run_check() {
  require_bun_and_report
  run_bun x tsc --noEmit
}

run_fmt() {
  require_bun_and_report
  run_bun x prettier --write src tests types.d.ts
}

run_lint() {
  require_bun_and_report
  run_bun x eslint src tests
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
  build:tspserver:v2|compile:v2)
    shift
    build_v2_server debug "$@"
    ;;
  build:tspserver:v2:rel|compile:v2:rel)
    shift
    build_v2_server release "$@"
    ;;
  test|test:unit|test:e2e)
    run_tests "$@"
    ;;
  check)
    run_check
    ;;
  fmt)
    run_fmt
    ;;
  lint)
    run_lint
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
  build:tspserver:v2          Compile the native TSP v2 host
  build:tspserver:v2:rel      Compile the native TSP v2 host in release mode
  test / test:unit / test:e2e Run Bun tests
  check                       Run TypeScript validation
  fmt                         Format source files with Prettier
  lint                        Lint source files with ESLint
  clean                       Remove Bun build output

  The bundled bun/build/debug/bun-debug(.exe) is preferred automatically.
  Set TSP_BUN_BIN to override the Bun executable.
EOF
    ;;
  *)
    echo "Unknown command: $1" >&2
    exit 2
    ;;
esac
