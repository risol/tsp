#!/bin/bash
# Update patch file
# Export all modifications from deno directory to deno_tsp_patch.diff

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"
DENO_DIR="$PROJECT_ROOT/deno"
PATCH_FILE="$PROJECT_ROOT/deno_tsp_patch.diff"

echo "=== Updating patch file ==="
echo ""

# Check if deno directory exists
if [ ! -d "$DENO_DIR" ]; then
    echo "Error: deno directory does not exist"
    exit 1
fi

cd "$DENO_DIR"

# Check if there are any modifications
if [ -z "$(git status --porcelain)" ]; then
    echo "Warning: deno directory has no modifications"
    exit 0
fi

# Show modification statistics
echo "Modified files:"
git diff --stat
echo ""

# Export all modifications to patch file
git diff --no-color > "$PATCH_FILE"

# Check if patch file was generated successfully
if [ -f "$PATCH_FILE" ] && [ -s "$PATCH_FILE" ]; then
    LINES=$(wc -l < "$PATCH_FILE")
    echo "✓ Patch file updated: $PATCH_FILE"
    echo "  Lines: $LINES"
else
    echo "Error: Patch file generation failed"
    exit 1
fi
