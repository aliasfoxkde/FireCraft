# T27-10 — Reconcile the release documentation against one scope authority

> **RP:** RP-00 · **Priority:** P1 · **Status:** IMPLEMENTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-03, T27-09

**Objective:** make T27, `docs/TASKS.md`, `docs/RELEASING.md`, `docs/TOOLCHAINS.md` and
the artifact matrix cite the same scope authority and contain no stale CI or status
claims.

**Inputs:** `docs/tasks/T27-cross-platform-release.md`; `docs/TASKS.md`;
`docs/RELEASING.md`; `docs/TOOLCHAINS.md`; `docs/release/ARTIFACT-MATRIX.md`;
`docs/decisions/`.

**Authority:** `docs/decisions/0075-release-scope-mvp.md` — the release-scope
authority, accepted on 2026-11-17. It resolves the conflict this row was opened
against: `docs/RELEASE-ENGINEERING-PLAN.md` §3 listed Windows/macOS/Android as
required while §1.1 proposed a reduced first cut. The approved scope is the MVP
candidate; §3 remains the standing **campaign** matrix, and the extended targets
stay outside the release cut until each carries its own acceptance evidence.

**Acceptance criteria:** every release document cites one scope authority; no document
claims a CI state, artifact state or platform support that the matrix does not
record; the §3-versus-§1.1 conflict is resolved in favour of the maintainer's
decision.

**Verification:** `grep -rn '0\.2\.5' docs/RELEASING.md docs/TOOLCHAINS.md docs/tasks/T27-cross-platform-release.md`

**Evidence:** All five release documents — `docs/release/ARTIFACT-MATRIX.md`, `docs/RELEASING.md`, `docs/TOOLCHAINS.md`, `docs/tasks/T27-cross-platform-release.md` and `docs/TASKS.md` — cite [decision 0075](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/decisions/0075-release-scope-mvp.md) as the release-scope authority and none of them restates the scope as a second source of truth. The approval wording is reconciled across those five, the plan (`docs/RELEASE-ENGINEERING-PLAN.md` §1.1) and the decision itself: the "recommended — awaiting maintainer approval" text is gone from every live document (the only remaining occurrence is `docs/climb/AUDIT-2026-11-13.md`, a dated audit record that states what was true on 2026-11-13). The stale-claim check passed: the `0.2.5` matches in the three probed files are the historical release and the rehearsal tag (`v0.2.6-dryrun.1`), not claims about a current version, and no release document claims the Aegis scan runs only for code changes — both CI systems scan every change now (row T27-62) **Done:** 2026-11-17.

**Blockers:** —

**Stopping condition:** the documents agree, or the disagreement is recorded as an
explicit blocker with an owner.
