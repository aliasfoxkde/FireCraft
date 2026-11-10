# T27-17 — Replace the invalid `gh release create` asset arguments

> **RP:** RP-07 · **Priority:** P0 · **Status:** READY
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

**Evidence:** `.github/workflows/release.yml:207` uses `assets+=(-a "$f")`. `gh release
create` has no `-a` flag in gh 2.46.0 — its usage is `gh release create [<tag>]
[<files>...]`. The upload step therefore cannot succeed as written, and the
artifact matrix's claim that the tag workflow performs a "draft-release upload" is
not true today.

**Blockers:** none — this row can start now.

**Stopping condition:** the workflow uses the positional form and the step is exercised
without publishing a production release.
