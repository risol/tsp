#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 || $# -gt 6 ]]; then
  echo "usage: $0 <tspserver> [output-dir] [pages-dir] [public-dir] [config-file] [tsp-worker]" >&2
  exit 2
fi

server_binary=$1
output_dir=${2:-dist/tspserver}
pages_dir=${3:-pages}
public_dir=${4:-public}
config_file=${5:-tsp.config.json}
worker_binary=${6:-}
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
agents_file="$script_dir/../docs/AGENTS.md"

[[ -f "$server_binary" ]] || { echo "server binary not found: $server_binary" >&2; exit 1; }
[[ -f "$config_file" ]] || { echo "config file not found: $config_file" >&2; exit 1; }
[[ -f "$agents_file" ]] || { echo "user guide not found: $agents_file" >&2; exit 1; }
if [[ -n "$worker_binary" && ! -f "$worker_binary" ]]; then
  echo "worker binary not found: $worker_binary" >&2
  exit 1
fi
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
cp "$agents_file" "$output_dir/AGENTS.md"

if [[ -n "$worker_binary" ]]; then
  if [[ "$worker_binary" == *.exe ]]; then worker_name=tsp-worker.exe; else worker_name=tsp-worker; fi
  cp "$worker_binary" "$output_dir/$worker_name"
fi

# Native process-worker distribution contract: the package contains the host
# plus its TSP-owned worker executable, never a separate JavaScript runtime.
# Pre-existing legacy runtime files are removed so re-packaging stays clean.
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
