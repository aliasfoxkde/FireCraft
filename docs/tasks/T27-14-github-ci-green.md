# T27-14 — Get the GitHub backup CI green on PR #9

> **RP:** RP-00 · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-13

**Objective:** make the GitHub backup CI pipeline (`CI (backup)`) conclude SUCCESS on
the release PR head.

**Inputs:** `.github/workflows/ci.yml` (the `fmt` job and the `gate` job); the PR head
SHA; the Aegis 0.6.3 pin.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §5 and §9 — a PR must carry CI
receipts; GitForge is primary and GitHub is the backup, and both must be green for the
same SHA.

**Acceptance criteria:** the `CI (backup)` `gate` job concludes SUCCESS on the PR head
and the `fmt` job remains SUCCESS.

**Verification:** `gh pr checks 9`

**Evidence:** currently FAILURE on runs `42401632606` and `42401626535`, both on the
Aegis gate.

**Blockers:** T27-13.

**Stopping condition:** `gh pr checks 9` reports every required check successful on the
candidate SHA — read from the check run, not from a watcher.
