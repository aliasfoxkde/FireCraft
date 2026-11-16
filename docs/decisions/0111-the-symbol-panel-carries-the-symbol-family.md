# 0111 — The symbol instance's panel carries the whole symbol family

Date: 2026-11-12 · Issue: [#136](https://github.com/aliasfoxkde/FireCraft/issues/136) ·
Status: accepted

## Context

The symbol family was the largest group the reachability gate
measured as unreachable: sixteen commands the engine could execute
and the shell's designed surface could not. The engine carried every
piece of it, and the properties panel already showed a symbol
instance's definition, its type, its states and its nine-slice
guides — but the panel could not *change* any of the family's
behavior:

- A button's four behavior flags (`ATSb`, `IDSb`, `DVLb`, `IODb`) and
  its export options (`A2Ts`, `ALTs`, `OBNs`) were invisible.
- A multi-frame instance's animation span — its frame count, travel,
  rotation, scaling and opacity ramp (`ANNi` and its family) — had no
  fields at all.
- A rich symbol's parameters (`currentValues`) and its per-instance
  custom data (`DTAv`) could not be read or written.
- The objects *inside* a definition could not be lifted out
  (`editDetach`) or reordered within the symbol (`editReorder`).
- The one CS6 verb that duplicates a symbol for an alias
  (`duplicateSymbolForAlias`) had no door.

The family is also the one a daily driver needs most: a button is
the common case in real design work, and a button that cannot be
told to auto-slice, to carry a link, or to hold its down state is a
button that has to be rebuilt.

## Decision

- **The instance's panel grows four sections, each owning one part
  of the family.**
  - *Button* — the four behavior flags and the three export options.
    The flags live on the definition, so the write names the symbol
    number; the options live on the definition's linked slices, so the
    panel reads the first linked slice and the engine applies the
    payload to every slice linked to the symbol.
  - *Animation* — the six `setAnimInstance*` writes, shown for an
    instance that already carries an animation or names an animated
    definition.
  - *Parameters* and *Custom data* — the instance's `currentValues`
    entries and its `DTAv` store, with a remove button for a
    custom-data key.
  - *Definition content* — every object in the symbol's cells, with
    the two edits CS6 offers on them: lift it out of the symbol, or
    move it within the symbol's z-order.
- **"Duplicate for alias" sits at the foot of the panel.** The engine
  reads the document's selection, so the panel syncs the view's
  selection into the document first (decision 0104's bridge).
- **The animation verbs are selection-scoped, so the panel syncs
  the selection before each write.** CS6's `setAnimInstance*` verbs
  address "the selected instances", so a write reaches every selected
  instance, not only the one the panel shows.
- **The opacity fields speak the wire's scale.** The model stores
  opacity per mille; the engine's payload is 0–100 and converts. The
  panel reads per mille and shows 0–100.

## Evidence

- The four flag commands address the *definition* by `MSNi` number
  and write one presence-tracked boolean each; the options command
  refuses when no slice is linked to the symbol, and the panel says
  so rather than pretending the write landed.
- The animation commands materialize the carrier with the Objects
  panel's defaults (one frame, no shift, 100 % scale, no rotation or
  opacity ramp) when the instance is plain, so a write to a plain
  instance is not a no-op.
- `symbol.editReorder`'s target index is the position *after* the
  object leaves the cell, so a move up is `position + 1` and a move
  down is `position - 1`; the panel offers a button only where the
  move exists.
- `symbol.duplicateSymbolForAlias` appends one copy per distinct
  pointed-to number and re-points the selected instances to the
  copies.
- The reachability gate measured all sixteen commands as carried by
  the engine and unbound by the shell — the largest family it
  measured.

## Consequences

- Sixteen more engine commands are reachable from the shell's
  designed surface, and the symbol family is fully bound: the
  measurement moves from 196 bound commands of 253 to **212 of 253**
  (77.5 % → 83.8 %).
- A button can be told what to do, an animation can be tuned, a rich
  symbol's parameters can be set, and a definition's contents can be
  edited — from the panel that owns them.

See also decisions [0104](0104-the-shells-selection-verbs-go-through-the-engine.md),
[0108](0108-the-properties-panel-writes-the-text-layout.md) and
[0110](0110-the-guides-and-the-layer-move-reach-the-engine.md) — the
same week's other steps in closing the measured surface gap.
