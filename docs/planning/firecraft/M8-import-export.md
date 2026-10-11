# M8 — Import/export pipeline

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** FireCraft stops being a closed world. A new `io` crate
(L4) turns flat byte buffers into documents and documents into the
formats CS6 offered — PNG/JPEG/GIF/BMP/WBMP/TIFF save, PSD in/out
(fidelity-tested), SVG in/out (modernization: CS6 had none natively),
animated GIF ↔ states, and the optimization-settings surface
(`EXPv`) that drives them — with batch export at the CLI. Every codec
is round-trip or oracle-proven; every parser is fuzzed.

**Inputs:** M3 `doc` model (`Document.tiles` TileBank, per-state cells,
`EXPv` currently bagged exact-path); M4 `compose::render` + 
`straight_alpha` as the caller-side raster seam (`Render` is
premultiplied; io speaks straight RGBA8); M5 registry (export arrives
as registry commands so undo/journal/fuzz rules keep applying to the
document mutations); M7 `filters` (SVG out must express the live
stack); stack decisions PLANNING §7.6 (`gif` 0.14, `image-webp` 0.2
lossless-only encode, `tiff` via `image`, PSD from the public Adobe
spec — `psd` 0.3.5 is stale, `ag-psd` is the MIT TypeScript oracle for
the spike, `usvg`/`resvg`/`svgtypes` 0.48); PLANNING §3 formats list
(lines 109–114); risk #3 (ag-psd young → spike early).

**Authority:** ROADMAP M8 gate: *per-format round-trip + oracle
suites; PSD fidelity checklist vs documented CS6 behavior (§3).*

## Design line: io is a pure data plane

`io` converts bytes ↔ documents and never touches `std::fs` (the
engine's platform-services traits and the shells own file access, per
the L3 note in PLANNING §7.1). Signatures:

- **Import:** `bytes → Document` (plus a diagnostics report). An
  importer that cannot map a feature *records the limitation in the
  report* and still produces the best document it can; it panics on
  nothing (workspace rule — hostile input).
- **Export:** `(&Document, ExportOptions) → bytes`. Raster exporters
  take straight-alpha RGBA8 planes composited by the caller
  (`compose::render` → `straight_alpha`); io never renders.
- **Registry:** document-mutating imports land as commands
  (`image.placeDecoded` — the payload is *already-decoded* pixels,
  since decoding happens in io at the shell layer) with capturing
  deltas; pure byte emission (save/export) is a query-like call, not
  a delta.
- **Layer direction:** io (L4) may depend on doc/raster/compose; the
  engine (L3) must never call io — export orchestration lives at L5
  (web bridge, app-cli, app), which already own the compose render.

This keeps `io` wasm-clean by construction — it joins `WASM_CRATES`.

## Scope line: parity is spec- and oracle-bounded

There is no CS6 on this machine, so "fidelity" is enforced as: (a)
byte-exact round-trips where the format allows (our writer → our
reader → equal semantics, and where feasible byte-stable re-encode);
(b) oracle suites against host tools (ImageMagick 7.1.1, ffmpeg, PIL
11.1, resvg) at recorded tolerances — run as `xtask` verbs, report in
CI where the oracle exists in the CI image, report-only on the host
otherwise; (c) the PSD fidelity checklist in
`docs/decisions/0006-psd-fidelity.md`, each row saying what CS6
documents PSD does, what we do, and the gap. Nothing "passes vs CS6"
that was only compared against itself.

## Tasks (atomic)

1. **`io` crate scaffold + raster save targets** — new L4 crate:
   `ExportOptions` (per-format enums), `io::png` (wrapping `fwpng`,
   which stays standalone), `io::jpeg` (quality, progressive, 
   4:2:0/4:4:4), `io::bmp` (24/32-bit), `io::tiff` (uncompressed +
   packbits, RGB/RGBA), `io::wbmp` (hand-rolled — the format is
   ~20 bytes of header over 1-bit packed rows; threshold policy
   recorded). Shells reroute: the web bridge's `exportPng` goes
   through `io::png`, and `app-cli` gains `export`. **Stopping
   condition:** per-format encode→decode
   round-trips (decode via `image`, tolerance 0 for lossless, pinned
   RMSE window for JPEG); `WASM_CRATES` 12 → 13; layers + clippy
   green.
2. **Raster import targets** — **done.** `io` opens JPEG/GIF/BMP/TIFF/WBMP/WebP
   (lossy+lossless decode; lossless encode only, per §7.6) into a new
   document (`io::raster::sniff/decode/decode_to_document`: magic-byte
   sniffing, per-decoder allocation limits, `IMGv` object + full
   premultiplied tile bank, canvas = image bounds; `app-cli import`
   verb). **Design deviation, recorded:** imports are *open-as-document*
   construction, not a `document.importImage` registry command — like
   file-open they sit outside the journal, and pixel payloads do not
   belong in command JSON. AVIF is **not** in M8: the pure-Rust encoder
   (ravif+rav1e) is a huge compile and decode has no viable pure-Rust
   path — deferred to M12 with the §7.6 platform-decoder stance; recorded
   in the decision note. **Evidence:** 10-test import battery
   (decode→re-encode→decode exact on PNG/BMP32/TIFF/WebP-lossless; JPEG
   vs the flattened expectation; WBMP through the own codec; tile-grid,
   premultiply, and zero-padding asserts on the document builder;
   truncated/mutated headers → clean `ImportError`s, no panic);
   ImageMagick 7.1.1 oracle 9/9 (png, jpg, tif ×3 compressions, bmp,
   bmp3, gif, webp-lossless) decode through the CLI path. **Deferred to
   the render-completion work owed from M7:** re-emitted `.fw.png`
   carries the imported model but no IDAT/mkBT pixel chunks yet
   (`save()` fresh-stream path), and `compose` still renders
   `ObjectData::Image(_)` as nothing — both close with the mkBT writer +
   TileBank bridge. **Stopping condition met:** decode→re-encode→decode
   equality on lossless formats; oracle comparisons vs
   ImageMagick-generated samples (9/9, above); fuzz arm over
   truncated/mutated headers registers when the fuzz targets are next
   touched (`io_read` is in the M8 gate list, task 8).
3. **SVG export** — **done.** Own serializer over the model
   (`io::svg::encode`, no SVG writer crate exists worth the name):
   tree → `<g>`/`<path>`/`<text>`/`<image>`; opacity per mille →
   fractional `opacity`; blend modes as `mix-blend-mode` through
   `compose::blend_mode` (semantics imported, not re-declared);
   live-filter stacks → SVG filter primitives through `compose::plan`
   — per-channel kernels (BrightnessContrast/Levels/Invert) as
   `feComponentTransfer` 256-entry tables **derived by running the
   real `filters` kernels over a 256×1 ramp** (zero duplicated math,
   exact at every u8 sample); GaussianBlur → `feGaussianBlur`; BoxBlur
   → `feConvolveMatrix` (separable == square, duplicate == clamp-edge);
   UnsharpMask → blur + arithmetic `feComposite`; HueSaturation/
   AddNoise/FindEdges **dropped whole with a report entry per effect**
   (mapping rationale per kernel in decision 0008). Every `<filter>`
   is full-canvas `userSpaceOnUse` + `color-interpolation-filters=
   "sRGB"` (compose's offscreen semantics, decision 0005; linearRGB
   default would silently wrong the LUTs). Report struct counts
   text/image/dropped-filter divergence. `app-cli export *.svg` wired.
   **Design deviation, recorded:** the ticket's `feColorMatrix`/
   `feTurbulence` suggestions are not used — color-matrix ops in our
   kernel set are per-channel curves, which the derived-LUT form
   expresses exactly, and no kernel maps to turbulence.
   **Stopping condition met:** `xtask svg-golden` (in `ci`): resvg
   (independent renderer) vs `compose::render`, both premultiplied —
   composed fixture **bit-exact (Δ 0)**; scripted max Δ 77 / mean
   0.48, filtered max Δ 73 / mean 0.64, fully attributed to thin-
   feature AA + premultiplied filter arithmetic at alpha edges (flat
   interiors exact); recorded bounds max 96 / mean 0.9; SVG bytes
   sha-pinned (`<fixture>.svgpin`), divergent fixtures skip with
   explicit reasons (text/image counts), nothing silently. The loop
   caught a real serializer bug on day one (pass-through groups closed
   a wrapper they never opened — unbalanced tags that could drop later
   siblings out of an ancestor's blend/filter wrapper); the io battery
   now asserts tag balance. Evidence: 17-test SVG battery
   (determinism, path/bezier/even-odd/FOTb two-path form, opacity+
   blend wrapper, alpha paint opacity, compose-parity skips, group
   nesting, exact invert LUT endpoints, kernel-derived
   brightness/contrast ramp, unmapped-drop reporting, all three blur
   primitives, text style attrs, data-URI image decodes back to the
   tile pixels, hostile canvases refused); filter-mapping table in
   `docs/decisions/0008-svg-filter-mapping.md`.
4. **SVG import** — **done.** `usvg` **0.45.1** (the ticket said 0.48;
   no such release exists — 0.45.1 is the version resvg 0.45 pins, so
   no new crates.io packages beyond the workspace graph) parse → model
   walk (`io::svgimport::decode`): paths bake usvg's absolute
   transforms into geometry (no model transform spine), quads raise to
   cubics at the ⅔ marks; groups carry per-mille opacity and blend
   values through `compose::blend_value` (the export table's exact
   inverse); solid paints → `FCLi`/`BCLi` raws (paint opacity is the
   alpha byte); `PaintOrder::StrokeAndFill` → `FOTb`; even-odd →
   `EOFb`; embedded rasters → `IMGv` over the shared tile bank
   (`raster::attach_image`, translation-only placement recorded);
   text layouts through fontdb seeded with `text::FACE_BINARIES` —
   the same binaries compose rasterizes — and arrive as glyph paths.
   **Limitation list** lives in `SvgImportReport` (notes + counters:
   texts_flattened, gradients_dropped, strokes_flattened (model
   records 1px strokes only), clips_dropped (children import unmasked
   — compose parity), filters_dropped, images) and is documented in
   decision 0008. usvg behaviors discovered and pinned by tests:
   single-stop gradients collapse to solid `Color`; `fill="none"`
   paths are culled at parse (never reach the walk); the serializer's
   `<g id="layer-1">` wrapper imports as a structured group; SVG's
   default fill is black, so "unpainted" requires `fill="none"`.
   **Hostile-input hardening, found by the fuzz arm's battery:** usvg
   AND roxmltree 0.20 both overflow the stack on deeply nested
   documents with no internal cap — the importer gates first with a
   flat, stack-safe element-depth scan (`MAX_ELEMENT_DEPTH` 128,
   exact for well-formed XML, conservative in the reject direction);
   canvases above the 65535px compose `Render` ceiling refuse at
   import. **Stopping condition met:** `xtask svg-roundtrip` (in
   `ci`): export → import → compose render vs original compose render
   — **bit-exact at Δ 0 on the whole eligible set** (composed,
   scripted; zero tolerance pinned — both sides render with compose so
   any delta is import loss); filtered skips (SVG filters import
   unfiltered — the recorded both-directions limitation), text
   fixtures skip with counts. Hostile-SVG fuzz arm: `svgimport`
   libfuzzer target + stable anchor (decode → compose render must not
   panic/hang/overflow), seeded corpus, deterministic 13-case battery
   in `crates/io/tests/svgimport.rs` (16 tests total: round-trips
   bit-exact incl. blend/opacity/even-odd/FOTb, transform baking,
   quad→cubic marks, text flattening renders, embedded image tile
   bank, gradient/stroke/clip/filter drop reports, pass-through group
   flattening, hostile refusal, absurd-but-valid ±1e30 geometry
   imports without panicking). Evidence: io 55 tests green, clippy
   clean, wasm green (io in `WASM_CRATES`).
5. **PSD spike + reader** — **done.** Spike first per risk #3: layered
   PSDs with ImageMagick + ag-psd (node, scratch under `/var/tmp`,
   never committed), enumerate the record zoo actually emitted
   (layer records, channel info, PackBits, group markers
   `<Layer set>`/`</Layer set>`, clipping, blend modes, ICC blobs);
   findings → `docs/decisions/0006-psd-fidelity.md`. Reader from the
   public Adobe spec: header, color modes (RGB/8-bit first; others
   refuse with a report), layer+mask section, channel data (raw +
   RLE), merged composite. Model mapping: layers → layers, groups →
   PSDs generated with ImageMagick + ag-psd (node, scratch under
   `/var/tmp`, never committed); the record zoo actually emitted is
   enumerated in `docs/decisions/0006-psd-fidelity.md` (bottom-first
   inverted group pairs, two real-world channel-stream layouts, the
   length-less composite section, the pooled composite RLE table, IM's
   byte-reversed blend key). Reader from the public Adobe spec
   (`io::psd::decode`): header gates, layer+mask walk, raw + PackBits
   channels (both layouts via interleaved-then-block retry), composite
   (prefixed + length-less detection), scope gates + note-drops per
   the decision's checklist. **Stopping condition met:** own-generated
   minimal fixtures (hand-built byte arrays) + the ag-psd oracle
   fixture (committed) + ImageMagick samples parse to expected trees —
   15-test battery in `crates/io/tests/psdread.rs` (composite-only
   flat file, layered offsets/premultiply, exact RLE decode, group
   pairs incl. the inverted real-world order, unicode names, hidden
   flags, text-as-raster, adjustment skip, reversed-key tolerance,
   clip/mask reports, the full blend table, the oracle tree, hostile
   stream refusals, negative-count alpha, the pooled composite RLE
   table); ImageMagick 7.1.1 corpus parses (`-depth 8` raw + RLE,
   flat + layered; 16-bit and ZIP refuse with reasons); zero panics
   on the mutated-byte fuzz arm (`psdimport` libfuzzer target +
   stable anchor, seeds built in code, anchors 2048 + panic_hunt 256
   seeds clean). **Defects the battery caught in shipped code:** a
   PackBits precedence underflow (`1 - control as usize` panicked on
   hostile runs), the composite length-prefix misalignment, the
   pooled composite RLE table, and `raster::attach_image`'s global
   tile-id collision (per-image numbering clobbered bank slot 0 on
   every multi-image import).
6. **PSD writer** — layered RGB/8 PSD from `Document` (maximize
   compatibility = merged composite + RLE layer channels); round-trip
   through our reader (semantic equality), open-provable by
   ImageMagick + PIL on the host (oracle report); `document.savePsd`
   command. **Stopping condition:** write→read equality on a scripted
   multi-layer document (paths, groups, opacity, blend modes, a bitmap
   object); oracle report archived in the devlog; checklist updated.
7. **Animated GIF ↔ states** — import: GIF89a frame loop →
   `frame_count = N`, frame *i* composited into cell *i* (disposal
   method respected; delays in the doc's per-state record — map to
   `FRVi`-style timing if the model carries it, else bag exact-path
   and record). Export: states → animated GIF (global palette with
   per-frame local palettes as needed, loop count, per-frame delay);
   the `AACb`/`AADb` crop/diff `EXPv` flags honored when set.
   **Stopping condition:** golden pin: N-frame animated GIF export
   (frame count + delays + per-frame ink anchors — a single-frame
   collapse cannot pin itself); import→export→import frame-count +
   delay equality; `gif` fuzz arm.
8. **Optimization settings, batch, surfaces + gates** — `EXPv` mapped
   both ways: `ExportOptions` children written under the dictionary's
   node tags where semantics are known, unknown children preserved
   exact-path (M3 rule); selective JPEG quality (per-slice `EXPv`
   quality table). Batch: `app-cli batch <dir> --format <f>
   --options <json>` over the registry. Shell: export dialog
   (format, quality/palette/dither controls, 2-Up preview via two
   compose renders). Floors raised where earned; fuzz arms; parity +
   scorecard regenerated; devlog. **Stopping condition:** `xtask ci`
   green; M8 scorecard row → done with gate evidence.

## CI image constraint

Tasks 1–2 add codec features (`image` features, `gif`, `tiff`,
`image-webp`), task 3–4 add `usvg`/`resvg` (xtask dev-dep) — the
lockfile moves, so `firecraft-ci-rust:1` **must be re-shipped to
fedora + t5500 before the push that carries them** (routing memory).
AVIF would add rav1e; it is excluded for exactly this reason too.

## Non-goals (M8)

PDF/FXG/CSS+Images/CSS Sprites/MXML export (M11), SWF/AI/EPS (no
viable clean-room target this milestone — AI/EPS postscript writer is
T3+), editable PSD/SVG text (raster+glyph-paths only), HEIC/JXL
(T4, OS-decoder stance), AVIF (see task 2), scripting-side batch
(`.jsf` batch scripts are M9 — the CLI batch here takes JSON options).
