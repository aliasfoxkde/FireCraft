# Phase 1 — PR #189 Green CI, Visual Polish Fixes & WASM Smoke

> **Goal:** Land PR `#189` cleanly on `main`, fix the 3 small visual defects spotted in the 2026-11-26 `xvfb-run` captures, and prove the new CS6 Studio UI boots and passes Playwright smoke in a real browser.
> **Branch:** `integration/firecraft-1.0` for `L1.1`; after merge, branch `feat/phase1-visual-polish` from `origin/main`.

---

### `L1.1` `[HUMAN GATE]` Verify PR `#189` Full CI Green & Merge to `main`

- **Files:** None (CI inspection; fix only if a GitHub runner fails)
- **Pre-check:**
  ```bash
  gh pr checks 189 --repo aliasfoxkde/FireCraft
  ```
- **Do:**
  1. Confirm every check on PR `#189` (`fmt`, `wasm32`, `gate`, `Windows`, `macOS`, `CodeQL`) is `pass`.
  2. If any job failed, read `gh run view <id> --log-failed`, fix the exact issue, verify on `ssh fedora`, commit, push `gitforge` then `origin`, and re-check.
  3. **STOP at `[HUMAN GATE]`:** Present PR `#189` status, final SHA, screenshots (`docs/img/app-main.png`), and `dist/windows-review/firecraft.exe` SHA-256 to the maintainer to merge into `main`.
- **Verify:**
  ```bash
  gh pr checks 189 --repo aliasfoxkde/FireCraft
  ```
- **Done when:** All checks are green and the maintainer has merged PR `#189`.

---

### `L1.2` `[LUNA]` Fix Top-Right Theme Button Clipping in Menu Bar

- **Files:** `crates/app/src/shell/chrome.rs`
- **Pre-check:**
  ```bash
  grep -n "Dark (CS6 Charcoal)" /nas/Temp/repos/FireCraft/crates/app/src/shell/chrome.rs
  ```
- **Do:**
  1. Shorten the top-right theme toggle button label in `cs6_menu_bar` to compact text (`Dark`, `Light`, `Auto`) with the full name (`Dark (CS6 Charcoal)` / `Light Studio` / `System`) in `.on_hover_text(...)`.
  2. Add 4 px right padding inside the right-aligned menu-bar cluster so the rightmost button border is fully visible at both `1440px` and `960px` widths.
  3. Keep all existing menu bar unit tests green.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** Theme toggle fits inside `960px` and `1440px` with zero clipping and full tooltip text.

---

### `L1.3` `[LUNA]` Fix Narrow-Viewport (`960x640`) Canvas Footer Overlap

- **Files:** `crates/app/src/shell/chrome.rs`
- **Pre-check:**
  ```bash
  grep -n "cs6_canvas_footer_bar" /nas/Temp/repos/FireCraft/crates/app/src/shell/chrome.rs
  ```
- **Do:**
  1. In `cs6_canvas_footer_bar`, read `let avail = ui.available_width();`.
  2. Progressively hide secondary metadata when space is tight so the left toggles (`Snap`, `Rulers`, `Guides`) NEVER collide with the right controls (state transport `1/1`, Zoom combo):
     - Hide `DPI` dropdown when `avail < 620.0`.
     - Hide `Resolution` dropdown when `avail < 540.0`.
     - Shorten `PNG32 (Document)` to `PNG32` when `avail < 680.0`.
  3. Add a unit test for the responsive footer visibility helper (`footer_visibility(width)`).
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** `960x640` capture shows zero overlapping text/buttons in the canvas footer.

---

### `L1.4` `[LUNA]` Eliminate Missing-Glyph Boxes (`□`) in Layers & Tools Footer (Interim Fix)

- **Files:** `crates/app/src/shell/panels.rs`, `crates/app/src/shell/chrome.rs`
- **Pre-check:**
  ```bash
  grep -n "🔒\|👁\|⇄\|▣\|⛶\|◧" /nas/Temp/repos/FireCraft/crates/app/src/shell/panels.rs /nas/Temp/repos/FireCraft/crates/app/src/shell/chrome.rs || true
  ```
- **Do:**
  1. Locate every non-ASCII icon glyph in the Layers action buttons (`panels.rs`) and bottom-left Tools footer (`chrome.rs` swap/default/no-fill/screen-mode buttons) that rendered as `□` in `shot-workspace.png`.
  2. Replace those glyphs with crisp painter icons (`vector_icons::draw` for now; Phase 2 replaces all chrome icons with unified Phosphor Icons) or ASCII labels where appropriate.
  3. Build Debian-glibc binary on `t5500`, capture `1440x900` and `960x640` screenshots under `xvfb-run`, and **inspect both PNGs** to confirm zero `□` boxes, zero top-right clipping, and zero footer overlap!
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- panel-snapshot && cargo run -p xtask -- aegis'
  ```
- **Done when:** Visually inspected `1440x900` and `960x640` screenshots show zero `□` boxes and zero clipped/overlapping widgets.

---

### `L1.5` `[LUNA]` Real-Browser Playwright Smoke of CS6 Studio WASM Build (`P2.10`)

- **Files:** `web/test.spec.ts` (only if assertions need updating)
- **Pre-check:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; which trunk wasm-opt chromium chromium-browser google-chrome node npx 2>&1 || true'
  ```
- **Do:**
  1. Ensure `trunk` and `wasm-opt` are available on the build host (or run inside the repo's CI container image that bakes them; if unavailable on `fedora`, install pinned `trunk` + Binaryen `wasm-opt` matching `docs/TOOLCHAINS.md`).
  2. Run `cargo run -p xtask -- dist --web`. Confirm `dist/web/` contains `.wasm`, `.js`, `index.html` AND `.wasm.gz`, `.js.gz`, `index.html.gz`.
  3. Serve `dist/web/` locally and run `npx playwright test web/test.spec.ts`, capturing `1440x900` and `960x640` browser screenshots with zero uncaught console/page errors.
  4. Visually inspect both browser screenshots and log raw `.wasm` and `.wasm.gz` byte counts in `PROGRESS.md`.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && cargo run -p xtask -- dist --web && ls -lh dist/web/'
  ```
- **Done when:** `xtask dist --web` + `web/test.spec.ts` pass with exit 0 and browser PNG screenshots look identical to desktop.
