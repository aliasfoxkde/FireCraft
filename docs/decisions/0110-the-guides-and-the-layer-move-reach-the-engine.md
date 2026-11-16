# 0110 — The Pages panel lists the guides; the Layers panel moves the selection

Date: 2026-11-12 · Issue: [#136](https://github.com/aliasfoxkde/FireCraft/issues/136) ·
Status: accepted

## Context

Two things FireCraft's shell could *make* but could not *undo or
move*:

- A guide could be dragged out from a canvas edge — the guide draft
  commits through `page.addGuide` — but the page's guide list had no
  rows. A guide could not be nudged, and it could not be removed at
  all. The engine carried `page.changeGuide` and `page.removeGuide`
  for exactly that.
- A layer could be renamed, locked, reordered and made active, and
  its objects could be selected — but the selection could not be
  *moved* onto another layer. The engine carried `object.moveToCell`
  for it.

Both are ordinary CS6 moves. Fireworks moves a guide by dragging it
in the Guides dialog and removes it with the same gesture; it moves
objects between layers from the Layers panel.

## Decision

- **The Pages panel lists the page's guides.** Each guide is a row
  with its position and a remove button. Editing the position writes
  `page.changeGuide`; the button writes `page.removeGuide`. The
  panel reads the list from the document (`page.guides`) rather than
  from view state, so a guide the user dragged out on the canvas
  shows up here like any other.
- **The Layers panel gains "move the selection here."** The button
  on a layer row moves the selected objects onto that layer, in the
  current state, through `object.moveToCell`. The engine's command
  moves one object at a time, so the panel moves the selection one
  by one and skips the objects that already live on the target
  layer — a move that changes nothing does not become an undo step.

## Evidence

- `page.addGuide` appends a position to the page's horizontal or
  vertical list; `page.changeGuide` replaces the first guide sitting
  at the given position and *refuses* a position that carries no
  guide; `page.removeGuide` succeeds without touching the document
  when the position carries no guide (CS6 documents the absent case
  as a no-op).
- `object.moveToCell` takes the target page, layer and cell, and an
  optional insert index (absent appends). A move within the source
  cell folds into one cell delta.
- The reachability gate measured all three commands as carried by
  the engine and unbound by the shell.

## Consequences

- Three more engine commands are reachable from the shell's designed
  surface: `page.changeGuide`, `page.removeGuide` and
  `object.moveToCell`.
- A guide can be corrected or discarded without opening a file, and
  an object can be handed to another layer from the panel that owns
  the layers.

See also decisions [0104](0104-the-shells-selection-verbs-go-through-the-engine.md),
[0108](0108-the-properties-panel-writes-the-text-layout.md) and
[0109](0109-the-text-properties-write-the-rendering-dials.md) — the
same week's other steps in closing the measured surface gap.
