#!/usr/bin/env bash
#
# Make this run's workspace crates rebuild from THIS run's sources.
#
# The GitForge pipeline's per-job Cargo cache is keyed by image and
# job, not by branch (decision 0098), so every branch's `clippy-test`
# job writes the same target tree. Cargo decides that a path
# dependency is fresh by comparing the source file's mtime against the
# mtime recorded when the artifact was built, and a source OLDER than
# that record counts as fresh.
#
# A run's checkout is written when the run starts, but the run may
# wait for the cache lock before it compiles. A branch that checked
# out later can therefore build the cache in between, and this run —
# whose sources carry the older checkout's timestamps — sees that
# branch's artifacts as fresh and compiles against them.
#
# Measured 2026-11-15: PR #155's clippy job failed with E0599 "no
# method named `id` found for struct `engine::Session`" on a tree
# whose engine does define `id`; the job's log shows no "Checking
# engine", because cargo reused another branch's engine rlib. The
# same shape appeared in three other branches (E0063 `stack_mode`,
# E0609 `affordances`).
#
# The guard: make the workspace sources newer than any artifact the
# cache holds, so cargo rebuilds them from this run's tree. The
# runner holds the cache directory's lock for the whole job, so every
# artifact in it was written before this step runs — a timestamp of
# "now" is therefore newer than all of them. Third-party artifacts are
# untouched: they are versioned by the registry and cannot be stale.
#
# The debt this leaves: the cache is still shared across branches, so
# every job that compiles the workspace has to run this guard. A
# per-branch cache directory in GitForge would remove the need for it
# — see the debt note in decision 0127.

set -euo pipefail

for dir in crates xtask web fuzz; do
  if [ ! -d "$dir" ]; then
    continue
  fi
  # The sources cargo tracks: the crate's own files and its manifest.
  find "$dir" -type f \( -name '*.rs' -o -name 'Cargo.toml' \) -exec touch {} +
done
for file in Cargo.toml Cargo.lock rust-toolchain.toml; do
  if [ -f "$file" ]; then
    touch "$file"
  fi
done
