# Decision 0052 — custom anti-alias dial setters (P rows → C)

**CS6 surface.** Three `Text` dom setters for custom anti-alias
tuning, all live in CS6's Anti-Aliasing panel when the object's
anti-alias mode is "custom":

- `dom.setTextCustomAntiAliasOverSample(overSample)` — the
  oversampling factor. CS6's UI offers 4, 8, and 16.
- `dom.setTextCustomAntiAliasSharpness(sharpness)` — the sharpness
  dial.
- `dom.setTextCustomAntiAliasStrength(strength)` — the strength dial.

## Design

- **Wire first: the fields already had nodes.** The dictionary
  records `TAAi` (anti-alias mode) plus three integer companions the
  reader bagged verbatim: `TA1i` (sharpness), `TA2i` (strength),
  `TOSi` (oversample). `TextObject` gains the three `Option<i64>`
  fields; the reader promotes them from the bag to typed slots, the
  writer emits each only when `Some`, immediately after `TAAi`.
  Marker-free files round-trip byte-exact; files carrying the nodes
  stop silently dropping them (the M13 missing-field debt this
  batch closes).
- **Oversample gates to the UI's set.** The panel offers 4, 8, and
  16, so the setter refuses anything else
  (`InvalidState`: "overSample N outside {4, 8, 16}"). A coerced
  out-of-set value would write a node CS6's own UI cannot produce.
- **Sharpness and strength gate to 0..=255.** Both dials are
  8-bit sliders in the panel; negative and >255 refuse.
  `aa_dial_setter!` generates both commands from one body — same
  gate shape, same write path, distinct field names
  (`sharpness`/`strength` payload fields, `anti_alias_sharpness`/
  `anti_alias_strength` model fields).
- **Unset stays unset.** `Option<i64>` with no default: an object
  written by CS6 without the nodes keeps none; a setter call writes
  the value it was given. The renderer reads the mode (`TAAi`) as
  before and consumes the dials when the per-dial shaping lands —
  recorded rendering limitation, same stage as the paragraph attrs
  and run groups.

## Consequences

- Registry 235 → **238**; parity 263 C / 8 P / 0 M / 85 N → **266 C
  / 5 P / 0 M / 85 N** (three flips).
- `doc::model::TextObject` gained `anti_alias_sharpness`/`_strength`/
  `_oversample` (`TA1i`/`TA2i`/`TOSi`, typed out of the bag);
  `doc::fwmap` reader/writer carry all three.
- Remaining P rows: 5 — `importFile` (IO placement),
  `setFillNColorNTexture`, `duplicateSymbolForAlias`, and the two
  permanent rows (`setAnimInstanceStartFrame`, `setSliceAutonaming`).
