# 0003 — Path booleans: bespoke curve-aware pipeline

- **Status:** Accepted (M6 spike, 2026-10-10)
- **Deciders:** Michael Kinney, Claude
- **Scope:** the `vector` crate (L2) — union / intersect / punch / crop
  over the `doc` path model, and the vector ops that will layer on it
  (M6: simplify, expand stroke, inset).
- **Supersedes:** none · **Superseded by:** none

## Context

Fireworks' Modify ▸ Combine Paths group (Union, Intersect, Punch, Crop)
operates on true curve geometry and returns editable paths — the result
must keep the operands' bezier spans (an arc stays an arc), not a
polygon approximation. The candidate strategies:

1. **Polygonize + CLIP-style library** (`geo-boom`/`geo`): robust,
   mature — but the output is polylines; every boolean would destroy
   curve fidelity, and re-fitting curves afterward reintroduces the
   tolerance we just threw away.
2. **`kurbo-intersect`-style pairwise flattening**: flattens both paths
   and intersects the *flattened* forms outright. Same fidelity loss,
   just smaller.
3. **Bespoke curve-aware pipeline**: flatten only as a *probe* to find
   crossings, split the true curves, and reassemble regions from the
   pieces. More code, but output fidelity equals input fidelity.

## Spike

`crates/vector` (workspace L2, deps `doc` + `geom` + `kurbo`) implements
strategy 3 end to end with a 26-test adversarial suite. The pipeline:

1. **Prepare** — contours → true `Seg`s (cubic iff paired handles, the
   bridge rule) + a flattened probe polyline per segment
   (`FLATTEN_TOLERANCE = 0.05` px, convex-hull deviation bound).
2. **Probe** — cross-operand AND intra-operand flat-segment pairs tested
   for transversal crossings; chord parameters must be strictly interior
   (`chord_interior`) before mapping to segment parameters via the
   probe's recorded `(t0, t1)` range. Intra-operand probing skips
   adjacent spans (shared anchors, never transversal).
3. **Split** — true segments split at found parameters with exact curve
   math (`kurbo::ParamCurve::subsegment`), never the proxy.
4. **Classify** — each piece sampled at three stations, `SIDE_OFFSET` =
   0.1 px to each side; inside-ness is the operand winding at the sample
   (parity for even-odd operands) under the op's predicate over the
   *global* operand pair — never swapped per-source (this is what makes
   punch keep the punched operand's boundary as hole edge). A piece is
   boundary when a majority of stations straddle with one consistent
   orientation; it is directed interior-left.
5. **Stitch** — pieces chain end-to-start on a `SNAP` = 1e-6 px grid;
   at degree-4 junctions the walk takes the first candidate
   counterclockwise from the reversed incoming direction (the standard
   face-walk rule for keeping interior left). Deterministic: ties break
   by piece index. Failure is `VectorError::OpenChain`, never a guess.
6. **Assemble** — loops become closed contours sorted by first anchor
   (`total_cmp`), wrapped in a neutral `PathObject` (opacity 1000,
   blend 0, nonzero fill — loops are self-wound so even-odd need not
   survive the op).

The adversarial set proved each stage: exact anchor sets for the
overlapping-squares family (union 8-anchor ring, intersect square,
punch L-shape, directional `a − b`), hole topology for concentric punch
and even-odd ring parity, shared-edge merge, disjoint operands,
circle–square topology, and no-panic + byte-determinism contracts for
tangent contact and self-intersecting (bowtie) operands.

## Decision

**Adopt the bespoke curve-aware pipeline** (strategy 3) for all four
boolean ops. Rationale:

- Curve fidelity survives: result spans are true `subsegment` pieces of
  the input curves (proved: the circle–square intersect result is
  entirely curve points, no chord anchors).
- No new heavy dependency; `kurbo` is already in the stack via `geom`.
- Every numeric gate is a named, documented constant, so behavior is
  tolerance-pinned and the proofs re-pin deliberately.

Rejected: strategy 1/2 (lossy output), Deflate-style arrangement
libraries (none mature in pure Rust with curve output), and porting
FreeType/Skia boolean internals (license/complexity out of proportion
for M6).

## Tolerance-pinned limitations (documented behavior)

These are pinned contracts, not bugs — each is proved by a test:

1. **Parallel/collinear flat pairs return no crossing** — region
   assembly resolves shared edges instead (proved: shared-edge union).
2. **Exact-corner crossings land at t=0/1** and are dropped by the
   interior gate; fixtures offset crossings strictly inside spans.
3. **Features narrower than `2 × SIDE_OFFSET` (0.2 px) are absorbed** —
   side samples land on the same region both sides. The ~0.38 px wedge
   regression pin documents the boundary case.
4. **Exactly-tangent contact** (loops kiss without crossing): the
   split never happens (chord-interior gate), so the result is the
   surviving majority-vote boundary — for inscribed containment the
   correct single loop, for external tangency the two untouched loops.
   A hostile configuration can strand the stitcher → `OpenChain`; that
   error is the honest outcome and is deterministic.

## Consequences

- `vector` is L2 (`doc` + `geom` only), wasm-clean by construction;
  layer enforcement and the wasm build gate it like the rest.
- Boolean results are *new* neutral `PathObject`s: the command layer
  (M6 registry work) decides Fireworks' stroke/fill inheritance rules
  on top, keeping geometry and style concerns separate.
- `SIDE_OFFSET` / `FLATTEN_TOLERANCE` / `SNAP` are exported so callers
  and future golden suites pin the same numbers.
- Future ops (expand stroke, inset) build on the same piece/stitch
  machinery; their offsets must respect the same feature-size floor.
