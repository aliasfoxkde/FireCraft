# Decision 0053 — duplicateSymbolForAlias (P row → C)

**CS6 surface.** `dom.duplicateSymbolForAlias()` (Fireworks 3, no
arguments): "If any symbol instances are selected, this function makes
duplicate symbols of all the symbols that are pointed to by those
instances. The selected instances are updated to point to the new
duplicate copies of the symbols. Duplicate symbols always result in
nonlinked duplicates." The dom's "alias" is an instance in document
terms.

## Design

- **One duplicate per pointed-to definition, not per instance.** The
  dom duplicates "the symbols that are pointed to" — the distinct
  `MSNi` set of the selection's instances, first-seen order. Two
  selected instances of one master share the single copy; the
  documented sentence updates "the selected instances" to "the new
  duplicate copies" (per duplicated symbol).
- **Content clones verbatim; only `MSNi` is fresh.** The duplicate
  takes the next free symbol number (the `next_symbol_number` rule:
  one above the highest, so replay over the restored library
  allocates the same numbers) and keeps name, origin, frame count,
  layers, background, type, nine-slice, button flags, and op code.
  "Nonlinked duplicates" is free here: linkage is not a modeled
  definition property, so every definition this model carries is
  already the nonlinked kind the dom guarantees.
- **Only the selected instances re-point.** An unselected instance of
  the same master keeps the old number — the dom scopes the update to
  "the selected instances", which is the verb's reason to exist
  (detach chosen instances from a master without disturbing the
  rest).
- **Gates.** An empty selection refuses (`InvalidState`), matching
  the selection-walking family. A selection with no instances is the
  no-op; a selected instance whose definition number has no library
  entry (dangling) skips under the walker rule — a selection of only
  dangling instances is the no-op. Non-instance selection members
  skip. All reads (number collection, content clones, per-id
  resolution) run before the first mutation (000557d).
- **Transaction.** `Delta::Library` (the whole pre-command
  definition list) captures the appends; each re-point contributes
  its `Delta::Object` mutated-inverse, folded in a `Delta::Group`.
  Undo restores the library list and the old numbers in one step.

## Consequences

- Registry 238 → **239**; parity 266 C / 5 P / 0 M / 85 N → **267 C
  / 4 P / 0 M / 85 N** (one flip).
- Remaining P rows: 4 — `importFile` (IO placement),
  `setFillNColorNTexture` (texture rasters: `TXFv`'s required `MSKv`
  cannot be minted — the name would write a node the observed grammar
  always pairs with a raster; recorded debt), and the two permanent
  rows (`setAnimInstanceStartFrame`, `setSliceAutonaming`).
