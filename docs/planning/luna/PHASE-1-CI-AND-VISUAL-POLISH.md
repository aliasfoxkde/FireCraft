# Phase 1 — CI Green, Live Review P0 Fixes & Core Experiment Parity

> **Goal:** Close every P0 defect observed during the 2026-11-26 live review ([`2026-11-26-live-review-defects.md`](../../integration/2026-11-26-live-review-defects.md)) and core workspace gaps from the 14-domain Experiment `v0.3.0` audit ([`2026-11-26-experiment-deep-audit.md`](../../integration/2026-11-26-experiment-deep-audit.md)) on `feat/phase1-visual-polish`, then merge cleanly into `main`.
> **Rule:** Never redeploy a downloaded/cached release zip. Always build fresh with `xtask dist --web` and visually inspect a screenshot of the live URL. Read `/nas/Temp/repos/FireCraft-CraftAnything-Experiment` strictly **read-only**.

---

### `L1.1` `[HUMAN GATE]` Verify PR `#189` Full CI Green & Merge to `main` — **DONE (`5a5a1b2c`)**
### `L1.2` `[LUNA]` Fix Top-Right Theme Button Clipping — **DONE (`84036eda`)**
### `L1.3` `[LUNA]` Fix Narrow-Viewport (`960x640`) Canvas Footer Overlap — **DONE (`3db7e682`)**
### `L1.4` `[LUNA]` Eliminate Missing-Glyph Boxes (`□`) in Layers & Tools Footer — **DONE (`77a69bc4`)**
### `L1.5` `[NEON]` Fix `web/src/main.rs` Studio Boot, Build Fresh WASM & Deploy Live — **DONE (`98812b17` / `c2d3596a`)**

---

### `L1.6` `[LUNA]` Replace Wrong App Icon / Logo With Approved FireCraft Flame Artwork

- **Files:** `crates/app/src/logo.png`, `web/favicon.png`, `site/assets/favicon.png`, `site/assets/firecraft-mark.png`, `crates/app/src/shell/welcome_overlay.rs`, `docs/ATTRIBUTION.md`
- **Experiment Ref (read-only):** `app/assets/{logo.svg,logo.png,firecraft.ico}`, `app/favicon.png`, `app/src/main.rs:27-36`
- **Pre-check:**
  ```bash
  ls -lh crates/app/src/logo.png web/favicon.png site/assets/firecraft-mark.png
  ```
- **Do:**
  1. Replace the generic orange circle in `crates/app/src/logo.png`, `web/favicon.png`, and `site/assets/{favicon,firecraft-mark}.png` with the crisp dark-rounded-square FireCraft Flame artwork matching Experiment `v0.3.0`'s clean-room `app/assets/logo.svg` / `logo.png` (and store `logo.svg` / `.ico` for packaging).
  2. Remove the white square backing box in `welcome_overlay.rs` so the flame emblem blends seamlessly on both Dark and Light surfaces.
  3. Capture and visually inspect desktop & web Welcome screenshots confirming the true FireCraft flame icon appears in OS window chrome, browser favicon, and Welcome modal.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo run -p xtask -- aegis'
  ```
- **Done when:** Window icon, browser favicon, and Welcome modal all display the crisp FireCraft flame mark.

---

### `L1.7` `[LUNA]` Default Theme Mode to `Auto / System` + WCAG AA Contrast Tests

- **Files:** `crates/app/src/shell/mod.rs`, `crates/app/src/shell/theme.rs`, `crates/app/src/shell/chrome.rs`
- **Experiment Ref (read-only):** `app/src/theme.rs:115-320`
- **Pre-check:**
  ```bash
  grep -n "theme_mode:" crates/app/src/shell/mod.rs
  ```
- **Do:**
  1. Change initial `theme_mode` in `crates/app/src/shell/mod.rs:852` from `ThemeMode::Dark` to `ThemeMode::System`.
  2. Cache native OS light/dark lookup via `OnceLock<bool>` (using `CREATE_NO_WINDOW` on Windows, matching Experiment `theme.rs:116-150`, and browser `prefers-color-scheme` on WASM) so switching to System never freezes or flashes a console.
  3. Port Experiment's WCAG 2.1 `relative_luminance` / `contrast_ratio` unit tests (`theme.rs:259-320`) asserting `>= 4.5:1` body/accent contrast and `>= 3.0:1` dim text contrast on both Dark and Light palettes.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app theme && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** Fresh launch defaults to `Auto / System`, detects OS/browser theme cleanly, and passes WCAG AA contrast tests.

---

### `L1.8` `[LUNA]` Fix Document Tab Switching, Stable Order, Drag-to-Swap, MRU Overflow & Tab Context Menu

- **Files:** `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/mod.rs`
- **Experiment Ref (read-only):** `app/src/layout/chrome.rs:9-443`
- **Pre-check:**
  ```bash
  sed -n '970,1060p' crates/app/src/shell/chrome.rs
  ```
- **Do:**
  1. Replace the broken slot-0 active-tab swap model (`chrome.rs:993`) with a stable ordered tab list + `active_doc: usize` so clicking Tab 2 moves ONLY the active highlight and active canvas without reordering tabs.
  2. Port `tab_click_without_drag` (8 px pointer slop so slight pointer movement during a click never drops a tab switch) and raw `Event::MouseWheel` 1-notch-per-tab stepping (replacing the broken `0`/`len-1` jump at `chrome.rs:981`).
  3. Add live drag-to-swap tab reordering with translucent ghost preview (`Order::Tooltip`), MRU `…` overflow dropdown (`Open Documents` + `Close All Tabs`), and the full tab right-click menu (`Close Tab`, `Close Other Tabs`, `Close Tabs to the Left`, `Close Tabs to the Right`, `Close All Tabs`, `Move Tab Left`, `Move Tab Right`).
  4. Add unit tests covering 3-tab create -> switch to index 2 -> switch to index 1 -> swap -> close index 0 -> verify active highlight index remains stable.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** Tabs stay in stable order, highlight the exact active tab in place, support drag-to-swap + MRU overflow + bulk close menu.

---

### `L1.9` `[LUNA]` Fix Canvas Clipping & Z-Order Over Floating Windows, Chrome, Menus & Dialogs

- **Files:** `crates/app/src/shell/canvas_ui.rs`, `crates/app/src/shell/panel_windows.rs`, `crates/app/src/shell/panels.rs`, `crates/app/src/shell/mod.rs`
- **Experiment Ref (read-only):** `app/src/canvas.rs:1087-1204`, `app/src/layout.rs:326-423`
- **Pre-check:**
  ```bash
  sed -n '615,645p' crates/app/src/shell/canvas_ui.rs && grep -rn "Order::Foreground" crates/app/src/shell/
  ```
- **Do:**
  1. Wrap ALL canvas painting in `canvas_ui.rs:617-780` (backdrop, page shadow, `image_rect`, selection box, handles, marching ants, guides, 9-slice overlays) inside `let painter = ui.painter().with_clip_rect(rect);` (matching Experiment `canvas.rs:1094`) so zoomed/scrolled canvases NEVER bleed over the Document Tab Strip, Rulers, Canvas Footer, or Docks!
  2. Audit every `Order::Foreground` site (`canvas_ui.rs:1710`, `mod.rs:3026`, `panel_windows.rs:789`, `panels.rs:5123`): ensure canvas inline text edit and drop hints never obscure floating `egui::Window`s, menus, or modal dialogs (`Order::Middle` for floating panels, `Order::Foreground` only for transient popups/menus, `Order::Tooltip` for drag ghosts).
  3. Capture and visually inspect a zoomed-in screenshot with an open dialog & dropdown menu overlapping the canvas.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- panel-snapshot'
  ```
- **Done when:** Canvas stays strictly clipped to its viewport rect and floating windows, menus, and dialogs always paint cleanly above it.

---

### `L1.10` `[LUNA]` Eliminate Canvas & WASM Main-Thread Performance Stalls

- **Files:** `crates/app/src/shell/optimize.rs`, `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/canvas_ui.rs`, `crates/app/src/shell/mod.rs`
- **Experiment Ref (read-only):** `app/src/panels/bodies.rs:8-108`, `app/src/canvas.rs:302-411`, `app/src/harness.rs` (frame-time budget test)
- **Pre-check:**
  ```bash
  sed -n '200,279p' crates/app/src/shell/optimize.rs && sed -n '545,612p' crates/app/src/shell/canvas_ui.rs
  ```
- **Do:**
  1. Stop `optimize.rs:203-278` from synchronously running full-canvas `compose::render()` + PNG deflate encode + `io::raster::decode()` + 1920×1080 texture upload on startup and every edit! Show a lightweight estimated size readout immediately (like Experiment `estimated_export_kb()`) and gate the real encoded byte/thumbnail refresh behind an explicit `"Refresh Preview"` button or idle debounce (400 ms) ONLY when the `Optimize` tab is active & expanded.
  2. Decouple interactive zoom/scroll from synchronous 16 MiPix `compose::render_scaled()` (`canvas_ui.rs:550-596`): while zooming/scrolling interactively, blit the cached GPU texture immediately with zero CPU re-render, and only re-rasterize at the new scale after zoom settles (e.g., 150 ms idle) and capped sensibly on `wasm32` (e.g., 4 MiPix max on WASM).
  3. Reuse the uploaded GPU texture across viewport drag, hover, panel switches, and UI-only repaints; never call `compose::render` when document stamp has not changed.
  4. Add unit tests asserting: (a) idle/scroll frames trigger zero `compose::render` calls, and (b) Optimize panel open on startup triggers zero synchronous PNG encode/decode.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- dist --web'
  ```
- **Done when:** Startup, scrolling, zooming, tab switching, and editing feel crisp with zero main-thread stalls on desktop and WASM.

---

### `L1.11` `[LUNA]` Fix Keybindings, Focus Guards (`text_edit_focused`), Global `Esc`, `Tab`/`F11` & Tool Cycling

- **Files:** `crates/app/src/shell/mod.rs`, `crates/app/src/keymap.rs`, `crates/app/src/shell/canvas_ui.rs`
- **Experiment Ref (read-only):** `app/src/layout.rs:1581-2031`, `app/src/canvas.rs:302-411`
- **Pre-check:**
  ```bash
  sed -n '2832,2895p' crates/app/src/shell/mod.rs
  ```
- **Do:**
  1. Replace `ctx.egui_wants_keyboard_input()` at `shell/mod.rs:2837` with `ctx.text_edit_focused()` (the exact Experiment fix at `layout.rs:1635` — clicking any tab or toolbar button must NEVER silently disable single-key tool shortcuts!).
  2. Handle `Escape` globally BEFORE `text_edit_focused()` (`layout.rs:1584-1634`): finish/cancel pen draft -> surrender text focus -> close any open menu/context-menu/popup/dialog -> cancel drag/inline text and return to Pointer tool.
  3. Handle `Shift+Tab` (toggle right dock) BEFORE bare `Tab` / `F4` (full-screen canvas, surrendering widget focus so egui doesn't focus-cycle widgets), and bind `F11` separately to true OS window fullscreen.
  4. Match Experiment's tool-group cycling (`U` Rect->Rounded->Ellipse->Polygon->Star, `B` Brush<->Pencil, `M` Marquee<->Lasso, `G` Gradient<->Fill), `Space`/Middle-drag Hand scroll, `[`/`]` brush size, bare `+`/`-` zoom, `Insert` Fit Window, `Home`/`End` First/Last State, `Ctrl+K` alias for Command Palette, and `Ctrl+Scroll` zoom / `Shift+Scroll` horizontal / plain scroll vertical / `Alt+Scroll` Hand scroll + trackpad `zoom_delta()`.
  5. Add unit tests covering every shortcut priority and proving a clicked tab/button does NOT block `V`/`U`/`Tab`/`Esc`.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** All core tool, `Esc`, `Tab`/`Shift+Tab`/`F11`, wheel/pinch, and cycling shortcuts match Experiment reliably on desktop & WASM.

---

### `L1.12` `[LUNA]` Upgrade Top Menu Bar (`Alt+` Mnemonics & Arrow Nav) & Right-Click Context Menus

- **Files:** `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/canvas_ui.rs`, `crates/app/src/menus.rs`
- **Experiment Ref (read-only):** `app/src/menus.rs:20-505,916-1181`, `app/src/canvas.rs:663-1085`
- **Pre-check:**
  ```bash
  sed -n '405,450p' crates/app/src/shell/chrome.rs && sed -n '1782,1855p' crates/app/src/shell/canvas_ui.rs
  ```
- **Do:**
  1. Port Experiment's `Alt+<letter>` menu mnemonics (`Alt+F/E/V/S/M/T/C/L/W/H`) with visual underline indicators when `Alt` is held, plus Left/Right/Up/Down/Enter/Space/Escape menu keyboard navigation (`MenuNav`).
  2. Upgrade `Text` menu with live selection readout + scrollable `Font` / `Size` submenus, and `Filters` menu with active live-effect summaries, inline enable checkboxes, and per-effect Remove rows.
  3. Replace the plain `egui::Popup` canvas context menu (`canvas_ui.rs:1789`) with a styled, viewport-clamped CS6 context menu with right-aligned shortcut labels, right-click pick-before-open, Undo/Redo, Cut/Copy/Copy CSS/Paste/Paste Attributes, Duplicate/Clone/Delete, Group/Ungroup, 4-level Arrange, type-specific actions (Text/Bitmap/Slice/Path), `Combine Paths` & `Align Selection` submenus, Snap To Pixel, Flatten, Convert to Symbol, Lock/Unlock, and rich empty-canvas actions (`Insert Shape / Element`, New Layer, Import/Export, Canvas Size, Fit, checked Rulers/Grid/Guides, Demo Showcase).
  4. Verify `menus::tests` and `xtask reachability` (`257/257`) remain 100% green; add unit tests for `Alt+` mnemonics and right-click pick + dispatch.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo run -p xtask -- reachability && cargo run -p xtask -- aegis'
  ```
- **Done when:** Top menus support `Alt+` mnemonics + arrow navigation and right-click menus match Experiment's polish with 257/257 reachability.

---

### `L1.13` `[LUNA]` Upgrade Styles Panel (6 CS6 Libraries + Rendered Thumbnails) & Layers Panel (Blend/Opacity + Object Hierarchy)

- **Files:** `crates/app/src/shell/panels.rs`, `crates/app/src/shell/constants.rs`, `crates/app/src/shell/mod.rs`
- **Experiment Ref (read-only):** `app/src/panels/bodies.rs:992-1177`, `app/src/doc.rs:638-750`, `app/src/panels/tabs.rs:146-354`
- **Pre-check:**
  ```bash
  sed -n '968,1061p' crates/app/src/shell/panels.rs && sed -n '528,600p' crates/app/src/shell/panels.rs
  ```
- **Do:**
  1. **Styles panel (`panels.rs:968`):** Wire the existing `constants::StyleLibrary` enum (`Current Document`, `Dark Chrome`, `Chrome & Metal`, `Glass & Gel`, `Plastic & Web UI`, `Shadow & Glow`) with built-in clean-room style presets + rendered **34×34 px multi-band gradient, bevel, glow, drop-shadow, and stroke thumbnail tiles** in a scrollable wrapped grid, active selection highlight, `+ New Style`, `Redefine`, `Import .stl`, `Scale Styles`, and `Delete` wired through `engine` commands (preserving `257/257` reachability).
  2. **Layers panel (`panels.rs:528`):** Add CS6 top header with active layer `Blend Mode` dropdown + `Opacity %` scrub; render layers in top-to-bottom Z-order with crisp Eye/Lock icon columns, Folder vs Web Slice icons, object counts (`Layer 1 (N)`), **indented expandable child object rows** showing per-object Eye toggle + 10×10 live fill/stroke color swatch + `Name [Type]` + `Shift/Ctrl` multi-select, inline rename, reorder controls, Shared Layers integration, and compact bottom action bar (`+ Web Slice`, `+ Layer`, Duplicate, Delete).
  3. Capture and visually inspect desktop & WASM screenshots showing both Styles thumbnails and Layers object tree.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings && cargo run -p xtask -- panel-snapshot && cargo run -p xtask -- reachability && cargo run -p xtask -- aegis'
  ```
- **Done when:** Styles displays rendered 6-library thumbnail grids and Layers displays blend/opacity + indented object hierarchy with color swatches.

---

### `L1.14` `[LUNA]` Upgrade Bottom Properties Inspector & Wire `cs6_color_button` Swatch Matrix Everywhere

- **Files:** `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/panels.rs`
- **Experiment Ref (read-only):** `app/src/panels/properties.rs:1-628`, `app/src/panels/mod.rs:445-745`, `app/src/panels/color.rs:1-295`
- **Pre-check:**
  ```bash
  grep -n "cs6_color_button" crates/app/src/shell/panels.rs
  ```
- **Do:**
  1. Wire the existing `cs6_color_button` 18-column CS6 Swatch Matrix popover (HEX copy, RGB scrubs, Opacity `%`, Transparent, expandable HSV Wheel) onto Properties Fill, Stroke, Gradient stops, and Document Canvas BG (not just Color Mixer!).
  2. Add Experiment's **Dedicated Multi-Selection Properties Bar** (`N Objects Selected`, Group/Duplicate/Delete, 6 quick Align buttons, batch Fill/Stroke/Opacity/`+ Shadow`) and inline object **Gradient Fill Editor** (`Start -> End` + `+ Gradient` / `Remove`) + live Filters `+ Add` / toggle checkboxes / `- Clear`.
  3. Wrap `properties_body` in a vertical `ScrollArea` so the 2nd property row never clips at the `118 px` dock height, and verify Color Palette `Selector | Mixers | Blender` 12-step blend ramp works cleanly.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** Properties supports multi-selection, inline gradient & live filters, internal scrolling, and CS6 Swatch Matrix pickers everywhere.

---

### `L1.15` `[LUNA]` Implement God View / Infinite Canvas (`∞ Canvas`) & Canvas Footer / Ruler / Slice Polish

- **Files:** `crates/app/src/shell/canvas_ui.rs`, `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/mod.rs`, `crates/app/src/shell/slice_editor.rs`
- **Experiment Ref (read-only):** `app/src/canvas.rs:1098-1188,1416-1550,1857-1882`, `app/src/layout/chrome.rs:639-668`, `docs/features/infinite-canvas.md`
- **Pre-check:**
  ```bash
  grep -n "infinite_canvas\|RulerUnit" crates/app/src/shell/chrome.rs crates/app/src/shell/mod.rs
  ```
- **Do:**
  1. Add **God View / Infinite Canvas** toggle (`∞ Canvas` button in canvas footer + `Ctrl+Shift+F`): extends page background across the viewport, shows off-page objects and ghosted hidden layers/objects (view-only, never mutating doc), draws dotted `PAGE BOUNDS · flat exports crop here` rectangle + `GOD VIEW` legend pill, and expands scroll bounds while keeping flat/SVG/HTML exports strictly page-cropped.
  2. Polish canvas rulers with fractional sub-ticks for `px` / `in` / `cm` / `mm` synced with `View` menu radio items and clickable top-left corner unit badge.
  3. Place slice URL labels **below** each slice and elide to slice width (matching `elide_to_width` in Experiment `canvas.rs:14-33,1857-1882`) so slice labels never obscure button text underneath; ensure Showcase opens unselected and fits live `960×640` canvas on frame 1.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app && cargo clippy -p app --all-targets -- -D warnings'
  ```
- **Done when:** `∞ Canvas` God View, fractional rulers, elided slice tabs, and first-frame fitting all work cleanly with unit tests.

---

### `L1.16` `[LUNA]` CS6 Tools Palette Polish, Amber Diamond Shape Handles, Clipboard/Drop Parity & Merge Phase 1 to `main`

- **Files:** `crates/app/src/shell/chrome.rs`, `crates/app/src/shell/canvas_ui.rs`, `crates/app/src/shell/mod.rs`
- **Experiment Ref (read-only):** `app/src/panels/tools.rs:245-575`, `app/src/canvas.rs:1988-2077`, commits `f6be9b6`, `03df4c2`, `90707ad`
- **Pre-check:**
  ```bash
  grep -n "AutoShape\|dropped_files\|Paste" crates/app/src/shell/chrome.rs crates/app/src/shell/mod.rs | head -n 25
  ```
- **Do:**
  1. Polish the left Tools palette (`chrome.rs`): subtle tinted category bands, left-clickable corner flyout triangles (in addition to right-click/press-and-hold), auto-compacting `Sides` / `Star %` + wrapped amber-diamond hint under Vector, and pinned color/full-screen footer outside the scroll area.
  2. Render interactive **amber diamond** Auto Shape control handles on canvas for selected Rounded Rectangles (corner radius drag) and Stars (inner radius ratio drag), wired through `engine` undoable commands.
  3. Verify OS clipboard `Ctrl+V` image paste fallback when egui emits empty `Event::Paste` and verify Drag & Drop accepts `.fw.png`, raster images, and `.svg` files cleanly.
  4. Rebuild fresh WASM (`xtask dist --web`), capture desktop + browser screenshots, open/verify PR merging `feat/phase1-visual-polish` -> `main` with 100% green CI, and **STOP for visual verification** before Phase 2.
- **Verify:**
  ```bash
  ssh fedora 'export PATH="$HOME/.cargo/bin:$PATH"; export CARGO_TARGET_DIR=/var/tmp/firecraft-target-fedora; cd /var/tmp/firecraft-int-1.0-git && git pull --ff-only && cargo test -p app --features native && cargo clippy --workspace --all-targets -- -D warnings && cargo run -p xtask -- panel-snapshot && cargo run -p xtask -- reachability && cargo run -p xtask -- aegis && cargo run -p xtask -- dist --web'
  ```
- **Done when:** All Phase 1 P0 defects and core Experiment parity items pass full CI, visual checks, and merge cleanly to `main`.
