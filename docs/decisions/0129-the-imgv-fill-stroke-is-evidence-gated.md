# 0129 — The bitmap inspector's fill/stroke absence is format-level, not a code bug

- **Status**: accepted
- **Date**: 2026-11-19
- **Ticket**: [T22](tasks/T22-screenshot-ui.md) / Phase 22.4
- **Supersedes**: decision 0019 (transform evidence-gate)

## Decision

**Deferring bitmap fill/stroke in the bitmap inspector is the correct call.**
The `IMGv` (Image) node does not carry fill/stroke node children in the
vendored Fireworks node dictionary. The gap is format-level evidence,
not an absent code path. The inspector already refuses to fake fields
and the code contains an explicit comment recording the gap.

## Evidence

1. **Node dictionary** (`docs/research/fireworks-png-nodes.md`):
   `IMGv` carries `LCKb`, `XOFf`, `YOFf`, `XLCf`, `YLCf`, `WPXi`,
   `HPXi`, `TSZi`, `TILv`, `BLDi`, `EFLv`, `OPAi`, `VISb`, `OBNs` —
   but **zero** of `PATv`, `FCLi`, `BCLi`, `FPLv`, or any paint-style
   node. `IMGv` is a raster image node, not a painted shape.

2. **Doc model** (`crates/doc/src/model.rs`): `ImageObject` has no
   `PaintStyle` field, no `fill_raw`/`stroke_raw`. Compare `PathObject`
   which has `pub paint: Option<PaintStyle>`.

3. **Engine** (`crates/engine/src/catalog/object.rs`): `object.setFill`
   and `object.setStroke` only accept `ObjectData::Path` — they return
   `InvalidState` for `ObjectData::Image`.

4. **Inspector** (`crates/app/src/shell/panels.rs`):
   `image_properties()` shows pixel geometry, tile count, opacity,
   blend, visibility — but no fill/stroke pickers. The code contains an
   explicit comment: the CS6 fill/stroke editors are absent because
   `IMGv` carries no fill/stroke nodes.

5. **Compose** (`crates/compose/src/render.rs`): `Matte` tile type
   carries a solid pixel color that functions as tile content, not an
   object-level fill/stroke paint style.

## Consequences

- The bitmap inspector remains as-is: shows pixel geometry, tile data,
  and display controls, but not fill/stroke color pickers. This matches
  the format contract.
- CS6 Fireworks may have fill/stroke tools for bitmaps via the "Image
  Editing" panel — a separate panel not yet wired. Those are tracked
  separately and do not block phase 22.4 closure.
- If future evidence (real `.fw.png` files containing `PATv` children
  on `IMGv`) emerges, the `ImageObject` struct would need `PaintStyle`
  fields, the engine catalog would need a new arm for `SetImageFill` /
  `SetImageStroke`, and `panels.rs::image_properties` would gain color
  pickers — analogous to the existing path fill/stroke path.
- The existing comment in `panels.rs::image_properties()` already records
  this as "open work in `docs/CS6-PARITY.md` rather than faked." No
  code change needed.

## Status

**Accepted.** Phase 22.4 bitmap inspector is evidence-gated at the
format level. Closing this item as deferred with explicit placeholder
refusal.
