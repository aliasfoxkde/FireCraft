# 0122 — The Cargo cache is scoped per job, not per commit, and the pipeline refreshes the crates it may have contaminated

Date: 2026-11-16 · Tickets: C4.08 ([P4-export-parity](../climb/P4-export-parity.md)) ·
Status: accepted

## Context

On 2026-11-16 the `generated-docs` job of GitForge run `29199c4e`
(branch `climb/C4.08-selective-jpeg`) failed with

```
error[E0425]: cannot find function `menu_census` in module `app::menus`
  --> xtask/src/climb.rs:140:39
```

`crates/app/src/menus.rs` **in that same tree** defines `menu_census`. A
compile error for a function that is present can only mean the compiler
was fed a different tree's crate.

The mechanism is the cache's scope. `.gitforge.yml` gives each job its
own cache directory, `{cache_root}/cargo/{image}-<job-name>`, so the
warm target directory is shared by **every branch that runs that job**.
Cargo decides that a cached artifact is fresh by comparing the source
files' mtimes with the mtime it recorded in that artifact's fingerprint:
a source whose mtime is *older* than the recorded one counts as fresh.
So the checkout's age decides the verdict.

The timeline on that run:

| When | What |
|---|---|
| 04:43 | the C4.08 workspace is checked out |
| 06:07 | the icon branch — cut from `main`, which has neither `xtask/src/climb.rs` nor `menu_census` — rebuilds the shared `generated-docs` cache |
| 06:13 | the C4.08 `generated-docs` job starts and mounts that cache |

The C4.08 checkout was older than the cached build, so cargo served the
icon branch's `app` artifact and xtask was compiled against a crate that
has no `menu_census`. The cache still carries the evidence: the
`clippy-test` cache holds an `app` rmeta with `hits=0` dated
2026-11-16 08:17:59 next to rmetas with `hits=1` — the same crate, two
builds, one of them from a different tree.

Nothing in this repository can scope the cache by commit. The template
is expanded in the GitForge platform repo
(`crates/gitforge-common/src/volume.rs::expanded_source`), and it
supports exactly three placeholders — `{image}`, `{cache_root}` and
`{docker_socket}`. There is no commit or run placeholder, so a pipeline
cannot ask for a cache that belongs to its own tree.

## Decision

- **Every compiling job starts with a refresh of the crates the cache
  may hold from another tree.** The step runs
  `scripts/ci_refresh_stale_crates.sh` before it compiles anything:
  `layers`, `clippy-test`, `wasm-web`, `corpus-golden`, `perf-budgets`,
  `fuzz`, `generated-docs` and `release`. `fmt-layers` compiles
  nothing and `detect-changes` / `release-ledger` do not build, so they
  are left alone.
- **The rule is narrow on purpose.** For each workspace crate the
  script compares the crate's sources with the newest cached
  fingerprint of that crate. If a source is already newer, cargo will
  rebuild the crate by itself and the script leaves the crate alone; if
  no source is newer, the cache may hold another tree's artifact, so
  the crate's sources are touched and cargo rebuilds it. The warm
  cache is kept for every crate that is not at risk, so the step costs
  nothing when nothing is wrong.
- **The step's own behavior is gated, not trusted.**
  `scripts/ci_refresh_stale_crates-selftest.sh` lays the mtimes out by
  hand against a synthetic workspace and cache and asserts both
  directions — a crate newer than the cache is left alone, a crate older
  than the cache is refreshed, a crate under the wasm target directory is
  refreshed, a crate with no cached fingerprint is left alone, and a
  missing workspace or cache exits 0. It runs in the `release-ledger`
  job, on every push, because that job is the one gate this image can
  run without a compiler.
- **The step is skipped when the script is absent.** Branches that
  predate it still run. The script's own failure fails the job, so the
  mitigation cannot silently stop working.
- **The platform defect stays open, with an owner and a named fix.**
  Owner: the GitForge platform lane (the GitForge repository). Fix:
  a commit-scoped cache — a `{commit}` (or `{run}`) placeholder in
  `expanded_source`, so a job can request
  `{cache_root}/cargo/{image}-<job>-{commit}`, or the platform
  invalidating a job's cache when the mounted checkout is newer than
  the cached build. When that lands, this step and this script are
  deleted in the same change, and this record is marked superseded.
  Until then the step is a documented compensation for a defect in
  another system, not a fix of it.

## Consequences

- A branch's CI verdict no longer depends on which other branch ran
  before it. The C4.08 branch is re-run on top of this step; its
  `generated-docs` job is the receipt.
- The cost is bounded and paid only when the cache is actually
  contended: in the contended case the local crates are rebuilt, which
  is the price of a verdict that means something.
- The mitigation is repository-side, self-tested, and does not depend on
  the runner's timing — the class of failure is closed by the pipeline
  itself rather than by an operator noticing a red.
- The platform defect is named, located and owned instead of being
  papered over: the step's header and this record both point at
  `crates/gitforge-common/src/volume.rs::expanded_source` as the place
  the defect lives.
