# FireCraft 1.0 — Phase 1 Source-to-Result Reconciliation Report

> **Date:** 2026-11-26
> **Integration Branch:** `integration/firecraft-1.0` (PR `#189`)
> **Base:** `origin/main` (`4ff8f786`)

## Source-to-Result Mapping

| Source | Ref / SHA | Disposition | Target / Notes |
|---|---|---|---|
| **PR `#188`** | `docs/firecraft-v1-consolidation-handoff` | `SEPARATE_DOCS_PR` | Primary 1.0 plan & handoff; kept separate from code integration. |
| **PR `#187`** | `feat/experiment-merge` (`72318ae7`) | `INTEGRATED` | Shell split, 21 CS6 panels, Phosphor icons, `icon-survey`, Aegis re-baseline. |
| **PR `#186`** | `temp/consolidation` | `CHERRY_PICKED_CLEAN_ONLY` | Salvaged `xtask/src/provenance.rs`, `dist.rs`, `release.yml`, `validate_release_aggregate*`, `decision 0129`, `web/version.json`, `web/_redirects`, `web/test.spec.ts`, `playwright.config.js`, `wasm.rs`. **Rejected** ~118 MB binaries (`target/windows/`, `web/dist/`) & `log/devlog.md`. |
| **Worktree `feat/dev-roadmap-improvements`** | `17438502` | `INTEGRATED` | Single GUI event loop, Windows GUI subsystem, Help links, `T29`–`T31`, `Tutorials.md`, benchmarks. |
| **Branch `fix/xtask-gate-repair-2026-11-24`** | `6a78399f` | `INTEGRATED` | Updated `crates/app/panels.snapshot`. |
| **`stash@{2}`** | `591c5b82` | `INTEGRATED` | WASM scripting `cfg(feature)` gating (`crates/engine/src/platform.rs`, `crates/app`). |
| **CI Workflow Pins** | `.github/workflows/{ci-cross,ci}.yml` | `FIXED` | Fixed 39-char `actions/checkout` SHA, unpinned `actions/cache@v4`, and 44-char corrupted `actions/cache` SHA; pinned `toolchain: 1.99.1`. |
