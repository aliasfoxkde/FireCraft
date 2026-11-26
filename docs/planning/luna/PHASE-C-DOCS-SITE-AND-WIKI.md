# Phase C — Marketing Site, Documentation, Live GitHub Wiki & OS Taskbar Launchers (`T30`)

> **Authority:** [`docs/planning/luna/README.md`](README.md) · [`docs/MARKETING-SITE-ROADMAP.md`](../../MARKETING-SITE-ROADMAP.md) · [`docs/tasks/T30-taskbar-launcher-menu.md`](../../tasks/T30-taskbar-launcher-menu.md)

---

### `L-C1` `[LUNA]` Complete Multi-Page Marketing Site (`site/`) & Workflow Checks (`P3.3` + `P3.7`)

- **Goal:** Finish the static Cloudflare Pages marketing site with dedicated pages, fresh verified CS6 Studio screenshots, multi-OS release download detection (`site/app.js`), and hardened `.github/workflows/deploy-site.yml`.
- **Pre-check:**
  ```bash
  ls -la /var/tmp/firecraft-int-1.0/site/ /var/tmp/firecraft-int-1.0/site/assets/
  ```
- **Steps:**
  1. Ensure `site/` contains self-hosted, zero-external-font pages sharing `site/styles.css` and `site/app.js`:
     - `site/index.html` (Hero with fresh CS6 Studio screenshot, Features, Download, Launch Web App CTA)
     - `site/features.html` (Honest feature matrix matching `docs/FEATURES.md`)
     - `site/download.html` (Dynamic multi-OS asset picker: Windows, macOS, Linux DEB/AppImage/Flatpak/tar.gz, Web ZIP, CLI, Source, SHA-256 verification instructions)
     - `site/roadmap.html` (Mirrors `docs/PUBLIC-ROADMAP.md` + modular extensions/WASM optimization status)
     - `site/docs.html` (Quickstart, shortcuts, tutorials, issue/feedback links)
  2. Update `site/sitemap.xml` and verify `site/_headers` strict CSP, `site/robots.txt`, and `site/app.js` hostname allowlist (`https://github.com/aliasfoxkde/FireCraft/`).
  3. Run `node --test` on `site/app.js` unit suite + Playwright responsive (`1440x900`, `390x844`) & no-JS checks + visually inspect captured PNGs of the site!
  4. Verify `.github/workflows/deploy-site.yml` validates every new page and passes `scripts/validate_workflow_pins.py`.
- **Verify:**
  ```bash
  cd /var/tmp/firecraft-int-1.0 && python3 scripts/validate_workflow_pins.py && node --test tests/*.test.js
  ```

---

### `L-C2` `[LUNA]` Deploy Isolated Cloudflare Pages Preview (`firecraft-site` `preview-v1`) (`P3.8`)

- **Goal:** Deploy `site/` to an isolated preview branch (`preview-v1`) — NEVER production `main` yet — verify live preview URL, headers, and record the rollback deployment ID.
- **Pre-check:**
  ```bash
  which wrangler || npx wrangler --version
  ```
- **Steps:**
  1. Confirm Cloudflare credentials exist in environment (if missing, log blocker and request operator token — NEVER hardcode or commit tokens!).
  2. Run `wrangler pages deploy site --project-name=firecraft-site --branch=preview-v1`.
  3. Curl preview URL to confirm HTTP 200 on `/`, `/features.html`, `/download.html`, `/roadmap.html`, `/docs.html`, valid CSP header, and zero broken asset links.
  4. Record preview URL, deployment ID, and current production rollback ID in `PROGRESS.md`.
- **Verify:**
  ```bash
  curl -fsSI "<preview-url>" | head -n 20
  ```

---

### `L-C3` `[HUMAN GATE]` Maintainer Approval for `firecraft-site` Production Promotion (`P3.9`)

- **Goal:** Present the live `preview-v1` URL, Playwright screenshots, and rollback ID to the maintainer. Do NOT promote to production `main` until approved (or defer production promotion to Phase D `L-D10` alongside `v1.0.0`).

---

### `L-C4` `[LUNA]` Reconcile `README.md`, Core Docs, `docs/wiki/` & `.github/` Templates (`P4.2`–`P4.4`, `P4.6`, `P4.8`)

- **Goal:** Make root `README.md`, `docs/`, `docs/wiki/`, and `.github/` templates 100% consistent with current verified reality (including the new CS6 Studio UI, modular `examples/`/`libraries/`/`extensions/` folders, and honest platform evidence ladder).
- **Pre-check:**
  ```bash
  ls -la /var/tmp/firecraft-int-1.0/docs/wiki/ /var/tmp/firecraft-int-1.0/.github/
  ```
- **Steps:**
  1. Refactor root `README.md` into a crisp 1.0 front page with the verified CS6 Studio screenshot, quickstart, honest feature/platform table, modular folder guide, and links to Wiki/Roadmap/Releases.
  2. Reconcile `docs/FEATURES.md`, `docs/CS6-PARITY.md`, `docs/PROGRESS.md`, `docs/HANDOFF.md`, `docs/NEXT.md`, `docs/PUBLIC-ROADMAP.md`, and `docs/README.md` — zero stale counts, zero conflicting claims.
  3. Audit and update all pages under `docs/wiki/` plus `_Sidebar.md` and `_Footer.md`.
  4. Audit and polish `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `.github/ISSUE_TEMPLATE/*.yml`, and `.github/PULL_REQUEST_TEMPLATE.md`.
  5. Run `scripts/sync-wiki.sh --check` (read-only diff against live GitHub Wiki), conflict-marker grep, link audit, and `xtask aegis`.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo run -p xtask -- aegis' && cd /var/tmp/firecraft-int-1.0 && bash scripts/sync-wiki.sh --check
  ```

---

### `L-C5` `[HUMAN GATE]` Publish Live GitHub Wiki (`P4.7`)

- **Goal:** Present the `scripts/sync-wiki.sh --check` diff to the maintainer. Only after explicit confirmation, run `bash scripts/sync-wiki.sh --publish --confirm-push` and record the synced wiki commit SHA in `PROGRESS.md`.

---

### `L-C6` `[LUNA]` OS-Native Taskbar / Dock / Desktop Launcher Integration (`T30.3`–`T30.5`)

- **Goal:** Connect the cross-platform recent/pinned model built in `L-A6` to native OS launchers while strictly preserving `unsafe_code = "forbid"`!
- **Pre-check:**
  ```bash
  cat /var/tmp/firecraft-int-1.0/docs/tasks/T30-taskbar-launcher-menu.md | head -n 45
  ```
- **Steps:**
  1. **Linux (`T30.5`):** Update `packaging/desktop/firecraft.desktop` with static `[Desktop Action NewDocument]` and `[Desktop Action OpenDocument]` using safe `Exec=firecraft --new` / `Exec=firecraft --open %F` quoting; register opened files in XDG `recently-used.xbel` via pure-Rust safe helper; validate with `desktop-file-validate`.
  2. **Windows (`T30.3`):** Add cfg-gated safe Jump List adapter (`#[cfg(target_os = "windows")]`) setting a stable AppUserModelID (`dev.firecraft.FireCraft`) and registering `New`/`Open` tasks + recent destinations using ONLY safe Rust abstractions (if the required Win32 COM crate needs `unsafe`, stop and log a decision record rather than violating `unsafe_code = "forbid"`!).
  3. **macOS (`T30.4`):** Add cfg-gated Dock menu integration seam (`#[cfg(target_os = "macos")]`) or document exact safe-abstraction boundary and native verification requirement on macOS runner.
  4. Record platform receipts per OS (`RECIPE` vs `BUILT` vs `LAUNCHED` — never claim Windows/macOS verified from Linux alone!).
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && (desktop-file-validate packaging/desktop/*.desktop 2>&1 || true)'
  ```

---

### `L-C7` `[HUMAN GATE]` Maintainer Phase C Documentation, Wiki & Launcher Review

- **Goal:** Present Phase C PR diff, link/aegis receipts, and launcher test results for maintainer approval before Phase D release engineering.
