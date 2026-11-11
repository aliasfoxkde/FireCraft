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
- **Interlacing is disclosed, not offered.** The `GFIb` flag cannot be
  written by this build (no Adam7 pass encoder), so the row shows the
  recorded state disabled with the reason, and a page that records the
  flag is offered a way to clear it. The panel never writes the flag
  as a side effect.
- **A knob the format cannot use is cleared.** Switching to JPEG
  retires a recorded GIF dither rather than leaving a setting behind
  that the JPEG path cannot honor. This needed an engine change: the
  `page.setExport` payload now distinguishes a JSON `null` (clear) from
  an absent key (keep).
- **Tests.** `crates/app/tests/optimize_panel.rs` (8) covers the
  binding end to end — the payload reaching the page's record, the
  record reaching the export resolution the encoders read, a format
  change clearing the inapplicable knobs, one edit being one undoable
  step, and the interlace flag only being cleared on request.
  `crates/app/src/shell/optimize.rs` (7) covers the settings' own
  reading and writing. `crates/engine/src/catalog/page.rs` (3 new)
  covers the animated knobs and the clear-versus-keep semantics.

**Open:**

- **No Adam7 interlacing.** PNG (and GIF) interlaced output needs a
  pass encoder the `png` crate does not provide. Named work, not a
  panel defect.
- **No selective-JPEG quality table.** CS6's Optimize panel carries a
  per-component quality table; this build has one global `JPQi`.
- **The panel is unstyled relative to the rest of the dock**, and the
  Phase 22 screenshot audit has not covered it.
