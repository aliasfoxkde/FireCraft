# 0014 — Symbol rendering: canvas-anchored rasters, instance spans, nine-slice

- **Status:** Accepted (M10 task 2, 2026-10-12)
- **Deciders:** Michael Kinney, Claude
- **Scope:** `compose` (L3) — how a `SymbolInstanceObject` (`ALSv`)
  becomes pixels; the definition walk, the `ANI*` interpolation, and
  the `NGSv` nine-slice re-grid.
- **Supersedes:** none · **Superseded by:** none

## Context

M10 task 2 turns symbol instances from no-ops into content. No
symbol-bearing corpus file exists (the corpus carries no `MSTv`), so
the wire semantics of `MOXf`/`MOYf`, `MTXv`, and the animation family
have no observational authority. The render model below is therefore
a design choice, pinned by hand-computed golden proofs in
`compose/tests/compose_proof.rs` (the M4 proof shape), and must be
revisited — not silently drifted — if corpus evidence later
contradicts it.

## Decision

### 1. Definitions render as canvas-anchored rasters

A definition's `LYLv` composes exactly like a page — same walk, full
-canvas offscreen, identity transform — and the instance paints that
raster through its `MTXv`. Rationale: CS6 symbols are created from
on-page selections, so definition content is authored in page
coordinates and instances move/scale/rotate the placed content. This
makes every child kind (text, images, nested filters) work inside
definitions with zero new plumbing. **Known limit:** definition
content outside the canvas bounds clips at the raster edge. A
definition-local raster over content bounds would lift the limit at
the cost of bounds+margin machinery across every object kind; deferred
until a corpus file demands it.

`MOXf`/`MOYf` stay metadata (the definition's reference point for
instance placement commands) — they do not participate in rendering.

### 2. Instances span forward from their placement cell

An instance placed in state *j* paints in every state *j..j+span*:

- animation instances (`ANNi` present): span = `ANNi` frames;
- instances of a button definition (`MSYi`=1): span = the
  definition's `FRCi` state cells, showing definition cell
  *k−j* clamped to the last cell (the last state sticks);
- plain graphic instances: their own cell only.

The walk rescans earlier cells newest-first for spanning instances;
the latest placement of an id supersedes earlier ones. Layer
display/per-frame visibility of the *current* state gates the layer;
the *placement* cell's visibility gates the instance. Groups do not
span — an instance inside a group renders only in its group's own
state (CS6 animates top-level instances; nested animation is out of
scope until evidence demands it).

### 3. The `ANI*` family interpolates linearly across the span

`frames` states carry `frames − 1` transitions: state *j* is the
untouched instance, the last state carries the full `ANXf`/`ANYf`
shift, `ANRi` rotation (degrees, canvas-origin pivot), and the scale
from 100% to `ASCf`%. Interpolation parameter *t = step/(frames−1)*;
opacity lerps `ASOi`→`AEOi` (each defaulting to 1000‰). There is no
pre-existing tween engine in the workspace — this linear pass *is*
the tween machinery (M10 ticket wording anticipated one).

## Consequences

- Deterministic: the nine-slice re-grid samples by integer
  nearest-neighbor math and the transformed blit samples nearest
  (`ImageQuality::Low`); golden proofs hold byte-exactly.
- Dangling `MSNi` is a typed refusal
  (`ComposeError::MissingDefinition`) — a document defect, never a
  silent hole, matching `DanglingId`/`TileBankMiss` semantics. The
  fuzz driver seeds one definition so generated instances resolve.
- Cycles and runaway nesting are typed refusals (`SymbolCycle`,
  `SymbolDepth` at 32), never recursion.
