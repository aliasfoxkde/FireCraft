# T27-57 — Harden the release workflows

> **RP:** RP-06 · **Priority:** P1 · **Status:** NOT STARTED
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

**Verification:** `grep -n 'uses:' .github/workflows/*.yml`

**Evidence:** `ci.yml` and `release.yml` still reference mutable tags (`@v4`,
`@stable`); only the Aegis scanner is SHA-pinned.

**Blockers:** T27-18.

**Stopping condition:** every `uses:` is a full commit SHA, the permission set is
least-privilege, and the failure-mode checks are demonstrated with one deliberately
broken input each.
