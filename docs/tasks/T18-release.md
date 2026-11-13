# T18 — Verify v0.2.5 release and web deployment

**Status:** the `v0.2.5` release is complete. The live deployment is
**not** the v0.2.5 release artifact and is **not** reproducible from
the v0.2.5 tag — measured 2026-11-11, receipt below. The deployed
source revision remains unknown.

## Deployment receipt — 2026-11-11

Three distinct web bundles were compared by digest. The trunk asset
name carries a source-tree hash, so a differing name is a differing
source; the wasm digest is the build's byte identity.

| Bundle | trunk asset name | wasm bytes | wasm sha256 |
|---|---|---|---|
| **live** — `https://firecraft-web.pages.dev` | `2274ac8c2925b61d` | 21 231 377 | `f4f0a3f06fa3f5057485715c5c039fda50de9a6a0e72a451a62bf56366dbdb11` |
| **v0.2.5 release asset** — `firecraft-web-0.2.5.zip` | `f9dbe2f14305b2d9` | 21 238 451 | `a8a4fa4f28aa81a9e5fa6bb2a24eec1fe8a38b7ae4759333e12cc8c38b201865` |
| **v0.2.5 tag rebuild** — `xtask dist --web` at tag `v0.2.5` | `103c62d3366aef17` | 21 229 720 | `5934702ff141ea4cc7b0557abebaf95b78219adbc9ed2dc64607740a4a3e9e7b` |

Consequences, each measured rather than inferred:

- **The live bundle is not the v0.2.5 release asset.** Different asset
  name, different wasm digest, different byte count. The JS text
  differs only in wasm-bindgen closure `shim_idx` values and the
  internal wasm-bindgen symbol hashes — a toolchain difference, not
  proof of a source difference; the asset *name* is the source
  identity, and it differs.
- **The live bundle is not reproducible from the v0.2.5 tag** with
  the current CI toolchain either (the tag rebuild carries a third
  asset name). The published v0.2.5 asset was therefore built from a
  tree that is neither the tag nor the live deployment.
- **The live HTML difference is explained.** The served `index.html`
  is the build's own `index.html` with the asset names and their
  `integrity` digests substituted, plus the Cloudflare Pages
  analytics beacon injected by the CDN. That accounts for the whole
  2 195 → 2 409 byte difference; there is no unexplained drift.
- **`appinfo.json` matches** the release asset byte for byte
  (`7d039c36f28e9b1f67f5ee45fd7bdfd705873e716cc13b09d093be19ea0a3ffa`),
  so the live app reports version `0.2.5` — but every commit since the
  v0.2.5 stamp also reports `0.2.5`, so the version stamp alone
  identifies nothing.

The earlier reconciliation recorded on this ticket — that the live
site's JavaScript and wasm "match the v0.2.5 release bytes" — is
**falsified**: the digests differ, and the match was inferred from
equal file *sizes* and an identical `appinfo.json`, not from
digests. The live site corresponds to no identifiable release, which
is what blocker B-6 in the task board says.

**What would close it:** deploy a bundle whose asset name and wasm
digest are recorded here, from a named commit, and re-measure the
live host against that record. That is a production publish and stays
gated for explicit maintainer approval.

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
