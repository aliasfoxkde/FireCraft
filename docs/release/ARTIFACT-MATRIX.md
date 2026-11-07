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
| Linux x86_64 | DEB | **required, tested** | `firecraft_{v}_amd64.deb` | `xtask dist --native` (pinned `packaging/deb/control`) | built; read back with `dpkg-deb`; **no clean install/upgrade/remove test on Ubuntu/Debian yet** |
| Linux x86_64 | AppImage | **required, recipe-only** | `firecraft-{v}-linux-x86_64.AppImage` | `packaging/build-appimage.sh` (operator, linuxdeploy) | parse-checked by `dist --native`; **never built/runtime-tested** |
| Linux x86_64 | Flatpak | **planned, no recipe** | `firecraft-{v}-x86_64.flatpak` | RP-01 to create | nothing exists |
| Linux x86_64 | RPM | **pending decision** | `firecraft-{v}-1.x86_64.rpm` | RP-01 decision | nothing exists |
| Linux aarch64 | any | **unassessed** | — | RP-01 assessment | nothing |
| Windows x86_64 | portable zip | **required, recipe-only** | `firecraft-{v}-windows-x86_64.zip` | `packaging/build-windows.ps1` (operator, off-fleet) | parse-checked; **never built** |
| Windows x86_64 | Inno Setup installer | **required, recipe-only** | `firecraft-{v}-windows-x86_64-setup.exe` | same script, ISCC present | parse-checked; **never built** |
| Windows ARM64 | any | **unassessed** | — | RP-02 assessment | nothing |
| macOS universal | DMG | **required, recipe-only** | `firecraft-{v}-macos-universal.dmg` | `packaging/build-macos-dmg.sh` (operator, off-fleet) | parse-checked; **never built**; notarization/signing not configured |
| Android ARM64 | APK/AAB | **gated feasibility spike (RP-04)** | `FireCraft-{v}-android-arm64.apk` | RP-04 | nothing; no native claim permitted |
| Web (WASM) | zip bundle | **required, tested** | `firecraft-web-{v}.zip` | `xtask dist --web` (trunk + headless bridge smoke) | built each release; deployed to Cloudflare Pages — **live deployment is stale vs tag (RP-05 reconciliation)** |
| WebOS | `appinfo.json` + host page | **required, tested** | (inside web bundle) | `packaging/webos/` | built |
| Source | tar.gz | **planned** | `FireCraft-{v}-source.tar.gz` | RP-05 | GitHub auto-archive exists implicitly; verified-clean-extract archive not produced |
| Metadata | manifest/checksums | **partial** | `MANIFEST.sha256`, `RELEASE_SHA256.txt` | `dist --native` + release.yml | present; no SBOM, no provenance, no signatures |
| Metadata | SBOM / provenance / signing | **planned** | — | RP-06 | nothing |

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
- Missing entirely: Flatpak manifest, RPM spec, Android, SBOM,
  provenance, signing config, tag-driven fail-closed aggregate job,
  install/launch smoke on packaged artifacts.

## Known gaps against RP-00 acceptance

1. Windows/macOS/AppImage outputs never built — fleet has no toolchain;
   operator recipes only (documented in TOOLCHAINS.md, not hidden).
2. DEB has no clean-VM install/upgrade/remove test.
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
