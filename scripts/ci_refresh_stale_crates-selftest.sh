#!/usr/bin/env bash
# Self-test for scripts/ci_refresh_stale_crates.sh — the step that keeps
# the pipeline from reusing a cached Cargo artifact built from another
# branch's tree.
#
# The step runs in every compiling job and decides, from file mtimes
# alone, whether cargo may trust the cache. A wrong decision in either
# direction is expensive: refreshing too much throws the warm cache away,
# refreshing too little lets cargo compile against another tree's crate.
# So both directions are demonstrated here, with the mtimes laid out by
# hand, against a synthetic workspace and a synthetic cache.
#
# The `delta` case is the regression for the defect that shipped first:
# cargo rewrites a fingerprint FILE in place and never touches the
# fingerprint DIRECTORY's mtime, so a comparison against the directory
# sees a months-old timestamp and decides that every source is newer
# than the cache. That is how GitForge run 90f22916 failed with this
# step already in place.
#
# Usage: bash scripts/ci_refresh_stale_crates-selftest.sh
set -uo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
script="$root/scripts/ci_refresh_stale_crates.sh"

work="$(mktemp -d)"
trap 'find "$work" -type f -delete; find "$work" -type d -empty -delete' EXIT

failures=0
passed=0

ok() {
  printf 'ok   %s\n' "$1"
  passed=$((passed + 1))
}

bad() {
  printf 'FAIL %s\n' "$1" >&2
  shift
  for line in "$@"; do
    printf '     %s\n' "$line" >&2
  done
  failures=$((failures + 1))
}

# --- a synthetic workspace and a synthetic cache ---------------------------
ws="$work/workspace"
target="$work/target"
mkdir -p "$ws/crates/alpha/src" "$ws/crates/beta/src" "$ws/crates/gamma/src" \
  "$ws/crates/delta/src" "$ws/xtask/src" \
  "$target/debug/.fingerprint/alpha-1111111111111111" \
  "$target/debug/.fingerprint/beta-2222222222222222" \
  "$target/debug/.fingerprint/delta-4444444444444444" \
  "$target/wasm32-unknown-unknown/debug/.fingerprint/gamma-3333333333333333"

printf '[package]\nname = "alpha"\n' >"$ws/crates/alpha/Cargo.toml"
printf 'pub fn alpha() {}\n' >"$ws/crates/alpha/src/lib.rs"
printf '[package]\nname = "beta"\n' >"$ws/crates/beta/Cargo.toml"
printf 'pub fn beta() {}\n' >"$ws/crates/beta/src/lib.rs"
printf '[package]\nname = "gamma"\n' >"$ws/crates/gamma/Cargo.toml"
printf 'pub fn gamma() {}\n' >"$ws/crates/gamma/src/lib.rs"
printf '[package]\nname = "delta"\n' >"$ws/crates/delta/Cargo.toml"
printf 'pub fn delta() {}\n' >"$ws/crates/delta/src/lib.rs"
printf '[package]\nname = "xtask"\n' >"$ws/xtask/Cargo.toml"
printf 'fn main() {}\n' >"$ws/xtask/src/main.rs"

# The fingerprints are the cache's records of the last build.
for fp in "$target/debug/.fingerprint/alpha-1111111111111111/lib-alpha" \
  "$target/debug/.fingerprint/beta-2222222222222222/lib-beta" \
  "$target/debug/.fingerprint/delta-4444444444444444/lib-delta" \
  "$target/wasm32-unknown-unknown/debug/.fingerprint/gamma-3333333333333333/lib-gamma"; do
  printf 'fingerprint\n' >"$fp"
done

# alpha: the checkout is NEWER than the cache — cargo rebuilds it by
# itself, so the script must leave it alone.
# beta: the cache is NEWER than the checkout — the cache may hold an
# artifact built from another tree, so the script must refresh it.
# gamma: the crate lives under the wasm target — the script must look
# there too.
# delta: the cache is NEWER than the checkout, but the fingerprint
# DIRECTORY's mtime is old — the shape cargo leaves behind when it
# rewrites a fingerprint file in place. The crate must still be
# refreshed.
# xtask: no cached fingerprint at all — nothing to refresh.
now="$(date +%s)"
for f in "$ws/crates/alpha/src/lib.rs" "$ws/crates/alpha/Cargo.toml"; do
  touch -d "@$((now + 3600))" "$f"
done
for f in "$ws/crates/beta/src/lib.rs" "$ws/crates/beta/Cargo.toml" \
  "$ws/crates/gamma/src/lib.rs" "$ws/crates/gamma/Cargo.toml" \
  "$ws/crates/delta/src/lib.rs" "$ws/crates/delta/Cargo.toml" \
  "$ws/xtask/src/main.rs" "$ws/xtask/Cargo.toml"; do
  touch -d "@$((now - 3600))" "$f"
done
for fp in "$target/debug/.fingerprint/alpha-1111111111111111/lib-alpha" \
  "$target/debug/.fingerprint/beta-2222222222222222/lib-beta" \
  "$target/debug/.fingerprint/delta-4444444444444444/lib-delta" \
  "$target/wasm32-unknown-unknown/debug/.fingerprint/gamma-3333333333333333/lib-gamma"; do
  touch -d "@$((now - 1800))" "$fp"
done
# The fingerprint directories themselves keep an old mtime, as they do on
# a real cache: cargo rewrites the file inside them in place.
for dir in "$target/debug/.fingerprint/alpha-1111111111111111" \
  "$target/debug/.fingerprint/beta-2222222222222222" \
  "$target/debug/.fingerprint/delta-4444444444444444" \
  "$target/wasm32-unknown-unknown/debug/.fingerprint/gamma-3333333333333333"; do
  touch -d "@$((now - 86400 * 30))" "$dir"
done

alpha_before="$(stat -c %Y "$ws/crates/alpha/src/lib.rs")"
beta_before="$(stat -c %Y "$ws/crates/beta/src/lib.rs")"
gamma_before="$(stat -c %Y "$ws/crates/gamma/src/lib.rs")"
delta_before="$(stat -c %Y "$ws/crates/delta/src/lib.rs")"
xtask_before="$(stat -c %Y "$ws/xtask/src/main.rs")"

output="$(bash "$script" "$ws" "$target" 2>&1)"
status=$?

if [ "$status" -eq 0 ]; then
  ok "the script exits 0"
else
  bad "the script exits 0" "exit status $status" "$output"
fi

alpha_after="$(stat -c %Y "$ws/crates/alpha/src/lib.rs")"
if [ "$alpha_before" = "$alpha_after" ]; then
  ok "a crate newer than the cache is left alone"
else
  bad "a crate newer than the cache is left alone" "mtime $alpha_before -> $alpha_after"
fi

beta_after="$(stat -c %Y "$ws/crates/beta/src/lib.rs")"
if [ "$beta_before" != "$beta_after" ] && [ "$beta_after" -gt "$beta_before" ]; then
  ok "a crate older than the cache is refreshed"
else
  bad "a crate older than the cache is refreshed" "mtime $beta_before -> $beta_after"
fi

gamma_after="$(stat -c %Y "$ws/crates/gamma/src/lib.rs")"
if [ "$gamma_before" != "$gamma_after" ] && [ "$gamma_after" -gt "$gamma_before" ]; then
  ok "a crate under the wasm target directory is refreshed"
else
  bad "a crate under the wasm target directory is refreshed" "mtime $gamma_before -> $gamma_after"
fi

delta_after="$(stat -c %Y "$ws/crates/delta/src/lib.rs")"
if [ "$delta_before" != "$delta_after" ] && [ "$delta_after" -gt "$delta_before" ]; then
  ok "a crate is refreshed when the fingerprint FILE is newer but its directory is old"
else
  bad "a crate is refreshed when the fingerprint FILE is newer but its directory is old" \
    "mtime $delta_before -> $delta_after"
fi

xtask_after="$(stat -c %Y "$ws/xtask/src/main.rs")"
if [ "$xtask_before" = "$xtask_after" ]; then
  ok "a crate with no cached fingerprint is left alone"
else
  bad "a crate with no cached fingerprint is left alone" "mtime $xtask_before -> $xtask_after"
fi

# A missing workspace or cache is not an error: there is nothing to fix.
if bash "$script" "$work/nope" "$target" >/dev/null 2>&1; then
  ok "a missing workspace exits 0"
else
  bad "a missing workspace exits 0" "exit status $?"
fi

if bash "$script" "$ws" "$work/nope" >/dev/null 2>&1; then
  ok "a missing cache exits 0"
else
  bad "a missing cache exits 0" "exit status $?"
fi

printf '%s passed, %s failed\n' "$passed" "$failures"
[ "$failures" -eq 0 ]
