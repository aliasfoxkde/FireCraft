# Decision 0043 — web/group setter verbs (P rows → C)

**CS6 surface.** Five dom setters the matrix classed P — geometry,
classification, and visibility writes over existing objects:

- `dom.setHotspotRectangle(boundingRectangle, bMakeCopy)` — "Sets the
  rectangle that bounds the specified slice or hotspot… If
  `bMakeCopy` is set to true, the method leaves the original hotspot
  unchanged and places a copy at the new location. The copy is
  selected." Single-selection ("If more than one object is selected,
  an error is returned" class — enforced).
- `dom.setHotspotShape(whatToSet, shapeType)` — shape ("rectangle",
  "oval", "polyline") applied to the selection filtered by
  `whatToSet`: "hotspots", "slices", or "hotspots and slices".
- `dom.copyToHotspot(hotspotType, whatIfMultipleSelected,
  makeRectangular)` — "Creates hotspots or slices from the current
  selection"; `hotspotType` "hotspot" (map-only) or "slice";
  `whatIfMultipleSelected` "single" (one area covering the union) or
  "multiple" (one per object); `makeRectangular` (default true)
  forces rectangle areas, false donates polygon shapes from the
  source geometry.
- `dom.setGroupType(groupType)` — CS3-era: "normal" (restores the
  FW4-deprecated mask restructure), "mask to image", "mask to path".
- `dom.setWebObjectsVisibility(value)` — shows/hides the page's Web
  Layer (CS3 bool form).

## Design

- **`setHotspotRectangle` rescales, it does not teleport.** The rect
  is the new frame; the hotspot's contours ride the old-frame → new
  rect affine (the decision-0041 mapper over `map_contours`), so an
  oval/polygon hotspot stretches with its frame. Refuses: non-finite
  or non-positive rect, zero-extent source frame (degenerate — no
  scale exists), non-slice target, not-exactly-one selection. Copy
  mode inserts a new `Slice` directly after the original in its cell
  (`attach_cell` at `position + 1`), selects the copy, and the delta
  group is `[ObjectSelection, attach, Created]` (LIFO undo: creation
  retracts first).
- **`setHotspotShape` is a filtered loop.** `whatToSet` maps to a
  slice-kind filter (`hotspots` → map-only, `slices` → foreground
  slice, "hotspots and slices" → both); members not matching the
  filter or not slices are skipped; members already at the target
  shape are no-ops (empty result → `Noop`); the rest mutate in place.
  Unknown `whatToSet`/`shape` strings refuse (CS6's dialog never
  produces them).
- **`copyToHotspot` covers, then selects.** Areas are computed from
  `content_bounds` (decision-0041 shared geometry): "single" unions
  all selected objects into one area; "multiple" makes one per
  object. `makeRectangular: false` donates contours from the source
  (slice or path, shape 2 polygon; anchors-only bounds for the
  frame). The page's Web Layer is provisioned lazily
  (`ensure_web_layer`; the `Layers` delta rides first when it is).
  `whatIfMultipleSelected "ask user"` refuses — there is no headless
  answer. The created areas replace the selection
  (`ObjectSelection` first in the group).
- **`setGroupType` only normalizes.** "normal" (or the empty CS3
  default) walks the selection and sets every mask group's `GRTi`
  back to 0 (regular); non-groups skip; already-normal → `Noop`. The
  mask spellings refuse: our mask groups are built and unmasked
  through `mask.*` (decision-0031 family), and the FW4
  restructure-to-path/image paths have no live surface — recorded
  refusal, not a partial reimplementation.
- **`setWebObjectsVisibility` provisions.** The Web Layer is the
  page's `is_special` layer; when absent it is created hidden-able
  via `ensure_web_layer`, and the delta flips its `display` flag.
  Whole-layer `before` rides one `Layer` delta (plus the provision
  `Layers` delta when minted).

## Consequences

- Registry 201 → **206**; parity 228 C / 43 P / 0 M / 85 N → **233 C
  / 38 P / 0 M / 85 N**.
- The fuzz CALLS table gained the five verbs; the shifted RNG stream
  exposed a latent `document.setImageSize` violation of 000557d —
  its pre-flight loop baked page N's images (minting bank tiles)
  before page N+1's resample cap refused. Fixed by hoisting every
  page's resolution (cap check + assembly, buffers held) ahead of
  the first bake — the `bake_core` hold-the-buffer pattern; the new
  `image_size_refusal_after_another_page_bakes_touches_nothing` test
  pins it. The 8eae611 batch itself was sound; its fuzz job failed
  on the latent debt.
- `content_bounds`/`map_contours`/`map_point` are now `pub(crate)`
  across the catalog (the shared-geometry law from decision 0041).
- Donated-polygon copy is bounds-framed: the frame is the anchors'
  rect, the contours are a direct clone (CS6 keeps the shape
  verbatim); donated contours with no anchors refuse.
