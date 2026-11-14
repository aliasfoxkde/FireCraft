# 0116 — The panel edits the rectangle, the pixel mask, and the pattern

Date: 2026-11-12 · Issue: [#136](https://github.com/aliasfoxkde/FireCraft/issues/136) ·
Status: accepted

## Context

The reachability gate measured five more engine commands the shell's
designed surface could not reach:

- the **rectangle primitive** — `primitive.setRectRoundness` and
  `primitive.setRectSides` (CS6's `dom.setRectRoundness` /
  `dom.setRectSides`, decision 0027);
- the **bitmap's pixel mask** — `bitmap.setPixelMask` and
  `bitmap.movePixelMaskBy` (CS6's `dom.setSelectionMask` and
  `dom.movePixelMaskBy`);
- the **pattern fill** — `path.setPatternFill`, the command surface
  for the Fill panel's pattern authoring.

Each one is a property a Fireworks user reaches for while drawing: a
rounded rectangle, a masked bitmap, a tiled fill. The engine could do
all three; the shell showed nothing.

## Decision

- **The Properties panel gains a Rectangle section** for a path that
  carries a primitive source. It edits the *source*, not the
  contours — that is what keeps the rectangle editable:
  - *radius* — the corner radius in pixels
    (`primitive.setRectRoundness`, `pixels` mode);
  - *roundness* — the same radius as a fraction of the short side
    (`percent` mode; 1.0 is the stadium limit, `min(w, h) / 2`);
  - *X / Y / W / H* — the untransformed source rectangle the
    contours derive from (`primitive.setRectSides`).
- **The Properties panel gains a Pixel mask section** for a bitmap.
  The mask is a coverage plane over the whole image, so the section
  reports its extent and offers:
  - *select all* — a full-coverage plane (`bitmap.setPixelMask`);
  - *clear* — no plane at all (`bitmap.setPixelMask` with a null
    mask);
  - the four nudge arrows — the selection slides without its pixels
    (`bitmap.movePixelMaskBy`).
  A full mask travels as one byte per pixel, so *select all* refuses
  past a megabyte of plane and says so.
- **The Fill & Stroke panel gains a Pattern section** for a selection
  that holds paths. An image in the document is the tile: the user
  picks one and the selection's fill becomes that raster, repeating
  (`path.setPatternFill`).

## The primitive's transform is not composed into the drawn shape

A defect surfaced under this binding, and it is **not** fixed here.

`PathObject.primitive` carries the `MTXv` matrix, the file writes it,
and the reader reads it back — but the renderer walks the *contours*,
and the engine's derivation (`rounded_rect_contour`) never composes
the matrix into them. So a rectangle primitive whose source carries a
transform draws as its untransformed rectangle, and editing its
roundness re-derives the contours without the transform.

Every primitive FireCraft itself mints has an identity transform, so
the defect is latent for engine-authored documents; it reaches a
document that carries a transformed rectangle from elsewhere.

**The open question is what CS6 stores**, and it needs corpus
evidence rather than a guess:

- does a skewed rectangle in a CS6 file keep *untransformed*
  contours plus an `MTXv` the renderer applies (so the shape is
  drawn transformed), or
- does it store *transformed* contours and keep `MTXv` only as the
  editable source?

The two readings put the fix in different places — the renderer in the
first, the engine's derivation in the second. Until a corpus sample
settles it, the derivation stays as it is and the limitation is
recorded here rather than papered over.

## Consequences

- Five more engine commands are reachable from the shell's designed
  surface: the measurement moves from 234 bound commands of 253 to
  **239 of 253** (92.5 % → 94.5 %).
- A user can round a rectangle, mask a bitmap, and tile a fill
  without leaving the app.

See also decisions [0104](0104-the-shells-selection-verbs-go-through-the-engine.md),
[0113](0113-the-filters-panel-bakes-and-renames.md),
[0114](0114-the-properties-panel-carries-the-mask.md) and
[0115](0115-the-pages-panel-marks-the-master-and-the-layers-panel-shares.md)
— the same week's other steps in closing the measured surface gap.
