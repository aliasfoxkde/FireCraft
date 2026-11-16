# 0120 — The PNG interlace is written as Adam7 passes, and the Optimize panel offers it

Date: 2026-11-16 · Tickets: [T7](../tasks/T7-optimize-panel.md) ·
Status: accepted

## Context

Decision
[0095](0095-optimize-panel-honors-only-what-export-honors.md) made
the Optimize panel offer exactly the controls the export pipeline
honors. One control was refused: PNG's Adam7 interlace (`GFIb`). The
`png` crate writes the interlace *flag* in the IHDR but cannot emit
the seven passes the flag promises, so an interlaced PNG was either a
corrupt file or an error. The export path therefore refused the flag
loudly, and the panel showed the row disabled with the reason
attached.

That was the honest state of the code, and it left two things
unfinished:

- a page recorded by CS6 with the interlace flag **could not be
  exported at all** — the export refused;
- the panel showed a control CS6 offers, permanently dead.

The reason was always the encoder, not the format. Adam7 is a fixed
geometry: seven passes, each taking the pixels whose coordinates fall
on its own stride, and each row filtered with the PNG row filters.
The `png` crate's *decoder* reads interlaced PNGs; only its writer is
absent.

## Decision

- **The workspace writes the passes itself.**
  `crates/fwpng/src/adam7.rs` owns the pass geometry and the row
  filters; `fwpng::image::encode_rgba8_interlaced` writes the IHDR
  with the interlace method set to 1, the filtered pass stream, and
  the IDAT that carries it. `fwpng` stays the single PNG
  implementation — it already depends on `flate2` and `crc32fast`,
  which is everything a PNG stream needs.
- **The filter choice is the specification's heuristic.** For each row
  the five filters are scored by the sum of the absolute values of the
  filtered bytes read as signed, and the smallest score wins. The
  choice is deterministic, and the rows are filtered against the
  previous row *of the same pass*, as the specification requires.
- **The export honors the flag.** `io::png::encode_with` writes the
  interlaced stream when `PngOptions::interlaced` is set; there is no
  longer a refusal path to write a test against.
- **The flag can be cleared, like the other knobs.** `page.setExport`
  reads `GFIb` with the double-option deserializer, so a JSON `null`
  clears it and an absent key keeps it. The Optimize panel therefore
  *writes* the flag for PNG and *clears* it for the formats that
  cannot honor it — the same rule the panel already follows for the
  dither and the JPEG quality.
- **The panel offers the knob where the export honors it.** The
  interlace row appears for the PNG format and is editable. A page
  that records the flag under another format still shows the way out
  (the clear button), because the record outlives the format.

## Consequences

- A CS6 document that records interlaced output exports as an
  interlaced PNG, and the file decodes to the same pixels as the
  non-interlaced write — the round trip is covered by a test.
- The panel's contract from decision 0095 is unchanged in kind and
  stronger in fact: the panel still offers only what the export
  honors, and the export now honors one more thing.
- GIF interlace remains unsupported and remains disclosed. The flag
  is PNG's; the GIF encoder does not read it, and the panel clears it
  when the format is GIF rather than promising an effect it cannot
  deliver.
- The Optimize panel's other open item — CS6's per-component JPEG
  quality table — is untouched by this decision and still open.
