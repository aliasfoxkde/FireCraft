# Decision 0061 — importFile resamples bilinearly, in the premultiplied domain

**Debt closed** (decision 0054 / HANDOFF §4): `bitmap.importFile`
scaled placed images with a nearest-neighbour crop of the source —
blocky on any upscale and aliased on any downscale. It now uses the
same bilinear filter `document.setImageSize` uses (decision 0041).

## Design

- **Same filter, same convention.** The resample keeps
  `resample_rgba`'s centre-point mapping — destination pixel `d`
  samples source coordinate `(2·d + 1)·src / (2·dst)` — and the same
  1/256 fixed-point weights, but in exact `u128` integer math because
  the import path's scaled extent is unbounded float arithmetic away
  from the canvas (setImageSize resamples inside a canvas-bounded
  crop; importFile computes the full scaled image first, then crops
  to the canvas). `source_axis` returns the two source samples and
  the blend weight per axis, with border clamp on both ends.
- **Premultiplied blend, straight plane.** The import plane is
  straight alpha (`retile_rgba` takes straight; tiles premultiply on
  store), and averaging straight colours against transparent pixels
  bleeds black into the result. The resample premultiplies a copy of
  the source, blends all four channels there, and unpremultiplies
  the output — transparent neighbours dilute alpha only, never the
  colour. `premultiply_all`/`unpremultiply_all` widened to
  `pub(crate)` for this.
- **Everything downstream inherits it.** `importFile` (file bytes),
  `document.importFile` payload placement, and
  `file.importPNG/importJPEG` (they route through the same
  `resample_crop`) all gain the filter in one change.

## Consequences

- Registry unchanged; parity unchanged (the row was C under decision
  0054 with the quality deviation recorded; that note is struck).
- Tests: +1 unit (`resample_crop_blends_in_the_premultiplied_domain`
  — half-alpha red beside transparent upscales to diluted alpha with
  the colour still pure red; a straight-domain blend would sink the
  red channel) and the three `import_file` integration expectations
  updated to the bilinear ramp (border pixels clamp pure, interior
  blends symmetrically, e.g. the 3/8 sample of a 2×1 red|blue stretch
  lands at 160/256 toward red → `[159, 0, 96, 255]`).
- Phase 9 debts remaining: pattern-authoring command only.
