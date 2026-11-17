# 0127 — A CI run compiles against this run's sources, not the cache's

Date: 2026-11-16 · Status: accepted

## Context

The GitForge pipeline's per-job Cargo cache is keyed by image and job,
not by branch ([decision 0098](0098-per-job-cargo-cache.md)): every
branch's `clippy-test` job writes the same target tree, so a run of
unmerged work starts warm. That sharing is the point of the cache, and
it has a consequence nobody wrote down.

Cargo decides that a path dependency is fresh by comparing the source
file's mtime against the mtime recorded in the fingerprint when the
artifact was built, and **a source older than that record counts as
fresh**. There is no content check; cargo's content-based freshness is
nightly-only (`-Zchecksum-freshness`).

A run's checkout is written when the run starts, but the run then waits
for the cache directory's exclusive lock before it compiles (GitForge
GF-11 — the runner holds that lock for the job's whole lifetime). A
branch that checked out later can therefore build the cache in between.
When this run gets the lock, its sources carry the *older* checkout's
timestamps, so the other branch's artifacts look fresh, and cargo
compiles against them.

The pipeline has been reporting this as a source defect. On 2026-11-15,
PR #155's `clippy-test` job failed with:

```
error[E0599]: no method named `id` found for struct `engine::Session`
  --> crates/app/src/shell/html_preview.rs:148:40
```

on a tree whose `crates/engine/src/session.rs` does define
`pub fn id(&self) -> u64` (`git show 4fc7f425:crates/engine/src/session.rs`).
The job's log shows `Checking app` and no `Checking engine`: cargo
never recompiled the engine, so the app was linked against another
branch's engine rlib. Three more branches failed in the same shape
(E0599 `id`, E0063 missing field `stack_mode`, E0609 no field
`affordances`). Four red branches, no red code.

## Decision

1. **Every job that compiles the workspace refreshes the workspace
   sources first.** `scripts/ci-fresh-workspace-crates.sh` touches the
   crates' sources and manifests, so their mtimes are newer than every
   artifact in the cache, and cargo rebuilds the workspace crates from
   this run's tree. It is the first step of the `layers`,
   `clippy-test`, `wasm-web`, `corpus-golden`, `perf-budgets`, `fuzz`,
   `generated-docs` and `release` jobs.
2. **The refresh is a timestamp, not a rebuild.** Third-party artifacts
   are untouched: they are versioned by the registry and cannot be
   stale. The workspace crates are rebuilt by cargo from the sources the
   run checked out — the guard only makes sure cargo looks at them.
3. **The guard runs where the cache is shared.** It is a GitForge
   pipeline step, because the shared cache is a GitForge property. The
   GitHub backup workflow checks out a fresh tree every run (its sources
   are always newer than its restored cache) and keeps its cache under a
   lockfile-derived key, so it does not need the guard. The local
   `xtask ci` gate uses a per-worktree target directory and does not
   need it either.

## Consequences

- The guard costs one `find`/`touch` pass over the crates' sources per
  job — seconds, against jobs that run for tens of minutes. It does not
  make a job slower in any way the job's duration can show.
- A run whose checkout is newer than the last build's checkout already
  rebuilt the workspace crates (cargo's own mtime rule). The guard
  closes the remaining window, the one where the checkout is *older*.
- **Debt.** The cache is still shared across branches, so every job that
  compiles the workspace must remember to run the guard: a new Cargo job
  that forgets it inherits the defect. The remedy that removes the need
  for the guard is a cache directory per branch, which needs a GitForge
  change — the cache source template supports only `{image}`,
  `{cache_root}` and `{docker_socket}`, with no ref- or
  commit-shaped placeholder. Until GitForge grows one, the guard is the
  tool-side fix and this decision records the debt rather than leaving
  it implicit. Owner: the GitForge repository
  (`/nas/Temp/repos/GitForge`), the pipeline's cache template.
- The GitHub backup workflow remains the weaker of the two receipts for
  the *same* reason: its cache key is
  `cargo-build-${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}`, so
  it is shared across branches as well. Its checkouts are always fresh,
  so the defect cannot bite there today; a workflow that restored a
  branch's own cache would need the same guard.
