# T27-03 — Audit PR #9 and PR #8 and write a disposition plan

> **RP:** RP-00 · **Priority:** P0 · **Status:** IN PROGRESS
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

**Evidence:** PR #9 head `eb26761`, `gate` job FAILURE (Aegis) on runs `42401632606`
and `42401626535`. PR #8 head `941e5b6`; 7 review bodies and 29 inline comments, last
round 2026-11-07T18:34Z. Full census in `docs/release/EXECUTION-BOARD.md` §6.

**Blockers:** PR #9's CI is red — see T27-14 and T27-15.

**Stopping condition:** the finding census and its disposition plan exist in the board,
and every finding maps to exactly one downstream row (T27-12/13 for Aegis, T27-17 for
the release workflow, T27-23/24/25 for the PR #8 application findings).
