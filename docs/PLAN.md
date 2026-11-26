# FireCraft 1.0 — Active Plan & Execution Entrypoint

> **Canonical Roadmap & GPT-6-Luna Execution Package:** [`docs/planning/luna/README.md`](planning/luna/README.md)
> **Atomic Progress & Receipt Ledger:** [`docs/planning/luna/PROGRESS.md`](planning/luna/PROGRESS.md)
> **Live Repository Task Board:** [`docs/TASKS.md`](TASKS.md)
> **Detailed Phase Backlog:** [`docs/NEXT.md`](NEXT.md) · [`docs/PRODUCT-UX-ROADMAP.md`](PRODUCT-UX-ROADMAP.md)
> **Master Specification:** [`docs/PLANNING.md`](PLANNING.md)

## Active Phase Order (Post-Audit 2026-11-26)

1. **Baseline Complete (Neon Audit & Core Fixes on `integration/firecraft-1.0`, PR `#189`):**
   - Full CS6 Studio UI/front-end extracted clean-room from `FireCraft-CraftAnything-Experiment` and wired onto FireCraft's real 17-crate engine (`theme.rs`, `vector_icons.rs`, `chrome.rs`, `demo.rs`, `welcome_overlay.rs`, `panel_windows.rs`, 7 real panels in `panels.rs`).
   - Clean-room comment audit (`constants.rs`), Windows MSVC CI fix (`dist.rs`), `iconsurvey.rs` fix, restored Trunk `data-wasm-opt="z"` in `web/index.html`, and fixed `.gz` sidecar matching in `xtask/src/dist.rs`.
   - Verified green on Fedora: `app` (278 tests), `xtask` (184 tests), `clippy -D warnings`, `wasm32`, `layers`, `toolchain`, `panel-snapshot`, `reachability` (257/257), `icon-survey`, and `aegis`.
2. **Phase A ([`PHASE-A-CI-UI-AND-POLISH.md`](planning/luna/PHASE-A-CI-UI-AND-POLISH.md)):** Land PR `#189` green CI, Playwright WASM browser proof, floating drag ghost/snap (`P2.6`), In-App Feedback dialog (`P8.2`), Command Palette discovery, persisted `Open Recent`/Pinned (`T30.1–T30.2`), and Windows 11 sign-off.
3. **Phase B ([`PHASE-B-OPTIMIZATION-AND-MODULAR-LAYOUT.md`](planning/luna/PHASE-B-OPTIMIZATION-AND-MODULAR-LAYOUT.md)):** `T31` + `0090` — `twiggy` size baseline, `wasm-opt -Oz` debug/name stripping, `egui_extras/svg` WASM diet, JS minification/mangling + Brotli `.br` & Gzip `.gz`, and splitting `examples/`, `libraries/`, `extensions/`, and lazy WASM feature `modules/` into clean runtime folders.
4. **Phase C ([`PHASE-C-DOCS-SITE-AND-WIKI.md`](planning/luna/PHASE-C-DOCS-SITE-AND-WIKI.md)):** Multi-page marketing site (`site/`) + Cloudflare preview, README/docs reconciliation, live GitHub Wiki sync, contributor templates, and OS-native Taskbar/Dock launchers (`T30.3–T30.5`).
5. **Phase D ([`PHASE-D-PACKAGING-RELEASE-AND-CLOSEOUT.md`](planning/luna/PHASE-D-PACKAGING-RELEASE-AND-CLOSEOUT.md)):** Scope decision (`0130`), full cross-platform packaging matrix, 31 issues / PRs / branches closeout, staging + external testers, and human-approved `v1.0.0` launch.
