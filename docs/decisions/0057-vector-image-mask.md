# Decision 0057 — mask_type "image" rasterizes a vector clipboard

**Debt (half closed).** `clipboard.pasteAsMask` refused
`mask_type "image"` when the clipboard held a path — "needs a
rasterizer hook (not wired)" — decision 0039's deviation 4. CS6
rasterizes the vector into the image mask: `dom.clipPasteAsMask` with
an image-kind mask turns a vector clipboard element into a grayscale
mask bitmap.

## Design

- **The rasterizer already existed; the hook was missing.**
  `mask.setMode "mask to image"` converts a path mask into a grayscale
  mask bitmap through `stamp_path` (path coverage, chord-closed open
  contours, no anti-aliasing — the CS6 binary posture) then
  `plane_mask_image` (coverage → gray RGBA8 tile grid, fresh ids).
  `paste_as_mask_core` now runs the same conversion: the path
  rasterizes over the *target's* bounds (`bounds_of`, the extent the
  mask applies to) with the negated origin as offset, and the
  resulting image replaces the clipboard element as the mask child.
  The helpers widen to `pub(crate)`; no logic moves.
- **The group becomes a grayscale mask.** `GRTi` follows the kind:
  `GRAYSCALE_MASK` (1) instead of `SHAPE_MASK` (2) — the render-side
  kind switch already keyed on the child's kind; now the kind and the
  child agree.
- **Undo carries the bank.** The conversion mints tile ids, so both
  pasteAsMask return paths append `Delta::Bank { added }` last —
  the reversed group fold drops the tiles after the mask object
  retracts. Empty-bank documents and documents with tiles both
  behave (`next` allocates from `max+1`).
- **Disjoint geometry is a valid outcome, not an error.** A path
  outside the target's bounds produces an all-zero coverage plane —
  content fully hidden everywhere — the same result the mask render
  gives a disjoint path mask today. No special case.
- **The other half of the debt stays refused.** `filterSelection` on a
  non-image selection (bake.rs) keeps its recorded refusal: applying a
  pixel filter to a vector object in CS6 *converts the object to a
  bitmap* — an object-kind mutation that deserves its own decision
  (identity, selection, and delta shape), not a rider on this one.
  The debt row narrows accordingly.

## Consequences

- Registry unchanged; parity table unchanged (the row was already C
  with a recorded deviation — decision 0039).
- Tests: +1 — copy path, pasteAsMask `mask_type "image"`: the group is
  a grayscale mask, the mask child is an image spanning the target's
  bounds with majority-opaque coverage, undo restores exact bytes and
  retracts the minted tiles; `mask_type "vector"` still lands a shape
  mask with a path child.
- Remaining Phase 9 debts: pattern-authoring command, mask union fold,
  bake-site rasterize-first (object conversion), shared-layer link
  render, importFile resample quality.
