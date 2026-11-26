# Phase 7 — Cross-Platform Packaging Matrix & Issue / PR / Branch Closeout

> **Goal:** Ratify the 1.0 release scope, harden every platform package (1 target per atomic task!), rehearse the full release workflow end-to-end, and systematically close out all 31 GitHub issues, open PRs (`#186`–`#188`), and obsolete branches/stashes.
> **Rule:** Never collapse the 9-rung verification ladder (`RECIPE` -> `BUILT` -> `INSPECTED` -> `INSTALLED` -> `LAUNCHED` -> `SMOKE TESTED` -> `SIGNED` -> `PUBLISHED` -> `DOWNLOAD-VERIFIED`).

---

### `L7.1` `[HUMAN GATE]` Ratify 1.0 Release Scope Decision (`0130`) & Signing Posture (`P5.1` + `P5.9`)

- **Files:** `docs/decisions/0130-firecraft-1.0-release-scope.md` (new)
- **Pre-check:**
  ```bash
  ls -la /nas/Temp/repos/FireCraft/docs/decisions/0070* /nas/Temp/repos/FireCraft/docs/decisions/0075*
  ```
- **Do:**
  1. Draft `docs/decisions/0130-firecraft-1.0-release-scope.md` superseding `0075` (narrow MVP) and `0070` (RPM deferred), explicitly specifying included OS/arch targets, modular folder layout, signing posture (unsigned vs Authenticode/Developer ID/GPG), and protected environments.
  2. **STOP at `[HUMAN GATE]`:** Obtain maintainer ratification before packaging.
- **Done when:** Maintainer signs off on `0130`.

---

### `L7.2` `[LUNA]` Standardize 1.0 Asset Naming, Manifest Schema & Release Self-Tests (`P5.2`)

- **Files:** `xtask/src/dist.rs`, `.github/workflows/release.yml`, `scripts/validate_release_aggregate.sh`
- **Pre-check:**
  ```bash
  cd /nas/Temp/repos/FireCraft && bash scripts/validate_release_aggregate-selftest.sh && bash scripts/classify_release_mode-selftest.sh
  ```
- **Do:**
  1. Align canonical 1.0 asset filenames, `RELEASE_SHA256.txt`, release manifest schema, and aggregate validator rules with decision `0130`.
  2. Run all 4 release script self-tests + `cargo test -p xtask`.
- **Verify:**
  ```bash
  cd /nas/Temp/repos/FireCraft && bash scripts/validate_release_aggregate-selftest.sh && bash scripts/classify_release_mode-selftest.sh && bash scripts/validate_workflow_pins-selftest.sh && bash scripts/verify_release_ledger-selftest.sh
  ```
- **Done when:** All release self-tests and `xtask` manifest tests pass with exit 0.

---

### `L7.3` `[LUNA]` Verify Linux DEB & Tarballs (`P5.3`)

- **Files:** `xtask/src/dist.rs`, `packaging/deb/*` (only if fixes needed)
- **Pre-check:**
  ```bash
  ls -la /nas/Temp/repos/FireCraft/packaging/deb/
  ```
- **Do:**
  1. Run `cargo run -p xtask -- dist --native` on `ssh fedora`.
  2. Inspect `.deb` metadata (`dpkg-deb -I`, `dpkg-deb -c`), confirm desktop icon + modular folders (`examples/`, `libraries/`, `extensions/`) + glibc floor, and verify reinstall/remove/purge in a clean Debian/Ubuntu container or host.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo run -p xtask -- dist --native && ls -lh dist/native/'
  ```
- **Done when:** Linux tar.gz, CLI tar.gz, and `.deb` pass inspection, manifest checks, and clean-host smoke.

---

### `L7.4` `[LUNA]` Harden & Build Linux AppImage (`P5.4`)

- **Files:** `packaging/build-appimage.sh`
- **Pre-check:**
  ```bash
  grep -n "target/release\|MIT\|linuxdeploy" /nas/Temp/repos/FireCraft/packaging/build-appimage.sh
  ```
- **Do:**
  1. Ensure `packaging/build-appimage.sh` honors `CARGO_TARGET_DIR`, declares `Apache-2.0` (not MIT), verifies a pinned `linuxdeploy` digest, and bundles `examples/`, `libraries/`, `extensions/`.
  2. Build the AppImage on `fedora` (or Linux runner), launch it under `xvfb-run`, and record exact byte size & SHA-256.
- **Verify:**
  ```bash
  bash -n /nas/Temp/repos/FireCraft/packaging/build-appimage.sh && grep -q "Apache-2.0" /nas/Temp/repos/FireCraft/packaging/build-appimage.sh
  ```
- **Done when:** AppImage builds reproducibly and launches cleanly.

---

### `L7.5` `[LUNA]` Harden Linux Flatpak (& Conditional RPM per `0130`, `P5.5`–`P5.6`)

- **Files:** `packaging/flatpak/dev.firecraft.FireCraft.yml`, `packaging/build-flatpak.sh` (and RPM spec if approved in `0130`)
- **Pre-check:**
  ```bash
  cat /nas/Temp/repos/FireCraft/packaging/flatpak/dev.firecraft.FireCraft.yml | head -n 45
  ```
- **Do:**
  1. Replace mutable `type: dir, path: ..` in Flatpak manifest with an immutable release archive/tag input and explicit XDG portal permissions.
  2. Build & sandbox-smoke the Flatpak bundle (if `flatpak-builder` is installed on `fedora`) and conditional RPM (if approved in `0130`).
- **Verify:**
  ```bash
  bash -n /nas/Temp/repos/FireCraft/packaging/build-flatpak.sh
  ```
- **Done when:** Flatpak manifest uses immutable inputs and passes local/CI validation.

---

### `L7.6` `[LUNA]` Harden Windows x64 ZIP + Inno Setup (`P5.7`)

- **Files:** `packaging/build-windows.ps1`, `packaging/windows/*`
- **Pre-check:**
  ```bash
  grep -n "ISCC\|RequireInstaller" /nas/Temp/repos/FireCraft/packaging/build-windows.ps1
  ```
- **Do:**
  1. Add `-RequireInstaller` fail-closed switch to `packaging/build-windows.ps1` so release CI NEVER silently skips `ISCC.exe`.
  2. Bundle `examples/`, `libraries/`, and `extensions/` into both the portable ZIP and Inno Setup installer.
  3. Verify on `windows-latest` GitHub Actions runner (`CI (cross-platform)` / release dry-run).
- **Verify:**
  ```bash
  grep -q "RequireInstaller" /nas/Temp/repos/FireCraft/packaging/build-windows.ps1
  ```
- **Done when:** `windows-latest` runner builds both portable ZIP and Inno Setup `.exe` with fail-closed validation.

---

### `L7.7` `[LUNA]` Harden macOS Universal DMG (`.icns` + Signing Hooks, `P5.8`)

- **Files:** `packaging/build-macos-dmg.sh`
- **Pre-check:**
  ```bash
  grep -n "CFBundleIconFile\|codesign\|icns" /nas/Temp/repos/FireCraft/packaging/build-macos-dmg.sh || true
  ```
- **Do:**
  1. Generate a proper `.icns` app icon (`CFBundleIconFile`), include `examples/`, `libraries/`, `extensions/` inside `FireCraft.app/Contents/Resources/`, and add env-driven Developer ID signing + notarization hooks (cleanly falling back to ad-hoc only in dry-run/unsigned mode).
  2. Verify on `macos-latest` GitHub Actions runner.
- **Verify:**
  ```bash
  bash -n /nas/Temp/repos/FireCraft/packaging/build-macos-dmg.sh && grep -q "CFBundleIconFile" /nas/Temp/repos/FireCraft/packaging/build-macos-dmg.sh
  ```
- **Done when:** `macos-latest` runner builds `.app` + `.dmg` cleanly with valid metadata and icon.

---

### `L7.8` `[LUNA]` Verify Standalone Web ZIP, Source Archive, CycloneDX SBOM & Provenance (`P5.10`–`P5.11`)

- **Files:** `xtask/src/dist.rs`, `xtask/src/provenance.rs` (only if fixes needed)
- **Pre-check:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && cargo test -p xtask sbom'
  ```
- **Do:**
  1. Extract `firecraft-web-<v>.zip` into a clean temp dir and verify standalone static serving (including `examples/`, `libraries/`, `extensions/`, `modules/`, `.gz`, `.br`) passes Playwright E2E.
  2. Extract the Source archive into a clean temp dir and verify offline/locked build checks.
  3. Validate generated CycloneDX 1.5 SBOM against the vendored JSON schema and verify provenance attestation format.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p xtask'
  ```
- **Done when:** Web ZIP, Source archive, SBOM, and provenance all pass automated checks.

---

### `L7.9` `[LUNA]` Run Full Multi-Platform Dry-Run Rehearsal & Update `ARTIFACT-MATRIX.md` (`P5.12`)

- **Files:** `docs/release/ARTIFACT-MATRIX.md`
- **Pre-check:**
  ```bash
  gh workflow list --repo aliasfoxkde/FireCraft
  ```
- **Do:**
  1. Trigger `workflow_dispatch` dry-run on `.github/workflows/release.yml` and wait for every job (Linux, Windows, macOS, Web, Source, SBOM, Aggregate Gate) to complete green.
  2. Update `docs/release/ARTIFACT-MATRIX.md` with exact workflow run ID, asset filenames, byte sizes, SHA-256 digests, and honest verification rungs.
- **Verify:**
  ```bash
  gh run list --workflow release.yml --limit 3 --repo aliasfoxkde/FireCraft
  ```
- **Done when:** Multi-platform dry-run is green and `ARTIFACT-MATRIX.md` matches every artifact byte-for-byte.

---

### `L7.10` `[LUNA]` Prepare Evidence Comments for All 31 Issues & Audit PRs/Branches (`P7.6`)

- **Files:** `plan/phase7-issue-audit.md`, `plan/phase7-pr-branch-audit.md`
- **Pre-check:**
  ```bash
  gh issue list --state open --limit 50 --repo aliasfoxkde/FireCraft && gh pr list --state open --repo aliasfoxkde/FireCraft
  ```
- **Do:**
  1. Update every row of the 31 open issues with its exact commit SHA, workflow run ID, or deferred milestone decision.
  2. Post non-destructive evidence comments on proven-resolved issues (closing ONLY issues with verified commits/receipts; leave `#1` and deferred items open).
  3. Run `bash scripts/verify_release_ledger.sh`.
- **Verify:**
  ```bash
  cd /nas/Temp/repos/FireCraft && bash scripts/verify_release_ledger.sh
  ```
- **Done when:** All 31 issues have dated evidence dispositions and `verify_release_ledger.sh` exits 0.

---

### `L7.11` `[HUMAN GATE]` Maintainer Approval for Issue `#1`, PRs `#186`–`#188` & Safe Branch/Stash Cleanup (`P7.7`–`P7.8`)

- **Do:** Present exact list of:
  - Governance Issue `#1` decision
  - Superseded PRs to close (`#186` binary-polluted, `#187` superseded by `#189`) and `#188` reconciliation
  - Merged/obsolete local & remote branches, prunable worktrees, and already-salvaged stashes (`stash@{0..2}`)
  Execute ONLY the deletions explicitly approved by the maintainer.
- **Done when:** Maintainer-approved PRs/branches/stashes are cleanly closed/pruned with zero data loss.
