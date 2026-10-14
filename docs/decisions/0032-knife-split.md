# Decision 0032 — Knife + splitPaths (path splitting)

**Date:** 2026-10-14 · **Phase:** 8.3 · **Status:** implemented

## Verbs (CS6 API text)

- `dom.splitPaths()` (Fireworks 3): "Splits the selected paths.
  Compound paths are split into separate contours."
- `dom.knifeElementsFromPoint(from, tolerance)` (Fireworks 3): "When
  the user clicks a single point while using the Knife tool, this
  function cuts paths within the specified tolerance." Returns
  Boolean — `true` if anything was cut.
- `dom.knifeElementsFromPoints(from, to, tolerance)`: "When the user
  drags while using the Knife tool, this function cuts additional
  items within the specified tolerance." Same Boolean return.

## Commands

`path.splitPaths {}` (selection-driven), `path.knifeElementsFromPoint
{from: [f64;2], tolerance}` , `path.knifeElementsFromPoints {from, to,
tolerance}` — all in `catalog/path.rs`; geometry in the new
`vector::knife` module (`knife_contour`/`knife_contours`).

## Split contract (shared)

Every changed path keeps its identity on the *first* resulting
contour; further contours become fresh `PathObject`s attached right
after it in the same cell, inheriting the display fields (paint,
opacity, blend, name, effects) with `primitive: None` (derived
geometry, the decision-0003 result contract). The results take over
the selection; untouched selected paths keep their slots. Deltas:
`ObjectSelection {before}` first, then per split
[attach, `Created`, `Mutated`]; undo is byte-exact.

## Knife algorithm (vector::knife)

Same pipeline contract as the booleans (decision 0003): a flattened
proxy *finds* cut positions; output geometry is true curve math
(kurbo `subsegment`), never the proxy.

- **Crossing pass:** each span is flattened (line spans → 2 chords,
  cubics → `flatten_cubic`); every chord×blade segment intersection
  with both parameters strictly interior contributes a span parameter.
  Parameters are deduped at [`SNAP`].
- **Tolerance clause:** when no crossing lands, the contour's nearest
  position to the blade is measured at chord granularity; if within
  `tolerance`, the contour is cut there — the dom's "cuts additional
  items within the specified tolerance". A nearest position on a span
  boundary (± `SNAP`) is not a cut: splitting an open contour at one
  of its anchors changes nothing.
- **Click semantics** (`from == to`) are the degenerate blade: only
  the tolerance clause can cut, at the nearest contour position.
- **Closed contours** open at the first cut (ring rotated so the head
  starts at the cut point); runs between cuts become open contours.
  Sliver pieces (chord < `SNAP`) are dropped; their cut marker rides
  the next surviving piece.
- `knife_contours` cuts each contour independently; missed contours
  pass through. `None` when nothing was cut — the command maps that
  to `Delta::Noop` (the dom's `false`).

## Deviations (recorded, not hidden)

1. **Selection-driven scope.** The dom verbs take no object list; the
   CS6 tool operates on whatever is selected. Same precedent as
   `cropSelection` (decision 0030). Non-path members refuse the whole
   command (bitmap knife is the filter-bake debt); group-nested or
   unplaced members refuse (pieces must attach to a cell).
2. **Parallel-segment distance** in the tolerance clause uses the four
   end-point projections — exact except near-parallel near-touching
   chords, which the crossing pass catches anyway.
3. **Chord-granularity cut position** for tolerance-clause cuts (the
   nearest chord's midpoint) — the flatten tolerance the whole crate
   already works at. Crossing cuts are exact span parameters.
4. **splitPaths non-path refusal.** The dom is silent on non-path
   members; we refuse whole-command (cropSelection consistency) since
   silently skipping hides selection mistakes.
5. **Zero-length drag** on `knifeElementsFromPoints` is a payload
   error — CS6's UI cannot produce it (the click verb owns that
   gesture); `from == to` on the click verb is the intended degenerate
   case.
6. **No corpus oracle** for any of the three verbs (no `.jsf` usage in
   the Configuration tree); the API text is the sole authority. The
   piece attachment position (immediately after the original) and the
   selection-takeover are our normalizations, consistent with
   decision 0003's provisional result contract.

## Consequences

- The 26 remaining M verbs no longer include any path-splitting
  surface; parity is 201 C / 45 P / 26 M / 84 N.
- `path.cropSelection` + knife + splitPaths complete the CS6
  Modify-path destructive surface on the wire we model.
- Undo of a split restores the original contour list and removes the
  minted pieces (byte-exact, proven in tests).
- Recorded debt: knife on bitmaps/text (pixel-space cut) is the filter
  bake debt (Phase 9), same as crop.
