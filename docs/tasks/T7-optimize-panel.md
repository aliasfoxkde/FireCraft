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
