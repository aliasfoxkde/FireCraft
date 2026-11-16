# T27-16 — Make the release path run the web bridge smoke with no skip

> **RP:** RP-00 · **Priority:** P0 · **Status:** BUILD VERIFIED
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

**Evidence:** The row's premise was wrong and is corrected here. `FIRECRAFT_DIST_SKIP_SMOKE` appears nowhere in `.gitforge.yml` or the GitHub workflows on `main` (it survives only in `xtask/src/dist.rs:52` as a comment, in the docs, and in issue #110), and `infrastructure/docker/ci-rust.Dockerfile:127` installs bookworm `chromium` for exactly this smoke. The smoke runs in CI: the GitForge `release` job logs `[xtask] dist web: 5 file(s), wasm 15573974 B (warning 25000000, hard ceiling 50000000), manifest MANIFEST.sha256, smoke green` and `[xtask] dist --web ok` — most recently job `ecb488d1-8965-443f-8309-71405883d754` for commit `fcfe6db1`, finished 2026-11-12T06:18:54Z, and 12 release jobs carry that line. **Done:** 2026-11-12: smoke green in the GitForge release job.

**Blockers:** —

**Stopping condition:** the release job's smoke runs without the skip, or the skip is
removed from the release lane and the reason is recorded.
