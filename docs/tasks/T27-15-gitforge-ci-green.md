# T27-15 — Get the authoritative GitForge pipeline green on the same SHA

> **RP:** RP-00 · **Priority:** P0 · **Status:** IN PROGRESS
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-13

**Objective:** make the authoritative GitForge pipeline — the 7-job chain
fmt-layers → clippy-test → wasm-web → corpus-golden → perf-budgets → fuzz →
generated-docs → release — conclude success on the same commit as T27-14.

**Inputs:** `.gitforge.yml`; the pushed SHA; the GitForge API status call for that run.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §5.1; the repo rule that the
authoritative CI chain is GitForge's.

**Acceptance criteria:** the `firecraft-ci` pipeline concludes success on the same
commit SHA that T27-14 verified, and every job in the chain reports success.

**Verification:** the GitForge API status call for the run id of the pushed SHA
(`gh` does not address GitForge; read the run row directly).

**Evidence:** GitForge runs exist for all six candidate heads, but the fleet has one online runner (`swarmone-docker`, capacity 4) and the candidate queue exceeds it, so the runs are still in flight. The fleet measurement and the two GitForge-side defects — a superseded run's running job keeps its runner slot, and the CLI has no cancel verb — are recorded in issue #126. No green GitForge run is claimed until the authoritative store reports `succeeded`.

**Blockers:** the fleet's single online runner; T27-13 is closed

**Stopping condition:** a pipeline run for the exact candidate SHA is recorded as
successful in the GitForge store.
