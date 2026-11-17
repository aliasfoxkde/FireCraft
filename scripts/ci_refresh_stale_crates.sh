#!/usr/bin/env bash
# ci_refresh_stale_crates.sh — make cargo distrust a cached artifact that
# may have been built from a DIFFERENT tree.
#
# WHY THIS EXISTS. The pipeline's Cargo cache is mounted per JOB, not per
# commit: every branch that runs the same job shares one warm target
# directory (`.gitforge.yml` — the cache source is
# `{cache_root}/cargo/{image}-<job-name>`). Cargo decides that a cached
# dependency artifact is fresh by comparing the source files' mtimes with
# the mtime it recorded in that artifact's fingerprint: a source whose
# mtime is OLDER than the recorded one counts as fresh. The pipeline's
# workspace is checked out when the job starts, but the cache it mounts
# may have been rebuilt by ANOTHER branch's run in between — and that
# other checkout can be NEWER than this one. When it is, cargo concludes
# that this run's sources are older than the cached build and reuses the
# artifact, even though the artifact was built from another branch's
# source.
#
# The failure is a false red, and it is silent about its cause: the log
# shows a compile error for a function that IS in the checked-out tree.
# Measured 2026-11-16 on branch `climb/C8.04-missing-docs-campaign`
# (GitForge run 90f22916): the `layers` job failed with
#
#     error[E0425]: cannot find function `menu_census` in module `app::menus`
#       --> xtask/src/climb.rs:140:39
#
# while `crates/app/src/menus.rs` in that same tree defines the function
# (line 2324), and neither `main` nor the chrome branch defines it at
# all. The timeline, from the runner's own log: the chrome branch's
# `layers` job held the cache and rebuilt `app` at 15:07:46, writing
# `app`'s fingerprint file; the campaign's `layers` job took the cache
# lock at 15:08:29 — AFTER the chrome build — and its checkout was from
# 15:04, so cargo served the chrome branch's `app` artifact and xtask was
# compiled against a crate that has no `menu_census`.
#
# WHAT IT DOES. For every workspace crate, compare the crate's sources
# with the newest cached fingerprint of that crate. If a source is
# already newer, cargo will rebuild the crate on its own and this script
# leaves it alone. If NO source is newer, the cache may hold an artifact
# from another tree, so the crate's sources are refreshed and cargo
# rebuilds it. Crates whose sources are newer — the normal case — keep
# the warm cache, so this costs nothing when there is nothing to fix.
#
# THE REFERENCE IS THE FINGERPRINT FILE, NOT ITS DIRECTORY. Cargo
# rewrites a fingerprint file in place (`File::create` + write), which
# updates the FILE's mtime and leaves the enclosing directory's mtime
# where it was. A first version of this script compared the sources with
# the fingerprint DIRECTORY's mtime; that mtime is stale by months, so
# every source looked newer than the cache and nothing was ever
# refreshed — which is why the 90f22916 run above still failed with the
# refresh step in place. The comparison now uses the newest FILE in the
# crate's fingerprint directory, which is the timestamp cargo itself
# compares against.
#
# The platform defect this compensates for is not here: the CI cache is
# scoped per job instead of per commit (or per run), and a job's
# workspace can be older than the cache it mounts. Until the platform
# scopes the cache per commit, this script keeps the pipeline's verdicts
# true. See docs/decisions/0122-the-cargo-cache-is-scoped-per-job-not-per-commit.md.
#
# Usage: bash scripts/ci_refresh_stale_crates.sh [workspace-root] [target-dir]
#
# The defaults are the CI's own paths: the workspace is mounted at
# /workspace and the cache at /cargo-target (CARGO_TARGET_DIR).
set -uo pipefail

root="${1:-/workspace}"
target="${2:-${CARGO_TARGET_DIR:-/cargo-target}}"

# A missing workspace or cache is not an error: there is nothing to
# refresh, and the job proceeds with whatever cargo does.
if [ ! -d "$root" ] || [ ! -d "$target" ]; then
  printf 'ci_refresh_stale_crates: nothing to refresh (root=%s target=%s)\n' "$root" "$target"
  exit 0
fi

# The crate's package name is its artifact's name in the fingerprint
# directory, which is not always the directory's name (crates/app-cli is
# `app-cli`, crates/web-export is `webexport`).
package_name() {
  sed -n 's/^name[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$1" | head -1
}

# One walk of the cache collects, per crate, the newest FILE in any of
# that crate's fingerprint directories. The fingerprint directories are
# named `<crate>-<hash>`, so the crate's name is the directory's name up
# to its last dash. The walk is a single pass because the cache lives on
# the NAS, where a listing costs; the depth covers the host target, the
# wasm target and the web crate's own target tree
# (`<target>/web/<triple>/debug/.fingerprint/`).
#
# The reference is a FILE, not its directory: cargo rewrites a
# fingerprint file in place, so the file's mtime is the build time cargo
# compares against while the directory's mtime stays where it was.
declare -A newest
while IFS= read -r fingerprint_file; do
  [ -n "$fingerprint_file" ] || continue
  directory="${fingerprint_file%/*}"
  base="${directory##*/}"
  name="${base%-*}"
  [ -n "$name" ] || continue
  if [ -z "${newest[$name]+x}" ] || [ "$fingerprint_file" -nt "${newest[$name]}" ]; then
    newest[$name]="$fingerprint_file"
  fi
done < <(find "$target" -maxdepth 7 -type f -path '*/.fingerprint/*/*' 2>/dev/null)

refreshed=0
checked=0

for manifest in "$root"/crates/*/Cargo.toml "$root"/xtask/Cargo.toml "$root"/web/Cargo.toml; do
  [ -f "$manifest" ] || continue
  dir="$(dirname "$manifest")"
  name="$(package_name "$manifest")"
  [ -n "$name" ] || continue
  checked=$((checked + 1))

  fingerprint="${newest[$name]-}"
  # No cached fingerprint for this crate: cargo has nothing to reuse.
  [ -n "$fingerprint" ] || continue

  # A source newer than the fingerprint is newer than the mtime cargo
  # recorded in it, so cargo rebuilds the crate by itself.
  newer="$(find "$dir" -type f \( -name '*.rs' -o -name '*.toml' \) \
    -newer "$fingerprint" -print -quit 2>/dev/null)"
  [ -z "$newer" ] || continue

  # Nothing in the crate is newer than the cache: refresh the sources so
  # cargo rebuilds it instead of reusing another tree's artifact.
  find "$dir" -type f \( -name '*.rs' -o -name '*.toml' \) \
    -exec touch {} + 2>/dev/null
  printf 'ci_refresh_stale_crates: refreshed %s (the cache is newer than the checkout)\n' "$name"
  refreshed=$((refreshed + 1))
done

printf 'ci_refresh_stale_crates: %s crate(s) checked, %s refreshed\n' "$checked" "$refreshed"
exit 0
