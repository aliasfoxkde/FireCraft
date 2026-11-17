# T27-03 — Audit PR #9 and PR #8 and write a disposition plan

> **RP:** RP-00 · **Priority:** P0 · **Status:** BUILD VERIFIED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** none

**Objective:** read every review body and inline comment on the two open release PRs,
classify each finding, and write the disposition plan that the downstream fix rows
execute.

**Inputs:** `gh pr view 9 --json reviews,comments,headRefOid`;
`gh pr view 8 --json reviews,headRefOid`;
`gh api repos/aliasfoxkde/FireCraft/pulls/8/comments`; `.github/workflows/release.yml`.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §9 (agent protocol — every PR must
carry receipts and unverified gaps); the campaign's rule that a review finding is
verified against current code before it is fixed.

**Acceptance criteria:** each PR has a written disposition; every open review finding
is enumerated with file, line and severity, and each is marked for fix or for
rejection-with-reason.

**Verification:** `gh pr view 9 --json reviews,comments && gh pr view 8 --json reviews`

**Evidence:** PR disposition plan: [`docs/release/PHASE-0-AUDIT.md`](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/release/PHASE-0-AUDIT.md) §7. PR #8 (`feat/dock-inspector-parity-matrix`) was closed unmerged; its shell-lineage work landed through the train (PR #107, closed issue #86). PR #9 (`docs/cross-platform-release-plan`) was closed unmerged; its documents are on `main` (`docs/RELEASE-ENGINEERING-PLAN.md`, `docs/release/ARTIFACT-MATRIX.md`, `docs/research/android-feasibility.md`, decision 0070). **Done:** 2026-11-09: audit §7 merged (PR #107).

**Blockers:** —

**Stopping condition:** the finding census and its disposition plan exist in the board,
and every finding maps to exactly one downstream row (T27-12/13 for Aegis, T27-17 for
the release workflow, T27-23/24/25 for the PR #8 application findings).
