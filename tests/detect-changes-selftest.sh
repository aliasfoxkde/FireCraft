#!/usr/bin/env bash
set -euo pipefail
ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
DETECT="$ROOT/scripts/detect-changes.sh"
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
fail(){ echo "FAIL: $*" >&2; exit 1; }
get(){ sed -n "s/^$2=//p" <<< "$1"; }
make_repo(){
  local name=$1
  mkdir -p "$WORK/$name"
  git -C "$WORK/$name" init -q -b main
  git -C "$WORK/$name" config user.name Test
  git -C "$WORK/$name" config user.email test@example.invalid
  echo base > "$WORK/$name/README.md"
  git -C "$WORK/$name" add .
  git -C "$WORK/$name" commit -qm base
  git -C "$WORK/$name" init -q --bare "$WORK/$name-origin.git"
  git -C "$WORK/$name" remote add origin "$WORK/$name-origin.git"
  git -C "$WORK/$name" push -q -u origin main
  git -C "$WORK/$name" fetch -q origin main
}
run(){ (cd "$1" && bash "$DETECT"); }
make_repo docs
git -C "$WORK/docs" checkout -qb docs-only
mkdir -p "$WORK/docs/docs"
echo notes > "$WORK/docs/docs/architecture.md"
git -C "$WORK/docs" add .
git -C "$WORK/docs" commit -qm docs
git -C "$WORK/docs" push -q origin docs-only
git -C "$WORK/docs" fetch -q origin main
OUT=$(run "$WORK/docs")
[[ "$(get "$OUT" CI_CHANGE_INCLUDE_DOCS)" == 1 ]] || fail "docs flag missing"
[[ "$(get "$OUT" CI_CHANGE_INCLUDE_CODE)" == 0 ]] || fail "docs-only classified as code"
make_repo code
git -C "$WORK/code" checkout -qb source
mkdir -p "$WORK/code/crates/app/src"
echo 'fn main() {}' > "$WORK/code/crates/app/src/main.rs"
git -C "$WORK/code" add .
git -C "$WORK/code" commit -qm source
git -C "$WORK/code" push -q origin source
git -C "$WORK/code" fetch -q origin main
OUT=$(run "$WORK/code")
[[ "$(get "$OUT" CI_CHANGE_INCLUDE_CODE)" == 1 ]] || fail "Rust change not classified"
[[ "$(get "$OUT" CI_CHANGE_INCLUDE_CRATES)" == 1 ]] || fail "crate flag missing"
make_repo mainpush
echo 'fn main() {}' > "$WORK/mainpush/main.rs"
git -C "$WORK/mainpush" add .
git -C "$WORK/mainpush" commit -qm main-change
git -C "$WORK/mainpush" push -q origin main
OUT=$(run "$WORK/mainpush")
[[ "$(get "$OUT" CI_CHANGE_INCLUDE_CODE)" == 1 ]] || fail "main-push fallback missed source"
echo "FireCraft detect-changes self-test: passed"
