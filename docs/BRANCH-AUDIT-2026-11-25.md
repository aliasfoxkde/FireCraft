# Branch and worktree audit — 2026-11-25

Read-only snapshot; active work was preserved. Branch counts are point-in-time and include historical remote refs, not just viable merge candidates.

## Current workspaces

| Checkout | Branch / revision | State | Handling |
|---|---|---|---|
| `/nas/Temp/repos/FireCraft` | `feat/experiment-merge`, `b8cb8c41` | Dirty: `crates/app/src/main.rs`, `crates/app/src/logo.png`; local splash experiment | Active user checkout. Left untouched. |
| `/var/tmp/firecraft-dev-roadmap-improvements` | `feat/dev-roadmap-improvements`, based on `b8cb8c41` | This task's isolated worktree | Active. |
| `/nas/Temp/repos/FireCraft-CraftAnything-Experiment` | `main`, `3a09973` | Dirty: `app/src/app.rs`, `app/src/main.rs`, `ROADMAP.md`, `docs/PROGRESS.md`, `docs/TASKS.md` | Active agent work. Read-only inspection only; do not cherry-pick/revert/clean until handed off. |

The experiment worktree's latest committed changes include a Fireworks-like categorized Tools palette and corrected panel collapse toggles (`7e0a972`), FileService/panel-registry seams, plus earlier shell changes. Its dirty app diff adds Intelligence-panel path tests; its dirty planning docs record interactive and Windows validations still unverified. The `app/shots/final-1360.png` capture shows a Fireworks-style shell and bottom Properties/Intelligence tabs, but is a single static capture and does not verify runtime interactions.

## Pull requests and checks

| PR | Branch | State at audit | Evidence / action |
|---|---|---|---|
| [#187](https://github.com/aliasfoxkde/FireCraft/pull/187) | `feat/experiment-merge` | Open; `main` base; head published as `399385f1` while local branch has progressed to `b8cb8c41` | Windows x86_64 and macOS aarch64 checks fail; two backup `classify-change` checks fail; core gate/fmt/WASM were skipped; CodeQL and CodeRabbit report pass. CodeRabbit flags build/startup and web/non-native open/save/drop concerns plus stale Phase 22 board status. Reconcile current source and fix required checks before merge. |
| [#186](https://github.com/aliasfoxkde/FireCraft/pull/186) | `temp/consolidation` | Open, merge state unstable | Windows and macOS checks fail; backup `classify-change` fails; main gate/fmt/WASM skipped. CodeRabbit review skipped at 121 files. Overlaps PR #187's consolidation and experiment changes; decide whether to close, split or rebase only after comparing unique changes and owners. |

The active `feat/experiment-merge` local checkout has two uncommitted splash/logo changes. Do not treat the PR head as including those changes. Both pull requests had successful CodeQL legs; this alone does not satisfy required CI.

## Branch inventory

At audit: 4 local branches, 128 remote-tracking branches, 2 registered worktrees. The unmerged-local-branch query listed `feat/dev-roadmap-improvements`, `feat/experiment-merge`, and `fix/xtask-gate-repair-2026-11-24`. `main` was one commit ahead of `origin/main`. Remote refs include many historical topic branches; no broad prune is safe based on ancestry alone because some may carry an open PR or active external work.

The clone also contains stashes, including WIP entries for `feat/experiment-merge`, `fix/xtask-gate-repair-2026-11-24`, and `feat/wasm-cdn-deploy`. Preserve these until their contents are mapped to live branches and owners confirm disposal.

## Safe next actions

1. Ask the experiment agent to provide a handoff commit/revision and exact files changed when its current work is ready; compare it against PR #187 and this FireCraft worktree.
2. Inspect PR check logs and fix required check configuration/failures on the owning branch. Do not infer green CI from skipped jobs.
3. Run a three-way unique-change inventory for #186, #187 and experiment `main`; select one integration route and close/split superseded PRs only after their unique work is accounted for.
4. Inventory every remote head against open PRs, last activity and merge-base before pruning; preserve unmerged or owner-active refs.
5. Reconcile `main`'s one-commit divergence from `origin/main` as a separate documented decision.

No branch, stash, PR or worktree was deleted, reset, force-pushed or merged during this audit.
