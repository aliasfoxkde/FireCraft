# Decision 0037 — the place-image trio

**Date:** 2026-10-14 · **Phase:** 8.1 · **Status:** landed —
`bitmap.addImage` / `bitmap.addImageViaCopy` / `bitmap.addImageViaCut`

## Verbs (CS6 API text)

- `dom.AddNewImage(boundRectangle, bEnterPaintMode)` — "Adds a new
  empty (transparent) image to the current layer at the specified
  location and size. You cannot create an image that is larger than
  the document; if you specify a rectangle that is larger, it is
  constrained to the document size." Fireworks 3.
- `dom.addNewImageViaCopy()` — "Adds a new image containing the
  contents of the current paint-mode selection. The new image is
  placed directly above the active bitmap. You must have a current
  pixel selection." Fireworks 3/MX.
- `dom.addNewImageViaCut()` — same, plus "The selection is cut from
  the previously active bitmap." Fireworks 3/MX.

No return value; none of the three appears in the CS6 `.jsf`
Configuration tree (the tools create images through the UI, not
through scripted DOM calls) — the API text plus the engine's existing
bitmap model are the authority.

## The engine path (why not `io::raster::attach_image`)

HANDOFF §4 originally pointed the command at
`io::raster::attach_image`, but that function is `pub(crate)` to the
`io` crate and import-shaped (it wraps a decoded foreign buffer for
SVG/raster import, minting the file's own tile grid). A place-image
command needs engine-side placement semantics, not an import: the
route is the `document.rotate` (decision 0034) assemble → transform →
re-tile machinery instead. Its two loops are now factored into shared
helpers on `engine::bitmap` so rotate and this family use one
implementation:

- `assemble_rgba(document, image, name)` — the image's `TILv` grid
  into one premultiplied RGBA buffer; `TMCi` matte slots bake to the
  renderer's premultiplied fill (`compose::matte_premultiplied`), the
  decision-0034 law. Missing referenced tiles are a refusal, not
  silent transparency.
- `retile_rgba(document, pixels, width, height)` — a premultiplied
  buffer over fresh `mkBT` bank slots via `flatten::next_tile_id`;
  returns the `TileRef` list and the minted ids (the flatten minting
  law).

`rotate_image` now calls both helpers; its tests guard the refactor.

## Design

- **`bitmap.addImage`** `{x, y, width, height, page, layer, cell,
  name?}` — `f64` bounds as CS6 passes them. The rectangle is
  intersected with the target page's canvas (`info.canvas`); an empty
  intersection is a refusal (`InvalidState` — nothing to place). The
  intersection rounds to integer pixel extents, the image location is
  the intersection origin (`XLCf`/`YLCf`), offset stays at the origin,
  and the grid is fresh zero tiles — **real bank tiles, not `TMCi`
  matte slots**: a matte-only grid is invisible to `gather`/`scatter`,
  so a later retouch would silently no-op. Fresh zero tiles make the
  new bitmap paintable. Placement is explicit `page/layer/cell`
  (validated by `access::cell_mut`) with optional `OBNs` name — the
  engine has no active-layer state, so the `insert_shape` payload
  convention applies. `bEnterPaintMode` is UI posture (entering
  bitmap editing mode); the engine has no paint-mode session state
  (deviation 1).
- **`bitmap.addImageViaCopy`** `{source}` — the source must be an
  `ImageObject` in a **cell** (group-nested sources refuse: there is
  no cell to sit "directly above" in); a pixel selection
  (`Document.selection`) must exist and its plane must span the
  source's pixel grid (`mask.width/height == image.pixels` — the
  marquee contract everywhere else in `bitmap.*`). The source's grid
  is assembled premultiplied, each pixel's channels are scaled by the
  mask coverage (`out = v · cov / 255` — compositing's source-scale;
  the premultiplied domain needs no alpha round-trip), and the result
  re-tiles over fresh ids. The new image copies the source's
  `location`/`offset`/`pixels`/`tile_size` but takes default paint
  state (opacity 1000, blend 0, visible, unlocked, no effects, no
  name) — a new bitmap, not a style clone. Placement: the source's
  cell at `position + 1` (directly above the active bitmap).
- **`bitmap.addImageViaCut`** `{source}` — identical, plus the source
  is rebuilt: pixels under any nonzero coverage clear to transparent
  over **fresh tile ids** (the shared-bank tile slots are never
  edited in place on this path — the same ids may be referenced
  elsewhere; decision 0034's minting rule). The source's `ObjectData`
  before-image is captured for undo; its old tiles are orphaned in the
  bank (the recorded bank-GC debt).
- **Deltas** — addImage/ViaCopy: `Group([Cell, Created, Bank])`;
  ViaCut: `Group([Cell, Mutated(source), Created, Bank])` with one
  `Bank` carrying every minted id. Undo removes the minted tiles,
  removes the new object, (restores the source), then the placement —
  byte-exact in both directions.

## Deviations (recorded, not hidden)

1. **No paint-mode posture.** `bEnterPaintMode` only describes CS6's
   UI entering bitmap edit mode; the engine has no such session state.
   The payload omits it.
2. **The trio address cells explicitly.** CS6 places on "the current
   layer" / "directly above the active bitmap" (UI session state);
   `addImage` takes explicit placement and ViaCopy/ViaCut derive it
   from the source's `find_owner` position.
3. **Cut clears any nonzero coverage.** A feathered marquee's partial
   pixels cut entirely (CS6's cut is also binary at the selection
   edge for the underlying bitmap).
4. **ViaCopy/ViaCut have no clipboard interaction.** Despite the
   names, the CS6 text says the selection content moves into a new
   bitmap — no `fw.clip` state involved; the clipboard surface (8.6)
   stays independent of this family.

## Consequences

- Parity: the family's rows flip M→C — four rows, since the CS6 verb
  list carries both case spellings of `AddNewImage` (the mergeDown
  precedent) — **212 C / 45 P / 12 M / 88 N** at this batch's close;
  registry 183 → 186.
- `engine::bitmap` gains the assemble/retile helpers; `rotate_image`
  deduplicates onto them (its golden + undo tests cover the refactor).
- Cut's orphaned source tiles are already-covered debt (bank GC);
  nothing new opens.
- No corpus fixture exercises the verbs (zero `.jsf` usage either);
  the gate is the synthetic command-level suite: placement, coverage
  math against hand-computed pixels, cut's source rebuild, refusals,
  and byte-exact undo for all three.
