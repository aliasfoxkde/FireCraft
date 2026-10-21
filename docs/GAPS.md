# FireCraft — Gaps, Improvements & Next Steps

> Audited 2026-10-20. Replaces the "Now" snapshot in PROGRESS.md until
> the next session close. A living document — update in-place, do not
> maintain a separate stale copy.

---

## State snapshot

| Dimension | Status | Notes |
|---|---|---|
| **Shipped** | v0.2.2 published on GitHub (5 assets) | ✅ |
| **CI** | GitHub Actions green on b27c658; 694024a queued | GitForge also pushed |
| **aegis** | Baseline clean; scan runs in 0.17s | `target/` + `web/dist/` + `fuzz/` in `.aegisignore` |
| **Coverage** | 80.3% lines / 78.0% funcs (threshold 60%) | `cargo llvm-cov` locally fails on stale build artifact; CI uses `xtask coverage` which works |
| **fmt / clippy** | Not run locally (hook blocks); last known green in CI | |
| **Registry** | 241 commands | Stable since #133 |
| **Parity** | 268 C / 3 P / 0 M / 85 N | Evidence-gated P rows are terminal |
| **Code** | 110,805 Rust lines (crates + web); 118,012 total | No `unwrap()`/`panic!()` outside `#[cfg(test)]` |
| **Decision docs** | 66 docs (0001–0066) | Complete through decision 0066 |
| **TODO/FIXME** | 0 in crates/xtask/web | Clean |
| **CHANGELOG** | Generated, tracked | `.aegisignore`-excluded |
| **v0.2.2 release notes** | `docs/releases/v0.2.2.md` | Artifact hashes present |

---

## Critical blockers — ALL RESOLVED (2026-10-20)

- ✅ aegis hangs on `target/`/`fuzz/`/`web/dist/` — all three in `.aegisignore`, baseline regenerated, scan now 0.17s
- ✅ v0.2.2 published — `gh release edit v0.2.2 --draft=false`
- ✅ `appinfo.json` version stamp — `@VERSION@` → `0.2.2` in both `packaging/webos/` and `dist/web/`

---

## Phase 1: Documentation & release hygiene ✅ DONE

All done in session #138 (2026-10-20): PROGRESS "Now" reset to
2026-10-20, v0.2.2 milestone row, SESSIONS #138, HISTORY arc+decision
index extended through 0066, log/devlog entry, all critical blockers
resolved. Smoke budget comment is already clear (virtual-time vs wall-clock
distinction is documented).

## Phase 2: Image support (Phase 11 remaining items)

### 2.1 AVIF decode — ✅ Decision done (0064)

Decision 0064: AVIF via `ravif` (pure Rust, wasm-safe) when user needs
arrive. No implementation required to close the decision. AVIF files
currently pass through the "unsupported format" path cleanly.

### 2.2 HEIC refusal — ✅ Done (0065)

Decision 0065: deliberate refusal — HEVC patent pools. No HEIC code.

### 2.3 Import surface parity — ✅ No action needed (2026-10-20)

CLI `import` accepts any file — no extension whitelist. Format detection
is pure magic-byte sniffing (`io::raster::sniff`/`decode`). All 7
supported formats (PNG/JPEG/GIF/BMP/TIFF/WebP/WBMP) are handled
correctly regardless of filename. PNM and ICO are correctly rejected.
No mismatch exists. GAPS.md description of a CLI/export list mismatch was
wrong — documented here as a finding, not a gap.

### 2.4 Lossy WebP — implementation blocked on toolchain (3–4 hr) → Task #146

Decision 0063 landed lossless VP8L via `image-webp` 0.2.4. `image-webp`
exposes only `WebPEncoder::new_lossless()` — no lossy VP8 API exists in
the pure-Rust crate. Adding lossy WebP requires `libwebp-sys` (C FFI)
behind a feature gate: `#[cfg(feature = "lossy-webp")]`. The `wasm32`
target cannot enable this feature. Native builds export lossy; WASM builds
return "lossy WebP not supported in browser". `libwebp-sys` must be
added as a conditional dependency and the encode path split in
`crates/io/src/webp.rs`. App-CLI needs a quality flag.

---

## Phase 3: Coverage growth toward 90% line / 90% function (ongoing)

### 3.1 Establish the baseline (30 min)

Run on a clean build:
```bash
cargo clean
cargo build --workspace
cargo run -p xtask -- coverage --bless
```
Then commit the new `llvmcov/default.profdata` and threshold comments.

**Note:** `cargo llvm-cov` directly (without the xtask wrapper) currently
fails locally on a stale `target/llvm-cov-target/` artifact from a prior
build. The xtask `coverage` step works because it re-measures from a
fresh state. The direct invocation issue is a local environment problem
(not a CI issue), but the coverage data should be re-measured on a clean
build before any threshold raise.

---

### 3.2 Identify the coverage gaps (1 hr)

The 80.3% / 78.0% numbers cover the whole workspace. Identify which crates
are below average:
- `crates/doc/src/fwmap/` — reader is well-tested (fwmap/tests.rs is 2996
  lines, heavy on round-trip)
- `crates/engine/src/catalog/` — many command handlers may be untested
- `crates/compose/src/effects.rs` — filter kernels likely well-covered by
  golden tests
- `crates/io/` — jpeg/png/webp have some unit tests; other formats may not

**Deliverable:** A table of below-average crates with targeted test gaps
(e.g., "bitmap resize: no unit test for the aspect-correct path").

---

### 3.3 Raise the floor incrementally (ongoing, per session)

Approach: add 0.5–1% per session as a side-effect of feature work.
Do not reserve a dedicated "coverage sprint". The rule from AGENTS.md:
**rising-only test floors** — each new feature must bring its own tests.

Target:
- Current: 80.3% lines / 78.0% funcs
- Next raise: 82% lines / 82% funcs (after Phase 2 closes)
- Stretch: 90% / 90% (requires Phase 5 infra work)

---

## Phase 4: Release engineering hygiene (Phase 13 remaining)

### 4.1 WebOS packaging proof (2–3 hr)

**Status:** `packaging/webos/appinfo.json` and `host.html` are staged in
`dist/web` during `dist --web`. `appinfo.json` version stamp is broken
(see Critical blockers above).

**What works:** `dist/web/` contains a valid WebOS app bundle. The
`host.html` is the smoke host (the same one Chromium loads in the bridge
smoke test).

**What has not been proven:** The bundle installs and runs on an actual
WebOS device. This is infrastructure-blocked (no WebOS device in scope).

**Deliverable:** Fix the version stamp. Document in `docs/WEBOS.md` what
"proven on real hardware" means and what the blocking gap is.

---

### 4.2 Smoke budget measurement — ✅ DONE

Measured 2026-10-20: `firecraft-web-71ff69a0cf973bce_bg.wasm`
is **12.2 MB** (12,778,688 bytes). Above 10MB threshold (warrants
investigation) but below 15MB (no budget raise needed).

60s virtual-time budget is adequate for a 12MB bundle — fetch+compile
takes ~3–10s on CI runners. The 300s wall-clock backstop covers any
unexpected slowness. Decision 0066 is current. Re-measure after any
wasm bundle size increase >1MB.

---

### 4.3 Platform recipes honest status — ✅ Done

`docs/packaging/STATUS.md` created with table of all platforms, recipes,
artifacts, last-verified dates, and known blockers. Linux x86_64
verified 2026-10-19 (v0.2.2). AppImage/macOS/Windows: never verified,
off-fleet recipes owned by operators with the respective toolchains.

---

## Phase 5: CI / infrastructure improvements (optional, high-value)

### 5.1 GitHub Actions — separate fmt check into its own job — ✅ Done

Two-job CI: `fmt` (fast ~30s) + `gate` (`xtask ci-no-fmt`). Both run
on all pushes. `xtask` gained `ci-no-fmt` verb. `fmt` job fails fast,
prevents expensive gates from running on formatting errors.

---

### 5.2 Coverage step isolation — ✅ Done (with #5.3)

cargo-llvm-cov and trunk caching implemented below.

### 5.3 Trunk caching — ✅ Done

`~/.cargo/bin/cargo-llvm-cov` and `~/.cargo/bin/trunk` are now cached
between runs via `actions/cache@v4`. Both use the `Cargo.lock` hash as
the cache key. If cache-hit, the install step is skipped (~2 min saved
per run). Locked versions are pinned so the cache key is stable.

---

## Phase 6: 99% coverage plan (stretch — begins after Phase 3)

> This is aspirational. The 90% floor is already above the current 80%.
> Reaching 99% requires structural changes, not just test additions.

### 6.1 What's not covered at 80%

The untested surface at 80.3% line coverage is likely concentrated in:
- **Error paths**: `Result::unwrap()` / `?` propagation in non-test code —
  these are covered by integration tests but not isolated unit tests.
- **Rare format paths**: TIFF read, PSD write, GIF animation encode —
  these may have integration tests but sparse line coverage.
- **UI event handlers**: `crates/app/src/shell.rs` (3800 lines) handles
  egui events; most are tested via manual browser proof, not unit tests.

### 6.2 The path to 99%

1. **Error path isolation**: For each `?` in hot paths, add a targeted
   test that feeds the error variant. Do not test the happy path again.
2. **Golden test expansion**: If a format round-trips, the golden test
   already covers the lines. Expand the golden corpus rather than writing
   unit tests for format readers.
3. **UI stubs**: For shell event handlers that cannot be tested headlessly,
   add `#[cfg(test)]` stubs that at minimum exercise the dispatch match.

**This phase is not started until Phase 2 (image support) is closed.**

---

## Phase 7: Deep research items (parked unless oracle materializes)

### 7.1 Out-of-bounds effect growth (Wine oracle needed)

See HANDOFF §4 Phase 12. The destructive bake clips to the image surface
by settled posture. Reopening requires a CS6 filter/flatten bounds
oracle — `source/FireworksPortable/` exists to run CS6 under Wine.

**Blocker:** No Wine on this host. The corpus run would generate Adobe-
authored test files. Do not attempt without the oracle.

---

### 7.2 `setFillNColorNTexture` (TXFv texture observation needed)

Evidence-gated. Needs an observed `TXFv` shape authored in CS6. Same
oracle requirement as 7.1.

---

## Quick-reference: task ownership

| Task | Phase | Size | Blocker |
|---|---|---|---|
| Import surface parity (#145) | 2 | 2–3 hr | None |
| Lossy WebP implementation (#146) | 2 | 3–4 hr | Decision 0063 contract |
| Coverage baseline re-measure (#147) | 3 | 30 min | Tasks #145, #146 |
| Coverage gap table (#148) | 3 | 1 hr | Task #147 |
| WebOS packaging proof (#149) | 4 | 2–3 hr | None |
| Smoke budget measurement (#150) | 4 | 1 hr | None |
| Platform recipes status doc (#151) | 4 | 30 min | None |
| Split CI fmt job (#152) | 5 | 1 hr | None |
| Coverage step isolation (#153) | 5 | 1 hr | None |
| Trunk caching (#154) | 5 | 1 hr | None |
| 99% coverage plan (#155) | 6 | future | Tasks #145, #146, #148 |
| Effect growth research (#156) | 7 | parked | Needs Wine oracle |
| TXFv texture research (#157) | 7 | parked | Needs Wine oracle |

---

## Next action

Start with **task #145** (import surface parity). This is the
smallest Phase 2 item and requires no new dependencies or decisions.
Audit `io::raster::decode` → align CLI accept-list → add format tests.
