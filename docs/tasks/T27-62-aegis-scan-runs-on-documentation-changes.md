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

**Evidence:** The defect and its fix are in [decision 0132](../decisions/0132-the-secret-scan-runs-on-documentation-changes.md). The scan covers `docs/`, but both CI systems ran it only for code changes: the GitHub `gate` job is gated on `code_changed` and its `docs-only-check` job validated whitespace only, and the GitForge pipeline had no Aegis job at all while every Cargo job carries a `skip_if` on `CI_CHANGE_INCLUDE_CODE`. PR #170's documentation-only merge therefore entered `main` with four unblessed Windows build literals, and the next code change's gate failed for content it never touched: on `main` `c26adffc` the `gate` job's Aegis leg reported 5 findings the baseline does not cover. The same defect is visible from the other CI system: PR #154 (`8067b53b`, the icon-resource survey) touches no file under `docs/tasks/`, and its GitForge run `fe161636` **succeeded** because the pipeline has no Aegis job to answer, while the GitHub `gate` job on the same SHA **failed** with the same 5 findings, every one of them in `docs/tasks/T27-41-windows-arm64-assessment.md`. The fix has three parts. `scripts/install-aegis.sh` is the single installer — one release URL, one SHA-256, one identity check (`aegis --help` must list the `scan` subcommand, because an unrelated tool also ships under the bare name `aegis`) — and it brings its own fetch, because the authoritative CI image ships no `curl` and no `wget`; the GitHub `gate` job's inline installer now calls it instead of carrying its own copy of the pin. The GitHub `docs-only-check` job installs Rust and the scanner and runs `cargo run --package xtask -- aegis`. The GitForge pipeline gains an `aegis` job with **no `skip_if`**, chained between `release-ledger` and `fmt-layers`, with its own cache directory. The pipeline definition validates (`scripts/validate_gitforge_pipeline.py`: 12 jobs, one root, no join) and the workflow's action pins validate. The fix is merged as PR #172 → `7bcf28fd`, and the scan has since run green on a documentation-only change in **both** systems: the GitForge `aegis` job has no `skip_if` and the GitHub `docs-only-check` job installs the scanner and runs it. **Done:** 2026-11-17.

**Blockers:** —

**Stopping condition:** the scan runs on a documentation-only change in both CI
systems, and the job logs show it.

## What this row does not claim

The scan reports findings; it does not adjudicate them. Admitting a new literal to
`.aegis-baseline.tsv` remains a human step — the baseline is written only after
someone has checked that the finding is a false positive. This row makes the
judgement get asked for at the moment the content lands; it does not automate the
judgement, and it does not claim that any finding in the tree is adjudicated.
