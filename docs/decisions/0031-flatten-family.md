# Decision 0031 — the flatten family (`flatten.mergeDown`,
# `flatten.selection`, `flatten.document`)

Date: 2026-10-14
Closes: HANDOFF §4 Phase 3 — parity rows `MergeDown`/`mergeDown`,
`flattenSelection`, `flattenDocument` flip `M` → `C`
(198 C / 45 P / 29 M / 84 N; the CS6 verb list carries both case
spellings of mergeDown, so the flip is four rows for three commands).

## CS6 verbs

- `dom.mergeDown()`: "Merges selected objects to the bitmap directly
  below the selected objects. Succeeds only if the object immediately
  below the selection is a bitmap."
- `dom.flattenSelection()`: "Flattens the selection into a single
  pixel image. This action is the same behavior as the Merge Images
  command."
- `dom.flattenDocument()`: "Flattens the entire document into a single
  pixel image. This is the same behavior as the Merge Layers command."

## Design: compositor-backed bakes

All three verbs are pixel bakes. The bitmap-bake debt (decision 0030's
cropSelection note) said the flatten family waited on a raster path —
but `compose` already rasterizes Path/Group/Text/Image/SymbolInstance
deterministically, and the layer gate allows engine → compose
sideways. So the family renders through a new
`compose::render_subset(document, page, frame, ids)`: the listed ids
in list order, no canvas matte (the document's canvas color stays a
live property after the bake), no other content. The resulting
premultiplied RGBA8 is cropped into fresh `mkBT` bank tiles
(`TILE_SIDE` grid, tail slots zero-padded, ids minted one above the
highest live slot — the same replay-stable law as symbol numbers) and
the participants are replaced by one canvas-sized
[`doc::ImageObject`]. Fireworks bitmaps are canvas-sized objects, so
the canvas extent is faithful, not a simplification.

- **mergeDown** → `flatten.mergeDown {}`. Selection-driven like CS6's
  single menu item. Preconditions: non-empty selection; every member
  sits in a cell (group-nested or unplaced objects refuse — the bake
  needs a z-position); all members share one cell with one page/layer
  (z-order is per-state); something sits below the lowest member; and
  that immediate neighbor is a bitmap. Paint order is the bitmap, then
  the selection bottom-first. The merged pixels **replace the bitmap
  in place** — its registry identity, name and cell position survive,
  the way `path.boolean` keeps `a`; extent/offset reset to the canvas
  origin, weight resets to defaults (below).
- **flattenSelection** → `flatten.selection {}`. Merge Images. The
  selection may span layers of one page (cross-page refuses — one
  bake paints one canvas). The result lands in the topmost member's
  cell at `position - (same-cell members below it)`, so it takes the
  selection's old stacking slot.
- **flattenDocument** → `flatten.document {page}`. Merge Layers. Every
  content layer's objects collapse into one bitmap on a single bottom
  content layer carrying the first content layer's name; special (web)
  layers survive in their relative order above it. The page is
  captured wholesale (`Delta::PageBody`), so undo is byte exact across
  the layer-list rewrite.

## Delta shapes

mergeDown: `Group[Bank{minted}, Mutated{below-before}, Cell detach +
Object Removed per member]`. flattenSelection:
`Group[Cell/Removed per member, Cell attach (the bake), Created,
Bank]`. flattenDocument: `Group[PageBody, Bank, Created, Removed per
content object]`. Undo is LIFO in every shape; the freshly minted bank
slots are exactly the ids `Delta::Bank` drops (only fresh ids are
inverse-expressible).

## Deviations

- **Baked pixels carry default weight** (opacity 1000, blend 0, no
  effects): the render already applied the sources' paint, weight and
  live effects; re-applying the source object's weight would double
  it.
- **Multi-state pages**: flattenDocument refuses a page with
  `frame_count > 1` (Merge Layers is a single-state operation in CS6).
  mergeDown/flattenSelection bake **state 0 only** — the engine has no
  current-frame state (the `frame.add` precedent); a member placed in
  a later state would bake as empty pixels. Recorded; the scripting
  layer resolves the current state before calling.
- **Symbol instances / text** bake through the compositor's own
  rendering (real text engine, instance transforms) — no CS6 script
  interpreter needed.
- **mkBT writer debt**: the bank reader exists; baked tiles do not
  PNG-round-trip until the tile-bank writer lands (HANDOFF Phase 9).

## Verification

`cargo test -p engine --lib` (277 tests) — 6 new tests: mergeDown
bake (identity/name kept, fresh tiles, paint order green-under-blue
over the red bitmap, byte-exact undo) + refusal matrix (no-op,
nothing-below, path-below, cross-cell, unplaced); flattenSelection
cross-layer bake (placement at the topmost slot, pixel checks,
undo) + no-op/cross-page/empty-canvas refusals; flattenDocument
collapse (special layer preserved, registry cleaned, pixel checks,
undo) + multi-frame/empty/unknown-page refusals. Fuzz drive exercises
all three verbs (registry pin 173).
