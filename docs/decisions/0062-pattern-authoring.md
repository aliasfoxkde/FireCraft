# Decision 0062 — `path.setPatternFill` authors a pattern from an image

**Debt closed** (HANDOFF §4, origin #85): pattern fills load, render,
place, and serialize end to end (decision 0055), but nothing could
*create* one — the CS6 Fill panel's pattern authoring is UI-only (it
writes `pathAttributes.fill` directly; the DOM has no pattern verb,
the parity matrix carries exactly one pattern row, `setFillPlacement`,
and it is C). This is the command surface for that property write.

## Design

- **`path.setPatternFill { id, source, name? }`.** The target path
  gains a `FillShape::Pattern` fill whose `FPTv` tile is the source
  image's raster at its native pixel extent; `name` defaults to the
  image's `OBNs`, else "Pattern". The fill-vector handles are left
  unset — the placement stays identity (raster anchored at the canvas
  origin, unscaled), which `path.setFillVector` and `setFillPlacement`
  move; those rows are already C.
- **No new minting law.** The source assembles through
  `bitmap::assemble_rgba` (premultiplied tile-domain view, missing
  bank tile refuses), tiles through `bitmap::retile_rgba` (fresh
  `next_tile_id` slots, tail slots padded transparent) — the flatten
  minting law verbatim. Authored tiles are "fresh ids", so decision
  0055's save-merge serializes them with no writer change, and
  library export carries them the same way it carries flatten bakes.
- **Undo is a group.** The path capture (`ObjectInverse::Mutated`)
  rides first, `Delta::Bank { added }` rides LAST — undo restores the
  paint and drops the minted slots in one step, matching the
  flatten/place-image law.
- **Refusals (all pre-flighted before the first mutation).** Target
  not a path; source not an image; source extent outside 1..=8192 on
  either axis — the same bound `compose::pattern_brush` enforces at
  render, enforced at write so an authored pattern can never render
  as a surprise refusal.

## Consequences

- Registry 240 → 241. Parity table unchanged (the debt was recorded
  as a Phase 9 limitation, not a parity row).
- Tests: +4 — authoring mints one bank tile per grid slot and
  installs the fill (undo drops the tiles and restores the paint,
  redo re-mints); a non-path target and a non-image source refuse
  without touching the bank; an over-extent source refuses; an
  authored pattern renders through compose (the path shows the
  source raster's pixels, not the solid fallback).
- Phase 9 debt list is now empty. Remaining parity work: the 3 P
  rows (setFillNColorNTexture evidence-backed; setAnimInstanceStartFrame
  and setSliceAutonaming permanent).
