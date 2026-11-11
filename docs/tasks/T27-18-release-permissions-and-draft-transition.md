# T27-18 — Scope release permissions and verify the draft-to-published transition

> **RP:** RP-07 · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-17

**Objective:** prove that the publishing job holds exactly the permission it needs, and
that a draft release can be created, populated, digest-checked and then flipped to
published.

**Inputs:** `.github/workflows/release.yml` (the `build` job's `permissions: contents:
read` and the `release` job's `contents: write`); the aggregate validation step;
`RELEASE_SHA256.txt`.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §5.1 and §4; the campaign §20
restriction that publishing requires maintainer approval — the draft transition must
therefore be exercised on a disposable tag only.

**Acceptance criteria:** `contents: write` exists only on the publishing job; a draft
is created; assets are uploaded; post-upload digests are re-validated against
`RELEASE_SHA256.txt`; the draft transitions to published only after that check.

**Verification:** `grep -n 'permissions:' -A3 .github/workflows/release.yml`

**Evidence:** `release.yml` currently sets `contents: read` on the build job and
`contents: write` on the release job — apparently compliant, but untested, and the
upload step is broken by T27-17.

**Blockers:** T27-17.

**Stopping condition:** the permission split and the draft-to-published sequence are
both demonstrated on a disposable tag, with digests re-verified after upload.
