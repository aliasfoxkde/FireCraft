# Decision 0049 — pixel-mask move + adjust (P rows → C)

**CS6 surface.** Two dom verbs over the active pixel selection:

- `dom.moveSelectionMaskBy(delta)` (Fireworks 4) — "Moves the current
  pixel mask by the specified amount. If there is no pixel selection,
  an exception is thrown."
- `dom.selectAdjustPixelSel(whatToDo, amount)` (Fireworks 3) —
  "Expands or reduces the pixel selection by the specified number of
  pixels, selects a border of pixels, or smooths the edge of the
  pixel selection." `whatToDo`: "expand", "contract", "border",
  "smooth".

## Design

- **The selection is a full-image coverage plane** (`SelectionMask`,
  one `0..=255` byte per pixel), so both verbs are pure plane
  operations — no image pixels move, no tiles load. Undo rides
  `selection_delta` like every other `bitmap.select*` command.
- **`moveSelectionMaskBy` translates coverage only.** Each plane byte
  re-reads from `(x − dx, y − dy)`; vacated area clears. This is the
  marquee move — the pixels under it move with `bitmap.movePixels`.
  Gates mirror the CS6 contract: no plane refuses (CS6 throws), a
  zero delta refuses (nothing would move), |delta| beyond 65535
  refuses (the plane-size bound).
- **Adjust is morphology over a 3-4 chamfer distance field.** One
  O(pixels) two-pass field per op (an axial step is 3 units, a
  diagonal 4 — Euclidean within ~8%), thresholded at `3 × amount`:
  - **expand**: every pixel within the radius of a selected one.
  - **contract**: selected pixels farther than the radius from any
    unselected one.
  - **border**: selected pixels within the radius of an unselected
    one (the edge band).
  - **smooth**: opening (contract, expand) then closing (expand,
    contract) at the same radius — specks and thin features go,
    pinholes fill.
- **The image edge is not a selection boundary.** Out-of-image
  neighbors contribute nothing to the field, so a plane with no
  unselected pixels (the full-image selection) is contract-smooth's
  identity. Recorded as the clean-room choice; CS6's behavior at the
  canvas edge is unverifiable from the docs.
- **The binary projection is what adjusts.** Feathers produce soft
  coverage; the morphological field thresholds on `alpha != 0` and
  writes hard 0/255 — adjusting a feathered selection collapses the
  gradient to hard coverage (recorded model limitation, like the
  single-run text note).
- **`amount` gates at 1..=65535.** 0 would be a no-op (the
  `bitmap.feather` radius-0 precedent), negatives never arise from
  CS6's UI (the doc's "any integer is acceptable" describes the
  parameter type, not behavior — negative refuses), and beyond 65535
  exceeds the plane bound. `whatToDo` is a serde enum of the four
  CS6 words — any other value fails payload decode.
- **`distance_field` is radius-independent** — the threshold compare
  takes any amount, so huge radii cost the same two passes.

## Consequences

- Registry 225 → **227**; parity 253 C / 18 P / 0 M / 85 N → **255 C
  / 16 P / 0 M / 85 N** (two flips).
- `bitmap.rs` gained `distance_field`/`expand_mask`/`contract_mask`/
  `border_mask`/`smooth_mask` — the first morphological machinery in
  the engine; `retouch`'s future refine passes can reuse the field.
- Remaining P rows: 16. The multi-run text cluster (9 rows) is the
  dominant block; then the anti-alias trio, `importFile`,
  `duplicateSymbolForAlias`, `setFillNColorNTexture`,
  `setAnimInstanceStartFrame` (permanent), and `setSliceAutonaming`
  (permanent).
