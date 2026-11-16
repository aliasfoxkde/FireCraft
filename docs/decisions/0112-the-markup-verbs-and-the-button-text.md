# 0112 — The text panel carries the markup verbs; the button carries its text

Date: 2026-11-12 · Issue: [#136](https://github.com/aliasfoxkde/FireCraft/issues/136) ·
Status: accepted

## Context

The text panel wrote the base style: `text.setAttrs` for the font,
the size and the bold/italic/underline flags. CS6's Format ▸ Text
menu does something else — it applies *markup*, which restyles the
**runs**, so that a text item with mixed formatting changes only the
part the user selected. The engine carried four verbs for that
(`text.applyCharacterMarkup`, `text.removeCharacterMarkup`,
`text.applyFontMarkup`, `text.removeFontMarkup`) and the shell
reached none of them.

The button family had the same gap on its text side: CS6's
`dom.replaceButtonTextStrings` and
`dom.replaceButtonTextStringsInInstances` — "change every button's
label" and "change the labels of the buttons these instances
reference" — were unreachable.

## Decision

- **The text panel grows a Markup section.** Character markup
  (`bold`, `italic`, `underline`, and CS6's `fwplain` for "no
  character markup, base included") and font markup (`size`, `face`).
  Each verb has its inverse — "clear bold", "clear size" — so the
  markup can be taken back as well as applied.
- **The font markup carries the dom's own value forms.** The size
  takes the `"NNNpt"` form the dom refuses to do without; the face
  takes a family name. The panel offers the text item's current base
  size and family, so the common case is one click.
- **The button section grows the two button-text verbs.** "In library"
  replaces the text of every button definition; "in instances"
  replaces the text of the definitions the selected instances
  reference, and syncs the view's selection into the document first
  (decision 0104's bridge).

## Evidence

- `restyle_runs` computes each run's *delta* against the previous
  effective style, so a markup verb states only what changed — the
  base style and the runs stay in step.
- `text.applyFontMarkup` refuses a bare number for the size ("a
  simple numeric value is not allowed"), so the panel sends the
  `"NNNpt"` form.
- The button-text verbs reach only *button* definitions — the
  instances-scoped one skips a definition that is not a button — and
  replace the runs with one run of the replacement characters.
- The reachability gate measured all six commands as carried by the
  engine and unbound by the shell.

## Consequences

- Six more engine commands are reachable from the shell's designed
  surface, and the text family is fully bound: the measurement moves
  from 212 bound commands of 253 to **218 of 253** (83.8 % → 86.2 %).
- A mixed-format text item can be formatted the way CS6 formats it,
  and a button's label can be changed from the panel that owns it.

See also decisions [0108](0108-the-properties-panel-writes-the-text-layout.md),
[0109](0109-the-text-properties-write-the-rendering-dials.md) and
[0111](0111-the-symbol-panel-carries-the-symbol-family.md) — the same
week's other steps in closing the measured surface gap.
