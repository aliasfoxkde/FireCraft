# FireCraft — Production TODO

> **Authority:** `PRODUCTION-READINESS-PLAN.md`, `PRODUCTION-ROADMAP.md`, `TASKS.md`.
> **Last updated:** 2026-11-20.
> **Status:** active — ongoing polish and release verification.

---

## COMPLETED (2026-11-20)

### WASM CDN + Pages Deployment ✅
- Upload WASM to GitHub Release v0.2.6 (95.6 MB)
- Patch `index.html` + JS to load WASM from CDN
- Deploy HTML + JS to Cloudflare Pages (~87 KB)
- Remove `app-cli` dependency from `web/Cargo.toml`
- Add `version.json` with build provenance
- Verified at `head.firecraft-web.pages.dev`

---

## PRIORITY 0 — Blocks production

### CI: Fix wasm32 gate
- **Status:** BLOCKED
- **Issue:** `xtask ci` gate job uses rustc 1.99.1; coverage check on rustc 1.99.1 has an `.rmeta` version bug that prevents wasm32 from compiling. rustc 1.100.0 fails wasm32 builds (`couldn't create a temp dir`).
- **Fix needed:** either (a) install rustc 1.100.0 as an additional CI toolchain and run wasm32 only on it, or (b) skip wasm32 in `xtask ci` and gate it separately.
- **Status:** NOT STARTED

### Coverage floor ratchet
- **Current:** ~81% lines / ~79% functions
- **Target:** 90%+ before v1.0
- **Action:** target worst-coverage crates (`doc`, `compose`, `engine`), add tests for catalog refusal branches
- **Status:** NOT STARTED

### Missing `missing_docs` evaluation
- **Status:** PENDING (T15.4)
- **Action:** evaluate per-crate enablement in `xtask ci`

### God file reduction — `shell/mod.rs` (~7,993 lines)
- **Action:** extract `begin_text_edit`, `run_command`, `save_recovery`, panel management into dedicated modules

---

## PRIORITY 1 — User-visible polish

### Mask stack screenshot acceptance + sampling resolution
- **Issue:** T9 landed PARTIAL. Add/remove/move/dispose bound. Remaining: screenshot acceptance, mask sampling resolution.
- **Status:** IN PROGRESS

### Optimize panel — Adam7, selective-JPEG quality table, screenshot
- **Status:** PARTIAL — core panel landed, residuals open

### Animation timeline — loop count, thumbnails, per-side opacity
- **Status:** PARTIAL — strip/timing/onion/playback landed, residuals open

### Phase 22.4 — bitmap inspector fill/stroke + transform strip
- **Status:** blocked on `IMGv` fill-stroke node support

### Phase 22.5 — remaining CS6 chrome
- **Status:** PARTIAL — screenshot acceptance remains

---

## PRIORITY 2 — Release artifact verification

### Native Linux artifact from clean tree
- **Status:** NOT STARTED — build on clean Debian 12, verify install/launch/corpus/export

### Windows MSVC build + portable ZIP
- **Status:** BLOCKED — needs Windows build agent

### macOS DMG build
- **Status:** BLOCKED — needs macOS host

### AppImage build
- **Status:** BLOCKED — `linuxdeploy` absent on fleet

### Flatpak build
- **Status:** BLOCKED — `flatpak` absent

### Source archive — clean build from tag
- **Status:** BUILD VERIFIED but never published

### SBOM — validate + publish
- **Status:** BUILD VERIFIED but not on main (F-9)

### Release manifest — validate + publish
- **Status:** IMPLEMENTED but never published

### End-to-end release rehearsal
- **Status:** NOT STARTED — full `release.yml` run with all assets

### Provenance and signing
- **Status:** BLOCKED — needs signing identities (Authenticode, Developer ID, GPG)

---

## PRIORITY 3 — Security + quality

### Pin GitHub Actions to commit SHAs
- **Status:** NOT STARTED

### Add `cargo audit` to CI
- **Status:** NOT STARTED

### SBOM validation in CI
- **Status:** NOT STARTED

### `cast_possible_truncation` evaluation
- **Status:** 1,825 hits — refuse at current scale

---

## PRIORITY 4 — Documentation

### Refresh all planning docs from current state
- Regenerate `parity.md`, scorecard, changelog from `xtask` tools

### Decision ID deduplication
- `0094`×2, `0096`×2, `0101`×2 on `main`

### Phantom layer crate removal
- `layers.rs:26–37` registers `ops`, `gpu`, `automation`, `testkit` — none exist

### CodeRabbit marker cleanup
- `coderabbit-ai-generated` markers in `NEXT`, `HANDOFF`, `CS6-PARITY`, `PROGRESS`

---

## PRIORITY 5 — WASM size optimization

### Measure current WASM at `main`
- Current: 95.6 MB raw (2026-11-20 trunk build)

### Profile and trim
- Target: reduce toward 16 MB summit
- Remove unused eframe features, audit `app-cli`, remove unused embedded fonts/icons

### JS minification
- Measure JS glue size vs total bundle; add Terser/esbuild if material

---

## Open issues to triage

- **71 open issues** — classify each: open-valid, duplicate, fixed-on-main, blocked externally, cannot reproduce, or obsolete
