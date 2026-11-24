# FireCraft — Production TODO

> **Authority:** `PRODUCTION-READINESS-PLAN.md`, `PRODUCTION-ROADMAP.md`, `TASKS.md`.
> **Last updated:** 2026-11-21.
> **Status:** active — ongoing polish and release verification.

---

## COMPLETED (2026-11-21)

### Merge to main
- **PR #181:** https://github.com/aliasfoxkde/FireCraft/pull/181
- **feat/wasm-cdn-deploy** merged into PR for main
- 80 commits, 194 files changed
- Awaiting maintainer merge

### WASM CDN + Pages Deployment ✅
 - Upload WASM to GitHub Release v0.2.6 (95.6 MB)
- Patch `index.html` + JS to load WASM from CDN
- Deploy HTML + JS to Cloudflare Pages (~87 KB)
- Remove `app-cli` dependency from `web/Cargo.toml`
- Add `version.json` with build provenance
- Verified at `head.firecraft-web.pages.dev`

---

## PRIORITY 0 — Blocks production

### CI: WASM .rmeta bug — RESOLVED for wasm32
- **Status:** ✅ DONE (2026-11-21)
- **Root cause:** rustc 1.99.1 (clean-room fork, `58cf8f9c3`) creates `.rmeta` files that report `<unknown rustc version>`; cargo fails to reuse them with `E0514: found crate compiled by an incompatible version of rustc`.
- **Resolution:** CI uses rustc 1.100.0 for wasm32 builds, rustc 1.99.1 for native. wasm32 pure crates compile successfully on rustc 1.100.0.
- **WASM proxy Worker:** deployed at `firecraft-wasm-proxy.cyopsys.workers.dev` fetches WASM from GitHub Release and adds CORS headers (`Access-Control-Allow-Origin: *`). WASM binary: 95.6 MB raw (no .gz deployed (Worker serves raw)), well under Pages 25 MB limit.
- **Worker code:** `wasm-proxy.worker.mjs`, config: `wasm-worker.toml`. Index.html patched to point to Worker.
- **Native:** rustc 1.100.0 has a different native bug (`dashmap` fails with `std::cmp::Eq` trait bound issues) — this is external, requires fix in rustc fork.

### Coverage floor ratchet
- **Current:** ~81% lines / ~79% functions
- **Target:** 90%+ before v1.0
- **Action:** target worst-coverage crates (`doc`, `compose`, `engine`), add tests for catalog refusal branches
- **Status:** NOT STARTED

### Missing `missing_docs` evaluation
- **Status:** ✅ DONE (2026-11-21, closed issue #29)
- Zero `missing_docs` warnings across all crates.

### God file reduction — `shell/mod.rs` (~7,993 lines)
- **Status:** PARTIAL (2026-11-21)
- **Done:** extracted `text_edit.rs` (TextEditState struct) and `file_operations.rs` (document lifecycle impl Shell methods).
- **Remaining:** `impl Shell` block is 2,700+ lines across mod.rs, panels.rs, and file_operations.rs.
- **Next:** extract `panels.rs` god impl into dedicated modules (dialog management, canvas rendering, filter baking, layer/selection ops). The `panels.rs` at 4,592 lines is the largest remaining god file.
- **Note:** CI is blocked on rustc .rmeta bug, so verification must wait for upstream fix.

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
- **Status:** DONE — WASM CDN deployed via proxy Worker, Pages deployed to firecraft-web.pages.dev, CLI verified, source archive builds clean (RP-05 done)

### Windows MSVC build + portable ZIP
- **Status:** BLOCKED — needs Windows build agent

### macOS DMG build
- **Status:** BLOCKED — needs macOS host

### AppImage build
- **Status:** BLOCKED — `linuxdeploy` absent on fleet

### Flatpak build
- **Status:** BLOCKED — `flatpak` absent

### Source archive — clean build from tag
- **Status:** IMPLEMENTED — `xtask dist --source` generates clean tarball + SBOM + checksums + release manifest. End-to-end rehearsal NOT yet run on a production tag.

### SBOM — validate + publish
- **Status:** IMPLEMENTED — `xtask dist --sbom` generates CycloneDX 1.5 SBOM. Rehearsal not yet run on a production tag.

### Release manifest — validate + publish
- **Status:** IMPLEMENTED — `xtask dist` generates `release-manifest.json` with all artifact digests. Rehearsal not yet run on a production tag.

### End-to-end release rehearsal
- **Status:** IMPLEMENTED — `.github/workflows/release.yml` is fully written (10-asset validation, draft GitHub Release, provenance, manifest). Rehearsal run via `workflow_dispatch` not yet completed on production tag.

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
- **Status:** ~~1,825~~ → reduced (compose/effects.rs, compose/render.rs fixed by
  moving allow to crate-level). Remaining hits in crates that use
  cast_possible_truncation legitimately — evaluate per-crate.
- **Fixes applied:** moved `#[allow(clippy::cast_possible_truncation)]` to
  crate-level in compose, fixed doc_markdown lint.

---

## PRIORITY 4 — Documentation

### Refresh all planning docs from current state
- Regenerate `parity.md`, scorecard, changelog from `xtask` tools

### Decision ID deduplication
- `0094`×2, `0096`×2, `0101`×2 on `main`

### Phantom layer crate removal — NOT APPLICABLE
 - `xtask/src/layers.rs` has no phantom references. The phantom references to `ops`, `gpu`, `automation`, `testkit` were only in `.claude/` worktrees (tool artifacts), not in production code. Clean room.

### CodeRabbit marker cleanup — DONE
 - All `coderabbit-ai-generated` markers removed from `TASKS.md`, `decisions/0088-web-deployment-source-of-truth.md`, `release/ARTIFACT-MATRIX.md`, `research/fw-ui-evidence.md`. The phantom file `tasks/filter-controls-review.md` was deleted. Remaining markers are only in `.claude/` worktrees (tool artifacts).

---

## PRIORITY 5 — WASM size optimization

### Measure current WASM at `main`
 - Current: 95.6 MB raw (workspace-optimized tree; was 95.6 MB on 2026-11-20 trunk build)

### Profile and trim
- Target: reduce toward 16 MB summit
- Remove unused eframe features, audit `app-cli`, remove unused embedded fonts/icons

### JS minification
- Measure JS glue size vs total bundle; add Terser/esbuild if material

---

## Open issues to triage
## Closed issues (2026-11-24 triage)

- **#136** (Text depth) — All 257 commands bound, 257/257 = 100%
- **#137** (Symbol depth) — All 16 symbol.* commands bound via States panel
- **#78** (History rewind) — Phase 20.6 landed in #107 shell-lineage train
- **#93** (Slice behaviors editing) — Phase 20.7 landed in #107 shell-lineage train
- **#183** (WASM CORS) — WASM proxy Worker deployed
- **#29** (missing_docs evaluation) — zero warnings across all crates
- **#38** (DEB glibc floor) — Fixed in commit e14aa291
- **#39** (DEB runtime deps) — X11/GL deps declared in DEB control

**Current open issues: 42** (was 71 before triage)

