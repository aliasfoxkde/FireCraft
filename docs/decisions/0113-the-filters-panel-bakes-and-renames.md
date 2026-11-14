# 0113 — The Filters panel bakes and renames

Date: 2026-11-12 · Issue: [#136](https://github.com/aliasfoxkde/FireCraft/issues/136) ·
Status: accepted

## Context

The Filters panel carried the *live* effect stack: `filter.add` to
push a preset onto the selection, `filter.setParam` to tune it,
`filter.remove` to drop it. Two CS6 verbs had no door:

- `dom.setEffectName(category, oldName, newName)` — Fireworks MX's
  way of naming an applied filter.
- The bake family — `dom.filterSelection`, `dom.filterSelectionByID`
  and `dom.filterSelectionByName`, plus `dom.convolveSelection` and
  `dom.motionBlurSelection` — which apply a filter to the selection
  and bake it into the pixels.

The bake family is the one a daily driver wants: it is how a user
applies a filter by its CS6 name, with its CS6 parameters, without
the live stack standing between the request and the bitmap.

## Decision

- **Every effect row gains a rename field.** The write names the
  effect's category and its current internal name, and carries the
  replacement — CS6's three-name contract, unchanged.
- **The panel gains a Bake section** with CS6's own verbs:
  - *bake* — the filter by category and name;
  - *bake by catalog name* — the same, with the filter's GUID taken
    from the catalog (`filterSelectionByName`);
  - *bake by id* — the filter by its `EffectMoaID`
    (`filterSelectionByID`);
  - *convolve* — an arbitrary integer kernel, taken as the dom takes
    it: raw coefficients, row major, no normalization;
  - *motion blur* — the dom's three kinds (linear, radial, zoom) with
    their angle, distance and tap count.
- **The kernel's coefficients are entered as text.** The dom takes
  them raw and unnormalized, so a numeric grid would invite the
  mistake of "summing to one". The panel parses the text, refuses a
  count that does not match the kernel's sides, and says so.
- **The bake verbs sync the selection first.** They read the
  *document's* selection, so the panel bridges the view's selection
  into the document (decision 0104's bridge) before the write.

## Evidence

- `filter.setEffectName` matches on both the category and the
  internal name, and renames every matching effect on the selected
  objects; no match is the no-op.
- The bake commands resolve their targets completely before any
  mutation, and refuse when the filter does not resolve to a pixel
  kernel — the engine's no-passthrough floor law.
- `filter.convolveSelection` checks that the coefficient count
  equals the kernel's cell count, and `filter.motionBlurSelection`
  refuses a kind outside `linear`/`radial`/`zoom` — the refusals the
  panel's own tests assert.
- The reachability gate measured all six commands as carried by the
  engine and unbound by the shell.

## Consequences

- Six more engine commands are reachable from the shell's designed
  surface, and the filter family is fully bound: the measurement
  moves from 218 bound commands of 253 to **224 of 253** (86.2 % →
  88.5 %).
- A filter can be named, and a filter can be applied to the pixels
  by name, by GUID, by kernel or as a motion blur.

See also decisions [0104](0104-the-shells-selection-verbs-go-through-the-engine.md),
[0109](0109-the-text-properties-write-the-rendering-dials.md) and
[0111](0111-the-symbol-panel-carries-the-symbol-family.md) — the same
week's other steps in closing the measured surface gap.
