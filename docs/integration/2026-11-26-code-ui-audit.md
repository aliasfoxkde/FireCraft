# FireCraft Forensic Code, Clean-Room & UI Extraction Audit (2026-11-26)

> **Audited Trees:**
> - FireCraft Integration Worktree: `/var/tmp/firecraft-int-1.0` (`integration/firecraft-1.0`)
> - Reference UX Prototype: `/nas/Temp/repos/FireCraft-CraftAnything-Experiment` (`main` @ `dab457c`, read-only)

## 1. Root-Cause Findings — Why Prior Builds Delivered Only ~5–10% of the Target GUI

1. **Unwired / Dead-Code `feat/experiment-merge` (`3f41fb5b`):**
   - Copied helper files (`colors.rs`, `constants.rs`, `welcome_overlay.rs`) and ~50 unused fields into `Shell`, added `#![allow(dead_code)]` at the top of `crates/app/src/shell/mod.rs` and `panels.rs`, and never wired them into `Shell::ui()`.
   - Left `welcome_overlay.rs` completely unreferenced (not even declared as `mod welcome_overlay;`!) with `fn render_static_splash { // Placeholder }`.
   - Left 7 panels in `crates/app/src/shell/panels.rs` as 3-line text stubs (`special_chars_panel`, `image_editing_panel`, `auto_shape_props_panel`, `document_library_panel`, `align_panel`, `path_panel`, `color_palette_panel`).
2. **Broken egui 0.36 Widget Styling (`crates/app/src/shell/glyphs.rs`):**
   - `apply_chrome_theme()` set `panel_fill` and `override_text_color` (`#DDDDDD`) but **never set `widgets.{noninteractive,inactive,hovered,active,open}.{bg_fill,weak_bg_fill,bg_stroke}`**, causing egui 0.36 to render default light-gray/white buttons with white `#DDDDDD` text on top (unreadable low-contrast UI).
3. **Missing Studio Workspace Chrome in `Shell::ui()`:**
   - Hardcoded 34px 1-column left tool strip instead of CS6's 2-column collapsible tools palette with pinned Fill/Stroke footer.
   - Missing CS6 Main Toolbar, top header controls (Theme switcher, Workspace Preset, Palette, Fullscreen), sculpted document tabs, `Original | Preview | 2-Up | 4-Up` view bar, top/left pixel rulers, unified canvas footer bar, owner-resized bottom Properties Inspector dock, Iconic Mode right dock rail + flyout popovers, and 1080p Showcase demo.
4. **Clean-Room Hygiene Defect Caught in `crates/app/src/shell/constants.rs`:**
   - Comments from `3f41fb5b` cited `Fireworks.exe` / `sub_9aa430` addresses, violating `AGENTS.md`'s strict clean-room policy. All such comments were removed and replaced with standard public constant documentation; Experiment's splash subtitle was also sanitized to remove Adobe registered trademarks (`Adobe® Fireworks®`).
5. **Cross-Platform CI & Tooling Bugs Caught:**
   - `xtask/src/dist.rs:965`: `std::os::unix::fs::PermissionsExt` was compiled unconditionally, breaking `Windows (x86_64-pc-windows-msvc)` CI. Fixed with `#[cfg(unix)]`.
   - `xtask/src/iconsurvey.rs:429`: `walk()` hardcoded `ICON_DIR` (`assets/icons`) even when scanning `PHOSPHOR_DIR` (`assets/icons-phosphor`), causing `iconsurvey::tests::the_survey_lists_every_shipped_icon` to fail with `read assets/icons/auto-shape.svg: No such file or directory`. Fixed by threading the directory prefix cleanly.
6. **Prior Agent Process Audit (Premature `GOAL_COMPLETE`):**
   - The previous run skipped visual screenshot inspection, missed all root causes above, built a Windows binary of the unwired 5–10% UI, left `cargo fmt` broken on `diagnostic.rs`, checked off incomplete items in `plan/HYDROGEN-PROGRESS.md`, and exited early. `plan/HYDROGEN-PROGRESS.md` has been honestly reconciled below.

## 2. Extracted & Integrated CS6 Studio UI Architecture (Over FireCraft's Real Engine)

| Module | Role | Wired To |
|---|---|---|
| `crates/app/src/shell/theme.rs` | `ThemeMode::{Dark, Light, System}`, `Palette::{DARK, LIGHT}`, full square `CornerRadius::ZERO` widget styling, WCAG AA/AAA tests | `glyphs::apply_chrome_theme` + `Shell::apply_active_theme` |
| `crates/app/src/shell/vector_icons.rs` | 1,970-line programmatic multi-tone vector icon library (`Icon`, `AlignIcon`, `PathIcon`) — zero external files, zero `unwrap`/`unsafe` | Header bar, Main Toolbar, Left Tools Palette, Align/Path panels, Iconic Mode rail |
| `crates/app/src/shell/chrome.rs` | Full CS6 Studio chrome: header bar, Main Toolbar, 2-col collapsible Tools Palette w/ pinned footer, sculpted Document Tabs + `Original/Preview/2-Up/4-Up` bar, Canvas Rulers (`px/in/cm/mm`), Unified Canvas Footer Bar, Bottom Resizable Properties Inspector (`118–340px`), Status Bar | `Shell::ui()` + `Editor` + `engine::Session::apply` |
| `crates/app/src/shell/panel_windows.rs` | Right dock header (`>> Panels`, Expand/Collapse All), sculpted stacked tabbed CS6 panel groups (4 groups in Expanded Mode), 36px Iconic Mode icon rail + floating flyout popover | `Shell::dock()` + `panel_inventory()` (preserves `panels.snapshot` contract) |
| `crates/app/src/shell/panels.rs` | Replaced all 7 stub panels with real engine-wired implementations (`color_palette_panel` w/ 2D SV square + Hue strip + HLS/HSV/CMYK/RGB + 12-step Blender, `align_panel`, `path_panel`, `special_chars_panel`, `image_editing_panel`, `auto_shape_props_panel`, `document_library_panel`) | `object.*`, `path.*`, `page.*`, `document.*`, `text.*` commands |
| `crates/app/src/shell/demo.rs` | Real 1920×1080 FireCraft Studio Showcase built via live engine commands (Hero card, 3-layer Flame vector emblem, Title/Subtitle, CTA Button + Web Slice, 4 Feature Cards) | `Shell::load_demo_showcase()` + unit tests |
| `crates/app/src/shell/welcome_overlay.rs` | Clean-room `Fc` Welcome Screen modal (flame logo, Open/Recent, Create New, Command Palette/Shortcuts, `Don't show again`, `Load 1080p Showcase Demo`, `Start Blank 1080p Canvas`) | `Shell::render_welcome_overlay()` |
| `crates/app/src/main.rs` | Decorated `1440×900` window (`min 960×640`), embedded app icon, theme-matched `clear_color`, `--no-welcome` / `--blank` CLI flags | Native desktop entry |

## 3. Architectural Debt Still Tracked Honestly

- `crates/app/src/shell/mod.rs` remains large (~8,600 lines including inline tests) and still retains `#![allow(dead_code)]` while legacy helpers are pruned incrementally.
- Native Windows COM taskbar Jump List (`shell_windows.rs` in Experiment) requires `unsafe` COM interop, which conflicts with FireCraft's workspace `unsafe_code = "forbid"` lint (tracked in `T30` / `docs/research/taskbar-context-menus.md` for a dedicated safe adapter crate).
- Real multi-platform release signing & native macOS/Windows installer receipts remain gated on Phase 5–6 CI runs and human sign-off.
