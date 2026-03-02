#!/bin/bash
# Apply patch file
# Apply all modifications from deno_tsp_patch.diff to deno directory

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"
DENO_DIR="$PROJECT_ROOT/deno"
PATCH_FILE="$PROJECT_ROOT/deno_tsp_patch.diff"

# Check if skip confirmation (environment variable or argument)
SKIP_CONFIRM="${1:-}"
if [ "$SKIP_CONFIRM" = "--yes" ] || [ "$SKIP_CONFIRM" = "-y" ]; then
    SKIP_CONFIRM=1
fi

echo "=== Applying patch file ==="
echo ""

# Check if patch file exists
if [ ! -f "$PATCH_FILE" ]; then
    echo "Error: Patch file does not exist: $PATCH_FILE"
    exit 1
fi

# Check if patch file is empty
if [ ! -s "$PATCH_FILE" ]; then
    echo "Error: Patch file is empty"
    exit 1
fi

# Check if deno directory exists
if [ ! -d "$DENO_DIR" ]; then
    echo "Error: deno directory does not exist"
    exit 1
fi

cd "$DENO_DIR"

# Check for uncommitted changes
if [ -n "$(git status --porcelain)" ]; then
    if [ -z "$SKIP_CONFIRM" ]; then
        echo "Warning: deno directory has uncommitted changes"
        echo "It is recommended to commit or stash current changes first"
        echo ""
        read -p "Continue? (y/n) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            echo "Cancelled"
            exit 0
        fi
    else
        echo "Skipping confirmation check (--yes mode)"
    fi
fi

# Apply patch
echo "Applying patch..."
if git apply --check "$PATCH_FILE" 2>/dev/null; then
    git apply "$PATCH_FILE"
    echo "✓ Patch applied successfully"
    echo ""
    echo "Modified files:"
    git diff --stat cli/
else
    echo "Error: Patch application failed, there may be conflicts"
    echo ""
    echo "Trying --3way merge mode..."
    if git apply --3way "$PATCH_FILE" 2>/dev/null; then
        echo "✓ Patch applied successfully (using 3way merge)"
    else
        exit 1
    fi
fi
