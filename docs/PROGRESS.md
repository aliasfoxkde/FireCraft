# FireCraft — Progress

> Live status board. Updated at every session close (see
> [`HANDOFF.md`](HANDOFF.md) §7). History and narrative:
> [`HISTORY.md`](HISTORY.md). Per-session log: [`SESSIONS.md`](SESSIONS.md).

## Now (2026-10-14)

- **Released:** v0.1.0 (2026-10-13), repo public.
- **Roadmap:** M0–M15 **all closed**.
- **Registry:** 178 commands. **CI:** green 7/7 on `25fedbc`; later
  campaign pushes verified per batch (runbook: HANDOFF §6).
- **CS6 DOM parity:** **203 C / 45 P / 20 M / 88 N** — the 20 M verbs
  are the active campaign; the phased plan is [`HANDOFF.md`](HANDOFF.md) §4.

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
| #105 | view flags → session state, rows → N (Phase 4, decision 0035) | this batch | **203 C / 45 P / 20 M / 88 N** |

Remaining families (HANDOFF §4 order): button symbols (5,
evidence-gated) → 8.1 place-image trio → 8.6 clipboard masks/6.6 →
6.4 resample → 5.4 setDocumentImageSize.

## Recorded debts (open)

| Debt | Blocks / feeds | Origin |
|---|---|---|
| object.duplicate shallow-clones groups | fidelity | pre-#91 |
| Pattern-authoring command | fills UX | #85 |
| mkBT tile-bank **writer** | bitmap round-trip out | M7 |
| Image resample pass | setDocumentImageSize | M8 |
| Clipboard surface | clipPasteAsMask/Inside | M10 |
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

(Pre-#91 counts are approximate; the matrix file is the authority.)
