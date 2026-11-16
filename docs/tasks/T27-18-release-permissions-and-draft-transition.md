# T27-18 — Scope release permissions and verify the draft-to-published transition

> **RP:** RP-07 · **Priority:** P0 · **Status:** IN PROGRESS
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

**Evidence:** `release.yml` build job `contents: read`, release job `contents: write` — compliant. **Half exercised**: the rehearsal created a draft with 8 uploaded assets and validated the digests, and the draft is still `draft: true` / `published_at: null`. The draft-to-published transition is untested because publishing is a maintainer-only act (campaign §20). **Done:** 2026-11-11: draft creation and digest validation verified (run 42937560976); the published transition awaits authorization.

**Blockers:** maintainer approval for a published release

**Stopping condition:** the permission split and the draft-to-published sequence are
both demonstrated on a disposable tag, with digests re-verified after upload.
