# Decision 0047 — loose-ends setters (P rows → C)

**CS6 surface.** Four dom verbs that the sweep had parked as partial,
each implementable against the existing model with no new wire
format:

- `dom.setEffectName(target, oldName, newName)` — "Changes the name
  of an effect in the selection." The target names an effect
  category; the rename hits every selected object carrying that
  `(CATs, INMs)` pair.
- `dom.addSwapImageBehaviorFromPoint(where)` — "Adds a swap image
  behavior to the selected slice or hotspot. The swap image is the
  slice or hotspot under the specified point." Returns false when no
  slice sits at the point.
- `dom.replaceButtonTextStrings(startDate, text, uniformAttrs)` and
  `dom.replaceButtonTextStringsInInstances(startDate, text,
  uniformAttrs)` — "Replaces the text in Button text items. Button
  Text items are defined as the topmost text items on each frame" —
  the first across every button symbol in the document, the second
  scoped to the symbols behind the selected instances.

## Design

- **`setEffectName` is a selection walker.** For every selected id:
  stale ids skip (the animation-instance walker's rule), objects
  without an effect spine skip, and objects carrying no
  `(category, old_name)` match skip — only actual renames produce a
  delta, so a sweep that matches nothing is the no-op (not an
  error). The rename writes `INMs` in place; `CATs` and `DCVs`
  properties are untouched (renaming never re-parameterizes).
  An empty `new_name` refuses before any capture (000557d).
- **`addSwapImageBehaviorFromPoint` resolves the source first.**
  Gates in order: finite `where`, exactly one selected slice/hotspot
  (CS6's "the selection is the behavior target" contract), then the
  point hit — the topmost slice/hotspot whose shape contains the
  point (rect/ellipse frames use their frame; polygons get a real
  even-odd contour test because their frame is only the bounding
  box). A miss refuses — CS6's documented `false` return becomes a
  refusal, not a silent success. The hit writes the kind-2
  (swap-image) behavior on the selection plus the `DTAv` `swapTarget`
  custom-data key carrying the source's object name (falling back to
  its internal name; an unnamed source writes the behavior with no
  target key, decision 0017's wire shape).
- **Button-text replacement filters to `SymbolType::Button`.** The
  dom says "Button text items"; a graphic symbol's text never
  changes, even when a graphic instance is in the selection — the
  filter lives in the shared tail so both variants are safe by
  construction. The walk takes the topmost text item per frame
  (cells in order, elements reversed). The rewrite collapses the
  item's runs to a single run with the new text (the single-run text
  model — `uniformAttrs` has nothing to preserve and is recorded as
  a no-op under the model limitation, mirroring `text.setText`).
  An empty text empties the runs. Definition numbering is
  deduplicated and resolved like `library.rename`; a number that no
  longer exists skips. The all-variant sweeps every Button
  definition in `document.symbols`; the instances variant reads
  `symbol_number` from the selection's instances (an empty selection
  refuses).
- **`setSliceAutonaming` stays P.** The autonaming flag has no wire
  node — the slice's `FILs` frame carries only the naming label.
  The flag is UI state, and fabricating a node for it would violate
  the no-format-fabrication rule.

## Consequences

- Registry 218 → **222**; parity 245 C / 26 P / 0 M / 85 N → **250 C
  / 21 P / 0 M / 85 N** (five flips — the CS6 alias
  `AddSwapImageBehaviorFromPoint` carries with the lowercase verb).
- `filter.rs` gained the shared `object_effects` spine accessor
  alongside `object_effects_len`/`with_effects`; `web.rs` gained the
  even-odd `polygon_contains` (fixed control flow, no
  chained-comparison trickery) and the document-wide `slice_at`
  scan.
- Remaining P rows: 21. The big clusters left are the text
  markup/para/run verbs (multi-run model), the anti-alias trio
  (renderer-side), and `importFile` (needs an IO-placement command
  design).
