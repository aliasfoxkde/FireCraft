# Fireworks CS6 binary notes — derived findings

Recorded verdicts from the Ghidra passes over the local `Fireworks.exe`
(27 MB PE32, MSVC, stripped). Clean-room discipline (PLANNING §4): what is
committed here is derived behavioral description only — no decompiled text,
no Adobe bytes. Raw extracts live in the scratch RE workspace
(`/nas/Temp/tmp/firecraft-re/`, gitignored by location; recipe below
regenerates everything).

Method: Ghidra headless full analysis; MSVC RTTI recovery by scanning the
image for dword references to each class's type descriptor (both the
class-hierarchy and complete-object structures anchor the descriptor at
+0x0C), validating the candidate complete-object locator, then locating the
vftable as the dword-slot referencing that locator (the vftable's [-1] meta
slot) and walking its function slots. ~3,300 class names recovered; the
names below are behaviorally verified, not just listed.

## Filter objects are property bags with an interpolation protocol

Classes inspected: `CBrightnessFilter` (vftable slots 0–10),
`CAutoLevelsFilter` (slots 0–19), `CAddNoiseFilter`, `CBlurFilter`,
`CCreateAlphaFilter`, `CCurvesFilter` (same shape).

Verdicts:

- **Filters/effect objects are `CMbObject` subclasses carrying typed
  properties addressed by named property atoms** — the setter paths store
  through a vtable-indexed property accessor keyed by a per-property id,
  not through C++ fields. Confirms the property-bag design FireCraft's
  `filter.setParam` registry command already uses; no schema change needed.
- **Every filter class carries a virtual "interpolate toward another
  instance" method** (slot 9 in the inspected vftables): per property,
  `a + t·(b − a)` with `t` clamped to `[0,1]` (the guard is explicit:
  values outside the range leave the property untouched). AutoLevels
  interpolates **19** properties; Brightness **2**. This is the animation/
  state-blend machinery — the missing semantic behind live-effect tweens.
- **The matching property-wise equality virtual** (AutoLevels slot 2)
  compares all properties pairwise, with distinct float handling for the
  float-typed ones — i.e. the object system distinguishes int and float
  property types at the protocol level.
- Consequence for M14: per-filter parameter tables (name, type, default,
  range) are first-class evidence targets; the counts above are the first
  entries.

## Pixel kernels: the `St*Processor` layer (AdobePSL-adjacent)

The kernels are **not** in the filter classes — the RTTI map shows a
dedicated processor layer: `StAddNoiseProcessor`,
`StInvertPixelsProcessor`, `StUnsharpMaskProcessor`,
`StExpandOrMaskAreaOrSmoothProcessor`, `StBlendPixels*` (six blend
variants), `StHistogramRGBProcessor`, `StCalcAverageProcessor`,
`StLineDrawingProcessor`, `StSamplePixelsProcessor`, plus `CBlurTool`/
`CBlurToolQuad3` and the `CBitmapQuad3/4/6` family (width-3/4/6 SIMD
variants of the same ops). The streaming machinery around them is the
`AdobePSL` namespace (Adobe's Photoshop-style pixel stream: `Stream`,
`StdFInputStream/OutputStream`, `T<>Processor` templates parameterized
on component accessor functors like `DoGetRed/Green/Blue`).

Kernels decoded (each is a single `operator()`-shaped virtual walking a
rect: `[left..right) × [top..bottom)` with a per-row stride rebase):

- **`StInvertPixelsProcessor`**: `pixel = (pixel & 0xff000000) |
  (~pixel & 0xffffff)` — the first three bytes inverted as one 24-bit
  word, the **fourth byte (alpha) preserved exactly**. One branchless
  word op per pixel.
- **`StAddNoiseProcessor`**: the RNG is the **MINSTD Lehmer generator
  (a = 48271, m = 2³¹−1, Schrage form)** — `x = 48271·x mod (2³¹−1)`
  seeded from a packed 16.16 pair; draws index a **precomputed `short`
  noise table** (pointer + range walked with wrap; `amount` is baked
  into the table range at generation, not applied per pixel). Per
  channel: `clamp(byte + noise, 0, 255)` (branchless low and high
  clamps), **alpha byte untouched**, and a mode flag picks one table
  value per pixel for all three channels (monochrome) or three
  successive values (color noise).

FireCraft parity verdict (`crates/filters`): the structural contract
matches — per-channel add with clamps, alpha preserved, monochrome =
one delta across RGB, amount scales the span. The one deliberate
substitution stands: SplitMix64 instead of MINSTD for the draw stream
(determinism is the contract; recorded in the kernel docs). The
table-walk structure (precomputed range, per-pixel stepping) is not
reproduced — per-pixel draws are equivalent behavior at our scale; the
table exists in CS6 for SIMD reuse across the Quad3/4/6 variants.

## Writer path: change-driven delta emission confirmed

Classes inspected: `WriteStreamProcess` (37+ vtable slots walked, 121
functions decompiled across the writer set), `CMbFile` (11 slots). RTTI
correction: the structure-outputter layer the planning docs called
`CMkbStructureOutputter`/`CWriteDictInfo` exists under the names
**`CMbWriter`**, **`CMbDict`**, **`CDictTagState`** (and, for the text
dict, `SLOMkbWriter`/`SLOMKbReader` in the `SLO` text-engine namespace).

Verdicts:

- `WriteStreamProcess` is a **node-type dispatcher** (a process/visitor
  that walks the document and routes each node to a type-specific emit
  path through a wide vtable), not the serializer itself.
- **Write-if-changed is implemented as a virtual** (slot 11): the emit call
  fires only when the new value differs from the current one. This is
  code-level confirmation of the delta/dict emission FireCraft's fwpng
  writer derived from corpus round-trips — the writer omits unchanged
  properties by design, not by accident of the corpus.
- **Emit protocol (decoded from slots 2/16/19/23/37/38/39).** The process
  holds a stream *sink* object at `this+0x40`; everything is emitted
  through the sink's vtable: `+8` scalar, `+0x10` wide-char token,
  `+0x14` sized value, `+0x20` formatted string, `+0x34`/`+0x38`
  begin/end *record group*. The generic value emit (slot 37) reads a
  tagged property cell whose vtable carries `+4` validity, `+0xc` scalar
  value, `+0x10` is-object, `+0x1c` inner object: scalars go straight to
  the sink, objects are emitted as `sink+0x34` → **the object's own
  self-write virtual (vtable slot 2, receiving the process as argument)**
  → `sink+0x38`. So the format's framing (length/record groups around
  every non-scalar value) and the visitor self-write contract are both
  code-level facts now; fwpng's writer mirrors this shape.
- Two backend flavors exist per node type — slot 2 dispatches `this+0x44`
  (plain) vs `this+0x9c` (changed): the changed flavor wraps the same
  record frame around a pre-walk of the object before its self-write
  (the delta-enumeration step). Slot 38 is a plain `return 1` predicate.
- **Tag emission policy** (FUN_0137fa00, the callee of the pre-walk):
  each tag record carries either a readable name string or a numeric id
  (`sprintf "%d"`), chosen from writer state — a names flag at `+0x8`,
  a depth budget at `+0x10` (compared against a constant 5 and the
  tag's own level at `tag[2]`), and a mode at `+0x14`. This is the
  corpus' named-tag/numeric-tag duality, now code-level.
- **The written objects implement `SLO::MTaggedStreamable`** (RTTI
  `__RTDynamicCast` from `SLO::MStreamable` in FUN_0137fb30): the
  self-write contract is the `MStreamable`/`MTaggedStreamable`
  interface pair, not the CDbObject interface. `CDbText`'s
  48-slot main vtable is its object interface (slot 2 there is a
  move/translate method); the *streaming* face is a separate small
  interface COL.
- String building goes through fixed-capacity wide-char buffers
  (`SLO::StaticArray<unsigned_short,32>`) — consistent with the short
  inline-string encoding observed in corpus chunks.
- **Transport layer (SLOMkbWriter/SLOMKbReader).** `SLOMkbWriter`
  slot 3 is the chunk-body write: it calls straight into libpng's
  `png_write_chunk_data` with the running byte count at `+0x14`,
  **behind a `UseZLib` global gate that compresses the payload first**
  (the deflate-path call family) — the code-level confirmation of
  mk-chunk zlib compression. `SLOMKbReader` mirrors it (remaining/
  consumed counters at `+0x18`/`+0x1c`, read callback at `+0x14`).
  Both derive from `CMbObject`.
- **`CMbWriter` capability slots**: slot 0 returns a version global,
  slots 1–3 return the four-character codes `'MKBY'` and `'PNGf'`
  (the classic-Mac file-type stamps for the format it writes).
- **Name correction settled**: the planning-era names
  `CMkbStructureOutputter`/`CWriteDictInfo` do not exist in RTTI; the
  real classes are `CMbWriter` (top-level service),
  `CMbDict` (the in-memory MOA-style property dict — one vtable slot
  per value type, COM-flavored refcounting), `CDictTagState` (tag
  machine), `SLOMkbWriter`/`SLOMKbReader` (transport). The mkTS
  *observable* parameter layout is already covered by fwmap's
  corpus-validated `TXTv`/run/style mapping; the per-field SLO
  text-engine layout behind it is an enhancement-phase unknown (rich
  text beyond corpus coverage), not a writer-architecture unknown.

## The live-filter surface (M14 enumeration, task 1)

Every live effect CS6 ships is identified by its `EPSv` `EffectMoaID`
GUID (the observational dictionary's `DCEv "EffectMoaID"` table —
complete, cross-checked against the `Configuration/` `.jsf` corpus and
the RTTI filter classes from the pass above). This table is the M14
completion checklist: an effect is done when its row carries a matching
kernel, pinned parameters, and a golden. Status legend: **interpreted**
(compose maps it to kernel ops today), **M14** (native CS6 effect to
implement), **plugin** (third-party gallery — passed through by design,
never guessed at).

Adjust Color family:

| Effect | GUID | Parameters (dictionary names) | Status |
|---|---|---|---|
| Brightness/Contrast | `3439b08c-1921-11d3-…` | `brightness_amount`, `contrast_amount` (−100..100) | interpreted |
| Levels | `d04ef8c2-71b5-11d1-…` | `source_low/high_rgb` + `gamma_rgb` + `dest_low/high_rgb`, and the per-channel scalar twins (`*_red/green/blue`; the `*_points` knot lists belong to Curves above) | interpreted (master channel only) — M14: per-channel |
| Curves | `3439b08e-1923-11d3-…` | `rgb_points`, `red_points`, `green_points`, `blue_points` (`GPLv`/`GPTv`/`XLCf`/`YLCf` knot lists) + `gamma_*` | interpreted (piecewise-linear knots; `gamma_*` unused — CS6 draws straight segments) — M14: golden |
| Hue/Saturation | `3439b08d-1922-11d3-…` | `hue_amount` (deg), `saturation_amount`, `lightness_amount` (−100..100), `hls_colorize` | interpreted (`hls_colorize` unchecked — M14: verify) |
| Auto Levels | `d04ef8c1-71b4-11d1-…` | histogram-derived per-channel stretch (the `StHistogramRGBProcessor` family) | interpreted (plain min/max stretch, no outlier clipping — pinned substitution) — M14: golden |
| Color Fill | `dd54adc0-a279-11d3-…` | `Color` (`#rrggbb` text), `Opacity` (0..100), `Blendmode` (numeric code) | interpreted (13 of the dictionary's 33 blend codes map onto the kernel set; the rest skip) — M14: golden |
| Invert | `d2541291-70d6-11d1-…` | none | interpreted (24-bit word op, alpha preserved — decoded above) |

Blur family:

| Effect | GUID | Parameters | Status |
|---|---|---|---|
| Blur | `f1cfce41-718e-11d1-…` | none (fixed radius 1) | interpreted |
| Blur More | `f1cfce42-718e-11d1-…` | none (fixed radius 2) | interpreted |
| Gaussian Blur | `d04ef8c0-71b3-11d1-…` | `gaussian_blur_radius` | interpreted |
| Motion Blur | `538016b6-ffd6-418f-…` | `motion_blur_angle` (0..360 deg), `motion_blur_distance` (1..100) | interpreted (one-sided taps `[0..=d]` along `(cos θ, −sin θ)` — pinned substitution) — M14: golden |
| Radial Blur | `d1c33142-2ad8-4215-…` | `radial_blur_amount`, `radial_blur_quality` | interpreted (spin: sweep in **degrees** 0..360, arc uniform over `[0..=sweep]` — pinned substitution) — M14: golden |
| Zoom Blur | `94dbd663-9b2e-44bf-…` | `zoom_blur_amount` (1..100), `zoom_blur_quality` (1..100) | interpreted (taps uniform in scale toward the center — pinned substitution) — M14: golden |

Sharpen family:

| Effect | GUID | Parameters | Status |
|---|---|---|---|
| Sharpen | `c20952b1-fc76-11d0-…` | none (preset `[0 −1 0 / −1 5 −1 / 0 −1 0]`) | interpreted (pinned substitution) — M14: golden |
| Sharpen More | `1f2f2591-9db7-11d1-…` | none (preset `[−1 −1 −1 / −1 9 −1 / −1 −1 −1]`) | interpreted (pinned substitution) — M14: golden |
| Unsharp Mask | `f1cfce44-718e-11d1-…` | `unsharp_mask_radius`, `unsharp_mask_amount`, `unsharp_mask_threshold` (absent = 0) | interpreted — threshold honored since M14 (M7-era skip retired) |

Stylize / other:

| Effect | GUID | Parameters | Status |
|---|---|---|---|
| Add Noise | `e4c0f4bc-c0a3-4cb3-…` | `add_noise_amount` (0..100), `add_noise_use_color` | interpreted (MINSTD decode above; seed substitution documented) |
| Find Edges | `fc7093f1-f95c-11d0-…` | none | interpreted |
| Convert to Alpha | `2932d5a2-ca48-11d1-…` | none documented (`CCreateAlphaFilter`; `EdgeThreshold` belongs to Glow/Bevel/Emboss, not here) | interpreted (house Rec.601 luma → opacity, color → black — pinned substitution) — M14: golden |
| Clouds | `ff84d00c-4494-11d8-…` | `sky_color`, `cloud_color` | M14 |
| Difference Clouds | `d2e7769c-6349-11d8-…` | `sky_color`, `cloud_color` | M14 |

Pinned substitutions (M14 kernels whose exact CS6 arithmetic is not
observable without a render oracle — each is a deliberate, recorded
choice, revisited only on oracle evidence): auto levels stretches
per-channel min/max with no outlier clipping; convert-to-alpha maps
house Rec.601 luma `(299r+587g+114b)/1000` to opacity over black;
the sharpen presets use the classic 5-point / 9-point kernels;
motion blur taps one-sided `[0..=d]` along `(cos θ, −sin θ)` (UI
counterclockwise, image y down); zoom blur averages taps uniform in
scale toward the center; radial (spin) reads `radial_blur_amount` as
sweep **degrees** (0..360) over an arc uniform in `[0..=sweep]`.
All pixel math is integer-exact per decision 0004 — the only f64 is
the one-time trig-table build (sin in thousandths over 4096 turn
divisions), mirroring the boxes_for_gauss precedent.

Shadow/glow/emboss — two encodings, one semantic family:

- **CS6 unified panel** ("Photoshop Live Effects Special Fill",
  `8eeadf50-9efe-11da-…`): one EPSv effect whose payload carries
  per-kind packed strings under `dropShadow`, `innerShadow`,
  `outerGlow`, `innerGlow`, `bevelEmboss`, `satin`, `solidFill`,
  `gradientFill`, `patternFill`, `stroke`. The dictionary documents
  the `"&"`-separated fields for `dropShadow` and `bevelEmboss`
  field-by-field (angle/size/distance/blend/`RGBColor;r;g;b` and the
  bevel style/stamp variants); `innerShadow` shares the dropShadow
  shape, `innerGlow`/`outerGlow` the glow shape (glow width, start
  distance, matte, gradient flag, `edgeGlow`/`centerGlow`).
- **MX-era native effects** (still shipped in CS6's menu): Drop Shadow
  `a7944db8-6ce2-11d1-…`, Inner Shadow `5600f702-774c-11d3-…`,
  Inner Glow `2ba87123-8220-11d3-…`, Bevel `7fe61102-6ce2-11d1-…`,
  Solid Shadow `b90c950e-64df-11d8-…` — scalar props
  `ShadowAngle/ShadowBlur/ShadowColor/ShadowDistance/ShadowType`,
  `GlowWidth/GlowStartDistance`, `LightAngle/LightDistance`,
  `BevelWidth/BevelContrast/BevelType`, `EmbossFaceColor`,
  `HiliteColor`, `OuterBevelColor`, `SlopeMultiplier/SlopeType`,
  `ShowObject`.

All of the family is **M14** (compose currently skips every row), as
composites over the proven blur/blend pieces — not new pixel math.

Plugin gallery (passed through by design, each recorded here so a
future GUID hit is a lookup, not a puzzle): the Fireworks MX
gallery set (all sharing the `d810e821-f86b-11d0-…` prefix —
Accented Edges, Angled Strokes, Bas Relief, Chalk & Charcoal, Chrome,
Colored Pencil, Conté Crayon, Craquelure, Crosshatch, Diffuse Glow,
Dark Strokes, Dry Brush, Film Grain, Glass, Glowing Edges, Grain,
Ink Outlines, Neon Glow, Note Paper, Ocean Ripple, Photocopy, Plastic
Wrap, Plaster, Palette Knife, Paint Daubs, Patchwork, Rough Pastels,
Reticulation, Smudge Stick, Sumi-e, Sponge, Sprayed Strokes, Spatter,
Stamp, Torn Edges, Texturizer, Underpainting, Water Paper, Watercolor),
the Eye Candy 4000 LE trio (Bevel Boss, Marble, Motion Trail), Alien
Skin Splat LE (Edges). These are third-party plugins, not CS6 surface.

## Status of §5.5 passes

| Pass | State | Committed output |
|---|---|---|
| Filter/effect pipeline | complete: property protocol, interpolation semantics, and the `St*Processor` kernel layer (invert + add-noise decoded; kernel family + AdobePSL stream mapped) | this section |
| PNG writer | complete: dispatcher, delta emission, emit protocol, tag policy, tagged-streamable interface, transport (chunk write + zlib gate), class names — all verdicts above | this section |
| Chunk serializers | transport confirmed (SLOMkbWriter/MKbReader); per-chunk emit order covered by fwmap's corpus round-trips | this section |
| UI resources | complete (menu tree, keymap, dialogs extracted); see `fw-ui-evidence.md` | `fw-ui-evidence.md` |

## Scratch workspace recipe

Toolchain: JDK 21 + Ghidra (headless) under `/nas/Temp/tmp/firecraft-re`
(`env.sh` exports `JAVA_HOME`, `PATH`, `GHIDRA_INSTALL_DIR`); persistent
Ghidra project `fwproj/Fireworks.gpr` holds the fully analyzed `Fireworks.exe`.
Scripts (Java postScripts): `find_vfts.java` (TD→COL→vftable walk,
brute-force dword scan), `decompile_vfts.java` (decompiles the vftable
functions per class). Re-run:

    . /nas/Temp/tmp/firecraft-re/env.sh
    <ghidra>/support/analyzeHeadless .../fwproj Fireworks \
      -process Fireworks.exe -noanalysis \
      -scriptPath .../scripts -postScript find_vfts.java

Outputs land in `findings-code/` (scratch, never committed).
