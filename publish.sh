#!/usr/bin/env bash

set -euo pipefail

VERSION=$(cargo metadata --no-deps --format-version 1 | python3 -c "import sys,json; print(json.load(sys.stdin)['packages'][0]['version'])")

echo "Publishing miden-watch v${VERSION} to crates.io..."

# Ensure working tree is clean
if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Error: working tree has uncommitted changes" >&2
  exit 1
fi

# Ensure we're on main and up to date
BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$BRANCH" != "main" ]; then
  echo "Error: must publish from main branch (current: ${BRANCH})" >&2
  exit 1
fi

git fetch origin main --quiet
if ! git merge-base --is-ancestor origin/main HEAD; then
  echo "Error: branch is behind origin/main, pull first" >&2
  exit 1
fi

cargo publish --locked

echo "Published miden-watch v${VERSION}"
echo "Tag this release with: git tag v${VERSION} && git push origin v${VERSION}"
