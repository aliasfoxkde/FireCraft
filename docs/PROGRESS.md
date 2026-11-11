# FireCraft — Progress

> Current snapshot. For the authoritative next-work list see
> [`TASKS.md`](TASKS.md); phase detail is in [`NEXT.md`](NEXT.md).
> Historical narrative: [`HISTORY.md`](HISTORY.md) and
> [`SESSIONS.md`](SESSIONS.md).

## Snapshot (2026-11-07, PR #8 branch; based on `main` at `5fac7a2`)

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:96efdab11c7df4b57fb72854f016b25100c7ae63868aab068c0acecdf0dbaf8c -->

- **Release:** `v0.2.5` is the latest tag. `main` has additional work
  since that tag. The public Pages manifest was checked on 2026-11-04
  20:01 local time; its JavaScript, wasm, and index hashes differ from
  the verified local optimized distribution. The deployed source
  revision is unknown.
- **Build evidence:** the earlier optimized `dist/web/` distribution,
  built 2026-11-04 16:45 local time, had a 21,274,323-byte wasm
  (below the 21,500,000-byte ceiling). It was built from `56ee1bf`,
  whose Git tree matches `5fac7a2`; all five entries in its
  `MANIFEST.sha256` verified. The deployment comparison and earlier
  bridge smoke covered that artifact. The later [T22 rebuild](tasks/T22-screenshot-ui.md#receipts--2026-11-04)
  used `5fac7a2` plus uncommitted dock and inspector changes, producing
  a 21,271,259-byte optimized wasm in `dist/web/`; T22's six browser
  captures and interaction checks covered this rebuilt artifact.
  The 18:21 `web/dist/` output was a raw 95,583,065-byte wasm bundle,
  not a release distribution. Neither directory is tracked by Git.
- **CI evidence:** PR #7's GitForge 8-job receipt and GitHub Actions
  gate are recorded in the session notes. PR #8 review remediation now
  has targeted regression tests in the branch, but this audit environment
  cannot execute the Rust suite; the current PR head has no GitHub Actions
  run/status receipt visible. Do not treat the new tests as passing until
  the repository CI or a local `cargo run -p xtask -- ci` completes.
- **Engine and parity:** 245 command IDs (source is authoritative);
  268 carried / 3 partial / 0 missing of the 356 DOM verbs.
- **Menu reachability:** 2 of 296 rows currently use `Action::Inert`; eight additional rows have explicit feature/evidence gates. The
  19.6 campaign (batches 1–17) is complete, and the Window menu's
  panel rows (including Optimize, CSS Properties, Find and Replace,
  and Common Library) all dispatch; the remaining rows are
  feature work or explicit evidence gates. Census source:
  `menus::tests::inert_worklist`; ceiling:
  `menus::tests::inert_count_never_rises`.
- **Current UI priority:** Phase 22.1 (panel inventory) and 22.2
  (tool strip) are complete in this PR branch; 22.5 (CS6 chrome audit)
  remains partial. The screenshot acceptance work is complete, with
  1280×800 and 1920×1080 captures and empty/vector/bitmap states
  recorded in [T22](tasks/T22-screenshot-ui.md).
- **Completed foundations:** M0–M15, DOM verb-surface campaign,
  Phases 19.1–19.5, 19.6 menu wiring, and 19b. Three partial DOM
  rows and the other evidence-gated items remain parked pending new
  evidence.
- **Clean-room boundary:** local CS6 files and Adobe-derived extracts
  remain uncommitted; visual assets require attribution.

## Build and deployment evidence

| Artifact or check | Observed | Meaning |
|---|---|---|
| Earlier optimized `dist/web/` | 2026-11-04 16:45 local; 21,274,323-byte wasm; manifest verified | Build from `56ee1bf`, whose Git tree matches `5fac7a2`; deployment comparison and earlier bridge smoke |
| T22 optimized rebuild in `dist/web/` | 2026-11-04; 21,271,259-byte wasm | `5fac7a2` plus dock and inspector changes; six browser captures and interaction checks in [T22](tasks/T22-screenshot-ui.md#receipts--2026-11-04) |
| Raw `web/dist/` | 2026-11-04 18:21 local; 95,583,065-byte wasm | Development output; not the optimized distribution |
| Browser smoke | Earlier optimized artifact: bridge smoke in session history; T22 rebuild: six captures and interaction checks in T22 | Local end-to-end proof for each recorded artifact; live deployed revision still unverified |
| Live Pages | Manifest fetched 2026-11-04 20:01 local | JS/wasm/index hashes differ from the earlier 21,274,323-byte optimized distribution; deployed revision unknown |

<!-- /coderabbit-ai-generated:v1 -->

## Milestones

| Milestone | Scope | Closed | Notes |
|---|---|---|---|
| M0 | Workspace bootstrap + method | 2026-10-10 | xtask gates, layers enforcement |
| M1 | Foundations (geom, fuzz anchors) | 2026-10-10 | corpus fallback: no Wine |
| M2 | fwpng read path + corpus | 2026-10-10 | |
| M3 | Doc model + fwmap round-trip | 2026-10-10 | rising round-trip floors |
| M4 | Compositor + golden gate | 2026-10-10 | compose crate, bench baseline |
| M5 | Catalog v1 + app shell + web trunk | 2026-10-10 | tasks #42–#46 |
| M6 | Vector depth (Auto Shapes, reshape, align) | 2026-10-11 | tasks #48–#50 |
| M7 | Text + bitmap filters | 2026-10-11 | tasks #51.x |
| M8 | Import/export (SVG, PSD, GIF, EXPv, batch) | 2026-10-12 | tasks 3–8 |
| M9 | Scripting (sandbox, DOM projection, .jsf) | 2026-10-12 | tasks 1–8 |
| M10 | Symbols (rich symbols, library I/O) | 2026-10-12 | tasks 5–7 |
| M11 | Web layer + export (HTML/CSS/slices) | 2026-10-12 | tasks 1–6 |
| M12 | Parity hardening + release matrix | 2026-10-12 | perf, memory, dist |
| — | **v0.1.0 released** | 2026-10-13 | repo public |
| M13 | Deep RE evidence + CS6 shell parity | 2026-10-13 | interaction parity + scorecard |
| M14 | Live-filter parity | 2026-10-13 | |
| M15 | Tool-surface parity | 2026-10-13 | tasks 5–6 + gaps #82–#85 |
| — | **v0.2.0 released** | 2026-10-15 | shell/tools/filters/styles; web bundle broken (Phase 10) |
| — | **v0.2.1 released** | 2026-10-15 | web bridge + repaint + honest smoke fix |
| — | **v0.2.2 released** | 2026-10-19 | AVIF/HEIC decisions, WebOS dry-run, smoke budget, 54 tests |
| — | **v0.2.3 released** | 2026-10-24 | Phase 11.4 pixel selection + marching ants, paint/IO batch, CI image re-ship |
| — | **v0.2.4 released** | 2026-11-02 | 19.6 selection/mask reachability, Lucide icons, context menu, web session persistence |
| — | **v0.2.5 tagged** | 2026-11-03 | Latest tag; `main` has post-tag shell fidelity work |

## Post-roadmap parity campaign (completed)

| Task | Family | Commit | Parity after |
|---|---|---|---|
| #86 | DOM verb-surface sweep (matrix created) | — | baseline |
| #87 | object.group / object.ungroup | 96aa656 | — |
| #88 | page guide/grid/onion commands | 32dd03a | — |
| #89 | frame management commands | 8187375 | — |
| #90 | object transform family | — | — |
| #91 | object mask family | faa738d | 162 C / 45 P / 67 M |
| #92 | layer/page structure family (7 commands, decision 0021) | e830802 (+ 535340f test fix) | 169 C / 45 P / 60 M |
| #93 | shared-layer model + sharing commands (decision 0022) | 25fedbc | **172 C / 45 P / 57 M** |
| #94 | handoff/progress/session docs + campaign plan | a1e0e9c | — |
| #95 | document canvas color + resolution setters (5.1/5.2) | 4b813ef | 174 C |
| #96 | object-selection model + selection basics (Phase 1) | cc85956 | 177 C |
| #97 | hierarchy selects + bitmap selectSimilar | 135cb6b | 180 C |
| #98 | selection-to-frames fans (Phase 2, decision 0026) | ba60816 | 183 C |
| #99 | rect primitive params + quadrangle; autoshape redraw (6.1/6.2, decisions 0027/0028) | 93c59b4, 614bc64 | 188 C |
| #100 | slice/hotspot point append (8.2, decision 0029) | 3516d89 | 190 C |
| #100b | cropSelection, fill-edge mode, symbol tween (8.5, decision 0030) | df8fe42 | 195 C |
| #101 | flatten family via compositor bakes (Phase 3, decision 0031) | 235e4d4 | 198 C (4 rows, mergeDown case pair) |
| #102 | knife + splitPaths (8.3, decision 0032) | 3baf178 | 201 C / 45 P / 26 M |
| #103 | text.convertToPaths (8.4, decision 0033) | 56c6f2f | **202 C / 45 P / 25 M** |
| #104 | document.rotate — page rotation re-bake (5.3, decision 0034) | 48ec811 | 203 C / 45 P / 24 M |
| #105 | view flags → session state, rows → N (Phase 4, decision 0035) | a363f0f | 203 C / 45 P / 20 M / 88 N |
| #106 | button symbols: 4 behavior booleans + slice options (Phase 7, decision 0036) | 3d74723 | **208 C / 45 P / 15 M / 88 N** |
| #107 | place-image trio + shared assemble/retile helpers (8.1, decision 0037) | 7ea2288 | **212 C / 45 P / 12 M / 88 N** |
| #108 | saved-selections trio (8.7, decision 0038) | f668d85 | **215 C / 45 P / 8 M / 88 N** |
| #109 | clipboard surface: copy/cut/paste + paste-mask trio (8.6/6.6, decision 0039) | 1d1c8dc | **221 C / 45 P / 5 M / 85 N** |
| #110 | filterSelection bake trio over compose kernels (8.8, decision 0040) | 3f67491 | **225 C / 45 P / 1 M / 85 N** |
| #111 | document.setImageSize — bilinear resample, the last M row (5.4, decision 0041) | a4455d6 | **226 C / 45 P / 0 M / 85 N** |
| #112 | convolveSelection + motionBlurSelection bake commands (8.8 ext., decision 0042) | 8eae611 | **228 C / 43 P / 0 M / 85 N** |
| #113 | web/group setter batch: setRectangle/setShape/copyToHotspot/setGroupType/setWebObjectsVisibility (8.9, decision 0043) + setImageSize preflight fix (000557d) | 12b5e40 | **233 C / 38 P / 0 M / 85 N** |
| #114 | anim instance parameter family: 6x setAnimInstance* over the ANNi carrier (8.10, decision 0044) | d98764b | **239 C / 32 P / 0 M / 85 N** |
| #115 | text rectangle trio: setTextRectangle / Auto / AutoFromPoint via TextEngine::auto_block (8.11, decision 0045) | 8f2919d | **242 C / 29 P / 0 M / 85 N** |
| #116 | canvas trim pair + deleteAllInDocument: extents/selection trim over XLCf/YLCf, per-element delete fold (8.12, decision 0046) | 51a3c3e | **245 C / 26 P / 0 M / 85 N** |
| #117 | loose-ends setters: setEffectName walker, addSwapImageBehaviorFromPoint (even-odd polygon hit, kind 2 + swapTarget), button-text replacement pair (8.13, decision 0047) | d482b5d | **250 C / 21 P / 0 M / 85 N** |
| #118 | on-path setters: setTextOnPathMode/Offset + reversePathTextDirection over a shared selection walk (8.14, decision 0048) | a0304d4 | **253 C / 18 P / 0 M / 85 N** |
| #119 | pixel-mask pair: moveSelectionMaskBy (plane translate) + adjustPixelSel (expand/contract/border/smooth via chamfer distance field) (8.15, decision 0049) | this batch | **255 C / 16 P / 0 M / 85 N** |
| #121 | paragraph setter trio: setTextParaIndent/SpacingBefore/After over the mapped PINf/PSBf/PSAf style fields (8.16, decision 0050) | this batch | **258 C / 13 P / 0 M / 85 N** |
| #122 | multi-run text model + markup verbs: TFSv marker-stream fold, setTextRuns + markup quartet (8.17, decision 0051) | this batch | **263 C / 8 P / 0 M / 85 N** |
| #123 | custom anti-alias dials: overSample {4,8,16} + sharpness/strength 0..=255; TA1i/TA2i/TOSi typed out of the bag (8.18, decision 0052) | this batch | **266 C / 5 P / 0 M / 85 N** |
| #124 | duplicateSymbolForAlias: one duplicate per pointed-to MSNi, selected instances re-point (8.19, decision 0053) | this batch | **267 C / 4 P / 0 M / 85 N** |
| #125 | importFile placement: decoded RGBA in the payload, degenerate/aspect/fill semantics, canvas crop (8.20, decision 0054) | this batch | **268 C / 3 P / 0 M / 85 N** |
| #126 | mkBT tile-bank load/write: open joins the bank, save merges fresh ids (`encode_mkbt` split law), library export carries rasters (Phase 9, decision 0055) | 2a6abf6 | 268 C (no surface change) |
| #127 | object.duplicate deep-clones groups: `clone_subtree` mints a slot per live descendant, undo retracts all (Phase 9, decision 0056) | 8b06184 | 268 C (no surface change) |
| #128 | pasteAsMask mask_type "image" rasterizes a vector clipboard into the mask bitmap (Phase 9, decision 0057) | this batch | 268 C (no surface change) |
| #129 | pasteAsMask mask_replace_options "add" unions the coverages (mask_coverage max-fold, Phase 9, decision 0058) | this batch | 268 C (no surface change) |
| #130 | bake family rasterizes non-image selections in place (render_subset → slot swap, Phase 9, decision 0059) | this batch | 268 C (no surface change) |
| #131 | shared-layer links render their definitions in the link's z-position (compose walk, decision 0060) | this batch | 268 C (no surface change) |
| #132 | importFile resamples bilinearly in the premultiplied domain (centre-point convention, exact u128; decision 0061) | this batch | 268 C (no surface change) |
| #133 | `path.setPatternFill` authors a pattern fill from an image (flatten minting law; decision 0062) | this batch | 268 C (registry 241 at write time; live count 243 — see note below) |
| #134 | WebP export (lossless VP8L through `image-webp`, zero new deps; decision 0063) | this batch | — (enhancement) |
| #135 | 19.1 transform handles + Numeric Transform dialog (canvas competence P0) | 7329087 | inert 181 → 178 |
| #136 | shell split: `shell.rs` (5.9k) → `shell/` modules (mod, glyphs, filters_ui, dialogs, panels, canvas_ui) | 32a110d | — (structure) |
| #137 | 19.2 Subselect path-node editing (anchors + handles, `path.movePoints`, one "Reshape" undo step) | 5e4f0ce | — |
| #138 | 19.3 gradient fill editor + on-canvas stop handles (`path.setFill` / `setFillVector`) | 1c84ee8 | — |
| #139 | 19.4 command palette (Ctrl+P) over `engine::catalog::ids()` + fuzz-shared payload templates | 4458f3a | 100% registry coverage |
| #140 | 19.5 grid + guides overlays, drag-out guides, snap-to, Edit Grid dialog | eb221ac | inert 178 → 171 |
| #141 | rendering fixes: crisp zoom (`compose::render_scaled`), ellipse shape, ABGR picker colors | 6e35c1e | — (defects) |
| #142 | 19.6 batch 1: selection/mask reachability, context menu, Lucide icons, web session persistence | e9ece7d | inert 171 → 144 |

**Campaign complete** (2026-10-14): no remaining M families; the
Phase 9 debt list closed the same day (#126–#132). The 3 P rows are
terminal/evidence-gated (HANDOFF §4 Phase 12). Current UI work is
Phase 22; rows #135+ are the shell-reachability/fidelity program, not
parity rows.

## Recorded debts (open)

| Debt | Blocks / feeds | Origin |
|---|---|---|
| ~~object.duplicate shallow-clones groups~~ **DONE** — deep clone landed in #127 (decision 0056) | fidelity | pre-#91 |
| ~~Pattern-authoring command~~ **DONE** — `path.setPatternFill` landed in #133 (decision 0062) | fills UX | #85 |
| ~~mkBT tile-bank **writer**~~ **DONE** — landed as open-join + save-merge + extract-copy in #126 (decision 0055) | bitmap round-trip | M7 |
| ~~Image resample pass~~ **DONE** — `bitmap::resample_rgba` in #111; importFile quality closed in #132 (decision 0061) | setDocumentImageSize (closed, decision 0041) | M8 |
| ~~mask_replace_options "add" (mask union fold)~~ **DONE** — landed in #129 (decision 0058) | clipPasteAsMask variant | #109 (decision 0039) |
| ~~Path→image rasterizer (mask_type "image" on vector clipboard)~~ **DONE** — landed in #128 (decision 0057); bake-site half remains | clipPasteAsMask variant | #109 (decision 0039) |
| ~~Path→image rasterizer (filterSelection on non-image selection)~~ **DONE** — landed in #130 (decision 0059) | 8.8 bake family | #110 (decision 0040) |
| Out-of-bounds effect growth (bake clips to the image surface) — **evidence-gated**: the live render already grows (compose applies kernels canvas-spanning, M14); the destructive bake clips by settled posture (0031/0040/0059). Reopening needs a CS6 filter/flatten bounds oracle (Wine corpus) — any growth rule without one is invented | tile-bank growth parity | #110 (decision 0040) |
| ~~Shared-layer link-resolution render~~ **DONE** — landed in #131 (decision 0060) | shared layers render empty | #93 (decision 0022) |
| Phase 22 shell fidelity; screenshot and responsive UI audit | UI/UX | TASKS.md / tasks/T22-screenshot-ui.md |

## Parity timeline

| Date | C | P | M | Event |
|---|---|---|---|---|
| 2026-10-13 | ~148 | 45 | ~90 | matrix created (#86 sweep) |
| 2026-10-14 | 162 | 45 | 67 | #91 mask family |
| 2026-10-14 | 169 | 45 | 60 | #92 structure family |
| 2026-10-14 | 172 | 45 | 57 | #93 shared layers |
| 2026-10-14 | 195 | 45 | 34 | #95–#100b (selection, frames, primitives, misc) |
| 2026-10-14 | 201 | 45 | 26 | #101 flatten + #102 knife/split |
| 2026-10-14 | **202** | 45 | **25** | #103 convertToPaths |
| 2026-10-14 | **203** | 45 | **24** | #104 rotateDocument |
| 2026-10-14 | 203 | 45 | **20** | #105 view flags → N (session state) |
| 2026-10-14 | **208** | 45 | **15** | #106 button symbols (decision 0036) |
| 2026-10-14 | **212** | 45 | **12** | #107 place-image trio (decision 0037) |
| 2026-10-14 | **215** | 45 | **8** | #108 saved selections (decision 0038) |
| 2026-10-14 | **221** | 45 | **5** | #109 clipboard surface (decision 0039) |
| 2026-10-14 | **225** | 45 | **1** | #110 filter bake family (decision 0040) |
| 2026-10-14 | **226** | 45 | **0** | #111 setImageSize — M table empty (decision 0041) |
| 2026-10-14 | **228** | 43 | 0 | #112 convolve + motionBlur bakes (decision 0042) |
| 2026-10-14 | **233** | 38 | 0 | #113 web/group setters + setImageSize preflight fix (decision 0043) |
| 2026-10-14 | **239** | 32 | 0 | #114 anim instance params over ANNi (decision 0044) |
| 2026-10-14 | **242** | 29 | 0 | #115 text rectangle fits (decision 0045) |
| 2026-10-14 | **245** | 26 | 0 | #116 canvas trim + delete-all (decision 0046) |
| 2026-10-14 | **250** | 21 | 0 | #117 loose-ends setters (decision 0047) |
| 2026-10-14 | **253** | 18 | 0 | #118 on-path setters (decision 0048) |
| 2026-10-14 | **255** | 16 | 0 | #119 pixel-mask pair (decision 0049) |
| 2026-10-14 | **258** | 13 | 0 | #121 paragraph setter trio (decision 0050) |
| 2026-10-14 | **263** | 8 | 0 | #122 multi-run text + markup verbs (decision 0051) |
| 2026-10-14 | **266** | 5 | 0 | #123 custom anti-alias dials (decision 0052) |
| 2026-10-14 | **267** | 4 | 0 | #124 duplicateSymbolForAlias (decision 0053) |
| 2026-10-14 | **268** | 3 | 0 | #125 importFile placement (decision 0054) |
| 2026-10-14 | 268 | 3 | 0 | #126 mkBT tile-bank load/write (decision 0055, no surface change) |
| 2026-10-14 | 268 | 3 | 0 | #127 duplicate deep-clones groups (decision 0056, no surface change) |
| 2026-10-14 | 268 | 3 | 0 | #128 vector→image-mask rasterization (decision 0057, no surface change) |
| 2026-10-14 | 268 | 3 | 0 | #129 mask "add" union fold (decision 0058, no surface change) |
| 2026-10-14 | 268 | 3 | 0 | #130 bake rasterize-first (decision 0059, no surface change) |
| 2026-10-14 | 268 | 3 | 0 | #131 shared-layer link render (decision 0060, no surface change) |
| 2026-10-14 | 268 | 3 | 0 | #132 importFile bilinear resample (decision 0061, no surface change) |
| 2026-10-14 | 268 | 3 | 0 | #133 pattern authoring (decision 0062, registry 241) |
| 2026-10-14 | 268 | 3 | 0 | #134 WebP export (decision 0063, enhancement) |

(Pre-#91 counts are approximate; the matrix file is the authority.
"Registry N" rows are point-in-time: `engine::catalog::ids()` is the
live authority and reads 243 as of 2026-10-29 — the same-day verb
batches #135+ (text markup/AA dials, symbol alias, importFile)
registered the remainder after row #133 was written. GAPS.md §1
already carries the 243 figure.)

