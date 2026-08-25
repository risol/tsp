#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 4 ]]; then
  echo "usage: $0 <tspserver_v2> <bun-worker> [routes-dir] [port]" >&2
  exit 2
fi

server=$(realpath "$1")
worker=$(realpath "$2")
source_routes=$(realpath "${3:-tests/v2_smoke/routes}")
port=${4:-9137}
[[ -x "$server" ]] || { echo "server binary is not executable: $server" >&2; exit 1; }
[[ -x "$worker" ]] || { echo "worker binary is not executable: $worker" >&2; exit 1; }
[[ -d "$source_routes" ]] || { echo "routes directory not found: $source_routes" >&2; exit 1; }

temp_root=$(mktemp -d "${TMPDIR:-/tmp}/tsp-v2-smoke.XXXXXX")
routes="$temp_root/routes"
mkdir -p "$routes"
cp -R "$source_routes/." "$routes/"
server_log="$temp_root/server.log"
pid=0

cleanup() {
  if [[ "$pid" -ne 0 ]] && kill -0 "$pid" 2>/dev/null; then kill "$pid" 2>/dev/null || true; fi
  if [[ "$pid" -ne 0 ]]; then wait "$pid" 2>/dev/null || true; fi
  rm -rf "$temp_root"
}
trap cleanup EXIT

TSP_PORT="$port" \
TSP_ROUTES_DIR="$routes" \
TSP_EMBEDDED_WORKER=1 \
TSP_WORKER_BIN="$worker" \
TSP_WORKER_COUNT=2 \
"$server" >"$server_log" 2>&1 &
pid=$!

for _ in $(seq 1 150); do
  if curl -fsS --max-time 1 "http://127.0.0.1:$port/" >/dev/null; then break; fi
  sleep 0.1
done
body=$(curl -fsS --max-time 30 "http://127.0.0.1:$port/")
[[ "$body" == *"Hello from TSP v2"* ]] || { cat "$server_log" >&2; exit 1; }
for _ in $(seq 1 4); do
  curl -fsS --max-time 30 "http://127.0.0.1:$port/" >/dev/null
done
metrics=$(curl -fsS --max-time 30 "http://127.0.0.1:$port/__tsp/metrics")
[[ "$metrics" == *"tsp_requests_total"* ]] || { cat "$server_log" >&2; exit 1; }

sed -i 's/Hello from TSP v2/Hello after reload/' "$routes/index.tsp"
for _ in $(seq 1 150); do
  body=$(curl -fsS --max-time 2 "http://127.0.0.1:$port/" || true)
  if [[ "$body" == *"Hello after reload"* ]]; then
    echo "TSP v2 embedded-worker smoke test passed"
    exit 0
  fi
  sleep 0.1
done
cat "$server_log" >&2
echo "v2 hot reload did not publish the changed route" >&2
exit 1
