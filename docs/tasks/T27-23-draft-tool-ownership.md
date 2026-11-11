# T27-23 — Track the originating tool in plotted-draft state, and cover it with regression tests

> **RP:** Phase-2 (PR #8 remediation) · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-03

**Objective:** make the in-progress plotted draft record which tool owns it, so points
drawn with one tool cannot be silently folded into a commit made by another.

**Inputs:** `crates/app/src/editor.rs` (the plotted-draft state and the tool-switch
path); the PR #8 review finding on `editor.rs:3271-3276`.

**Authority:** the PR #8 review (CodeRabbit Major, data integrity); the repo's rule
that a review finding is verified against current code before it is fixed.

**Acceptance criteria:** draft state records its owning plotting tool; switching tools
resets the draft or explicitly converts it; regression tests cover Pen → Hotspot and
Hotspot → Pen, and fail against the pre-fix behaviour.

**Verification:** `cargo test -p app`

**Evidence:** CodeRabbit Major on `crates/app/src/editor.rs:3271-3276` — Pen points
folded into a Hotspot commit.

**Blockers:** none.

**Stopping condition:** the tests exist, they fail on the old behaviour and pass on the
new, and the fix is committed with its receipt.
