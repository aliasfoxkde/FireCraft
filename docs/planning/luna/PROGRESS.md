# FireCraft 1.0 — GPT-6-Luna Atomic Progress Tracker & Receipt Ledger

> **Instructions for GPT-6-Luna:**
> 1. Work strictly in order from the FIRST unchecked `[ ]` task below.
> 2. Read [`README.md`](README.md), then open ONLY the matching `PHASE-<N>-*.md` file.
> 3. Mark `[x]` ONLY after the task's `Verify` command exits `0` on the final commit (plus visual PNG inspection for UI/icon/web/site tasks).
> 4. Append a 1-line row under **Execution Receipts** for every completed task.
> 5. **STOP immediately** before any `[HUMAN GATE]` or after 2 failed verify attempts.

---

## Completed Baseline (Neon Forensic Audit & Core Integration — 2026-11-26)
- [x] `BASE-1` `[NEON]` Forensic code, clean-room, CI, and visual screenshot audit (FireCraft vs `FireCraft-CraftAnything-Experiment`)
- [x] `BASE-2` `[NEON]` Extract & wire clean-room CS6 Studio UI (`theme.rs`, `vector_icons.rs`, `chrome.rs`, `demo.rs`, `welcome_overlay.rs`, `panel_windows.rs`, 7 real panels in `panels.rs`) onto FireCraft's 17-crate engine
- [x] `BASE-3` `[NEON]` Fix CI & clean-room blockers (`constants.rs`, Windows `#[cfg(unix)]` in `dist.rs`, `iconsurvey.rs`, `cargo fmt`, `clippy -D warnings`, `aegis`)
- [x] `BASE-4` `[NEON]` Fix WASM regressions (`web/index.html` restored `data-wasm-opt="z"`, `xtask/src/dist.rs` fixed `.gz` extension matching + regression unit tests)
- [x] `BASE-5` `[NEON]` Visual desktop screenshot proof (`1440x900` Welcome + Workspace + `960x640` min viewport inspected; refreshed `docs/img/app-{main,welcome}.png` & `site/assets/app-main.png`)
- [x] `BASE-6` `[NEON]` Built fresh Windows GUI review binary (`dist/windows-review/firecraft.exe`, PE32+ GUI, SHA-256 recorded below)
- [x] `BASE-7` `[NEON]` Authored 8-phase atomic GPT-6-Luna roadmap & task packs (`PHASE-1` through `PHASE-8`)

---

## Phase 1 — PR #189 CI, Visual Bugs & WASM Smoke (`PHASE-1-CI-AND-VISUAL-POLISH.md`)
- [ ] `L1.1` `[HUMAN GATE]` Verify PR `#189` full CI green & merge to `main`
- [ ] `L1.2` `[LUNA]` Fix top-right theme button clipping in menu bar (`chrome.rs`)
- [ ] `L1.3` `[LUNA]` Fix narrow-viewport (`960x640`) canvas footer overlap (`chrome.rs`)
- [ ] `L1.4` `[LUNA]` Eliminate missing-glyph `□` boxes in Layers & Tools footer + visual proof (`panels.rs`, `chrome.rs`)
- [ ] `L1.5` `[LUNA]` Real-browser Playwright smoke of CS6 Studio WASM build (`xtask dist --web` + `web/test.spec.ts`)

## Phase 2 — Full Phosphor Icons Unification (`PHASE-2-PHOSPHOR-ICONS.md`)
- [ ] `L2.1` `[LUNA]` Audit complete GUI icon inventory vs 57 shipped Phosphor SVGs (`phosphor-icon-map.md`)
- [ ] `L2.2` `[LUNA]` Vendor missing MIT Phosphor SVGs & register in `icons.rs` + `ATTRIBUTION.md` + `icon-survey`
- [ ] `L2.3` `[LUNA]` Wire Phosphor Icons into left Tools rail, tool flyouts & Tools footer
- [ ] `L2.4` `[LUNA]` Wire Phosphor Icons into Main Toolbar, Header bar, Tab View Modes & Canvas Footer
- [ ] `L2.5` `[LUNA]` Wire Phosphor Icons into Docked Panels, Iconic Mode rail, Align/Path & Welcome modal
- [ ] `L2.6` `[HUMAN GATE]` Dark + Light + `960x640` Phosphor Icon visual sign-off

## Phase 3 — Core UX Features & Windows Review (`PHASE-3-CORE-UX-FEATURES.md`)
- [ ] `L3.1` `[LUNA]` Floating panel drag ghost preview (`P2.6` Part 1)
- [ ] `L3.2` `[LUNA]` Floating panel vertical stack grouping & edge snapping (`P2.6` Part 2)
- [ ] `L3.3` `[LUNA]` Build In-App Feedback dialog UI (`Help ▸ Send Feedback / Report Bug...`) over `diagnostic.rs` (`P8.2`)
- [ ] `L3.4` `[LUNA]` Upgrade Command Palette (`Ctrl+P`) with user-language aliases, descriptions & MRU
- [ ] `L3.5` `[LUNA]` Persisted Recent & Pinned document model (`crates/app/src/recent.rs`, `T30.1`)
- [ ] `L3.6` `[LUNA]` Wire `File ▸ Open Recent`, Welcome recent list & `--new`/`--open` CLI args (`T30.1–T30.2`)
- [ ] `L3.7` `[LUNA]` Capture Phase 3 screenshot suite & build fresh Windows review binary (`T29` Gate D)
- [ ] `L3.8` `[HUMAN GATE]` Maintainer Windows 11 & Phase 3 UX sign-off (`P2.12`)

## Phase 4 — Binary & WASM Optimization, Minification & Brotli/Gzip (`PHASE-4-BINARY-AND-WASM-OPT.md`)
- [ ] `L4.1` `[LUNA]` Reproducible WASM + native size & `twiggy` baseline (`docs/planning/generated/size-baseline.md`, `T31.1`)
- [ ] `L4.2` `[LUNA]` Web-only Cargo profile tuning & `wasm-opt -Oz --strip-*` name/debug stripping (`T31.2`)
- [ ] `L4.3` `[LUNA]` Build-time Phosphor icon atlas/font to drop runtime `resvg`/`usvg`/`fontdb` from WASM (keeping Phosphor!)
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
- [ ] `L6.1` `[LUNA]` Build multi-page static marketing site (`site/{index,features,download,roadmap,docs}.html`, `P3.3`)
- [ ] `L6.2` `[LUNA]` Harden `site/app.js`, Playwright site audit & `deploy-site.yml` (`P3.7`)
- [ ] `L6.3` `[LUNA]` Deploy isolated Cloudflare Pages preview (`firecraft-site` `preview-v1`, `P3.8`)
- [ ] `L6.4` `[HUMAN GATE]` Maintainer approval for `firecraft-site` production promotion (`P3.9`)
- [ ] `L6.5` `[LUNA]` Refactor root `README.md` & reconcile 7 core docs (`P4.2`–`P4.3`)
- [ ] `L6.6` `[LUNA]` Update `docs/wiki/` pages & `.github/` contributor templates + `sync-wiki.sh --check` (`P4.4`, `P4.6`, `P4.8`)
- [ ] `L6.7` `[HUMAN GATE]` Publish live GitHub Wiki (`P4.7`)
- [ ] `L6.8` `[LUNA]` Linux `.desktop` launcher actions & XDG recent-docs integration (`T30.5`)
- [ ] `L6.9` `[LUNA]` Windows Jump List safe adapter investigation/implementation (`T30.3`)
- [ ] `L6.10` `[LUNA]` macOS Dock menu seam & platform receipt update (`T30.4`)

## Phase 7 — Packaging Matrix & Repo Closeout (`PHASE-7-PACKAGING-AND-REPO-CLOSEOUT.md`)
- [ ] `L7.1` `[HUMAN GATE]` Ratify 1.0 release scope decision (`0130`) & signing posture (`P5.1` + `P5.9`)
- [ ] `L7.2` `[LUNA]` Standardize 1.0 asset naming, manifest schema & release self-tests (`P5.2`)
- [ ] `L7.3` `[LUNA]` Verify Linux DEB & tarballs (`P5.3`)
- [ ] `L7.4` `[LUNA]` Harden & build Linux AppImage (`P5.4`)
- [ ] `L7.5` `[LUNA]` Harden Linux Flatpak (& conditional RPM per `0130`, `P5.5`–`P5.6`)
- [ ] `L7.6` `[LUNA]` Harden Windows x64 ZIP + Inno Setup (`-RequireInstaller`, `P5.7`)
- [ ] `L7.7` `[LUNA]` Harden macOS universal DMG (`.icns` + signing hooks, `P5.8`)
- [ ] `L7.8` `[LUNA]` Verify standalone Web ZIP, Source archive, CycloneDX SBOM & provenance (`P5.10`–`P5.11`)
- [ ] `L7.9` `[LUNA]` Run full multi-platform dry-run rehearsal & update `ARTIFACT-MATRIX.md` (`P5.12`)
- [ ] `L7.10` `[LUNA]` Post evidence comments on all 31 open issues & run `verify_release_ledger.sh` (`P7.6`)
- [ ] `L7.11` `[HUMAN GATE]` Maintainer approval for Issue `#1`, PRs `#186`–`#188` & safe branch/stash cleanup (`P7.7`–`P7.8`)

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
| 2026-11-26 | `BASE-1..4` | `942bf239` (`integration/firecraft-1.0`) | `ssh fedora`: `app` 278/278 ok, `xtask` 184/184 ok, `clippy -D warnings` 0 errors, `wasm32` ok, `layers`/`toolchain`/`panel-snapshot`/`reachability` (257/257)/`icon-survey`/`aegis` all ok |
| 2026-11-26 | `BASE-5` | `shot-{welcome,workspace,min-960x640}.png` | Real `xvfb-run` captures from `t5500` Debian-glibc release binary visually inspected; 3 minor polish items extracted into `L1.2`–`L1.4`; refreshed `docs/img/app-{main,welcome}.png` & `site/assets/app-main.png` |
| 2026-11-26 | `BASE-6` | `/nas/Temp/repos/FireCraft/dist/windows-review/firecraft.exe` | Built on `t5500` (`x86_64-pc-windows-gnu`, release) from `942bf239`: PE32+ GUI, 32,142,623 bytes, SHA-256 `8fdf2b6db032a84c9b0e0321081963fe71e558a86a638541b77638b4a42f78d6` |
| 2026-11-26 | `BASE-7` | `docs/planning/luna/` | 8-phase atomic GPT-6-Luna handoff (63 total atomic tasks, 1-goal/1-verify each) committed & Aegis-verified |

## Blockers & Escalations
- None blocking `L1.1`. Note: `ssh fedora` does not currently have `trunk`, `wasm-opt`, `twiggy`, or `brotli` on PATH (`which` checked 2026-11-26) — `L1.5` / `L4.1` should install pinned versions or run `xtask dist --web` inside the CI container image.
