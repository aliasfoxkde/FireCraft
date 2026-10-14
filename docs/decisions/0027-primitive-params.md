# Decision 0027 — rectangle-primitive params and the quadrangle
transform

Date: 2026-10-14
Closes: HANDOFF §4 Phase 6.1 — parity rows `setRectRoundness`,
`setRectSides`, `setQuadrangle` flip `M` → `C`;
`setRectRoundnessMode` flips `M` → `N`.

## Wire / model

`PathObject.primitive` (`PRIv`: `RCTv` rect, `RDSf` radius, `MTXv`
transform, `RDMb` maintain) is already modeled and round-tripped. A
parameter change rewrites the source and re-derives the editable
contours from it — capture the path, mutate, store back, one
`Delta::Object {Mutated}`.

**`setRectRoundnessMode` has no wire home.** The dictionary's `PRIv`
children list is exhaustive (PCFb, fill points, MTXv, RCTv, RDMb,
EFLv, plus reader-observed RDSf) and carries no percent/pixel mode
node — the mode is the properties panel's display unit, session
state. Per the Phase-4 rule (do not invent wire state), it gets no
command and the row goes `N`. `setRectRoundness` therefore takes its
unit explicitly in the payload.

## Commands

- `primitive.setRectRoundness {id, roundness, mode}` — CS6
  `dom.setRectRoundness(roundness, mode)`. `percent` scales the
  shorter side: radius = roundness × min(w, h) / 2, so 1.0 is the
  stadium limit (validated 0..1); `pixels` stores the value as given
  (≥ 0) and derivation clamps to min(w, h) / 2. Radius zero re-derives
  the sharp 4-corner contour; above zero the 8-anchor rounded-rect
  derivation (`rounded_rect_contour`, kappa handles, `primary` =
  forward handle — the ellipse contour's working convention).
- `primitive.setRectSides {id, rect}` — CS6
  `dom.setRectSides(newSides)`: the new *untransformed* sides; the
  live `MTXv` keeps showing the transformed result. The stored radius
  carries over untouched (clamping is derivation-time, not stored —
  `RDMb`'s resize behavior is the panel's concern).
- `object.setQuadrangle {ids, top_left, top_right, bottom_right,
  bottom_left}` — CS6 `dom.setQuadrangle(...)`, described by the text
  itself as "the same as performing a transform operation": an affine
  maps the selection's union bounds rectangle onto the quadrangle
  (corners given as unit-square percentages of the bounds: corner =
  origin + percent × size), so the selection fills the quad. Only
  parallelogram quads are accepted — a projective corner refuses
  (`InvalidState`), since `MTXv` is affine-only. It lives in the
  object/transform family (decision 0019) and reuses `transform_core`
  wholesale (pivot `[0,0]` = the raw matrix). CS6's `options` strings
  ("transformAttributes", "autoTrimImages") have no model effect here —
  autoTrim is bitmap-side and the transform path refuses bitmaps.

## Deviations

- Mode is a payload argument, not stored state (no wire node).
- Quadrangle percentages resolve against the selection bounds; the
  CS6 text's "arbitrary square" is underspecified and the selection
  bounds are the only reading that makes the example's numbers land
  inside a document.
- Percent roundness at 1.0 = `min/2` is the natural reading of "100%
  roundness"; corpus samples with `RDSf` at the stadium limit would
  confirm.

## Verification

Engine unit tests (4 new): percent derivation to the stadium radius
with exact undo; pixel store-and-clamp with refusals (percent out of
range, negative pixels, no-PRIv target); sides rebasing with radius
carry-over and undo; quadrangle mapping of the fixture path x→2x with
undo and the projective refusal. Fuzz: three new `CALLS` ids (quads
mostly parallelogram-safe, one free corner in five); CALLS==catalog
pin holds (registry 161 → 164). Parity 185C → 188C / 45P / 40M /
83N.
