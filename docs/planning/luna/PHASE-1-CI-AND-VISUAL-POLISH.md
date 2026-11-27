# Phase 1 — CI Green, Live Review P0 Fixes & WASM Smoke

> **Goal:** Close every P0 defect observed during the 2026-11-26 live review ([`docs/integration/2026-11-26-live-review-defects.md`](../../integration/2026-11-26-live-review-defects.md)) on `feat/phase1-visual-polish` and merge cleanly into `main`.
> **Rule:** Never redeploy a downloaded/cached release zip. Always build fresh with `xtask dist --web` and visually inspect a screenshot of the live URL.

---

### `L1.1` `[HUMAN GATE]` Verify PR `#189` Full CI Green & Merge to `main` — **DONE (`5a5a1b2c`)**
### `L1.2` `[LUNA]` Fix Top-Right Theme Button Clipping — **DONE (`84036eda`)**
### `L1.3` `[LUNA]` Fix Narrow-Viewport (`960x640`) Canvas Footer Overlap — **DONE (`3db7e682`)**
### `L1.4` `[LUNA]` Eliminate Missing-Glyph Boxes (`□`) in Layers & Tools Footer — **DONE (`77a69bc4`)**
### `L1.5` `[LUNA]` Fix `web/src/main.rs` Studio Boot, Build Fresh WASM & Deploy Live — **DONE (`98812b17` / `c2d3596a`)**

---

### `L1.6` `[LUNA]` Replace Wrong App Icon / Logo With Approved FireCraft Flame Artwork

- **Files:** `crates/app/src/logo.png`, `web/favicon.png`, `site/assets/favicon.png`, `site/assets/firecraft-mark.png`, `crates/app/src/shell/welcome_overlay.rs`, `docs/ATTRIBUTION.md`
- **Pre-check:**
  ```bash
  ls -lh assets/branding/ crates/app/src/logo.png web/favicon.png site/assets/firecraft-mark.png
  ```
- **Do:**
  1. Replace the generic orange circle in `crates/app/src/logo.png` and `web/favicon.png` with the approved clean-room FireCraft flame icon from `assets/branding/` (verify crisp transparent PNG at 256×256 and 64×64).
  2. Remove the white square box behind the icon in `welcome_overlay.rs` so the flame blends cleanly onto dark and light modal surfaces.
  3. Capture and visually inspect desktop & web Welcome screenshots confirming the true FireCraft flame icon appears in window chrome, favicon, and Welcome modal.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo run -p xtask -- aegis'
  ```
- **Done when:** Window icon, browser favicon, and Welcome modal all display the FireCraft flame cleanly on transparent background.

---

### `L1.7` `[LUNA]` Default Theme Mode to `Auto / System` (`ThemeMode::System`)

- **Files:** `crates/app/src/shell/mod.rs`, `crates/app/src/shell/theme.rs`, `crates/app/src/shell/chrome.rs`
- **Pre-check:**
  ```bash
  grep -n "theme_mode:" crates/app/src/shell/mod.rs
  ```
- **Do:**
  1. Change initial `theme_mode` in `crates/app/src/shell/mod.rs:852` from `ThemeMode::Dark` to `ThemeMode::System`.
  2. Verify `ThemeMode::System` resolves OS/browser `prefers-color-scheme` cleanly on native desktop and WASM (falling back cleanly to Dark when unavailable).
  3. Add unit tests verifying default `ThemeMode::System` resolves both dark and light system preferences.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app theme && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** Fresh launch defaults to `Auto / System` and follows OS/browser theme preference.

---

### `L1.8` `[LUNA]` Fix Document Tab Switching Visual Active State & Stable Tab Order

- **Files:** `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/mod.rs`
- **Pre-check:**
  ```bash
  sed -n '985,1035p' crates/app/src/shell/chrome.rs
  ```
- **Do:**
  1. Replace the broken "active document always rendered at slot 0" model (`chrome.rs:993`) with a stable ordered tab sequence tracking `active_tab_index: usize` (so opening Tab 2 and clicking between Tab 1 and Tab 2 moves ONLY the active highlight and canvas, NEVER reordering tabs!).
  2. Verify middle-click close, `×` close, mouse-wheel switch, and `+` new tab preserve stable order and highlight the exact clicked tab.
  3. Add unit tests covering 3-tab create -> switch to index 2 -> switch to index 1 -> close index 1 -> verify active highlight index.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** Clicking any document tab highlights that exact tab in place without jumping to slot 0.

---

### `L1.9` `[LUNA]` Fix Canvas Z-Order Rendering Over Floating Windows, Menus & Dialogs

- **Files:** `crates/app/src/shell/canvas_ui.rs`, `crates/app/src/shell/panel_windows.rs`, `crates/app/src/shell/panels.rs`, `crates/app/src/shell/mod.rs`
- **Pre-check:**
  ```bash
  grep -rn "Order::Foreground" crates/app/src/shell/
  ```
- **Do:**
  1. Audit every `Order::Foreground` / `layer_painter` call (`canvas_ui.rs:1710`, `mod.rs:3026`, `panel_windows.rs:789`, `panels.rs:5123`) and ensure the canvas and canvas selection/ruler/guide overlays paint strictly inside the central panel's clipped `ui.painter()` (`Order::Background`/`Order::Middle`), NEVER above floating `egui::Window`s, popup menus, flyouts, or modal dialogs.
  2. Verify floating panels, menus, tooltips, and dialogs always render crisply ON TOP of the canvas and accept pointer input without canvas bleed-through.
  3. Capture and visually inspect a screenshot with a floating panel + open menu overlapping the canvas.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- panel-snapshot'
  ```
- **Done when:** Floating windows, menus, and dialogs always render above the canvas with zero Z-order bleed.

---

### `L1.10` `[LUNA]` Fix Canvas & WASM Runtime Performance Bottlenecks

- **Files:** `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/optimize.rs`, `crates/app/src/shell/mod.rs`, `crates/app/src/shell/canvas_ui.rs`
- **Pre-check:**
  ```bash
  grep -rn "optimize_preview\|compose::render" crates/app/src/shell/
  ```
- **Do:**
  1. Stop synchronous full-canvas PNG encoding on startup/every edit in the `Optimize` panel: collapse Group 4 or gate `optimize_preview` behind explicit debounce (e.g. 400 ms idle) + only run when the `Optimize` tab is actually visible & expanded!
  2. Never recompute `compose::render` when document revision/page/frame has not changed; reuse the uploaded GPU texture across pan, zoom, hover, and UI-only repaints!
  3. Profile frame time on both desktop (`1440x900`) and WASM (`chromium`) during continuous pan/zoom and drag; target `< 16 ms` UI frame time on desktop and smooth interaction in WASM.
  4. Add unit test asserting idle/pan/zoom frames incur zero redundant `compose::render` calls and zero eager `optimize_preview` encodes when Optimize is collapsed.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- dist --web'
  ```
- **Done when:** Pan, zoom, panel switching, and WASM startup feel instantaneous with zero main-thread stalls.

---

### `L1.11` `[LUNA]` Align Keybindings, Shortcuts & Focus Handling With `FireCraft-CraftAnything-Experiment`

- **Files:** `crates/app/src/keymap.rs`, `crates/app/src/shell/mod.rs`, `crates/app/src/shell/canvas_ui.rs`
- **Pre-check:**
  ```bash
  grep -rn "wants_keyboard_input\|Tab\|Key::" crates/app/src/shell/ | head -n 30
  ```
- **Do:**
  1. Compare shortcut dispatch in FireCraft against `/nas/Temp/repos/FireCraft-CraftAnything-Experiment` (read-only): single-key tool selection (`V`, `A`, `P`, `B`, `U`, `T`, `Z`, `H`, `K`, etc.), `Tab` workspace hide/restore toggle, `Ctrl+Z`/`Ctrl+Shift+Z`/`Ctrl+Y`, `Ctrl+C`/`V`/`X`/`D`, `Space` temporary Hand pan, `Esc` priority (close popup -> cancel drag -> clear selection), and arrow-key nudge.
  2. Guard single-key shortcuts cleanly when any text input (`ctx.wants_keyboard_input()`) has focus so typing never triggers tools, while ensuring clicking the canvas always restores shortcut focus.
  3. Add unit tests for every shortcut priority and text-focus guard.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** Core tool, undo/redo, clipboard, `Space` pan, `Tab` toggle, and `Esc` shortcuts work reliably on native & WASM.

---

### `L1.12` `[LUNA]` Polish Menus & Fix Right-Click Context Menus

- **Files:** `crates/app/src/menus.rs`, `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/canvas_ui.rs`, `crates/app/src/shell/panels.rs`
- **Pre-check:**
  ```bash
  grep -rn "context_menu\|secondary_clicked" crates/app/src/shell/ | head -n 30
  ```
- **Do:**
  1. Audit top-level CS6 menus and every right-click context menu (canvas empty vs selected object vs vector path node vs layer row vs document tab) against `/nas/Temp/repos/FireCraft-CraftAnything-Experiment` (read-only).
  2. Fix right-click hit-testing/selection (right-clicking an unselected object selects it before opening context menu), prevent context menus from being clipped or dismissed on release, and remove broken/redundant menu items while keeping `menus::tests` and `xtask reachability` (257/257) green.
  3. Add unit tests verifying right-click selection + action dispatch.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo run -p xtask -- reachability && cargo run -p xtask -- aegis'
  ```
- **Done when:** Top menus and right-click context menus behave crisply on canvas, layers, and tabs.

---

### `L1.13` `[LUNA]` Port Complete Styles, Layers & Missing Features From `FireCraft-CraftAnything-Experiment`

- **Files:** `crates/app/src/shell/panels.rs`, `crates/app/src/shell/chrome.rs`, `docs/integration/experiment-feature-diff.md` (new)
- **Pre-check:**
  ```bash
  ls -la /nas/Temp/repos/FireCraft-CraftAnything-Experiment/app/src/panels/ 2>&1 || true
  ```
- **Do:**
  1. Perform a file-by-file feature parity diff against the latest `/nas/Temp/repos/FireCraft-CraftAnything-Experiment` (read-only) and record every remaining missing UX feature in `docs/integration/experiment-feature-diff.md`.
  2. Upgrade **Layers panel** (`panels.rs`): visual layer/object hierarchy rows, crisp preview thumbnails, inline rename, drag-reorder, clear eye/lock toggles, blend mode dropdown, opacity scrub, and intuitive mask controls.
  3. Upgrade **Styles panel** (`panels.rs`): visual multi-column rendered style preview tiles (fill/stroke/gradient/effect swatches), search/category filter, apply/save/redefine/delete style wired through `engine`.
  4. Visually verify desktop + WASM screenshots of upgraded Layers & Styles panels and merge `feat/phase1-visual-polish` to `main` via PR with 100% green CI.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app --features native && cargo clippy --workspace --all-targets -- -D warnings && cargo run -p xtask -- panel-snapshot && cargo run -p xtask -- reachability && cargo run -p xtask -- aegis'
  ```
- **Done when:** Styles, Layers, and missing Experiment UX features are ported cleanly, visually verified, and merged to `main`.
