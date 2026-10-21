# FireCraft — Gaps, Improvements & Next Steps

> Audited 2026-10-20. Replaces the "Now" snapshot in PROGRESS.md until
> the next session close. A living document — update in-place, do not
> maintain a separate stale copy.

---

## State snapshot

| Dimension | Status | Notes |
|---|---|---|
| **Shipped** | v0.2.2 tag + GitHub draft, 5 assets uploaded | Draft not yet published |
| **CI** | GitHub Actions green (last: 28m21s, commit b27c658) | GitForge not checked (hub unavailable) |
| **aegis** | Baseline clean; local scan hangs on `target/` | `target/` not in `.aegisignore` — scanner traverses 110K+ Rust lines of build artifacts |
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

## Critical blockers (fix before next push)

### [CRITICAL] aegis scan hangs locally on `target/`
**Problem:** `aegis scan .` traverses `target/`, which contains ~110K lines of
build artifact `.rlib` and `.d` files — the scanner hangs indefinitely
(>120s before SIGTERM). CI does not hit this because it runs on a clean
runner with no `target/` from prior builds.

**Root cause:** aegis does **not** respect `.gitignore` by default.
`.aegisignore` excludes specific generated files but not directories.

**Fix:** Add `target/` to `.aegisignore`.
```
target/
```

This is a local-only issue. CI remains unaffected.

---

### [HIGH] v0.2.2 draft not published
**Problem:** `gh release view v0.2.2` shows `isDraft: true`. The tag was
pushed, assets uploaded, but the release was never published.

**Fix:** `gh release edit v0.2.2 --draft=false`

---

### [HIGH] `appinfo.json` version stamp shows `@VERSION@`
**Problem:** `packaging/webos/appinfo.json` has `"version": "@VERSION@"` —
the substitution placeholder is present in the staged copy, meaning the
WebOS `appinfo.json` was never actually stamped during the `dist --web`
run.

**Fix:** `sed -i 's/@VERSION@/0.2.2/' dist/web/appinfo.json`; also update
`packaging/webos/appinfo.json` to `0.2.2`.

---

## Phase 1: Documentation & release hygiene (small, high-leverage)

### 1.1 Update stale docs (30 min)

- **PROGRESS.md "Now"**: Update from 2026-10-15 to 2026-10-20.
  Add v0.2.2 release row and Phase 11/13 status lines.
- **PROGRESS.md parity table**: Add v0.2.2 row (268 C / 3 P / 0 M / 85 N,
  registry 241, no surface change — enhancement-only release).
- **SESSIONS.md**: Add entry for #138 (this session, 2026-10-20):
  "CI workflow repair, v0.2.2 staged, gaps audit".
- **HISTORY.md**: Add arc entry for v0.2.2.
- **`log/devlog.md`**: Append this session's entry.

**Verify:** `git log --oneline -5` shows no unexpected commits from this session yet.

---

### 1.2 Fix smoke budget comment (2 min)
**Problem:** `xtask/src/dist.rs` line 211 comment says
`--virtual-time-budget=60000` is "60s" — but the comment reads as if this
is the wall-clock timeout. It is not; it is the Chromium virtual-time
budget. The wall-clock backstop is `SMOKE_TIMEOUT = 300s`.

**Fix:** Clarify the comment. The actual issue is: if the wasm bundle grows
and fetch+compile exceeds 60s virtual time, the smoke will fail
spuriously. But 60s virtual = ~60s wall on a slow runner, and the 300s
wall-clock covers it. The comment should state this explicitly.

---

## Phase 2: Image support (Phase 11 remaining items)

These are the standing enhancement direction from HANDOFF §4 Phase 11.

### 2.1 AVIF decode — decision doc (1–2 hr)

**Status:** `image` crate's `avif-native` feature exists but is not
wasm-compatible (requires C). A decision doc must record:
- AVIF import path: OS decoder on desktop, "format unsupported in browser"
  message in WASM.
- AVIF export: out of scope until a pure-Rust encoder exists.
- The `image` crate's `avif-native` feature should NOT be added to the
  workspace `Cargo.toml` without the decision doc.

**Deliverable:** `docs/decisions/0067-avif-decode-policy.md`

**Stopping condition:** Decision doc committed; import surface unchanged unless
the decision explicitly permits an OS-level decoder path.

---

### 2.2 HEIC — likely refusal (1 hr)

**Expected outcome:** No maintainable open HEIC encoder/decoder without
patent-pool or C dependencies. The decision doc records the refusal and
the import path: OS decode on desktop builds only, error in browser.

**Deliverable:** `docs/decisions/0068-heic-refusal.md`

**Stopping condition:** Decision doc committed; no HEIC code added.

---

### 2.3 Import surface parity with the export matrix (2–3 hr)

**Problem:** The CLI `import` command accepts jpg/png/webp. The
`io::raster::decode` supports bmp/tiff/gif/bmp/wbmp/pnm/ico. These lists
are misaligned. A user running `firecraft import image.tiff` gets an
"unsupported format" error even though the decoder handles TIFF.

**Deliverable:**
- Audit all format extensions accepted by `io::raster::decode`.
- Update CLI error strings to list all supported import formats.
- Add a test per format that verifies: `import --help` lists the format,
  or `import` with that extension produces a valid document (not an
  "unknown format" error).

**Stopping condition:** CLI `--help` format list matches what `decode`
actually handles; one test per newly-supported format.

---

### 2.4 Lossy WebP — decision already made, implementation remains (3–4 hr)

**Status:** Decision 0063 records the lossless VP8L path (landed). Lossy
VP8 (libwebp-sys) is the remaining question: feature-gated native-only,
WASM gets the "lossy WebP export not supported in browser" error.

**Deliverable:**
- Feature gate `lossy-webp` on the `image-webp` crate or a new
  `webp-encoder` crate that wraps libwebp-sys.
- WASM build does **not** enable `lossy-webp`.
- Native build enables `lossy-webp`.
- Decision doc 0063 already covers this — implementation follows that
  contract.

**Stopping condition:** `cargo build --release` produces a native binary
that exports lossy WebP; `trunk build --release` (WASM) succeeds without
libwebp-sys.

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

### 4.2 Smoke budget measurement (1 hr)

**Problem:** `--virtual-time-budget=60000` is a constant. If the WASM
bundle grows past ~15MB compiled, fetch+compile can exceed 60s on slow
CI runners (not virtual time — actual wall time).

**Action:** Measure wasm bundle size over time. If it approaches 15MB,
raise the virtual-time budget to match measured load time + 50% headroom.

**Current bundle:** `firecraft-web-0.2.2.wasm` — check size.
```bash
ls -lh dist/web/*.wasm
```

If >10MB: investigate. If >15MB: raise budget to measured time.

---

### 4.3 Platform recipes honest status (30 min)

The `packaging/` directory has AppImage, macOS dmg, Windows scripts that
have never been executed. The release notes correctly state "built and
verified on Linux x86_64".

**Action:** For each untested platform, add a "last verified" date and
the known blocker (e.g., "macOS: no macOS SDK on this host; no CI runner
for darwin target"). Do not ship untested platform artifacts.

**Deliverable:** A `packaging/STATUS.md` or comments in each script
describing the last-verified date and blocker.

---

## Phase 5: CI / infrastructure improvements (optional, high-value)

### 5.1 GitHub Actions — separate fmt check into its own job (1 hr)

**Problem:** The full gate runs in one `ci` job. If fmt fails, clippy and
tests never run. Separating fmt into its own first job saves 10+ minutes
of runner time per failed push.

**Approach:** Two-job CI:
1. `fmt` — runs `cargo fmt --all --check` (fast, ~30s)
2. `gate` — runs `cargo run -p xtask -- ci` excluding fmt (the rest)

Both run on all pushes. The `fmt` job fails fast.

---

### 5.2 Coverage step isolation (1 hr)

**Problem:** `cargo install cargo-llvm-cov --locked` runs on every CI
job (every push). Installing a binary takes ~2 min. The xtask `coverage`
step is the only thing that needs it.

**Approach:** Move `cargo-llvm-cov` install into a dedicated `coverage`
job that runs in parallel with the main gate. Or: use a composite action
that caches the installed binary.

---

### 5.3 Trunk caching (1 hr)

**Problem:** `cargo install trunk --locked` runs on every CI job.
Trunk is only needed for `dist --web`. It is installed in the `ci`
job but only used in the smoke sub-step (which only runs on tags).

**Approach:** Move trunk install into the `dist --web` step. Or: cache the
`~/.cargo/bin/trunk` binary between runs.

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
| Fix `.aegisignore` + aegis hang | 1 | 5 min | None |
| Publish v0.2.2 release | 1 | 2 min | None |
| Fix `appinfo.json` version stamp | 1 | 5 min | None |
| Update stale docs (PROGRESS/SESSIONS/HISTORY) | 1 | 30 min | None |
| Clarify smoke budget comment | 1 | 2 min | None |
| AVIF decode decision doc | 2 | 1–2 hr | None |
| HEIC refusal decision doc | 2 | 1 hr | None |
| Import surface parity | 2 | 2–3 hr | None |
| Lossy WebP implementation | 2 | 3–4 hr | Decision 0063 contract |
| Coverage baseline re-measure | 3 | 30 min | None |
| Coverage gap table | 3 | 1 hr | Baseline |
| WebOS packaging proof | 4 | 2–3 hr | Version stamp fix first |
| Smoke budget measurement | 4 | 1 hr | None |
| Platform recipes status doc | 4 | 30 min | None |
| Split CI fmt job | 5 | 1 hr | None |
| Trunk caching | 5 | 1 hr | None |
| 90% coverage floor raise | 3 | ongoing | Phase 2 first |
| 99% coverage plan | 6 | future | Phase 2 + Phase 3 |

---

## Summary: what to do first

1. **`sed 's/target/target/' .aegisignore`** — add `target/` to fix local hang
2. **`gh release edit v0.2.2 --draft=false`** — publish the draft
3. **`sed -i 's/@VERSION@/0.2.2/' packaging/webos/appinfo.json dist/web/appinfo.json`** — fix version stamp
4. Update docs (PROGRESS.md "Now", SESSIONS.md entry, HISTORY.md arc)
5. `git add -A && git commit -m "fix: resolve aegis local hang, close v0.2.2, update docs" && git push gitforge main && git push origin main`
6. Proceed to Phase 2 (image support decisions)
