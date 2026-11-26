# Phase 2 — Systematic Phosphor Icons (MIT) Integration Across the Entire GUI

> **Goal:** Unify FireCraft's mixed icon rendering (`icons.rs` 57 Phosphor SVGs vs `vector_icons.rs` programmatic glyphs) onto one cohesive, razor-sharp **Phosphor Icons (MIT)** visual system across every tool, toolbar button, tab, footer control, panel header, and inspector — with full `xtask icon-survey` & `docs/ATTRIBUTION.md` compliance and visual Dark/Light/HiDPI proof.
> *(Note: Phase 4 `L4.3` converts these Phosphor SVGs into a build-time atlas/font so WASM drops `resvg` while keeping 100% authentic Phosphor Icons!)*

---

### `L2.1` `[LUNA]` Audit Complete GUI Icon Inventory vs Shipped 57 Phosphor SVGs

- **Files:** `docs/integration/phosphor-icon-map.md` (new mapping table)
- **Pre-check:**
  ```bash
  ls /nas/Temp/repos/FireCraft/assets/icons-phosphor/*.svg | wc -l && grep -n "pub enum Icon" -A 70 /nas/Temp/repos/FireCraft/crates/app/src/shell/vector_icons.rs
  ```
- **Do:**
  1. List every icon slot across `chrome.rs` (Tools rail, Main Toolbar, Header bar, View modes, Canvas footer), `panel_windows.rs` (Iconic Mode rail + panel headers), `panels.rs` (Layers actions, Align, Path, History, Styles), and `welcome_overlay.rs`.
  2. Map each slot to either:
     - An existing file in `assets/icons-phosphor/*.svg` (57 currently shipped), OR
     - The exact canonical MIT Phosphor Regular icon name to vendor in `L2.2`, OR
     - `bespoke` (only for live color swatches like active Fill/Stroke boxes).
  3. Commit the inventory table to `docs/integration/phosphor-icon-map.md`.
- **Verify:**
  ```bash
  test -s /nas/Temp/repos/FireCraft/docs/integration/phosphor-icon-map.md && wc -l /nas/Temp/repos/FireCraft/docs/integration/phosphor-icon-map.md
  ```
- **Done when:** Every UI icon slot has a single unambiguous Phosphor SVG name recorded in `phosphor-icon-map.md`.

---

### `L2.2` `[LUNA]` Vendor Missing MIT Phosphor SVGs & Register in `icons.rs` + `icon-survey`

- **Files:** `assets/icons-phosphor/*.svg`, `crates/app/src/shell/icons.rs`, `docs/ATTRIBUTION.md`, `docs/planning/generated/icon-resources-survey.md` (if generated)
- **Pre-check:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && cargo run -p xtask -- icon-survey'
  ```
- **Do:**
  1. Add ONLY the missing MIT Phosphor Regular SVGs listed in `docs/integration/phosphor-icon-map.md` (uniform 256×256 viewBox, consistent stroke weight, `currentColor` ink).
  2. Register every new SVG in `crates/app/src/shell/icons.rs::ICONS` and add a clean typed lookup enum (`UiIcon`) alongside `tool_icon(Tool)`.
  3. Update `docs/ATTRIBUTION.md` icon count/notes and regenerate/verify `xtask icon-survey`.
  4. Add a unit test in `icons.rs` asserting every `UiIcon` and every `Tool` variant resolves to a non-empty embedded Phosphor SVG!
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app icons && cargo test -p xtask iconsurvey && cargo run -p xtask -- icon-survey && cargo run -p xtask -- aegis'
  ```
- **Done when:** Zero missing Phosphor icons, `icons.rs` lookup test passes, and `xtask icon-survey` passes.

---

### `L2.3` `[LUNA]` Wire Phosphor Icons Into Left Tools Rail, Tool Flyouts & Tools Footer

- **Files:** `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/canvas_ui.rs`
- **Pre-check:**
  ```bash
  grep -n "tool_vector_icon\|icons::paint" /nas/Temp/repos/FireCraft/crates/app/src/shell/chrome.rs /nas/Temp/repos/FireCraft/crates/app/src/shell/canvas_ui.rs
  ```
- **Do:**
  1. Switch `cs6_tools_panel` in `chrome.rs` to paint every tool button with `icons::paint(ui, rect, icons::tool_icon(tool), 16.0, ink)` (matching `canvas_ui.rs::tool_flyout_ui`!).
  2. Paint the Tools footer buttons (Swap, Default Black/White, No Fill, Standard/Full Screen modes) using Phosphor icons (keeping the live Fill/Stroke color boxes as crisp colored squares).
  3. Verify active-tool highlight tint (`p.selection` / bright text) works cleanly in both Dark and Light themes.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** Left Tools rail and tool flyouts render identical crisp Phosphor icons with zero mixed styles.

---

### `L2.4` `[LUNA]` Wire Phosphor Icons Into Main Toolbar, Header Bar, Tab View Modes & Canvas Footer

- **Files:** `crates/app/src/shell/chrome.rs`
- **Pre-check:**
  ```bash
  grep -n "vector_icons::draw" /nas/Temp/repos/FireCraft/crates/app/src/shell/chrome.rs
  ```
- **Do:**
  1. Replace `vector_icons::draw` calls in `cs6_main_toolbar` (New, Open, Save, Export, Cut, Copy, Paste, Undo, Redo, Group, Ungroup, Align, Rotate, Flip), `cs6_document_tab_strip` (`Original`, `Preview`, `2-Up`, `4-Up`), and `cs6_canvas_footer_bar` (Play/Pause/Prev/Next/First/Last transport, Rulers, Guides, Snap) with `icons::paint_ui(ui, rect, UiIcon::..., size, ink)`.
  2. Ensure uniform 14–16 px optical sizing and theme-aware hover/active/disabled tinting.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** Top chrome, document tab view bar, and canvas footer use 100% Phosphor icons.

---

### `L2.5` `[LUNA]` Wire Phosphor Icons Into Docked Panels, Iconic Mode Rail, Align/Path & Welcome Modal

- **Files:** `crates/app/src/shell/panel_windows.rs`, `crates/app/src/shell/panels.rs`, `crates/app/src/shell/welcome_overlay.rs`
- **Pre-check:**
  ```bash
  grep -n "vector_icons::" /nas/Temp/repos/FireCraft/crates/app/src/shell/panel_windows.rs /nas/Temp/repos/FireCraft/crates/app/src/shell/panels.rs /nas/Temp/repos/FireCraft/crates/app/src/shell/welcome_overlay.rs
  ```
- **Do:**
  1. Switch `panel_windows.rs` Iconic Mode rail buttons and collapse/expand controls to Phosphor `UiIcon`s.
  2. Switch `panels.rs` (Layers eye/lock/add/delete/mask/share, Align panel, Path boolean ops, History, Styles) and `welcome_overlay.rs` action icons to Phosphor `UiIcon`s.
  3. Confirm `xtask panel-snapshot` and `xtask reachability` remain 100% green (`257/257`).
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /nas/Temp/repos/FireCraft-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- panel-snapshot && cargo run -p xtask -- reachability && cargo run -p xtask -- aegis'
  ```
- **Done when:** Zero mixed icon families remain across the entire GUI shell.

---

### `L2.6` `[HUMAN GATE]` Dark + Light + `960x640` Phosphor Icon Visual Sign-Off

- **Files:** `docs/img/app-main.png`, `site/assets/app-main.png` (refreshed after verification)
- **Pre-check:**
  ```bash
  which xvfb-run import
  ```
- **Do:**
  1. Build Debian-glibc binary on `t5500`, copy locally, capture `1440x900` (Dark), `1440x900` (Light), and `960x640` (Dark) screenshots under `xvfb-run`.
  2. Inspect all 3 PNGs visually: verify crisp Phosphor strokes, strong contrast on both Dark & Light backgrounds, zero clipping, zero `□` glyphs.
  3. Refresh `docs/img/app-main.png` and `site/assets/app-main.png`, commit, push `gitforge` then `origin`, and **STOP at `[HUMAN GATE]`** for maintainer visual approval!
- **Verify:**
  ```bash
  ls -lh /var/tmp/firecraft-linux-check/shot-phosphor-*.png
  ```
- **Done when:** Maintainer approves the unified Phosphor Icons GUI screenshots.
