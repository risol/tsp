#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 3 ]]; then
  echo "usage: $0 <tspserver_v2> [routes-dir] [port]" >&2
  exit 2
fi

server=$(realpath "$1")
source_routes=$(realpath "${2:-tests/v2_smoke/routes}")
port=${3:-9137}
[[ -x "$server" ]] || { echo "server binary is not executable: $server" >&2; exit 1; }
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
TSP_WORKER_COUNT=2 \
"$server" >"$server_log" 2>&1 &
pid=$!

for _ in $(seq 1 150); do
  if ! kill -0 "$pid" 2>/dev/null; then
    # The master died during startup -- most likely a native / FFI
    # crash in the embedded-worker boot path. Dump the captured
    # stderr so the next CI log carries enough context to point
    # at the failing function instead of just "exit 139".
    echo "tspserver_v2 exited before becoming ready" >&2
    wait "$pid" || status=$?
    echo "exit status: ${status:-unknown}" >&2
    cat "$server_log" >&2
    exit 1
  fi
  if curl -fsS --max-time 1 "http://127.0.0.1:$port/" >/dev/null; then
    ready=1
    break
  fi
  sleep 0.1
done
if ! body=$(curl -fsS --max-time 30 "http://127.0.0.1:$port/"); then
  # The readiness loop just polled 150 times with 100ms gaps and
  # never saw a 200, yet the master is still alive. The final
  # long-timeout curl is the last gate before the assertions; if
  # it fails the server log is the only place the real reason
  # (slow startup, panic in a worker, etc.) lives.
  echo "server did not become ready within 30s" >&2
  cat "$server_log" >&2
  exit 1
fi
[[ "$body" == *"Hello from TSP v2"* ]] || { cat "$server_log" >&2; exit 1; }
for _ in $(seq 1 4); do
  curl -fsS --max-time 30 "http://127.0.0.1:$port/" >/dev/null
done
metrics=$(curl -fsS --max-time 30 "http://127.0.0.1:$port/__tsp/metrics")
[[ "$metrics" == *"tsp_requests_total"* ]] || { cat "$server_log" >&2; exit 1; }

# Hot-reload trigger: rewrite the route file in place. The
# previously-used `sed -i 's/.../.../' file` form is GNU-specific;
# BSD sed (macOS) parses the trailing path as a backup-suffix
# argument and aborts with `invalid command code f`. The
# `sed > new && mv` pipeline is portable across every POSIX
# `sed` and avoids leaving a `.bak` artefact on disk.
sed 's/Hello from TSP v2/Hello after reload/' "$routes/index.tsp" >"$routes/index.tsp.next"
mv "$routes/index.tsp.next" "$routes/index.tsp"
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
