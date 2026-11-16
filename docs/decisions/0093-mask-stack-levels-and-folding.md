# 0093 — A mask stack is nested mask groups, folded bottom-up

Date: 2026-11-11 · Status: accepted

## Context

Decision 0092 decided *where* a mask level's combine mode lives (a
namespaced `DCEv` entry inside the mask group's `DTAv` node, owned by
the model). It deliberately left the rest open: how a stack of levels
is *shaped* in the document, how the levels *fold* into one coverage,
and what a disabled level means.

CS6 has no stack (0092's first section), so again there is no CS6
behavior to copy — the shape has to be one FireCraft can write into a
Fireworks PNG node tree without corrupting a CS6-authored file, and
that the compositor can evaluate.

The document format gives one shape that is already load-bearing: the
element mask itself. A masked object is a group whose children are
`[mask, content]` and whose group type is `GRTi` 1 (grayscale mask) or
2 (shape mask). That pair is already how the model, the writer, and
the compositor describe one element mask.

## Decision

- **A stack is nested mask groups.** A level is a mask group; the
  level's *content child* is the level below it. The outermost group
  is the **top** level and the innermost is the **bottom**. This is
  the shape the document already had for one mask, extended by one
  level: no new group type, no new node, no new wire form. A
  single-level stack is exactly the element mask FireCraft already
  wrote, so the common case costs nothing extra and a CS6 file with
  one mask reads as a one-level stack.
- **The bottom level replaces; every level above combines.** Folding
  is bottom-up. The bottom level's coverage stands on its own — there
  is nothing below it to combine with. Each level above combines its
  own coverage with the coverage below it, using that level's
  combine mode (`MaskCombineMode`): `Add` is the union, `Subtract`
  cuts the level out of the coverage below, `Intersect` keeps the
  overlap, and `Invert` replaces the coverage below with the level's
  inverted coverage. The folded coverage is then applied to the
  content's pixels.
- **A disabled level is skipped, not dropped.** `mask.enable` on a
  level leaves the mask group in place with `MNAb` set (`mask_hidden`)
  and the group type dropped to `GRTi` 0. A level in that state
  contributes **no coverage** — it is skipped in the fold, so the
  level below it applies on its own — but it stays in the stack, in
  place, with its mask and its mode. Toggling a level off is
  therefore reversible and does not reorder anything.
- **The stack's commands are structural, so they must undo
  byte-exactly.** `mask.addLevel`, `mask.removeLevel`, and
  `mask.moveLevel` move objects between containers — the content
  child of one level becomes the group of another. Each of them
  therefore emits a delta whose inverse restores the document's
  serialized bytes exactly, and the engine's fuzz property (a failed
  call mutates nothing; a successful call undoes and redoes
  byte-exactly) covers them.
- **The read side lists the levels.** `engine::catalog::element_mask_stack`
  returns the stack's levels, top level first, each with its group
  id, its mask id, its combine mode, and whether it is enabled. It
  resolves a level's group, a level's mask, or the object the stack
  restricts — so the shell can render one row per level without
  knowing how the stack is nested.

### What this does not do

The compositor folds coverage in the canvas's own space; a level's
mask is sampled at the canvas resolution rather than at the mask
image's own resolution, and the fold is a per-pixel operation with no
feathering beyond the mask's own gray level. The stack's shape is
also not yet editable from the panel beyond enable/mode/reorder/remove
(see task 143's remaining scope).

## Consequences

- The registry grows by three commands (`mask.addLevel`,
  `mask.removeLevel`, `mask.moveLevel`) and the fuzz call table gains
  the matching payload templates, so the fuzz gate's
  call-table/catalog equality still holds.
- The compositor now applies element masks to pixels: a grayscale mask
  contributes its gray level as coverage, a shape mask its painted
  coverage, and the content's own alpha and color scale with the
  folded coverage. The mask artwork is never composited as content.
- The engine's mask family grows a structural half (`element_mask_stack`,
  the level commands) beside its state half (`element_mask_state`,
  `mask.enable`); the singular read side that 0092 published is gone,
  because the stack listing subsumes it.
- Decision 0092's "stored but not rendered" note is now historical:
  the mode reaches pixels. 0092 remains the authority on *where* the
  mode is stored; this record is the authority on the stack's shape
  and its fold.
