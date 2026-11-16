# T27-17 — Replace the invalid `gh release create` asset arguments

> **RP:** RP-07 · **Priority:** P0 · **Status:** BUILD VERIFIED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-03

**Objective:** fix the release workflow's asset upload so the draft release can actually
receive its artifacts, and prove the fixed form handles the awkward filenames.

**Inputs:** `.github/workflows/release.yml` (the "Create draft release" step);
`gh release create --help` (gh 2.46.0).

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §5.1 and §10 (exit criteria — the
release aggregation must fail closed and must publish); the campaign's prohibition on
publishing without maintainer approval (a disposable test tag is permitted).

**Acceptance criteria:** assets are passed as positional filenames; the invocation is
tested for a missing file, a duplicate name and a name containing spaces; a missing
artifact still fails the step closed.

**Verification:** `gh release create --help`

**Evidence:** [`release.yml`](https://github.com/aliasfoxkde/FireCraft/blob/main/.github/workflows/release.yml) used `assets+=(-a "$f")`; `gh release create` has no `-a` flag in gh 2.46 — assets are positional `[<files>...]`. **Fixed and exercised**: the rehearsal's `create draft release` job (run 42937560976) succeeded and the draft carries 8 uploaded assets, so the positional invocation works against the real CLI. **Done:** 2026-11-11: rehearsal `v0.2.5-dryrun.3`, run 42937560976.

**Blockers:** —

**Stopping condition:** the workflow uses the positional form and the step is exercised
without publishing a production release.
