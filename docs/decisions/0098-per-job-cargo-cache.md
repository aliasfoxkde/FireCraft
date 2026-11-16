# 0098 — The Cargo cache is one directory per job, not one per image

Date: 2026-11-11 · Tickets: [T27-15](../release/EXECUTION-BOARD.md) · Status: accepted

## Context

The GitForge pipeline is the authoritative receipt for a merge, and it is
slow enough to be the binding constraint on the whole release campaign.

The `firecraft-ci` pipeline is a chain of eight jobs — `fmt-layers`,
`clippy-test`, `wasm-web`, `corpus-golden`, `perf-budgets`, `fuzz`,
`generated-docs`, `release` — and every one of them compiles the workspace
from scratch. Cargo's target output went to a disposable directory inside
the job container, so no job ever started warm. Measured on the
`swarmone-docker` runner under fleet load on 2026-11-11: `fmt-layers`
10 min, `clippy-test` 31 min, `wasm-web` 27 min, `corpus-golden` 11 min,
`perf-budgets` 10.5 min, `fuzz` 29 min, `generated-docs` 11 min, and
`release` still running after ~90 min. One run's chain took about seven
hours before its last job started; a merge could not be claimed on a
receipt that had not been produced yet.

The runner already has the mechanism this needs. A job's `cache:` block
bind-mounts a host directory at `/cargo-target` and exports
`CARGO_TARGET_DIR` into the container, so the compiled artifacts live on
the host and survive the job.

That mechanism comes with a rule that decides the whole shape of the
change: **the runner takes an exclusive advisory lock on every writable
host directory a job mounts, and holds it for the job's whole lifetime**
(GitForge GF-11). The lock exists because two jobs writing one Cargo
target tree clobber each other's binaries — a measured failure on the DIL
repository on 2026-11-05, where two coverage jobs lost their test
binaries to a concurrent rebuild.

So the cache directory is a serialization point, not just a directory.

## Decision

1. **One cache directory per job, named for the job.**
   Every Cargo job declares its own source, for example
   `{cache_root}/cargo/{image}-clippy-test`, so no two jobs in the fleet
   ever contend for the same tree. A single directory per image — the
   bare default — would serialize every Cargo job in the fleet into one
   lane, which is the opposite of what the cache is for. Each job keeps
   its own warm tree across runs; two runs of the *same* job still
   contend, which is exactly the case the lock exists for.
2. **The offline validator enforces the shape.** It requires a `cache:`
   mapping on any job whose steps run Cargo, requires the source to name
   the job, rejects a source another job already uses, and rejects
   `CARGO_TARGET_DIR=/tmp/` in a step — the disposable-container shape
   this change replaces.
3. **The validator runs in CI.** The GitForge API has no dry-run
   endpoint and creating a pipeline needs authentication, so a malformed
   `.gitforge.yml` fails every run at parse time, after the commit has
   already landed. The GitHub backup workflow's `fmt` job now runs
   `scripts/validate_gitforge_pipeline.py`, which checks the same
   contract offline in seconds.

## Consequences

- The first run after this change is still cold: the cache directories
  do not exist yet, so that run pays the full compile cost. Every later
  run of every branch reuses the artifacts and starts warm. The change is
  therefore a net loss for exactly one run and a large win for all of
  them after it.
- The cache is keyed by image and job, not by branch: the branches share
  one tree, which is what makes a warm run possible for work that has
  not been merged yet.
- A cache that grows without bound is a real possibility. It lives under
  the runner's own home on the NAS mount, which had 2.2 TB free on
  2026-11-11; nothing prunes it yet. A prune policy is owed before the
  tree becomes a disk problem, and it is recorded as debt rather than
  silently assumed.
- The pipeline definition that governs a push is the one in the pushed
  commit, so a branch without this change keeps running cold. The change
  reaches other branches by merging `main` into them.
- The cache does not shorten the `release` job's work: `xtask -- dist`
  still builds the web bundle and the native set. It shortens the
  compile inside it, from the measured 88 min toward the warm cost.
  Whether the release path should run on every branch push at all is a
  separate question, still open in [T27-15](../release/EXECUTION-BOARD.md).
