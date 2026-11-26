#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WIKI_URL="https://github.com/aliasfoxkde/FireCraft.wiki.git"
TMP_WIKI="/tmp/firecraft-wiki-sync-$$"
trap 'rm -rf "${TMP_WIKI}"' EXIT

MODE="${1:---check}"
CONFIRM="${2:-}"

git clone --depth 1 "${WIKI_URL}" "${TMP_WIKI}" >/dev/null 2>&1

if [ "${MODE}" = "--check" ]; then
  echo "=== Diffing docs/wiki/ vs live FireCraft.wiki.git ==="
  diff -ru -x .git "${TMP_WIKI}" "${REPO_ROOT}/docs/wiki" || true
  exit 0
fi

if [ "${MODE}" = "--publish" ]; then
  cp "${REPO_ROOT}"/docs/wiki/*.md "${TMP_WIKI}/"
  cd "${TMP_WIKI}"
  git status --short
  if [ "${CONFIRM}" != "--confirm-push" ]; then
    echo "Dry-run complete. Re-run with: bash scripts/sync-wiki.sh --publish --confirm-push"
    exit 0
  fi
  git add -A
  if git diff --cached --quiet; then
    echo "Wiki already up to date."
    exit 0
  fi
  SRC_SHA="$(git -C "${REPO_ROOT}" rev-parse --short HEAD)"
  git commit -m "docs(wiki): sync from FireCraft ${SRC_SHA}"
  git push origin HEAD
  exit 0
fi

echo "Usage: $0 [--check | --publish [--confirm-push]]" >&2
exit 2
