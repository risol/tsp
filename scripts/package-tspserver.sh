#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 5 ]]; then
  echo "usage: $0 <tspserver> [output-dir] [pages-dir] [public-dir] [config-file]" >&2
  exit 2
fi

server_binary=$1
output_dir=${2:-dist/tspserver}
pages_dir=${3:-pages}
public_dir=${4:-public}
config_file=${5:-tsp.config.json}

[[ -f "$server_binary" ]] || { echo "server binary not found: $server_binary" >&2; exit 1; }
[[ -f "$config_file" ]] || { echo "config file not found: $config_file" >&2; exit 1; }
mkdir -p "$output_dir"
if [[ "$server_binary" == *.exe ]]; then server_name=tspserver.exe; else server_name=tspserver; fi
server_target="$output_dir/$server_name"

if [[ "$(realpath "$server_binary")" != "$(realpath "$server_target" 2>/dev/null || true)" ]]; then
  cp "$server_binary" "$server_target"
fi
rm -rf "$output_dir/pages" "$output_dir/routes" "$output_dir/public"
if [[ -d "$pages_dir" ]]; then cp -R "$pages_dir" "$output_dir/pages"; fi
if [[ -d "$public_dir" ]]; then cp -R "$public_dir" "$output_dir/public"; fi
if [[ "$(realpath "$config_file")" != "$(realpath "$output_dir/tsp.config.json" 2>/dev/null || true)" ]]; then
  cp "$config_file" "$output_dir/tsp.config.json"
fi

# embedded-worker distribution contract: the packaged directory must NOT
# ship a standalone `bun(.exe)`. The master self-spawns (Windows)
# or pre-forks (Unix) the same `tspserver[.exe]`; shipping a
# separate Bun would be a regression of the single-binary
# contract. Pre-existing standalone files are removed so a
# re-packaging against a stale dist/tspserver stays clean.
if [[ -f "$output_dir/bun" ]]; then
  echo "removing stale standalone bun from $output_dir (embedded-worker ships a single binary)" >&2
  rm -f "$output_dir/bun"
fi
if [[ -f "$output_dir/bun.exe" ]]; then
  echo "removing stale standalone bun.exe from $output_dir (embedded-worker ships a single binary)" >&2
  rm -f "$output_dir/bun.exe"
fi

# Do not leave the retired runtime manifest in a reused output directory.
if [[ -f "$output_dir/tspserver-runtime.json" ]]; then
  echo "removing retired runtime manifest from $output_dir" >&2
  rm -f "$output_dir/tspserver-runtime.json"
fi

echo "Packaged single-file TSP runtime $server_name at $output_dir"
