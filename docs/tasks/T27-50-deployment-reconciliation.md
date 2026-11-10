# T27-50 — Reconcile the production Pages deployment against the release bundle

> **RP:** RP-05 · **Priority:** P0 · **Status:** NOT STARTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-48

**Objective:** make the live web deployment equal a specific release bundle, and record
the comparison as a machine-readable report.

**Inputs:** the tagged bundle and its manifest from T27-48;
`https://firecraft-web.pages.dev/MANIFEST.sha256`; the Cloudflare Pages project and
deployment id; `CLOUDFLARE_API_TOKEN`; `wrangler pages deploy`.

**Authority:** `docs/RELEASE-ENGINEERING-PLAN.md` §10 — public web deployment hashes
must match the exact release bundle.

**Acceptance criteria:** the Pages project, deployment id, deployed asset inventory and
per-file hashes are compared to the tagged bundle, and the result is written as a
machine-readable report that states "match" or the exact per-file differences.

**Verification:** `curl -s https://firecraft-web.pages.dev/MANIFEST.sha256`

**Evidence:** the live deployment serves bundle `2274ac8c`; the tree at the time of the
artifact matrix was `e44296eb` — the deployed bytes are stale and are not a release
bundle. The deploy step in `.github/workflows/release.yml` is commented out.

**Blockers:** no `CLOUDFLARE_API_TOKEN`; the deploy step is disabled.

**Stopping condition:** the report shows a byte-level match between the deployed bundle
and a tagged release bundle, or it names every differing file and the deployment is
still stale.
