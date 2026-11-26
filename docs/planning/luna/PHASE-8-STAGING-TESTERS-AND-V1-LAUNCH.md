# Phase 8 — Staging, External Testers & Human-Approved `v1.0.0` Launch

> **Goal:** Validate the exact release-candidate bytes on isolated staging targets, complete external tester validation and rollback rehearsal, obtain formal Go/No-Go authorization, and execute a clean, byte-verified `v1.0.0` public launch.

---

### `L8.1` `[LUNA]` Deploy Isolated Staging (`firecraft-site` + `firecraft-web` RC Bytes) & Prove Hash Match (`P9.1`)

- **Files:** `docs/planning/luna/PROGRESS.md` (staging receipts)
- **Pre-check:**
  ```bash
  test -s /nas/Temp/repos/FireCraft/docs/release/ARTIFACT-MATRIX.md
  ```
- **Do:**
  1. Deploy staging previews (`firecraft-site` branch `rc-v1`, `firecraft-web` branch `rc-v1`) from the exact RC build bytes — NEVER touch production `main`!
  2. Download the live staging `.wasm` over HTTPS and assert its `sha256sum` matches `firecraft-web-<rc>.zip` byte-for-byte.
  3. Record staging URLs, deployment IDs, and current production rollback IDs in `PROGRESS.md`.
- **Verify:**
  ```bash
  curl -fsSI "<staging-web-url>" | head -n 10
  ```
- **Done when:** Staging site and web editor are live and byte-hash-verified against the RC archive.

---

### `L8.2` `[HUMAN GATE]` External Tester Cohort Validation & P0/P1 Burn-Down (`P9.2`–`P9.4`)

- **Files:** `docs/release/V1-RC-TESTER-GUIDE.md`
- **Do:**
  1. Share `docs/release/V1-RC-TESTER-GUIDE.md`, staging URLs, and RC archives with external testers (target 5–10 testers across Windows, macOS, Linux, and browsers).
  2. Triage every report; fix every P0/P1 blocker with a unit/integration regression test and re-verify on `fedora`.
- **Done when:** Tester cohort sign-offs are recorded and zero P0/P1 blockers remain open.

---

### `L8.3` `[LUNA]` Rehearse Rollback Procedures (Cloudflare Pages & GitHub Releases, `P9.5`)

- **Files:** `docs/RELEASING.md` (update rollback runbook if any step drifted)
- **Do:**
  1. Verify exact rollback commands and recorded deployment IDs for both `firecraft-site` and `firecraft-web`.
  2. Test preview-branch rollback cleanly without touching production; document exact recovery time and steps.
- **Verify:**
  ```bash
  grep -i "rollback" /nas/Temp/repos/FireCraft/docs/RELEASING.md | head -n 10
  ```
- **Done when:** Verified rollback steps and deployment IDs are documented and tested on preview.

---

### `L8.4` `[HUMAN GATE]` Final `v1.0.0` Go / No-Go Authorization (`P9.6`)

- **Do:** Present the full readiness dossier (CI run IDs, `ARTIFACT-MATRIX.md`, `size-baseline.md`, tester cohort results, open known-limitations list, rollback IDs) to the maintainer. **STOP** until explicit Go authorization is granted.
- **Done when:** Maintainer grants formal Go authorization to tag `v1.0.0`.

---

### `L8.5` `[LUNA]` Freeze `1.0.0` Stamps, Tag `v1.0.0`, Build Draft Release & Independently Re-Hash Every Asset (`P10.1`–`P10.4`)

- **Files:** 8 version-stamped files (`xtask stamps`), `docs/CHANGELOG.md`, `docs/releases/v1.0.0.md`
- **Pre-check:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && cargo run -p xtask -- stamps'
  ```
- **Do:**
  1. Stamp `1.0.0` across all 8 version files, finalize `docs/CHANGELOG.md` and `docs/releases/v1.0.0.md`, and verify full `cargo run -p xtask -- ci` is green on `fedora`.
  2. Push annotated `v1.0.0` tag to `gitforge` then `origin` (ONLY after `L8.4` Go!).
  3. Wait for `.github/workflows/release.yml` to finish creating the draft `v1.0.0` release.
  4. Independently download EVERY draft asset to `/var/tmp/firecraft-v1-verify/`, verify `sha256sum -c RELEASE_SHA256.txt`, validate CycloneDX SBOM, and stage `site/` + `firecraft-web` from exact `v1.0.0` bytes.
- **Verify:**
  ```bash
  cd /var/tmp/firecraft-v1-verify && sha256sum -c RELEASE_SHA256.txt
  ```
- **Done when:** Every draft `v1.0.0` asset downloads cleanly and matches `RELEASE_SHA256.txt` 100%.

---

### `L8.6` `[HUMAN GATE]` Publish GitHub `v1.0.0` Release & Promote Cloudflare Pages (`P10.5`–`P10.6`)

- **Do:** Only after explicit maintainer approval on the verified draft release:
  1. Publish GitHub Release `v1.0.0` (`gh release edit v1.0.0 --draft=false`).
  2. Promote production `firecraft-site` and `firecraft-web` from the exact verified `v1.0.0` bytes (zero rebuild drift!).
- **Done when:** Public GitHub Release `v1.0.0` and both Cloudflare Pages production sites are live.

---

### `L8.7` `[LUNA]` Independent Public Post-Launch Verification & Final Launch Report (`P10.7`–`P10.9`)

- **Files:** `docs/releases/v1.0.0-launch-report.md`, `docs/planning/luna/PROGRESS.md`
- **Do:**
  1. Independently fetch live public URLs (`github.com/aliasfoxkde/FireCraft/releases/tag/v1.0.0`, `firecraft-site`, `firecraft-web.pages.dev`) and confirm live WASM SHA-256 matches `firecraft-web-1.0.0.zip`.
  2. Write `docs/releases/v1.0.0-launch-report.md` with exact version, tag, 40-char SHA, CI run IDs, Cloudflare deployment & rollback IDs, complete asset table (filename, target, size, SHA-256, signing status, verification rung, host/date), synced Wiki SHA, issue/PR/branch disposition totals, and honest known limitations.
- **Verify:**
  ```bash
  test -s /nas/Temp/repos/FireCraft/docs/releases/v1.0.0-launch-report.md
  ```
- **Done when:** Public endpoints pass hash verification and the complete Launch Report is committed.
