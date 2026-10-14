# Decision 0040 — Filter bake family (`filterSelection`)

**Status:** accepted (2026-10-14)
**Parity rows:** `filterSelection`, `filterSelectionByID`,
`filterSelectionByName`, `filterSelectonByName` (the CS6 manual's own
usage-line typo — both spellings resolve to the same carrier; the
matrix carries both rows) → **C**. New commands:
`filter.selection`, `filter.selectionByID`, `filter.selectionByName`.

## Context

CS6's three bake verbs apply a pixel filter to the selection as a
permanent action, "not as a Live Effect" (Extending Fireworks, p. 80):

- `dom.filterSelection(LiveEffect)` — FW3. Takes a full Effect object
  (the manual's example passes `{EffectMoaID: "{3439b08d-…}",
  hls_colorize:true, hue_amount:30, …}` — the Hue/Saturation GUID
  plus scalar params). "Items that are not images are converted into
  images before the filter is applied."
- `dom.filterSelectionByID(ID)` — FW8. `ID` is the filter's
  `EffectMoaID`.
- `dom.filterSelectionByName(category, name)` / the `filterSelectonByName`
  spelling — categories and names "depend on which filters you have
  installed".

FireCraft's kernel carrier already exists: `compose::plan(&[Effect])`
resolves an effect identity (normalized `EffectMoaID`, or internal
name fallback) into `KernelOp`s, and `compose::apply_stack` runs them
on a `filters::Surface` — the same interpreter the Live Effect stack
and the flatten bakes use. The bake verbs are the permanent-action
front door to that machinery.

## Design

One shared core, three identity front doors, in a new
`engine::catalog::bake` module:

- `filter.selection` payload: `{name, category?, params?}` — the same
  effect shape `filter.add` takes, validated by the same
  `param_value` encoding rules (text xor knot list).
- `filter.selectionByID` payload: `{id, params?}` — the MoaID;
  braces and case normalized before matching (compose's own rule).
- `filter.selectionByName` payload: `{category, name, params?}` —
  resolved through a category/name → GUID table built from the
  dictionary's `EffectMoaID` listing (names as the dictionary spells
  them) under CS6's Filter-menu categories: Adjust Color, Blur, Noise,
  Sharpen, Other. Unknown category/name refuses — never guessed.

Bake semantics per selected image:

1. Assemble the image's tiles to premultiplied RGBA
   (`bitmap::assemble_rgba`), wrap as `filters::Surface`.
2. `compose::plan(&[effect])` must yield at least one op and skip
   nothing (`skipped == 0`) — an unrecognized identity or missing
   parameter refuses; the no-passthrough floor law holds.
3. `compose::apply_stack` transforms the surface.
4. A `document.selection` mask (the active pixel selection) gates the
   result per pixel in image-local sampling of the canvas-space mask;
   outside the mask plane the original pixel stands. The blend runs in
   the premultiplied domain the tiles already store.
5. `bitmap::retile_rgba` mints fresh tiles; the slot swaps.

Ordering law (000557d): all target resolution and validation happens
before any mutation — object resolution, Image-kind checks, plan
validation, and tile assembly (read-only) all precede the first
`retile`. Deltas: `Mutated` per baked image (whole-object capture →
byte-exact undo), then one `Bank` for the newly minted tile ids,
wrapped in a `Group`. Empty object selection refuses
(`InvalidState`), as `clipboard.copy` does.

## Deviations (recorded)

- **Non-image members refuse whole-command.** CS6 converts paths and
  text to images before filtering; that rasterizer hook is the same
  recorded debt as `mask_type "image"` on vector clipboard contents.
  A mixed selection refuses untouched.
- **Out-of-bounds effects clip to the image surface.** CS6 dialogs can
  grow the object bounds for drop shadow and friends; the bake applies
  within the image's own extent — what `apply_stack` produces is the
  defined result. Recorded as debt.
- **No dialog.** `filterSelectionByID`/`ByName` "always displays a
  dialog box" in CS6; the engine is headless — parameters arrive in
  the payload, defaults per kernel otherwise.
- **`filterSelectonByName`** (the manual's typo) rides the same
  command; no separate registration.

## Consequences

Parity: four M rows flip to C (**225 C / 45 P / 1 M / 85 N** at this
batch's close — only `setDocumentImageSize` remains, which rides the
6.4 resample work). Registry 195 → 198.
