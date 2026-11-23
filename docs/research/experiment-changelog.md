# Changelog

All notable changes to **FireCraft** are documented in this file.

## [0.2.1] — 2026-11-22

### Added
- **Dynamic CS6 Panel Grouping, Floating Windows & Magnetic Snapping:**
  - Every panel group now supports collapse/expand (`▾`/`▸` chevron or double-click header), bottom vertical resize splitters, draggable tabs (group onto another header, dock into a column, or tear off onto the canvas as a floating window), undock/pin (`⧉` / `📌`), and a `≡` Panel Options menu.
  - Dual-column right docking (`RightOuter` + `RightInner`), left docking (`LeftInner`), and floating windows (`DockZone::Floating`) with magnetic edge-to-edge snapping.
  - 5 new CS6 panels: **Color Mixer**, **Auto Shape Properties**, **Common Library**, **URL**, and **Behaviors** (21 panels total).
  - New workspace presets: `CS6 Dual-Column` and `All Panels Floating`.
- **Shortcuts & Navigation:**
  - `Insert` key centers and fits the page to the viewport (matching Fusion 360).
  - `F11` added alongside `Tab` and `F4` for Full-Screen Canvas mode.
- **Path & Subselection Audit:**
  - `combine_paths_op` now operates on multi-selection (`selected_set`) for `join`, `union`, `intersect`, `crop`, and `punch`.
  - `Tool::Subselect` supports dragging individual path vertices (`DragKind::MoveNode`), and `DragKind::Move` displays live smart alignment guides.
- **CI/CD:**
  - Added `.github/workflows/ci.yml` for automated formatting, Clippy (`-D warnings`), unit tests, and `wasm32-unknown-unknown` verification.

### Changed
- Removed the redundant `Fc` icon and badge next to `File` in the menu bar so menus align cleanly to the left.

### Fixed
- Cached `system_is_light()` in `theme.rs` with `OnceLock<bool>` and `CREATE_NO_WINDOW` on Windows, eliminating the UI freeze when switching to System/Auto theme.

## [0.2.0] — 2026-11-22

### Added
- Dual-target native (`x86_64-pc-windows-msvc`) and WebAssembly (`wasm32-unknown-unknown`) builds with GitHub Release automation (`.github/workflows/release.yml`).
- CS6 `Alt+<key>` menu mnemonics with underline indicators and full arrow-key menu navigation.
- `Ctrl+Scroll` cursor-anchored zoom, `Shift+Scroll` horizontal pan, `Scroll` vertical pan, and `Alt+Scroll` / Middle-Drag / `Space+Drag` 2D Hand Pan.
- OS clipboard image/text/multi-object Copy/Cut/Paste (`arboard`) and redesigned CS6 Welcome Splash Screen.
- Redesigned **Align** and **Path** panels with custom CS6 vector diagram icons, FireCraft flame branding, and `1920×1080 @ 144 ppi` default page size.
