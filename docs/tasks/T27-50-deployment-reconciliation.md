# T27-50 — Reconcile the production Pages deployment against the release bundle

> **RP:** RP-05 · **Priority:** P0 · **Status:** IMPLEMENTED
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

**Evidence:** the comparison is a tracked tool and a machine-readable report. [`scripts/reconcile_deployment.py`](https://github.com/aliasfoxkde/FireCraft/blob/main/scripts/reconcile_deployment.py) fetches the deployment's `MANIFEST.sha256` and every file it names, hashes the bytes it actually received, and compares them file by file with the bundle's manifest; [`scripts/reconcile_deployment-selftest.py`](https://github.com/aliasfoxkde/FireCraft/blob/main/scripts/reconcile_deployment-selftest.py) covers the four ways the two can disagree — identical, changed bytes, a deployment-only file, and a bundle file the deployment does not serve. The report against the live deployment is [`docs/release/DEPLOYMENT-RECONCILIATION.json`](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/release/DEPLOYMENT-RECONCILIATION.json) and its result is `differences`: the live deployment serves bundle `2274ac8c2925b61d` while the v0.2.5 release asset carries `f9dbe2f14305b2d9`, each asset name appearing only on its own side of the inventory; `appinfo.json` matches, and `index.html` and `host.html` differ only by the substituted asset names, their integrity digests and the Pages analytics beacon. This confirms the earlier hand comparison (PR #127, `8d5c442`) in machine-readable form: the live deployment corresponds to no identifiable release (blockers B-6, B-11, B-12). **Done:** 2026-11-16: the comparison landed as a tracked tool and a machine-readable report.

**Blockers:** no `CLOUDFLARE_API_TOKEN`; the deploy step in `release.yml` is commented out — a deployment that names a commit stays maintainer-gated

**Stopping condition:** the report shows a byte-level match between the deployed bundle
and a tagged release bundle, or it names every differing file and the deployment is
still stale.
