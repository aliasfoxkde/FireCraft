# Phase 6 — Marketing Site, Documentation, Live GitHub Wiki & OS Taskbar Launchers (`T30`)

> **Goal:** Complete the multi-page static Cloudflare Pages marketing site (`site/`), reconcile root `README.md` + `docs/` + `docs/wiki/` + `.github/` templates, sync the live GitHub Wiki, and wire OS-native taskbar/Dock launchers cleanly (1 OS per atomic card!).

---

### `L6.1` `[LUNA]` Build Multi-Page Static Marketing Site (`site/*.html`, `P3.3`)

- **Files:** `site/index.html`, `site/features.html`, `site/download.html`, `site/roadmap.html`, `site/docs.html`, `site/sitemap.xml`
- **Pre-check:**
  ```bash
  ls -la /nas/Temp/repos/FireCraft/site/ /nas/Temp/repos/FireCraft/site/assets/
  ```
- **Do:**
  1. Cherry-pick/integrate `f9b69f54` (`site/extensions.html`, extension program landing page) from `feat/experiment-merge` cleanly so zero site work is lost.
  2. Add `features.html`, `download.html`, `roadmap.html`, and `docs.html` sharing `site/styles.css`, `site/app.js`, and self-hosted local assets (`./assets/app-main.png`, `./assets/firecraft-mark.png` — ZERO external Google Fonts or hotlinks!).
  3. Keep every feature and platform claim strictly aligned with `docs/FEATURES.md` and `docs/release/ARTIFACT-MATRIX.md`.
  4. Update `site/sitemap.xml` and verify cross-page navigation links work without JS.
- **Verify:**
  ```bash
  cd /nas/Temp/repos/FireCraft && for f in index features download roadmap docs; do test -s "site/${f}.html" || exit 1; done && grep -rn "fonts.googleapis.com\|raw.githubusercontent.com" site/ && exit 1 || true
  ```
- **Done when:** All 5 pages exist, are self-hosted, and have zero external font/image leaks.

---

### `L6.2` `[LUNA]` Harden `site/app.js`, Playwright Site Audit & `deploy-site.yml` (`P3.7`)

- **Files:** `site/app.js`, `tests/*.test.js`, `.github/workflows/deploy-site.yml`
- **Pre-check:**
  ```bash
  ls -la /nas/Temp/repos/FireCraft/tests/ && head -n 45 /nas/Temp/repos/FireCraft/.github/workflows/deploy-site.yml
  ```
- **Do:**
  1. Ensure `site/app.js` recognizes Windows, macOS, Linux DEB/AppImage/Flatpak/tar.gz, Web ZIP, CLI, Source, and SHA-256 assets and strictly validates download URLs against `https://github.com/aliasfoxkde/FireCraft/`.
  2. Update `.github/workflows/deploy-site.yml` to validate all 5 HTML pages, `site/_headers`, and `site/sitemap.xml`, keeping every GitHub Action SHA-pinned (`scripts/validate_workflow_pins.py`).
  3. Run `node --test` + Playwright responsive (`1440x900`, `390x844`) & no-JS checks, and visually inspect captured site PNGs!
- **Verify:**
  ```bash
  cd /nas/Temp/repos/FireCraft && python3 scripts/validate_workflow_pins.py && node --test tests/*.test.js
  ```
- **Done when:** Unit tests, pin checks, and Playwright responsive screenshots all pass.

---

### `L6.3` `[LUNA]` Deploy Isolated Cloudflare Pages Preview (`firecraft-site` `preview-v1`, `P3.8`)

- **Files:** `docs/planning/luna/PROGRESS.md` (receipt only)
- **Pre-check:**
  ```bash
  which wrangler || npx wrangler --version
  ```
- **Do:**
  1. Confirm Cloudflare credentials exist in environment (NEVER commit/hardcode tokens; stop and escalate if absent).
  2. Deploy preview only: `wrangler pages deploy site --project-name=firecraft-site --branch=preview-v1`.
  3. Verify HTTP 200 and headers on all 5 pages (`curl -fsSI "<preview-url>" | head -n 20`) and record preview URL, deployment ID, and current production rollback ID in `PROGRESS.md`.
- **Verify:**
  ```bash
  curl -fsSI "<preview-url>" | head -n 20
  ```
- **Done when:** Isolated preview URL serves all 5 pages cleanly and rollback ID is recorded.

---

### `L6.4` `[HUMAN GATE]` Maintainer Approval for `firecraft-site` Production Promotion (`P3.9`)

- **Do:** Present preview URL, Playwright screenshots, and rollback ID to the maintainer (production promotion may wait for Phase 8 `L8.6` so it launches alongside `v1.0.0`).
- **Done when:** Maintainer records promotion decision.

---

### `L6.5` `[LUNA]` Refactor Root `README.md` & Reconcile Core Docs (`P4.2`–`P4.3`)

- **Files:** `README.md`, `docs/README.md`, `docs/FEATURES.md`, `docs/CS6-PARITY.md`, `docs/PROGRESS.md`, `docs/HANDOFF.md`, `docs/NEXT.md`, `docs/PUBLIC-ROADMAP.md`
- **Pre-check:**
  ```bash
  head -n 40 /nas/Temp/repos/FireCraft/README.md
  ```
- **Do:**
  1. Make `README.md` a crisp 1.0 front page showcasing the verified CS6 Studio screenshot (`docs/img/app-main.png`), quickstart, honest feature/platform table, modular folder guide (`examples/`, `libraries/`, `extensions/`), and Wiki/Releases links.
  2. Reconcile command counts, panel counts, Phosphor icon status, WASM size baseline, and release status across the 7 core docs so zero contradictions remain.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo run -p xtask -- aegis'
  ```
- **Done when:** Zero conflicting counts or broken internal links remain across `README.md` and core docs.

---

### `L6.6` `[LUNA]` Update `docs/wiki/` Pages & `.github/` Contributor Templates (`P4.4`, `P4.6`, `P4.8`)

- **Files:** `docs/wiki/*.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `.github/ISSUE_TEMPLATE/*`, `.github/PULL_REQUEST_TEMPLATE.md`
- **Pre-check:**
  ```bash
  ls -la /nas/Temp/repos/FireCraft/docs/wiki/ /nas/Temp/repos/FireCraft/.github/
  ```
- **Do:**
  1. Audit and update every page in `docs/wiki/` (including `_Sidebar.md`, `_Footer.md`, `Tutorials.md`, `Distribution-and-SDK.md`) to match current UI, Phosphor icons, modular folders, and honest release status.
  2. Polish `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, and `.github/` issue/PR templates.
  3. Run read-only `bash scripts/sync-wiki.sh --check` and conflict-marker check.
- **Verify:**
  ```bash
  cd /nas/Temp/repos/FireCraft && bash scripts/sync-wiki.sh --check && ! grep -rn "^<<<<<<< \|^=======$\|^>>>>>>> " docs/ .github/
  ```
- **Done when:** Wiki diff is clean and ready for human publish approval, with `xtask aegis` green.

---

### `L6.7` `[HUMAN GATE]` Publish Live GitHub Wiki (`P4.7`)

- **Do:** Present `scripts/sync-wiki.sh --check` diff to the maintainer. Only after explicit confirmation, run `bash scripts/sync-wiki.sh --publish --confirm-push` and record the live wiki commit SHA in `PROGRESS.md`.
- **Done when:** Live GitHub Wiki matches `docs/wiki/` with recorded SHA.

---

### `L6.8` `[LUNA]` Linux Desktop Launcher Actions & Recent Documents (`T30.5`)

- **Files:** `packaging/desktop/firecraft.desktop`, `crates/app/src/recent.rs` (XDG `recently-used.xbel` safe writer)
- **Pre-check:**
  ```bash
  cat /nas/Temp/repos/FireCraft/packaging/desktop/firecraft.desktop
  ```
- **Do:**
  1. Add static `[Desktop Action NewDocument]` (`Exec=firecraft --new`) and `[Desktop Action OpenDocument]` (`Exec=firecraft --open %F`) to `packaging/desktop/firecraft.desktop` with valid quoting.
  2. Add a pure-Rust safe helper (zero `unsafe`) to register opened `.fw.png` files into `$XDG_DATA_HOME/recently-used.xbel` when enabled.
  3. Validate `.desktop` syntax (`desktop-file-validate`) and add unit tests for XML escaping and missing-dir handling.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app recent && (desktop-file-validate packaging/desktop/*.desktop 2>&1 || true)'
  ```
- **Done when:** Desktop actions validate cleanly and XDG recent-file unit tests pass.

---

### `L6.9` `[LUNA]` Windows Jump List Safe Adapter Investigation / Implementation (`T30.3`)

- **Files:** `crates/app/src/shell/windows_launcher.rs` (`#[cfg(target_os = "windows")]`) or `docs/decisions/0132-windows-jump-list-safety.md`
- **Pre-check:**
  ```bash
  grep -n "unsafe_code" /nas/Temp/repos/FireCraft/Cargo.toml
  ```
- **Do:**
  1. Verify whether a safe Rust crate exposes Windows AppUserModelID (`dev.firecraft.FireCraft`) + Jump List static tasks (`New`, `Open`) without violating workspace `unsafe_code = "forbid"`.
  2. If a safe abstraction exists, implement behind `#[cfg(target_os = "windows")]` and verify compilation on `ssh t5500` (`x86_64-pc-windows-gnu`) and Windows MSVC CI; if COM requires `unsafe`, record an explicit decision doc (`0132`) rather than weakening `unsafe_code = "forbid"`.
  3. Record exact status (`BUILT` only until verified natively on Windows 11!).
- **Verify:**
  ```bash
  ssh t5500 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-t5500; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo check --target x86_64-pc-windows-gnu -p app'
  ```
- **Done when:** Safe Windows Jump List compiles cleanly (or explicit `0132` policy decision is recorded).

---

### `L6.10` `[LUNA]` macOS Dock Menu Seam & Platform Receipt Update (`T30.4`)

- **Files:** `crates/app/src/shell/macos_launcher.rs` (`#[cfg(target_os = "macos")]`) or `docs/tasks/T30-taskbar-launcher-menu.md`
- **Pre-check:**
  ```bash
  grep -n "T30.4" -A 8 /nas/Temp/repos/FireCraft/docs/tasks/T30-taskbar-launcher-menu.md
  ```
- **Do:**
  1. Wire a clean cfg-gated macOS Dock menu seam (if safely achievable under `unsafe_code = "forbid"`), or document the exact safe AppKit delegate requirement and keep `T30.4` marked honestly in `docs/TASKS.md` until macOS runner verification.
  2. Ensure `CI (cross-platform)/macOS` passes green after any change.
- **Verify:**
  ```bash
  gh pr checks --repo aliasfoxkde/FireCraft || true
  ```
- **Done when:** macOS CI is green and `T30` status matches real platform receipts.
