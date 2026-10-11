# 0008 — SVG export: the live-filter mapping table and the resvg golden loop

- **Status:** Accepted (M8 tasks 3–4, 2026-10-11)
- **Deciders:** Michael Kinney, Claude
- **Scope:** `io::svg` (L4) — how a document's `EFLv` effect stacks,
  paints, and geometry serialize to SVG 1.1, and how the serializer is
  proven (`xtask svg-golden`); and `io::svgimport` (L4) — the inverse
  walk, its recorded limitations, and how it is proven
  (`xtask svg-roundtrip`).
- **Supersedes:** none · **Superseded by:** none

## Context

M8 task 3 requires an own SVG serializer over the doc model with live
filters mapped to SVG filter primitives "for the supported kernel set",
unmappable filters dropped *with a report entry*, never silently — and
a golden loop that renders the SVG with `resvg` and compares it against
`compose::render` at recorded tolerance.

Two design constraints drove everything:

1. **One source of truth per semantic.** `compose` already owns the
   blend-value table and the effect interpretation (`effects::plan`);
   the serializer must not carry a second copy of either.
2. **No formula duplication.** A transfer curve re-derived in SVG
   format-specific code will drift from the kernel that actually ships.

## Decision

### 1. Semantics are imported, not re-declared

The serializer consumes `compose::blend_mode` (the M4 value → `Mix`
table), `compose::plan` (the M7 effect interpreter), and
`compose::straight_alpha` (the house un-premultiply). SVG never gets
its own opinion about what blend `2` or effect
`"brightnesscontrast"` means.

### 2. Per-channel kernels ride a derived LUT, not a formula

BrightnessContrast, Levels, and Invert serialize as
`<feComponentTransfer>` with `type="table"` and 256-entry
`feFuncR/G/B` table values. The LUT is **derived by running the real
`filters` kernel over a 256×1 opaque gray ramp** (`Surface::from_straight`
of `0,1,…,255`), then reading the three channels back:

- exact transfer curves by construction — the SVG carries what the
  kernel computes, at every u8 sample point;
- opaque ⇒ premultiplied == straight, so the ramp needs no alpha
  reasoning;
- `color-interpolation-filters="sRGB"` on every `<filter>` — SVG
  filters default to linearRGB, which would silently wrong every LUT.

At 256 table entries, resvg's linear interpolation between entries is
exact at every u8 sample.

### 3. The mapping table

| `KernelOp` | SVG primitive | Fidelity notes |
|---|---|---|
| BrightnessContrast | `feComponentTransfer` table (derived LUT) | exact at u8 samples |
| Levels | `feComponentTransfer` table (derived LUT) | exact at u8 samples |
| Invert | `feComponentTransfer` table (derived LUT) | exact at u8 samples |
| GaussianBlur | `feGaussianBlur stdDeviation` | both sides are 3-box cascades; radius rounding differs by ≤ a few LSBs |
| BoxBlur r | `feConvolveMatrix order="2r+1"`, `kernelMatrix` all ones, `divisor=(2r+1)²`, `edgeMode="duplicate"`, `preserveAlpha="false"` | separable box == square box; clamp-to-edge == duplicate |
| UnsharpMask | `feGaussianBlur result="blur"` + `feComposite operator="arithmetic" in="SourceGraphic" in2="blur" k2=1+a k3=−a` (a = amount/100) | matches compose only on opaque pixels — arithmetic composite is premultiplied-space, compose's unsharp is straight-space |
| HueSaturation | **dropped** | no SVG primitive computes HSL per pixel |
| AddNoise | **dropped** | SVG is a deterministic format; noise has no primitive |
| FindEdges | **dropped** | `|Gx|+|Gy|` Sobel — the abs prevents a single-matrix convolution, and SVG has no abs primitive |

**Any unmappable op drops the whole filter** (compose's `plan` applies
all ops or none — a partial chain would be a different semantic), with
one report line per effect naming the kernel. The object itself still
exports, unfiltered, exactly as compose renders it.

### 4. Structure mirrors compose's render walk

- One `<g id="layer-N">` per visible layer/cell; group and object
  opacity/blend/filter ride a `<g>` wrapper (`opacity`,
  `style="mix-blend-mode:…"`, `filter="url(#fcN)"`). A group whose
  wrapper would be empty emits **nothing** — and is not closed either
  (the balanced-tag bug the golden loop caught on day one).
- `FOTb` fill-on-top emits two `<path>` elements — stroke first, fill
  over — because one element cannot order its own paint; this mirrors
  compose's stroke-then-fill order.
- Filter region is the full canvas in `userSpaceOnUse` units:
  compose rasterizes filtered objects over a full-canvas offscreen
  (decision 0005), so blur spread reaches the canvas edge there, and
  the SVG region must match.
- Stroke is always 1px centered (the recorded M4 limitation), paths
  with no paint are skipped with a report entry, `mask_hidden` groups
  composite children unmasked — all compose parity.
- Text exports as `<text>` with a report entry (compose lowers glyphs;
  pixel parity depends on the viewer's font stack). Images embed as
  base64 data-URI PNGs with a report entry (compose renders images as
  nothing yet — M7-owed work). Slice/symbol instances render as
  nothing, with a report entry.

### 5. The golden loop: independent renderer, recorded tolerance

`xtask svg-golden` (in `ci`) serializes each corpus fixture, rasterizes
the SVG with `resvg` — an independent renderer, not our stack — and
compares against `compose::render` of the same document, both
premultiplied. The SVG bytes are sha-pinned (`<fixture>.svgpin`,
`--bless` to re-pin) so an unintended serializer change fails before
the comparison is even trusted.

Fixtures whose compose/SVG semantics provably diverge are skipped,
each with its reason (text=N, image=N, dropped_filters=N): text and
image fixtures today. Nothing skips silently.

**Recorded tolerance** (measured over the eligible set, then pinned):

| fixture | max Δ | mean Δ | profile |
|---|---|---|---|
| composed.fw.png | **0** | 0.0000 | bit-exact end to end |
| scripted.fw.png | 77 | 0.4779 | 205/3072 samples >8, at petal tips |
| filtered.fw.png | 73 | 0.6368 | 280/3072 samples >8, at blur ramps |

Pinned bounds: `MAX_CHANNEL_DELTA = 96`, `MAX_MEAN_DELTA = 0.9`.

The divergence is fully attributed: flat interiors are bit-exact
(composed proves the whole pipeline — geometry, paint, blend wrapper,
opacity — survives the round trip through SVG exactly); the only
differences live on thin curved features (autoshape petal tips, where
tiny-skia's scanline AA and vello_cpu's tesselator AA legitimately
disagree) and on blurred alpha ramps (premultiplied-space SVG filter
arithmetic vs compose's straight-space kernels, §3 UnsharpMask row).
A structural bug — wrong geometry, blend, or filter mapping —
saturates to 255 over a region and cannot hide under these bounds.
Re-measure after renderer or kernel changes with
`cargo run -p xtask -- svg-golden --measure`; re-pin only with intent.

### 6. The import half: bake, don't represent (task 4)

`io::svgimport::decode` walks the usvg tree into the model with the
mirror-image rule set:

- **Transforms bake into geometry.** The model has no transform spine,
  so usvg's per-node absolute transforms compose into the path data
  (quads raise to cubics at the ⅔ control-point marks — the exact
  degree-elevation identity). Groups keep only semantics the model can
  represent: per-mille opacity, blend (through
  `compose::blend_value`, the export table's exact inverse), and
  structure.
- **Unrepresentable features drop with a report entry, never
  silently** — the same discipline as export. `SvgImportReport`
  counts `texts_flattened` (text layouts through fontdb seeded with
  `text::FACE_BINARIES`, the same binaries compose rasterizes — they
  arrive as glyph paths, which is lossless *as rendered*),
  `gradients_dropped` (no model gradient paint yet), `strokes_flattened`
  (the model records 1px strokes; width ≠ 1 and dash arrays flatten
  with notes), `clips_dropped` (children import unmasked — compose
  parity, §4), `filters_dropped`, and `images` (embedded rasters →
  `IMGv` over the shared tile bank; nested SVG images refuse).
- **usvg behaviors pinned by tests** (behavioral specs, not code
  copy): single-stop gradients collapse to solid `Color` at parse;
  `fill="none"` paths are culled at parse and never reach the walk;
  SVG's default fill is black, so "unpainted" requires `fill="none"`;
  a serializer's `<g id="layer-1">` wrapper imports as a structured
  group.

**The hostile-input finding: two upstream crates recurse without a
depth cap.** The fuzz arm's first deep-nesting input (4096 `<g>`
elements) overflowed the 8 MB main-thread stack *inside
`roxmltree 0.20`'s `Document::parse`* — before usvg's own converter
ever ran, and usvg's converter recurses unbounded too. Neither crate
exposes a depth knob (`ParsingOptions` is `allow_dtd` + `nodes_limit`
only). The importer therefore gates first with a flat, stack-safe
element-depth scan (`MAX_ELEMENT_DEPTH = 128`; exact for well-formed
XML, conservative only in the reject direction) and refuses canvases
above the 65535px compose `Render` ceiling (u16 dimensions —
importing larger would just relocate the failure). This is recorded
as an upstream defect worth reporting: hostile SVG is web-edge input,
and a stack overflow is a crash, not a clean error.

**Provenance (`xtask svg-roundtrip`, in `ci`):** export → import →
compose render vs the original compose render — both sides render
with compose, so any delta is import loss. Measured **bit-exact at
Δ 0 on the whole eligible set** (composed, scripted); zero tolerance
pinned. Fixtures skip with reasons: filtered (SVG filters import
unfiltered — the recorded both-directions limitation), text. SVG
bytes are already sha-pinned by the export gate, so no additional
pins.

## Consequences

- SVG export fidelity is checked by an outside renderer, so a bug in
  vello_cpu cannot validate itself.
- The mapping table is now contract: adding a kernel to `filters`
  without deciding its SVG story (map or documented drop) fails the
  gate via `plan` growing an unmapped op that ships to the drop path.
- Straight-vs-premultiplied blur remains a recorded, bounded
  divergence; closing it would mean premultiplied kernels in
  `filters` — an M7-affecting change, deliberately not taken here.
- SVG now round-trips at Δ 0 through our own serializer and importer;
  any future serializer *or* importer change that loses a pixel fails
  `svg-roundtrip` in CI. Gradient, filter, and clip support arriving
  in the model re-opens both halves of this note deliberately.
- The depth gate is a load-bearing seam: bumping `MAX_ELEMENT_DEPTH`
  or adopting a fixed usvg/roxmltree must re-prove the deep-nesting
  fuzz case.

## References

- `docs/planning/firecraft/M8-import-export.md` tasks 3–4 (stopping conditions)
- `docs/decisions/0005-live-effect-render.md` (offscreen margins, cache)
- `docs/decisions/0004-filter-color-order.md` (pinned channel order the LUTs inherit)
