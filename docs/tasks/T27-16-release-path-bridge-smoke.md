# T27-16 — Make the release path run the web bridge smoke with no skip

> **RP:** RP-00 · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-05

**Objective:** ensure the release lane exercises the headless web bridge smoke rather
than skipping it, so the published bundle is known to run in a browser.

**Inputs:** `.gitforge.yml` (the release job); `.github/workflows/release.yml`;
`xtask/src/dist.rs` (`smoke_bridge`, `strip_script_bodies`); the `FIRECRAFT_DIST_SKIP_SMOKE`
environment variable.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §5.1; the repo's rule that a verdict
string appearing in the page's own `<script>` source is not evidence — the DOM
assertions must be made against the stripped page.

**Acceptance criteria:** the release path executes the bridge smoke; a skip exists only
for non-release CI and is documented as such; the smoke's verdict is read from the
rendered DOM, not from the page source.

**Verification:** `grep -n FIRECRAFT_DIST_SKIP_SMOKE .gitforge.yml .github/workflows/*.yml`

**Evidence:** `.gitforge.yml:168` sets `FIRECRAFT_DIST_SKIP_SMOKE=1` for the release
job; the artifact matrix does not record this skip.

**Blockers:** chromium is missing from the CI image.

**Stopping condition:** the release job's smoke runs without the skip, or the skip is
removed from the release lane and the reason is recorded.
