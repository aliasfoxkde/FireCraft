# T27-02 — Audit release tags and every published release asset

> **RP:** RP-00 · **Priority:** P0 · **Status:** IN PROGRESS
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-01

**Objective:** enumerate every tag and every published asset, and record the gap
between what the tag workflow now produces and what was actually published.

**Inputs:** `gh release list`; `gh release view v0.2.5 --json assets`;
`git tag --format='%(refname:short) %(objectname)'`; `.github/workflows/release.yml`.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §4 (metadata) and §5.1 (job graph);
`docs/release/ARTIFACT-MATRIX.md`.

**Acceptance criteria:** every published asset is listed with size and URL, every tag
is mapped to its commit, and the published-asset count is compared against the
aggregate the tag workflow expects.

**Verification:** `gh release list && gh release view v0.2.5 --json assets`

**Evidence:** `v0.2.5` → `3a4832b`; 5 assets published — desktop tar.gz (22,084,726 B),
CLI tar.gz (8,072,377 B), web zip (7,174,826 B), DEB (14,819,904 B), `MANIFEST.sha256`
(398 B). `release.yml` now expects 8 assets: the v0.2.5 release predates the source
archive, the SBOM and `RELEASE_SHA256.txt`.

**Blockers:** none for the audit itself; the missing artifacts are tracked as
T27-52, T27-55 and T27-53.

**Stopping condition:** the audit table exists in `docs/release/EXECUTION-BOARD.md` §2
and the published-vs-expected difference is stated explicitly.
