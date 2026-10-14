# Decision 0034 — rotateDocument (page rotation with a geometry re-bake)

**Date:** 2026-10-14 · **Phase:** 5.3 · **Status:** implemented

## Verb (CS6 API text)

- `dom.rotateDocument(rotationAmount)` — "Acceptable values for
  rotationAmount are 90, 180, and 270 … Rotates the entire document
  90º, 180º, or 270º clockwise. Rotating 270º is the same behavior as
  rotating 90º counterclockwise."

## Command

`document.rotate {rotation, page}` in `catalog/document.rs` (the
family drops the `Document` suffix, like `setCanvasColor` /
`setResolution`).

## Design

**Page-scoped, not document-scoped.** Canvas dimensions live per page
(`PDCv`), and CS6's rotate command operates on the active page's
canvas — a "rotate the entire document" reading would clobber every
other page's independent size. The command rotates exactly one page:
canvas, guides, grid, and every object on it.

**The map.** `rotation_map` = rotate about the old canvas center, then
translate the rotated extent onto the new rect (width/height swapped
for 90/270, origin anchored) — every object survives and nothing is
clipped. The linear part is built from integers, not
`Affine::rotate(pi/2)`: trig-approximated quarter turns smear
`cos(pi/2) ≈ 6e-17` into every coordinate (caught by the corner-anchor
test), and a quarter turn must be exact. Positive steps are clockwise
on the y-down canvas (kurbo's y-up counter-clockwise convention
inverted).

**Per-kind re-bake (compute all before mutate):**

| Kind | Re-bake |
|---|---|
| Path | anchors + handles through the map; primitive `rect` via `map_rect` (90° steps keep rects axis-aligned) + `transform` composed |
| Text | transform-only: `MTXv ← map · MTXv`. The frame is the *pre-rotation layout input*; `finish` rotates the laid-out output, so re-wrapping would move the glyphs. Frame untouched, `on_path` untouched |
| Image | assemble tile grid → one premultiplied RGBA buffer (matte slots materialize via `compose::matte_premultiplied`, now public) → rotate by index permutation → re-tile over fresh bank slots (the flatten minting law, `next_tile_id` made `pub(crate)`) |
| Slice | `frame` via `map_rect`; polygon contours point-for-point |
| SymbolInstance | `transform ← map · transform` |
| Group | no geometry; children are walked (depth-first, each id once) and rotate individually — they hold world-space coordinates |

**Guides and grid.** Horizontal lines (y positions) and vertical lines
(x positions) swap under 90/270 and mirror under 180 — each new
position read off the map at one point of the old line. Grid origin
rides the map; cell size is rotation-free.

**Deltas.** `[PageBody(page), Bank(minted), Object{Mutated} × n]` —
the page delta restores canvas/guides/grid/layers in one, the bank
delta drops the re-tiled slots on undo (original tiles are never
removed, so the pre-command image stays resolvable), object deltas
restore every re-baked body. Undo is byte-exact.

**Refusals (before any capture, document byte-identical after):**
rotation ∉ {90,180,270}; page out of range; degenerate canvas
(0×0/non-finite); an image with a tile size other than the 128 px
`mkBT` grid; an image referencing a missing bank tile.

## Deviations (recorded, not hidden)

1. **Explicit page, not active page** — the engine keeps no
   current-page state (`page.delete` precedent); CS6 targets the
   active page. The scripting layer maps the active page in.
2. **270 ≠ "90 counter-clockwise" reading** — CS6's doc sentence is
   behavioral equivalence; we implement one clockwise map per step
   value, matching the text's primary definition.
3. **The `MKBv` root canvas is left alone** — page canvases are
   independent; only `PDCv` of the rotated page changes. Root/page
   canvas-size mirroring stays a round-trip debt (the setters family
   keeps them in sync for color/resolution because those genuinely
   are document-wide).
4. **Image rotation bakes matte slots** into real premultiplied
   pixels — the flatten bake debt (matte is a live property elsewhere;
   a rotated image holds real bytes).
5. **No corpus oracle** — no `.jsf` usage of `rotateDocument` in the
   CS6 Configuration tree; the API text is the sole authority.

## Consequences

- Parity 203 C / 45 P / 24 M / 84 N; the transform family's only
  remaining verb is `setDocumentImageSize` (resample debt).
- Bank growth per image rotation is bounded by undo history — orphaned
  tiles accumulate until a bank GC lands (recorded with the mkBT
  writer debt).
- Exactness contract: rotating 4× through 90 is pixel-identical to
  rotating 360 once (integer linear parts; the permutation buffers are
  lossless) — the corner-anchor and pixel tests pin this.
