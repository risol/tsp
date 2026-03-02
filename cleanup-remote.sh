#!/bin/bash
# Clean up remote repository - remove all branches, tags, and keep only current commit

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Cleaning up remote repository...${NC}"

# Check if we're in a git repository
if ! git rev-parse --git-dir > /dev/null 2>&1; then
    echo -e "${RED}Error: Not a git repository${NC}"
    exit 1
fi

# Get project root
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"

# Get version from deno.json
get_version() {
    local deno_json="$PROJECT_ROOT/deno.json"
    if [ -f "$deno_json" ]; then
        grep -m1 '"version"' "$deno_json" | sed 's/.*"version": *"\([^"]*\)".*/\1/'
    else
        echo "0.0.0"
    fi
}

VERSION=$(get_version)
echo "Version: $VERSION"

# Get remote name (default: origin)
REMOTE="${1:-origin}"

# Check if remote exists
if ! git remote get-url "$REMOTE" > /dev/null 2>&1; then
    echo -e "${RED}Error: Remote '$REMOTE' does not exist${NC}"
    exit 1
fi

echo "Remote: $REMOTE"

# Delete all remote tags
echo -e "${YELLOW}Deleting remote tags...${NC}"
for tag in $(git ls-remote --tags "$REMOTE" | cut -f2 | sed 's|refs/tags/||'); do
    if [ -n "$tag" ] && [ "$tag" != "${tag//^/}" ]; then
        git push "$REMOTE" --delete "$tag" 2>/dev/null || true
    fi
done
# Also delete tags with full ref
git ls-remote --tags "$REMOTE" | grep -v '\^{}' | cut -f2 | while read ref; do
    tag="${ref#refs/tags/}"
    if [ -n "$tag" ]; then
        git push "$REMOTE" --delete "$tag" 2>/dev/null || true
    fi
done

# Delete all remote branches (except master/main)
echo -e "${YELLOW}Deleting remote branches...${NC}"
for branch in $(git ls-remote --heads "$REMOTE" | cut -f2 | sed 's|refs/heads/||'); do
    if [ -n "$branch" ] && [ "$branch" != "master" ] && [ "$branch" != "main" ]; then
        git push "$REMOTE" --delete "$branch" 2>/dev/null || true
    fi
done

# Create orphan branch with current commit
echo -e "${YELLOW}Creating clean history...${NC}"
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)

# Delete temp branch if it already exists (both local and remote)
git branch -D temp-cleanup 2>/dev/null || true
git push "$REMOTE" --delete temp-cleanup 2>/dev/null || true

# Create temporary orphan branch
git checkout --orphan temp-cleanup

# Commit current state
git commit -m "Release v$VERSION" 2>/dev/null || echo "No changes to commit"

# Force push to target branch (master or main)
TARGET_BRANCH="master"
git push "$REMOTE" temp-cleanup:$TARGET_BRANCH --force

# Create and push version tag
echo -e "${YELLOW}Creating version tag v$VERSION...${NC}"
# Delete local tag if it exists
git tag -d "v$VERSION" 2>/dev/null || true
# Delete remote tag if it exists
git push "$REMOTE" --delete "v$VERSION" 2>/dev/null || true
# Create and push new tag
git tag -a "v$VERSION" -m "Release v$VERSION"
git push "$REMOTE" "v$VERSION"

# Switch back to original branch
git checkout "$CURRENT_BRANCH"

# Delete temporary branch
git branch -D temp-cleanup

echo -e "${GREEN}Done! Remote cleaned successfully.${NC}"
echo ""
echo "Remote: $REMOTE"
echo "Branch: $TARGET_BRANCH"
echo "Tag: v$VERSION"
echo "Commits: 1"
