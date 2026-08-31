#!/usr/bin/env bash
set -euo pipefail

# TSP build, test, and local runtime workflow.
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RUST_TOOLCHAIN="${TSP_RUST_TOOLCHAIN:-nightly-2026-07-20}"

die() { echo "$*" >&2; exit 1; }

resolve_file() {
  local candidate
  for candidate in "$@"; do
    if [[ -f "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

require_command() { command -v "$1" >/dev/null 2>&1 || die "$1 is required"; }

resolve_bun() {
  if [[ -n "${TSP_BUN_BIN:-}" ]]; then
    [[ -f "$TSP_BUN_BIN" ]] || die "TSP_BUN_BIN does not exist: $TSP_BUN_BIN"
    printf '%s\n' "$TSP_BUN_BIN"
    return 0
  fi
  if command -v bun >/dev/null 2>&1; then command -v bun; return 0; fi
  resolve_file "$ROOT_DIR/.bun-bootstrap/node_modules/bun/bin/bun.exe" "$ROOT_DIR/.bun-bootstrap/node_modules/bun/bin/bun" ||
    die "Bun is required; install Bun or set TSP_BUN_BIN"
}

resolve_runtime_binary() {
  resolve_file "$ROOT_DIR/bun/build/release/bun.exe" "$ROOT_DIR/bun/build/release/bun" "$ROOT_DIR/bun/build/debug/bun-debug.exe" "$ROOT_DIR/bun/build/debug/bun-debug" ||
    die "single-file TSP runtime not found; run './tsp.sh build:worker'"
}

resolve_host() {
  resolve_file "$ROOT_DIR/dist/tspserver/tspserver.exe" "$ROOT_DIR/dist/tspserver/tspserver" ||
    die "host not found; run './tsp.sh build:host'"
}

build_worker() {
  local bun_bin
  bun_bin="$(resolve_bun)"
  echo "Building the single-file TSP runtime..."
  (cd "$ROOT_DIR/bun" && "$bun_bin" run build:release)
}

build_host() {
  local binary
  binary="$(resolve_runtime_binary)"
  mkdir -p "$ROOT_DIR/dist/tspserver"
  if [[ "$binary" == *.exe ]]; then
    cp "$binary" "$ROOT_DIR/dist/tspserver/tspserver.exe"
  else
    cp "$binary" "$ROOT_DIR/dist/tspserver/tspserver"
  fi
  echo "Built single-file TSP runtime in $ROOT_DIR/dist/tspserver"
}

package_runtime() {
  local host
  host="$(resolve_host)"
  bash "$ROOT_DIR/scripts/package-tspserver.sh" "$host" "$ROOT_DIR/dist/tspserver" "$ROOT_DIR/routes" "$ROOT_DIR/public"
}

build_runtime() { build_worker; build_host; package_runtime; }

run_host() {
  local host
  host="$(resolve_host)"
  TSP_PORT="${TSP_PORT:-9000}" TSP_ROUTES_DIR="${TSP_ROUTES_DIR:-$ROOT_DIR/routes}" TSP_PUBLIC_DIR="${TSP_PUBLIC_DIR:-$ROOT_DIR/public}" TSP_EMBEDDED_WORKER="${TSP_EMBEDDED_WORKER:-1}" TSP_WORKER_COUNT="${TSP_WORKER_COUNT:-2}" "$host" "$@"
}

run_smoke() {
  local host
  host="$(resolve_host)"
  bash "$ROOT_DIR/scripts/smoke-tspserver.sh" "$host" "$ROOT_DIR/tests/smoke/routes" "${TSP_PORT:-9137}"
}

run_tests() {
  require_command rustup
  (cd "$ROOT_DIR/bun" && rustup run "$RUST_TOOLCHAIN" cargo test -p bun_runtime_tsp --lib --no-fail-fast --locked)
  (cd "$ROOT_DIR/bun" && rustup run "$RUST_TOOLCHAIN" cargo test -p bun_runtime_tsp --test worker_integration --no-fail-fast --locked -- --test-threads=1)
}

run_check() {
  require_command rustup
  rustup run "$RUST_TOOLCHAIN" cargo check --manifest-path "$ROOT_DIR/bun/Cargo.toml" -p bun_bin --locked
}

# Phase 11 tooling: thin wrappers that exec the host's
# introspection subcommands against the user's routes dir.
# They do not start the server; they read the routes and
# print / write the result, then exit.
run_routes() {
  local host
  host="$(resolve_host)"
  TSP_ROUTES_DIR="${TSP_ROUTES_DIR:-$ROOT_DIR/routes}" "$host" routes
}

run_graph() {
  local host
  host="$(resolve_host)"
  TSP_ROUTES_DIR="${TSP_ROUTES_DIR:-$ROOT_DIR/routes}" "$host" graph
}

run_check_app() {
  local host
  host="$(resolve_host)"
  TSP_ROUTES_DIR="${TSP_ROUTES_DIR:-$ROOT_DIR/routes}" "$host" check
}

run_typings() {
  local host out_dir
  host="$(resolve_host)"
  out_dir="${TSP_TYPINGS_DIR:-$ROOT_DIR/.tsp-types}"
  mkdir -p "$out_dir"
  "$host" typings --out "$out_dir"
}

case "${1:-help}" in
  build|build:tspserver|build:tspserver:rel) build_runtime ;;
  build:host) build_host ;;
  build:worker) build_worker ;;
  start|dev) shift; run_host "$@" ;;
  test) run_tests; run_smoke ;;
  test:rust) run_tests ;;
  test:smoke) run_smoke ;;
  check) run_check ;;
  check:app) run_check_app ;;
  routes) run_routes ;;
  graph) run_graph ;;
  typings) shift; run_typings "$@" ;;
  package) package_runtime ;;
  clean) rm -rf "$ROOT_DIR/dist/tspserver" ;;
  help|-h|--help)
    cat <<'EOF'
Usage: ./tsp.sh <command>

  build                  Build the single-file runtime and package
  build:host             Copy the built runtime into dist/tspserver
  build:worker           Build the single-file runtime
  start                  Run the server with self-created workers
  dev                    Run the server (route hot reload is always enabled)
  test                   Run Rust tests and the embedded-worker smoke test
  test:rust              Run Rust unit and Worker IPC tests
  test:smoke             Run the hot-reload smoke test
  check                  Run cargo check for the bundled runtime
  check:app              Run tsp check for the application routes
  routes                 List the application routes (tspserver routes)
  graph                  Print the application module graph
                          (tspserver graph)
  typings                Write the tsp:* TypeScript declaration files
                          (default output: ./.tsp-types; override with
                          --out <DIR> or TSP_TYPINGS_DIR)
  package                Package the single runtime binary
  clean                  Remove package output

Environment:
  TSP_PORT, TSP_ROUTES_DIR, TSP_PUBLIC_DIR, TSP_BUN_BIN
  TSP_TYPINGS_DIR        default --out target for the `typings` command
EOF
    ;;
  *) die "unknown command: $1" ;;
esac
