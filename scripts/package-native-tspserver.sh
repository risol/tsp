#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 6 ]]; then
  echo "usage: $0 <server-binary> <worker-binary> <output-dir> <routes-dir> <public-dir> <config-file>" >&2
  exit 2
fi

server_binary=$1
worker_binary=$2
output_dir=$3
routes_dir=$4
public_dir=$5
config_file=$6
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
agents_file="$script_dir/../docs/AGENTS.md"

for file in "$server_binary" "$worker_binary" "$config_file" "$agents_file"; do
  [[ -f "$file" ]] || { echo "required file not found: $file" >&2; exit 1; }
done
for directory in "$routes_dir" "$public_dir"; do
  [[ -d "$directory" ]] || { echo "required directory not found: $directory" >&2; exit 1; }
done
[[ -s "$routes_dir/manifest.json" ]] || { echo "compiled manifest is missing" >&2; exit 1; }
[[ -s "$routes_dir/bundle.js" ]] || { echo "compiled route bundle is missing" >&2; exit 1; }

rm -rf "$output_dir"
mkdir -p "$output_dir"
if [[ "$server_binary" == *.exe ]]; then
  server_name=tspserver.exe
  worker_name=tsp-worker.exe
else
  server_name=tspserver
  worker_name=tsp-worker
fi
cp "$server_binary" "$output_dir/$server_name"
cp "$worker_binary" "$output_dir/$worker_name"
cp -R "$routes_dir" "$output_dir/routes"
cp -R "$public_dir" "$output_dir/public"
cp "$config_file" "$output_dir/tsp.config.json"
cp "$agents_file" "$output_dir/AGENTS.md"

echo "Packaged native TSP server at $output_dir"
