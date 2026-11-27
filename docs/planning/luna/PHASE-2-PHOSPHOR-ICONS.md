# Phase 2 — Systematic Phosphor Icons (MIT) Unification Across the Entire GUI

> **Goal:** Unify FireCraft's split-brain icon rendering (`icons.rs` 57 runtime-parsed Phosphor SVGs via `resvg` vs `vector_icons.rs` 2,157 lines of programmatic glyphs) onto **one cohesive, razor-sharp MIT Phosphor Icons system** across every tool, toolbar button, tab, footer control, panel header, and inspector — with full `xtask icon-survey` & `docs/ATTRIBUTION.md` compliance and visual Dark/Light/HiDPI proof.
> **Proven Architecture From Experiment `v0.3.0` (`768a83d`):** Experiment embedded **`Phosphor-Regular.ttf`** (488 KB MIT icon font, `app/assets/fonts/Phosphor-Regular.ttf`) directly into `egui::FontDefinitions` via `install_phosphor_font(&ctx)` and mapped 60+ `Icon` variants to Unicode codepoints in a 65-line file (`app/src/icons/phosphor.rs`). Adopting this font-based Phosphor approach gives 100% authentic Phosphor Icons at infinite HiDPI sharpness on **both native AND WASM** while allowing Phase 4 (`L4.3`) to drop the heavy runtime `resvg`/`usvg`/`fontdb` stack!

---

### `L2.1` `[LUNA]` Audit Complete GUI Icon Inventory & Phosphor Codepoint Map

- **Files:** `docs/integration/phosphor-icon-map.md` (new mapping table)
- **Experiment Ref (read-only):** `app/src/icons/phosphor.rs`, `app/src/icons.rs:1-190`
- **Pre-check:**
  ```bash
  ls assets/icons-phosphor/*.svg | wc -l && grep -n "pub enum Icon" -A 70 crates/app/src/shell/vector_icons.rs
  ```
- **Do:**
  1. List every icon slot across `chrome.rs` (Tools rail, Main Toolbar, Header bar, View modes, Canvas footer), `panel_windows.rs` (Iconic Mode rail + panel headers), `panels.rs` (Layers eye/lock/folder/slice/actions, Align, Path, History, Styles), and `welcome_overlay.rs`.
  2. Map each slot to its canonical MIT **Phosphor Regular** codepoint (referencing Experiment's `app/src/icons/phosphor.rs`) OR `bespoke-diagram` (for specialized CS6 Align/Path schematic diagrams and live Fill/Stroke color boxes).
  3. Commit the inventory table to `docs/integration/phosphor-icon-map.md`.
- **Verify:**
  ```bash
  test -s docs/integration/phosphor-icon-map.md && wc -l docs/integration/phosphor-icon-map.md
  ```
- **Done when:** Every UI icon slot has a single unambiguous Phosphor Regular glyph or diagram assignment in `phosphor-icon-map.md`.

---

### `L2.2` `[LUNA]` Embed MIT `Phosphor-Regular.ttf` Font, Add `phosphor.rs` Map & Update Attribution / `icon-survey`

- **Files:** `assets/fonts/Phosphor-Regular.ttf`, `assets/fonts/LICENSE-phosphor-mit.txt`, `crates/app/src/shell/icons.rs`, `crates/app/src/shell/phosphor.rs` (new), `crates/app/src/main.rs`, `web/src/main.rs`, `docs/ATTRIBUTION.md`
- **Experiment Ref (read-only):** `app/assets/fonts/{Phosphor-Regular.ttf,LICENSE-MIT}`, `app/src/icons.rs:7-21`, `app/src/icons/phosphor.rs`, `app/src/icons/tests.rs`
- **Pre-check:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && cargo run -p xtask -- icon-survey'
  ```
- **Do:**
  1. Bundle MIT `Phosphor-Regular.ttf` + `LICENSE-phosphor-mit.txt` under `assets/fonts/` and register `"FireCraft Phosphor"` in `egui::FontDefinitions` via `icons::install_phosphor_font(&ctx)` at both native (`crates/app/src/main.rs`) and WASM (`web/src/main.rs`) startup (plus `Shell` tests).
  2. Create `crates/app/src/shell/phosphor.rs` mapping every `UiIcon` and `Tool` variant to its verified Phosphor Regular codepoint, with crisp `icons::paint_phosphor(painter, rect, icon, size, color)`.
  3. Update `docs/ATTRIBUTION.md` and keep `xtask icon-survey` passing cleanly.
  4. Add unit tests verifying every `Tool` and `UiIcon` resolves to a valid non-empty Phosphor glyph (matching Experiment `app/src/icons/tests.rs`).
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app icons && cargo test -p xtask iconsurvey && cargo run -p xtask -- icon-survey && cargo check --target wasm32-unknown-unknown -p firecraft-web && cargo run -p xtask -- aegis'
  ```
- **Done when:** Embedded Phosphor font installs cleanly on desktop & WASM, all lookup tests pass, and `xtask icon-survey` + `aegis` are green.

---

### `L2.3` `[LUNA]` Wire Phosphor Icons Into Left Tools Rail, Tool Flyouts & Tools Footer

- **Files:** `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/canvas_ui.rs`
- **Pre-check:**
  ```bash
  grep -n "tool_vector_icon\|icons::paint" crates/app/src/shell/chrome.rs crates/app/src/shell/canvas_ui.rs
  ```
- **Do:**
  1. Switch `cs6_tools_panel` in `chrome.rs` AND `tool_flyout_ui` in `canvas_ui.rs` to paint every tool button using unified Phosphor glyphs at uniform optical sizing.
  2. Paint the Tools footer buttons (Swap, Default Black/White, No Fill, Full Screen modes) using Phosphor icons (replacing any ASCII labels while keeping live Fill/Stroke color boxes as crisp colored squares).
  3. Verify active-tool highlight tint works cleanly in both Dark and Light themes.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** Left Tools rail, tool flyouts, and Tools footer render unified crisp Phosphor icons with zero mixed styles.

---

### `L2.4` `[LUNA]` Wire Phosphor Icons Into Main Toolbar, Header Bar, Tab View Modes & Canvas Footer

- **Files:** `crates/app/src/shell/chrome.rs`
- **Pre-check:**
  ```bash
  grep -n "vector_icons::draw" crates/app/src/shell/chrome.rs
  ```
- **Do:**
  1. Replace `vector_icons::draw` calls in `cs6_menu_bar` (Folder/Hand/Zoom), `cs6_main_toolbar` (New, Open, Save, Import, Export, Cut, Copy, Paste, Duplicate, Undo, Redo, Group, Ungroup, Bring/Send 4-way, Rotate, Flip), `cs6_document_tab_strip` (`Original`, `Preview`, `2-Up`, `4-Up`), and `cs6_canvas_footer_bar` (StateFirst/Prev/Play/Pause/Next/Last transport) with unified Phosphor glyphs.
  2. Ensure uniform 14–16 px optical sizing and theme-aware hover/active/disabled tinting.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** Top chrome, document tab view bar, and canvas footer use 100% Phosphor icons.

---

### `L2.5` `[LUNA]` Wire Phosphor Icons Into Docked Panels, Iconic Mode Rail, Layers/Align/Path & Welcome Modal

- **Files:** `crates/app/src/shell/panel_windows.rs`, `crates/app/src/shell/panels.rs`, `crates/app/src/shell/welcome_overlay.rs`
- **Pre-check:**
  ```bash
  grep -n "vector_icons::" crates/app/src/shell/panel_windows.rs crates/app/src/shell/panels.rs crates/app/src/shell/welcome_overlay.rs
  ```
- **Do:**
  1. Switch `panel_windows.rs` Iconic Mode rail buttons and collapse/expand controls to Phosphor glyphs.
  2. Switch `panels.rs` (Layers eye/lock/folder/slice/duplicate/trash, History, Styles, Color Palette icons) and `welcome_overlay.rs` action icons to Phosphor glyphs; keep specialized geometric Align/Path schematic icons sharp and consistent in line weight.
  3. Confirm `xtask panel-snapshot` and `xtask reachability` remain 100% green (`257/257`).
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- panel-snapshot && cargo run -p xtask -- reachability && cargo run -p xtask -- aegis'
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
  1. Build Debian-glibc binary on `t5500`, copy locally, capture `1440x900` (Dark), `1440x900` (Light), and `960x640` (Dark) screenshots under `xvfb-run` + rebuild fresh WASM (`xtask dist --web`) and capture live browser screenshot.
  2. Inspect all PNGs visually: verify crisp Phosphor strokes, strong contrast on both Dark & Light backgrounds, zero clipping, zero `□` glyphs.
  3. Refresh `docs/img/app-main.png` and `site/assets/app-main.png`, commit, push `gitforge` then `origin`, and **STOP at `[HUMAN GATE]`** for maintainer visual approval!
- **Verify:**
  ```bash
  ls -lh /var/tmp/firecraft-linux-check/shot-phosphor-*.png
  ```
- **Done when:** Maintainer approves the unified Phosphor Icons GUI screenshots.
