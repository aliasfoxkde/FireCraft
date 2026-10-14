# Decision 0030 — crop selection, fill edge mode, tween
(`path.cropSelection`, `path.setFillEdgeMode`, `symbol.tween`;
`setGradientName` → N)

Date: 2026-10-14
Closes: HANDOFF §4 Phase 8.5 — parity rows `cropSelection`,
`tween`, `setFillEdgeMode` flip `M` → `C` and `setGradientName`
flips `M` → `N` (194 C / 45 P / 33 M / 84 N).

## CS6 verbs

- `dom.cropSelection(boundingRectangle)`: "Crops the selection to
  the specified rectangle."
- `dom.setFillEdgeMode(edgemode, featherAmt)`: "Sets the edge type
  for selected items with fills." — `"hard edge"` / `"antialias"` /
  `"feather"`.
- `dom.tween(numSteps, bDistribute)`: "Tweens between the two
  selected instances."
- `dom.setGradientName(currentName, newName)`: "Renames a gradient."

## Mapping

**cropSelection → `path.cropSelection {rect: [x0, y0, x1, y1]}`.**
Each selected path is combined with a synthetic rectangle blade via
`vector::boolean(.., Crop)` (the intersection — the same op
Fireworks' combine-path Crop uses, decision 0013), replacing the
path's geometry in place while keeping its display fields, exactly
like `path.boolean`. A path entirely outside the rectangle produces
an empty result and is removed (detach + `ObjectInverse::Removed`),
mirroring the boolean empty-result contract. The whole selection is
cropped *before* anything mutates, so one hostile member refuses the
command untouched; empty-result removals are pre-flighted with
`ensure_detachable`. An empty selection is an explicit
`Delta::Noop`. Non-path members refuse ("only paths crop"):
cropping a bitmap is a pixel operation and belongs to the bitmap
bake pass (debt), and groups/text/slices have no crop semantics on
the wire.

**setFillEdgeMode → `path.setFillEdgeMode {id, mode, feather?}`.**
The three CS6 strings map onto the two edge slots the `FPLv` fill
already carries: `hard` sets `FRDb` (`hard_edge`) and clears `FEFi`
(`feather`); `antialias` clears both; `feather` clears `FRDb` and
stores the amount in `FEFi`. Nothing new on the wire — this is the
Phase-4 rule (do not invent state) applied to an existing model
slot. A path whose paint has no fill definition, or no paint at
all, gains one (a solid definition with the requested edge), the
same materialization `path.setFill` performs when the panel first
opens Edge on a solid fill.

**tween → `symbol.tween {numSteps, distribute, frame}`.** CS6
tweens "between the two selected instances"; the engine requires
exactly two selected instances of one `symbol_number` sharing one
cell of one page (the dialog's own precondition — it refuses
otherwise). Each new instance interpolates the Transform panel's
knobs — translation, rotation, scale — via polar decomposition of
the endpoint affines, with the rotation taking the shortest arc
(`to -= round(turn) * TAU`). Copies are fresh instances: unlocked,
unnamed, no animation, carrying the first endpoint's custom data.
With `bDistribute`, step *i* lands on frame `frame + i`
(`distributeToFrames` placement, decision 0022's model), appending
frames when `frame + numSteps + 1` exceeds the page's count; the
whole page is captured first (`Delta::PageBody`), so undo is byte
exact across the frame growth. Without it, instances attach after
the first endpoint inside the shared cell.

**setGradientName → N.** No document home for the name: gradient
definitions live in Fireworks' gradient library (application
preferences, not the document), and the document-side `FGLv` names
that do exist ride unknown bags on fills — there is no gradient
library node to rename. Same verdict as `setRectRoundnessMode`
(decision 0027's rule): renaming preferences state is not an
engine verb, so the row is `N` with evidence rather than an
invented model.

## Deviations

- Tween interpolates transform only: `SymbolInstanceObject` has no
  opacity field and per-instance animation parameters are not
  tweened (copies carry none) — CS6's dialog can also tween
  opacity/effects where they exist; those become engine debt under
  the instance model, not silent gaps.
- cropSelection on bitmaps/text/groups is refused, not performed —
  the CS6 verb crops a bitmap selection; it returns with the
  bitmap bake pass.

## Verification

`cargo test -p engine --lib` (271 tests) — 5 new path tests (mode
mapping + paint materialization + refusal matrix; crop clip +
field inheritance + exact undo; outside-path removal; empty
selection Noop + bad-rect/non-path refusals) and 4 new symbol
tests (lerped placement + exact undo; shortest-arc rotation;
distribute frame growth + exact undo; refusal matrix). Fuzz drive
exercises all three verbs (registry pin 170).
