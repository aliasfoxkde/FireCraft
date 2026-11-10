# Packaging recipes — toolchain requirements

`xtask dist` builds what the release fleet can build and verify:
`dist --web` (the static web bundle + bridge smoke; Rust opt-level=z and Binaryen wasm-opt=z), `dist --native`
(the Linux x86_64 tar.gz / deb / CLI set, manifest-verified),
`dist --source` (the clean-extract-verified source archive) and
`dist --sbom` (the CycloneDX 1.5 SBOM). Everything else under
`packaging/` is a **recipe** — a checked-in script the operator runs on
a host that has the toolchain.

> **Reconciled 2026-11-09** against the Phase 0 baseline audit
> ([PHASE-0-AUDIT.md](release/PHASE-0-AUDIT.md), PR #16 — the link
> resolves once that PR merges). Per-artifact evidence
> state: [ARTIFACT-MATRIX.md](release/ARTIFACT-MATRIX.md). Release
> scope: [decision 0075](decisions/0075-release-scope-mvp.md).

**The no-untested-claims rule:** the repo never carries an artifact it
did not build, and never claims a platform is "supported" beyond what a
gate exercised. A recipe shipped in `packaging/` has been **parse-checked
at most**; its *output* is the operator's to build and verify on the
target platform, with a receipt.

**Parse-checked is not built.** Three of the recipes below are
`bash -n`-checked by `xtask dist --native` on every run, so a broken
recipe fails the gate. Two of them cannot even be parsed on this fleet.
The off-fleet packaging recipes (AppImage, Flatpak, Windows, and macOS) have not yet produced target-native artifacts with install and launch receipts. This statement does not describe the separately built and published Linux tarballs, DEB, CLI, and web bundle.

## Recipe table

| Recipe | Target artifact | Toolchain | Runs on | Parse status on this fleet | Artifact ever produced? |
|---|---|---|---|---|---|
| `web/` (trunk) | `dist/web/` bundle | rustup `wasm32-unknown-unknown`, trunk, Cargo opt-level=z, Binaryen wasm-opt=z | release fleet (`xtask dist --web`) | n/a — it is built, not parsed | **Yes.** Built and bridge-smoked; published as the v0.2.5 web ZIP (rung 11) |
| `packaging/webos/` (`appinfo.json`, `host.html`) | WebOS packaging stamped `@VERSION@` into `dist/web/` | none (text copy + stamp) | release fleet (`xtask dist --web`) | n/a — data, not a script | **Yes.** Staged into `dist/web/` (rung 5); device/runtime smoke unverified |
| `packaging/deb/control` | `firecraft_<v>_amd64.deb` | dpkg-deb | release fleet (`xtask dist --native`) | n/a — data, not a script | **Yes.** Published (rung 11); the declared glibc floor is an open defect (B-7) |
| `packaging/build-appimage.sh` | `firecraft-<v>-linux-x86_64.AppImage` | linuxdeploy-x86_64.AppImage (FUSE or `--appimage-extract-and-run`), GNU coreutils | any Linux x86_64 host with linuxdeploy | `bash -n` **parse-checked** by `dist --native` | **No.** `linuxdeploy` is absent from this fleet (B-8) |
| `packaging/build-flatpak.sh` + `packaging/flatpak/dev.firecraft.FireCraft.yml` | `firecraft-<v>-linux-x86_64.flatpak` | flatpak + flatpak-builder, `org.freedesktop.Sdk//24.08` | any Linux x86_64 host with the Flatpak SDK | script `bash -n` **parse-checked**; manifest **YAML-parse-checked** by `dist --native` | **No.** `flatpak`/`flatpak-builder` absent (B-11) |
| `packaging/build-macos-dmg.sh` | `firecraft-<v>-macos-universal.dmg` | macOS + Xcode CLT, rustup targets `aarch64`/`x86_64-apple-darwin`, lipo, codesign, hdiutil | macOS only | `bash -n` **parse-checked** by `dist --native` | **No.** `lipo`/`hdiutil`/Xcode CLT absent on this Linux host |
| `packaging/build-windows.ps1` + `packaging/windows/firecraft.iss` | portable zip + optional Inno Setup installer | Windows, PowerShell 5.1+/7, MSVC (rustup target `x86_64-pc-windows-msvc` + VS Build Tools), optional Inno Setup 6 | Windows only | **cannot be parsed here** — no `pwsh`, no `ISCC`. The gate only checks that the files exist | **No.** `pwsh` absent and no MSVC linker on this fleet (B-12) |

Two of the three `bash -n`-checked recipes (`build-macos-dmg.sh`,
`build-appimage.sh`) plus the Windows recipe point at
`packaging/TOOLCHAINS.md`, which does not exist — the document is
`docs/TOOLCHAINS.md`. That is a defect in the recipes' comments, not a
missing document.

## Verification burden per artifact

- **web bundle / tar.gz / deb / CLI archives / source archive / SBOM** —
  verified by the dist gate itself: manifest `sha256sum -c`,
  `dpkg-deb --info`/`--contents`, the DEB glibc floor check against the
  binaries' real requirement, the clean-extract `cargo metadata
  --locked` check for the source archive, and the bridge smoke on the
  release bytes.
- **AppImage, Flatpak, dmg, Windows zip/installer** — built and verified
  by the operator from these recipes; record the sha256 of what you ship
  in the release notes. The dist manifest covers only what the fleet
  built. A GNU cross-build under wine is **preview evidence, never
  native certification**.
- **Parse status of the recipes**: `build-appimage.sh`,
  `build-flatpak.sh` and `build-macos-dmg.sh` are `bash -n`-checked by
  `xtask dist --native` on every run, so a broken recipe fails the gate;
  the Flatpak manifest is YAML-parsed by the same gate. The PowerShell
  and Inno Setup recipes cannot be parsed on this fleet (no `pwsh`, no
  `ISCC`); their syntax is verified on the target platform by whoever
  first runs them — if you do, fold the parse check into the recipe run.

## Versioning

Every recipe derives its version from `[workspace.package] version` in
the root `Cargo.toml` — the same number `packaging/deb/control` pins
literally (the dist gate fails on drift between the two).

## Platforms that are honestly unsupported today

The audit's toolchain inventory, observed 2026-11-09, fixes the
following absences on this fleet: `pwsh`, `ISCC`, `linuxdeploy`,
`flatpak`/`flatpak-builder`, `lipo`/`hdiutil`/Xcode CLT, and an MSVC
linker. Therefore:

| Platform / format | Honest support statement today |
|---|---|
| Linux x86_64 (tar.gz, CLI tar.gz, DEB) | **Supported on x86_64 Linux.** Built, manifest-verified, published, and (for the CLI) launched. The DEB's declared glibc floor is an open defect (B-7) |
| Web / WASM | **Supported.** Built, bridge-smoked, published. The *deployed* bundle is not reconciled to any release (B-6) |
| Linux AppImage | **Unsupported on this fleet** — recipe-only, `linuxdeploy` absent. Cross-compiled or recipe-only elsewhere is still preview evidence |
| Linux Flatpak | **Unsupported on this fleet** — recipe-only, Flatpak SDK absent |
| Windows x86_64 (ZIP, installer) | **Unsupported** — the recipe needs a Windows host with MSVC; `pwsh` and `ISCC` are absent here |
| macOS universal DMG | **Unsupported** — needs macOS + Xcode CLT |
| Android arm64 | **Unsupported / experimental** — a feasibility report only; no native entrypoint, no SDK image, no device |
| Linux aarch64, Windows ARM64 | **Unassessed** — no ARM64 host and no ARM64 test environment |
| Linux RPM | **Deferred** by [decision 0070](decisions/0070-rpm-deferred.md) |

A recipe is not a platform claim. "Supported" above means the fleet can
build the artifact and has a receipt for it.

## Planned release targets — not yet verified by this inventory

The cross-cutting code cleanup, WASM optimization, product QA, and agent execution sequence live in [PRODUCTION-READINESS-PLAN.md](PRODUCTION-READINESS-PLAN.md). The comprehensive target matrix and execution criteria live in
[`docs/RELEASE-ENGINEERING-PLAN.md`](RELEASE-ENGINEERING-PLAN.md)
(ticket [T27](tasks/T27-cross-platform-release.md)). Which of them
constitute an acceptable release cut is decided by
[decision 0075](decisions/0075-release-scope-mvp.md).

- Flatpak is recipe-only (manifest parse-checked) until a bundle is
  built and sandbox-smoked on a host with the Flatpak SDK. RPM is
  unsupported by decision 0070.
- Android is a feasibility-gated native port; no native Android support
  should be claimed until an APK is installed and exercised on a device.
- Existing Windows and macOS scripts are recipes until their output is
  built and launch-tested on native runners.
- AppImage is recipe-only until an actual image is built and runtime-tested.
- Source archive and CycloneDX SBOM generation are implemented on main in
  `xtask dist --source` and `xtask dist --sbom`, and are wired into the
  tag workflow. They still need an end-to-end tag-release receipt, a clean
  source build, and schema validation of the SBOM.
  Provenance/attestation and signed release metadata remain unimplemented
  until the required credentials and verification path are configured.

Update this inventory only when a target has concrete toolchain,
artifact, and verification evidence. Keep "recipe exists", "built",
"installed", "launched", and "published" as separate states.

Per-target status (required/optional/recipe-only/unbuilt) and the
secrets inventory live in
[`docs/release/ARTIFACT-MATRIX.md`](release/ARTIFACT-MATRIX.md)
(RP-00 output, ticket [T27](tasks/T27-cross-platform-release.md)).
