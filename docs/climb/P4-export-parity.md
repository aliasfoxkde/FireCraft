# P4 — Export parity (Fireworks' core job)

**Goal:** everything a CS6 user did in the Optimize panel and the Export
dialog produces correct files. That covers:
- PNG8, PNG24 and PNG32, with palette, transparency and interlace;
- GIF, and JPEG including selective quality;
- @Nx scale and saved presets;
- HTML Setup and Update HTML;
- Batch Process from the GUI.

Two modern outputs are added: PDF, and table-free HTML/CSS.

**Exit check:**
- every Optimize format CS6 offers round-trips through `EXPv` and encodes;
- `xtask web-golden` and `xtask svg-golden` floors are not lowered;
- S5c −4 or better.

**Lane:** export. Starts after P0.

**Key files:**
- `crates/io/src/{png,gif,jpeg,webp,expv}.rs`
- `crates/doc/src/model.rs::ExportOptions` (the `EXPv` mapping; read its
  doc comment)
- `crates/app/src/shell/optimize.rs`
- `crates/webexport/`
- `docs/research/fireworks-png-nodes.md` (the node dictionary)
- decisions 0010, 0017, 0063, 0095

**Clean-room note:** which `EXPv` field values mean PNG8, PNG24 or PNG32
(`FMTi`=2 plus `MCTi`/`PLTi`, per the model's doc comment) must come from the
node dictionary and the corpus, never from the binary. Rungs tagged
`[EVIDENCE]` establish this first.

---

### C4.01 — Pin the PNG bit-depth encoding in `EXPv` `[ ]`
- **Needs:** C0.01 · **Tags:** `[EVIDENCE]` `[DOCS]` · **Size:** S
- **Steps:**
  1. Search `docs/research/fireworks-png-nodes.md` for `MCTi`, `PLTi`,
     `PALv`, `PTNi`, `XSZi`/`YSZi` and `LR*i`. Run
     `cargo run -p xtask -- corpus --report` and list the `EXPv` values seen
     in corpus files whose Optimize format is known from their file
     metadata.
  2. Write `docs/decisions/NNNN-png-bit-depth-in-expv.md`. It covers:
     - the field and value for PNG8, PNG24 and PNG32;
     - where the palette (`PALv`) lives;
     - the transparency types (`PTNi`): none, index, alpha.
  3. Mark any value that is not proven as **unpinned** — see the
     *Fallback* line below.
- **Done when:** the decision is merged, and each value cites a dictionary
  line or corpus file.
- **Verify:** review the citations.
- **Fallback:** if the bit-depth value is unproven, P4 still proceeds. The
  UI and the encoder then key off a FireCraft-side setting, which is
  persisted only in FireCraft's own unknown-bag-safe way, and the decision
  says so.
- **Moves:** unblocks C4.02–C4.06.

### C4.02 — Extract the palette quantizer into `io::palette` `[ ]`
- **Needs:** C2.01 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. `crates/io/src/gif.rs` holds `NearestPalette`, `quantize` (line 566)
     and `quantize_diff`. Move the palette-building and nearest-color code
     into a new `crates/io/src/palette.rs`, with a
     `pub(crate) fn build_palette(rgba, max_colors, …)` and
     `pub(crate) fn map_with_dither(…)`.
  2. GIF calls the new module.
  3. Change no behaviour. The GIF goldens and tests must stay byte-identical.
- **Done when:** `cargo test -p io gif` passes unchanged, and `palette.rs` has
  its own unit tests.
- **Verify:** `cargo test -p io`.
- **Moves:** S6 ↑.

### C4.03 — PNG8 encoder (indexed + tRNS) `[ ]`
- **Needs:** C4.02 · **Tags:** `[HOST:linux]` · **Size:** M
- **Steps:**
  1. In `crates/io/src/png.rs`, add `PngOptions { depth: PngDepth
     { Rgba32, Rgb24, Indexed8 { max_colors, dither, transparency } }, …}`.
  2. `Indexed8` builds the palette with `io::palette`.
  3. Output: PLTE + tRNS. Alpha transparency gives per-entry alpha. Index
     transparency gives one fully transparent entry. None gives a matte
     composite.
  4. Use the `png` crate's indexed color type, which supports PLTE and tRNS.
  5. Write tests:
     - a 2-color image gives bit depth 8 and a PLTE of 2 entries;
     - alpha survives decode;
     - `max_colors` is respected;
     - a hostile `max_colors` of 0 or >256 is a typed error.
- **Done when:** decode(encode(x)) matches within the palette for test
  images.
- **Verify:** `cargo test -p io png`.
- **Moves:** S6 ↑.

### C4.04 — PNG24 encoder `[ ]`
- **Needs:** C4.03 · **Tags:** `[HOST:linux]` · **Size:** XS
- **Steps:**
  1. `Rgb24` composites onto the matte, using the same matte logic as JPEG
     (`expv::matte_from_abgr`), and writes `ColorType::Rgb`.
  2. Test the matte composite.
- **Done when:** tests pass.
- **Verify:** `cargo test -p io png`.
- **Moves:** S6 ↑.

### C4.05 — Map `EXPv` to the PNG depth `[ ]`
- **Needs:** C4.01, C4.04 · **Tags:** `[HOST:linux]` · **Size:** S
- **Steps:**
  1. Extend `ExportOptions` with the fields that C4.01 pinned. Follow the
     doc-comment style, and remove those fields from the "unknown bag" list.
  2. Update the fwmap reader and writer. The round-trip stays byte-identical
     on the corpus, and the `xtask corpus` floor holds.
  3. Make `expv::png_options` return the depth.
- **Done when:** the corpus floor is unchanged, and a fixture with PNG8
  settings exports indexed.
- **Verify:** `cargo run -p xtask -- corpus`; `cargo test -p io expv`.
- **Moves:** S6 ↑.

### C4.06 — The Optimize panel offers PNG8, PNG24 and PNG32 `[ ]`
- **Needs:** C4.05, C2.05 · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. In `crates/app/src/shell/optimize.rs`, the format dropdown gains PNG 8,
     PNG 24 and PNG 32 (CS6's labels).
  2. PNG8 shows these controls: palette type, max colors (2–256), dither %,
     transparency (none / index / alpha), and matte.
  3. The live preview uses the same `io` path (decision 0095: "the controls
     are the fields the export honors").
  4. Show the file-size readout.
- **Done when:** the browser proof shows PNG8 at 16 colors with a visibly
  posterized preview and a smaller byte count.
- **Verify:** a Playwright screenshot plus `cargo test -p app optimize`.
- **Moves:** S5a +1 / S5b −1 (the Optimize row).

### C4.07 — Adam7 interlaced PNG `[ ]`
- **Needs:** C4.03 · **Tags:** `[HOST:linux]` · **Size:** M
- **Steps:**
  1. `png.rs` refuses interlace today, because the `png` crate can't write
     the passes.
  2. Implement an Adam7 writer:
     - split the image into the 7 passes;
     - apply the per-scanline filter per pass;
     - concatenate;
     - compress with the zlib crate already in the tree (`cargo tree -p io`).
       Do not add a C dependency.
  3. Validate by decoding with the `png` crate, whose decoder supports
     interlace, for each depth.
  4. Remove the refusal.
- **Done when:** interlaced output decodes identically to non-interlaced for
  RGBA, RGB and indexed.
- **Verify:** `cargo test -p io png::interlace`.
- **Moves:** S6 ↑; the Optimize residual (#23) shrinks.

### C4.08 — Selective JPEG quality `[ ]`
- **Needs:** C0.01 · **Tags:** `[EVIDENCE]` `[HOST:linux]` · **Size:** M
- **Steps:**
  1. CS6's selective JPEG uses a lower quality outside a saved selection
     mask.
  2. Establish where the selection and the secondary quality are stored.
     Check `docs/tasks/T7-optimize-panel.md`, the dictionary, and the
     corpus.
  3. Implement it in `io/src/jpeg.rs` as region-weighted quantization if the
     encoder allows per-block tables. If it doesn't, use a two-pass
     composite: encode the lower-quality region separately and blend the
     decoded result. Record the approach in a decision.
  4. Add the UI in the Optimize panel: "Selective quality" plus a checkbox to
     use the current saved selection.
- **Done when:** a test shows lower PSNR outside the mask and equal PSNR
  inside it.
- **Verify:** `cargo test -p io jpeg::selective`.
- **Moves:** closes the second residual of #23.

### C4.09 — Export at scale (@1x/@2x/@3x) `[ ]`
- **Needs:** C4.05 · **Tags:** `[HOST:linux]` `[HOST:browser]` · **Size:** M
- **Steps:**
  1. `ExportOptions` already notes `XSZi`/`YSZi` (scale) as preserved
     unknowns. Pin them with evidence, as in C4.01.
  2. Add a FireCraft "Export at scale" list (multiple scales emit
     `name@2x.png` and so on).
  3. Raster export renders the page through `compose` at that scale. That
     means re-rendering vectors and text, not resampling the 1x bitmap.
  4. Add UI to the Export dialog.
- **Done when:** `@2x` output has exactly double the dimensions, and vector
  edges are crisp. Compare against a 2x golden rendered by `compose`.
- **Verify:** `cargo test -p app export_scale`; a golden image.
- **Moves:** modern feature; S6 ↑.

### C4.10 — Saved export presets `[ ]`
- **Needs:** C4.06 · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. In the Optimize panel, "Save Settings…", "Delete Settings" and the
     preset dropdown store named `ExportOptions` sets:
     - desktop: in the preferences file next to `ShellLayout`;
     - web: in IndexedDB.
  2. Ship FireCraft-authored defaults matching CS6's preset *names* (GIF Web
     Snap 256, JPEG – Better Quality, PNG 8, PNG 24, PNG 32). Their values
     come from the C4.01 evidence or from FireCraft's own choice, recorded in
     a decision.
- **Done when:** presets persist across restarts.
- **Verify:** a shell test plus a browser reload check.
- **Moves:** S5.

### C4.11 — Export Area tool `[ ]`
- **Needs:** C4.06 · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. Add a tool, in CS6's Crop group (`crates/app/src/toolstrip.rs`), that
     drags a rectangle.
  2. On Enter or double-click, open Export with that region. The
     `ExportOptions` export area `LR*i` is already noted as preserved;
     pin it per C4.01's method.
  3. Escape cancels.
- **Done when:** exporting a dragged area produces the cropped image.
- **Verify:** a shell test plus a browser proof.
- **Moves:** S5c −1 (missing tool).

### C4.12 — Lossy WebP on web, or a recorded refusal `[ ]`
- **Needs:** C0.01 · **Tags:** `[DOCS]`/`[HOST:linux]` · **Size:** S
- **Steps:**
  1. Lossy WebP is native-only today (libwebp FFI, `io/src/webp_lossy.rs`).
  2. Survey pure-Rust lossy WebP encoders that are wasm-safe. If a
     maintained one exists with a permissive licence, wire it behind the web
     build.
  3. If none qualifies, write a decision explaining why web offers lossless
     only. Have the Optimize panel say so when running on web; it should not
     silently fall back.
- **Done when:** either web lossy works, or the refusal is recorded and shown.
- **Verify:** a browser proof.
- **Moves:** S5b.

### C4.13 — AVIF decode (#95) `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** M
- **Steps:**
  1. Read `docs/decisions/0064-avif-decode.md`; it states the chosen
     approach and its constraints.
  2. Implement exactly that approach, or mark the rung `[-]` if the decision
     says to defer.
  3. Add a fuzz anchor for the decoder entry point.
  4. Wire File ▸ Open and Import to the new format.
- **Done when:** an AVIF test file opens on desktop and web, or the decision
  defers it explicitly.
- **Verify:** `cargo test -p io avif`; the fuzz anchors.
- **Moves:** close #95.

### C4.14 — PDF export: decision and first slice `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:linux]` · **Size:** M
- **Steps:**
  1. Write a decision choosing a pure-Rust, wasm-safe PDF writer. Evaluate
     `pdf-writer` (MIT/Apache) and record its licence.
  2. Implement the first slice: a single page of vector paths (fills and
     strokes, solid colors) as PDF vector operators.
  3. Text, bitmaps and gradients come in C4.15.
  4. Add File ▸ Export As ▸ PDF.
- **Done when:** a rectangle and a bezier path export as vectors.
- **Verify:** check the content stream text for `re`/`c` operators;
  `cargo test -p io pdf`.
- **Moves:** modern feature.

### C4.15 — PDF export: bitmaps, text, gradients, pages `[ ]`
- **Needs:** C4.14 · **Tags:** `[HOST:linux]` · **Size:** M
- **Steps:**
  1. Embed bitmaps as image XObjects, using Flate.
  2. Convert text to outlines, via the existing convert-to-paths code from
     decision 0033. Embedding fonts is out of scope; record that.
  3. Linear and radial gradients become shading patterns. Filters and live
     effects are rasterized via `compose`.
  4. Each page becomes a PDF page.
- **Done when:** a multi-page corpus document exports, and the pages render
  in a PDF viewer. Attach screenshots.
- **Verify:** `cargo test -p io pdf`; manual viewer check.
- **Moves:** S5c −1 (the PDF row).

### C4.16 — HTML Setup dialog `[ ]`
- **Needs:** C0.01 · **Tags:** `[EVIDENCE]` `[HOST:browser]` · **Size:** M
- **Steps:**
  1. CS6's File ▸ HTML Setup sets the HTML style, extension, spacer and
     comment options, and the slice naming scheme. Establish the option
     list from the CS6 help text and the `HTML Code/` export templates
     (behavioural spec only).
  2. Map each option onto the `webexport` options. Some may already exist
     in `crates/webexport`.
  3. Persist the options in the document's HTML family under `EXPv`, which
     is already preserved.
  4. Build the dialog.
  5. Replace the menu row's `ReportGap`.
- **Done when:** changing the extension and the naming scheme changes the
  export output, with golden tests.
- **Verify:** `cargo run -p xtask -- web-golden --measure`; a shell test.
- **Moves:** S3 −1, S5c −1.

### C4.17 — Update HTML `[ ]`
- **Needs:** C4.16 · **Tags:** `[EVIDENCE]` `[HOST:linux]` · **Size:** M
- **Steps:**
  1. CS6 rewrites the FireCraft-generated region of an existing HTML file
     between its marker comments.
  2. Establish the marker format from the `HTML Code/` templates
     (behavioural).
  3. Implement `webexport::update_html(existing, export) ->
     Result<String, _>`. It replaces only the marked regions and refuses
     when the markers are missing.
  4. Desktop: pick a file. Web: pick the file through C3.15 or upload it,
     then download the result.
- **Done when:** a golden test leaves bytes outside the markers untouched.
- **Verify:** `cargo test -p webexport update_html`.
- **Moves:** S3 −1, S5c −1.

### C4.18 — Table-free HTML/CSS export mode `[ ]`
- **Needs:** C4.16 · **Tags:** `[HOST:linux]` · **Size:** M
- **Steps:**
  1. Add an HTML Setup style "HTML5 + CSS (absolute)", and a "flex"
     variant if the slice layout allows it.
  2. Emit semantic `<div>`s with CSS positioning and no spacer GIFs.
  3. Keep the CS6 table style as the default, for parity.
  4. Add web-golden fixtures for both styles.
- **Done when:** the golden tests pass, and the output validates as HTML5
  (`npx html-validate`).
- **Verify:** `cargo run -p xtask -- web-golden --measure`.
- **Moves:** modern feature.

### C4.19 — Batch Process wizard (GUI) `[ ]`
- **Needs:** C4.10 · **Tags:** `[HOST:browser]` · **Size:** M
- **Steps:**
  1. The CLI already batches (`crates/app-cli`). Reuse its pipeline
     functions; move them to a shared crate or module if needed, keeping to
     the layer rules.
  2. The wizard collects:
     - the files (desktop: a file list; web: a multi-select input or a
       directory handle);
     - the actions: export with a preset, scale, find/replace, rename,
       commands (`.jsf`);
     - the output location (web: a zip download).
  3. Use pure-Rust zip writing, wasm-safe.
- **Done when:** 3 files batch-export to PNG8 in one run, on desktop and web.
- **Verify:** a shell test plus a browser proof.
- **Moves:** S3 −1, S5c −1.

### C4.20 — Animated GIF document loop count `[ ]`
- **Needs:** C0.01 · **Tags:** `[HOST:browser]` · **Size:** S
- **Steps:**
  1. `ExportOptions.frame_loop` (`FLPi`) already exists. Expose it in the
     States panel and the Optimize panel as CS6 does (loop: forever /
     none / N times).
  2. Make sure the GIF encoder honours it; `expv::gif_options` reads it.
- **Done when:** the exported GIF's NETSCAPE2.0 loop field matches the UI.
- **Verify:** a test decoding the loop extension.
- **Moves:** shrinks the #24 residual.
