# Decision 0038 — named saved selections

**Date:** 2026-10-14 · **Phase:** 8.7 · **Status:** landed —
`selection.save` / `selection.restore` / `selection.deleteSaved`

## Verbs (CS6 API text)

- `dom.saveSelection({selectionName}, {toDocument}, {operation})` —
  Fireworks 4. "Stores the current selection in bitmap mode as the
  saved selection." `selectionName` defaults to `"default"`;
  `operation` ("new or replace" / "add" / "subtract" / "intersect")
  folds the new selection into an existing saved one under the same
  name; `toDocument` targets another open document.
- `dom.restoreSelection(selectionName, fromDocument, {operation},
  {invert})` — Fireworks 4. Restores a named selection;
  `operation` folds it with the active selection; `invert` flips the
  saved mask before the fold. Returns `true` if inverted.
  `selectionName` defaults to `"default"`.
- `dom.deleteSavedSelection(selection)` — Fireworks 3. Deletes the
  named saved selection. (The manual's Description paragraph under
  this verb is a copy-paste of `deleteSelection`; the Usage and
  Arguments are unambiguous.)

No `.jsf` usage of any of the three exists in the CS6 Configuration
tree; the API text is the authority.

## Design

- **Model:** `Document.saved_selections: Vec<SavedSelection>` —
  `SavedSelection { name, mask: SelectionMask }`. Editing state,
  never file content: the format has no node for it and the writer
  never reads the field — the same class as `Document.selection` and
  the `jpeg_mask` stash. Order is save order (a small list, scanned
  by name).
- **Delta:** `Delta::SavedSelections { before: Vec<SavedSelection> }`
  — the whole list rides the capture (the state is small and the
  restore is trivially byte-exact), mirroring `Delta::JpegMask`.
- **`selection.save`** `{name?, operation?}` — the active pixel
  selection is required (an empty save is unrestoreable state; CS6's
  UI posture guarantees a bitmap-mode marquee — deviation 2). With no
  saved selection under the name, the active mask is stored as-is.
  With one present, the stored mask becomes
  `fold(stored, active, operation)` — replace/add/subtract/intersect
  through the same `mask::fold` the marquee commands use
  (subtract = stored minus active; the CS6 text gives no operand
  order — the chosen order is recorded here).
- **`selection.restore`** `{name?, operation?, invert?}` — unknown
  name is `MissingTarget`. The saved mask is cloned, inverted
  (`255 - coverage`) when `invert` is set, then folded with the
  active selection through `operation` (default replace; no active
  selection → the saved mask becomes the selection). A fold that
  covers nothing leaves no selection — the collapsed-marquee law.
  CS6's boolean return (was it inverted) has no command channel; the
  mask itself is observable.
- **`selection.deleteSaved`** `{name}` — unknown name is
  `MissingTarget`; otherwise the entry is removed.

## Deviations (recorded, not hidden)

1. **Single-document.** `toDocument` / `fromDocument` index other
   open documents; the engine is single-document, so saved selections
   are per-document state and the arguments are omitted from the
   payloads.
2. **No active selection refuses `save`.** CS6 in vector mode would
   save the element selection here; the model keeps the bitmap-mode
   marquee as the only saveable selection (the verbs are documented
   bitmap-mode operations).
3. **No boolean return.** `restoreSelection` reports whether it
   inverted; commands return deltas — read the restored mask instead.
4. **Subtract operand order is ours:** save folds the stored mask as
   the base (subtract = saved − active); restore folds the active
   selection as the base (subtract = active − saved) — the fold API's
   base/operand order everywhere else in `bitmap.*`. Mismatched plane
   sizes fall back to replace (the `mask::fold` law).

## Consequences

- Parity: the three rows flip M→C (**215 C / 45 P / 8 M / 88 N** at
  this batch's close); registry 186 → 189.
- The remaining M families after this batch: clipboard mask pastes
  (8.6/6.6, needs the clipboard surface), the `filterSelection` bake
  trio (4 rows, one a CS6-typo duplicate), and
  `setDocumentImageSize` (rides the resample pass).
- Saved selections survive undo byte-exactly and are invisible to the
  wire round-trip (never file content).
