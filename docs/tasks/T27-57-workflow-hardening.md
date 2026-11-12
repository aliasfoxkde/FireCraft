# T27-57 — Harden the release workflows

> **RP:** RP-06 · **Priority:** P1 · **Status:** IN PROGRESS
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-18

**Objective:** harden the CI and release workflows so a compromised or drifted
dependency cannot silently alter a release.

**Inputs:** `.github/workflows/ci.yml`; `.github/workflows/release.yml`; the Aegis
version pin (already by SHA); the GitForge pipeline.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §5 and §10; the campaign's requirement
that all artifacts bind to one tag SHA and that the gate fails closed.

**Acceptance criteria:** third-party actions are pinned to full commit SHAs;
permissions are least-privilege; build, sign and publish are separate jobs; all
artifacts bind to one tag SHA; the gate fails on missing outputs, a version mismatch,
a checksum mismatch, unexpected extra artifacts and a skipped required platform; a
safe test-release mode exists.

**Verification:** `python3 scripts/validate_workflow_pins.py .github/workflows` and
`bash scripts/validate_workflow_pins-selftest.sh`

**Evidence (2026-11-11):** decision
[0099](../decisions/0099-workflow-actions-are-pinned.md) — all 19 action references
in `ci.yml`, `release.yml` and `deploy-site.yml` name a 40-hex commit SHA (the
version is kept in a trailing comment). `scripts/validate_workflow_pins.py` fails
the workflow on an unpinned reference and runs in the backup CI's `changes` job —
the job every push and pull request runs, and the job `fmt` and `gate` depend on.
`scripts/validate_workflow_pins-selftest.sh` demonstrates each failure mode with one
deliberately broken input (a mutable tag, a mutable branch, a `docker://` image, a
short SHA) and the two accepted inputs (a pinned SHA, a local action). The deploy
trigger no longer lists the workflow's own file, so a workflow edit is not a
production publication.

**Already satisfied before this change:** least-privilege permissions (`contents: read`
at workflow level, `contents: write` only for the draft-release job); build and
publish are separate jobs; the artifacts bind to the tag SHA through the checkout;
the aggregate gate fails closed on a missing, empty, misnamed or digest-mismatched
artifact and on a version mismatch.

**Outstanding:** the "skipped required platform" check is not in the workflow, and a
safe test-release mode is not yet a documented workflow state. The pin check is in
the GitHub backup CI; mirroring it into the GitForge pipeline waits for the `:2`
pipeline shape (PR #123, PR #129).

**Blockers:** none for the pinning; the remaining two criteria are unstarted work.

**Stopping condition:** every `uses:` is a full commit SHA, the permission set is
least-privilege, and the failure-mode checks are demonstrated with one deliberately
broken input each.
