# Decision 0045 — `text.setTextRectangle*` (P rows → C)

**CS6 surface.** Three dom verbs over the text frame:

- `dom.setTextRectangle(boundingRectangle)` — "Sets the rectangle
  that contains text in the selected text object. The text is not
  scaled or transformed; it simply re-flows."
- `dom.setTextRectangleAuto()` — no argument form: shrink/grow the
  frame to the smallest box that holds the text.
- `dom.setTextRectangleAutoFromPoint(anchorPoint)` — auto-fit where
  `anchorPoint` stays pinned. CS6 documents the anchor semantics
  through the object's justification: a left-justified block keeps
  its left edge at the point, a centered block keeps its midpoint
  there.

## Design

- **`setTextRectangle` is a frame write, nothing else.** The dom
  promise "not scaled or transformed" is honored by leaving
  `transform` and the runs untouched — re-flow is a layout-time
  consequence (the text crate wraps at `frame.width()` on every
  measure), so the command moves only the `LFTf/TOPf/RITf/BOTf`
  corners. Gates: all four edges finite, positive extents.
  Non-finite edges cannot survive the JSON surface (serde_json
  serializes them to `null`, which fails payload decode first), so
  the finite gate is defense-in-depth exercised by the model, not by
  JSON tests.
- **Auto-fit measures with a new `TextEngine::auto_block`.** The
  engine lays the runs out unconstrained (`layout(...,
  unconstrained: true)` — no frame width, no wrap), then reduces the
  widest glyph advance (kerned, horizontally scaled) and the deepest
  `line_top + line_height`. That is the smallest box the text
  actually needs — the same number the CS6 auto-fit lands on for
  single-style runs.
- **Auto keeps the frame's top-left corner** (`setTextRectangleAuto`);
  **`AutoFromPoint` re-anchors by justification**: left (0) and
  default keep `x0 = anchor.x`; center (1) keeps the horizontal
  midpoint at the anchor (`x0 = anchor.x − width/2`); right (2) is
  the documented mirror (`x1 = anchor.x`) — not in the dom docs, but
  the symmetric completion of the documented pair, recorded here.
  `y0 = anchor.y` in every mode (top-anchored; baseline math is a
  render concern).
- **`ATEb` (auto-enlarge) is untouched.** The live auto-enlarge flag
  makes the frame follow edits continuously in CS6; these verbs are
  one-shot fits. Turning the flag on is `text.setAttrs
  auto_enlarge` — conflating them would let a one-shot command
  change persistent mode. Recorded as a deliberate non-equivalence.
- **Selection-driven like `text.convertToPaths`:** exactly one
  selected object, it must be text; two selected, a path, or an
  empty selection refuses before any capture (000557d).
- **`fit_frame` is the shared tail:** measure → gate (no extent →
  refusal "lays out to no box", i.e. empty runs) → capture → write
  `slot.frame` via the command's placement closure → `Mutated`
  delta. The inverse is the byte-exact prior object.

## Consequences

- Registry 212 → **215**; parity 239 C / 32 P / 0 M / 85 N → **242 C
  / 29 P / 0 M / 85 N** (three flips).
- The text crate gained `layout(unconstrained)` (used by the existing
  layout when `false`) and `auto_block`, both infallible and
  allocation-light; no glyph rasterization is involved.
- Remaining P rows: 29. The next text cluster (markup/para/run verbs)
  needs the multi-run text model before it can land.
