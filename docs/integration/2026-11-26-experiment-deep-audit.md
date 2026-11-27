# Deep Module-by-Module Audit: `FireCraft-CraftAnything-Experiment` (`v0.3.0`) vs `FireCraft`

> **Audit Date:** 2026-11-26 (read-only forensic inspection of source, git history, screenshots, and CI workflows)
> **Reference Repo (READ-ONLY):** `/nas/Temp/repos/FireCraft-CraftAnything-Experiment` — branch `fix/version-only-release-title` (`6233a49`), `main` / tag **`v0.3.0`** (`e415a33`), 38 `.rs` files (`33.3k` LOC), 180 passing tests, 67.61% line coverage ratchet, 8 published multi-platform release assets + live GitHub Pages WASM deploy.
> **Target Repo:** `/nas/Temp/repos/FireCraft` (worktree `.worktrees/integration-firecraft-1.0`, branch `feat/phase1-visual-polish` at `17dff0fc`, ahead of `main` `5a5a1b2c`).
> **Purpose:** Exhaustively inventory every feature, UX enhancement, visual polish pass, bug fix, architectural seam, test harness, and release-packaging improvement present in `FireCraft-CraftAnything-Experiment` that is missing, broken, or inferior in `FireCraft`, and map each gap to an atomic GPT-6-Luna task card.

---

## Executive Verdict

Comparing the live codebases side-by-side explains why the maintainer's live review felt like a massive step backward on `FireCraft`:

1. **Where `FireCraft-CraftAnything-Experiment` (`v0.3.0`) is dramatically ahead:**
   - **Desktop/Web GUI polish & usability:** Experiment completed 50+ targeted UX, visual, and interaction commits through `v0.3.0` (`768a83d`..`6233a49`), including embedded MIT **Phosphor Regular icon font**, crisp FireCraft Flame vector logo + `.ico` + animated splash, `ThemeMode::System` default, clipped & fast 60 fps canvas rendering, **God View / Infinite Canvas**, live drag-to-swap document tabs with MRU overflow + bulk close + **multi-monitor OS canvas windows**, real CS6 **Styles** (6 libraries + rendered 34×34 gradient/bevel/glow/shadow thumbnails) & **Layers** (blend mode, opacity `%`, indented object hierarchy w/ per-object eye toggles & color swatches), full floating/snapping/stacking panel windows, 1-click collapse chevrons, `text_edit_focused()` shortcut safety + global `Escape` + `Alt+<key>` menu mnemonics, 4-mode Find & Replace, `Intelligence` AI tab, `Check for Updates`, 1,017-line headless click/frame-budget GUI harness, and a **passing 4-platform release pipeline** (Windows ZIP, universal macOS ZIP+DMG, Linux tar.gz/AppImage/DEB/Flatpak, WASM ZIP + GitHub Pages).
2. **Where `FireCraft` (this repo) is structurally stronger underneath:**
   - 17-crate clean-room workspace (`fwpng` binary chunk/nodetree parser, full `doc` CS6 document model with master pages, shared layers, mask stacks, 9-slice guides, symbol instances, `vector`, `filters`, `text`, `compose`, `io`, `scripting`, `webexport`), **257/257 reachable engine commands**, crash-recovery journal sidecar, IndexedDB/localStorage web persistence & `postMessage` WebOS bridge, fuzz anchors, and `xtask` verification gates (`layers`, `corpus`, `reachability`, `panel-snapshot`, `aegis`).
3. **Why the earlier shell merge fell short:**
   - The earlier CS6 shell extraction ported only a subset of chrome and left **dead, unwired struct fields** (`floating_doc_tabs`, `show_update_dialog`, `show_page_setup_dialog`, `StyleLibrary`), left `canvas_ui.rs` unclipped and synchronously re-rendering CPU bitmaps on scroll/zoom, left `optimize.rs` synchronously running full-canvas CPU compose + PNG encode + PNG decode on the UI thread, kept `egui_wants_keyboard_input()` (which kills shortcuts after the first tab click), kept slot-0 tab swapping, and left `Styles` and `Layers` as bare text lists.

---

## 1. Smoking-Gun Root Causes for All 10 Live-Review Defects

Every live-review symptom maps to verified source lines in `FireCraft` vs a solved implementation in `Experiment`:

| # | Maintainer Report | Verified Bug in `FireCraft` | Solved Reference in `Experiment` | Luna Task |
|---|---|---|---|---|
| 1 | **Wrong app icon / logo** | `crates/app/src/logo.png` & `web/favicon.png` are a generic orange circle; `welcome_overlay.rs` draws a white backing square behind it. | `app/assets/{logo.svg,logo.png,firecraft.ico,splash.png}` & `app/favicon.png` (clean dark rounded square + crisp red/gold flame); `main.rs:27` loads `logo.png` as OS window icon. | `L1.6` |
| 2 | **Performance is poor** | **Two compounding main-thread stalls:** (a) `chrome.rs:299` opens `Optimize` by default, and `optimize.rs:231-237` synchronously runs full `1920×1080` `compose::render` + PNG encode + `io::raster::decode` + texture upload just for a 128×128 preview; (b) `canvas_ui.rs:1656` zooms on every wheel tick, triggering synchronous `compose::render_scaled` (up to 16 MiPix) on the UI thread. | `panels/bodies.rs:98` uses lightweight analytical `estimated_export_kb()` (zero sync encode/decode); `canvas.rs:1616` renders vectors directly via GPU tessellation; `harness.rs` enforces a 500-object `@ 1920×1080` frame-time budget; `theme.rs:116` caches `system_is_light()` in `OnceLock` with `CREATE_NO_WINDOW`. | `L1.10` |
| 3 | **Icons are not the requested Phosphor Icons** | Split-brain icons: `icons.rs` embeds 57 Phosphor SVGs parsed at runtime by `resvg` (bloating WASM), while `chrome.rs`, `panel_windows.rs`, `panels.rs`, and `welcome_overlay.rs` still call programmatic `vector_icons::draw` (2,157 LOC) + ASCII text. | `app/assets/fonts/Phosphor-Regular.ttf` (488 KB MIT font) embedded once via `icons::install_phosphor_font(&ctx)` (`icons.rs:10`) + `app/src/icons/phosphor.rs` (65 LOC mapping 60+ `Icon` variants directly to Phosphor glyphs) — **zero `resvg` bloat** on native or WASM! | Phase 2 (`L2.1`–`L2.6`) |
| 4 | **Canvas renders over other windows & chrome** | `canvas_ui.rs:617-780` draws the page shadow, texture (`image_rect`), selection box, handles, and guides directly onto unclipped `ui.painter()`, bleeding over the Document Tab Strip, Rulers, and Canvas Footer whenever zoomed/scrolled; plus `Order::Foreground` overlays (`canvas_ui.rs:1710`, `mod.rs:3026`, `panel_windows.rs:789`, `panels.rs:5123`). | `canvas.rs:1094` wraps all canvas painting in `let clip_vp = painter.with_clip_rect(viewport_rect);` and clips page contents to `page_rect.intersect(viewport_rect)` (except in God View, which stays clipped to `viewport_rect`); floating windows use `Order::Middle`, drag ghosts use `Order::Tooltip`, menus/popups use `Order::Foreground`. | `L1.9` |
| 5 | **Keybindings & shortcuts broken** | `shell/mod.rs:2837` checks `ctx.egui_wants_keyboard_input()`, which becomes `true` after clicking ANY tab/button/combo box — silently killing every single-key shortcut for the rest of the session! Also missing global `Esc` focus surrender, `Alt+<key>` mnemonics, `Shift+Tab` precedence, `Tab` focus surrender, tool cycling (`U`/`B`/`M`/`G`), and `Ctrl/Shift/Alt+Scroll` navigation. | `layout.rs:1635` explicitly uses `!ui.ctx().text_edit_focused()` (never `egui_wants_keyboard_input()`), handles global `Escape` BEFORE text-focus checks (`layout.rs:1591`), handles `Shift+Tab` before bare `Tab` (`layout.rs:1648-1662`), dispatches `Alt+letter` menu mnemonics (`layout.rs:1696`), and cycles tool groups (`layout.rs:1958`). | `L1.11` |
| 6 | **Styles & Layers incomplete / poor UX** | `panels.rs:968-1061` (`styles_panel`) is a bare text list (`selectable_label` + `"redefine"`) with zero swatches and dead/unused `StyleLibrary`; `panels.rs:528-688` (`layers_section`) has no Blend Mode combo, no Opacity `%`, no indented child objects, no per-object visibility/color swatch, and crude `"all"` / `"To"` buttons. | `panels/bodies.rs:992-1177` + `doc.rs:638-750`: 6 CS6 Style Libraries + 34×34 rendered gradient/bevel/glow/shadow/stroke tiles + `+ New Style` + Delete; `panels/tabs.rs:146-354`: top Blend Mode + Opacity `%`, reversed Z-order, folder/Web Slice icons, object counts, indented child objects w/ per-object Eye + 10×10 color swatch + `Shift/Ctrl` multi-select + `+ Web Slice` / `+ Layer` / Duplicate / Delete. | `L1.13` |
| 7 | **Menus poor & right-click issues** | Plain `egui::MenuBar` without `Alt+` mnemonics/underlines or arrow navigation; `canvas_ui.rs:1789` uses a plain unstyled `egui::Popup` lacking aligned shortcuts, edge clamping, Undo/Redo, Paste Attributes, Combine Paths/Align/Insert submenus, and View toggles. | `menus.rs:198-1252`: custom `MenuNav` with `Alt+` underlines, full Left/Right/Up/Down/Enter/Esc keyboard nav, live document-derived `Text` Font/Size submenus, live `Filters` active-effect summaries/toggles/remove rows, File Preview/Print/Page Setup/HTML Setup/Recent; `canvas.rs:706-1085`: styled edge-clamped context menu with aligned shortcuts + rich object & empty-canvas menus. | `L1.12` |
| 8 | **Tab switching visually broken (slot 0 always active)** | `chrome.rs:993` (`// Active document tab (index 0 visually)`) stores active doc in `self.editor` and parked docs in `self.tabs`, always painting active doc at slot 0 and swapping clicked tabs into slot 0. | `layout/chrome.rs:43-443`: stable ordered `open_docs: Vec<Document>` + `active_doc: usize`, `tab_click_without_drag` (8 px slop), live drag-to-swap with ghost preview, MRU `…` overflow, raw `Event::MouseWheel` 1-notch step, bulk close menu (`Other`/`Left`/`Right`/`All`), dirty close prompt, drag-out-to-float OS window! | `L1.8` |
| 9 | **Theme defaults to Dark instead of Auto / System** | `shell/mod.rs:852` hardcodes `theme_mode: theme::ThemeMode::Dark`. | `app.rs` defaults to `ThemeMode::System`; `theme.rs:116-320` caches `system_is_light()` in `OnceLock` (`CREATE_NO_WINDOW` on Windows) and unit-tests WCAG AA (`>= 4.5:1`) for both Dark and Light palettes. | `L1.7` |
| 10 | **Missing Experiment features across workspace** | See 14-domain matrix below: God View / Infinite Canvas, Multi-Monitor OS canvas windows, Floating/Snapping/Stacking panels & grip bars, 1-click collapse chevron, Properties multi-select & inline gradient, Amber Diamond handles, 4-mode Find & Replace, `Intelligence` AI tab, Startup Splash, In-app File Browser, `Check for Updates`, Headless GUI Harness, and 4-platform Release CI. | Fully implemented & tested in `FireCraft-CraftAnything-Experiment` `v0.3.0`. | `L1.14`–`L1.16`, Phase 2–3, Phase 7 |

---

## 2. Complete 14-Domain Feature Parity & Gap Matrix

Status key: **`MISSING`** (absent in FireCraft), **`DEAD STUB`** (struct field/enum copied into FireCraft but never wired), **`BROKEN`** (present but buggy), **`INFERIOR`** (present in bare/incomplete form), **`PARITY`**, **`FIRECRAFT+`** (FireCraft engine goes deeper).

### Domain 1 — Branding, App Icon & Launch Splash

| Feature | Experiment (`v0.3.0`) Reference | FireCraft Current State | Status | Target Task |
|---|---|---|---|---|
| Clean-room FireCraft Flame mark (`logo.svg`, `logo.png`, `favicon.png`, `firecraft.ico`) | `app/assets/{logo.svg,logo.png,firecraft.ico}`, `app/favicon.png` (dark rounded square + red/gold flame) | Generic orange circle in `crates/app/src/logo.png` & `web/favicon.png`; white square behind Welcome logo | **BROKEN** | `L1.6` |
| Animated semi-transparent Startup Splash (`splash.png`, 2.2 s pulse + progress bar, click-to-skip, skipped on `--screenshot`) | `app/src/dialogs/mod.rs:1453-1548`, `app/assets/splash.png` | Absent (0 hits) | **MISSING** | `L3.8` |
| Single-source version from `Cargo.toml` (`--version`, About, bundles, CI) | `scripts/version.sh`, `env!("CARGO_PKG_VERSION")` everywhere | 8 separate version stamps across workspace & `web/` | **INFERIOR** | `L7.2` |

### Domain 2 — Icon System (MIT Phosphor Icons)

| Feature | Experiment (`v0.3.0`) Reference | FireCraft Current State | Status | Target Task |
|---|---|---|---|---|
| Embedded **Phosphor-Regular.ttf** icon font (488 KB MIT) registered in `egui::FontDefinitions` | `app/assets/fonts/Phosphor-Regular.ttf`, `app/src/icons.rs:10`, `app/src/icons/phosphor.rs` (60+ glyphs) | 57 runtime-parsed Phosphor SVGs via `resvg` in `icons.rs` (only used in flyouts), while `chrome.rs`/`panels.rs`/`panel_windows.rs` use 2,157 LOC of programmatic `vector_icons.rs` | **INFERIOR** | `L2.1`–`L2.6`, `L4.3` |
| Dual-tone Align & Path schematic diagram icons (`AlignIcon`, `PathIcon`) | `app/src/icons.rs:95-190`, `panels/bodies.rs:247-854` | Partial (`vector_icons.rs`) | **INFERIOR** | `L2.5` |
| Third-party Phosphor MIT notice bundled in crate & release archives | `app/assets/fonts/LICENSE-MIT`, `THIRD-PARTY-NOTICES.md` | `docs/ATTRIBUTION.md` + `xtask icon-survey` | **PARITY** | `L2.2` |

### Domain 3 — Theme & Accessibility

| Feature | Experiment (`v0.3.0`) Reference | FireCraft Current State | Status | Target Task |
|---|---|---|---|---|
| Default to `ThemeMode::System` (Auto) | `app.rs` default `ThemeMode::System` | Hardcoded `ThemeMode::Dark` (`shell/mod.rs:852`) | **BROKEN** | `L1.7` |
| Cached `system_is_light()` (`OnceLock`) + Windows `CREATE_NO_WINDOW` (prevents UI freeze & console flash) | `app/src/theme.rs:116-150` | Needs browser `prefers-color-scheme` + cached native check | **INFERIOR** | `L1.7` |
| WCAG AA contrast ratio machine tests (`>= 4.5:1` text, `>= 3.0:1` dim, `>= 4.5:1` accent) | `app/src/theme.rs:259-320` | No automated theme contrast ratio test | **MISSING** | `L1.7` |
| Accessibility checklist (`docs/A11Y.md`) | `docs/A11Y.md` | AccessKit present (`FIRECRAFT+`), checklist doc missing | **PARTIAL** | `L6.5` |

### Domain 4 — Canvas Viewport, Z-Order, Performance & God View

| Feature | Experiment (`v0.3.0`) Reference | FireCraft Current State | Status | Target Task |
|---|---|---|---|---|
| Strict viewport clip rect on canvas rendering (never paints over tabs, rulers, footer, or floating windows) | `app/src/canvas.rs:1094` (`painter.with_clip_rect(viewport_rect)`) | `canvas_ui.rs:617-780` paints unclipped onto `CentralPanel`'s `ui.painter()` | **BROKEN** | `L1.9` |
| Zero synchronous full-canvas PNG encode/decode on Optimize panel draw | `panels/bodies.rs:98` (`estimated_export_kb()`) | `optimize.rs:231-237` synchronously runs `compose::render` + PNG encode + PNG decode + 1920×1080 texture upload on UI thread! | **BROKEN** | `L1.10` |
| Fast interactive scroll/zoom/drag without main-thread 16 MiPix CPU re-raster stalls | `canvas.rs:1616` GPU-tessellated vectors + `harness.rs` 500-object frame-time budget test | `canvas_ui.rs:550-596` & `1656` triggers sync `compose::render_scaled` on every wheel zoom tick | **BROKEN** | `L1.10` |
| **God View / Infinite Canvas (`∞ Canvas`, `Ctrl+Shift+F`)** — extended page background, out-of-page editing, ghosted hidden layers (`0.28`/`0.30` alpha), dotted `PAGE BOUNDS · flat exports crop here` box, `GOD VIEW` badge, `±100k` scroll bounds | `app/src/canvas.rs:1098-1188`, `layout/chrome.rs:639-668`, `docs/features/infinite-canvas.md` | Absent (0 hits) | **MISSING** | `L1.15` |
| CS6 Mouse-Wheel & Pinch Navigation (`Ctrl+Scroll` cursor-anchored zoom, `Shift+Scroll` horizontal, plain Scroll vertical, `Alt+Scroll` Hand drag, `zoom_delta()` trackpad pinch, `PointingHand` cursor) | `app/src/canvas.rs:237-411` | `canvas_ui.rs:1656` zooms unconditionally on bare scroll wheel; no `Shift+Scroll` horizontal or `Alt+Scroll` Hand mode | **BROKEN** | `L1.11` / `L1.15` |
| Clickable top-left Ruler corner unit switcher (`px` / `in` / `cm` / `mm`) with fractional sub-ticks | `app/src/canvas.rs:1416-1550` + `168-182` | `RulerUnit` enum exists (`constants.rs:168`, `chrome.rs:1692`); verify sub-ticks & menu radio sync | **PARTIAL** | `L1.15` |
| Radial gradient object clipping (no disc bleed over neighbors or unpainted corners) | `app/src/canvas.rs:1784-1800` | Handled by `compose` (`FIRECRAFT+`) | **PARITY** | — |
| Slice URL tab placed **below** slice, elided to width (never covers button labels underneath) | `app/src/canvas.rs:1857-1882` (`elide_to_width`) | Verify in `slice_editor.rs` / `canvas_ui.rs` | **PARTIAL** | `L1.15` |
| Showcase opens unselected & fits live canvas rect on first frame (`pending_fit`) | `app/src/canvas.rs:45-47`, `app/src/demo.rs` | First fit present; verify showcase unselected & 960×640 fit | **PARTIAL** | `L1.15` |
| Multi-line text bounds use longest line + line count (not newline-char over-measurement) | `app/src/doc.rs` (`text_width`) | Handled by `text` crate (`FIRECRAFT+`) | **PARITY** | — |

### Domain 5 — Document Tabs, Dirty-Close Protection & Multi-Monitor Windows

| Feature | Experiment (`v0.3.0`) Reference | FireCraft Current State | Status | Target Task |
|---|---|---|---|---|
| Stable ordered tab list + `active_doc` index (clicking Tab 2 highlights Tab 2 in place without swapping into slot 0) | `app/src/layout/chrome.rs:43-236` (`open_docs`, `active_doc`) | `chrome.rs:993` ALWAYS draws active doc in slot 0 and swaps parked tabs! | **BROKEN** | `L1.8` |
| `tab_click_without_drag` (8 px pointer slop so slight mouse jitter never eats a tab click) | `app/src/layout/chrome.rs:9-22`, `panels/mod.rs:1007-1020` | Missing | **MISSING** | `L1.8` |
| Live drag-to-swap tab reordering + translucent ghost tab preview (`Order::Tooltip`) | `app/src/layout/chrome.rs:237-274`, `411-424` | Missing | **MISSING** | `L1.8` |
| MRU tab overflow (`…` dropdown listing all open docs + `Close All Tabs`) & raw `Event::MouseWheel` 1-notch stepping | `app/src/layout/chrome.rs:61-135`, `347-386` | Basic `hidden_count`; wheel jumps straight to `0` or `len-1` (`chrome.rs:981`) | **BROKEN** | `L1.8` |
| Full Tab Right-Click Menu (`Close Tab`, `Close Other`, `Close to Left`, `Close to Right`, `Close All`, `Move Left/Right`, `Float in New Window`) | `app/src/layout/chrome.rs:275-345` | Only `New`, `Close Other`, `Close Tab` (`chrome.rs:1050`) | **INFERIOR** | `L1.8` |
| Dirty-tab close confirmation (`Save / Don't Save / Cancel`) + non-destructive bulk-close queue (`pending_close_tabs`) + `Unsaved Changes / Quit` (`Save All & Quit`, `Discard & Quit`, `allow_close` gate) | `app/src/app/files.rs:208-365`, `app/src/dialogs/mod.rs:1076-1261` | Basic `confirm` modal; no bulk-close queue or Save-All-untitled trampoline | **INFERIOR** | `L3.7` |
| **Floating Multi-Monitor OS Canvas Windows** (drag tab out or click `⧉` -> `ctx.show_viewport_immediate` + `Dock to Main Window` / `Dock All`) | `app/src/layout.rs:552-661`, `layout/chrome.rs:261,394-410` | `floating_doc_tabs: Vec<usize>` copied into `shell/mod.rs:407` but **NEVER RENDERED**! | **DEAD STUB** | `L3.7` |

### Domain 6 — Keybindings, Shortcuts & Focus Handling

| Feature | Experiment (`v0.3.0`) Reference | FireCraft Current State | Status | Target Task |
|---|---|---|---|---|
| Guard single-key shortcuts with `text_edit_focused()`, NEVER `egui_wants_keyboard_input()` | `app/src/layout.rs:1635-1672` | `shell/mod.rs:2837` uses `egui_wants_keyboard_input()` — breaks shortcuts after clicking any tab/button! | **BROKEN** | `L1.11` |
| Global `Escape` priority chain BEFORE `text_edit_focused()` (finish pen -> surrender focus -> close menu/popup/dialog -> cancel drag/inline text & return to Pointer) | `app/src/layout.rs:1584-1634` | `shell/mod.rs:2857` blocks `Esc` when `text_focus` is true and only cancels drafts | **BROKEN** | `L1.11` |
| `Shift+Tab` (toggle right dock) consumed BEFORE bare `Tab` / `F4` (full-screen canvas, surrendering widget focus) + `F11` (OS window fullscreen) | `app/src/layout.rs:1646-1662`, `1734-1738` | `Tab` conflicts with egui widget focus; `F11` vs `Tab`/`F4` not cleanly separated | **BROKEN** | `L1.11` |
| `Alt+<letter>` menu bar mnemonics (`Alt+F/E/V/S/M/T/C/L/W/H`) with visual underline indicators + full arrow-key / `Enter` / `Space` menu navigation | `app/src/layout.rs:1696-1730`, `app/src/menus.rs:200-296` | Absent in `chrome.rs:411` (`egui::MenuBar`) | **MISSING** | `L1.11` / `L1.12` |
| Single-key tool-group cycling (`U` Rect->Rounded->Ellipse->Polygon->Star, `B` Brush<->Pencil, `M` Marquee<->Lasso, `G` Gradient<->Fill) | `app/src/layout.rs:1958-1975` | Single-key dispatch without Experiment's cycle behavior | **INFERIOR** | `L1.11` |
| Convenience shortcuts: `[`/`]` brush size, bare `+`/`-` zoom, `Insert` Fit Window, `Home`/`End` First/Last State, `Ctrl+Shift+F` Infinite Canvas, `Ctrl+Alt+I` Intelligence, `Ctrl+K`/`Ctrl+P` Command Palette | `app/src/layout.rs:1777-1806` | Partially mapped; missing `Insert`, `Home`/`End`, `Ctrl+Shift+F`, `Ctrl+Alt+I`, `Ctrl+K` alias | **INFERIOR** | `L1.11` |

### Domain 7 — Top Menus & Right-Click Context Menus

| Feature | Experiment (`v0.3.0`) Reference | FireCraft Current State | Status | Target Task |
|---|---|---|---|---|
| Document-derived `Text -> Font / Size` scrollable submenus + live selection readout | `app/src/menus.rs:916-986` | Static menu rows | **INFERIOR** | `L1.12` |
| Document-derived `Filters` menu with active live-effect summaries, inline enable checkboxes, and per-effect Remove + categorized submenus | `app/src/menus.rs:1031-1181` | Static filter rows without live active-effect toggle/remove list | **INFERIOR** | `L1.12` |
| `File` menu completeness (`New from Template`, `Open Recent`, `Save a Copy`, `Save All`, `Revert`, `HTML Setup…`, `Preview in Browser F12`, `Page Setup…`, `Print…`) | `app/src/menus.rs:506-608` | `show_page_setup_dialog` is a dead stub (`mod.rs:414`); `Open Recent` unpersisted; `Print…` missing | **INFERIOR** | `L1.12` / `L3.6` |
| Styled CS6 Canvas Right-Click Context Menu (aligned shortcuts, viewport edge clamping, Undo/Redo, Paste Attributes, Combine Paths submenu, Align Selection submenu, Snap To Pixel, Flatten, Insert Shape submenu, checked Rulers/Grid/Guides, Demo Showcase) | `app/src/canvas.rs:663-1085` | Plain `egui::Popup` in `canvas_ui.rs:1789` — no aligned shortcuts, no submenus, no Undo/Redo/Paste Attributes/View toggles | **INFERIOR** | `L1.12` |

### Domain 8 — Styles, Layers, Swatches & Color Panels

| Feature | Experiment (`v0.3.0`) Reference | FireCraft Current State | Status | Target Task |
|---|---|---|---|---|
| **Styles panel:** 6 built-in CS6 Style Libraries (`Current Document`, `Dark Chrome`, `Chrome & Metal`, `Glass & Gel`, `Plastic & Web UI`, `Shadow & Glow`), scrollable 34×34 multi-band gradient + bevel + glow + drop-shadow + stroke thumbnail cards, selection border, `+ New Style`, Delete | `app/src/panels/bodies.rs:992-1177`, `app/src/doc.rs:638-750` | `panels.rs:968-1061` renders bare text rows; `StyleLibrary` enum in `constants.rs:58` is completely unused! | **BROKEN / INFERIOR** | `L1.13` |
| **Layers panel:** top `Blend Mode` combo + `Opacity %` scrub, reversed Z-order, Eye/Lock icons, Folder vs Web Slice icons, layer object counts, **indented child object tree** with per-object Eye toggle + 10×10 color swatch + `Name [Type]` + `Shift/Ctrl` multi-select, bottom `+ Web Slice` / `+ Layer` / Duplicate / Delete | `app/src/panels/tabs.rs:146-354` | `panels.rs:528-688` shows flat layer rows only with `"all"` / `"To"` buttons; no blend/opacity header, no indented object tree or swatches | **INFERIOR** | `L1.13` |
| **CS6 Color Swatch Matrix Popup (`cs6_color_button`)** — 18-col swatch cube grid + checkerboard transparency + HEX copy + RGB scrubs + Opacity `%` + `Transparent` + expandable HSV Wheel — wired onto Properties Fill, Stroke, Gradient stops, Canvas BG, and Color Mixer | `app/src/panels/mod.rs:445-745`, `panels/properties.rs:230-261,433,550` | `cs6_color_button` exists in `panels.rs:5015` but is ONLY called in Color Mixer (`panels.rs:4919,4936`), NOT Properties! | **PARTIAL** | `L1.14` |
| **Color Palette panel (`Selector | Mixers | Blender`)** — 12-step interactive Stroke->Fill Blend Ramp + HLS/HSV/CMYK/RGB + 2D SV square + Hue bar + Swap/No-Fill | `app/src/panels/color.rs:1-295` | Partially ported to `panels.rs`; verify Blender + Mixer parity & `cs6_color_button` | **PARTIAL** | `L1.14` |
| **Pages & States panels:** page color preview thumbnails, state count & delay `/100s` scrub, Onion toggle, `Tween 3x` | `app/src/panels/tabs.rs:7-145` | Functional via engine (`FIRECRAFT+` master pages/shared layers), visually sparser | **PARTIAL** | `L1.13` |

### Domain 9 — Panel Docking, Floating Windows, Stacking, Snapping & Workspace Presets

| Feature | Experiment (`v0.3.0`) Reference | FireCraft Current State | Status | Target Task |
|---|---|---|---|---|
| **Pure geometry module (`panels/geometry.rs`)** — 12 px magnetic edge/window snap, 28 px column align snap, vertical window stacking, unit-tested without display | `app/src/panels/geometry.rs:1-188` | Absent | **MISSING** | `L3.2` |
| **Floating Panel Groups & Vertical Window Stacking** (`stack_parent` moves whole column as 1 unit, `Detach from Window Stack`, `Group With…`, `Ungroup Active Tab Below`, `Pin to Primary/Inner-Right/Left Dock`) | `app/src/layout.rs:354-422`, `panels/mod.rs:1182-1242` | `floating_pos`/`floating_width`/`stack_parent` fields exist on `PanelGroup` (`panel_windows.rs:290-296`) but floating `egui::Area` rendering & stacking are NOT wired! | **DEAD STUB** | `L3.1`–`L3.2` |
| **Full Translucent Panel Drag Ghost (`draw_widget_drag_ghost`)** showing header, tabs, mock rows & drop hint at `Order::Tooltip` | `app/src/layout.rs:821-891` | Absent | **MISSING** | `L3.1` |
| **Floating / Snapping Tools & Right Panels Dock Grip Bars** (dotted grip drags to float or snap Left/Right; double-click toggles collapse; `Panels` label + single state-aware `▾`/`▸` collapse-all toggle) | `app/src/layout.rs:916-1205` | Static docked Tools & Right panel; no grip drag-to-float/snap | **MISSING** | `L3.2` |
| **Single-click collapse chevron fix** (background header double-click target registered BEFORE chevron / pointer-rect test so 1st click ALWAYS works!) | `app/src/panels/mod.rs:86-173`, `1093-1116` | Needs audit & click-level regression test | **PARTIAL** | `L3.1` |
| **Panel tab overflow (`»` menu)** + per-group bottom resize splitter (`72..520 px`) + always-visible panel scrollbars + unique `push_id` per panel body | `app/src/panels/mod.rs:1244-1376` | Tabs drop silently past `tabs_right` (`panel_windows.rs:1148`); no per-group resize splitter | **INFERIOR** | `L3.1` |
| **6 Workspace Presets:** `Expanded Mode`, `CS6 Dual-Column` (`RightOuter` + `RightInner`), `All Panels Floating`, `Iconic Mode` (36 px strip + icon-anchored flyouts), `Iconic Mode With Panel Names` (94 px), `Netbook Mode` | `app/src/menus.rs:364-381`, `layout.rs:166-279,1235-1427` | Only 3 presets (`Expanded`, `Iconic`, `Compact`); flyout position is fixed rather than anchored to clicked icon | **INFERIOR** | `L3.2` |
| **`PANELS` Descriptor Registry Seam** (single data table mapping `PanelId -> default_height + render fn`) | `app/src/panels/mod.rs:315-443` | Separate `PANELS` spec table + giant `match spec.kind` in `panel_windows.rs:1236` | **INFERIOR** | `L3.2` |

### Domain 10 — Bottom Properties Inspector & Intelligence Tab

| Feature | Experiment (`v0.3.0`) Reference | FireCraft Current State | Status | Target Task |
|---|---|---|---|---|
| Single shared collapsible header (`◊ Properties | Symbol Properties | Intelligence`), app-owned resizable height (`118..340 px` that never snaps back), internal vertical scroll so 2nd row never clips | `app/src/layout/chrome.rs:522-597`, `panels/properties.rs:9-15` | `Properties | Symbol Properties | Info`; verify scroll & 1-click chevron | **PARTIAL** | `L1.14` |
| **Dedicated Multi-Selection Properties Bar** (`N Objects Selected`, Group/Duplicate/Delete, 6 Align buttons, batch Fill/Stroke/Opacity/`+ Shadow`) | `app/src/panels/properties.rs:19-132` | Missing rich multi-select summary bar | **MISSING** | `L1.14` |
| Inline Object **Gradient Fill Editor** (`Start -> End` swatches + `+ Gradient` / `Remove`) & Live Filters `+ Add` / checkboxes / `- Clear` | `app/src/panels/properties.rs:427-525` | Separate Filters / Fill panels only | **INFERIOR** | `L1.14` |
| **`Intelligence` Bottom-Dock Tab** (OpenAI-compatible endpoint/model/API key via `FIRECRAFT_AI_API_KEY`, non-blocking worker sending real selection/doc context, response viewer, `Copy MCP config`) | `app/src/panels/bodies.rs:1182-1312` | Absent (0 hits) | **MISSING** | `L3.9` |

### Domain 11 — Tools Palette & Direct Manipulation

| Feature | Experiment (`v0.3.0`) Reference | FireCraft Current State | Status | Target Task |
|---|---|---|---|---|
| Compact 6-category 2-column CS6 Tools palette with tinted category bands, bright corner flyout triangles (left-click triangle OR right-click), auto-compacting `Sides` / `Star %` + wrapped hint, pinned Fill/Stroke footer | `app/src/panels/tools.rs:1-633` | `chrome.rs` tools rail uses programmatic icons, missing auto-compacting `Sides`/`Star %` hint & Phosphor icons | **INFERIOR** | `L1.16` / `L2.3` |
| **Interactive Amber Diamond Canvas Control Handles** (drag directly on canvas to adjust Rounded Rectangle corner radius & Star inner ratio!) | `app/src/canvas.rs:1988-2001`, `2033-2077` | Absent | **MISSING** | `L1.16` |
| Real Freehand Lasso polygon preview & selection, Magic Wand color-match, drag Gradient, Knife split, Pen rubber-band + double-click/Enter commit | `app/src/canvas.rs:443-566`, commit `77dc436` | Verify parity per tool in `canvas_ui.rs` / `editor.rs` | **PARTIAL** | `L1.16` |
| Clipboard `Ctrl+V` image paste fallback when egui emits empty `Event::Paste` (`arboard`) | Commit `03df4c2`, `app/src/app/edit.rs` | Verify OS image paste when `Event::Paste("")` fires | **PARTIAL** | `L1.16` |
| Drag & Drop images, SVGs, and `.fw.png` docs onto canvas (multi-format) | Commit `90707ad`, `layout.rs:670` | `mod.rs:3017` exists for `.fw.png`/rasters; verify SVG import & visual feedback | **PARTIAL** | `L1.16` |

### Domain 12 — Dialogs, File Browser, Find & Replace, Command Palette & Auto-Updater

| Feature | Experiment (`v0.3.0`) Reference | FireCraft Current State | Status | Target Task |
|---|---|---|---|---|
| Standardized dialog visual grammar (`dialog_header` accent bar + subtitle, `primary_button`, `dialog_footer_rule`) | `app/src/dialogs/mod.rs:23-50` | Ad-hoc `egui::Window` layouts | **INFERIOR** | `L3.8` |
| **In-App CS6 File Browser** (editable address bar + Go/Up/Refresh, clickable breadcrumbs, live search filter, file-type combo, Quick Places sidebar, dense columns) | `app/src/dialogs/file_browser.rs:1-734` | Native OS file dialogs only (plus web `<input>`) | **MISSING** (optional complement) | `L3.8` |
| **4-Mode Find & Replace Dialog** (`Text & object names`, `Fill color`, `Stroke color`, `Any color` across all pages including hidden objects, `Use fill`/`Use stroke`, live match indicator dot, `Find Next`, `Replace All`) | `app/src/dialogs/find_replace.rs:1-161`, `app/src/app/find_replace.rs` | `FindReplaceMode` enum copied into `constants.rs:387` — verify dialog wiring vs panel-only | **PARTIAL** | `L3.8` |
| **Supercharged Command Palette (`Ctrl+K` / `Ctrl+P`)** searching 7 categories: `replace:old->new`, Quick Actions, Workspace Presets, all 21 Panels, 11 Filters, 10 Common Library Widgets, live Canvas Objects (`find:<text>`), and catalog commands | `app/src/dialogs/mod.rs:579-972` | `palette.rs` searches canonical command IDs only | **INFERIOR** | `L3.4` |
| **`Help ▸ Check for Updates…` Dialog** (queries GitHub Releases latest tag, `Check Now`, `Open Releases Page`, `Copy Version Info`) | `app/src/dialogs/mod.rs:1263-1325` | `show_update_dialog: bool` is a **DEAD STUB** (`shell/mod.rs:420`)! | **DEAD STUB** | `L3.8` |
| **`HTML Setup…` & `Page Setup…` & `Print…` & `Preview in Browser (F12)`** | `app/src/dialogs/mod.rs:314-506`, `menus.rs:581-593` | `show_page_setup_dialog` is a **DEAD STUB** (`shell/mod.rs:414`); `Print…` missing | **DEAD STUB** | `L3.6` / `L3.8` |
| **Interactive `MCP & Headless CLI Console` Dialog** (preset JSON-RPC test buttons, live dispatch, reply viewer, CLI cheat sheet) | `app/src/dialogs/mod.rs:975-1072` | Headless CLI + MCP server exist (`app-cli` — `FIRECRAFT+`), in-app test console missing | **MISSING** | `L3.9` |
| **Redesigned `About FireCraft` Dialog** (64 px flame logo, `CARGO_PKG_VERSION`, clickable Wiki/Releases links, striped Workflow Shortcuts table) | `app/src/dialogs/mod.rs:1327-1450` | Minimal text `about_ui` in `panels.rs:58` | **INFERIOR** | `L3.8` |

### Domain 13 — OS Integration, Headless Testing & Architecture Seams

| Feature | Experiment (`v0.3.0`) Reference | FireCraft Current State | Status | Target Task |
|---|---|---|---|---|
| **`FileService` Trait Seam** (`open`/`save` trait + `FormatFileService` + `FakeFiles` test double) | `app/src/app/files.rs:52-67,378-456` | `engine::platform::FileSystem` exists (`FIRECRAFT+`) | **PARITY** | — |
| **Headless GUI Harness (`harness.rs`, 1,017 LOC)** driving real frames at `1360×860` & `960×640`: click-level chevron test, every `Alt+` mnemonic & menu row, panel float/merge/stack/detach/snap, Find/Replace, Styles/Filters, **500-object frame-time budget test** | `app/src/harness.rs:1-1017` | Unit tests exist (`350` in `app`), but no headless click-level frame/layout/frame-budget harness | **MISSING** | `L3.9` |
| **Deterministic CLI Capture Flags (`--size WxH`, `--state menu|flyout|find-replace`, `--splash`, `--about`, `--new`, `--screenshot`)** + throwaway `persistence_path` so persisted window size never overrides `--size` | `app/src/main.rs:54-196`, `scripts/gui-suite.sh` | Only `--no-welcome` / `--blank` (`crates/app/src/main.rs`) | **INFERIOR** | `L3.6` / `L3.9` |
| **Windows Shell Integration (`windows_subsystem = "windows"`, AppUserModelID, Jump List)** | `app/src/main.rs:1`, `app/src/shell_windows.rs` | `windows_subsystem` + `T30.3` planned; note FireCraft's `unsafe_code = "forbid"` requires a safe adapter crate (no raw `unsafe` COM in workspace) | **PLANNED** | `L6.9` |
| **Hard Module Size Budget (`scripts/module-size.sh`, 2,000-line cap) & Coverage Ratchet (`67.61%`)** | `scripts/module-size.sh`, `coverage-floor.txt` | `shell/mod.rs` is **8,626 lines**, `panels.rs` is **6,065 lines**, `editor.rs` is **5,912 lines**! | **INFERIOR** | Phase 3+ cleanup |
| **5 User Tutorials (`docs/tutorials/`)** | `docs/tutorials/{getting-started,keybindings,drawing-and-editing,pages-and-states,optimize-and-export}.md` | Planned for Phase 6 (`docs/wiki/`) | **MISSING** | `L6.6` |

### Domain 14 — Multi-Platform Packaging & Release CI

| Feature | Experiment (`v0.3.0`) Reference | FireCraft Current State | Status | Target Task |
|---|---|---|---|---|
| **Proven 4-Platform Tag Release Pipeline** producing **8 verified assets** (Windows ZIP, universal macOS `arm64+x86_64` ZIP + DMG w/ `make-iconset.swift`, Linux `tar.gz` + `AppImage` + `DEB` + `Flatpak`, WASM ZIP + live Pages deploy) + `RELEASE-MANIFEST.json` + `SOURCE-SHA.txt` + `SHA256SUMS.txt` + tag-only version-match check | `.github/workflows/release.yml:1-321`, `scripts/package-{windows,macos,linux}.sh`, `packaging/{linux,macos}/*` — **PROVEN GREEN on `v0.3.0` (`e415a33`)!** | `.github/workflows/release.yml` lacks Windows/macOS jobs, has Pages deploy commented out, `v0.2.5` is stale Latest, `v0.2.6` has only 1 `.wasm` file | **BROKEN / INFERIOR** | Phase 7 (`L7.2`–`L7.9`) |

---

## 3. Dead / Unwired Stub Fields Found in `FireCraft`

The earlier merge copied several state fields and enums into `crates/app/src/shell/` without wiring their UI or logic. Each must be **wired cleanly** (per its assigned task card) rather than left dead:

1. `crates/app/src/shell/mod.rs:407,826` — `floating_doc_tabs: Vec<usize>` (never rendered via `show_viewport_immediate` -> `L3.7`).
2. `crates/app/src/shell/mod.rs:414,418,830,832` — `show_page_setup_dialog: bool` & `page_setup: PageSetupConfig` (dialog never rendered -> `L3.8`).
3. `crates/app/src/shell/mod.rs:420,833` — `show_update_dialog: bool` (update checker & dialog never rendered -> `L3.8`).
4. `crates/app/src/shell/constants.rs:58-90` — `StyleLibrary` enum (6 libraries defined, never consumed by `panels.rs::styles_panel` -> `L1.13`).
5. `crates/app/src/shell/panel_windows.rs:290-296` — `PanelGroup::{floating_pos, floating_width, stack_parent}` (stored on groups, never rendered as floating `egui::Area` windows or stacked -> `L3.1`–`L3.2`).
6. `crates/app/src/shell/panels.rs:5015` — `cs6_color_button` (wired only inside Color Mixer, never wired to Properties Fill/Stroke/Canvas BG -> `L1.14`).

---

## 4. Clean-Room & Safety Constraints When Porting From Experiment

When implementing any task card referencing `/nas/Temp/repos/FireCraft-CraftAnything-Experiment`:

- **Strictly read-only** on the Experiment repository — never modify, reset, or commit inside it.
- **No `unsafe` code:** FireCraft enforces `unsafe_code = "forbid"` workspace-wide. Do **not** port Experiment's raw Win32 COM `shell_windows.rs` directly; use a safe wrapper crate or isolate per `T30.3` (`L6.9`).
- **No `unwrap()` / `expect()` / `panic!()` in non-test code:** Experiment's `main.rs`/`dialogs/mod.rs` occasionally use `.expect(...)` (e.g., splash decode); every FireCraft port must return typed `Result` or graceful fallback.
- **Clean-room UI copy:** Experiment's `start_panel` (`panels/bodies.rs:1597`) contains `"Adobe® Fireworks® CS6 Parity"` — **never** carry Adobe trademarks or `®` strings into FireCraft UI copy; keep clean-room `"FireCraft Studio — Editable Vector, Bitmap & Web Graphics"`.
- **Phosphor Font License Compliance:** When adopting `Phosphor-Regular.ttf` (MIT) in Phase 2 (`L2.2`), include its MIT license in `assets/fonts/LICENSE-phosphor-mit.txt`, register it in `docs/ATTRIBUTION.md`, and keep `xtask icon-survey` & `xtask aegis` green.
- **Preserve FireCraft's stronger engine seams:** Wire every upgraded panel (`Styles`, `Layers`, `Properties`, `Find & Replace`, `Menus`) through `self.editor` / `self.run_command(...)` so undo/redo, `.fw.png` persistence, crash recovery, and `xtask reachability` (`257/257`) remain 100% intact.
