# Decision 0041 — `document.setImageSize` (the last M row)

**CS6 surface.** `dom.setDocumentImageSize(boundingRectangle,
resolution, currentPageOnly)` (Fireworks 3; `currentPageOnly` added in
CS3): "Scales the document to fit in the specified rectangle at the
specified resolution." The corpus batch template
(`Configuration/Batch Code/BatchTemplate.jst`, the Scale workflow)
passes `newDocRect = {left: 0, top: 0, right: newWidth, bottom:
newHeight}` — the rect carries the exact target dimensions the caller
computed — and `newDocResolution = {pixelsPerUnit: theDoc.resolution,
units: theDoc.resolutionUnits}`, omitting `currentPageOnly` (default
true).

## Design

- **The rect is the new document bounds, not a fit-inside box.** The
  corpus anchors it at (0, 0) and passes exact target extents; the
  manual's "fit in" is loose prose over that. Content maps through
  `T(rect.origin) · S(sx, sy)` with `sx = rectW / oldCanvasW`, `sy`
  likewise — an anisotropic rect scales the axes independently (the
  API shape admits it; the corpus only ever passes uniform scales).
- **The pixel grid scales with the canvas.** Each image resamples
  `pixels` by the same `(sx, sy)`, rounded, through the new bilinear
  pass `engine::bitmap::resample_rgba` (edge-clamped, 1/256 fixed
  weights, premultiplied domain — deterministic integer math). The
  result re-tiles over fresh bank slots (the flatten minting law);
  resample output is capped at the editable working space
  (1..=8192 per side) so the minted tiles stay gatherable.
- **Everything else rides the rotate machinery** (decision 0034):
  paths/slices map point-for-point, primitive rects map and their
  transforms compose, text/symbol transforms compose, guides read off
  one point per old line, the grid offset maps, deltas are
  `[Info, PageBody × pages, Bank, Object × n]` with byte-exact undo.
  `rotate_point/rotate_contours` renamed to `map_point/map_contours` —
  they were always plain affine mappers; rotate's inline image
  preflight is now the shared `preflight_page_images`.
- **Resolution** rides the root canvas plus each scaled page (the
  `setResolution` law scoped to the affected pages); the whole-page
  `PageBody` covers the page copy, so no separate `PageInfo` deltas.
- **Scope.** `currentPageOnly` defaults to true (CS6); false scales
  every page into the same rect, each by its own canvas ratio. The
  page itself is an explicit payload field — no current-page state
  (the `document.rotate` precedent).

## Deviations

- The resolution argument does not interact with the pixel scale —
  the rect governs pixels, resolution is a property write. The
  corpus's only usage passes the current resolution, so no evidence
  contradicts this.
- No dialog (headless engine); the UI layer owns Image Size
  affordances.
- Resample is bilinear; CS6's exact kernel is unobservable from the
  corpus. Recorded as a P-class note, not a debt row (solid-color and
  identity behavior are exact; interpolation differs only on
  photographic content).

## Consequences

- Parity **225 C / 45 P / 1 M / 85 N → 226 C / 45 P / 0 M / 85 N** —
  the missing table is empty; document-mutation coverage 271/271.
- Registry 198 → 199 commands.
- The parity campaign's M-work is complete; remaining surface work is
  the 45 P rows and HANDOFF Phase 9 debts.
