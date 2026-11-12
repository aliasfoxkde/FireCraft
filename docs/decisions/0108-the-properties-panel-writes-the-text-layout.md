# 0108 — The Properties panel writes the text object's layout

Date: 2026-11-12 · Issue: [#136](https://github.com/aliasfoxkde/FireCraft/issues/136) ·
Status: accepted

## Context

The Properties panel showed a text object's characters, its font, its
size and its bold/italic/underline flags. It showed nothing of the
*layout*: a paragraph's indent, the space above and below it, where a
text-on-a-path item starts along its path, and the frame the text
flows inside.

The engine has carried all of it: `text.setTextParaIndent`,
`text.setTextParaSpacingBefore` and `text.setTextParaSpacingAfter`
write the `TFSv` paragraph attributes (`PINf`, `PSBf`, `PSAf`),
`text.setTextOnPathOffset` writes the `CHOf` start offset, and
`text.setTextRectangleAuto` measures the laid-out block and shrinks
the frame around it.

A text box in FireCraft therefore behaved as a fixed rectangle: the
user could not indent a paragraph, could not space paragraphs apart,
and could not tighten the frame around the text — the three things
every word processor offers in the same place.

## Decision

- **The text properties carry the paragraph's three values.** Each is
  read from the object's text style and written through the matching
  paragraph setter — the engine owns the values, the panel owns the
  field.
- **A text item on a path carries its offset.** The offset field is
  offered only for a text object that has a path, and writes
  `text.setTextOnPathOffset`.
- **"Fit box to text" measures and writes the frame.**
  `text.setTextRectangleAuto` is CS6's `dom.setTextRectangleAuto()`:
  the frame becomes the smallest box that encloses the text.
- **The two selection-scoped setters sync the view's selection into
  the document first.** `text.setTextOnPathOffset` and
  `text.setTextRectangleAuto` read the *document's* selection, while
  the panel shows the *view's* selection — so the panel bridges them
  the same way the Select menu's verbs do (decision 0104).

## Evidence

- The paragraph setters write `style.paragraph_indent`,
  `style.space_before` and `style.space_after` on the object's
  `TextStyle` — the same fields the panel reads.
- `text.setTextRectangleAuto` measures the block with no width
  constraint and writes the frame left/top-anchored; the fit refuses
  a block that lays out to no box.
- `text.setTextOnPathOffset` walks the document's selection and skips
  items that are not on a path.

## Consequences

- Five more engine commands are reachable from the shell's designed
  surface: `text.setTextParaIndent`,
  `text.setTextParaSpacingBefore`, `text.setTextParaSpacingAfter`,
  `text.setTextOnPathOffset` and `text.setTextRectangleAuto`.
- The text box behaves like a word processor's: paragraphs are laid
  out, and the frame can follow the text.
- The panel's custom anti-alias fields stay unbound: the engine has
  no command for the anti-alias *preset*, so a sharpness or strength
  field would have nothing to switch it on with.

See also decisions [0104](0104-the-shells-selection-verbs-go-through-the-engine.md)
and [0107](0107-the-states-panel-reorders-and-fans-the-selection.md) —
the same week's other steps in closing the measured surface gap.
