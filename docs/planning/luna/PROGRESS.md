# FireCraft 1.0 — GPT-6-Luna Atomic Progress Tracker & Receipt Ledger

> **Instructions for GPT-6-Luna:**
> 1. Work strictly in order from the FIRST unchecked `[ ]` task below.
> 2. Read [`README.md`](README.md) and [`docs/integration/2026-11-26-experiment-deep-audit.md`](../../integration/2026-11-26-experiment-deep-audit.md), then open ONLY the matching `PHASE-<N>-*.md` file.
> 3. Mark `[x]` ONLY after the task's `Verify` command exits `0` on the final commit (plus visual PNG inspection for UI/icon/web/site tasks).
> 4. **NEVER** reuse or redeploy an old downloaded release zip. Always compile fresh (`xtask dist --web`) and verify the live deployed WASM hash matches the fresh build.
> 5. **STOP immediately** before any `[HUMAN GATE]` or after 2 failed verify attempts.

---

## Completed Baseline (Neon Forensic Audit & Core Integration — 2026-11-26)
- [x] `BASE-1` `[NEON]` Forensic code, clean-room, CI, and visual screenshot audit (FireCraft vs `FireCraft-CraftAnything-Experiment`)
- [x] `BASE-2` `[NEON]` Extract & wire clean-room CS6 Studio UI (`theme.rs`, `vector_icons.rs`, `chrome.rs`, `demo.rs`, `welcome_overlay.rs`, `panel_windows.rs`, 7 real panels in `panels.rs`) onto FireCraft's 17-crate engine
- [x] `BASE-3` `[NEON]` Fix CI & clean-room blockers (`constants.rs`, Windows `#[cfg(unix)]` in `dist.rs`, `iconsurvey.rs`, `cargo fmt`, `clippy -D warnings`, `aegis`)
- [x] `BASE-4` `[NEON]` Fix WASM regressions (`web/index.html` restored `data-wasm-opt="z"`, `xtask/src/dist.rs` fixed `.gz` extension matching + regression unit tests)
- [x] `BASE-5` `[NEON]` Visual desktop screenshot proof (`1440x900` Welcome + Workspace + `960x640` min viewport inspected; refreshed `docs/img/app-{main,welcome}.png` & `site/assets/app-main.png`)
- [x] `BASE-6` `[NEON]` Built fresh Windows GUI review binary (`dist/windows-review/firecraft.exe`, PE32+ GUI, SHA-256 recorded below)
- [x] `BASE-7` `[NEON]` Authored 8-phase atomic GPT-6-Luna roadmap, live defect audit (`2026-11-26-live-review-defects.md`), and deep 14-domain Experiment `v0.3.0` audit (`2026-11-26-experiment-deep-audit.md`)

---

## Phase 1 — CI Green, Live Review P0 Fixes & Core Experiment Parity (`PHASE-1-CI-AND-VISUAL-POLISH.md`)
- [x] `L1.1` `[HUMAN GATE]` Verify PR `#189` full CI green & merge to `main` (`5a5a1b2c`)
- [x] `L1.2` `[LUNA]` Fix top-right theme button clipping in menu bar (`84036eda`)
- [x] `L1.3` `[LUNA]` Fix narrow-viewport (`960x640`) canvas footer overlap (`3db7e682`)
- [x] `L1.4` `[LUNA]` Eliminate missing-glyph `□` boxes in Layers & Tools footer (`77a69bc4`)
- [x] `L1.5` `[NEON]` Fix `web/src/main.rs` Studio boot (`98812b17`), build fresh CS6 Studio WASM (`xtask dist --web`, 12.98 MB raw / 4.7 MB gzip), deploy live to `firecraft-web.pages.dev` (`c2d3596a`) & visually inspect live screenshot
- [x] `L1.6` `[LUNA]` **P0** Replace wrong app icon/favicon/Welcome logo with approved FireCraft flame artwork (`0e61e572`, `7b25a867`) — replaced `logo.png`, `favicon.png`, `firecraft-mark.png` with approved 512×512 flame PNG; fixed welcome_overlay badge colors from hard-coded `rgb(36,34,20)` to theme `p.banner`; blessed aegis baseline for logo.svg SVG path coords false positive. `fedora`: `cargo test -p app` (229 tests green), `cargo clippy -p app` clean, `xtask aegis` exit 0.
- [ ] `L1.7` `[LUNA]` **P0** Default theme mode to `Auto / System` (`ThemeMode::System` in `shell/mod.rs:852`), cache OS lookup (`OnceLock` + `CREATE_NO_WINDOW`) & add WCAG AA contrast unit tests
- [ ] `L1.8` `[LUNA]` **P0** Fix document tab switching visual active state, stable order, `tab_click_without_drag` (8 px), drag-to-swap ghost, MRU `…` overflow, raw wheel stepping & full tab context menu (`chrome.rs:993`)
- [ ] `L1.9` `[LUNA]` **P0** Fix canvas clipping (`painter.with_clip_rect(rect)` in `canvas_ui.rs:617-780`) & `Order::Foreground` Z-order so canvas never draws over chrome, floating windows, menus, or dialogs
- [ ] `L1.10` `[LUNA]` **P0** Fix canvas & WASM main-thread perf stalls (stop sync 1920×1080 Optimize PNG encode+decode in `optimize.rs:231-237`; debounce zoom re-rasterization & reuse cached GPU texture)
- [ ] `L1.11` `[LUNA]` **P0** Fix keybindings & focus (`text_edit_focused()` replacing `egui_wants_keyboard_input()` at `mod.rs:2837`, global `Esc`, `Shift+Tab`/`Tab`/`F11`, tool cycling `U`/`B`/`M`/`G`, `Ctrl/Shift/Alt+Scroll` & pinch zoom)
- [ ] `L1.12` `[LUNA]` **P0** Upgrade top menus (`Alt+` mnemonics & arrow nav, live `Text`/`Filters` submenus) & styled edge-clamped CS6 canvas right-click context menu
- [ ] `L1.13` `[LUNA]` **P0** Upgrade Styles panel (wire 6 `StyleLibrary` sets + 34×34 gradient/bevel/glow/shadow thumbnails) & Layers panel (Blend Mode, Opacity `%`, indented object tree w/ Eye + 10×10 color swatches)
- [ ] `L1.14` `[LUNA]` **P0** Upgrade Bottom Properties Inspector (multi-selection bar, inline gradient & live filters, internal scroll) & wire `cs6_color_button` 18-col Swatch Matrix everywhere
- [ ] `L1.15` `[LUNA]` **P0** Implement God View / Infinite Canvas (`∞ Canvas`, `Ctrl+Shift+F`), fractional `px`/`in`/`cm`/`mm` rulers, elided below-slice URL tabs & first-frame showcase fit
- [ ] `L1.16` `[LUNA]` **P0** Polish CS6 Tools palette (tinted bands, clickable flyout triangles, auto-compacting `Sides`/`Star %`), add interactive Amber Diamond canvas handles, verify clipboard/SVG drop & merge `feat/phase1-visual-polish` -> `main`

## Phase 2 — Full Phosphor Icons Unification (`PHASE-2-PHOSPHOR-ICONS.md`)
- [ ] `L2.1` `[LUNA]` Audit complete GUI icon inventory & Phosphor Regular codepoint map (`phosphor-icon-map.md`)
- [ ] `L2.2` `[LUNA]` Embed MIT `Phosphor-Regular.ttf` font (488 KB, matching Experiment `v0.3.0`), add `phosphor.rs` map, register native + WASM, update `ATTRIBUTION.md` & `icon-survey`
- [ ] `L2.3` `[LUNA]` Wire Phosphor Icons into left Tools rail, tool flyouts & Tools footer (replacing programmatic icons & ASCII labels)
- [ ] `L2.4` `[LUNA]` Wire Phosphor Icons into Main Toolbar, Header bar, Tab View Modes & Canvas Footer
- [ ] `L2.5` `[LUNA]` Wire Phosphor Icons into Docked Panels, Iconic Mode rail, Layers/Align/Path & Welcome modal
- [ ] `L2.6` `[HUMAN GATE]` Dark + Light + `960x640` Phosphor Icon visual sign-off

## Phase 3 — Core UX Features, Full Experiment `v0.3.0` Parity & Windows Review (`PHASE-3-CORE-UX-FEATURES.md`)
- [ ] `L3.1` `[LUNA]` Floating panel `egui::Area` windows, translucent drag ghost facsimile, 1-click collapse chevron fix, resize splitters & `»` tab overflow (`P2.6` Part 1)
- [ ] `L3.2` `[LUNA]` Pure `panel_geometry.rs` (12 px magnetic snap + 28 px column align + vertical stacking), floating Tools/Panels grip bars & all 6 CS6 workspace presets (`P2.6` Part 2)
- [ ] `L3.3` `[LUNA]` Build In-App Feedback dialog UI (`Help ▸ Send Feedback / Report Bug...`) over `diagnostic.rs` (`P8.2`)
- [ ] `L3.4` `[LUNA]` Supercharge Command Palette (`Ctrl+K`/`Ctrl+P`) with 7 categories (panels, filters, widgets, `find:<text>` object jump, `replace:old->new`, aliases, MRU)
- [ ] `L3.5` `[LUNA]` Persisted Recent & Pinned document model (`crates/app/src/recent.rs`, `T30.1`)
- [ ] `L3.6` `[LUNA]` Wire `File ▸ Open Recent`, `Save a Copy`/`Save All`/`Revert`/`Print`/`Preview in Browser`, Welcome recents & `--new`/`--open`/`--size`/`--state` CLI args (`T30.1–T30.2`)
- [ ] `L3.7` `[LUNA]` Wire Multi-Monitor Floating OS Canvas Windows (`show_viewport_immediate`), dirty bulk-close queue (`pending_close_tabs`) & Quit Confirm (`allow_close` gate)
- [ ] `L3.8` `[LUNA]` Standardized dialog grammar, 4-mode Find & Replace (`Text`/`Fill`/`Stroke`/`Any Color`), `HTML Setup`/`Page Setup`, Startup Splash, `Check for Updates` & redesigned `About`
- [ ] `L3.9` `[LUNA]` Add `Intelligence` bottom-dock tab (OpenAI-compatible AI + `Copy MCP config`), interactive MCP Console & Headless GUI Click / 500-Object Frame-Budget Harness (`gui_harness.rs` + `gui-suite.sh`)
- [ ] `L3.10` `[LUNA]` Capture Phase 3 6-state screenshot suite & build fresh Windows review binary (`T29` Gate D)
- [ ] `L3.11` `[HUMAN GATE]` Maintainer Windows 11 & Phase 3 UX sign-off (`P2.12`)

## Phase 4 — Binary & WASM Optimization, Minification & Brotli/Gzip (`PHASE-4-BINARY-AND-WASM-OPT.md`)
- [ ] `L4.1` `[LUNA]` Reproducible WASM + native size & `twiggy` baseline (`docs/planning/generated/size-baseline.md`, `T31.1`)
- [ ] `L4.2` `[LUNA]` Web-only Cargo profile tuning & `wasm-opt -Oz --strip-*` name/debug stripping (`T31.2`)
- [ ] `L4.3` `[LUNA]` Remove runtime `resvg`/`usvg`/`fontdb` icon dependency from WASM (enabled by Phase 2's embedded `Phosphor-Regular.ttf` font!)
- [ ] `L4.4` `[LUNA]` Embedded font deduplication audit (`default_fonts` vs `LiberationSans`)
- [ ] `L4.5` `[LUNA]` JS glue & HTML minification + local identifier mangling in `stage_web()`
- [ ] `L4.6` `[LUNA]` Brotli (`.br` q11) + Gzip (`.gz` l9) pre-compression & `web/_headers` verification
- [ ] `L4.7` `[LUNA]` Native desktop binary strip & size benchmark (Linux & Windows)
- [ ] `L4.8` `[HUMAN GATE]` Maintainer size optimization report sign-off

## Phase 5 — Modular Folders & Lazy WASM Modules (`PHASE-5-MODULAR-FOLDERS-AND-EXTENSIONS.md`)
- [ ] `L5.1` `[LUNA]` Create modular `examples/` folder with 4 editable `.fw.png` samples & `manifest.json`
- [ ] `L5.2` `[LUNA]` Create modular `libraries/` folder (`styles/`, `symbols/`, `swatches/`) & `manifest.json`
- [ ] `L5.3` `[LUNA]` Implement lazy desktop + WASM runtime resource loader (`resources.rs`) & `File ▸ Open Example`
- [ ] `L5.4` `[LUNA]` Define sandboxed `extensions/` contract (`0131`) & capability-enforcing loader (`T31.4` Part 1)
- [ ] `L5.5` `[LUNA]` Ship 2 real first-party extensions (`css-design-tokens`, `batch-social-presets`) & manager dialog (`T31.4` Part 2)
- [ ] `L5.6` `[LUNA]` Extract 1 true lazy WASM feature module (`dist/web/modules/`) proven by Playwright waterfall (`T31.3`)
- [ ] `L5.7` `[LUNA]` Stage modular folder hierarchy (`bin/`, `examples/`, `libraries/`, `extensions/`) in all native & web dist scripts
- [ ] `L5.8` `[HUMAN GATE]` Maintainer modular architecture & lazy-load sign-off

## Phase 6 — Site, Docs, Live Wiki & OS Taskbar Launchers (`PHASE-6-SITE-DOCS-WIKI-AND-LAUNCHERS.md`)
- [ ] `L6.1` `[LUNA]` Build multi-page static marketing site (`site/{index,features,download,roadmap,docs,extensions}.html`, `P3.3`)
- [ ] `L6.2` `[LUNA]` Harden `site/app.js`, Playwright site audit & `deploy-site.yml` (`P3.7`)
- [ ] `L6.3` `[LUNA]` Create `firecraft-site` Cloudflare Pages project, refresh GitHub secret from active token & deploy preview (`P3.8`)
- [ ] `L6.4` `[HUMAN GATE]` Maintainer approval for `firecraft-site` production promotion (`P3.9`)
- [ ] `L6.5` `[LUNA]` Refactor root `README.md`, add `docs/A11Y.md` & reconcile 7 core docs (`P4.2`–`P4.3`)
- [ ] `L6.6` `[LUNA]` Update `docs/wiki/` pages (including 5 user tutorials from Experiment) & `.github/` contributor templates + `sync-wiki.sh --check` (`P4.4`, `P4.6`, `P4.8`)
- [ ] `L6.7` `[HUMAN GATE]` Publish live GitHub Wiki (`P4.7`)
- [ ] `L6.8` `[LUNA]` Linux `.desktop` launcher actions & XDG recent-docs integration (`T30.5`)
- [ ] `L6.9` `[LUNA]` Windows Jump List safe adapter investigation/implementation (`T30.3`, respecting `unsafe_code = "forbid"`)
- [ ] `L6.10` `[LUNA]` macOS Dock menu seam & platform receipt update (`T30.4`)

## Phase 7 — Packaging Matrix & Repo Closeout (`PHASE-7-PACKAGING-AND-REPO-CLOSEOUT.md`)
- [ ] `L7.1` `[HUMAN GATE]` Ratify 1.0 release scope decision (`0130`) & signing posture (`P5.1` + `P5.9`)
- [ ] `L7.2` `[LUNA]` Standardize 1.0 asset naming, manifest schema & release self-tests (`P5.2`, referencing Experiment `v0.3.0`'s green `release.yml`)
- [ ] `L7.3` `[LUNA]` Verify Linux DEB & tarballs (`P5.3`)
- [ ] `L7.4` `[LUNA]` Harden & build Linux AppImage (`P5.4`)
- [ ] `L7.5` `[LUNA]` Harden Linux Flatpak (& conditional RPM per `0130`, `P5.5`–`P5.6`)
- [ ] `L7.6` `[LUNA]` Harden Windows x64 ZIP + Inno Setup (`-RequireInstaller`) & wire into `release.yml` (`P5.7`)
- [ ] `L7.7` `[LUNA]` Harden macOS universal DMG (`.icns` + `lipo` + signing hooks) & wire into `release.yml` (`P5.8`)
- [ ] `L7.8` `[LUNA]` Verify standalone Web ZIP, Source archive, CycloneDX SBOM & provenance (`P5.10`–`P5.11`)
- [ ] `L7.9` `[LUNA]` Run full multi-platform dry-run rehearsal & update `ARTIFACT-MATRIX.md` (`P5.12`)
- [ ] `L7.10` `[LUNA]` Post evidence comments on all 31 open issues & run `verify_release_ledger.sh` (`P7.6`)
- [ ] `L7.11` `[HUMAN GATE]` Maintainer approval for Issue `#1`, closing conflicting PR `#188`, reconciling `/nas/Temp/repos/FireCraft` onto `main`, and pruning the 21 stale remote branches (`P7.7`–`P7.8`)

## Phase 8 — Staging, Testers & `v1.0.0` Launch (`PHASE-8-STAGING-TESTERS-AND-V1-LAUNCH.md`)
- [ ] `L8.1` `[LUNA]` Deploy isolated staging (`firecraft-site` + `firecraft-web` RC bytes) & prove hash match (`P9.1`)
- [ ] `L8.2` `[HUMAN GATE]` External tester cohort validation & P0/P1 burn-down (`P9.2`–`P9.4`)
- [ ] `L8.3` `[LUNA]` Rehearse rollback procedures (Cloudflare Pages & GitHub Releases, `P9.5`)
- [ ] `L8.4` `[HUMAN GATE]` Final `v1.0.0` Go / No-Go authorization (`P9.6`)
- [ ] `L8.5` `[LUNA]` Freeze `1.0.0` stamps, tag `v1.0.0`, build draft release & re-hash every asset (`P10.1`–`P10.4`)
- [ ] `L8.6` `[HUMAN GATE]` Publish GitHub `v1.0.0` release & promote Cloudflare Pages (`P10.5`–`P10.6`)
- [ ] `L8.7` `[LUNA]` Independent public post-launch verification & final launch report (`P10.7`–`P10.9`)

---

## Execution Receipts

| Date | Task ID | Commit / Artifact | Verify Command & Observed Result |
|---|---|---|---|
| 2026-11-26 | `BASE-1..7` | `ac2c840d` (`integration/firecraft-1.0`) | Full CS6 Studio extraction, clean-room/CI/WASM fixes, desktop screenshots, Windows review `.exe` (`8fdf2b6d...`), and 8-phase Luna handoff verified on `fedora` |
| 2026-11-26 | `L1.1` | `5a5a1b2c` (PR `#189` squash merge) | All GitHub required checks passed; merged into `main`; PRs `#186` & `#187` closed |
| 2026-11-26 | `L1.2`–`L1.4` | `84036eda`, `3db7e682`, `77a69bc4` | Top-right theme clip, `960x640` footer overlap, and `□` boxes fixed & visually inspected on `feat/phase1-visual-polish` |
| 2026-11-26 | `L1.5` | `98812b17` / Pages `c2d3596a` | Fixed `web/src/main.rs` Studio boot + native clippy/MCP tests; `fedora` `app --features native` (350+ tests), `app-cli`, `xtask`, `clippy -D warnings`, `wasm32`, `aegis` all exit 0; `xtask dist --web` (`c8df704b0f48d240_bg.wasm`, 12.98 MB raw / 4.7 MB gz, `smoke green`); deployed live to `https://firecraft-web.pages.dev` & visually verified via Chromium screenshot |
| 2026-11-26 | `AUDIT-EXP` | `docs/integration/2026-11-26-experiment-deep-audit.md` | Deep 14-domain module-by-module read-only audit of `FireCraft-CraftAnything-Experiment` (`v0.3.0` `e415a33` / `HEAD` `6233a49`) vs FireCraft (`17dff0fc`); identified exact source root causes (unclipped `canvas_ui.rs:634`, sync Optimize PNG encode+decode `optimize.rs:231-237`, `egui_wants_keyboard_input()` at `mod.rs:2837`, 6 dead stub fields) & expanded Luna Phases 1–3 with full Experiment feature parity |

## Active Notes & Escalations
- **Maintainer status (2026-11-26):** Deep 14-domain audit of `FireCraft-CraftAnything-Experiment` (`v0.3.0`) vs FireCraft completed and mapped into `L1.6`–`L1.16`, `L2.1`–`L2.6`, and `L3.1`–`L3.11` ([`2026-11-26-experiment-deep-audit.md`](../../integration/2026-11-26-experiment-deep-audit.md)). Maintainer plans to focus on `/nas/Temp/repos/FireCraft-CraftAnything-Experiment` next; FireCraft's complete audit, receipts, and atomic roadmap are preserved cleanly here when work resumes.
- **`fedora` WASM tooling resolved:** Static musl `trunk 0.21.14` is now installed at `fedora:~/.local/bin/trunk` and `xtask dist --web` passes natively with `chromium-browser`.
- **Cloudflare auth note:** Local `CLOUDFLARE_BUILD_TOKEN` is active; GitHub Actions `CLOUDFLARE_API_TOKEN` was removed after returning an authentication error and needs updating before `deploy-site.yml` / automated Pages deploys can succeed.
