#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 4 ]]; then
  echo "usage: $0 <tspserver_v2> [output-dir] [routes-dir] [public-dir]" >&2
  exit 2
fi

server_binary=$1
output_dir=${2:-dist/tsp-v2}
routes_dir=${3:-routes}
public_dir=${4:-public}

[[ -f "$server_binary" ]] || { echo "server binary not found: $server_binary" >&2; exit 1; }
mkdir -p "$output_dir"
if [[ "$server_binary" == *.exe ]]; then server_name=tspserver_v2.exe; else server_name=tspserver_v2; fi
server_target="$output_dir/$server_name"

if [[ "$(realpath "$server_binary")" != "$(realpath "$server_target" 2>/dev/null || true)" ]]; then
  cp "$server_binary" "$server_target"
fi
rm -rf "$output_dir/routes" "$output_dir/public"
if [[ -d "$routes_dir" ]]; then cp -R "$routes_dir" "$output_dir/routes"; fi
if [[ -d "$public_dir" ]]; then cp -R "$public_dir" "$output_dir/public"; fi

# v2.4 distribution contract: the packaged directory must NOT
# ship a standalone `bun(.exe)`. The master self-spawns (Windows)
# or pre-forks (Unix) the same `tspserver_v2[.exe]`; shipping a
# separate Bun would be a regression of the v2 single-binary
# contract. Pre-existing standalone files are removed so a
# re-packaging against a stale dist/tsp-v2 stays clean.
if [[ -f "$output_dir/bun" ]]; then
  echo "removing stale standalone bun from $output_dir (v2.4 ships a single binary)" >&2
  rm -f "$output_dir/bun"
fi
if [[ -f "$output_dir/bun.exe" ]]; then
  echo "removing stale standalone bun.exe from $output_dir (v2.4 ships a single binary)" >&2
  rm -f "$output_dir/bun.exe"
fi

cat > "$output_dir/tsp-v2-runtime.json" <<JSON
{
  "runtime": "tsp-v2",
  "server": "$server_name",
  "worker": "$server_name",
  "embedded_worker": true,
  "routes": "routes",
  "public": "public",
  "resolver": "bundled-runtime"
}
JSON

echo "Packaged single-file TSP runtime $server_name at $output_dir"
