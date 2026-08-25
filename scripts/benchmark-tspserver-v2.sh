#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 4 ]]; then
  echo "usage: $0 <tspserver_v2> <bun> [routes-dir] [requests]" >&2
  exit 2
fi

server=$(realpath "$1")
bun=$(realpath "$2")
routes=$(realpath "${3:-routes}")
requests=${4:-50}
port=${TSP_BENCHMARK_PORT:-9140}
[[ -x "$server" ]] || { echo "server binary is not executable: $server" >&2; exit 1; }
[[ -x "$bun" ]] || { echo "Bun runtime is not executable: $bun" >&2; exit 1; }
[[ -d "$routes" ]] || { echo "routes directory not found: $routes" >&2; exit 1; }

if [[ "${TSP_BENCHMARK_EMBEDDED:-0}" == "1" ]]; then
  TSP_PORT="$port" TSP_ROUTES_DIR="$routes" TSP_EMBEDDED_WORKER=1 TSP_WORKER_BIN="$bun" "$server" >/tmp/tsp-v2-bench.out 2>/tmp/tsp-v2-bench.err &
else
  TSP_PORT="$port" TSP_ROUTES_DIR="$routes" TSP_BUN_BIN="$bun" "$server" >/tmp/tsp-v2-bench.out 2>/tmp/tsp-v2-bench.err &
fi
pid=$!
samples=$(mktemp)
cleanup() { rm -f "$samples"; kill "$pid" 2>/dev/null || true; }
trap cleanup EXIT

for _ in $(seq 1 100); do
  if curl -fsS --max-time 1 "http://127.0.0.1:$port/" >/dev/null; then break; fi
  sleep 0.1
done

start=$(date +%s%N)
curl -fsS "http://127.0.0.1:$port/" >/dev/null
end=$(date +%s%N)
cold_ms=$(( (end - start) / 1000000 ))
for _ in $(seq 1 "$requests"); do
  curl -fsS -o /dev/null -w '%{time_total}\n' "http://127.0.0.1:$port/" >>"$samples"
done
mapfile -t ordered < <(sort -n "$samples")
quantile() {
  local q=$1
  local n=${#ordered[@]}
  local index=$(( (n * q + 99) / 100 - 1 ))
  (( index < 0 )) && index=0
  (( index >= n )) && index=$((n - 1))
  awk -v value="${ordered[$index]}" 'BEGIN { printf "%.3f", value * 1000 }'
}
min_ms=$(head -n 1 "$samples" | awk '{printf "%.3f", $1 * 1000}')
max_ms=$(sort -n "$samples" | tail -n 1 | awk '{printf "%.3f", $1 * 1000}')
cat <<JSON
{"requests":$requests,"cold_ms":$cold_ms,"p50_ms":$(quantile 50),"p95_ms":$(quantile 95),"p99_ms":$(quantile 99),"min_ms":$min_ms,"max_ms":$max_ms}
JSON
