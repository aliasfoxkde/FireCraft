# T18 — Verify v0.2.5 release and web deployment

**Status:** the `v0.2.5` release is complete. On 2026-11-04, the served Pages manifest's JS/wasm/index hashes differed from local `dist/web`; the deployed source revision remains unknown.

**Objective:** verify the already tagged v0.2.5 release and confirm which
web build is live on Cloudflare Pages.

**Inputs:** `docs/RELEASING.md` (the release runbook); the 8 version
stamps (memory: firecraft-release-doors); `xtask dist --native/--web`;
`wrangler pages deploy firecraft-web`; GitForge tags only.

**Authority:** repo release policy (`RELEASING.md`); release
immutability per global scaffolding rules.

**Stopping condition:** record the GitHub release and tag receipts,
compare the served Pages manifest to the intended artifact, and record
the deployed SHA/build evidence.

**Verification:** smoke the live site (R/E shape-tool drag + export
diff), confirm release assets, and use the release-time CI receipt.
