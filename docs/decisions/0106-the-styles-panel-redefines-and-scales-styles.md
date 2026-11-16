# 0106 — The Styles panel redefines and scales styles

Date: 2026-11-12 · Issue: [#140](https://github.com/aliasfoxkde/FireCraft/issues/140) ·
Status: accepted

## Context

The Styles panel could apply a style to the selection and import a
style library, and nothing else. The engine has carried
`style.redefineByIndex`, `style.redefineByName` and `style.scaleStyles`
from the start — the CS6 DOM verbs `redefineStyleByIndex`,
`redefineStyleByName` and `scaleStyles` — and the shell could not reach
any of them.

The consequence was a style library that could only be edited outside
the application. Redefining a style is how a user keeps a style in sync
with the object they have just formatted; scaling the library is how a
design is rescaled across a set of type styles. In FireCraft both
required leaving the app.

## Decision

- **Each style row carries a "redefine" button.** It captures the
  appearance of the current selection into that style, through
  `style.redefineByIndex` — the row knows its index, so the index is
  the addressing the engine wants.
- **The panel carries a "text scale" factor and a "Scale Styles"
  button.** The button dispatches `style.scaleStyles` with the factor,
  which multiplies every library entry's text size. It is one factor
  for the whole library, because the CS6 verb is one factor for the
  whole library.
- **The scale lives in the shell, not the document.** It is a control
  value, like the rename field, and it starts at 1.0.
- **The engine's refusals stand.** A non-positive or non-finite factor
  is refused by `style.scaleStyles`; scaling an empty library is a
  no-op. Neither is reimplemented in the panel.

## Evidence

- `style.redefineByIndex {index, source}` captures the named object's
  appearance into the library entry at `index`; `style.scaleStyles
  {scale}` multiplies every entry's `text.size` and refuses a
  non-positive factor.
- The Styles panel already bound `style.apply` and
  `style.redefineFromSelection`; the two new commands are the same
  shape of binding — a button in the panel, a command through the
  engine's door, the document's answer in the next frame.
- The reachability gate measured `style.redefineByIndex`,
  `style.redefineByName` and `style.scaleStyles` as carried by the
  engine and unbound by the shell.

## Consequences

- Two more engine commands are reachable from the shell's designed
  surface: `style.redefineByIndex` and `style.scaleStyles`.
- A style library can be kept in sync with the canvas and rescaled
  without leaving the application.
- `style.redefineByName` remains unbound: it is the same capability as
  the index form, addressed by name, and the panel addresses styles by
  index. It stays in the registry for the scripting host.

See also decision [0104](0104-the-shells-selection-verbs-go-through-the-engine.md)
— the same week's other step in closing the measured surface gap.
