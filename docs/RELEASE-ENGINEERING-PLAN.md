# FireCraft Cross-Platform Release Engineering Plan

> **Status:** Approved execution plan; implementation in progress and not release-certified  
> **Created:** 2026-11-06  
> **Repository baseline:** `main` at `2cfdeeee9c45bc3d399e1b6ddb781506e5eb7fd2` (`v0.2.5`, observed 2026-11-10)  
> **Owner:** FireCraft maintainers  
> **Purpose:** Give parallel agents an authoritative, evidence-driven plan for producing, validating, signing, and publishing a complete multi-platform FireCraft release.

This plan is a work order, not a claim that the listed packages or platform builds already exist. A checked-in script, workflow definition, successful cross-compilation, or CI artifact upload is not by itself proof that an end-user release works. Every shipped artifact needs a build receipt and an install/launch or equivalent runtime verification receipt on its target platform.

## 1. Release objective

Produce a complete, version-consistent release set for FireCraft with:

- Native desktop builds and installable packages for Windows, macOS, and Linux.
- Linux distribution formats: AppImage, Debian package, Flatpak, and RPM where support can be maintained.
- Android APK and Play-ready AAB **only after a platform feasibility spike proves the existing application can run as a native Android application**. Do not mislabel the current browser/WASM build as a native Android app.
- The existing browser/WASM distribution, plus WebOS package if it remains supported.
- CLI-only binaries where the CLI is useful and supported on the target.
- Complete source archives and source/build metadata.
- SHA-256 checksums, SBOM, build provenance/attestations, and cryptographic signatures where signing credentials are configured.
- Release notes, installation/upgrade/uninstall instructions, known limitations, and a machine-readable artifact manifest.
- Automated CI build and test coverage, with target-native runtime tests and explicit evidence for any target that cannot be tested in hosted CI.

The release should be reproducible from a clean, tagged source tree and must never mix outputs from different commits or versions.

## 1.1 MVP v1 candidate — recommended first release cut

The complete matrix below is the cross-platform roadmap, not a reason to
hold the first stable release indefinitely. For an initial public MVP,
the recommended candidate scope is:

- **Web/WASM:** optimized tagged build, bridge smoke against the release
  bytes, Cloudflare Pages deployment, and a live manifest/hash comparison
  proving the deployed bundle is the one being released.
- **Linux x86_64:** desktop tar.gz and DEB, plus CLI archive. Require
  install/launch checks and a small open → edit → save/export smoke on a
  clean supported Linux environment; require CLI version/help and one
  import/export round trip.
- **Release metadata:** exact-tag source archive, CycloneDX SBOM,
  SHA-256 manifest and release-wide checksum list, release notes, and a
  known-limitations/install guide.
- **Release gate:** the current PR remediation must be merged, the
  canonical `cargo run -p xtask -- ci` gate must pass, packaged artifacts
  must be verified, and the deployed web hashes must match the tagged
  build before publishing.

For this MVP candidate, Windows GNU cross-builds are preview evidence,
not a production Windows release. A native MSVC build/launch, macOS
DMG/signing, AppImage, Flatpak, Android, ARM64, and RPM remain separate
follow-on targets and must not be advertised as supported production
packages until their target-specific acceptance evidence exists. RPM is
deferred by decision 0070.

This is a recommended scope cut for maintainer approval, not an automatic
change to the long-term cross-platform target matrix. If v1 is required to
include every platform in that matrix, retain the full matrix as a release
blocker and plan the corresponding native-runner work explicitly.

## 2. Baseline inventory and constraints

The repository currently has a useful release foundation:

- Workspace version is `0.2.5`; workspace package metadata uses Rust edition 2024 and `rust-version = 1.95`.
- `cargo run -p xtask -- ci` is the canonical quality gate.
- `cargo run -p xtask -- dist --web` stages the web bundle, checks the WASM size ceiling, writes a SHA-256 manifest, and normally runs a bridge smoke test. The web release build uses Rust opt-level=z and Binaryen wasm-opt=z. GitHub tag CI runs the browser smoke; GitForge still has a temporary smoke-skip path until PR #40's Chromium image work is integrated. A skipped smoke is not release acceptance.
- `cargo run -p xtask -- dist --native` currently targets Linux x86_64 and produces desktop+CLI tar.gz, CLI-only tar.gz, and .deb packages.
- Existing packaging recipes: `packaging/build-appimage.sh`, `packaging/build-macos-dmg.sh`, `packaging/build-windows.ps1`, and `packaging/windows/firecraft.iss`.
- `docs/TOOLCHAINS.md` documents those recipes as off-fleet/operator-built outputs rather than verified release artifacts.
- `.github/workflows/release.yml` stages Linux native, web, source-archive, and SBOM outputs on tag builds, validates the aggregate, and creates a draft GitHub Release. Signing and Cloudflare Pages deployment remain outside the automated release path.
- A Flatpak manifest and build recipe exist, but no verified Flatpak artifact is established. No native Android build is established; Android remains feasibility-gated.
- The public Pages deployment previously differed from local `dist/web` hashes. Deployment reconciliation remains a release blocker.
- The release process currently treats signing as an operator ceremony. Secrets and credentials must remain outside the repository.

For application cleanup, code refactoring, WASM profiling, UI polish, agent ownership, and cross-workstream sequencing, follow [PRODUCTION-READINESS-PLAN.md](PRODUCTION-READINESS-PLAN.md).

Before changing packaging architecture, inspect the current scripts, workflows, `xtask` distribution code, workspace features, app entrypoints, and all existing documentation. Extend working mechanisms where practical; do not create parallel packaging implementations without a reason.

## 3. Artifact matrix and completion rules

The initial release target matrix is below. Agents may propose dropping a target only with a written compatibility/toolchain rationale and maintainer approval. Mark each row **Built**, **Installed**, **Launched**, **Smoke-tested**, and **Published** independently.

| Platform | Required deliverables | Architectures | Minimum acceptance evidence |
|---|---|---|---|
| Windows desktop | Portable ZIP; signed Setup EXE (Inno Setup) | x86_64 required; ARM64 feasibility assessment | Clean Windows VM install/launch, file open/save/export, uninstall, signature verification |
| macOS desktop | DMG; optionally PKG for managed deployment | Universal arm64+x86_64 preferred; otherwise separate native artifacts | Install on supported macOS, Gatekeeper/notarization validation, launch, open/save/export, uninstall |
| Linux desktop | AppImage; DEB; Flatpak; tar.gz; RPM if supported | x86_64 required; aarch64 target assessment/build | Install/run on clean distro images, desktop entry/icon/MIME handling, open/save/export, uninstall |
| Android | APK for sideload/testing; AAB only for Play distribution | arm64-v8a required; x86_64 emulator useful | Native install/launch, storage/document picker, touch/UI, lifecycle/resume, import/edit/export, signed package |
| Web | Static optimized WASM/JS bundle | Browser-supported targets | Release-byte browser smoke; open/edit/export; bridge tests; deployed hash match |
| WebOS | Version-stamped package/host integration if still supported | Target device/runtime | Package validation and real device/runtime smoke |
| CLI | Standalone archive/binary where supported | Match supported native targets | CLI help/version, command smoke, file operations where applicable |
| Source | GitHub source ZIP/tar.gz plus canonical source archive | Architecture-independent | Clean extraction, locked build/test, license and vendored-source audit |
| Metadata | SHA256SUMS/MANIFEST, SBOM, provenance, signatures, release JSON | All published artifacts | Independent verification from downloaded release assets |

### 3.1 Windows

- Retain and audit `packaging/build-windows.ps1` and `packaging/windows/firecraft.iss`.
- Produce a portable ZIP with a stable top-level directory and documented portable/user-data behavior.
- Produce an Inno Setup installer with correct product identity, version, publisher, install scope, Start Menu shortcut, optional desktop shortcut, file associations only if implemented, upgrade behavior, and clean uninstall.
- Build with the MSVC toolchain on a Windows runner. Do not treat Linux cross-compilation as a substitute for Windows launch verification.
- Sign EXE/installer and relevant executable payloads with Authenticode when a certificate is configured. Verify signatures and timestamping. Unsigned artifacts must be explicitly labeled as unsigned test builds, not production-ready Windows releases.
- Validate Windows Defender/SmartScreen reputation limitations honestly; signing alone does not guarantee reputation.
- Test clean install, upgrade from prior version, launch, open/save/export, paths containing spaces and non-ASCII characters, user-data preservation, uninstall, and reinstall.
- Assess ARM64 separately. Do not publish ARM64 claims until the native target and dependencies build and launch on Windows ARM64.

### 3.2 macOS

- Audit `packaging/build-macos-dmg.sh`; build on macOS runners with Xcode Command Line Tools.
- Prefer a universal2 binary containing arm64 and x86_64 slices if every native dependency supports both. Otherwise publish clearly named architecture-specific DMGs.
- Create a polished DMG with app bundle, icon, Applications shortcut, correct bundle identifier, Info.plist metadata, version, minimum OS, and embedded licenses/notices.
- Ensure app bundle layout, resources, fonts, icons, and runtime dynamic-library references work after moving the app out of the mounted DMG.
- Use Developer ID Application signing, hardened runtime, secure timestamp, and Apple notarization/stapling for production distribution when credentials are available. Ad-hoc signing is a local test mechanism, not a substitute for notarized distribution.
- Test Gatekeeper assessment, first launch, update/replace behavior, open/save/export, permissions, and uninstall.
- Document signing identities, entitlements, notarization credentials, and secret names without recording secret values.

### 3.3 Linux

Required formats:

1. **AppImage:** self-contained runtime, desktop integration, icon, MIME metadata, architecture declaration, and AppImage runtime validation. Test execution with and without FUSE where supported (including extract-and-run).
2. **DEB:** correct package metadata, dependencies, architecture, maintainer scripts only when necessary, desktop entry, icon, MIME registration, and clean install/upgrade/remove tests on Debian/Ubuntu.
3. **Flatpak:** add a maintained manifest (prefer upstreamable Flathub-compatible manifest), pinned source/build inputs, correct runtime/SDK branch, app ID, finish-args with least privilege, filesystem/network permissions justified by actual features, desktop entry, icon, and metadata. Build with `flatpak-builder`, run with `flatpak run`, and test sandbox file access via portals/document picker. Never grant broad filesystem access merely to make a test pass.
4. **RPM:** add a spec and build recipe for Fedora/RHEL-compatible distributions if dependencies and release maintenance are supportable. Verify package metadata, dependency resolution, install/upgrade/remove, desktop integration.
5. **tar.gz:** retain as portable fallback, with stable layout, licenses, CLI and desktop binaries as appropriate.

- x86_64 is the baseline. Build aarch64 packages only after native/cross toolchain and dependency validation. Do not label x86_64 artifacts as generic Linux.
- Test at least one Debian/Ubuntu and one Fedora-family environment; add Flatpak sandbox test and AppImage launch test.
- Confirm Wayland and X11 behavior where supported, display scaling, file chooser, clipboard, fonts, and GPU fallback.
- Record runtime dependencies and minimum glibc baseline. Prefer building on the oldest supported glibc baseline or using a documented container/sysroot strategy.
- Snap is optional, not a release blocker unless maintainers explicitly choose to support it. Avoid multiplying formats without maintainers for them.

### 3.4 Android feasibility and release

Android is a **gated workstream**, not an assumed packaging task.

- First inspect `crates/app`, `eframe`/egui configuration, native entrypoint, windowing, filesystem abstractions, dialogs, clipboard, fonts, GPU backend, and dependencies for Android target support.
- Run a minimal Android target spike with the exact app crate and locked dependencies. Establish whether an Android activity/entrypoint, JNI glue, Android-specific eframe integration, and NDK configuration are required.
- Identify unsupported APIs and platform assumptions: direct filesystem paths, save dialogs, clipboard, window lifecycle, permissions, backgrounding, GPU, and file-provider/document-picker integration.
- Define a platform adapter for Storage Access Framework/document picker rather than assuming desktop filesystem semantics.
- Implement touch-sized controls, virtual keyboard behavior, orientation/resizing, lifecycle pause/resume, safe state persistence, and import/export through Android content URIs.
- Build APK for internal testing. Build AAB only when Play signing, package identity, versionCode/versionName, privacy/data-safety disclosures, content rating, screenshots, and store listing are ready.
- Test on a physical ARM64 device and emulator; test install/upgrade, open/import, edit, save/export, app background/restore, rotation, low-memory recovery, and uninstall.
- If the native port is not feasible without a substantial application-shell project, stop after the spike, document the blocker and estimate, and ship the existing responsive web/PWA experience as a separately labeled browser target. Do not call a web wrapper a native Android application.

### 3.5 Web and WebOS

- Preserve the canonical `xtask dist --web` path and release-byte bridge smoke.
- Remove the CI Chromium smoke skip by installing/caching a compatible headless browser in the release runner, or keep the skip only for non-release CI while making release certification require the smoke.
- Publish the exact staged `dist/web` output. Verify deployed `index.html`, JS, and WASM hashes against the artifact manifest after deployment.
- Exercise the real hosted site with Playwright: load, create/open document, edit, undo/redo, save/download, import, export PNG, export native document, reload, and bridge API.
- Reconcile Cloudflare Pages project/branch/commit and ensure deploy uses the tagged release bundle, not an unrelated working directory.
- Keep WebOS package stamping tests and add package/device smoke if WebOS remains in the supported matrix.

### 3.6 Source and developer distributions

- Publish GitHub-generated source ZIP/tar.gz for the exact release tag and a separately generated canonical source archive if additional files are needed.
- Ensure the source archive contains all workspace crates, `Cargo.lock`, licenses, README, contribution/build docs, release notes, required codegen inputs, and packaging manifests; exclude caches, local secrets, untracked artifacts, and personal data.
- Verify a clean extraction can run `cargo build --locked`, `cargo test --locked`, and the documented relevant `xtask` gates with declared prerequisites.
- Publish a source-only checksum and include commit SHA/tag, Rust toolchain, target triples, and build instructions.
- Do not claim byte-for-byte reproducible native binaries unless independent rebuilds produce matching hashes. Start with reproducible build inputs and provenance, then measure reproducibility.

## 4. Release metadata, trust, and supply-chain security

Every release must include:

- `SHA256SUMS` or a single top-level `MANIFEST.sha256` covering every downloadable artifact (including source archives and metadata files where practical).
- `release-manifest.json` with version, tag, commit SHA, build timestamp, artifact filename, target triple, package format, byte size, SHA-256, workflow run URL/ID, and verification status.
- SBOM in SPDX JSON and/or CycloneDX JSON, generated from locked dependency graphs and packaged native dependencies where tooling permits. Clearly state coverage boundaries.
- Build provenance/attestation linking artifact digest to repository, tag, workflow, runner, and source commit (GitHub artifact attestations/SLSA provenance if available).
- Detached signatures for checksums/manifest and signed native executables/installers where supported.
- License/notice bundle for MIT OR Apache-2.0 and all third-party dependencies/assets. Re-run license gate.
- No credentials, signing keys, tokens, keystores, provisioning profiles, or private configuration in artifacts or logs.
- Pin third-party GitHub Actions by full commit SHA for release workflows. Use least-privilege workflow permissions, protected environments for signing/publishing, and artifact retention/immutability where available.
- Treat unsigned builds as prerelease/test artifacts. Never imply that checksums alone establish publisher authenticity.

## 5. CI/CD architecture

Create or extend a tag-driven release workflow with separate build jobs and a final aggregation/publish job.

### 5.1 Proposed job graph

1. **preflight:** verify clean tag/version consistency, tag points to tested commit, changelog, licenses, lockfile, and full `xtask ci` gate.
2. **web-linux:** build optimized WASM bundle; run release-byte bridge smoke; package web and WebOS outputs.
3. **linux-native:** build x86_64 desktop + CLI; produce tar.gz and DEB; run package inspection and smoke.
4. **linux-appimage:** build AppImage on supported Linux runner/container; execute and smoke.
5. **linux-flatpak:** build with Flatpak SDK/runtime; install into test environment; run sandbox smoke.
6. **linux-rpm:** build RPM and test on Fedora-family image if RPM is in supported matrix.
7. **windows:** native Windows runner; portable ZIP + installer; install/launch/functional smoke; sign in protected release environment.
8. **macos:** native macOS runner; universal or split architecture DMG; codesign/notarize/staple and verify when credentials available; launch smoke.
9. **android:** gated native Android build/test; APK/AAB only when feasibility and signing prerequisites pass.
10. **source-metadata:** source archives, SBOM, manifest inputs, provenance metadata.
11. **aggregate:** download all successful artifacts, reject missing required targets, validate naming/version/digests, generate final manifest and checksums, attach signatures/attestations.
12. **publish:** create GitHub Release only after all required gates pass; upload all artifacts, source archives, notes, checksums, SBOM, provenance, and signature files.
13. **post-publish:** download from the public release URL, verify signatures/checksums, install representative artifacts, deploy web from the same release output, verify deployed hashes, and publish a verification report.

### 5.2 Workflow rules

- Use a matrix for target/format builds only where setup and verification are genuinely shared; keep platform-specific jobs explicit.
- Each build job uploads immutable artifacts with a unique name containing platform, architecture, format, and run identity.
- The aggregate job must fail if any required artifact is missing, duplicated, empty, mislabeled, or has inconsistent version/commit metadata.
- Optional/experimental targets (Android until proven, RPM until adopted, ARM64 until validated) must be declared in a machine-readable required/optional matrix. Optional failure must never be silently reported as success.
- Build from the exact tag SHA. Do not rebuild a mutable branch in a later job.
- Use `--locked` for Cargo builds and pin external tool versions.
- Cache dependencies, not release output directories. Do not let stale `target/` or `dist/` files leak into release staging.
- Run smoke tests on the packaged artifact, not only on the build-tree binary.
- Separate build, sign, and publish permissions. Fork PRs must not access signing secrets.
- Produce logs/receipts even when a target fails. Keep failures actionable and target-specific.

## 6. Versioning and artifact naming

- The root `[workspace.package] version` is the source of truth; keep existing `xtask changelog` and `dist` consistency checks.
- Ensure every package metadata file, installer, app bundle, Android manifest, Flatpak metadata, RPM spec, WebOS appinfo, CLI version output, and release manifest uses the same version.
- Tag format: `vX.Y.Z`; release workflow must verify tag == workspace version.
- Include architecture and format in names. Suggested examples:
  - `FireCraft-0.2.5-windows-x86_64-portable.zip`
  - `FireCraft-0.2.5-windows-x86_64-setup.exe`
  - `FireCraft-0.2.5-macos-universal.dmg`
  - `FireCraft-0.2.5-linux-x86_64.AppImage`
  - `firecraft_0.2.5_amd64.deb`
  - `firecraft-0.2.5-x86_64.flatpak` (or repository bundle/Flatpak ref naming if publishing a repo)
  - `firecraft-0.2.5-1.x86_64.rpm`
  - `FireCraft-0.2.5-android-arm64.apk`
  - `FireCraft-0.2.5-source.tar.gz`
- Confirm naming against ecosystem conventions before implementation; examples are not a mandate to violate DEB/RPM conventions.

## 7. Work packages and agent execution order

Agents should work in isolated branches/worktrees. One owner per work package; parallelize only after shared interfaces and acceptance criteria are agreed. Every PR must include changed files, commands run, results, artifact paths/digests, limitations, and follow-up tickets.

### RP-00 — Inventory and release contract (P0)

**Objective:** Turn this plan into an implementation-ready target matrix and map existing code/scripts/workflows.

Tasks:
- Inspect all existing packaging scripts, workflow YAML, `xtask dist`, app crate features, icons/resources, license handling, and release docs.
- Verify exact current output filenames and what each script actually builds.
- Identify platform blockers, minimum OS versions, architecture support, runtime dependencies, and required secrets.
- Produce `docs/release/ARTIFACT-MATRIX.md` and update `docs/TOOLCHAINS.md`.
- Reconcile the stale public web deployment before any release claim.

Acceptance:
- No undocumented target assumptions.
- Each artifact has an owner, build host/toolchain, test host, required/optional status, and acceptance evidence.
- Current recipes classified as tested, parse-checked only, or unverified.

### RP-01 — Linux package completion (P0)

**Objective:** Make Linux a complete, tested package set.

Tasks:
- Extend `xtask dist --native` or add explicit subcommands without breaking existing output contracts.
- Audit AppImage recipe, produce AppImage, validate runtime and desktop integration.
- Add Flatpak manifest, metadata, icon, permissions, build script, and sandbox smoke.
- Add RPM spec/build/test if approved for support.
- Build x86_64 first; assess aarch64 separately.
- Add DEB clean install/upgrade/remove test on Ubuntu/Debian.
- Ensure all packages include license notices and correct version/architecture.

Acceptance:
- Required x86_64 AppImage, DEB, Flatpak, and tar.gz built from the same tag SHA.
- Install/launch/functional smoke receipts on clean environments.
- Checksums and manifest entries verified after download.
- No excessive Flatpak permissions; document every permission.

### RP-02 — Windows packaging and verification (P0)

**Objective:** Produce a production-quality Windows release.

Tasks:
- Audit/fix PowerShell packaging script and Inno Setup configuration.
- Build portable ZIP and installer on Windows x64.
- Verify resources, icon, shortcuts, version metadata, dependencies, and user data path.
- Add install/upgrade/uninstall/reinstall tests.
- Add Authenticode signing and timestamp verification through protected secrets when configured.
- Assess Windows ARM64 separately.

Acceptance:
- Actual x64 portable ZIP and setup EXE from tag SHA.
- Clean Windows VM install, launch, open/edit/save/export, upgrade, uninstall receipts.
- Signature validation if signing credentials exist; otherwise clearly marked unsigned prerelease.

### RP-03 — macOS packaging and notarization (P0)

**Objective:** Produce installable macOS desktop release.

Tasks:
- Audit DMG script and app bundle.
- Build universal2 if dependency graph permits; otherwise publish arm64 and x86_64 separately.
- Add bundle metadata, icon, licenses, minimum OS, and runtime dependency checks.
- Implement Developer ID signing, hardened runtime, notarization, stapling, and verification.
- Test on clean supported macOS hosts.

Acceptance:
- Actual DMG(s) from tag SHA.
- Gatekeeper/notarization verification for production release.
- Launch, open/edit/save/export, upgrade/replace and uninstall evidence.
- Explicit architecture/OS support declaration.

### RP-04 — Android feasibility spike and native port (P1 gated)

**Objective:** Determine whether FireCraft can ship as a real Android app, then implement only if feasible and approved.

Tasks:
- Produce a short feasibility report with exact Rust target, eframe integration, NDK/JNI, dependency blockers, and UI/storage changes.
- Build and launch a minimal app on emulator and physical device.
- If feasible, implement Android entrypoint, document picker/SAF, touch layout, lifecycle, storage, and export.
- Build APK; build AAB only for Play distribution readiness.
- Add signing configuration using secure CI secrets and store metadata only when required.

Acceptance:
- Feasibility report first; no fake completion via web wrapper.
- APK installs and passes import/edit/export and lifecycle tests on ARM64 device.
- AAB passes bundle validation if Play target is included.
- If blocked, publish explicit decision and retain Android as unsupported/experimental.

### RP-05 — Source, CLI, and web release assets (P0)

**Objective:** Ensure the release is usable by developers, browser users, and CLI users.

Tasks:
- Verify CLI target support and package archives.
- Generate source archives from the exact tag.
- Verify clean source extraction and locked build/test instructions.
- Run browser release-byte smoke with no skip in release certification.
- Reconcile Pages deployment to the exact release bundle.
- Retain WebOS only if target runtime support is tested and maintained.

Acceptance:
- Source archives, CLI assets, web bundle, and optional WebOS package have exact tag/commit metadata.
- Browser hosted deployment hashes match release manifest.
- Clean source extraction builds and passes the declared verification subset.

### RP-06 — Supply-chain metadata and signing (P1)

**Objective:** Make artifacts verifiable and trustworthy.

Tasks:
- Generate SPDX/CycloneDX SBOM.
- Generate provenance/attestations for each artifact digest.
- Sign release manifest/checksum file and platform executables/installers where possible.
- Add signature verification instructions.
- Audit action pinning, workflow permissions, secret isolation, and artifact immutability.

Acceptance:
- Every release asset has a SHA-256 entry and source commit association.
- SBOM and provenance published.
- Signature verification succeeds for signed artifacts.
- No secrets leak into logs/artifacts.

### RP-07 — Release orchestration and publication (P0)

**Objective:** Make a release tag produce one complete, validated release.

Tasks:
- Refactor `.github/workflows/release.yml` into preflight/build/aggregate/sign/publish/post-publish stages.
- Add required/optional artifact matrix and fail-closed aggregation.
- Publish release notes and all artifacts only after required gates pass.
- Verify public download and deployed web hashes.
- Produce a release verification report and archive workflow receipts.

Acceptance:
- A test tag produces all required artifacts from one commit.
- Aggregation fails on missing artifact, wrong version, wrong commit, duplicate name, or checksum mismatch.
- Public release assets can be downloaded and independently verified.
- Release report states every target as built/tested/published or explicitly unsupported.

### RP-08 — Final release certification (P0)

**Objective:** Certify the complete release, not merely its CI run.

Required final checklist:
- [ ] Correct signed/tagged source commit; clean tree; version consistency.
- [ ] Full `cargo run -p xtask -- ci` green.
- [ ] Release-byte browser smoke green (not skipped).
- [ ] Windows x64 installer/portable archive installed and tested.
- [ ] macOS DMG installed and tested; notarization verified for production.
- [ ] Linux AppImage, DEB, Flatpak, and tar.gz installed/launched/tested.
- [ ] RPM tested if declared supported.
- [ ] Android APK tested on physical device if declared supported; AAB validated if Play target.
- [ ] CLI binaries smoke-tested on each claimed target.
- [ ] Source archive clean-build check passed.
- [ ] SBOM, provenance, checksums, and signatures verified.
- [ ] GitHub Release assets match manifest.
- [ ] Production web deployment matches release bundle hashes.
- [ ] Release notes contain installation, upgrade, uninstall, OS/architecture support, and known limitations.
- [ ] No package is labeled supported based solely on cross-compilation or recipe presence.

## 8. Suggested parallel-agent assignment

| Agent | Assignment | Dependencies |
|---|---|---|
| Release coordinator | RP-00 inventory, artifact matrix, merge coordination, final report | None |
| Linux packager | RP-01 AppImage/DEB/Flatpak/RPM and distro tests | RP-00 |
| Windows packager | RP-02 PowerShell/Inno, native tests, signing integration | RP-00 |
| macOS packager | RP-03 DMG/universal build/notarization | RP-00 |
| Android specialist | RP-04 feasibility spike, then gated implementation | RP-00 |
| Web/source engineer | RP-05 web, CLI, source archives, deployment reconciliation | RP-00 |
| Supply-chain engineer | RP-06 SBOM, provenance, signing, action hardening | Artifact contract from RP-00 |
| CI/release engineer | RP-07 orchestration and aggregation | RP-01 through RP-06 interfaces |

Do not have multiple agents concurrently edit `.github/workflows/release.yml`, `docs/RELEASING.md`, `docs/TOOLCHAINS.md`, or the same `xtask` module. Assign one integration owner; other agents submit focused PRs.

## 9. Agent operating protocol

Every agent must:

1. Read `docs/HANDOFF.md`, `docs/TASKS.md`, this plan, `docs/RELEASING.md`, and `docs/TOOLCHAINS.md`.
2. Inspect current code and current branch state before editing. Do not trust stale task notes or this plan over executable behavior.
3. Create a dedicated branch/worktree and keep diffs focused.
4. Never commit generated binaries, secrets, local caches, or unverified build outputs into the source repository. Upload release artifacts to workflow artifacts/GitHub Releases.
5. Use locked dependencies and pinned toolchains; record tool versions and runner image.
6. Test the packaged artifact on its target runtime, not only the source binary.
7. Preserve existing release interfaces or document migrations and update every caller.
8. Add tests for packaging metadata, version stamping, manifest completeness, and failure cases.
9. Update `docs/TASKS.md`, `docs/TOOLCHAINS.md`, `docs/RELEASING.md`, and this plan with evidence-backed status as work lands.
10. Report exact commands, exit codes, logs/receipts, artifact names, sizes, SHA-256 digests, commit SHA, and unverified limitations.
11. Do not claim a target is done when only cross-compiled, recipe-checked, or uploaded.
12. Do not bypass failing security, license, test, or provenance gates to force publication.

### Required PR handoff template

- **Work package:** RP-XX
- **Source commit:** full SHA
- **Artifacts:** exact names, target triples, sizes, SHA-256
- **Build environment:** OS image, toolchain versions, commands
- **Verification:** install/launch/functional test commands and results
- **CI:** workflow/run/job URLs
- **Signing/provenance:** signature and attestation verification results
- **Known gaps:** explicit list; no silent deferrals
- **Docs updated:** exact paths
- **Next owner/action:** named work package

## 10. Release exit criteria

FireCraft is ready for a **complete cross-platform release** only when all required artifacts in the approved matrix have been built from one tagged commit, tested on their actual target runtime, independently checksum-verified, and published with release metadata.

Android, RPM, ARM64, and WebOS may be declared optional/experimental only through explicit maintainer decisions documented in the artifact matrix. The release must never silently omit a target that the release notes imply is supported.

A release is **not** complete merely because:
- the CI workflow is green while platform jobs are skipped;
- a package recipe parses;
- a cross-compiled binary exists but has not launched;
- an AppImage/Flatpak/installer is uploaded without installation testing;
- the source archive omits required build inputs;
- a manifest exists but does not cover all published assets;
- the website is live but does not match the tagged release.

## 11. Immediate execution order

1. RP-00: inventory and artifact contract; resolve release workflow and deployment facts.
2. RP-01 + RP-02 + RP-03 + RP-05: parallel native desktop, Linux formats, and web/source work.
3. RP-04: Android feasibility spike; proceed only on evidence.
4. RP-06: integrate SBOM/provenance/signing against stable artifact names.
5. RP-07: aggregate all target outputs and fail closed.
6. RP-08: build a release candidate, download it independently, install/test each supported target, and certify.

**Immediate rule:** do not cut or publish a new production tag until the existing PR #8 correctness findings are resolved or explicitly dispositioned, the release workflow is validated, and the release artifact matrix is agreed.
