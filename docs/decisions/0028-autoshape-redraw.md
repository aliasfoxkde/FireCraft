# Decision 0028 — autoshape.redraw (CS6 redrawSmartShape)

Date: 2026-10-14
Closes: HANDOFF §4 Phase 6.2 — parity row `redrawSmartShape` flips
`M` → `C` (189 C / 45 P / 39 M / 83 N).

## CS6 verb

`dom.redrawSmartShape()` (Fireworks 8, no arguments): "Forces the Auto
Shapes in the document to redraw. This method is useful when modifying
a Auto Shape outside a tool." In CS6 an Auto Shape instance carries
its parameters inside the shape's own JSF generator script; the verb
re-runs that script from the instance's stored state. CS6 edits the
parameters through the shape's control handles, not through this verb.

## Wire / model mapping

FireCraft's Auto Shape library (`autoshape.*`) bakes
geometry at insert: the inserted object is a plain path with
`primitive: None`, parameters live at the calling layer, and there is
no script interpreter and no wire node for script state (the
dictionary has no smart-shape parameter node — the shape's JSF is a
Configuration-tree artifact, not a PNG payload). A faithful
"redraw from embedded state" is therefore impossible by construction;
the engine surface this verb names is the *parametric* redraw the CS6
UI drives behind the tool:

- `autoshape.redraw {id, kind, params}` — `kind` selects the library
  entry (strings match the `autoshape.*` command-id suffixes, so
  `arrowLine`/`connectorLine` keep their camelCase), `params` carries
  that entry's geometry surface (dedicated structs — the create
  payloads' page/layer/cell/name fields are dead weight here; the
  object already sits in its cell). The generator runs, output is
  finite-checked (`finite_geometry`, same gate as insert), and the
  target path's contours + `even_odd` swap behind one captured
  `Delta::Object {Mutated}`.

Styling (`paint`, opacity, blend, effects), name, and cell placement
are untouched: a redraw re-parameterizes, it does not restyle or move.

Any path object is a valid target — the model has no marker that
distinguishes an autoshape-baked path from a user-drawn one, and
inventing a marker would be wire state CS6 does not carry either
(its distinction lives in the script, off-wire).

## Deviations

- Parameters travel in the command payload instead of living in the
  shape instance; CS6's no-argument document-wide form is a no-op on
  baked geometry and is not modeled.
- Redraw across kinds is allowed (star → doughnut) — CS6 could not do
  this (the script is fixed per shape), but with kind explicit in the
  payload refusing it would be pure ceremony.

## Verification

Engine unit tests (4 new): re-parameterize a 5-point star to 6 and
undo to the exact original path; the golden gate — a redrawn shape's
contours equal a freshly created shape's contours for the same
parameters; star → doughnut flips `even_odd` while the name survives;
refusals (unknown kind string, wrong-shape params, unknown id) leave
the document byte-identical. Fuzz: `autoshape.redraw` joins `CALLS`
with an arm covering all eleven kinds; the CALLS==catalog pin holds
(registry 164 → 165).
