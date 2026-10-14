# 0019 — Object transforms bake per variant; the free matrix is the primitive

- **Status:** Accepted (post-roadmap gap close, 2026-10-14)
- **Deciders:** Michael Kinney, Claude
- **Scope:** `engine::catalog::object` (L2) — the `object.scale`,
  `object.rotate`, `object.skew`, `object.reflect`, `object.resize`,
  `object.transform`, `object.setBounds` family — plus the parity
  matrix's transform rows and any future Wine-oracle corpus playback.
- **Supersedes:** the stale "NO object transform commands" gap note in
  the parity matrix · **Superseded by:** none

## Context

The scale/rotate/reflect/skew/resize/transform family is the largest
single parity gap left in the DOM surface (7 M rows). The engine
already carries every ingredient: `geom::Affine` (kurbo) with
`transform::{map_rect, scale_about}` helpers, `object.translate` as the
variant-dispatch template, and a model that stores transforms exactly
where the format does.

The design question is where a transform *lives*: a persistent
per-object matrix (a "transform spine") or immediate geometry rewrite.

## Evidence

1. **Format dictionary** (`docs/research/fireworks-png-nodes.md`):
   `MTXv` (2×3 affine, `M00f…M12f`) exists on exactly three node
   kinds — the `PRIv` primitive source, the text style (`TFSv` block),
   and the symbol instance (`ALSv`). Plain vector paths carry **no**
   matrix node: their `PLCl` contours bake every transform into point
   coordinates. A global transform spine on `ObjectData` would invent
   wire-format surface the reference format does not have for paths.
2. **Model truth** (`doc::model`): `PrimitiveSource.transform: Affine`,
   `TextStyle.transform: Option<Affine>`, `SymbolInstanceObject.
   transform: Affine` — round-tripped by the fwmap writer. Path
   contours, text/slice frames, and image locations are absolute
   document-space geometry, matching the format.
3. **CS6 signatures** ("Extending Fireworks"):
   `scaleSelection(xScaleAmount, yScaleAmount, opts)` (floats, "0.0 or
   greater"), `rotateSelection(rotationDegrees, opts)` (degrees,
   clockwise), `reflectSelection(bHoriz, bVert, opts)`,
   `resizeSelection(width, height)` ("keeping the top-left corner of
   the selection in place"), `setSelectionBounds(boundingRectangle,
   opts)` ("moves and resizes in a single operation"),
   `transformSelection(matrix, opts)` (3×3 matrix). **None takes a
   pivot** — CS6 pivots at the selection's bounds center; the `opts`
   strings (`"transformAttributes"`, `"autoTrimImages"`,
   `"rememberQuad"`) are attribute-propagation and raster-trim flags.
   `skewSelection` is absent from the CS4 help set (CS6 API inventory
   only): designed as two skew angles in degrees, pivot at bounds
   center, flagged below.
4. **Precedent**: `object.translate` already established the
   variant-dispatch shape — paths move point-wise (contours, fill
   handles, `PRIv` rect), text/slice frames move, images move
   `location`, symbol instances compose their transform, groups recurse
   so a shared descendant moves once.

## Decision

One primitive, six sugars. `object.transform {ids, matrix}` applies a
caller-supplied affine (kurbo coefficient order) through a single
`apply_matrix` walk; `object.scale` / `rotate` / `skew` / `reflect` /
`resize` / `setBounds` compose their specific matrix and call the same
walk. Per variant:

| Variant | Application |
|---|---|
| `Path` | matrix-map every contour anchor + handles, the fill handles, and — when a `PRIv` source exists — compose `primitive.transform = M ∘ old`, rect untouched |
| `Text` | compose `style.transform` (creating a default style when absent); frame untouched (the `MTXv` carries the layout-box transform) |
| `Slice` | matrix-map frame via `map_rect` and every contour point |
| `SymbolInstance` | compose `instance.transform` |
| `Image` | **refused** (`InvalidState`) — pixel data is tile-banked (`TILv`/`WPXi`); any geometry rewrite desyncs the bank. Rotate/reflect of bitmaps needs the resample pass; recorded as the family's limitation |
| `Group` | recurse; each child sees the same caller matrix, and carries its own captured inverse |

**Pivot.** Commands take `pivot: Option<[f64; 2]>`; `None` means the
union bounds center of the target set (computed by `map_rect` over each
variant's local bounds: contour bbox / text frame / slice frame /
instance `transform ∘ frame`-proxy), mirroring CS6's selection-center
default without a selection surface. `resize` and `setBounds` keep the
top-left corner fixed (documented CS6 behavior) and take no pivot.

**Refusals.** Empty `ids` (as translate); non-finite matrix/scalars;
negative scale (CS6 documents "0.0 or greater" — zero is accepted and
bakes a collapse); image targets (above). Validate all targets before
the first mutation — a refused call leaves the document untouched.

**Undo.** Every object whose data changed captures a `Mutated` inverse
before mutation, in the translate idiom. No minting, so no LIFO
retraction concerns; undo is exact because each inverse carries the
whole pre-command object.

## Falsification path

A Wine-oracle playback scripting `rotateSelection` / `scaleSelection`
on (a) a raw path and (b) a primitive-backed auto shape, then diffing
the written `PLCl` points and `PRIv` `MTXv`. If CS6 writes a live
`MTXv` on a plain path instead of baking points, the corpus wins: add
the path-level matrix to the model and re-cut this family. Until such
a file exists, the dictionary (no `MTXv` on paths) is the authority.
The image refusal flips the day the resample pass lands — the command
detail strings name that dependency explicitly so the gap is greppable.

## Consequences

- Parity rows `scaleSelection`, `rotateSelection`, `skewSelection`,
  `reflectSelection`, `resizeSelection`, `transformSelection`,
  `setSelectionBounds` move M → C; `removeTransformation` stays N (the
  bake model has no spine to clear).
- Recorded limitations: bitmap transforms refuse until resample;
  `opts` flags (`autoTrimImages`, `rememberQuad`,
  `transformAttributes`) are not modeled — fill handles ride every
  transform unconditionally (the `transformAttributes` default is the
  common case); `skewSelection`'s signature rests on the API inventory,
  not the CS4 help set.
- The fuzz drive gains the new commands with matrices built from
  bounded random coefficients; degenerate (singular) matrices are
  in-band values, not refusals, and must round-trip through undo
  byte-exactly.
