# 0107 — The States panel reorders the states and fans the selection

Date: 2026-11-12 · Issue: [#139](https://github.com/aliasfoxkde/FireCraft/issues/139) ·
Status: accepted

## Context

The States panel could add, duplicate and delete a state, set its
delay and turn onion-skinning on. It could not move a state, and it
could not put the *selection* onto the page's other states.

The engine has carried all of it: `frame.reorder` is CS6's
`dom.reorderFrame`; `selection.duplicateToFrames` is
`dom.duplicateSelectionToFrames`; `selection.duplicateToFrameRange` is
`dom.duplicateSelectionToFrameRange`; `selection.distributeToFrames`
is `dom.distributeSelectionToFrames`; and `cell.setVisible` /
`cell.setLocked` carry the two flags a cell can hold. The panel had no
row for any of them.

The two selection fans matter most for the animation workflow the
panel exists for: an animator draws a run of instances on one state
and wants them spread across the states, or wants one drawing copied
onto every state. The panel's "Distribute" button did the first of
those for a *layer* (`frame.distributeLayer`), never for the
selection the user actually has on the canvas.

## Decision

- **The strip gains a move.** "Move left" and "Move right" dispatch
  `frame.reorder` for the current state. The engine's `before` is an
  index in the strip's own numbering, and it shifts the list by one
  when the moved state came first — so a move to the right names the
  state *after* the target, and a move to the left names the target
  itself. The panel computes that, not the engine.
- **The selection gains three fans.** "Duplicate to next" and
  "Duplicate to all" dispatch `selection.duplicateToFrames`; the
  range row ("from" / "to") dispatches
  `selection.duplicateToFrameRange`; "Distribute selection" dispatches
  `selection.distributeToFrames`.
- **Every fan syncs the view's selection into the document first.**
  The engine's fans read the document's selection, and the view's
  selection is the one the user sees — the same bridge the Select
  menu's verbs use (decision 0104).
- **The active layer's cell in the current state gains its two
  flags.** A "visible" checkbox and a "locked" checkbox dispatch
  `cell.setVisible` and `cell.setLocked`. The flags live on the cell,
  so they belong to a layer rather than to the page — the panel says
  so, and offers them only for a cell the active layer actually has.

## Evidence

- `frame.reorder` refuses an out-of-range `from` or `before`, and
  treats a move onto its own slot as CS6's no-op.
- `selection.duplicateToFrames` takes a fan — "all", "previous",
  "next" or "end" — and `selection.duplicateToFrameRange` takes an
  inclusive range whose `-1` names the current frame; the range never
  creates frames.
- `selection.distributeToFrames` moves the items one per state from
  the source state and adds frames when the run outruns the page; one
  item is the documented no-op.
- The reachability gate measured all six commands as carried by the
  engine and unbound by the shell.

## Consequences

- Six more engine commands are reachable from the shell's designed
  surface: `frame.reorder`, `cell.setVisible`, `cell.setLocked`,
  `selection.duplicateToFrames`, `selection.duplicateToFrameRange`
  and `selection.distributeToFrames`.
- The animation workflow can be driven from the panel alone: draw,
  fan, reorder.
- The panel carries two more control values — the range fan's "from"
  and "to" — in the shell's state, not the document's, because they
  are a control, not content.

See also decisions [0104](0104-the-shells-selection-verbs-go-through-the-engine.md)
and [0106](0106-the-styles-panel-redefines-and-scales-styles.md) — the
same week's other steps in closing the measured surface gap.
