# 0084 — The release bytes are the source of truth for the web deployment

Date: 2026-11-10 · Ticket: [T18](../tasks/T18-release.md) · Status: accepted

## Context

The live site `https://firecraft-web.pages.dev` serves a web bundle that
matches no published release and no build this repository produces. The
Phase 0 audit recorded this as blocker **B-6** and left it open: the
deployed bundle `2274ac8c2925b61d` is unattributed to any commit, and
the deployment's own `MANIFEST.sha256` cannot verify the deployment,
because Cloudflare Pages injects its analytics beacon into the two HTML
files after the manifest is written.

Re-verified 2026-11-10 against the live site:

- the deployed `.js` (`3b0403fc…`) and `_bg.wasm` (`f4f0a3f0…`,
  21,231,377 B) differ from the published v0.2.5 bundle
  (`be288850…`, `a8a4fa4f…`, 21,238,451 B) — **different bytes**;
<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:76f45b6fbea47af691cab441a492d6a283099340bb4def54e30f4c31376732c3 -->

- the deployed `host.html` and `appinfo.json` are byte-identical to the
  released ones, so the two bundles share those WebOS shell files;
  their source relationship is unknown without a build receipt;

<!-- /coderabbit-ai-generated:v1 -->

- `sha256sum -c` against the live manifest passes for the three
  non-HTML entries and **fails for `host.html` and `index.html`**, by
  construction.

The audit named the condition but not the resolution: the two directions
— redeploy the site from the release bytes, or cut the next release from
the deployed bytes — were both still open, and each leads to a different
release process.

## Decision

**The release is the source of truth. The deployment is redeployed from
the release bytes.**

The reverse is rejected. Cutting a release from the deployed bytes would
publish an artifact that (a) has no build receipt and no commit SHA
behind it, so it is unattributable to a source revision, and (b) cannot
satisfy its own checksum list, because two of the five files that list
covers are rewritten by the platform after the manifest is written.

Verification of a deployment therefore happens against the **pre-deploy**
`dist/web/MANIFEST.sha256`, never against the live site. The live
manifest's two HTML entries are expected to differ and are not evidence
of corruption.

## Consequences

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:6278fcea3e528def9d2b8f99a230703ebf06e9a84436244e533965d2dd0cd961 -->

- The next web release must be redeployed from the release ZIP's bytes.
  The redeploy receipt records the deployed bundle stem and the source
  SHA when established by provenance. If the published ZIP lacks
  provenance, it records the source SHA as **unknown**; the release tag
  alone does not establish source attribution.
- Deployment of `firecraft-web` stays a manual operator action: the
  Cloudflare Pages step in `release.yml` is commented out, so the release
  workflow does not deploy `firecraft-web`. Separately, `deploy-site.yml`
  deploys `firecraft-site` on matching pushes to `main` and by manual
  dispatch. The web app redeploy is gated behind explicit maintainer
  approval (campaign §20).

<!-- /coderabbit-ai-generated:v1 -->

- Until the redeploy happens, the live site is stale and unattributed.
  No evidence rung above 5 may be claimed for it, and it is not evidence
  of any release.
- The deployed bytes are deliberately **not** promoted to a release and
  not added to the release asset list, even though they are the only
  bytes a WebOS user can currently fetch.

The evidence and the comparison table live in
[the artifact matrix](../release/ARTIFACT-MATRIX.md#live-web-deployment-b-6-reconciliation);
the underlying finding is **B-6** in
[the Phase 0 audit](../release/PHASE-0-AUDIT.md). Tracked by issue #81
and task #158.
