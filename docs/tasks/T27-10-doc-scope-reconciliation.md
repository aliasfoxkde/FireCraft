# T27-10 — Reconcile the release documentation against one scope authority

> **RP:** RP-00 · **Priority:** P1 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-03, T27-09

**Objective:** make T27, `docs/TASKS.md`, `docs/RELEASING.md`, `docs/TOOLCHAINS.md` and
the artifact matrix cite the same scope authority and contain no stale CI or status
claims.

**Inputs:** `docs/tasks/T27-cross-platform-release.md`; `docs/TASKS.md`;
`docs/RELEASING.md`; `docs/TOOLCHAINS.md`; `docs/release/ARTIFACT-MATRIX.md`;
`docs/decisions/`.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §1.1 and §3 (the target matrix and
the MVP candidate currently disagree — §3 lists Windows/macOS/Android as required
while §1.1 proposes a reduced first cut).

**Acceptance criteria:** every release document cites one scope authority; no document
claims a CI state, artifact state or platform support that the matrix does not
record; the §3-versus-§1.1 conflict is resolved in favour of the maintainer's
decision.

**Verification:** `grep -rn '0\.2\.5' docs/RELEASING.md docs/TOOLCHAINS.md docs/tasks/T27-cross-platform-release.md`

**Evidence:** `docs/release/ARTIFACT-MATRIX.md` is reconciled in this change; the
remaining scope conflict is unresolved pending T27-09.

**Blockers:** T27-09 (scope decision); `docs/TASKS.md` is owned by another agent in
this session and is not edited here.

**Stopping condition:** the documents agree, or the disagreement is recorded as an
explicit blocker with an owner.
