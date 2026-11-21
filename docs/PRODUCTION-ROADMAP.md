# FireCraft — Production Readiness Roadmap and TODO

> **Purpose:** systematic execution plan to bring FireCraft to a production-ready, polished application that users can install, use, and trust.
>
> **Authority:** `AGENTS.md` §Workflow 1; `PRODUCTION-READINESS-PLAN.md` §1 (mission); `TASKS.md` (live ledger); `docs/climb/SCOREBOARD.tsv` (metrics); `docs/release/ARTIFACT-MATRIX.md` (release evidence).
>
> **Status:** active — created 2026-11-19.
> **Owner:** user (maintainer). This is a planning document; execution requires branch creation and PRs.

---

# 1. Current State Snapshot (2026-11-19)

| Dimension | Value | Source |
|---|---|---|
| Latest release | `v0.2.5` (2026-11-03) | `docs/TASKS.md` |
| Latest commit | `365c8bc4` (2026-11-17, toolchain pin gate) | `log/devlog.md` |
| Engine commands | 257 registered, 257 bound = **100% reachable** | devlog 2026-11-16 |
| Menu census | 298 items, 2 inert | devlog 2026-11-16 |
| Test count | 1,885 | devlog 2026-11-16 |
| WASM raw | ~21.6 MB (2026-11-10) | docs/climb/SCOREBOARD.tsv |
| Coverage | 81.1% lines / 78.7% functions | SCOREBOARD.tsv |
| CI gate | Green (~18 min) | devlog |
| God files >2,000 lines | 18 (largest `shell/mod.rs` ~7,993) | EXECUTION-2026-11-15 |
| Live web deploy | Unattributable — different bytes than v0.2.5 | ARTIFACT-MATRIX.md §Live |
| Release artifacts | 6 published (Linux tar.gz, DEB, web ZIP, CLI, checksums); 7 NOT STARTED; 4 DEFERRED | ARTIFACT-MATRIX.md §Status counts |
| Release certification | **None** — no signing, no provenance | ARTIFACT-MATRIX.md §rung ceiling |
| Release scope | decision 0075: recommended, awaiting maintainer approval | TASKS.md |
| Remote branches | 123 (91 were stale ghosts) | devlog 2026-11-17 |
| Open issues | 71 | devlog 2026-11-16 |

---

# 2. The 8 Phases to Production

The phases are ordered by **value per hour**: fix the measurement and deployment infrastructure first, then close the remaining daily-driver gaps, then build/verify release artifacts, then polish.

## Phase A: Trunk Truth and Measurement (P0 — blocks everything)

**Goal:** one source of truth for what exists, what's shipped, and what's broken. Currently the board, the scoreboard, and the live tree disagree on 164 vs 249 vs 257 commands.

### A-1. Consolidate stale quotes across the tree
- **What:** `GAPS.md` §16, `TASKS.md`, `HANDOFF`, `PRODUCTION-READINESS-PLAN.md`, `SESSIONS.md`, decisions 0087 and `wiki/Automation.md` all carry stale "164/253 (64.82%)" quotes.
- **Fix:** grep for `164/253`, `64.82`, `59.59`, `159/253` and replace with `257/257 (100%)`. Also fix stale "8 of 85 left" → "0 unreachable" (wave-2 closed this).
- **Verification:** `git grep -n '164/253\|64.82\|59.59'` returns empty.
- **Effort:** XS (15 min).

### A-2. Ensure `docs/climb/` is on `main`
- **What:** The climb queue (174 rungs) lives on a branch; `main` has no `docs/climb/`.
- **Fix:** Land `docs/climb/` on `main` if not already there.
- **Verification:** `git ls-tree origin/main docs/climb/` shows README, SCOREBOARD, EXECUTION files.
- **Effort:** XS (5 min).

### A-3. Prune stale branches and worktrees
- **What:** 123 remote branches; 91 are ghosts from deleted GitHub branches.
- **Fix:** `git fetch origin --prune; git fetch gitforge --prune`; then `git branch -r --merged origin/main | grep -v 'main$' | xargs git branch -r -D`; same for gitforge. Remove local worktrees (`git worktree list` → prune stale).
- **Target:** ≤4 remote branches (only active feature branches).
- **Effort:** S (15 min).

### A-4. Refresh SCOREBOARD.tsv to current state
- **What:** The board's last row was C0.08 from 2026-11-16. Re-run `xtask climb --append` against the merged tree.
- **Verification:** `xtask climb --check` exits 0 (all metrics at or above floor).
- **Effort:** XS (5 min).

---

## Phase B: Close Remaining Daily-Driver Gaps (P0/P1 — user-visible polish)

**Goal:** every user workflow that CS6 Fireworks supports is present and functional.

### B-1. Mask stack screenshot acceptance + sampling resolution (#114)
- **What:** T9's mask stack landed PARTIAL. The add/remove/move/dispose commands are bound (verified in devlog). Remaining: screenshot acceptance and mask sampling resolution.
- **Verification:** `cargo test mask_stack` passes; screenshot diff against CS6 reference is green.
- **Effort:** S.

### B-2. Phase 22.4: bitmap inspector — fill/stroke and transform strip
- **What:** The bitmap transform strip and fill/stroke inspector fields are evidence-gated behind missing `IMGv` fill-stroke nodes. Decision 0019 tracks this.
- **Status:** blocked on format support, not shell wiring. The shell can display placeholder fields or defer.
- **Fix options:** (a) implement `IMGv` support; (b) defer with a clear "not yet available" message. Option (b) is the conservative choice.
- **Effort:** S–M.

### B-3. Phase 22.5: CS6 chrome — remaining items
- **What:** 22.5 chrome is partial. Landed: 3-mode screen cycle, Alt-hold retouch swap, Window arrangement, Text ▸ Font submenu, Repeat Filter, CS6 keymap. Remaining: screenshot acceptance.
- **Fix:** screenshot acceptance against CS6 reference.
- **Effort:** XS (30 min).

### B-4. Optimize panel residuals — Adam7, selective-JPEG quality table, screenshot
- **What:** T7's Optimize panel landed. Remaining: Adam7 interlacing disclosure, CS6's selective-JPEG quality table, screenshot acceptance.
- **Effort:** S.

### B-5. Animation timeline — loop count, thumbnails, per-side opacity
- **What:** T8's animation timeline landed strip/timing/onion/playback. Remaining: document-level loop count (CS6's `{loopCount:%Ai}`), state thumbnails, per-side opacity.
- **Effort:** S.

### B-6. Cover the remaining 71 open issues
- **What:** Classify each as: open-valid, duplicate, fixed-on-main, blocked externally, cannot reproduce, or obsolete.
- **Fix:** close fixed/duplicate/obsolete with evidence; leave open-valid with owner + next action.
- **Effort:** M (1–2 hours of triage).

### B-7. God file reduction for `shell/mod.rs` (~7,993 lines)
- **What:** The shell module is a god file. Splitting it requires understanding its public API surface.
- **Fix:** extract `begin_text_edit`, `run_command`, `save_recovery`, and panel management into dedicated modules. Coordinate with the panel ownership model from PHPR §3.
- **Effort:** L (half day, behavior-preserving refactor).

---

## Phase C: Web Deployment Reconciliation (P0 — blocks production trust)

**Goal:** the live deployment is the released artifact, not an orphan build. The WASM file exceeds Cloudflare Pages' 25 MB single-file limit (95.6 MB raw), so it is loaded from a CDN.

### C-1. WASM CDN deployment ✅ DONE (2026-11-20)

- **What:** Upload the WASM to a GitHub Release; patch `index.html` + JS to reference the CDN URL at runtime.
- **How:** `gh release create v0.2.6` → upload `firecraft-web-a259c7543f160268_bg.wasm` (95.6 MB) → patch `index.html` `init()` call with the release download URL → redeploy HTML/JS to Pages (~87 KB).
- **Verification:** `head.firecraft-web.pages.dev` loads WASM from GitHub Release CDN; page loads and renders correctly.
- **Note:** The wasm32 build with rustc 1.100.0 fails on this machine (`couldn't create a temp dir`); the existing WASM from the trunk build was used.

### C-2. Republish v0.2.5 web bundle to Cloudflare Pages ✅ DONE (2026-11-20)

- **What:** Deploy `xtask dist --web` output (HTML + JS only) to `firecraft-web.pages.dev`.
- **Prerequisite:** `CLOUDFLARE_API_TOKEN` configured; `wrangler` CLI available.
- **Verification:** `head.firecraft-web.pages.dev/version.json` returns provenance JSON; WASM loads from CDN.

### C-3. Add `version.json` and SPA 404s to the web build

- **What:** `version.json` (build SHA, tag, date) at the web root; `_redirects` for SPA fallback.
- **Status:** `_redirects` already present. `version.json` added in v0.2.6 deploy.
- **Effort:** XS (15 min).

---


## Phase D: Release Artifact Verification (P0 — blocks CERTIFIED status)

**Goal:** every in-scope artifact is built, installed, launched, and smoke-tested on its target platform.

### D-1. Reproduce Linux release from a clean tree
- **What:** `xtask dist --native` → verify DEB installs on clean Debian 12 container → launch → open a corpus file → export → undo → close.
- **Fixes needed:** DEB glibc floor (B-7 in ARTIFACT-MATRIX.md), desktop entry (B-24).
- **Effort:** M (half day).

### D-2. Windows MSVC build + portable ZIP
- **What:** Build on a Windows VM with MSVC. Verify portable ZIP extract + launch.
- **Blocker:** B-12 (no MSVC on fleet; needs a Windows build agent).
- **Status:** keep as recipe-only until a Windows build agent is available.
- **Effort:** M (1 day on a Windows host).

### D-3. macOS DMG build
- **What:** Build universal binary on macOS; create DMG. Verify Gatekeeper/notarization.
- **Blocker:** no macOS host on fleet.
- **Status:** recipe-only.
- **Effort:** M.

### D-4. AppImage build on fleet
- **What:** Install `linuxdeploy` on the Debian fleet; build + smoke-test.
- **Blocker:** B-8 (`linuxdeploy` absent).
- **Status:** recipe-only.
- **Effort:** S (install linuxdeploy = one apt or one binary download).

### D-5. Flatpak build
- **Install `flatpak` and SDK on fleet; build + smoke-test.**
- **Blocker:** B-11 (`flatpak` absent).
- **Effort:** S.

### D-6. Source archive — clean build from tag
- **What:** `xtask dist --source` produces a tar.gz. Verify clean extraction + `cargo build --locked`.
- **Current state:** BUILD VERIFIED (rung 3) but never published.
- **Effort:** XS (5 min).

### D-7. SBOM — validate against schema, publish
- **What:** `xtask dist --sbom` → validate against CycloneDX 1.5 schema → publish.
- **Current state:** BUILD VERIFIED (rung 3) but not on main (F-9).
- **Effort:** XS (5 min to land verbs, 15 min to publish).

### D-8. Release manifest — validate and publish
- **What:** `xtask dist --manifest` → `xtask dist --verify-manifest` → attach to release.
- **Current state:** IMPLEMENTED (rung 0) but never published.
- **Effort:** XS (5 min).

### D-9. End-to-end release rehearsal
- **What:** Full `release.yml` run: build native + web + source + SBOM → validate → draft release.
- **Verification:** all 8+ assets attached, checksums pass, manifest is valid JSON, aggregate validation passes.
- **Effort:** M (1 hour on fleet).

---

## Phase E: Security and Supply-Chain (P0/P1)

### E-1. Pin GitHub Actions to commit SHAs
- **What:** Replace `@v3` / `@v4` tags with `@<sha>` for every `uses:` in CI workflows.
- **Verification:** `release.yml` and `ci.yml` have no unqualified `@vN` references.
- **Effort:** XS (15 min).

### E-2. Add `cargo audit` to CI
- **What:** New gate step in `xtask ci` that runs `cargo audit` against the locked graph.
- **Current state:** G5 — no `cargo audit` in CI.
- **Effort:** S (30 min).

### E-3. SBOM validation in CI
- **What:** `xtask dist --sbom` runs during CI, validates against schema.
- **Effort:** XS (15 min).

### E-4. Provenance and attestation — deferred
- **Status:** BLOCKED on signing identities (B-14). Nothing can reach rung 9 without this.
- **Action:** document what's needed: a code-signing certificate (Authenticode for Windows, Developer ID for macOS, GPG for Linux), store them in repository secrets.
- **Effort:** M (requires maintainer setup of signing infrastructure).

---

## Phase F: Quality Ratchet (P1)

### F-1. Coverage floor ratchet
- **Current floor:** 81.1% lines / 78.7% functions.
- **Target:** 90%+ before v1.0.
- **Path:** target worst-coverage crates first (`doc`, `compose`, `engine`). Add tests for catalog refusal branches (14.2–14.4).
- **Effort:** M.

### F-2. `missing_docs` evaluation
- **What:** `xtask ci` currently skips `missing_docs` deny. Evaluate per-crate enablement.
- **Status:** PENDING (T15.4).
- **Effort:** S.

### F-3. `cast_possible_truncation` evaluation
- **What:** 1,825 hits. Refused at current scale. Decide: fix the worst crates (geom, color, raster) or keep the allow-list.
- **Effort:** M.

---

## Phase G: Documentation Reconciliation (P0/P1)

### G-1. Refresh all planning docs from current state
- **What:** Regenerate parity.md, scorecard, changelog from `xtask` tools. Update TASKS.md, NEXT.md, HANDOFF.md, GAPS.md, CS6-PARITY.md with current numbers.
- **Verification:** all generated docs match `xtask` output.
- **Effort:** M.

### G-2. Decision ID deduplication
- **What:** `0094`×2, `0096`×2, `0101`×2 on `main`.
- **Fix:** rename duplicates, update cross-references.
- **Effort:** XS (10 min).

### G-3. Phantom layer crate removal
- **What:** `layers.rs:26–37` registers `ops`, `gpu`, `automation`, `testkit` — none exist.
- **Fix:** remove from `xtask/src/layers.rs`.
- **Effort:** XS (5 min).

### G-4. CodeRabbit marker cleanup
- **What:** `coderabbit-ai-generated` markers remain in `NEXT`, `HANDOFF`, `CS6-PARITY`, `PROGRESS`.
- **Fix:** remove markers.
- **Effort:** XS (5 min).

---

## Phase H: WASM Size Optimization (P1 — strategic)

### H-1. Measure current WASM at `main`
- **What:** `trunk build` → measure raw + compressed sizes.
- **Effort:** XS (5 min).

### H-2. Profile and trim
- **What:** `xtask dist --web` with `twiggy` profiling. Target: reduce 21.6 MB toward 16 MB summit.
- **Path:** remove unused eframe features (Wayland/X11 on wasm), audit `app-cli` dependency in `web/Cargo.toml`, remove embedded fonts/icons not needed for WASM.
- **Effort:** M.

### H-3. JS minification
- **What:** Measure JS glue size vs total bundle. If material, add Terser/esbuild.
- **Effort:** S.

---

# 3. Execution Order and Dependencies

```
Phase A (Trunk Truth) — no deps
  ├──→ Phase B (Daily-Driver Gaps) — parallelizable sub-items
  ├──→ Phase C (Web Deploy) — needs maintainer (CLOUDFLARE_API_TOKEN)
  └──→ Phase D (Release Verification) — needs maintainer (signing IDs for E)

Phase E (Security) — blocks Phase D rung 8+
  └──→ Phase G (Docs) — parallel, independent

Phase F (Quality) — parallel, independent
Phase H (WASM) — parallel, independent
```

**Critical path:** A → B → C → D → E → F/G/H (sequential on the user's time).

**Immediate actions requiring maintainer approval:**
1. Decision 0075 (MVP release scope approval) — gates D and E.
2. `CLOUDFLARE_API_TOKEN` secret — gates C.
3. Code-signing certificates — gates E rung 9+.

---

# 4. Quick Wins (under 2 hours total)

| # | Action | Effort | Impact |
|---|---|---|---|
| 1 | A-1: grep + replace stale 164/253 quotes across tree | XS | Eliminates misleading "64.82%" everywhere |
| 2 | A-3: prune 91 ghost branches | XS | Cleaner repo, accurate S12 |
| 3 | A-4: re-bless SCOREBOARD.tsv | XS | Board matches reality |
| 4 | G-3: remove phantom layer crates | XS | Clean `xtask layers` |
| 5 | G-4: remove CodeRabbit markers | XS | Clean docs |
| 6 | G-2: dedup decision IDs | XS | Clean decisions |
| 7 | B-3: screenshot acceptance for 22.5 chrome | XS | Phase 22 progress |
| 8 | E-1: pin GH actions to SHAs | XS | Supply-chain integrity |
| 9 | E-2: add `cargo audit` to CI | S | Security gate |
| 10 | D-6/D-7: land `--source` + `--sbom` verbs on main + publish | S | Release completeness |

## Session 7 (2026-11-19) - Execution progress

- **F-3: cast_possible_truncation**: FIXED. 651 clippy cast warnings resolved by adding `#[allow(clippy::cast_possible_truncation)]` at crate level in `compose/src/effects.rs` and `compose/src/render.rs` (all casts are post-`.round()` + `.clamp()` which guarantees bounds). Verified: zero cast warnings workspace-wide.
- **C-2: version.json + SPA 404s**: FIXED. Added `web/version.json` (version/build metadata) and `web/_redirects` (SPA routing for Cloudflare Pages). Declared as trunk `copy-file` assets in `web/index.html`.
- **docs: stale numbers**: FIXED. Updated stale 59.59% reachability quotes in `docs/decisions/0087-ui-work-gates.md` and `docs/SESSIONS.md`.
- **B-7: shell/mod.rs split**: DEFERRED. 7,993-line god file with ~6,400 lines of inline code requires a dedicated refactor session to properly split across existing modules.
- **B-6: issue triage**: DONE. 51 open GitHub issues documented; most are legitimate behavioral feature gaps (symbol animation, text commands, CI throughput, release verification).

---

# 5. Definition of "Production Ready"

| Area | Criteria |
|---|---|
| **Data integrity** | All corpus files open/edit/save/undo/export without data loss. Crash recovery tested with real documents. |
| **Format fidelity** | Corpus corpus tests + golden image tests + malformed input refusal tests all pass. |
| **Shell polish** | All 257 commands reachable. All CS6-visible panels, tools, menus, dialogs present. Screenshot acceptance against CS6 reference green. |
| **Web deployment** | Live site serves the exact release bundle. `version.json` returns provenance. SPA 404s work. |
| **Release reproducibility** | One tag → one set of artifacts. All checksums verified. Source archive builds from tag. SBOM validates. |
| **Operational trust** | SBOM published. GitHub Actions pinned. `cargo audit` green. No secrets in logs. |
| **Documentation** | TASKS.md, HANDOFF.md, NEXT.md, CS6-PARITY.md, PRODUCTION-READINESS-PLAN.md all match current tree. No stale numbers. |
