# T27-50 — Reconcile the production Pages deployment against the release bundle

> **RP:** RP-05 · **Priority:** P0 · **Status:** IN PROGRESS
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

**Evidence:** the live deployment was compared with the release bytes by digest and the result recorded in `docs/tasks/T18-release.md` (PR #127, squash-merged 2026-11-12 as `8d5c442`): the live bundle `2274ac8c` (wasm 21,231,377 B, sha256 `f4f0a3f0…`) matches neither the v0.2.5 release asset `f9dbe2f1` (21,238,451 B, `a8a4fa4f…`) nor a tag rebuild `103c62d3` (21,229,720 B, `5934702f…`). The served `index.html` differs from the build's own only by the substituted asset names, their integrity digests and the Pages analytics beacon, so the difference is explained rather than drift — and the live deployment corresponds to no identifiable release (blocker B-6). The machine-readable form of the comparison is still owed. **Done:** 2026-11-12: deployment receipt merged (PR #127).

**Blockers:** no `CLOUDFLARE_API_TOKEN`; the deploy step in `release.yml` is commented out — a deployment that names a commit stays maintainer-gated

**Stopping condition:** the report shows a byte-level match between the deployed bundle
and a tagged release bundle, or it names every differing file and the deployment is
still stale.
