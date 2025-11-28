#!/usr/bin/env bash
set -euo pipefail

# Generate changelogs from scratch by iterating through all tags
# Usage: ./scripts/generate-changelogs.sh [NEW_VERSION]
# Example: ./scripts/generate-changelogs.sh v0.2.2

NEW_VERSION=${1:-""}
WORKSPACE_ROOT=$(git rev-parse --show-toplevel)
cd "$WORKSPACE_ROOT"

# Directories to generate per-crate changelogs for
CRATE_DIRS=("scotty-core" "scotty-types" "scotty" "scottyctl" "ts-generator" "frontend")

echo "=== Generating changelogs from scratch ==="

# Phase 1: Delete existing changelogs and create empty ones
echo "Phase 1: Cleaning up old changelogs..."
rm -f CHANGELOG.md
touch CHANGELOG.md
for dir in "${CRATE_DIRS[@]}"; do
  rm -f "$dir/CHANGELOG.md"
  touch "$dir/CHANGELOG.md"
done

# Phase 2: Iterate over tags from old to new
echo "Phase 2: Processing historical tags..."

# Get all tags matching the pattern, sorted chronologically (oldest first)
# Store in temp file to avoid subshell issues
TAGS_FILE=$(mktemp)
git tag --sort=creatordate | grep -E '^v[0-9]' > "$TAGS_FILE"

PREV_TAG=""
while IFS= read -r TAG; do
  if [ -z "$PREV_TAG" ]; then
    # First tag: generate from beginning to first tag
    RANGE="$TAG"
    echo "  Processing initial range: up to $TAG"
  else
    # Subsequent tags: generate from previous to current
    RANGE="$PREV_TAG..$TAG"
    echo "  Processing range: $RANGE"
  fi
  
  # Generate for root changelog
  git cliff "$RANGE" --tag "$TAG" -p CHANGELOG.md 2>/dev/null || true
  
  # Generate for each crate/directory (only if there are commits for that path)
  for dir in "${CRATE_DIRS[@]}"; do
    # Check if there are any commits in this range that touch this directory
    COMMIT_COUNT=$(git log "$RANGE" --oneline -- "$dir/" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$COMMIT_COUNT" -gt 0 ]; then
      git cliff "$RANGE" --tag "$TAG" --include-path "$dir/*" -p "$dir/CHANGELOG.md" 2>/dev/null || true
    fi
  done
  
  PREV_TAG="$TAG"
done < "$TAGS_FILE"

rm -f "$TAGS_FILE"

# Phase 3: Add unreleased commits with new version tag
if [ -n "$NEW_VERSION" ]; then
  echo "Phase 3: Adding unreleased commits as $NEW_VERSION..."
  
  # Get the latest tag
  LATEST_TAG=$(git tag --sort=-creatordate | grep -E '^v[0-9]' | head -1)
  
  # Generate unreleased section with new version
  git cliff "$LATEST_TAG..HEAD" --unreleased --tag "$NEW_VERSION" -p CHANGELOG.md
  
  # Generate for each crate/directory (only if there are commits for that path)
  for dir in "${CRATE_DIRS[@]}"; do
    # Check if there are any unreleased commits that touch this directory
    COMMIT_COUNT=$(git log "$LATEST_TAG..HEAD" --oneline -- "$dir/" 2>/dev/null | wc -l | tr -d ' ')
    if [ "$COMMIT_COUNT" -gt 0 ]; then
      git cliff "$LATEST_TAG..HEAD" --unreleased --tag "$NEW_VERSION" --include-path "$dir/*" -p "$dir/CHANGELOG.md" 2>/dev/null || true
    fi
  done
  
  echo "✓ Changelogs generated with version $NEW_VERSION"
else
  echo "Phase 3: Skipped (no new version specified)"
  echo "✓ Changelogs generated up to latest tag"
fi

echo ""
echo "=== Changelog generation complete ==="
echo "Files updated:"
echo "  - CHANGELOG.md"
for dir in "${CRATE_DIRS[@]}"; do
  if [ -f "$dir/CHANGELOG.md" ]; then
    echo "  - $dir/CHANGELOG.md"
  fi
done

# Stage all changelogs for commit
echo ""
echo "Staging changelogs for commit..."
git add CHANGELOG.md */CHANGELOG.md frontend/CHANGELOG.md

echo "✓ All changelogs staged"
