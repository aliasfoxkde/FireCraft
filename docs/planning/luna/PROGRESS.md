# FireCraft 1.0 — GPT-6-Luna Atomic Progress Tracker & Receipt Ledger

> **Instructions for GPT-6-Luna:**
> 1. Work strictly in order from the FIRST unchecked `[ ]` task below.
> 2. Read [`README.md`](README.md) first, then open ONLY the matching `PHASE-*.md` task pack.
> 3. Mark a task `[x]` ONLY after its exact `Verify` command succeeds with exit code `0` on the final commit (plus visual PNG inspection for UI/web/site tasks).
> 4. Append a 1-line row under **Execution Receipts** for every completed task.
> 5. **STOP immediately** at any `[HUMAN GATE]` or after 2 failed verify attempts on the same task.

---

## Completed Baseline (Neon Forensic Audit & Core Integration — 2026-11-26)
- [x] `BASE-1` `[NEON]` Forensic code, clean-room, CI, and visual screenshot audit (FireCraft vs `FireCraft-CraftAnything-Experiment`)
- [x] `BASE-2` `[NEON]` Extract & wire clean-room CS6 Studio front-end (`theme.rs`, `vector_icons.rs`, `chrome.rs`, `demo.rs`, `welcome_overlay.rs`, `panel_windows.rs`, 7 real panels in `panels.rs`) onto FireCraft's 17-crate engine
- [x] `BASE-3` `[NEON]` Fix CI & clean-room blockers (`constants.rs` comments scrubbed, `dist.rs` Windows `#[cfg(unix)]`, `iconsurvey.rs` prefix bug, `cargo fmt --all`, `clippy -D warnings`, `aegis` green)
- [x] `BASE-4` `[NEON]` Fix WASM optimization/compression regressions (`web/index.html` restored `data-wasm-opt="z"`, `xtask/src/dist.rs` fixed leading-dot `.gz` matching bug + regression tests)
- [x] `BASE-5` `[NEON]` Visual desktop screenshot proof (`1440x900` Welcome + Studio Workspace + `960x640` min viewport captured & visually inspected; refreshed `docs/img/app-main.png` & `site/assets/app-main.png`)
- [x] `BASE-6` `[NEON]` Build fresh Windows GUI review binary (`dist/windows-review/firecraft.exe`) from verified CS6 Studio UI branch
- [x] `BASE-7` `[NEON]` Author complete GPT-6-Luna roadmap, phased task packs (Phases A–D including Phosphor Icons integration, `T30` launchers, and `T31` binary optimization + modular folder split), and honest progress ledger

---

## Phase A — PR #189 Green CI, Phosphor Icons, UI Polish, WASM Proof, Feedback & Open Recent (`PHASE-A-CI-UI-AND-POLISH.md`)
- [ ] `L-A1` `[HUMAN GATE]` Verify PR `#189` full CI green across GitForge + GitHub (`fmt`, `wasm32`, `gate`, `macOS`, `Windows`, `CodeQL`) & merge to `main`
- [ ] `L-A2` `[LUNA]` Real-browser Playwright proof of CS6 Studio WASM build (`xtask dist --web` + `web/test.spec.ts` + `1440x900` & `960x640` browser screenshots)
- [ ] `L-A3` `[LUNA]` Unify & integrate **Phosphor Icons** across the entire GUI + fix the 3 visually-logged polish defects (`□` glyph boxes, top-right theme label clip, narrow `960x640` footer overlap)
- [ ] `L-A3b` `[LUNA]` Floating panel drag ghost preview, vertical stack grouping & edge snapping (`P2.6`)
- [ ] `L-A4` `[LUNA]` Build In-App Feedback dialog UI (`Help ▸ Send Feedback / Report Bug...`) over `diagnostic.rs` (`P8.2` + `P8.7`)
- [ ] `L-A5` `[LUNA]` Upgrade Command Palette (`Ctrl+P`) with user-language names, descriptions, aliases, categories, shortcuts & MRU
- [ ] `L-A6` `[LUNA]` Persisted `File ▸ Open Recent` (MRU + Clear Recent) + Pinned/Favorites document model & CLI launch args (`T30.1` + `T30.2`)
- [ ] `L-A7` `[LUNA]` Responsive/Dark/Light/A11y screenshot suite & fresh Windows review binary receipt (`P2.11` + `T29` Gate D)
- [ ] `L-A8` `[HUMAN GATE]` Maintainer Phase A visual & Windows 11 functional sign-off (`P2.12`)

---

## Phase B — Binary Optimization, Minification, Compression & Modular Folder Layout (`PHASE-B-OPTIMIZATION-AND-MODULAR-LAYOUT.md`)
- [ ] `L-B1` `[LUNA]` Reproducible WASM + native size & `twiggy` profiler baseline (`T31.1` -> `docs/planning/generated/size-baseline.md`)
- [ ] `L-B2` `[LUNA]` High-ROI WASM & binary optimization pass (build-time Phosphor atlas/font to drop runtime `resvg`/`usvg`/`fontdb`, font deduplication, web-only LTO/strip/`wasm-opt -Oz --strip-debug --strip-dwarf --strip-producers`) (`T31.2`)
- [ ] `L-B3` `[LUNA]` JS/HTML/CSS minification + identifier mangling + Brotli (`.br` q11) & Gzip (`.gz` l9) delivery pipeline in `xtask/src/dist.rs`
- [ ] `L-B4` `[LUNA]` Modular runtime folders: split out `examples/` and `libraries/` with lazy WASM fetch & desktop folder discovery
- [ ] `L-B5` `[LUNA]` Modular `extensions/` folder & versioned capability manifest (`firecraft-extension/v1`) + 2 real first-party extensions (`T31.4`)
- [ ] `L-B6` `[LUNA]` True lazy WASM feature module split (`dist/web/modules/<feature>.wasm`) proven via Playwright network waterfall (`T31.3`)
- [ ] `L-B7` `[LUNA]` Verify modular distribution folder layout (`bin/`, `examples/`, `libraries/`, `extensions/`) across native & web packages
- [ ] `L-B8` `[HUMAN GATE]` Maintainer review of before/after size report & modular architecture

---

## Phase C — Marketing Site, Docs, Live GitHub Wiki & OS Taskbar Launchers (`PHASE-C-DOCS-SITE-AND-WIKI.md`)
- [ ] `L-C1` `[LUNA]` Complete multi-page marketing site (`site/{index,features,download,roadmap,docs}.html`), multi-OS `app.js`, Playwright checks & `deploy-site.yml` (`P3.3` + `P3.7`)
- [ ] `L-C2` `[LUNA]` Deploy isolated Cloudflare Pages preview (`firecraft-site` `preview-v1`) & record rollback ID (`P3.8`)
- [ ] `L-C3` `[HUMAN GATE]` Maintainer approval for `firecraft-site` production promotion (`P3.9`)
- [ ] `L-C4` `[LUNA]` Reconcile `README.md`, `docs/`, `docs/wiki/`, and `.github/` templates (`P4.2`–`P4.4`, `P4.6`, `P4.8`)
- [ ] `L-C5` `[HUMAN GATE]` Publish live GitHub Wiki via `scripts/sync-wiki.sh --publish --confirm-push` (`P4.7`)
- [ ] `L-C6` `[LUNA]` OS-native Taskbar/Dock/Desktop launcher adapters (`T30.3` Windows Jump List, `T30.4` macOS Dock, `T30.5` Linux `.desktop`) respecting `unsafe_code = "forbid"`
- [ ] `L-C7` `[HUMAN GATE]` Maintainer Phase C review

---

## Phase D — Packaging Matrix, Issue/PR/Branch Closeout, Staging, Testers & `v1.0.0` Launch (`PHASE-D-PACKAGING-RELEASE-AND-CLOSEOUT.md`)
- [ ] `L-D1` `[HUMAN GATE]` Ratify `docs/decisions/0130-firecraft-1.0-release-scope.md` (superseding `0075` & `0070`) & confirm signing posture (`P5.1` + `P5.9`)
- [ ] `L-D2` `[LUNA]` Standardize canonical 1.0 asset naming, manifest schema & release self-tests (`P5.2`)
- [ ] `L-D3` `[LUNA]` Harden & verify Linux DEB, tar.gz, AppImage, Flatpak & conditional RPM (`P5.3`–`P5.6`)
- [ ] `L-D4` `[LUNA]` Harden Windows ZIP + Inno Setup (`-RequireInstaller`), macOS universal DMG (`.icns`), Web ZIP, Source, CycloneDX SBOM & Provenance (`P5.7`, `P5.8`, `P5.10`, `P5.11`)
- [ ] `L-D5` `[LUNA]` Execute multi-platform dry-run rehearsal (`workflow_dispatch`) & update `docs/release/ARTIFACT-MATRIX.md` (`P5.12`)
- [ ] `L-D6` `[HUMAN GATE]` Issue, PR & branch closeout (all 31 open issues, PRs `#186`–`#188`, obsolete branches/stashes) (`P7.6`–`P7.8`)
- [ ] `L-D7` `[LUNA]` Staging deployment (`firecraft-site` + `firecraft-web` RC bytes), external tester cohort & rollback rehearsal (`P9.1`–`P9.5`)
- [ ] `L-D8` `[HUMAN GATE]` Final `v1.0.0` Go / No-Go Gate (`P9.6`)
- [ ] `L-D9` `[LUNA]` Freeze `1.0.0` stamps/changelog/notes, push annotated `v1.0.0` tag, download & re-hash every draft asset (`P10.1`–`P10.4`)
- [ ] `L-D10` `[HUMAN GATE]` Publish GitHub `v1.0.0` Release, promote Cloudflare Pages from exact `v1.0.0` bytes & emit Final Launch Report (`P10.5`–`P10.9`)

---

## Execution Receipts

| Date | Task ID | Commit / Artifact | Verify Command & Observed Result |
|---|---|---|---|
| 2026-11-26 | `BASE-1..4` | `942bf239` (`integration/firecraft-1.0`) | `ssh fedora` full gate: `app` 278/278 ok, `xtask` 184/184 ok, `clippy -D warnings` 0 errors, `wasm32` ok, `layers`/`toolchain`/`panel-snapshot`/`reachability` (257/257)/`icon-survey`/`aegis` all ok (exit 0) |
| 2026-11-26 | `BASE-5` | `shot-{welcome,workspace,min-960x640}.png` | Real `xvfb-run` captures from `t5500` Debian-compatible Linux release binary visually inspected via `read_file`; CS6 Studio dark charcoal theme, rulers, 1080p showcase, 4 panel groups, live color picker, live Optimize preview confirmed; 3 minor polish items logged in `L-A3` |
| 2026-11-26 | `BASE-6` | `/nas/Temp/repos/FireCraft/dist/windows-review/firecraft.exe` | Built on `t5500` (`x86_64-pc-windows-gnu`, release) from `942bf239`: PE32+ GUI subsystem, 32,142,623 bytes, SHA-256 `8fdf2b6db032a84c9b0e0321081963fe71e558a86a638541b77638b4a42f78d6` |

## Blockers & Escalations
- None currently blocking Phase A entry (`L-A1`). Note: `ssh fedora` currently does not have `trunk`, `wasm-opt`, `twiggy`, or `brotli` on PATH (`which` checked 2026-11-26) — `L-A2` / `L-B1` should install them or run `xtask dist --web` on the CI/build container image that bakes them.
