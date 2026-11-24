# T18 — Verify v0.2.5 release and web deployment

**Status:** the `v0.2.5` release is complete. The live deployment was
**not** the v0.2.5 release artifact — measured 2026-11-11, receipt
below. It was **republished 2026-11-24 from `main` @ `6c68c9e3`**
under the maintainer-directed P0 web-deployment fix and re-measured
against the build's own manifest: every file matched, browser proof
PASS (receipt below). Later the same day a parallel release
experiment republished over it (deployment `29225fe2`), and the
maintainer directed that it stay live — so **the verified bundle is
no longer what production serves**, and the current live deployment
is recorded in the "superseded" section below. The reconciliation
closes for real when the next tagged publish (v0.2.7) is live with
its manifest.

## Deployment receipt — 2026-11-24 (republish, maintainer-approved)

The maintainer chose "Fix the web deployment (P0)" in the MVP gap
plan (2026-11-22), which is the recorded approval for this
production publish. `xtask dist --web` ran green at `6c68c9e3`
(6 files, wasm 15 783 714 B under the 25 000 000 warning ceiling,
manifest `MANIFEST.sha256`, bridge smoke green) and was deployed
with `wrangler pages deploy dist/web --project-name firecraft-web
--branch main`.

| Field | Value |
|---|---|
| deployment | `7f9adb41`, branch `main`, source `6c68c9e3` (= `origin/main`) |
| bundle | `firecraft-web-d25b8751233853ef` |
| wasm | 15 783 714 B — sha256 `44652f43a2a5e448edb7bf44ed4814a11d23c89939a573c4e57559152da52fd9` |
| JavaScript | sha256 `3248368c62eb0e9f2ce80c6bdfa34e4692dd87a26db97512b76a61a3c4a5cb32` |
| appinfo.json | `7d039c36…` — byte-identical to the manifest; still reports `0.2.5` because the version stamps move with the v0.2.7 cut |
| favicon.png | `6e5975dd…` |
| HTML | `index.html` `f9829371…`, `host.html` `379720cc…` (Pages serves it at `/host` via a 308 pretty-URL redirect) |

Measured, not inferred:

- **Every manifest file matches the served bytes.** sha256 of each
  served response equals `dist/web/MANIFEST.sha256`. The two HTML
  files match after stripping Cloudflare's injected analytics — an
  `<!-- Cloudflare Pages Analytics -->` comment plus the beacon
  script, the same documented delta as #104. That is the entire
  difference; there is no unexplained drift.
- **Browser proof PASS** (headless Chromium, `DISPLAY` unset — see
  the X11 note in the devlog): canvas 1280×800 painted (501 distinct
  colors in a probe grid), wasm loaded same-origin with a 200
  `application/wasm` response, zero CORS failures, zero page errors.
  Screenshot: `/var/tmp/fc-shot/production-proof.png`.
- **The prior unidentified bundle was gone at measurement time.**
  The old asset `2274ac8c2925b61d` (wasm `f4f0a3f0…`) no longer
  resolved on the live host's HTML; production served
  `d25b8751233853ef`. (Superseded the same day — see the section
  below.)
- **Operational gotcha, now known:** a bare `wrangler pages deploy`
  defaults to a *preview* deployment of the current git branch
  (first attempt landed on a `fix-web-deploy-p0` alias while
  production kept serving the old bytes). Production requires
  `--branch main`.

**Relationship to releases:** the verified live site was a `main`
branch build from a named commit, digest-verified. It was never yet
a release-tag artifact; republishing from the v0.2.7 tag is the
remaining step (workflow deploy path), after which this ticket's
receipt table should name the tag alongside the commit.

## Superseded the same day — current live deployment, 2026-11-24

After the receipt above was measured, a parallel release-experiment
workstream republished production from the `feat/wasm-cdn-deploy`
branch. Measured after the overwrite:

| Field | Value |
|---|---|
| deployment | `29225fe2`, environment Production, branch `main`, recorded source `fcf0b47` (= a commit on `feat/wasm-cdn-deploy`, pushed) |
| bundle | `firecraft-web-3841839c20c25d86`, wasm 19 571 892 B (built 2026-11-23 in `web/dist`, "v0.3.0 experiment" line) |
| manifest | **absent** — `/MANIFEST.sha256` falls through the Pages SPA fallback and serves `index.html` with 200, so the deployment's own checksum list cannot verify it |
| browser proof | PASS on 2026-11-24 (canvas 1280×800, 506 distinct colors, same-origin wasm 200, zero CORS/page errors) — it is a functioning app |

**Maintainer direction (2026-11-24):** leave the other workstream's
deployment alone — no re-deploy, no changes to its branch or
checkouts. The experiment's commits are pushed on
`feat/wasm-cdn-deploy`; nothing of theirs was touched.

Consequences, recorded rather than assumed:

- The verified-receipt bundle (`7f9adb41` / `d25b8751233853ef`) is
  **not** what production serves; re-asserting it is one
  `wrangler pages deploy dist/web --branch main` from a `dist --web`
  tree, deliberately not taken per the direction above.
- Blocker **B-6 stays open in a new form**: the live deployment
  still names no *release*, and now also serves no manifest.
- The next tagged publish (v0.2.7) with `--branch main` closes both.

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
gated for explicit maintainer approval. **Satisfied 2026-11-24** —
receipt above.

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
