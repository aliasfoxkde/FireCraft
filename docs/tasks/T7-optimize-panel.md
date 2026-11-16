# T7 — 20.2 Optimize panel

**Objective:** the CS6 Optimize panel: palette/bitrate controls over
`ExportOptions` with live preview of the export result.

**Inputs:** `doc::ExportOptions` (the `EXPv` model, incl. the
selective-JPEG quality table); the existing export preview path in the
shell panels; CS6 Optimize panel behavior (format select, palette size,
loss, dither, transparency); `.jsf` corpus scripts touching
`ExportOptions`.

**Authority:** CS6 observed behavior + dictionary-documented `EXPv`
fields only.

**Stopping condition:** panel renders, edits persist through
`ExportOptions` round trip, preview reflects edits; tests cover the
binding; wasm ceiling holds.

**Verification:** `cargo test -p app`, kittest snapshot (when task 21's
harness lands), full gate before commit.

## Status — LANDED (2026-11-11)

Decision
[0095](../decisions/0095-optimize-panel-honors-only-what-export-honors.md).

**Landed:**

- **The panel's controls are the export's honored fields.** Format
  (`FMTi`), JPEG quality and progressive scans and the matte (`JPQi`,
  `JPPb`, `FCLi`), GIF dither and loop and the animated crop and
  frame-difference knobs (`PLDi`, `FLPi`, `AACb`, `AADb`). Every knob
  reaches an encoder through `io::expv`, the same resolution the
  export path uses.
- **The preview is the export.** The panel encodes the current page
  through `io::expv` and shows the decoded result with its byte size,
  cached against the settings, the page, the frame, and the
  document's journal state.
- **Interlacing is offered where the export honors it.** The `GFIb`
  flag is written as the seven Adam7 passes (decision
  [0120](../decisions/0120-the-png-interlace-is-written-as-adam7-passes.md)):
  `crates/fwpng/src/adam7.rs` owns the pass geometry and the row
  filters, `fwpng::image::encode_rgba8_interlaced` writes the stream,
  and `io::png::encode_with` honors `PngOptions::interlaced`. The
  panel offers the knob for PNG and clears the flag for the formats
  that cannot honor it, the same rule the dither and the JPEG quality
  follow.
- **A knob the format cannot use is cleared.** Switching to JPEG
  retires a recorded GIF dither rather than leaving a setting behind
  that the JPEG path cannot honor. This needed an engine change: the
  `page.setExport` payload now distinguishes a JSON `null` (clear) from
  an absent key (keep).
- **Tests.** `crates/app/tests/optimize_panel.rs` (9) covers the
  binding end to end — the payload reaching the page's record, the
  record reaching the export resolution the encoders read, a format
  change clearing the inapplicable knobs, one edit being one undoable
  step, and the interlace flag being written for PNG and cleared for
  the formats that cannot honor it.
  `crates/app/src/shell/optimize.rs` (8) covers the settings' own
  reading and writing. `crates/engine/src/catalog/page.rs` (3 new)
  covers the animated knobs and the clear-versus-keep semantics.
  `crates/fwpng/src/adam7.rs` (4) covers the pass geometry, the
  filter arithmetic, and the row count each pass contributes.

**Open:**

- **No selective-JPEG quality table.** CS6's Optimize panel carries a
  per-component quality table; this build has one global `JPQi`.
- **The panel is unstyled relative to the rest of the dock**, and the
  Phase 22 screenshot audit has not covered it.
- **No GIF interlace.** The flag is PNG's; the GIF encoder does not
  read it, and the panel clears it rather than promising an effect it
  cannot deliver.
