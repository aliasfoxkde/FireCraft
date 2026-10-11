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
  (`document.importImage`, …) with capturing deltas; pure byte
  emission (save/export) is a query-like call, not a delta.

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
   recorded). Engine gains `document.export` (format + options +
   destination-memory) and the web bridge's `exportPng` reroutes
   through it. **Stopping condition:** per-format encode→decode
   round-trips (decode via `image`, tolerance 0 for lossless, pinned
   RMSE window for JPEG); `WASM_CRATES` 12 → 13; layers + clippy
   green.
2. **Raster import targets** — `io` opens JPEG/GIF/BMP/TIFF/WBMP/WebP
   (lossy+lossless decode; lossless encode only, per §7.6) into a new
   document (`document.importImage` command: frame-sized `IMGv` object
   + tile payload, canvas = image bounds). AVIF is **not** in M8: the
   pure-Rust encoder (ravif+rav1e) is a huge compile and decode has no
   viable pure-Rust path — deferred to M12 with the §7.6 platform-
   decoder stance; recorded in the decision note. **Stopping
   condition:** decode→re-encode→decode equality on lossless formats;
   oracle comparisons vs ImageMagick-generated samples (report);
   fuzz arm over truncated/mutated headers → no panic, clean refusals.
3. **SVG export** — own serializer over the model (no SVG writer
   crate exists worth the name): tree → `<g>`/`<path>`/`<text>`/
   `<image>`, transforms, opacity, blend modes as `mix-blend-mode`
   where SVG names one, live-filter stacks → SVG filter primitives
   for the supported kernel set (feGaussianBlur, feColorMatrix,
   feComponentTransfer, feConvolveMatrix, feTurbulence-family only if
   semantics match; unmappable filters are dropped *with a report
   entry*, never silently). **Stopping condition:** golden loop —
   render our SVG with `resvg` (xtask dev-dep) vs `compose::render`
   of the same document at recorded tolerance; SVG bytes sha-pinned;
   filter-mapping table in the decision note.
4. **SVG import** — `usvg` 0.48 parse → model walk: paths (fills/
   strokes/gradients), groups (transforms, opacity, clipping as
   masks), embedded rasters → `IMGv`; text via usvg's fontdb layout
   over the bundled Liberation Sans dir (text arrives as glyph paths
   — editable-text import is a recorded limitation). **Stopping
   condition:** round-trip our own SVG export → import → compose
   render ≈ original render (pinned tolerance); hostile-SVG fuzz arm
   (no panic); limitation list in the report struct, documented.
5. **PSD spike + reader** — spike first per risk #3: generate layered
   PSDs with ImageMagick + ag-psd (node, scratch under `/var/tmp`,
   never committed), enumerate the record zoo actually emitted
   (layer records, channel info, PackBits, group markers
   `<Layer set>`/`</Layer set>`, clipping, blend modes, ICC blobs);
   findings → `docs/decisions/0006-psd-fidelity.md`. Reader from the
   public Adobe spec: header, color modes (RGB/8-bit first; others
   refuse with a report), layer+mask section, channel data (raw +
   RLE), merged composite. Model mapping: layers → layers, groups →
   groups, opacity/blend/visibility/clipping mapped (blend-mode table
   PSD↔Fireworks in the decision note), text layers import as raster
   (limitation recorded), adjustment/vector-mask layers flatten into
   the composite. **Stopping condition:** own-generated minimal PSD
   fixtures (hand-built byte arrays, house style) + ImageMagick
   samples parse to expected trees; zero panics on the mutated-byte
   fuzz arm; fidelity checklist rows for every mapped feature.
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
