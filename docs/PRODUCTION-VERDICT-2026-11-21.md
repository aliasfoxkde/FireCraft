# FireCraft Production-Readiness Verdict (2026-11-21)

> **Assessment date:** 2026-11-21
> **Canonical source:** `main` at `b61f198` + branch `feat/wasm-cdn-deploy` (+80 commits)
> **Latest release:** `v0.2.5` (2026-11-03, operator-published by hand)
> **Scope authority:** decision 0075 — *recommended, awaiting maintainer approval*

---

## 1. CS6 Feature Parity: ~72% User-Visible

### Feature Matrix

| Category | CS6 | FireCraft | Status |
|---|---|---|---|
| Menu bar (10 menus, ~296 rows) | 296 | 296 rows, 257/257 commands bound, 2 inert rows | **~85%** — all commands executable via Ctrl+P palette |
| Tools & flyouts (30+ groups) | 30+ | 25 keys bound, 28/30 implemented | **~97%** — 3D tools excluded (legacy) |
| Panels (23 CS6 panels) | 23 | 14 complete, 4 partial, 2 N/A | **~65%** — Project/Site panels excluded |
| Property inspector (7 selection kinds) | 7 | 2 complete, 5 partial | **~30%** — engine has capability, UI exposes subset |
| Import/Export (~15 formats) | ~15 | 13 formats, 2 partial | **~90%** — PNG, JPEG, GIF, BMP, TIFF, WebP, SVG, PSD, animated GIF |
| Canvas interactions (~10 types) | ~10 | 10 implemented | **100%** — transform handles, path nodes, gradient editor, guides, snap |
| Keyboard access (128 command IDs, 167 bindings) | 167 | 128 IDs fully bound, 4 alt keymaps missing | **~97%** |
| Selection states (8 states) | 8 | 4 complete, 4 partial | **~50%** |
| Filters & effects (30 filters, 6 live) | 30+6 | 30+6 implemented | **100%** |
| Symbols (graphic/button/movie-clip) | Full | Full support, nested editing, nine-slice | **100%** |
| Text editing | Full | In-canvas, text-on-path, rich attributes | **~85%** — static font list (5 families) |
| Web export (9 workflows) | 9 | 9 workflows | **100%** |
| Scripting (.jsf) | Full | .jsf host headless, .fwsteps journal, MCP server, CLI | **~85%** — .jsf runs CS6 corpus headless |
| Modern additions | None | HiDPI, autosave, command palette v2, preferences, accesskit | **+30% beyond CS6** |

**Engine commands:** 257 registered, 257/257 (100%) bound — all reachable from GUI.
**DOM verb parity:** 268 carried / 3 partial / 0 missing across 356 verbs.

### Missing CS6 Features (not blockers)

| Feature | Reason |
|---|---|
| 3D tools | Legacy CS6 feature, explicitly out of scope |
| Project panel | Legacy CS6 feature |
| Site panel | Legacy CS6 feature |
| 4 alt keymap sets (Illustrator/Photoshop/FreeHand/Web Standard) | Nice-to-have, main keymap covers 128 commands |
| Template creation | CS6 template workflow, not core editing |

---

## 2. Production Readiness: ~55% — Not Ready for v1.0 Public Release

### Readiness by Dimension

| Dimension | Readiness | Evidence |
|---|---|---|
| Application core (editing, tools, filters, undo/redo, autosave) | ~90% | 257/257 commands bound; corpus parses; crash recovery works |
| Menu/panel surface | ~85% | 296 menu rows; 18 panels docked; command palette v2 |
| WASM/web build | ~85% | Builds clean (rustc 1.100.0); CDN strategy works; bridge smoke passes |
| Native build (Linux x86_64) | ~90% | `dist --native` produces tar.gz + DEB; published v0.2.5 |
| CI quality gates | ~60% | xtask ci runs 17+ gates; rustc .rmeta bug blocks wasm32 on rustc 1.99.1 |
| Release infrastructure | ~30% | Release workflow never ran end-to-end; no artifacts signed |
| Cross-platform coverage | ~10% | Only Linux x86_64 published; Windows/macOS/AppImage/Flatpak recipe-only |
| Documentation/ledger | ~70% | Parity matrix published; task board active; 6 decision ID duplicates |

---

## 3. P0 Blockers — Must Resolve Before First Public Release

| # | Blocker | Impact | Resolution |
|---|---|---|---|
| **B-6** | Live web deployment is an orphan — no build receipt ties deployed bundle to any source revision | Users served unknown build | Requires Cloudflare deploy access to republish from canonical bytes |
| **B-14** | No artifact is signed or attested — zero signing credentials configured | No artifact can be verified as authentic | Requires Authenticode/Developer ID/GPG credentials (maintainer setup) |
| **B-4/B-21** | Release workflow never succeeded end-to-end; v0.2.5 was operator-published by hand | Build process is undocumented and unrepeatable | Must run `release.yml` on a disposable tag with all assets |
| **G-1/G-2** | CI toolchain mismatch: rustc 1.99.1 has `.rmeta` bug; rustc 1.100.0 has `dashmap` bug | Neither wasm32 nor native builds are verified in CI | rustc 1.100.0 fixed for wasm32; native blocked by `dashmap` fork |
| **T27-50** | Deployment reconciliation not done — deploy access required | Live site doesn't match any release | `CLOUDFLARE_API_TOKEN` needed; `wrangler` CLI |
| **Decision 0075** | MVP release scope not approved by maintainer | No authorized scope cut means full (unrealistic) matrix is standing scope | Maintainer decision required |

---

## 4. P1 Items — Should Fix but Not Block First Release

| # | Item | Effort |
|---|---|---|
| **G-7** | `shell/mod.rs` is 7,996-line god file (221 functions) | L (half day refactor) |
| **G-8** | ~651 `cast_possible_truncation` warnings in native-only (cannot verify until rustc fix) | S |
| **G-10** | 6 decision ID duplicates on main (0094, 0096, 0101 x2) | XS |
| **22.4** | Property inspector: bitmap transform strip, fill/stroke, multi-selection, document-level, enter-group | S |
| **20.2/20.3** | Optimize panel Adam7 interlacing, selective JPEG quality table; animation timeline loop count, thumbnails | S |
| **20.4** | Mask stack: screenshot acceptance + remaining add/remove/move affordances | S |
| **Coverage ratchet** | Floor at 81.1% lines / 78.7% functions; target 90%+ | M |
| **Issue triage** | 71 open issues need classification | M |

---

## 5. Build System Status

| Surface | Status | Evidence |
|---|---|---|
| **Native (Linux x86_64)** | Builds, publishes tar.gz + DEB | Published v0.2.5 DEB (14.8 MB) |
| **WASM32 (web)** | Builds for pure crates only (11 crates) | `xtask wasm` — app/app-cli excluded (scripting→dashmap) |
| **WASM size** | 95.6 MB raw, ~4.5 MB gzipped via CDN proxy Worker | Under Pages 25 MB limit with CDN offloading |
| **CI gate (xtask ci-no-fmt)** | Runs 17+ gates including aegis, coverage, clippy, test, layers, wasm, corpus, golden, perf, fuzz, reachability | Green on rustc 1.99.1 for wasm32 with .rmeta workaround |
| **Release workflow** | `release.yml` — native + web + source + SBOM + checksums + draft release — implemented but publish step commented out | Never executed end-to-end |
| **wasm32 fix (current branch)** | **PASS — 0 errors** | `feat/wasm-cdn-deploy` branch; 7 files changed; clean build verified twice |

---

## 6. Pre-Release Checklist — What Must Happen Before First Public Release

1. **Approve MVP release scope** — maintainer signs decision 0075 defining which platforms constitute acceptable first release
2. **Run release workflow end-to-end on disposable tag** — prove `release.yml` builds native + web + source + SBOM and creates draft release
3. **Reconcile web deployment** — republish Cloudflare Pages from canonical release bytes, verify hash match
4. **Implement SHA-256 manifest for all published assets** — `xtask dist --manifest` exists, needs CI integration
5. **Smoke-test published artifacts** — install/launch CLI binary; bridge smoke against release bytes; verify DEB installs on clean Debian 12
6. **Close decision ID duplicates** — dedup 0094, 0096, 0101 on main
7. **Finalize parity matrix against current `main`** — refresh CS6-PARITY.md to current SHA
8. **Verify `xtask ci` gate passes on `main`** with the wasm32 fix merged
9. **Publish release notes** with platform support limitations stated explicitly

---

## 7. Honest Assessment: Close Enough to Publish?

### Application: ~75% ready for a public preview

**The core product is mature:**
- 257/257 engine commands bound and accessible from GUI
- Full vector editing (paths, primitives, booleans, gradients, patterns, auto-shapes)
- Full bitmap editing (paint family, pixel selection, retouch family)
- Full text editing (in-canvas, text-on-path, rich attributes, paragraph spacing)
- Full symbol management (graphic/button/movie-clip, nested editing, nine-slice)
- Full filter/effects system (30 filters, 6 live effects)
- Full import/export (PNG, JPEG, GIF, BMP, TIFF, WebP, SVG, PSD, animated GIF)
- Full web export (9 workflows)
- Scripting (.jsf headless, .fwsteps journal, MCP server, CLI automation)
- Modern differentiators (autosave, HiDPI, command palette v2, preferences, accesskit)

**What's missing from the UI surface:**
- Property inspector for 3 selection kinds (document-level, multi-selection, bitmap transform)
- Screenshot acceptance for some panels (chrome, mask stack, optimize)
- 4 alternative keymap sets (Illustrator, Photoshop, FreeHand, Web Standard)

### Release Infrastructure: ~30% ready

**Three deal-breakers prevent a publishable v1.0:**

1. **No reproducible release pipeline.** The release workflow has never run end-to-end. v0.2.5 was operator-published by hand. For a public release, the build must be reproducible from a tag.

2. **No trust chain.** Zero signing credentials configured. No artifact can be verified as authentic. SHA-256 checksums alone are insufficient for a production product.

3. **Orphan deployment.** The live web deployment serves a bundle with no build receipt and no source attribution.

### Bottom Line

| What | Ready? |
|---|---|
| Core editing application | ~75% — usable for daily-driver work on Linux |
| Web build | ~85% — builds, loads, CDN works |
| First public release | ~30% — release infrastructure not ready |
| Closed beta (limited audience) | ~65% — application works, trust chain can be communicated |
| Full v1.0 public release | Not ready — requires 2-3 focused engineering cycles on release infra |

**The application is the easier half to complete.** The release infrastructure is the harder half because it depends on external resources (signing credentials, cross-platform build hosts, Cloudflare deploy access) that are outside this repository's control.

**Recommended path to first release:**
1. Approve MVP scope (decision 0075)
2. Run release workflow end-to-end (rehearsal on disposable tag)
3. Deploy canonical release to Cloudflare Pages
4. Ship SHA-256 checksums with all artifacts
5. Publish as `v0.2.6-preview` with explicit "preview" labeling
6. Iterate toward v1.0 with signing credentials and cross-platform builds
