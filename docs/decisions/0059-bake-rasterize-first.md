# Decision 0059 — filterSelection rasterizes non-image selections

**Debt closed.** The bake family (`filterSelection` trio +
`convolveSelection` + `motionBlurSelection`) refused any non-image
selection: "object is not an image — CS6 would rasterize it first
(recorded debt)" (decision 0040). CS6 converts the object to a
bitmap, then applies the filter.

## Design

- **Convert in place, then filter.** `bake_core` resolves non-image
  targets (decision 0059's `BakeTarget::Convert`) by rendering the
  object's own pixels through `compose::render_subset` — the same
  per-object renderer the full walk and the flatten bakes use — in
  the read-only resolve pass. The transform pass runs the filter over
  that render exactly as it would over an assembled image, then the
  slot mutates in place: `ObjectData::Path | Text | Group |
  SymbolInstance` → `ObjectData::Image`, same registry id, same
  parent position. The undo inverse is `ObjectInverse::Mutated` — the
  original object rides the delta and a single undo restores it.
- **The image spans the canvas at the origin** — the flatten family's
  bake posture (decision 0031), not CS6's object-bounds crop. Pixels
  outside the object are transparent, so the render is identical;
  cropping to CS6's shape would need rendered-extent logic
  (control-point bounds understate stroke spill) for a purely
  cosmetic tile saving. Recorded as a posture deviation, not a debt.
- **The baked pixels carry the old weight.** The render composites
  the object's own opacity/blend/effects into the pixels, so the
  converted image resets to opacity 1000, blend 0, no effects — the
  same accounting mergeDown applies to the bitmap below.
- **Slices refuse.** They are web annotations (`URLv`), not
  rasterizable content; CS6's filter menu does not reach them. The
  refusal mutates nothing (resolve pass order).
- **The page resolves through the ownership walk.** `render_subset`
  needs a page; `owning_page` walks `find_owner` up `Owner::Group`
  links to the owning cell, so group-nested selections convert too.
  An unplaced id carries no page and refuses (the orphan posture).
  Hidden objects render as nothing and convert to a transparent
  image — the render posture (hidden contributes nothing), not a
  special case.
- **The mask gate maps through the location.** `blend_through_mask`
  takes the image location instead of the whole `ImageObject`; a
  conversion's canvas-spanning image at the origin maps canvas-space
  mask coordinates through unchanged.

## Consequences

- Registry unchanged; parity table unchanged (the rows were already C
  with recorded deviations — decision 0040).
- Tests: +2 — a filled path converts in place and inverts (fill
  inverted inside the square, transparent outside, undo restores the
  path and retracts the tiles); the same through a group wrapper
  (ownership walk) plus the slice refusal (nothing mutated).
- Remaining Phase 9 debts: pattern-authoring command, shared-layer
  link render, importFile resample quality.
