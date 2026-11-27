# FireCraft — Live Review Defects & Repo Disconnect Audit (2026-11-26)

> **Source:** Direct maintainer live review of `https://firecraft-web.pages.dev` and desktop build vs `/nas/Temp/repos/FireCraft-CraftAnything-Experiment`, plus forensic audit of git worktrees, Cloudflare Pages, and GitHub Actions.
> **Status:** All items below are logged as P0/P1 tasks in [`docs/planning/luna/PHASE-1-CI-AND-VISUAL-POLISH.md`](../planning/luna/PHASE-1-CI-AND-VISUAL-POLISH.md) (`L1.6`–`L1.13`) and [`PHASE-2-PHOSPHOR-ICONS.md`](../planning/luna/PHASE-2-PHOSPHOR-ICONS.md) (`L2.1`–`L2.6`).

---

## 1. Live Product & GUI Defects Reported by Maintainer

| # | Defect Reported | Verified Root Cause in FireCraft Source | Assigned Luna Task |
|---|---|---|---|
| **1** | **Wrong app icon / logo** | `crates/app/src/logo.png` (embedded in `welcome_overlay.rs` & `main.rs`) and `web/favicon.png` render a generic orange gradient circle rather than the approved FireCraft flame mark. | `L1.6` |
| **2** | **Performance is poor (WASM & desktop)** | `apply_studio_defaults()` / `apply_web_studio_defaults()` loads a full `1920x1080` canvas AND opens Group 4 (`Optimize`), which synchronously runs full-canvas CPU composition + PNG encoding (`108_188` bytes) on the UI thread; canvas rasterization lacks dirty-region / debounced updates and viewport-scaled preview caching. | `L1.10` |
| **3** | **Icons are not the requested Phosphor Icons** | `chrome.rs`, `panel_windows.rs`, `panels.rs`, and `welcome_overlay.rs` still draw programmatic `vector_icons.rs` glyphs (and `SW`/`D` ASCII labels) instead of unifying all chrome on MIT Phosphor Icons (`assets/icons-phosphor/*.svg` via `icons.rs`). | Phase 2 (`L2.1`–`L2.6`) |
| **4** | **Canvas renders over other windows** | Overlay/canvas painters using `egui::Order::Foreground` (e.g., `crates/app/src/shell/canvas_ui.rs:1710`, `mod.rs:3026`, `panel_windows.rs:789`, `panels.rs:5123`) paint above floating `egui::Window`s, menus, and dialogs. | `L1.9` |
| **5** | **Keybindings & shortcuts broken** | Shortcut routing, focus capture, `Tab` canvas-focus toggling, and single-key tool shortcuts diverge from `/nas/Temp/repos/FireCraft-CraftAnything-Experiment`. | `L1.11` |
| **6** | **Styles & Layers panels incomplete / poor UX** | `panels.rs` Styles & Layers layouts lack the visual swatch thumbnails, rich layer rows, hierarchy, drag-reorder, blend/opacity/masking polish present in `FireCraft-CraftAnything-Experiment`. | `L1.13` |
| **7** | **Menus poor & right-click context menu issues** | Top-level menu ergonomics and canvas/layer/tab right-click context menus have hit-testing, focus, and action gaps vs `FireCraft-CraftAnything-Experiment`. | `L1.12` |
| **8** | **Visually switching document tabs broken (always keeps first tab active)** | `crates/app/src/shell/chrome.rs:993` always renders the active document (`self.editor`) in visual slot 0 (`// Active document tab (index 0 visually)`) and swaps parked tabs (`self.tabs`), instead of preserving stable tab order with an `active_tab_idx`. | `L1.8` |
| **9** | **Theme defaults to Dark instead of Auto / System** | `crates/app/src/shell/mod.rs:852` hardcodes `theme_mode: theme::ThemeMode::Dark` instead of `ThemeMode::System`. | `L1.7` |
| **10** | **Feature parity gap vs `FireCraft-CraftAnything-Experiment`** | Experiment contains additional polished interactive behaviors, panel UX, shortcuts, and styling that must be systematically inventoried and ported cleanly. | `L1.13` + Phase 2–3 |

---

## 2. Infrastructure & Repository Disconnects Audited (2026-11-26)

1. **Stale Nov 3 WASM deployed to `firecraft-web.pages.dev` (FIXED 2026-11-26):**
   - A prior session unzipped `firecraft-web-0.2.5.zip` (dated Nov 3, `f9dbe2f14305b2d9_bg.wasm`) into `dist/web` and deployed it to production.
   - **Fixed:** Wired `apply_web_studio_defaults()` in `web/src/main.rs` (`98812b17`), ran `xtask dist --web` cleanly on `fedora` (`firecraft-web-c8df704b0f48d240_bg.wasm`, 12.98 MB raw / 4.7 MB gzip, `smoke green`), deployed to `https://firecraft-web.pages.dev` (deployment `c2d3596a`), and visually verified via headless Chromium screenshot.
2. **Split-brain worktrees & stale branches (STILL OPEN — `L7.11`):**
   - `/nas/Temp/repos/FireCraft` remains on closed `feat/experiment-merge` (`7d622af5`, 35 dirty files) while `feat/phase1-visual-polish` (`98812b17`) lives in `.worktrees/integration-firecraft-1.0`.
   - PR `#188` remains open & `CONFLICTING`; 21 remote branches exist on `origin`; 31 GitHub issues remain open.
3. **Cloudflare Pages & GitHub Release automation gaps (STILL OPEN — Phase 6–8):**
   - GitHub Actions secret `CLOUDFLARE_API_TOKEN` was expired and deleted (causing `Deploy FireCraft marketing site` run `45895429508` to fail); local `CLOUDFLARE_BUILD_TOKEN` is active.
   - Cloudflare Pages project `firecraft-site` has not been created (`code: 8000007`).
   - `.github/workflows/release.yml` only triggers on `v*` tags, has `wrangler pages deploy` commented out (lines 428–431), and does not yet build/attach Windows or macOS binaries; `v0.2.5` remains GitHub `Latest` and `v0.2.6` has only a single `.wasm` asset.
