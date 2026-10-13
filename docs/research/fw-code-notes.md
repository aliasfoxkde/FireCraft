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
