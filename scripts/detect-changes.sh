#!/usr/bin/env bash
# Conservative diff classifier for GitForge's downstream skip_if jobs.
set -euo pipefail
git fetch --quiet origin main --tags 2>/dev/null || true
BASE=""
if git rev-parse --verify origin/main >/dev/null 2>&1; then
  BASE=$(git merge-base origin/main HEAD 2>/dev/null || true)
fi
if [[ -z "$BASE" || "$BASE" == "$(git rev-parse HEAD)" ]]; then
  BASE=$(git rev-parse HEAD^1 2>/dev/null || true)
fi
CHANGED_FILES=""
if [[ -n "$BASE" ]]; then CHANGED_FILES=$(git diff --name-only "$BASE" HEAD 2>/dev/null || true); fi
IS_TAG=0
if git tag --points-at HEAD | grep . >/dev/null; then IS_TAG=1; fi
CODE=0; DOCS=0; CRATES=0; TESTS=0; INFRA=0; GENERATED=0
if [[ -z "$CHANGED_FILES" && "$IS_TAG" == "0" ]]; then CODE=1; fi
while IFS= read -r file; do
  [[ -z "$file" ]] && continue
  case "$file" in
    docs/planning/generated/*|crates/app/panels.snapshot)
      DOCS=1; GENERATED=1; continue ;;
    docs/*|README.md|CHANGELOG.md|CONTRIBUTING.md|LICENSE|CODE_OF_CONDUCT.md)
      DOCS=1; continue ;;
    tests/*|*/tests/*|*_test.rs|*.fw.png|corpus/*|*/fixtures/*)
      TESTS=1; continue ;;
    .github/*|.gitforge.yml|infrastructure/*|docker/*|scripts/*)
      INFRA=1; CODE=1; continue ;;
    crates/*/Cargo.toml|crates/*/src/*|xtask/*|web/*|Cargo.toml|Cargo.lock|rust-toolchain*|.cargo/*)
      CODE=1
      if [[ "$file" == crates/* ]]; then CRATES=1; fi
      continue ;;
    *.rs|*.toml|*.lock|*.yml|*.yaml|*.sh|*.py|*.js|*.ts|*.tsx|*.jsx|*.html|*.css|*.json)
      CODE=1; continue ;;
    *) CODE=1 ;;
  esac
done <<< "$CHANGED_FILES"
if [[ "$IS_TAG" == "1" ]]; then CODE=1; fi
COUNT=0
if [[ -n "$CHANGED_FILES" ]]; then COUNT=$(printf '%s\n' "$CHANGED_FILES" | wc -l | tr -d ' '); fi
echo "CI_CHANGE_FILE_COUNT=$COUNT"
echo "CI_CHANGE_INCLUDE_CODE=$CODE"
echo "CI_CHANGE_INCLUDE_DOCS=$DOCS"
echo "CI_CHANGE_INCLUDE_CRATES=$CRATES"
echo "CI_CHANGE_INCLUDE_TESTS=$TESTS"
echo "CI_CHANGE_INCLUDE_INFRA=$INFRA"
echo "CI_CHANGE_INCLUDE_GENERATED=$GENERATED"
echo "CI_CHANGE_IS_TAG=$IS_TAG"
