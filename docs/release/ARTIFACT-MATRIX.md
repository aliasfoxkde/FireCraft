# Release artifact matrix — RP-00 inventory (T27)

> **Reconciled 2026-11-09** against the Phase 0 baseline audit
> ([PHASE-0-AUDIT.md](PHASE-0-AUDIT.md), PR #16, branch
> `docs/phase-0-baseline-audit`). The audit is the evidence authority
> for what exists; [decision 0075](../decisions/0075-release-scope-mvp.md)
> is the release-scope authority.
>
> Authority: [RELEASE-ENGINEERING-PLAN.md](../RELEASE-ENGINEERING-PLAN.md).
> Execution ledger: [EXECUTION-BOARD.md](EXECUTION-BOARD.md) — one atomic row
> per unit of work, with the evidence and the blockers for each.
>
> One row per artifact. Each row states **what exists today**, **how it
> is produced**, and **its evidence state** — a status word from the
> campaign vocabulary plus the receipt that supports it.
>
> Status vocabulary (campaign §3.2): `NOT STARTED` · `READY` ·
> `IN PROGRESS` · `BLOCKED` · `IMPLEMENTED` · `BUILD VERIFIED` ·
> `INSTALLED` · `LAUNCHED` · `SMOKE TESTED` · `PUBLISHED` · `CERTIFIED` ·
> `DEFERRED` · `REJECTED`.
>
> **Recipe ≠ built ≠ installed ≠ launched ≠ published ≠ certified.**
> Those are separate states and are never collapsed. A row with no
> receipt is `IMPLEMENTED` or `BLOCKED`, never `PUBLISHED` or
> `CERTIFIED`.

## Reading the matrix

- **Status** is the artifact's state on the campaign's 11-rung evidence
  ladder (audit §3): 1 recipe exists → 11 downloaded published artifact
  independently verified.
- **Receipt** names what was observed, on what host, on what date. A
  build receipt without a date is not a receipt.
- **Blocker** is the verified blocker ID from the audit's register
  (§4). A blocker ID means the condition was *observed*, not inferred.
- The evidence ladder has a hard ceiling at **rung 8** for everything in
  this repository: no artifact is signed and no provenance/attestation
  exists (B-14), so **no artifact can reach rung 9**, and therefore
  **nothing is `CERTIFIED`**.

## Target matrix

| # | Target | Format | What exists today | How it is produced | Status | Rung | Receipt | Blocker |
|---|---|---|---|---|---|---|---|---|
| 1 | Linux x86_64 | tar.gz (desktop) | `firecraft-0.2.5-linux-x86_64.tar.gz` | `xtask dist --native` → `dist/native/` | `PUBLISHED` | 11 | GitHub release v0.2.5 asset, 22,084,726 B. Downloaded and checksum-verified 2026-11-09; `sha256sum -c` passes against the published `MANIFEST.sha256`; unpacks to `firecraft-0.2.5/bin/{firecraft,firecraft-cli}` + `share/licenses/LICENSE` | B-18 (a rebuild of the current tree does not reproduce these bytes), B-21 (published out of band, not by the release workflow) |
| 2 | Linux x86_64 | tar.gz (CLI) | `firecraft-cli-0.2.5-linux-x86_64.tar.gz` | `xtask dist --native` → `dist/native/` | `PUBLISHED` | 11 | GitHub release v0.2.5 asset, 8,072,377 B. Downloaded and checksum-verified 2026-11-09; the extracted `firecraft-cli` **launches** and prints its usage | B-18, B-21 |
| 3 | Linux x86_64 | DEB | `firecraft_0.2.5_amd64.deb` | `xtask dist --native` from the pinned `packaging/deb/control`; the gate reads the package back with `dpkg-deb` and fail-closes on the declared libc6 floor against the binaries' max GLIBC symbol version (via `objdump`) | `PUBLISHED` | 11 | GitHub release v0.2.5 asset, 14,819,904 B. Downloaded and checksum-verified 2026-11-09; `dpkg-deb --info` reads `Version: 0.2.5`, `Architecture: amd64`; the extracted `firecraft-cli` launches. **Clean-container smoke passed 2026-11-06** on the Debian 12 image: `apt install` → functional CLI import/convert/export round trip → reinstall → remove → purge | **B-7 — OPEN DEFECT.** The declared glibc floor is below the binaries' real requirement, confirmed on the **published** artifact by `objdump -p`, not only on a local rebuild. The floor numbers and the `objdump` floor logic are owned by the stacked `fix/deb-glibc-floor` PR; this row deliberately does not restate them. A package built from this tree under-declares its dependency, so it installs cleanly and then fails to start on older distributions. **This is a defect, not a settled fact.** **Desktop integration is not covered by the recorded smoke** (board T27-22): the payload stages only `usr/bin/{firecraft,firecraft-cli}` + `usr/share/doc/firecraft/copyright` — no `.desktop` entry, no icons, no MIME registration |
| 4 | Linux x86_64 | AppImage | recipe only — `packaging/build-appimage.sh`, with `packaging/appimage/firecraft.desktop` carrying `Icon=firecraft` and the icon shipping in-tree at `assets/icons/hicolor/256x256/apps/firecraft.png` | operator-run on a host with `linuxdeploy`; the script is `bash -n` parse-checked by `dist --native` | `NOT STARTED` | 1 | recipe exists and passes the gate's `bash -n` check; the missing-`Icon=` build failure was fixed in-tree (`a4d74a0`) but the recipe has **never been re-run end to end** | B-8 (`linuxdeploy` absent, no pinned copy vendored), B-9 (the audit's wording predates the icon fix: `Icon=firecraft` and the icon file are present on this branch, but linuxdeploy has still never run), B-10 (the script stages `target/release/…` and ignores `CARGO_TARGET_DIR`) |
| 5 | Linux x86_64 | Flatpak | recipe only — `packaging/build-flatpak.sh` + `packaging/flatpak/dev.firecraft.FireCraft.yml` (both added on this branch; **no recipe on `main`**) | operator-run on a host with the Flatpak SDK; the script is `bash -n` parse-checked and the manifest is YAML-parse-checked by `dist --native` | `NOT STARTED` | 1 | recipe and manifest exist on this branch and the gate parse-checks them; **never built** | B-11 (`flatpak` and `flatpak-builder` absent). The manifest's module source is `type: dir, path: ..` — the local working tree, not an immutable upstream revision — so a bundle built from it is **not reproducible from the tag SHA** (board T27-29) |
| 6 | Windows x86_64 | portable ZIP | recipe only — `packaging/build-windows.ps1` | operator-run on a Windows host with MSVC | `NOT STARTED` | 1 | recipe exists; the gate **existence-checks** it only — it cannot parse PowerShell on this fleet; **never executed** | B-12 (`pwsh` absent; no MSVC linker on this host). A GNU cross-build and a wine smoke are recorded on the PR #9 branch — preview evidence only, never native certification (campaign §8). The required Windows artifact does not yet exist: the MSVC build, the installer and the clean-Windows-VM install are all outstanding (board T27-36/37/38) |
| 7 | Windows x86_64 | Inno Setup installer | recipe only — `packaging/windows/firecraft.iss` | compiled by the PowerShell recipe on a Windows host with Inno Setup 6 | `NOT STARTED` | 1 | recipe exists and is existence-checked by the gate; the `__APPVERSION__` placeholder is substituted by the PowerShell recipe, which has never run; **never built** | B-12 (`ISCC` absent). The PowerShell recipe *skips* the installer when `ISCC` is missing, so a Windows run without Inno Setup ships no installer and says nothing |
| 8 | macOS universal | DMG | recipe only — `packaging/build-macos-dmg.sh` | operator-run on macOS: two `apple-darwin` slices → `lipo` → `hdiutil` | `NOT STARTED` | 1 | recipe exists and is `bash -n` parse-checked by the gate; **never executed** | needs macOS + Xcode CLT; `lipo`/`hdiutil` are absent on this Linux host, so the recipe is off-fleet by design |
| 9 | Web (WASM) | ZIP bundle | `firecraft-web-0.2.5.zip` | `xtask dist --web` → `dist/web/`, then zipped | `PUBLISHED` | 11 | GitHub release v0.2.5 asset, 7,174,826 B. Downloaded 2026-11-09: 6 entries (bundle `firecraft-web-f9dbe2f14305b2d9{,_bg.wasm}`, `index.html`, `host.html`, `appinfo.json`, inner `MANIFEST.sha256`); `sha256sum -c` passes on all 5 manifest entries | **B-21 / F-16 — the published bundle was never built by the release workflow.** Its entries are stamped 2026-11-03 12:08, six hours before the workflow's own build finished at 18:15:59Z, and its wasm is a different size than the workflow's. No recorded bridge smoke ever ran against the published bytes. Separately, the **live deployment** serves a fourth bundle (`2274ac8c2925b61d`) that matches no release — B-6; the live comparison was **re-verified 2026-11-10** and is recorded in [Live web deployment](#live-web-deployment-b-6-reconciliation) below |
| 10 | WebOS | `appinfo.json` + `host.html` | staged into `dist/web/` by `dist --web` from `packaging/webos/` | `xtask dist --web` | `BUILD VERIFIED` | 5 | staged `appinfo.json` reads 0.2.5; the web manifest covers `appinfo.json` and `host.html`; the live site serves the same `appinfo.json` bytes — re-verified 2026-11-10, byte-identical (`7d039c36…`) | device/runtime smoke is `UNVERIFIED` (audit §6); the Pages deploy is manual and currently stale (B-6) |
| 11 | CLI binaries | `firecraft-cli` (inside #1 and #2) | `firecraft-cli` | `xtask dist --native` | `PUBLISHED` | 11 | the **published** `firecraft-cli` was downloaded and extracted 2026-11-09 and **launches**, printing its full usage string; the locally rebuilt one does the same | — |
| 12 | Source | tar.gz | `FireCraft-0.2.5-source.tar.gz` — **not on `main`** | `xtask dist --source` (git archive of HEAD; the verb verifies a clean extraction and runs `cargo metadata --locked` inside it). The verb exists on this branch and on the PR #9 / #12 trees | `BUILD VERIFIED` | 3 | produced 2026-11-09 on `docs/release-execution-board` @ `08eae4a`: 3.35 MB, 661 entries, sha256 `a0fcaafe…`; clean-extract + `cargo metadata --locked` verified inside the verb | F-9 (the verb is absent from `main`), B-13 (never published) |
| 13 | SBOM | CycloneDX 1.5 JSON | `firecraft-0.2.5-sbom.cdx.json` — **not on `main`** | `xtask dist --sbom` over the locked dependency graph; the serial is a deterministic UUIDv8 derived from the locked graph. The verb exists on this branch and on the PR #9 / #12 trees | `BUILD VERIFIED` | 3 | produced 2026-11-09 on `docs/release-execution-board` @ `08eae4a`: 249 KB, 493 components, 494 dependency edges, sha256 `837a731b…` | F-9 (the verb is absent from `main`), B-13 (never published) |
| 14 | Metadata | checksums / manifest | `dist/native/MANIFEST.sha256` (3 native artifacts) and `dist/web/MANIFEST.sha256` (5 bundle files) | `xtask dist --native` / `dist --web`; `release.yml` additionally writes a release-wide `RELEASE_SHA256.txt` over 8 assets | `PUBLISHED` | 11 | `MANIFEST.sha256` attached to release v0.2.5 (398 B, 3 native files) and embedded in the web ZIP (532 B). Both checked against the published downloads on 2026-11-09 — 3/3 and 5/5 pass | B-17 (neither list covers the ZIP files themselves and no release-wide `RELEASE_SHA256.txt` was ever published, although `release.yml` writes one), B-21 (the manifest and the assets it covers were published out of band, so the list is self-consistent but not independently attested) |
| 15 | Metadata | provenance / attestation | **nothing on any branch** | no generator, no `gh attestation` step, no in-toto/SLSA material anywhere in the repository | `NOT STARTED` | 0 | nothing exists | B-14 |
| 16 | Metadata | release manifest (JSON) | **nothing on any branch** | the campaign and the plan require a machine-readable artifact manifest; none exists | `NOT STARTED` | 0 | nothing exists | B-14 |
| 17 | Metadata | signature (Authenticode / Developer ID / GPG) | **nothing**; no signing identity is configured anywhere | no signing step in any workflow; signing remains an operator ceremony | `NOT STARTED` | 0 | nothing exists | B-14 — no artifact can reach rung 9 until this is resolved, so **no artifact can be `CERTIFIED`** |
| 18 | Android arm64 | APK/AAB | feasibility report only — [android-feasibility.md](../research/android-feasibility.md) | not started: no native entrypoint, no NDK/SDK CI image | `DEFERRED` | 0 | the report concludes "feasible with bounded work", disposition experimental | the feasibility-gate decision is unrecorded; no NDK/SDK image, no device |
| 19 | Linux | RPM | no spec, no recipe | — | `DEFERRED` | 0 | [decision 0070](../decisions/0070-rpm-deferred.md), accepted 2026-11-06 | decision 0070 is not on `main`; RPM is **not** reopened by [decision 0075](../decisions/0075-release-scope-mvp.md) |
| 20 | Linux aarch64 | any | nothing | RP-01 assessment | `DEFERRED` | 0 | no ARM64 host and no ARM64 test environment on the fleet | extended scope (decision 0075) |
| 21 | Windows ARM64 | any | nothing | RP-02 assessment | `DEFERRED` | 0 | no ARM64 host and no ARM64 test environment on the fleet | extended scope (decision 0075) |

### Status counts

| Status | Count | Artifacts |
|---|---|---|
| `NOT STARTED` | 8 | AppImage, Flatpak, Windows ZIP, Windows installer, macOS DMG, provenance/attestation, release manifest JSON, signature |
| `READY` | 0 | — |
| `IN PROGRESS` | 0 | — |
| `BLOCKED` | 0 | — |
| `IMPLEMENTED` | 0 | — |
| `BUILD VERIFIED` | 3 | WebOS package, source archive, SBOM |
| `INSTALLED` | 0 | — |
| `LAUNCHED` | 0 | — |
| `SMOKE TESTED` | 0 | — |
| `PUBLISHED` | 6 | desktop tar.gz, CLI tar.gz, DEB, web bundle, CLI binaries, checksums/manifest |
| `CERTIFIED` | 0 | — |
| `DEFERRED` | 4 | Android arm64, Linux RPM, Linux aarch64, Windows ARM64 |
| `REJECTED` | 0 | — |

**No artifact has reached `CERTIFIED`.** Five artifacts (desktop tar.gz,
CLI tar.gz, DEB, web bundle, CLI binaries) reach **rung 11** — they were
downloaded from the published release and verified against their
published checksum lists on 2026-11-09, and the CLI binary was launched.
Nothing has reached rung 9: no artifact is signed and no provenance
exists.

## Build surfaces (what actually exists)

- `xtask dist --native` — pinned Linux x86_64: builds `--locked`,
  stages, produces desktop tar.gz, CLI tar.gz and DEB (read back with
  `dpkg-deb --info`/`--contents`), writes `MANIFEST.sha256`, and
  parse-checks the packaging recipes.
- `xtask dist --web` — trunk wasm build, wasm size ceiling, manifest,
  and the headless bridge smoke. The smoke has no skip switch: a
  missing browser or a failed round trip fails the dist run (B-5 is
  remediated).
- `xtask dist --source` and `xtask dist --sbom` — **on this branch and
  the PR #9 / #12 trees; not on `main`** (F-9). Both are wired into
  `release.yml`.
- `.github/workflows/release.yml` — tag-driven: native + web + source +
  SBOM artifacts, a release-wide `RELEASE_SHA256.txt`, a fail-closed
  eight-asset aggregate validation, and a **draft** GitHub release whose
  assets are passed positionally. The Cloudflare Pages deploy step is
  **commented out** — deployment is manual.
- `.github/workflows/ci.yml` — backup CI (`fmt` + `gate`). The job's
  `Web bundle smoke` step is **tag-gated** (`if: startsWith(github.ref,
  'refs/tags/v')`) and is therefore **skipped on branch pushes**.
- `.gitforge.yml` — the eight-job pipeline; its `release` job builds
  the web bundle **with** the bridge smoke, in the headless chromium
  the CI image bakes (B-5 remediated).
- Recipes (operator-owned, off-fleet): `build-windows.ps1`,
  `build-macos-dmg.sh`, `build-appimage.sh`, `build-flatpak.sh`.

## Live web deployment (B-6 reconciliation)

**The deployed bundle is not the released bundle.** They are different
bytes, and the difference is recorded here rather than left to be
rediscovered at release time.

### What the live site serves (re-verified 2026-11-10)

Fetched from `https://firecraft-web.pages.dev/` on 2026-11-10 and checked
against the site's **own** `MANIFEST.sha256`:

| File | Live sha256 | Size | Manifest verdict |
|---|---|---|---|
| `appinfo.json` | `7d039c36…` | 250 B | **OK** |
| `firecraft-web-2274ac8c2925b61d.js` | `3b0403fc…` | 86,028 B | **OK** |
| `firecraft-web-2274ac8c2925b61d_bg.wasm` | `f4f0a3f0…` | 21,231,377 B | **OK** |
| `host.html` | expected `379720cc…` | 2,209 B served | **FAILED** |
| `index.html` | expected `2617be68…` | 2,409 B served | **FAILED** |

The two HTML failures are **not** corruption: Cloudflare Pages injects its
analytics beacon (`static.cloudflareinsights.com/beacon.min.js`) into the
served HTML after the manifest was written, so the served bytes can never
equal the built bytes. A live `sha256sum -c` therefore fails on those two
entries **by construction** — the manifest is only verifiable against the
pre-deploy bytes.

### Deployed vs released (same table, two builds)

| | Live deployment | Released v0.2.5 ZIP |
|---|---|---|
| bundle stem | `2274ac8c2925b61d` | `f9dbe2f14305b2d9` |
| `.js` sha256 | `3b0403fc…` | `be288850…` |
| `_bg.wasm` sha256 | `f4f0a3f0…` | `a8a4fa4f…` |
| `_bg.wasm` size | 21,231,377 B | 21,238,451 B (7,074 B larger) |
| `host.html` (pre-injection) | `379720cc…` | `379720cc…` — **identical** |
| `index.html` (pre-injection) | `2617be68…` | `fb2a5140…` — different |
| `appinfo.json` | `7d039c36…` | `7d039c36…` — **identical** |

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:80cec6c38b55085d90003b5fad6dff9ca2922022c500d27f2f1ec3dafe246ad5 -->

**Relationship: different artifact bytes; source relationship unknown
unless established by a build receipt.** The shared `host.html` and
`appinfo.json` show the two bundles carry the same WebOS shell files; the
differing wasm (7,074 B) and differing `index.html` establish that the
artifacts differ, but do not establish different Rust sources. No
receipt ties `2274ac8c2925b61d` to any commit SHA, so the deployed build
cannot be attributed to a source revision at all — it is an orphan. It
is also *not* the CI-built bundle recorded in the audit (21,231,517 B,
`cf1bec8315c7bfe5`): the live wasm differs from that by 140 B, so a
**fifth** distinct wasm build of nominally the same 0.2.5 release now
exists.

<!-- /coderabbit-ai-generated:v1 -->
### Decision

Recorded as [decision 0084](../decisions/0084-web-deployment-source-of-truth.md).

**The release is the source of truth; the deployment is redeployed from
the release bytes.** The reverse — cutting the release from the deployed
bytes — is rejected, for two reasons that are both evidence-backed:

1. The deployed bundle is **unattributable**: no build receipt, no
   commit SHA, no CI run ties `2274ac8c2925b61d` to any revision.
2. The deployed bytes are **not attestable**: two of the five files the
   deployment's own manifest covers can never verify against it, so a
   release cut from those bytes would publish an artifact whose
   checksum list fails by construction.

Consequences, recorded so the next release starts from a known state:

- The next web release must be **redeployed** from the release ZIP's
  bytes, and the redeploy must record the deployed bundle stem and the
  source SHA in a receipt. Deploying is an operator action (the Pages
  step in `release.yml` is commented out) and is **not** performed by
  this repository's automation.
- The live `MANIFEST.sha256` must **not** be used as the deployment's
  verification list. Verification happens against the pre-deploy
  `dist/web/MANIFEST.sha256`; the live HTML entries are expected to
  differ.
- Until a redeploy happens, the live site is **stale and unattributed**:
  it is not evidence of any release, and no rung above 5 may be claimed
  for it.

**Status: `BLOCKED`** — the reconciliation is complete and the decision is
recorded; the redeploy itself is gated behind explicit maintainer
approval (campaign §20 forbids publishing or deploying production
artifacts without it). Tracked by issue #81 / task #158, defect **B-6**.

## Known gaps against RP-00 acceptance

1. **Windows/macOS/AppImage/Flatpak outputs are recipe-only.** No
   artifact of those kinds has ever been produced on a target-native
   host. The GNU cross-build and wine smoke recorded on the PR #9 branch
   are preview evidence, never native certification (campaign §8).
2. **The DEB's declared glibc floor is below the binaries' real
   requirement** — confirmed on the published artifact (B-7). This is an
   open defect with an owner (the stacked `fix/deb-glibc-floor` PR), not
   a closed item. The floor numbers and the `objdump` check are that
   PR's to correct; this matrix deliberately does not restate them.
3. **The live web deployment is not reconciled with a release bundle**
   (B-6), and its own checksum list cannot verify it — Cloudflare
   Pages rewrites the two HTML files after the manifest is written, so
   `sha256sum -c` against the live site fails on those two entries by
   construction. **Reconciled 2026-11-10** — see
   [Live web deployment](#live-web-deployment-b-6-reconciliation): the
   deployed bundle is a different, unattributable build, and the
   decision is that the site is redeployed from the release bytes.
   The redeploy itself is gated behind maintainer approval.
4. **The release workflow has never produced a release** (B-4, B-21).
   All six published releases were operator-published by hand.
5. **The source archive and the SBOM have never been published** (B-13),
   and their verbs do not exist on `main` (F-9).
6. **No release-wide checksum file has ever been published** (B-17),
   and every published asset reports a null GitHub digest.
7. **Nothing is signed and no provenance exists** (B-14), so nothing
   can be `CERTIFIED`.
8. **RPM is deferred** by [decision 0070](../decisions/0070-rpm-deferred.md)
   and is **not** reinstated by
   [decision 0075](../decisions/0075-release-scope-mvp.md).
9. **The published artifacts are not reproducible** from the current
   tree (B-18), and the bundler version is not recorded in any build
   receipt (F-17). Four builds of nominally the same release differ by
   42,806 B.

## Toolchain / secrets inventory

- Fleet (Linux x86_64, Debian 13): rustup stable, trunk, dpkg-deb,
  chromium (bridge smoke). Per-artifact detail and the
  parse-check status of every recipe: [TOOLCHAINS.md](../TOOLCHAINS.md).
- Off-fleet operator: Windows MSVC + Inno Setup; macOS Xcode CLT +
  `hdiutil` + `lipo`; `linuxdeploy` for AppImage; Flatpak SDK for
  Flatpak. None of these hosts exists on this fleet.
- Secrets: `CLOUDFLARE_API_TOKEN` — optional, deploy step disabled.
  Whether it is configured in repository secrets is `UNVERIFIED` (audit
  §6). No signing credentials are configured anywhere — Windows
  Authenticode, macOS Developer ID, and GPG/nonce provenance are all
  absent (RP-06 scope, B-14).

## Scope note

This matrix describes **evidence state**, not release scope. Which of
these artifacts constitute an acceptable release cut is decided by
[decision 0075](../decisions/0075-release-scope-mvp.md), whose approval
state is *recommended — awaiting maintainer approval*.
