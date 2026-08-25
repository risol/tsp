#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 || $# -gt 5 ]]; then
  echo "usage: $0 <tspserver_v2> <bun> [output-dir] [routes-dir] [public-dir]" >&2
  exit 2
fi

server_binary=$1
bun_binary=$2
output_dir=${3:-dist/tsp-v2}
routes_dir=${4:-routes}
public_dir=${5:-public}

[[ -f "$server_binary" ]] || { echo "server binary not found: $server_binary" >&2; exit 1; }
[[ -f "$bun_binary" ]] || { echo "Bun runtime binary not found: $bun_binary" >&2; exit 1; }

mkdir -p "$output_dir"
cp "$server_binary" "$output_dir/tspserver_v2"
cp "$bun_binary" "$output_dir/bun"
if [[ -d "$routes_dir" ]]; then cp -R "$routes_dir" "$output_dir/routes"; fi
if [[ -d "$public_dir" ]]; then cp -R "$public_dir" "$output_dir/public"; fi
cat > "$output_dir/tsp-v2-runtime.json" <<'JSON'
{
  "runtime": "tsp-v2",
  "server": "tspserver_v2",
  "bun": "bun",
  "routes": "routes",
  "public": "public",
  "resolver": "bundled-runtime"
}
JSON

echo "Packaged tspserver_v2 and bundled Bun runtime at $output_dir"
