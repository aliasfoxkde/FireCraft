# 0018 — `path.setFillColor` collapses a gradient fill to solid

- **Status:** Accepted (post-roadmap gap close, 2026-10-13)
- **Deciders:** Michael Kinney, Claude
- **Scope:** `engine::catalog::path::PathSetFillColor` (L2), the
  M15 tool-surface record, and any future Wine-oracle corpus playback
  of the DOM verb.
- **Supersedes:** none · **Superseded by:** none

## Context

M15 shipped the Gradient tool headless as three DOM verbs:
`path.setFill` (whole definition), `path.setFillColor` (the narrow
colour verb), `path.setFillVector` (the handles). The colour verb's
behavior on a gradient-filled object was pinned as "the definition
collapses to the solid family" with the note *playback pending corpus
evidence* — the honest state at the time, since no CS6-oracle corpus
exercised it. This decision resolves the pin from the documented
evidence that is available on every dev host, and names the one
experiment that could still overturn it.

## Evidence

1. **CS6 help (Edit a gradient fill):** with a gradient fill selected,
   clicking the Fill Color box opens the Edit Gradient pop-up; colour
   changes happen **per stop** ("To set or change the color of a color
   swatch, click the color swatch and select a color"). The documented
   UI path that keeps a gradient has no bare-colour entry point — the
   Fill Color box *is* the gradient editor on such objects.
2. **"Extending Fireworks":** `dom.setFillColor(color)` takes a bare
   colour and "changes the fill color of the selection";
   `dom.setFill(fill)` takes the whole Fill object; there is no
   per-stop DOM verb (`setFillNColor` also takes a Fill object — "sets
   the selection to the specified fill **and** fill color"). The
   colour-preserving edit documented in (1) is reachable only by
   scripting `getFill()` → mutate `gradient.nodes` → `setFill()`.
3. **`pathAttributes.fillColor`:** "the color used for rendering the
   Fill object, **if any**" — the colour slot renders only where the
   definition does not paint (i.e. solid fills). A colour-slot-only
   write on a gradient object is visually inert, which would make the
   verb's own description false for exactly the objects in question.
4. **Format dictionary:** `PATv` = `BCLi` + `FCLi` + `FPLv?`; `FPLv`
   carries `FSHi` (0 = solid) and the optional `FGLv` ramps. A solid
   fill is a definition (`FSHi` 0), not the absence of one, so
   "make it this colour" is a definition rewrite — the same object
   shape our collapse produces (`FillShape::Solid`, gradient cleared).
5. **Configuration `.jsf` tree:** the only `setFill*` call sites in
   the shipped scripts are `setFill(null)` (remove fill); no script
   calls `setFillColor`, and the FXG exporter reads gradient nodes and
   the colour slot as independent fields — confirming coexistence in
   the data model but giving no counter-behavior.

## Decision

`path.setFillColor` on any path replaces the fill with the solid
colour: `fill_raw` moves, `fill.shape` becomes `Solid`, and the
gradient/dither definitions clear. The gradient handles
(`fill_start`/`fill_end`/`fill_third`) stay where they are — they are
dormant under a solid definition, and CS6's category-switch keeps the
last-used gradient geometry the same way. One undo step restores the
whole prior definition (the `Mutated` inverse already carries it).

The recorded corpus invariant stands unchanged: `FCLi` rides the first
RGB stop on gradient-filled files because CS6 maintains the fallback
colour slot when a definition is present — it is not evidence that the
colour verb writes only that slot.

## Falsification path

The one experiment that could overturn this: Wine-oracle playback
(`corpus` incoming battery, unavailable on this host — no Wine, no
`corpus/incoming/`) scripting `setFill(gradient)` then
`setFillColor(color)` against CS6 and diffing the written `PATv`. If a
played-back file keeps `FGLv`/`FSHi` ≠ 0 after a `setFillColor`, the
corpus wins: flip to colour-slot-only (or definition-preserving)
semantics here and record the discrepancy. Until such a file exists,
the documented model above is the authority, and an encode diff
against any future real-corpus file surfaces the disagreement
automatically in the round-trip battery.

## Consequences

- The M15 "playback pending corpus evidence" gap closes as
  evidence-resolved: the collapse is the documented reading, pinned by
  an engine test (collapse, undo restore, redo), and the pin's
  falsifier is named rather than left implicit.
- No format, render, or export change: the collapsed state is the
  `FPLv` solid definition the writer already emits.
