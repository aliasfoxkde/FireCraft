# FireCraft — Live Review Defects & Repo Disconnect Audit (2026-11-26)

> **Source:** Direct maintainer live review of `https://firecraft-web.pages.dev` and desktop build vs `/nas/Temp/repos/FireCraft-CraftAnything-Experiment` (`v0.3.0`), plus forensic audit of source code, git worktrees, Cloudflare Pages, and GitHub Actions.
> **Full 14-Domain Module Audit:** [`docs/integration/2026-11-26-experiment-deep-audit.md`](2026-11-26-experiment-deep-audit.md)
> **Status:** All items below are mapped to atomic GPT-6-Luna task cards in [`PHASE-1-CI-AND-VISUAL-POLISH.md`](../planning/luna/PHASE-1-CI-AND-VISUAL-POLISH.md) (`L1.6`–`L1.16`), [`PHASE-2-PHOSPHOR-ICONS.md`](../planning/luna/PHASE-2-PHOSPHOR-ICONS.md) (`L2.1`–`L2.6`), and [`PHASE-3-CORE-UX-FEATURES.md`](../planning/luna/PHASE-3-CORE-UX-FEATURES.md) (`L3.1`–`L3.11`).

---

## 1. Live Product & GUI Defects Reported by Maintainer

| # | Defect Reported | Verified Root Cause in FireCraft Source | Assigned Luna Task |
|---|---|---|---|
| **1** | **Wrong app icon / logo** | `crates/app/src/logo.png` (embedded in `welcome_overlay.rs` & `main.rs`) and `web/favicon.png` render a generic orange gradient circle with a white backing box instead of the crisp FireCraft flame artwork shipped in Experiment `v0.3.0` (`app/assets/{logo.svg,logo.png,firecraft.ico}`). | `L1.6` |
| **2** | **Performance is poor (WASM & desktop)** | **Two verified UI-thread stalls:** (1) `chrome.rs:299` opens `Optimize` by default, causing `optimize.rs:231-237` to synchronously run a full `1920×1080` `compose::render` + PNG deflate encode + `io::raster::decode` + texture upload just to draw a 128×128 thumbnail; (2) `canvas_ui.rs:1656` zooms on every mouse wheel tick, triggering synchronous `compose::render_scaled` (up to 16 MiPix) on the main thread. | `L1.10` |
| **3** | **Icons are not the requested Phosphor Icons** | `chrome.rs`, `panel_windows.rs`, `panels.rs`, and `welcome_overlay.rs` still draw programmatic `vector_icons.rs` glyphs (2,157 LOC) while `icons.rs` parses 57 Phosphor SVGs via `resvg` only in flyouts. Experiment `v0.3.0` embeds `Phosphor-Regular.ttf` (488 KB MIT font) directly into `egui::FontDefinitions` (`icons/phosphor.rs`), giving 100% crisp Phosphor icons with zero `resvg` WASM bloat. | Phase 2 (`L2.1`–`L2.6`) |
| **4** | **Canvas renders over other windows** | `crates/app/src/shell/canvas_ui.rs:617-780` paints the canvas shadow, texture (`image_rect`), selection frame, handles, and guides directly onto unclipped `ui.painter()` instead of `ui.painter().with_clip_rect(rect)`, bleeding over tabs, rulers, and footer when zoomed/scrolled; plus `Order::Foreground` overlays (`canvas_ui.rs:1710`, `mod.rs:3026`, `panel_windows.rs:789`, `panels.rs:5123`). | `L1.9` |
| **5** | **Keybindings & shortcuts broken** | `crates/app/src/shell/mod.rs:2837` guards shortcuts with `ctx.egui_wants_keyboard_input()`, which becomes `true` after clicking ANY tab, button, or combo box — silently killing every single-key shortcut for the rest of the session! Also missing global `Esc` focus surrender, `Shift+Tab`/`Tab`/`F11` split, `Alt+` menu mnemonics, tool-group cycling (`U`/`B`/`M`/`G`), and `Ctrl/Shift/Alt+Scroll` navigation. | `L1.11` |
| **6** | **Styles & Layers panels incomplete / poor UX** | `panels.rs:968-1061` (`styles_panel`) is a plain text list with zero rendered thumbnails (and leaves `constants.rs:58` `StyleLibrary` completely unused!); `panels.rs:528-688` (`layers_section`) lacks Blend Mode, Opacity `%`, indented child object tree, per-object Eye toggles, and live color swatches. | `L1.13` |
| **7** | **Menus poor & right-click context menu issues** | Plain `egui::MenuBar` lacks `Alt+` mnemonics/underlines, arrow navigation, and live document-derived `Text`/`Filters` submenus; `canvas_ui.rs:1789` uses an unstyled `egui::Popup` missing aligned shortcuts, edge clamping, Undo/Redo, Paste Attributes, Combine Paths/Align/Insert submenus, and View toggles. | `L1.12` |
| **8** | **Visually switching document tabs broken (always keeps first tab active)** | `crates/app/src/shell/chrome.rs:993` (`// Active document tab (index 0 visually)`) always draws `self.editor` in slot 0 and swaps parked tabs (`self.tabs`), instead of preserving stable tab order with `active_doc`, `tab_click_without_drag` (8 px slop), drag-to-swap, MRU `…` overflow, and bulk close. | `L1.8` |
| **9** | **Theme defaults to Dark instead of Auto / System** | `crates/app/src/shell/mod.rs:852` hardcodes `theme_mode: theme::ThemeMode::Dark` instead of `ThemeMode::System` (with cached OS detection and WCAG AA contrast unit tests). | `L1.7` |
| **10** | **Deep Experiment `v0.3.0` feature parity gaps & unwired stub fields** | Full 14-domain audit ([`2026-11-26-experiment-deep-audit.md`](2026-11-26-experiment-deep-audit.md)) catalogued missing features + 6 dead stub fields (`floating_doc_tabs`, `show_update_dialog`, `show_page_setup_dialog`, `StyleLibrary`, `PanelGroup` floating/stack fields, under-wired `cs6_color_button`): God View / Infinite Canvas, Multi-Monitor OS windows, Properties multi-select & inline gradient, Amber Diamond handles, Floating/Snapping/Stacking panels, 4-mode Find & Replace, `Intelligence` AI tab, Startup Splash, `Check for Updates`, Headless GUI Harness, and 4-platform Release CI. | `L1.13`–`L1.16`, Phase 2–3, Phase 7 |

---

## 2. Infrastructure & Repository Disconnects Audited (2026-11-26)

1. **Stale Nov 3 WASM deployed to `firecraft-web.pages.dev` (FIXED 2026-11-26):**
   - A prior session unzipped `firecraft-web-0.2.5.zip` (dated Nov 3, `f9dbe2f14305b2d9_bg.wasm`) into `dist/web` and deployed it to production.
   - **Fixed:** Wired `apply_web_studio_defaults()` in `web/src/main.rs` (`98812b17`), ran `xtask dist --web` cleanly on `fedora` (`firecraft-web-c8df704b0f48d240_bg.wasm`, 12.98 MB raw / 4.7 MB gzip, `smoke green`), deployed to `https://firecraft-web.pages.dev` (deployment `c2d3596a`), and visually verified via headless Chromium screenshot.
2. **Split-brain worktrees & stale branches (STILL OPEN — `L7.11`):**
   - `/nas/Temp/repos/FireCraft` remains on closed `feat/experiment-merge` (`7d622af5`, 35 dirty files) while `feat/phase1-visual-polish` lives in `.worktrees/integration-firecraft-1.0`.
   - PR `#188` remains open & `CONFLICTING`; 21 remote branches exist on `origin`; 31 GitHub issues remain open.
3. **Cloudflare Pages & GitHub Release automation gaps (STILL OPEN — Phase 6–8):**
   - GitHub Actions secret `CLOUDFLARE_API_TOKEN` was expired and deleted (causing `Deploy FireCraft marketing site` to fail); local `CLOUDFLARE_BUILD_TOKEN` is active.
   - Cloudflare Pages project `firecraft-site` has not been created yet.
   - `.github/workflows/release.yml` only triggers on `v*` tags, has `wrangler pages deploy` commented out (lines 428–431), and does not yet build/attach Windows or macOS binaries (contrast Experiment `v0.3.0`'s green 8-asset release workflow); `v0.2.5` remains GitHub `Latest` and `v0.2.6` has only a single `.wasm` asset.
