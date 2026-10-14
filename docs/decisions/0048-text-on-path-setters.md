# Decision 0048 — text-on-path setters (P rows → C)

**CS6 surface.** Three dom verbs, all Fireworks 3, all operating on
"the selected text-on-a-path items" (plural):

- `dom.setTextOnPathMode(mode)` — "Determines how the selected
  text-on-a-path items are displayed." `mode` is one of "rotate",
  "vertical", "skew vertical", "skew horizontal" — the `TAMi` words.
- `dom.setTextOnPathOffset(offset)` — "Sets the offset value between
  the items in the selected text-on-a-path", a float in pixels —
  `CHOf`.
- `dom.reversePathTextDirection()` — "For all text-on-a-path items in
  the selection, it reverses the direction of the text along the
  path."

## Design

- **The model already carries everything.** `TextOnPath` holds
  `alignment: Option<i64>` (`TAMi`) and `offset: f64` (`CHOf`) next
  to the guide contours (`PTHv`/`PBLv`); `text.attachPath` has
  accepted both since decision 0013. These verbs are setters over
  that existing state — no new wire shape, so the P rows were model
  debt, not format debt.
- **A shared selection walk.** CS6 addresses the selection, plural,
  so all three walk `object_selection`: text items carrying a guide
  participate; non-text, guide-less, and stale members skip (the
  setEffectName walker's rule); only items that actually change
  produce deltas, so a sweep that changes nothing is `Delta::Noop`.
  The empty selection refuses ("nothing is selected"). `edit`
  closures compare-before-write, so re-applying the current state is
  the no-op.
- **`setTextOnPathMode` gates `TAMi` to 0..=3** before the walk (the
  same range `attachPath` validates); the mode words map to integers
  at the UI layer, as the wire value is the integer.
- **`setTextOnPathOffset` gates finiteness.** Negative offsets are
  legal (leading glyphs fall off the rail and drop — the same
  contract `attachPath` documents).
- **`reversePathTextDirection` reverses the rail, not a flag.** The
  text lays along contour order, so the direction of travel is the
  point order itself: each contour's points reverse and each point's
  handles swap (incoming becomes outgoing), which preserves the
  curve exactly while flipping traversal. `is_curve` and `closed`
  are direction-neutral and stay. The `CHOf` offset is untouched —
  CS6 documents no compensation, and the reversed rail's start is
  where the text now begins (that IS the effect).
- **A guide with no contours skips** (nothing to reverse); the
  renderer would draw no glyphs on it either way.

## Consequences

- Registry 222 → **225**; parity 250 C / 21 P / 0 M / 85 N → **253 C
  / 18 P / 0 M / 85 N** (three flips).
- `text.rs` gained the shared `on_path_selection_walk`; the
  `Command` trait import in the test module (the trio's tests use
  `apply_json` directly like filter/web tests).
- Remaining P rows: 18. The dominant cluster is the multi-run text
  model (apply/remove character & font markup, para indent/spacings,
  `setTextRuns` — 9 rows, the recorded M15 gap); then the anti-alias
  trio (renderer-side custom-AA tuning), `importFile` (IO-placement
  design), and the singles (`duplicateSymbolForAlias`,
  `moveSelectionMaskBy`, `selectAdjustPixelSel`,
  `setFillNColorNTexture`).
