# Release artifact matrix — RP-00 inventory (T27)

> Status: inventory of **current** outputs as of 2026-11-06, baseline
> `5fac7a2` + T27 plan commit. Recipe ≠ built ≠ installed ≠ launched ≠
> published — states are tracked separately and never collapsed.
> Authority: [RELEASE-ENGINEERING-PLAN.md](../RELEASE-ENGINEERING-PLAN.md).

## Target matrix

| Target | Format | Status | Artifact name | Built by | Verification state |
|---|---|---|---|---|---|
| Linux x86_64 | tar.gz (desktop) | **required, tested** | `firecraft-{v}-linux-x86_64.tar.gz` | `xtask dist --native` | built each release; sha256-manifested |
| Linux x86_64 | tar.gz (CLI) | **required, tested** | `firecraft-cli-{v}-linux-x86_64.tar.gz` | `xtask dist --native` | built; manifest |
| Linux x86_64 | DEB | **required, tested** | `firecraft_{v}_amd64.deb` | `xtask dist --native` (pinned `packaging/deb/control`; xtask verifies the libc6 floor against the binaries' max GLIBC symbol version via objdump) | built; `dpkg-deb` read-back; **clean-container smoke passed 2026-11-06** (Debian 12 image: apt install → functional CLI import/convert/export round trip → reinstall → remove → purge; caught and fixed a floor of 2.31 vs binaries needing 2.35) |
| Linux x86_64 | AppImage | **required, recipe + staged** | `firecraft-{v}-linux-x86_64.AppImage` | `packaging/build-appimage.sh` (operator, linuxdeploy) | parse-checked by `dist --native`; **linuxdeploy smoke on fleet 2026-11-06 failed-then-fixed**: continuous build hard-fails on a missing `Icon=` desktop entry ("ERROR: Icon entry missing", no output image) — fix landed (`a4d74a0`: in-tree original icon + `Icon=firecraft` + staging); full linuxdeploy run pending re-execution |
| Linux x86_64 | Flatpak | **required, recipe-only** | `firecraft-{v}-linux-x86_64.flatpak` | `packaging/build-flatpak.sh` + manifest `packaging/flatpak/dev.firecraft.FireCraft.yml` | manifest YAML-parse-checked by `dist --native`; **never built** (no Flatpak SDK on fleet) |
| Linux x86_64 | RPM | **unsupported** ([decision 0070](../decisions/0070-rpm-deferred.md)) | — | — | nothing; revisit on demonstrated demand |
| Linux aarch64 | any | **unassessed** | — | RP-01 assessment | nothing |
| Windows x86_64 | portable zip | **required, built + wine-smoked** | `firecraft-{v}-windows-x86_64.zip` | cross-built on fleet (`x86_64-pc-windows-gnu`, cargo 19m39s); zip staged per the `build-windows.ps1` layout | **built 2026-11-06**: `0cd17a87…710f72`, 72 MB (both exes + LICENSE); **wine 10.0 smoke passed**: CLI `import`→`export` PNG round trip functional; GUI launched, real 1024×768 "FireCraft" window on Xvfb. Note: fleet zip is GNU-toolchain — the MSVC operator build remains the release-path authority |
| Windows x86_64 | Inno Setup installer | **required, recipe-only** | `firecraft-{v}-windows-x86_64-setup.exe` | same script, ISCC present | parse-checked; **never built** |
| Windows ARM64 | any | **unassessed** | — | RP-02 assessment | nothing |
| macOS universal | DMG | **required, recipe-only** | `firecraft-{v}-macos-universal.dmg` | `packaging/build-macos-dmg.sh` (operator, off-fleet) | parse-checked; **never built**; notarization/signing not configured |
| Android ARM64 | APK/AAB | **experimental** — feasibility: "feasible with bounded work" ([report](../research/android-feasibility.md)); support claim gated on ARM64 device + Android SDK runner | `FireCraft-{v}-android-arm64.apk` | not started (no entrypoint, no CI image) | report only |
| Web (WASM) | zip bundle | **required, tested** | `firecraft-web-{v}.zip` | `xtask dist --web` (trunk + headless bridge smoke) | built each release; deployed to Cloudflare Pages — **live deployment stale as of 2026-11-06: serves bundle `2274ac8c` vs current-tree `e44296eb`; reconciliation = task #158 (T18), publish action** |
| WebOS | `appinfo.json` + host page | **required, tested** | (inside web bundle) | `packaging/webos/` | built |
| Source | tar.gz | **required, tested** | `FireCraft-{v}-source.tar.gz` | `xtask dist --source` (git archive of HEAD; clean-extract + `cargo metadata --locked` verified; wired into release.yml) | built + verified on fleet |
| Metadata | manifest/checksums | **required, tested** | `MANIFEST.sha256`, `RELEASE_SHA256.txt` | `dist --native` + release.yml | present |
| Metadata | SBOM (CycloneDX 1.5) | **required, tested** | `firecraft-{v}-sbom.cdx.json` | `xtask dist --sbom` (full locked graph, deterministic serial; wired into release.yml) | built + JSON-validated on fleet |
| Metadata | provenance / signing | **planned** | — | RP-06 remaining scope | no signing credentials configured anywhere; attestation blocked until they exist |

## Build surfaces (what actually exists)

- `xtask dist --native` — pinned Linux x86_64: builds `--locked`, stages,
  produces desktop tar.gz, CLI tar.gz, DEB (verified via `dpkg-deb`),
  writes `MANIFEST.sha256`, parse-checks the three packaging recipes.
- `xtask dist --web` — trunk wasm build + headless browser bridge smoke.
- `.github/workflows/release.yml` — tag-driven: native + web artifacts,
  explicit-file checksums, draft GH release with notes from
  `docs/releases/{tag}.md`. Cloudflare Pages deploy is **commented out**.
- `.github/workflows/ci.yml` — backup CI (fmt + gate); GitForge pipeline
  is primary.
- Recipes (operator-owned, off-fleet per `packaging/TOOLCHAINS.md`):
  `build-windows.ps1`, `build-macos-dmg.sh`, `build-appimage.sh`.
- Missing entirely: Android, provenance/attestation, signing config
  (no credentials — see secrets inventory), tag-driven fail-closed
  aggregate job, install/launch smoke on packaged artifacts.
  (Flatpak recipe landed RP-01; RPM declined by
  [decision 0070](../decisions/0070-rpm-deferred.md); source archive
  and SBOM landed RP-05/RP-06.)

## Known gaps against RP-00 acceptance

1. ~~Windows/macOS/AppImage outputs never built — fleet has no toolchain;~~
   **partially closed 2026-11-06**: Windows portable zip cross-built on
   fleet (GNU toolchain) and wine-smoked (CLI round trip + GUI window);
   AppImage icon-blocker found via a real linuxdeploy run and fixed in
   the recipe (`a4d74a0`); full AppImage re-run and Inno installer still
   pending. macOS DMG remains operator-recipe-only.
2. ~~DEB has no clean-VM install/upgrade/remove test~~ — closed 2026-11-06: clean-container smoke passed (Debian 12; install/functional/reinstall/remove/purge), and `dist --native` now fail-closes on a control-file libc6 floor below the binaries' real GLIBC requirement.
3. Live web deployment not reconciled with a release bundle.
4. No source archive recipe.
5. No SBOM/provenance/signature pipeline.
6. Release workflow aggregates 5 assets without a required/optional
   matrix or fail-closed validation of naming/version/digests.
7. No RPM decision recorded.

## Toolchain / secrets inventory

- Fleet (Linux x86_64): rustup stable, trunk, dpkg-deb, chromium (web smoke).
- Off-fleet operator: Windows MSVC + ISCC; macOS Xcode CLT + hdiutil + lipo;
  linuxdeploy for AppImage. Pinning rules in `docs/TOOLCHAINS.md`.
- Secrets: `CLOUDFLARE_API_TOKEN` (optional, deploy step disabled).
  No signing credentials configured anywhere — Windows Authenticode,
  macOS Developer ID, GPG/nonce provenance all absent (RP-06 scope).
