# FireCraft 1.0 — Consolidated Master Plan & UI Integration Roadmap

> **Authority Map:**
> - Historical architecture & `.fw.png` format specification: [`docs/PLANNING.md`](PLANNING.md)
> - Authoritative live task board: [`docs/TASKS.md`](TASKS.md)
> - Detailed phase criteria: [`docs/NEXT.md`](NEXT.md)
> - Forensic code, clean-room & Experiment UI extraction audit: [`docs/integration/2026-11-26-code-ui-audit.md`](integration/2026-11-26-code-ui-audit.md)
> - UI parity gap matrix: [`docs/integration/2026-11-26-ui-gap-matrix.md`](integration/2026-11-26-ui-gap-matrix.md)

## 1. Architectural Strategy

FireCraft combines two complementary strengths into one clean-room Rust workspace:
1. **FireCraft Core Engine (17 crates, L0–L5):** Real `.fw.png` parser/writer (`fwpng`), editable document model (`doc`), tile raster store (`raster`), vector/filter/text/ops engines, vello/skia compositor (`compose`), 257 registered commands with 100% shell reachability (`engine`), import/export codecs (`io`), sandboxed `.jsf` scripting (`scripting`), HTML/CSS/slice export (`webexport`), CLI/MCP (`app-cli`), and WASM embedding (`web/`).
2. **CS6 Studio Front-End (`crates/app/src/shell/`):** Extracted from `FireCraft-CraftAnything-Experiment`'s studio workspace design and wired directly over FireCraft's real `Editor` + `engine::Session::apply` command bus:
   - Crisp CS6 Charcoal Dark / Classic Light / System theme engine (`theme.rs`) with square `CornerRadius::ZERO`, dark widget fills, and WCAG AA/AAA contrast tests.
   - Programmatic multi-tone vector icon library (`vector_icons.rs`) + existing Phosphor SVG table.
   - Full CS6 desktop workspace chrome (`chrome.rs`): 10-menu top bar + header controls (Zoom, Palette, Fullscreen, Theme, Workspace Preset), Main Toolbar, 2-column collapsible Left Tools Palette with pinned non-clipping Fill/Stroke footer, Sculpted Document Tab Strip + `Original | Preview | 2-Up | 4-Up` view bar + Page selector, top/left Canvas Rulers (`px/in/cm/mm`), Unified Canvas Footer Bar (Format, Snap, Rulers, Guides, State Transport, Zoom, Size, DPI), owner-resized Bottom Properties Inspector Dock (`118–340px`), and crisp Bottom Status Bar.
   - Right stacked collapsible tabbed CS6 Panel Groups + 36px Iconic Mode strip with flyout popovers (`panel_windows.rs`) and real engine-wired panel bodies (`panels.rs` — zero placeholder stubs).
   - Clean-room CS6 Welcome Screen modal (`welcome_overlay.rs`) + real 1080p FireCraft Studio Showcase (`demo.rs`) built through live engine commands.

## 2. Release & Verification Sequence

1. **Consolidation & CI Gate Green (`integration/firecraft-1.0`, PR `#189`):** Fix cross-platform build breaks (`xtask/src/dist.rs` Windows `#[cfg(unix)]`, `xtask/src/iconsurvey.rs` path prefix), pass `cargo test`, `cargo clippy -- -D warnings`, `xtask` gates (`layers`, `toolchain`, `panel-snapshot`, `reachability`, `aegis`), and `wasm32-unknown-unknown`.
2. **Visual & Functional Proof:** Launch desktop binary under Xvfb, capture screenshots of Welcome modal, Expanded Mode showcase, Iconic Mode flyout, and Light theme, inspect visually, and produce a fresh Windows `firecraft.exe` review build on `t5500`.
3. **Marketing Site, Docs & Wiki Sync:** Self-hosted `site/` verified in Playwright desktop/mobile, reconciled root `README.md` / `docs/TASKS.md` / `docs/HANDOFF.md` / `docs/wiki/`, repeatable `scripts/sync-wiki.sh`.
4. **Packaging Matrix & Draft Release:** Harden Linux (DEB, AppImage, Flatpak, tarballs), Windows (portable ZIP + Inno Setup), macOS (universal DMG), WASM (`firecraft-web`), Source archive, CycloneDX SBOM & Provenance; verify clean-host installs and human sign-off before public `v1.0.0` promotion.
