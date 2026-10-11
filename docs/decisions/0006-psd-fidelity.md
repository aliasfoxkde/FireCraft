# 0006 — PSD fidelity: the mapped-feature checklist and the import/export contract

- **Status:** Accepted (M8 task 5, 2026-10-11; extended by task 6, 2026-10-11)
- **Deciders:** Michael Kinney, Claude
- **Scope:** `io::psd` (L4) — the PSD reader's feature mapping, what
  imports faithfully, what flattens, what refuses; the fidelity
  checklist the M8 gate demands.
- **Supersedes:** none · **Superseded by:** none

## Context

The M8 ticket demands PSD *in* (out is task 6) fidelity-tested against
documented CS6 behavior. There is no CS6 on this machine, so "CS6
behavior" is grounded in: the public Adobe Photoshop File Formats
specification (the record layout), `ag-psd` 0.8 (MIT TypeScript
oracle, PLANNING §7.6 — used here as a **writer** for oracle fixtures
and as the strictness baseline), and ImageMagick 7.1.1 samples
(hostile-world corpus). The spike enumerated the record zoo these
three actually emit.

### Spike findings (the record zoo, observed)

- Header: `8BPS`, version (1 = PSD, 2 = PSB), channels, height,
  width, depth, color mode. **ImageMagick writes 16-bit depth by
  default** — every IM sample needed `-depth 8` to be in scope.
- Layer records are stored **bottom-first** — the same z-order the
  model's `ELMv` lists and compose's render walk already speak.
- Groups are **two records in bottom-first order**: the *bounding
  divider* (an `lsct` block with i32 type 3, zero-size rect, legacy
  pascal `</Layer set>` / `</Layer group>`) sits **below** the
  children, and the *header* (`lsct` type 1 open / 2 closed, carrying
  the group's real name, blend, opacity, and visibility) sits
  **above** them. The first reader draft walked the pair top-down and
  mis-grouped every real file; the oracle fixture caught it.
- Channel streams come in **two real-world layouts**: interleaved
  after each record's extra data (ImageMagick) or as one block after
  all records (the spec layout Photoshop and ag-psd write). The
  reader walks interleaved first and retries the block layout on any
  structural failure.
- Names exist twice: a pascal string in the record (padded to 4) and
  a Unicode name in `luni` (u32 count + UTF-16BE). The Unicode name
  wins when both exist. ag-psd writes the legacy `<Layer set>` in
  *both* slots of a group header — a group's display name is whatever
  the authoring tool put there.
- The merged composite section is spec'd with a u32 length prefix;
  **ag-psd omits it** (compression marker directly), and so does
  ImageMagick. An exact length-vs-remaining match selects the
  prefixed layout; otherwise the length-less form is tried first. A
  prefix cut inside a layered file's composite can therefore accept
  under the length-less reading — benign, because with layers present
  the composite is validation-only (never the document).
- Composite RLE **pools all channels' scanline byte counts into one
  table** (`rows x channels` u16 entries) followed by a single packed
  stream — the spec's image-data layout, confirmed byte-level against
  an ImageMagick sample and in ag-psd's `writeDataRLE` (its write
  cursor starts past `offsets.length * height` count entries). Layer
  channels are the opposite: each carries its own table inside its
  own channel length. The first reader draft reused the layer layout
  for the composite and every real flat RLE file misparsed; the IM
  sample caught it.
- ag-psd's writer **never emits the layer clipping byte** (the
  property is read-only in its model) — clipped members of oracle
  fixtures arrive unclipped; the hand-built fixtures pin clipping.
- Layer opacity is one **u8** (0–255); 50% exports as 128 → model
  per-mille `(128 * 1000 + 127) / 255` = 502 (ag-psd reads it back
  0.50196 — the model's per-mille is strictly finer).
- Blend keys are 4-byte tags; the full table is fixed by the spec:
  `pass norm diss dark mul  idiv lbrn dkCl lite scrn div  lddg lgCl
   over sLit hLit vLit lLit pLit hMix diff smud fsub fdiv hue  sat
   colr lum `.
- Channel data: per channel a u16 compression (0 raw, 1 PackBits
  RLE, 2 ZIP, 3 ZIP-with-prediction); RLE prefixes the packed rows
  with a per-scanline u16 byte-count table (u32 in PSB).
- **ImageMagick writer bug, found by the spike:** on the
  parenthesized-image-list code path IM 7.1.1-43 emits the blend key
  byte-reversed (`38 42 49 4d 6d 72 6f 6e` — `8BIM` + literal
  `mron`) — `'norm'` packed into a native-endian u32 and written
  little-endian. The `ag-psd` reader throws on it; real-world files
  in the wild will contain it. **Contract:** unknown or malformed
  keys never fail the import — they land as a report note and the
  layer imports `normal`. (Upstream-worthy; both halves of the
  pair are recorded here.)
- `ag-psd`'s reader is *strict*: it throws on unknown blend modes and
  uninitialized canvases. Our reader is deliberately the opposite —
  the house import rule (record, never crash, best document
  possible).

## Decision

### Scope gates (refuse with a reason, never approximate)

| Feature | Behavior |
|---|---|
| signature ≠ `8BPS` | refuse (`Decode`) |
| version 2 (PSB) | refuse (`Unsupported`: "PSB (large-document) variant") |
| depth ≠ 8 | refuse (`Unsupported`: "16/32-bit depths") — IM default hits this |
| color mode ≠ 3 (RGB) | refuse (`Unsupported` naming the mode) |
| canvas axis > 65535 | refuse — compose `Render`'s u16 ceiling (same as raster/SVG paths) |
| channel compression 2/3 (ZIP) | refuse (`Unsupported`) — recorded, not approximated |
| negative layer count | first merged alpha holds transparency; abs() + note |

### The fidelity checklist (every mapped feature: CS6/spec does → we do → gap)

| PSD feature | We do | Gap / note text |
|---|---|---|
| RGB/8 layers (raw + PackBits) | full pixels → `IMGv` over the shared tile bank, offset at the record's `left`/`top`, one model layer each | none — lossless |
| Layer order | bottom-first, unchanged | none |
| Layer visibility (flags bit 1) | `visible` | none |
| Layer opacity u8 | per-mille | rounding ≤ 1‰ |
| Layer blend keys | the 16 `blend_value`-invertible modes map exactly | `diss lbrn dkCl lddg lgCl vLit lLit pLit hMix fsub fdiv` have no model equivalent → import `normal` + note |
| `pass` (group pass-through) | group imports blend `normal` | pass-through ≠ compose's group-blend semantics (compose flattens the group, then blends — pass-through blends each child into the backdrop directly); noted |
| Groups (divider 3 below, `lsct` 1/2 header above) | model `Layer` whose cell holds one `GroupObject` with the member objects as children (nested groups nest); the header's name/blend/opacity/visibility wins | a header with no divider, or a divider never closed, is noted and skipped |
| Unicode names (`luni`) | `luni` wins over the pascal name | none |
| Layer masks (channel −2) | **dropped** | "layer mask imports unmasked" — compose has no per-object mask yet (parity with the SVG clip story) |
| Clipping (u8 flag) | **dropped** | "clipping to the layer below is dropped" — no model clipping pair |
| Text layers (`TySh`) | **raster + note** | "text layer imports as raster pixels" (per the ticket's recorded limitation) |
| Adjustment layers | **skipped + note** | no channel pixels of their own; the merged composite (below) already carries their effect |
| Merged composite (raw + RLE; RLE = one pooled byte-count table) | parsed always; when a file has **no layer records** it becomes the document (single `IMGv`, like the raster import path) | layer-carrying files use layers; the composite is validated for length but not kept |
| Non-RGB modes (Gray/CMYK/Lab/Indexed…) | refuse with the mode named | deliberate: approximation would be silent falsification |

### Provenance

- **Hand-built fixtures** (house style): the reader's unit battery
  builds PSD byte streams with a small in-test writer — raw and
  PackBits channels, a group pair with `lsct`/`luni`, opacity 128,
  hidden flag, the merged composite, and every refuse gate.
- **ag-psd oracle**: the grouped sample (`backdrop` + group(`inner-a`
  multiply, `inner-b` screen, `ghost` hidden) + `topmost` overlay @
  128) must parse to the expected tree.
- **ImageMagick corpus**: `-depth 8` samples parse (raw and RLE
  composites, flat and layered); the 16-bit default refuses with its
  reason; ZIP-compressed channels refuse with the compression named;
  the reversed-key sample imports with the unknown-key note. The RLE
  composite sample is what exposed the pooled-table layout above.
- **Fuzz arm**: `psd_read` anchor (decode → best-effort render must
  not panic) over the seeded corpus, plus the deterministic
  mutated-byte battery in the io tests.

## Consequences

- PSD import is honest by construction: every feature either maps
  exactly, flattens with a recorded note, or refuses with a reason —
  the checklist above is the gate's contract, and task 6 (writer)
  must satisfy it in reverse (write → read equality on the same
  rows).
- The IM reversed-key finding means blend-key parsing is
  fault-tolerant by policy; tightening it would break on real
  writers.
- ZIP-compressed layers stay unimportable until a real file demands
  them; the refusal names the compression so the gap is loud.

## Task 6: the writer, in reverse (same rows, opposite direction)

The writer (`io::psd::encode`) satisfies the checklist read backwards
— write → read equality through our own reader on the same rows, plus
the host oracles:

| Model feature | We write | Note |
|---|---|---|
| Model layers, bottom-first | one raster record each, in `layers` order | record order == render order |
| Image objects (`IMGv`) | pixels extracted from the tile bank at the object's bounds (premultiplied → straight), record rect at the object's location | lossless; requires `tile_size` 128 and in-bank tiles — anything else refuses |
| Path/Text/Slice/Symbol objects | flattened to a full-canvas raster through a single-object probe render (compose) | the flatten bakes the object's opacity into its pixels (alpha against the empty backdrop), so the record writes **opaque** and keeps only the blend key live — writing both would dim twice on read-back |
| Groups | the pair: divider (`lsct` 3, pascal `</Layer group>`, **flags hidden**) below, header (`lsct` 1) above with the group's name/blend/opacity/visibility | divider hidden so other apps never show an empty layer; nesting depth capped at 64, records capped at 32 767 |
| Blend modes | the 16 `blend_value`-invertible keys as `8BIM` 4-byte tags | the writer's table is exactly the reader's inverse; unknown model blends would write `norm` (unreachable today) |
| Opacity per-mille | u8 `(pm * 255 + 500) / 1000` | round trip ±2‰ (700 → 179 → 702, pinned by test) |
| Visibility | flags bit 1 on the record; layer `display` | hidden layers stay hidden |
| Names | pascal (≤255, 4-padded) + `luni` Unicode block | our reader prefers `luni`, as does ag-psd |
| Merged composite | always written (maximize compatibility): 3-channel RGB, RLE, **one pooled byte-count table**, and **no u32 length prefix** | the spec'd prefix made PIL fail the composite by exactly 4 bytes; ag-psd, ImageMagick, and PIL all expect it bare — the reader accepts both forms |
| Clipping/masks/effects | never written | no model pair — the same gap list as import, in reverse |

Oracle report (devlog, 2026-10-11): the scripted 64×64 sample (wash
bitmap + orange square path + group of two dots + hidden ghost) opens
in ImageMagick (composite + all 5 raster frames, geometry and pixels
exact — wash `(40,60,200)`, dot-a `(255,255,0)`, square `(255,136,0)`
in and out of the composite) and in PIL (all 7 records, names,
bboxes, PackBits). Known gap carried from M7: compose renders image
objects as nothing, so the **composite** (and any flattened record)
omits image objects — the layered records carry them losslessly from
the tile bank; the flat preview in other apps shows only the vector
content until the compose tile path lands.

## References

- `docs/planning/firecraft/M8-import-export.md` task 5 (stopping condition)
- PLANNING §7.6 (ag-psd as the MIT oracle; PSD from the public spec)
- `docs/decisions/0008-svg-filter-mapping.md` (the same
  record-or-refuse import discipline, on the SVG side)
