# T27-62 — Run the Aegis secret scan on documentation-only changes, in both CI systems

> **RP:** RP-00 (the release contract's own gates) · **Priority:** P1 · **Status:** IMPLEMENTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-24

**Objective:** make the Aegis secret/PII scan run on every change — including a
change that only touches documentation — so the committed baseline cannot go stale
unnoticed.

**Inputs:** `xtask/src/aegis.rs`; `.aegis-baseline.tsv`; `.gitforge.yml`;
`.github/workflows/ci.yml`; `scripts/install-aegis.sh`;
[decision 0132](../decisions/0132-the-secret-scan-runs-on-documentation-changes.md).

**Authority:** decision 0132 (the scan runs on documentation-only changes, in both
CI systems); decision 0074 (the baseline is content-keyed and human-blessed).

**Acceptance criteria:** a documentation-only change is scanned by the Aegis leg in
both CI systems; the scanner is installed from one script with one pinned checksum
in both systems; no CI system skips the scan for a documentation-only diff.

**Verification:** the GitForge pipeline definition passes
`scripts/validate_gitforge_pipeline.py`; the GitHub workflow passes
`scripts/validate_workflow_pins.py`; a documentation-only change on the branch
produces a scan in both systems' job logs.

**Evidence:** [decision 0132](../decisions/0132-the-secret-scan-runs-on-documentation-changes.md) **Done:** 2026-11-24.

**Blockers:** —

**Stopping condition:** the scan runs on a documentation-only change in both CI
systems, and the job logs show it.

## What this row does not claim

The scan reports findings; it does not adjudicate them. Admitting a new literal to
`.aegis-baseline.tsv` remains a human step — the baseline is written only after
someone has checked that the finding is a false positive. This row makes the
judgement get asked for at the moment the content lands; it does not automate the
judgement, and it does not claim that any finding in the tree is adjudicated.
