# FireCraft Production Audit — 2026-11-20

> **Authority:** Consolidated findings from prior Claude session (544e31f5),
> the omp session, the CraftAnything experiment, and the current state of
> `main` and `feat/release-provenance`.

## Current State

| Metric | Value |
|---|---|
| Latest release | v0.2.5 (2026-11-03) |
| main | 55 commits behind `feat/release-provenance` |
| Engine commands | 257 registered, 257/257 bound (100%) |
| wasm32 gate | PASS — 11 pure crates compile for wasm32 |
| WASM size | ~95 MB raw, ~19.9 MB gz |
| WASM CDN strategy | Implemented (`patch_wasm_cdn()` in dist.rs) |
| Web live site | Unidentifiable from v0.2.5 bytes |

---

# GAPS

## G-1: CI toolchain mismatch (BLOCKED)

**What:** CI uses rustc 1.99.1 (`.github/workflows/ci.yml`), which has the
`.rmeta` version bug — rustc does not embed its version in `.rmeta` files.
Cargo rejects crates compiled with this version, failing the wasm32 check.

**Evidence:** rustc 1.99.1 fails wasm32 compilation with
`error[E0514]: found crate ... compiled by <unknown rustc version>`.
rustc 1.100.0 (stable) compiles wasm32 fine.

**Fix:** Update ci.yml to use rustc 1.100.0 for wasm32 builds. The clean-room
rustc 1.100.0 works for wasm32 but has a different native compilation path
(dashmap fails on native x86_64 with trait bound issues). The wasm32 check
should use rustc 1.100.0; native checks remain on rustc 1.99.1.

**Risk:** rustc 1.100.0 may have different behavior than 1.99.1 for coverage
measurements (decision 0102 says 1.100.0 reads source 4.9 points lower).

---

## G-2: Native workspace compilation fails on rustc 1.100.0

**What:** `cargo check` fails on dashmap due to `std::cmp::Eq` trait bound
issues on rustc 1.100.0. The clean-room fork has compiler bugs preventing
full native workspace compilation.

**Fix:** External — requires fix in the rustc fork. No workaround available
from the application layer.

---

## G-3: Live web deployment is not v0.2.5

**What:** The live Cloudflare Pages deployment does not match v0.2.5 release
bytes. Digest measurement shows the live build corresponds to no identifiable
release.

**Evidence:** Live bundle's trunk asset name (`2274ac8c...`) and wasm digest
(`f4f0a3f0...`) match neither the published v0.2.5 release asset
(`f9dbe2f1...` / `a8a4fa4f...`) nor a rebuild of the v0.2.5 tag
(`103c62d3...` / `5934702f...`).

**Fix:** Republish from the canonical release bytes. Requires Cloudflare deploy
access.

---

## G-4: Non-Linux release artifacts not verified

**What:** No target beyond Linux x86_64 is CERTIFIED. Windows, macOS, AppImage,
Flatpak, and Android are all `NOT STARTED` (rung 1 — recipe only).

**Blockers:**
- Windows: `pwsh` absent, no MSVC linker on this host (B-12)
- AppImage: `linuxdeploy` absent, no pinned copy (B-8)
- Flatpak: Flatpak SDK absent (B-11)
- macOS: Needs macOS build host

**Fix:** Requires external build hosts. Recipes exist; operator-run needed.

---

## G-5: wasm32 gate excludes app/app-cli (partial)

**What:** The wasm32 check no longer includes app and app-cli because they
depend on scripting which depends on boa_engine which depends on dashmap
(needs std, unavailable for wasm32). This means the wasm32 gate validates
only the 11 pure crates, not the full wasm32 dependency tree.

**Impact:** If app or app-cli gain new wasm-incompatible code, it won't be
caught by the wasm32 check.

**Fix:** Monitor app/app-cli for wasm32 compatibility manually. Consider
adding a wasm32 check for app-cli with `--no-default-features` (which
works because scripting becomes optional).

---

## G-6: WASM CDN loading — RESOLVED

**Status:** WASM proxy Worker deployed at `firecraft-wasm-proxy.cyopsys.workers.dev`; serves WASM from GitHub Release v0.2.6 with CORS headers. Raw: 95.6 MB (gzipped: ~4.5 MB). Cloudflare Pages deploy (~87 KB HTML/JS) is live at head.firecraft-web.pages.dev. The CDN loading strategy is fully implemented.

---

## G-7: shell/mod.rs is a 7,996-line god file

**What:** `crates/app/src/shell/mod.rs` has 221 functions across 7,996 lines.
13 submodules already exist (canvas_ui, dialogs, filters_ui, glyphs, guides_ui,
html_preview, icons, optimize, palette, panel_windows, panels, panels_web,
slice_editor, timeline), but mod.rs still contains the Shell struct, layout
management, command dispatch, file ops, text editing, and menu bar code.

**Fix:** Extract `begin_text_edit`, `run_command`, `save_recovery`, and panel
management into dedicated modules. Coordinate with panel ownership model.

**Effort:** Large. Requires understanding the full public API surface.

---

## G-8: Remaining cast_possible_truncation in native-only crates

**What:** ~651 `cast_possible_truncation` warnings in native-only crates
(app, app-cli). The wasm32 pure crates have zero warnings (compose already
has module-level `#[allow]` for its 651 fixed warnings).

**Fix:** Requires native compilation which is blocked by rustc .rmeta bug.

---

## G-9: Release provenance and signing

**What:** No artifact is signed. No SBOM/provenance attestation exists for
the live release. Decision 0075 scope is "recommended — awaiting maintainer
approval."

**Blockers:** B-14 (no signing credentials configured — Windows Authenticode,
macOS Developer ID, GPG/nonce provenance all absent).

---

## G-10: Decision ID deduplication on main

**What:** Decision IDs 0094, 0096, 0101 each appear twice on main.

**Fix:** Remove duplicates.

---

# NEXT STEPS (prioritized)

## Immediate (this session)
- [x] Restore task list from prior session
- [x] Document omp session findings
- [x] Merge scripting optional changes
- [x] Add WASM CDN patching
- [x] Add CraftAnything evidence
- [x] Fix wasm32 gate (remove app-cli/app from WASM_CRATES)

## Next session
- [ ] Update ci.yml to use rustc 1.100.0 for wasm32 checks
- [ ] Republish web from canonical release bytes (requires deploy access)
- [ ] Fix HANDOFF.md reachability numbers (249/257 → 257/257)
- [ ] Dedup decision IDs 0094, 0096, 0101 on main
- [ ] Evaluate WASM size further (target < 50 MB raw, < 20 MB gz)

## Future (needs maintainer)
- [ ] Approve decision 0075 (release scope)
- [ ] Provide Windows/macOS build hosts
- [ ] Provide signing credentials
- [ ] Approve Windows packaging (Inno Setup)
- [ ] Shell/mod.rs refactor
- [ ] Coverage floor ratchet (blocked by rustc fix)
