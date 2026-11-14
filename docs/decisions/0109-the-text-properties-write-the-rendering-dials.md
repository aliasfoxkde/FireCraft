# 0109 — The Properties panel writes the text object's rendering

Date: 2026-11-12 · Issue: [#136](https://github.com/aliasfoxkde/FireCraft/issues/136) ·
Status: accepted

## Context

Decision [0108](0108-the-properties-panel-writes-the-text-layout.md) gave the
Properties panel the text object's *layout*: the paragraph's indent and
spacing, the on-path offset, and the frame that fits the text.

Three things the panel showed about a text object were still inert, and
the engine carried all of them:

- The custom anti-alias dials — MX 2004's `TA1i` sharpness, `TA2i`
  strength and `TOSi` oversampling — sit on the text object and had no
  field.
- The structured string's *runs*. The panel showed only the first
  run's characters, so a text object with several styled runs looked
  like a single line, and the user could not edit the run structure.
- The frame the text flows inside. The panel's geometry row moved the
  object's *bounds* (`object.setBounds`); the text box is a different
  rectangle, the one the shaper wraps at, and it had no field.

Decision 0108 recorded that the anti-alias dials would stay unbound
because the engine carries no anti-alias *preset* setter. That was
wrong about the engine: the three custom dials have setters of their
own and write their fields directly, with no preset to switch on
first.

## Decision

- **The three custom anti-alias dials get fields.** Sharpness and
  strength are 0..=255; the oversampling is a choice of 4, 8 or 16,
  the only values the engine accepts. They write
  `text.setTextCustomAntiAliasSharpness`,
  `text.setTextCustomAntiAliasStrength` and
  `text.setTextCustomAntiAliasOverSample`.
- **The run list gets a field.** One line per run, written through
  `text.setTextRuns`. The base style rides along unchanged — the
  command writes the run list alone — but a run's own style changes
  are *not* carried over: the field writes characters, so editing the
  list gives every run the same style. The field says so in its hint.
- **The text frame gets its own width and height.** They write
  `text.setTextRectangle`, the rectangle the text is laid out in.
  The object's bounds keep their own row.
- **"Fit box at the corner" anchors the fitted box.**
  `text.setTextRectangleAutoFromPoint` is CS6's
  `dom.setTextRectangleAutoFromPoint(anchorPoint)`; the panel anchors
  at the frame's top-left corner, which is where a left-top-anchored
  text item already sits.

## Evidence

- `text.setTextCustomAntiAliasSharpness` and
  `text.setTextCustomAntiAliasStrength` write `anti_alias_sharpness`
  and `anti_alias_strength`; both refuse a value outside 0..=255.
- `text.setTextCustomAntiAliasOverSample` writes
  `anti_alias_oversample` and refuses anything outside {4, 8, 16}.
- `text.setTextRuns` replaces the run list and leaves the base style
  alone when `initialAttrs` is absent; an empty run list clears the
  text.
- `text.setTextRectangle` and `text.setTextRectangleAutoFromPoint`
  read the *document's* selection and refuse a selection that is not
  exactly one text object, so the panel syncs the view's selection
  into the document first — the same bridge decision 0108 established.

## Consequences

- Six more engine commands are reachable from the shell's designed
  surface: `text.setTextCustomAntiAliasSharpness`,
  `text.setTextCustomAntiAliasStrength`,
  `text.setTextCustomAntiAliasOverSample`, `text.setTextRuns`,
  `text.setTextRectangle` and `text.setTextRectangleAutoFromPoint`.
- A text item can be anti-aliased by hand, edited as a run
  structure, and laid out in a frame of the user's choosing — the
  three things CS6's Text panel offers and FireCraft did not.
- Decision 0108's consequence about the anti-alias fields is
  superseded: the dials are bound.

See also decisions [0104](0104-the-shells-selection-verbs-go-through-the-engine.md),
[0107](0107-the-states-panel-reorders-and-fans-the-selection.md) and
[0108](0108-the-properties-panel-writes-the-text-layout.md) — the same
week's other steps in closing the measured surface gap.
