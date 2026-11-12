# 0095 — The Optimize panel offers only what the export honors

Date: 2026-11-11 · Ticket: [T7](../tasks/T7-optimize-panel.md) · Status: accepted

## Context

The Optimize panel existed as a thin form over the page's `EXPv`
record: a format select, a JPEG quality slider, a progressive checkbox,
a GIF loop select, a dither slider, and an interlace checkbox. Two
defects sat under it.

1. **The interlace checkbox offered a setting the encoder refuses.**
   `GFIb` reaches `io::png::encode_with` as `PngOptions::interlaced`,
   and the `png` crate 0.18.1 writes the IHDR interlace *flag* but has
   no Adam7 pass encoder. So a page that recorded the flag could not
   export a PNG at all: `io::png::encode_with` returns
   `ExportError::Unsupported { format: "png", reason: "interlaced
   output needs an Adam7 pass encoder (not implemented)" }`. The panel
   offered the checkbox anyway, so a user could set a flag that broke
   their own export, and — because the flag was written on every
   unrelated edit — could not set *any* other option without breaking
   the export too. (GIF has the same gap: `io::expv::gif_options` reads
   no interlace field at all, so a `GFIb` on an animated page was
   silently ignored by the GIF path and fatal to the PNG path.)
2. **The panel's knobs could not reach the encoders it named.** The
   panel wrote only `format`, `jpeg_quality`, `jpeg_progressive`,
   `dither`, `frame_loop` and `interlaced`. CS6's Optimize panel also
   carries the matte (`FCLi`) and the animated-GIF crop and
   frame-difference knobs (`AACb`, `AADb`) — all three are fields the
   export resolution (`io::expv`) already honors. They were simply
   not exposed.

Underneath both defects sat a third, in the engine rather than the
shell: `page.setExport`'s payload could not express "clear this
field". Its optional fields were plain `Option<T>`, and serde reads a
JSON `null` into an `Option<T>` as `None` — indistinguishable from an
absent key. A knob the chosen format cannot use (a GIF dither on a
JPEG page) therefore could not be cleared: the panel sent `null`, the
engine treated it as "not mentioned", and the stale setting stayed in
the record forever.

## Decision

- **The panel offers a control only when the export path honors the
  field it writes.** Every knob in the Optimize panel maps to a field
  `io::expv` reads for that format: `JPQi`/`JPPb`/`FCLi` for JPEG,
  `PLDi`/`FLPi`/`AACb`/`AADb` for GIF, `FMTi` for the choice. The
  panel's shape is CS6's; the field set is this build's truth.
- **A setting the codecs cannot honor is disclosed, not offered.** The
  interlace row shows the recorded `GFIb` state as a disabled control
  with the reason in its hover text, and — when the page records the
  flag — a button that clears it through `page.setExport`. The panel
  never *writes* the flag as a side effect of another edit, so an
  unrelated setting change cannot break the export.
- **A knob the chosen format cannot use is cleared, not left behind.**
  The panel writes `null` for the inapplicable fields; the engine
  distinguishes "absent" from "null" so that `null` clears.
- **The engine's payload distinguishes absent from null.** The
  optional fields of `page.setExport`'s payload are
  `Option<Option<T>>` with a `double_option` deserializer. A JSON
  `null` clears the field; an absent key leaves it alone. This is a
  change to the engine's contract, not a panel-side convention: the
  documented semantics ("absent keeps, null clears") become real.
- **The preview is the export.** The panel's live preview encodes the
  current page through the same `io::expv` resolution the export path
  uses — `io::gif::encode_animated` for GIF, `compose::render` →
  `io::jpeg::encode` / `io::png::encode_with` for the others — and
  decodes the result back for display. What the panel shows is what
  the file gets, not a re-implementation of it. The preview is cached
  against a stamp of the settings, the page, the frame, and the
  document's journal state, so it regenerates exactly when the
  encoded bytes could have moved.

## Consequences

- The Optimize panel's control set and the export path's honored field
  set are the same set, by construction. A knob that cannot reach an
  encoder cannot be offered.
- PNG and GIF interlacing remain unimplemented (Adam7 pass encoding).
  This is recorded as open work rather than papered over: a page that
  already records the flag is offered the way out of it.
- `page.setExport` now clears fields. The engine's three existing
  `set_export_*` unit tests still hold, and three more pin the new
  semantics: the animated-GIF knobs are written, a `null` clears a
  knob the format cannot use while an absent key leaves it alone, and
  the matte and progressive flag clear the same way.
- The panel's bindings do not change the reachability measurement:
  `page.setExport` was already bound by the old panel, so the floor
  stays at **168 of 253 registry commands (66.40%)**, 85 unreachable.
- The panel moved out of `shell/panels.rs` into its own module
  `shell/optimize.rs`, matching the timeline module: the panel has
  its own settings type, its own preview cache, and its own tests.
