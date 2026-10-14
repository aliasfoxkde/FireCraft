# Decision 0054 — importFile placement (P row → C)

**CS6 surface.** `dom.importFile(fileURL, boundingRectangle,
bMaintainAspectRatio)` (Fireworks 3): "Imports the specified file at
the specified location." A rectangle with `left == right` and
`top == bottom` brings the file in unscaled with its top-left corner
at the point (the ratio argument ignored); `bMaintainAspectRatio`
true scales to the largest fit inside the rectangle that keeps the
file's ratio ("a handy option for creating thumbnails"); false
stretches to fill.

## Design

- **Content in the payload, IO at the boundary — the
  `library.importSymbol` law.** The engine command registry is
  pure and fuzz-driven; a `file://` read inside `apply` would be an
  untestable side channel. The shell (CLI/scripting) decodes the
  file with the `io` crate's importers and hands the command the
  decoded RGBA plane plus its source dimensions; `bitmap.importFile`
  owns the documented placement semantics. The dom's file argument
  and our pixel argument are the same verb split at the sanctioned
  seam.
- **Placement is the pixel-object law.** The scaled extent
  intersects the page canvas (CS6 constrains oversized rectangles to
  the document); an empty intersection refuses; the intersection
  rounds to integer pixel extents; the grid is fresh zero tiles via
  the decision-0037 `retile_rgba` (real bank tiles, not matte slots,
  so the bitmap is paintable). Placement is explicit
  `page/layer/cell` pre-flighted before anything mints (000557d).
- **Aspect fit centers in the rectangle.** The largest ratio-keeping
  fit (`min(rect_w/w, rect_h/h)` scale) rounds to at least one pixel
  per axis and centers in the bounds — the thumbnail reading of the
  doc text. Stretch-to-fill takes the rectangle's rounded extents
  directly. A rectangle degenerate on one axis only (`left == right`
  with `top != bottom`) is zero-area and refuses — the unscaled form
  requires both pairs equal, per the doc's "and".
- **Nearest-neighbour resampling, honestly recorded.** The crop
  window (destination ≤ canvas) samples each destination pixel from
  the scaled plane's centre point; products run in `u128` because
  the scaled extent is unbounded float arithmetic away from the
  canvas-bounded crop. Bilinear/box quality is the recorded HANDOFF
  §4 resample debt — this landing places and scales correctly first.
- **Refusals.** Non-finite bounds, non-positive source extents,
  plane length ≠ `w·h·4` (with a checked-multiply overflow gate),
  zero-area rectangles, empty canvas intersections, and
  out-of-range placement cells — all before the first mutation.

## Consequences

- Registry 239 → **240**; parity 267 C / 4 P / 0 M / 85 N → **268 C
  / 3 P / 0 M / 85 N** (one flip).
- Remaining P rows: 3 — `setFillNColorNTexture` (no observed `TXFv`
  shape in the corpus and no texture rasters to mint; recorded debt)
  and the two permanent rows (`setAnimInstanceStartFrame`,
  `setSliceAutonaming`). Every other CS6 dom mutation verb is
  carried.
