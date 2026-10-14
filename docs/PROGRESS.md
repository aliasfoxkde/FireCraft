# FireCraft — Progress

> Live status board. Updated at every session close (see
> [`HANDOFF.md`](HANDOFF.md) §7). History and narrative:
> [`HISTORY.md`](HISTORY.md). Per-session log: [`SESSIONS.md`](SESSIONS.md).

## Now (2026-10-14)

- **Released:** v0.1.0 (2026-10-13), repo public.
- **Roadmap:** M0–M15 **all closed**.
- **Registry:** 206 commands. **CI:** green 7/7 on `f668d85`; later
  campaign pushes verified per batch (runbook: HANDOFF §6).
- **CS6 DOM parity:** **233 C / 38 P / 0 M / 85 N** — **the M table is
  empty** (2026-10-14): every CS6 dom mutation verb is carried or
  partial. Remaining surface work is the 38 P rows and the recorded
  debts ([`HANDOFF.md`](HANDOFF.md) §4 Phase 9).

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

## Post-roadmap parity campaign (the current work)

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
| #113 | web/group setter batch: setRectangle/setShape/copyToHotspot/setGroupType/setWebObjectsVisibility (8.9, decision 0043) + setImageSize preflight fix (000557d) | this batch | **233 C / 38 P / 0 M / 85 N** |

**Campaign complete:** no remaining M families. Next: P-class partials
and Phase 9 debts (HANDOFF §4).

## Recorded debts (open)

| Debt | Blocks / feeds | Origin |
|---|---|---|
| object.duplicate shallow-clones groups | fidelity | pre-#91 |
| Pattern-authoring command | fills UX | #85 |
| mkBT tile-bank **writer** | bitmap round-trip out | M7 |
| ~~Image resample pass~~ **DONE** — landed as `bitmap::resample_rgba` in #111 | setDocumentImageSize (closed, decision 0041) | M8 |
| mask_replace_options "add" (mask union fold) | clipPasteAsMask variant | #109 (decision 0039) |
| Path→image rasterizer (mask_type "image" on vector clipboard) | clipPasteAsMask variant | #109 (decision 0039) |
| Path→image rasterizer (filterSelection on non-image selection) | 8.8 bake family refuses whole | #110 (decision 0040) |
| Out-of-bounds effect growth (bake clips to the image surface) | tile-bank growth parity | #110 (decision 0040) |
| Shared-layer link-resolution render | shared layers render empty | #93 (decision 0022) |

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

(Pre-#91 counts are approximate; the matrix file is the authority.)
