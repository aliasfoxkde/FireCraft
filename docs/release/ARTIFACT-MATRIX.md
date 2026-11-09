# Release artifact matrix — RP-00 inventory (T27)

> Status: inventory of implementation and verification evidence as of 2026-11-07.
> The implementation branch is based on `5fac7a2`; recipe ≠ built ≠ installed
> ≠ launched ≠ published — states are tracked separately and never collapsed.
> Authority: [RELEASE-ENGINEERING-PLAN.md](../RELEASE-ENGINEERING-PLAN.md).
> Execution ledger: [EXECUTION-BOARD.md](EXECUTION-BOARD.md) — one atomic row
> per unit of work, with the evidence and the blockers for each.

## Target matrix

| Target | Format | Status | Artifact name | Built by | Verification state |
|---|---|---|---|---|---|
| Linux x86_64 | tar.gz (desktop) | **required, tested** | `firecraft-{v}-linux-x86_64.tar.gz` | `xtask dist --native` | built each release; sha256-manifested |
| Linux x86_64 | tar.gz (CLI) | **required, tested** | `firecraft-cli-{v}-linux-x86_64.tar.gz` | `xtask dist --native` | built; manifest |
| Linux x86_64 | DEB | **required, tested (CLI only)** | `firecraft_{v}_amd64.deb` | `xtask dist --native` (pinned `packaging/deb/control`; xtask verifies the libc6 floor against the binaries' max GLIBC symbol version via objdump) | built; `dpkg-deb` read-back; **clean-container smoke passed 2026-11-06** (Debian 12 image: apt install → functional CLI import/convert/export round trip → reinstall → remove → purge; caught and fixed a floor of 2.31 vs binaries needing 2.35). **Reconciled 2026-11-08:** the payload stages only `usr/bin/{firecraft,firecraft-cli}` + `usr/share/doc/firecraft/copyright` — no `.desktop`, no icons, no MIME — so the recorded smoke does not cover desktop integration (board T27-22) |
| Linux x86_64 | AppImage | **required, recipe + staged** | `firecraft-{v}-linux-x86_64.AppImage` | `packaging/build-appimage.sh` (operator, linuxdeploy) | parse-checked by `dist --native`; **linuxdeploy smoke on fleet 2026-11-06 failed-then-fixed**: continuous build hard-fails on a missing `Icon=` desktop entry ("ERROR: Icon entry missing", no output image) — fix landed (`a4d74a0`: in-tree original icon + `Icon=firecraft` + staging); full linuxdeploy run pending re-execution |
| Linux x86_64 | Flatpak | **required, recipe-only** | `firecraft-{v}-linux-x86_64.flatpak` | `packaging/build-flatpak.sh` + manifest `packaging/flatpak/dev.firecraft.FireCraft.yml` | manifest YAML-parse-checked by `dist --native`; **never built** (no Flatpak SDK on fleet). **Reconciled 2026-11-08:** the manifest's module source is `type: dir, path: ..` — the local working tree, not an immutable upstream revision — so a bundle built from it is not reproducible from the tag SHA (board T27-29) |
| Linux x86_64 | RPM | **unsupported** ([decision 0070](../decisions/0070-rpm-deferred.md)) | — | — | nothing; revisit on demonstrated demand |
| Linux aarch64 | any | **unassessed** | — | RP-01 assessment | nothing |
| Windows x86_64 | portable zip | **required, cross-built + wine-smoked (not the release artifact)** | `firecraft-{v}-windows-x86_64.zip` | cross-built on fleet (`x86_64-pc-windows-gnu`, cargo 19m39s); zip staged per the `build-windows.ps1` layout | **built 2026-11-06**: `0cd17a87…710f72`, 72 MB (both exes + LICENSE); **wine 10.0 smoke passed**: CLI `import`→`export` PNG round trip functional; GUI launched, real 1024×768 "FireCraft" window on Xvfb. Note: fleet zip is GNU-toolchain — the MSVC operator build remains the release-path authority. **Reconciled 2026-11-08:** the required Windows artifact does not yet exist — the MSVC build, the installer and the clean-Windows-VM install are all outstanding (board T27-36/37/38) |
| Windows x86_64 | Inno Setup installer | **required, recipe-only** | `firecraft-{v}-windows-x86_64-setup.exe` | same script, ISCC present | parse-checked; **never built** |
| Windows ARM64 | any | **unassessed** | — | RP-02 assessment | nothing |
| macOS universal | DMG | **required, recipe-only** | `firecraft-{v}-macos-universal.dmg` | `packaging/build-macos-dmg.sh` (operator, off-fleet) | parse-checked; **never built**; notarization/signing not configured |
| Android ARM64 | APK/AAB | **experimental** — feasibility: "feasible with bounded work" ([report](../research/android-feasibility.md)); support claim gated on ARM64 device + Android SDK runner | `FireCraft-{v}-android-arm64.apk` | not started (no entrypoint, no CI image) | report only |
| Web (WASM) | zip bundle | **required, tested** | `firecraft-web-{v}.zip` | `xtask dist --web` (trunk + headless bridge smoke) | built each release; deployed to Cloudflare Pages — **live deployment stale as of 2026-11-06: serves bundle `2274ac8c` vs current-tree `e44296eb`; reconciliation = task #158 (T18), publish action**. Fresh 2026-11-06 build green: `728936e0…fa447f`, wasm 21.27 MB (ceiling 21.5), 5 files, smoke green |
| WebOS | `appinfo.json` + host page | **required, tested** | (inside web bundle) | `packaging/webos/` | built |
| Source | tar.gz | **required, tested** | `FireCraft-{v}-source.tar.gz` | `xtask dist --source` (git archive of HEAD; clean-extract + `cargo metadata --locked` verified; wired into release.yml) | built + verified on fleet |
| Metadata | manifest/checksums | **required, tested** | `MANIFEST.sha256`, `RELEASE_SHA256.txt` | `dist --native` + release.yml | present |
| Metadata | SBOM (CycloneDX 1.5) | **required, implementation updated; verification pending** | `firecraft-{v}-sbom.cdx.json` | `xtask dist --sbom` (locked Cargo package inventory, source-appropriate bom-refs, resolved dependency edges; wired into release.yml) | earlier artifact was JSON-validated on fleet; rebuild and validate the updated graph output before release certification |
| Metadata | provenance / signing | **planned** | — | RP-06 remaining scope | no signing credentials configured anywhere; attestation blocked until they exist |

## Build surfaces (what actually exists)

- `xtask dist --native` — pinned Linux x86_64: builds `--locked`, stages,
  produces desktop tar.gz, CLI tar.gz, DEB (verified via `dpkg-deb`),
  writes `MANIFEST.sha256`, parse-checks the three packaging recipes.
- `xtask dist --web` — trunk wasm build + headless browser bridge smoke.
- `.github/workflows/release.yml` — tag-driven: native + web artifacts,
  explicit-file checksums, draft GH release with notes from
  `docs/releases/{tag}.md`. Cloudflare Pages deploy is **commented out**.
  **Reconciled 2026-11-08:** the asset-upload step is broken — it passes
  `assets+=(-a "$f")` to `gh release create`, which has no `-a` flag (gh 2.46.0
  takes assets positionally), so the draft-release upload cannot succeed as
  written (board T27-17). No end-to-end tag run has ever succeeded.
- `.github/workflows/ci.yml` — backup CI (fmt + gate); GitForge pipeline
  is primary.
- `.gitforge.yml` — the authoritative 7-job chain. **Reconciled 2026-11-08:**
  its release job sets `FIRECRAFT_DIST_SKIP_SMOKE=1`, so the authoritative CI
  never executes the web bridge smoke on the release path (board T27-16).
- Recipes (operator-owned, off-fleet per `packaging/TOOLCHAINS.md`):
  `build-windows.ps1`, `build-macos-dmg.sh`, `build-appimage.sh`.
- Still missing or incomplete: native Android build; provenance/attestation;
  signing configuration (no credentials); target-native install/launch
  smoke for every recipe-built package; and a successful end-to-end tag
  release run. The tag workflow includes a fail-closed artifact aggregate,
  but the draft-release upload step it feeds is itself broken
  (`gh release create -a`; reconciled 2026-11-08) — so "aggregate + upload"
  is **not** a working publication path today. Flatpak remains recipe-only;
  RPM is explicitly deferred by [decision 0070](../decisions/0070-rpm-deferred.md).
  Source archive and CycloneDX SBOM generation are implemented and wired
  into the tag workflow, but still require an end-to-end release run.

## Known gaps against RP-00 acceptance

1. ~~Windows/macOS/AppImage outputs never built — fleet has no toolchain;~~
   **partially closed 2026-11-06**: Windows portable zip cross-built on
   fleet (GNU toolchain) and wine-smoked (CLI round trip + GUI window);
   AppImage icon-blocker found via a real linuxdeploy run and fixed in
   the recipe (`a4d74a0`); full AppImage re-run and Inno installer still
   pending. macOS DMG remains operator-recipe-only.
2. ~~DEB has no clean-VM install/upgrade/remove test~~ — closed 2026-11-06: clean-container smoke passed (Debian 12; install/functional/reinstall/remove/purge), and `dist --native` now fail-closes on a control-file libc6 floor below the binaries' real GLIBC requirement.
3. Live web deployment not reconciled with a release bundle.
4. Source archive generation exists and is wired into the tag workflow;
   validate it on an actual tagged release.
5. CycloneDX SBOM generation exists; provenance and signing remain open.
6. The tag workflow validates the required eight-asset aggregate and
   recorded digests; execute an end-to-end tag run before certification.
7. RPM is explicitly deferred by decision 0070; no RPM package is planned
   for MVP v1.
8. **Reconciled 2026-11-08 (new):** the published `v0.2.5` release carries 5
   assets (desktop tar.gz, CLI tar.gz, web zip, DEB, `MANIFEST.sha256`) while
   the tag workflow now expects 8 — the release predates the source archive,
   the SBOM and `RELEASE_SHA256.txt`. A published release is not evidence that
   the current workflow works (board T27-19).
9. **Reconciled 2026-11-08 (new):** the Aegis security gate fails on the
   release PR head with 5 findings absent from the 520-entry committed
   baseline, so the release branch is not CI-green today (board T27-11/12/13).

## Toolchain / secrets inventory

- Fleet (Linux x86_64): rustup stable, trunk, dpkg-deb, chromium (web smoke).
- Off-fleet operator: Windows MSVC + ISCC; macOS Xcode CLT + hdiutil + lipo;
  linuxdeploy for AppImage. Pinning rules in `docs/TOOLCHAINS.md`.
- Secrets: `CLOUDFLARE_API_TOKEN` (optional, deploy step disabled).
  No signing credentials configured anywhere — Windows Authenticode,
  macOS Developer ID, GPG/nonce provenance all absent (RP-06 scope).
