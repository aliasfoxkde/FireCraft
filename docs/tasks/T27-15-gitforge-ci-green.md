# T27-15 — Get the authoritative GitForge pipeline green on the same SHA

> **RP:** RP-00 · **Priority:** P0 · **Status:** NOT STARTED
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

**Evidence:** none yet — no green GitForge run exists for the candidate.

**Blockers:** T27-13; if `Cargo.lock` moves, the CI image must be re-shipped before a
rerun is meaningful.

**Stopping condition:** a pipeline run for the exact candidate SHA is recorded as
successful in the GitForge store.
