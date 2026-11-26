# Phase D — Packaging Matrix, Issue/PR/Branch Closeout, Staging, Tester Cohort & `v1.0.0` Launch

> **Authority:** [`docs/planning/luna/README.md`](README.md) · [`docs/RELEASE-ENGINEERING-PLAN.md`](../../RELEASE-ENGINEERING-PLAN.md) · [`docs/release/ARTIFACT-MATRIX.md`](../../release/ARTIFACT-MATRIX.md)
> **Rule:** Never collapse the 9-rung verification ladder (`RECIPE` -> `BUILT` -> `INSPECTED` -> `INSTALLED` -> `LAUNCHED` -> `SMOKE TESTED` -> `SIGNED` -> `PUBLISHED` -> `DOWNLOAD-VERIFIED`). Every release claim requires an observed host, date, commit SHA, and SHA-256!

---

### `L-D1` `[HUMAN GATE]` Ratify FireCraft 1.0 Release Scope & Signing Posture (`P5.1` + `P5.9`)

- **Goal:** Reconcile `docs/decisions/0075-release-scope-mvp.md` (narrow MVP cut) and `docs/decisions/0070-rpm-deferred.md` (RPM deferred) via a superseding decision record (`docs/decisions/0130-firecraft-1.0-release-scope.md`), and confirm signing credentials, protected GitHub environments, and ARM64/RPM inclusion.
- **Pre-check:**
  ```bash
  ls -la /var/tmp/firecraft-int-1.0/docs/decisions/0070* /var/tmp/firecraft-int-1.0/docs/decisions/0075*
  ```
- **Steps:**
  1. Draft `docs/decisions/0130-firecraft-1.0-release-scope.md` explicitly listing target formats, architectures, signing mode (unsigned vs Authenticode / Developer ID / GPG), and modular folder layout (`bin/`, `examples/`, `libraries/`, `extensions/`).
  2. **STOP at `[HUMAN GATE]`:** Obtain maintainer ratification of `0130` and confirmation of available secrets/hosts before packaging.

---

### `L-D2` `[LUNA]` Standardize Canonical 1.0 Asset Naming, Manifest Schema & Aggregator Tests (`P5.2`)

- **Goal:** Ensure every asset filename, SHA-256 list, release manifest, and `scripts/validate_release_aggregate.sh` agrees on one canonical 1.0 naming convention.
- **Pre-check:**
  ```bash
  cd /var/tmp/firecraft-int-1.0 && bash scripts/validate_release_aggregate-selftest.sh && bash scripts/classify_release_mode-selftest.sh
  ```
- **Steps:**
  1. Reconcile `xtask/src/dist.rs`, `.github/workflows/release.yml`, and `scripts/validate_release_aggregate.sh` to match `0130`.
  2. Run all workflow and release self-tests (`validate_release_aggregate-selftest.sh`, `classify_release_mode-selftest.sh`, `validate_workflow_pins-selftest.sh`, `verify_release_ledger-selftest.sh`, `cargo test -p xtask`).
- **Verify:**
  ```bash
  cd /var/tmp/firecraft-int-1.0 && bash scripts/validate_release_aggregate-selftest.sh && bash scripts/classify_release_mode-selftest.sh && bash scripts/validate_workflow_pins-selftest.sh && bash scripts/verify_release_ledger-selftest.sh
  ```

---

### `L-D3` `[LUNA]` Harden & Verify Linux Packaging (DEB, tar.gz, AppImage, Flatpak, Conditional RPM) (`P5.3`–`P5.6`)

- **Goal:** Build, inspect, and clean-host smoke-test every approved Linux package carrying the modular folder layout.
- **Pre-check:**
  ```bash
  head -n 80 /var/tmp/firecraft-int-1.0/packaging/build-appimage.sh /var/tmp/firecraft-int-1.0/packaging/flatpak/dev.firecraft.FireCraft.yml
  ```
- **Steps:**
  1. Verify `xtask dist --native` (`tar.gz`, `cli.tar.gz`, `.deb`) on `ssh fedora`, checking `dpkg-deb -I`, reinstall/remove/purge, desktop icon, and glibc floor.
  2. Verify `packaging/build-appimage.sh` respects `CARGO_TARGET_DIR`, declares `Apache-2.0` license, uses pinned `linuxdeploy`, bundles `examples/`, `libraries/`, `extensions/`, and launches cleanly under `xvfb-run`.
  3. Update `packaging/flatpak/dev.firecraft.FireCraft.yml` to use an immutable tagged source archive (not `type: dir, path: ..`) and document/execute sandbox smoke.
  4. Build & test RPM on `ssh fedora` if approved in `0130`.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo run -p xtask -- dist --native && ls -lh dist/native/'
  ```

---

### `L-D4` `[LUNA]` Harden Windows ZIP + Inno Setup, macOS Universal DMG, Web ZIP, Source & CycloneDX SBOM (`P5.7`, `P5.8`, `P5.10`, `P5.11`)

- **Goal:** Ensure Windows (`packaging/build-windows.ps1`), macOS (`packaging/build-macos-dmg.sh`), Web ZIP (`firecraft-web-<v>.zip`), Source archive, CycloneDX 1.5 SBOM, and provenance all pass fail-closed checks on real runners.
- **Pre-check:**
  ```bash
  head -n 80 /var/tmp/firecraft-int-1.0/packaging/build-windows.ps1 /var/tmp/firecraft-int-1.0/packaging/build-macos-dmg.sh
  ```
- **Steps:**
  1. Add `-RequireInstaller` fail-closed mode to `packaging/build-windows.ps1` (never silently skip `ISCC.exe` in release mode) and verify it bundles `examples/`, `libraries/`, `extensions/`.
  2. Add proper `.icns` icon generation + `CFBundleIconFile` + optional env-driven Developer ID / notarization hook in `packaging/build-macos-dmg.sh`.
  3. Verify `firecraft-web-<v>.zip` extracts cleanly into a standalone static directory (including `examples/`, `libraries/`, `extensions/`, `modules/`, `.gz`, `.br`) and passes Playwright E2E.
  4. Verify clean-extract Source archive builds offline/locked, CycloneDX SBOM validates against the vendored JSON schema, and provenance attestations verify.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p xtask'
  ```

---

### `L-D5` `[LUNA]` Execute Multi-Platform Dry-Run Rehearsal & Update `ARTIFACT-MATRIX.md` (`P5.12`)

- **Goal:** Run a `workflow_dispatch` dry-run release across Linux, Windows, macOS, and Web runners; verify the draft rehearsal assets and update `docs/release/ARTIFACT-MATRIX.md` with exact observed run IDs, sizes, digests, and rungs.
- **Pre-check:**
  ```bash
  gh workflow list --repo aliasfoxkde/FireCraft
  ```
- **Verify:**
  ```bash
  gh run list --workflow release.yml --limit 5 --repo aliasfoxkde/FireCraft
  ```

---

### `L-D6` `[HUMAN GATE]` Issue, PR & Branch Closeout (31 Issues, PRs `#186`–`#188`, Branches & Stashes) (`P7.6`–`P7.8`)

- **Goal:** Reconcile every open GitHub issue, superseded PR, local/remote branch, worktree, and stash cleanly with zero data loss and full evidence receipts.
- **Pre-check:**
  ```bash
  gh issue list --state open --limit 50 --repo aliasfoxkde/FireCraft && gh pr list --state open --repo aliasfoxkde/FireCraft
  ```
- **Steps:**
  1. Reconcile `plan/phase7-issue-audit.md` and `plan/phase7-pr-branch-audit.md` against current verified commits.
  2. Post non-destructive evidence comments on every proven-resolved or superseded issue (`#3`, `#23`, `#24`, `#26`, `#27`, `#31`, `#36`, `#42`, `#45`, `#48`–`#66`, `#96`) referencing exact commits/run IDs.
  3. Run `bash scripts/verify_release_ledger.sh`.
  4. **STOP at `[HUMAN GATE]`:** Obtain explicit maintainer approval for:
     - Governance Issue `#1` (`artcraft-candidates` incubation decision).
     - Closing superseded PRs `#186` (polluted binaries) and `#187` (superseded by `#189`) with replacement links, and reconciling/merging `#188`.
     - Deleting approved merged/obsolete local/remote branches, prunable worktrees, and already-salvaged stashes (`stash@{0..2}`).
- **Verify:**
  ```bash
  cd /var/tmp/firecraft-int-1.0 && bash scripts/verify_release_ledger.sh
  ```

---

### `L-D7` `[LUNA]` Staging Deployment, External Tester Cohort & Rollback Rehearsal (`P9.1`–`P9.5`)

- **Goal:** Deploy RC bytes to isolated staging targets (`firecraft-site` preview + `firecraft-web` staging), verify byte-exact SHA-256 match between staging WASM and `firecraft-web-<rc>.zip`, run external tester cohort using `docs/release/V1-RC-TESTER-GUIDE.md`, fix every P0/P1 blocker with regression tests, and rehearse Cloudflare Pages + GitHub Release rollback procedures.
- **Verify:**
  ```bash
  test -s /var/tmp/firecraft-int-1.0/docs/release/V1-RC-TESTER-GUIDE.md
  ```

---

### `L-D8` `[HUMAN GATE]` Final `v1.0.0` Go / No-Go Gate (`P9.6`)

- **Goal:** Present full CI receipts, `docs/release/ARTIFACT-MATRIX.md`, size optimization report, tester cohort summary, zero open P0/P1 blockers, and rollback IDs for formal maintainer Go/No-Go authorization.

---

### `L-D9` `[LUNA]` Freeze `1.0.0` Stamps, Cut Tag, Build Draft Release & Re-Hash Every Asset (`P10.1`–`P10.4`)

- **Goal:** Stamp `1.0.0` across all 8 version stamps (`xtask stamps`), finalize `docs/CHANGELOG.md` and `docs/releases/v1.0.0.md`, pass full `xtask ci`, push annotated `v1.0.0` tag (after `L-D8` approval), wait for `release.yml` draft completion, independently download and verify `sha256sum` on EVERY draft asset + CycloneDX SBOM + provenance, and stage `site/` + `firecraft-web` against exact `v1.0.0` bytes.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo run -p xtask -- ci'
  ```

---

### `L-D10` `[HUMAN GATE]` Publish GitHub `v1.0.0` Release, Promote Cloudflare Pages & Emit Final Launch Report (`P10.5`–`P10.9`)

- **Goal:** Only after explicit maintainer sign-off on the verified draft release:
  1. Publish GitHub `v1.0.0` release (`--draft=false`).
  2. Promote production `firecraft-site` and `firecraft-web` from the exact verified `v1.0.0` bytes (zero rebuild drift!).
  3. Run independent public HTTP + hash post-launch verification against live URLs.
  4. Emit the complete Launch Report (commit SHA, tag, GitHub Release URL, CI run IDs, Cloudflare deployment & rollback IDs, full asset table with SHA-256 & verification rungs, wiki SHA, issue/PR/branch disposition totals, and honest known limitations).
