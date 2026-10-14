# Decision 0058 — pasteAsMask "add" unions the coverages

**Debt closed.** `clipboard.pasteAsMask` refused
`mask_replace_options "add"` as recorded debt (decision 0039). CS6:
`add` — "if an element mask already exists, add the pasted mask to
it" (`dom.clipPasteAsMask` argument doc, the `/var/tmp` ext extract).

## Design

- **The union is a coverage-plane max-fold.** Both operands reduce to
  a `SelectionMask` over the *target object's* bounds
  (`bounds_of` — the extent the mask applies to): the existing mask
  through the new `mask_coverage` helper, the incoming clipboard
  element through the same helper, then
  `slot = slot.max(gray)` pointwise and `plane_mask_image` mints the
  folded plane as a fresh grayscale tile grid. CS6's "add" is a union
  of opaque areas; the max over coverage is exactly that, in the
  engine's binary-coverage terms.
- **`mask_coverage` reads both mask kinds.** A `Path` stamps through
  the shared `stamp_path` (decision 0057's rasterizer). An `Image`
  must overlay the target exactly — dims and location must match the
  target bounds, else the command refuses (the CS6
  `whatIfResolutionDifferent` resample argument is unmodeled and
  stays out: silently resampling would invent pixels). Any other kind
  refuses. The alpha channel is the coverage; color is ignored, which
  is what a grayscale mask means.
- **The result is always a grayscale image mask.** Regardless of
  either operand's kind (path or image, on either side) the folded
  mask child is an `ObjectData::Image` and the group flips to
  `GRAYSCALE_MASK` (1) — after a union there is no vector geometry
  left to keep a `SHAPE_MASK` honest.
- **Undo carries the bank.** The fold mints tile ids, so
  `Delta::Bank { added }` rides last in the existing-mask return
  path — the reversed group fold drops the tiles after the old mask
  object revives and the fresh one retracts.
- **"add" with no existing mask is a plain paste.** CS6's "add" only
  means something when a mask is already there; the fall-through is
  the ordinary fresh-group paste path.

## Consequences

- Registry unchanged; parity table unchanged (the row was already C
  with a recorded deviation — decision 0039).
- Tests: +1 — an existing top-band image mask over a path clipboard,
  "add" unions: the band's pixels survive (existing contributed), the
  path's interior is opaque (incoming contributed), the corner is
  transparent (neither); undo restores exact bytes and retracts the
  minted tiles while the old mask object revives; a mismatched
  non-overlay image refuses whole.
- Remaining Phase 9 debts: pattern-authoring command, bake-site
  rasterize-first (object conversion), shared-layer link render,
  importFile resample quality.
