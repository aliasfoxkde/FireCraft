# Decision 0050 — paragraph attribute setters (P rows → C)

**CS6 surface.** Three dom verbs, all Fireworks MX, each addressing the
selected text:

- `dom.setTextParaIndent(paraIndent)` — "The number of pixels by
  which to indent the first line of a paragraph."
- `dom.setTextParaSpacingBefore(paraSpaceBefore)` — "The number of
  pixels to move down before starting a new paragraph."
- `dom.setTextParaSpacingAfter(paraSpaceAfter)` — "The number of
  pixels to move down before starting the next paragraph." The dom
  further records that a before on the second paragraph and an after
  on the first **sum** at layout time.

## Design

- **The wire nodes already map.** `PINf`/`PSBf`/`PSAf` sit in the
  `TFSv` style set (reader folds them into
  `TextStyle::{paragraph_indent, space_before, space_after}`; the
  writer emits them in dictionary order) and again as `TXTv`-level
  mirrors — the mirrors are style copies and stay bagged verbatim,
  exactly like the `FONs`/`PTSf` copies. These verbs are setters over
  existing mapped state: model debt only, no new wire shape.
- **Id-addressed, not selection-walked.** CS6's "the selected text"
  resolves to an object id at the shell; the registry command takes
  the id directly (`text.setAttrs`/`text.setFont` precedent).
  `MissingTarget` for a stale id or a non-text object, before any
  mutation (000557d).
- **The gate is finiteness only.** The dom types the arguments as
  pixels and records no range: negative indent/spacing is expressible
  (an outdent; overlapped paragraphs), zero is the meaningful clear,
  and NaN/inf would poison the `f` node — refused. This is the
  `setTextOnPathOffset` gate, not the `moveSelectionMaskBy` one.
- **One style set.** Paragraph attributes live on the shared
  `TextStyle`, so the verbs style the whole object — CS6's
  per-paragraph application needs a character selection, which the
  model does not carry yet (the multi-run gap recorded below).
  Writing materializes the default style on a style-less object
  (`style_of`), as every attribute command does.
- **Undo rides `ObjectInverse::Mutated`** via the shared `mutate_text`
  capture — the whole `TextObject` (runs included) round-trips.

## Consequences

- Registry 227 → **230**; parity 255 C / 16 P / 0 M / 85 N → **258 C
  / 13 P / 0 M / 85 N** (three flips).
- Remaining P rows: 13. The markup quartet + `setTextRuns` (5 rows)
  need per-run styles: the documented `TFSv` shape encodes run
  divergence as begin/end marker pairs (`UNDb` begin/end et al.),
  which the reader currently folds last-write-wins into the shared
  style — folding it into per-run effective styles is the multi-run
  model work. Then the anti-alias trio, `importFile`,
  `setFillNColorNTexture`, `duplicateSymbolForAlias`, and the two
  permanent rows.
- The sum-at-layout rule (before+after) is a renderer concern; layout
  ignores the three fields until text layout consumes them (the
  `TFSv` round-trips regardless).
