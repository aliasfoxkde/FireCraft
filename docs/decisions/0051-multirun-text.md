# Decision 0051 — multi-run text model + markup verbs (P rows → C)

**CS6 surface.** The five verbs the single-run model parked:

- `dom.setTextRuns(textRuns)` — a `TextRuns` object:
  `initialAttrs` (a `TextAttrs` set) plus `textRuns`, an array of
  `SingleTextRun { changedAttrs, characters }` — the dom's own
  statement of the marker-stream model.
- `dom.applyCharacterMarkup(tag)` — "b", "i", "u", and (FW4)
  "fwplain" for "text with no character markup".
- `dom.removeCharacterMarkup(tag)` — "b", "i", "u"; "Reapplies the
  default value for the specified markup type."
- `dom.applyFontMarkup(fontAttribute, value)` — "size" (value must be
  a point size like "14pt"; "a simple numeric value is not allowed")
  and "face".
- `dom.removeFontMarkup(fontAttribute)` — "size", "color", "face";
  again "reapplies the default value".

## Design

- **Per-run styling rides the documented `TFSv` marker stream — the
  model catches up to the wire, not the other way round.** The
  dictionary's `TFSv` is "a list of formatting operations and text
  runs": style nodes between `TRNs` runs are the changes that take
  effect at the next run (`UNDb` true begins underline, false ends
  it; every other style node restates its field). `TextRun` gains
  `changes: TextStyle` — the fields that run's group stated — and the
  dom triple maps exactly: `initialAttrs` is the base scalars,
  `changedAttrs` is the run's group, `characters` is the `TRNs`
  payload.
- **The reader folds the stream; the writer re-diffs it.** Markers
  before the first run extend the base; markers between runs
  accumulate into the next run's `changes`; markers after the last
  run have no run to attach to and bag verbatim at their original
  child index, so the write splices them back in place byte-exact.
  The writer emits each run's group (dictionary order) before its
  `TRNs`. Files without divergence — every corpus sample and every
  prior fixture — emit the same bytes as before (a two-cycle
  byte-stability test pins the divergent shape).
- **Delta groups restate, never unset.** "Inherit" is the absence of
  a node, so a mid-stream unstate has no wire form; the restyle
  machinery re-diffs each run's effective style against the previous
  run's and emits only departures (`style_delta`), while
  `overlay_changes` resolves inheritance for reading. Whole-style
  unsetting lives where CS6 puts it: `removeFontMarkup` drops the
  field from the base, and the runs' groups re-diff to empty.
- **The markup verbs restyle every run's effective style.** The dom
  addresses "the selected text"; with no character selection in the
  model, the object's whole run list restyles — `applyCharacterMarkup`
  sets the tag true on each run's effective style, `remove…` sets it
  false, `applyFontMarkup` sets size/face. `fwplain` is the one verb
  that touches the base: it clears bold/italic/underline there too
  ("no character markup" is the state, not an override). The canonical
  form keeps groups minimal: a run whose effective equals the previous
  run's emits an empty group (inheritance, not restatement).
- **`removeFontMarkup`'s default is the unset value.** The dom says
  "reapplies the default value" without naming one; the recorded
  choice is unset (the renderer's own fallback face/size/black). The
  base drops the field and every run's effective drops it with it.
- **`applyFontMarkup` enforces the dom's size grammar.** The value
  must parse as a point size with the `pt` suffix (finite, positive);
  a bare number refuses — the doc's "not allowed" is a refusal, not a
  coercion. `face` takes a non-empty family name.
- **`setTextRuns` replaces the structure.** Stated `initialAttrs`
  fields write into the base (unstated keep current); the run list
  replaces every run, each group becoming its `changes`. An empty run
  list clears the text (the `text.setText` empty-string precedent).
  All gates (sizes positive, enums in range, finites) run before the
  first mutation (000557d).
- **Layout ignores run groups for now** — the same recorded stage the
  paragraph attrs sit in: the model and wire carry divergence
  faithfully; the text crate's shaping reads the base style until
  per-run shaping lands (no wire risk; a recorded rendering
  limitation).

## Consequences

- Registry 230 → **235**; parity 258 C / 13 P / 0 M / 85 N → **263 C
  / 8 P / 0 M / 85 N** (five flips — the recorded M15 text gap
  closes).
- `TextRun` gained `changes: TextStyle` (`serde(default)` — old
  journals decode); `doc::fwmap` gained the fold
  (`apply_style_node`, marker accumulation, trailing-marker bagging)
  and `push_style_nodes` (shared base/group emission). `text.rs`
  gained `overlay_changes`/`style_delta`/`restyle_runs` and the
  `TextAttrsPayload` attribute gate shared by `setTextRuns`.
- Remaining P rows: 8 — the anti-alias trio (renderer-side),
  `importFile` (IO placement), `setFillNColorNTexture`,
  `duplicateSymbolForAlias`, and the two permanent rows
  (`setAnimInstanceStartFrame`, `setSliceAutonaming`).
