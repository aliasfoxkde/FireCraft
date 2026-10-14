# Decision 0023 — document canvas color & resolution commands

Date: 2026-10-14
Closes: HANDOFF §4 Phase 5.1 (`setDocumentCanvasColor`) + 5.2
(`setDocumentResolution`) — parity rows `M` → `C`.

## Context

`BGCi`, `RESf`, and `RSUi` are parsed and rendered (77f55af paints the
`BGCi` matte under every layer walk; `RESf`/`RSUi` ride the reader and
writer) but nothing could set them. The CS6 verbs are
`dom.setDocumentCanvasColor(color)` and
`dom.setDocumentResolution({pixelsPerUnit, units})` — both
*document-scoped* (one setting in the UI), while the wire carries the
values twice: on the `MKBv` root canvas and on every page's `PDCv`
canvas.

## Decision

Two commands in a new `catalog/document.rs`, `document.*` prefix:

- `document.setCanvasColor {color_raw}` — one ABGR raw (house
  `color_raw` convention; the scripting layer maps CS6's color string
  to the raw before the command runs). Validated `0..=0xFFFFFFFF`.
- `document.setResolution {resolution, unit}` — `unit` is
  `"inch"` | `"cm"` mapping to `RSUi` 0 | 1 (dictionary values).
  Resolution must be finite and positive.

Both write the document canvas **and every page's canvas copy** so the
round-trip stays consistent and `compose`'s per-page matte follows the
UI's single setting. The delta captures `Delta::Info` plus one
`Delta::PageInfo` per page, each `before` taken before the first
mutation.

## Consequences

- Undo restores per-page values independently (each page carries its
  own captured `PageInfo`).
- A hand-edited file whose pages deliberately disagree on `BGCi` loses
  that disagreement the moment the color is set — consistent with CS6,
  where the setting is document-wide.
- No wire change: the writer already emits both copies.

## Verification

- Unit tests: document-wide apply + per-page undo, range/non-positive
  refusals, unit mapping.
- Fuzz: both ids in the CALLS table with payload builders.
- Parity: both rows flip to `C` in the next
  `docs/planning/generated/dom-parity.md` regeneration.
