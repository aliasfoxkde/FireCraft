# Decision 0029 — web-object point append
(hotspot/slice.appendPoint)

Date: 2026-10-14
Closes: HANDOFF §4 Phase 8.2 — parity rows `appendPointToHotspot`,
`appendPointToSlice` flip `M` → `C` (191 C / 45 P / 37 M / 83 N).

## CS6 verb

`dom.appendPointToHotspot(pt, tolerance)` /
`dom.appendPointToSlice(pt, tolerance)` (Fireworks 3; the slice form
appears under deprecated arguments, but both verbs remain in the
surface): "Appends a point to the selected unclosed polygon
[hotspot | slice]. If an unclosed polygon [hotspot | slice] is not
selected, a new polygon [hotspot | slice] is created with the single
point that passed in." `tolerance` closes the polyline once the new
point lands within it of the starting point.

## Mapping

The verbs are selection-driven — no object id on the wire — and the
engine now has the state they read: [`Document::object_selection`]
(decision 0024). The command scans the selection topmost-first for a
slice/hotspot (`ObjectData::Slice`) that is a polygon (`URSi` 2),
of the command's class, with an open outline:

- **Append branch** — capture the slice, push the anchor onto the
  last contour (open polylines are anchor-only: `is_curve` false, no
  handles), grow `frame` to keep carrying the bbox, and close the
  contour when `dist(pt, start) <= tolerance` (tolerance 0 is the
  exact-match close; the closing point itself still lands — CS6
  appends then closes). One `Delta::Object {Mutated}`.
- **Create branch** — mirror `slice.create` on the Web Layer
  (provisioning rides the same group delta), `URSi` 2, a single-point
  **open** contour, degenerate frame at the point. The page for a
  fresh object rides the payload — the engine has no active page (the
  `frame.add` precedent).

Class is `TSLi` presence (decision 0017): a slice command matches
`TSLi`-bearing objects only, a hotspot command matches `TSLi`-less
objects only. Rect/ellipse shapes never match — CS6 says "unclosed
polygon", and a rect slice carries no point list to extend, so a
selected rect correctly produces the create branch.

Wire names: `slice.appendPoint` / `hotspot.appendPoint`, matching the
`slice.*` / `hotspot.*` split the family already uses. `tolerance`
must be finite and >= 0 (`BadPayload`); points must be finite.

## Deviations

- CS6's created single-point object is a live tool state; ours is a
  stored open contour (`PBPv` supports the closed bit, so the wire is
  faithful). A one-point open polyline survives a round trip.
- The scan takes the topmost matching object if several qualify;
  CS6's text presumes one.
- The polyline closes on tolerance; CS6's UI equivalence (the
  pointer tool's auto-close click) is not modeled separately.

## Verification

Engine unit tests (3 new): the full state machine — fresh slice from
an empty selection, three appends growing the frame, the exact-match
close, and two-step undo back to no slice; class/shape predicates
(selected rect ignored, selected open hotspot invisible to the slice
verb and vice versa, extends in place for the right class); refusal
leaving the document byte-identical. Fuzz: both ids join `CALLS` with
a shared arm (tolerance mostly >= 0, one negative draw in ten);
CALLS==catalog pin holds (registry 165 → 167).
