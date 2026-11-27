# FireCraft 1.0 — Active Plan & Execution Entrypoint

> **Canonical 8-Phase Roadmap & GPT-6-Luna Handoff:** [`docs/planning/luna/README.md`](planning/luna/README.md)
> **Atomic Progress & Receipt Ledger:** [`docs/planning/luna/PROGRESS.md`](planning/luna/PROGRESS.md)
> **Live Repository Task Board:** [`docs/TASKS.md`](TASKS.md)
> **Detailed Phase Backlog:** [`docs/NEXT.md`](NEXT.md) · [`docs/PRODUCT-UX-ROADMAP.md`](PRODUCT-UX-ROADMAP.md)
> **Master Specification:** [`docs/PLANNING.md`](PLANNING.md)

## 8-Phase Atomic Execution Order (Post-Audit 2026-11-26)

- **Baseline Complete (Neon on `integration/firecraft-1.0`, PR `#189`):** Clean-room CS6 Studio UI integrated & visually verified, CI/clean-room blockers fixed, `web/index.html` `data-wasm-opt="z"` restored, `xtask/src/dist.rs` `.gz` bug fixed, and fresh Windows review binary built (`dist/windows-review/firecraft.exe`).
- **Phase 1 ([`PHASE-1-CI-AND-VISUAL-POLISH.md`](planning/luna/PHASE-1-CI-AND-VISUAL-POLISH.md)):** PR `#189` merged (`5a5a1b2c`), `L1.2`–`L1.5` done (fresh 12.98 MB CS6 Studio WASM live on `firecraft-web.pages.dev`), plus P0 live-review fixes `L1.6`–`L1.13` (flame app icon, `ThemeMode::System` default, stable document tab switching, canvas Z-order over windows, WASM/canvas perf & deferred Optimize encode, Experiment keybindings/focus, menus/right-click, Styles & Layers parity — see [`docs/integration/2026-11-26-live-review-defects.md`](integration/2026-11-26-live-review-defects.md)).
- **Phase 2 ([`PHASE-2-PHOSPHOR-ICONS.md`](planning/luna/PHASE-2-PHOSPHOR-ICONS.md)):** Audit, vendor, and unify MIT **Phosphor Icons** across every tool, toolbar, tab, footer, panel, and modal + Dark/Light visual sign-off (`L2.1`–`L2.6`).
- **Phase 3 ([`PHASE-3-CORE-UX-FEATURES.md`](planning/luna/PHASE-3-CORE-UX-FEATURES.md)):** Drag ghost & edge snap (`P2.6`), In-App Feedback dialog (`P8.2`), Command Palette discovery, persisted `Open Recent`/Pinned + CLI args (`T30.1–T30.2`), Windows 11 review (`L3.1`–`L3.8`).
- **Phase 4 ([`PHASE-4-BINARY-AND-WASM-OPT.md`](planning/luna/PHASE-4-BINARY-AND-WASM-OPT.md)):** `twiggy` size baseline (`T31.1`), web profile + `wasm-opt -Oz --strip-*`, build-time Phosphor atlas/font (sheds runtime `resvg` WASM bloat while keeping Phosphor!), font audit, JS minification/mangling, Brotli `.br` + Gzip `.gz` (`L4.1`–`L4.8`).
- **Phase 5 ([`PHASE-5-MODULAR-FOLDERS-AND-EXTENSIONS.md`](planning/luna/PHASE-5-MODULAR-FOLDERS-AND-EXTENSIONS.md)):** Split `examples/`, `libraries/`, desktop+WASM lazy loader, `extensions/` contract + 2 real extensions (`T31.4`), true lazy WASM `modules/` (`T31.3`), modular archive layout (`L5.1`–`L5.8`).
- **Phase 6 ([`PHASE-6-SITE-DOCS-WIKI-AND-LAUNCHERS.md`](planning/luna/PHASE-6-SITE-DOCS-WIKI-AND-LAUNCHERS.md)):** Multi-page `site/` + preview deploy, `README.md`/core docs, `docs/wiki/` + templates + live wiki sync, OS-native Linux/Windows/macOS taskbar launchers (`T30.3–T30.5`) (`L6.1`–`L6.10`).
- **Phase 7 ([`PHASE-7-PACKAGING-AND-REPO-CLOSEOUT.md`](planning/luna/PHASE-7-PACKAGING-AND-REPO-CLOSEOUT.md)):** Scope decision `0130`, Linux/Windows/macOS/Web/Source/SBOM packaging, rehearsal, 31 issues + PRs `#186`–`#188` + branch closeout (`L7.1`–`L7.11`).
- **Phase 8 ([`PHASE-8-STAGING-TESTERS-AND-V1-LAUNCH.md`](planning/luna/PHASE-8-STAGING-TESTERS-AND-V1-LAUNCH.md)):** Staging + hash proof, external testers + P0/P1 burn-down, rollback rehearsal, `v1.0.0` Go/No-Go, tag/re-hash, publish + launch report (`L8.1`–`L8.7`).
