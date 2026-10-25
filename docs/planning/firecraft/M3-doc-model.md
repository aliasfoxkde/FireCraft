# M3 — `doc` model + format mapping + round-trip writer

> **CLOSED milestone (historical reference).** See docs/PROGRESS.md for the closure date and docs/HANDOFF.md for current work.

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** a typed document model (pages/states/layers/objects) fully
mapped to the Fireworks node tree, with lossless write-back — including
preservation of everything not yet modeled.

**Inputs:** M2 generic node tree; vendored node dictionary (`MKBv`, `PDCv`,
`LAYv`/`LSLv`, `GRPv`, `PTHv`/`PBPv`/`CPTv`, `RCTv`/`PRIv`, `TXTv`/`TFSv`,
`MSKv`, `GDSv`, `EXPv`, `FGLv`, `NGSv`, …); corpus; photocraft §8
compile-fail trick.

**Authority:** ROADMAP M3 gate. Model shape follows CS6 semantics (PLANNING
§3), not convenience — parity target.

## Tasks (atomic)

1. **`doc` core** — flat node registry (ids, not recursive structs) +
   derived indexes: layer tree, page→states→layers→objects ordering,
   cached bounds. Types: Document, Page, State, Layer, Object variants
   (Path, Primitive, Text, Group, Mask, SymbolInstance, Image),
   Fill/Stroke/Gradient/Effect parameter structs, Guides, ExportSettings.
   **Stopping condition:** M2 corpus docs convert into the model without
   loss of node coverage (coverage report, not just "compiles").
2. **Typed mapping (read)** — node tree ⇄ model for the corpus-covered
   subset; color-int layouts (RGBA/ABGR/ARGB-by-context) resolved from
   corpus evidence and recorded in `docs/research/color-layouts.md`;
   string/color/filter encodings mapped per dictionary + verification.
3. **Unknown bag** — unrecognized nodes/fields/attrs retained per parent,
   re-emitted verbatim on write (ordered). Test: mutate a modeled field,
   write, verify unknowns unchanged byte-for-byte.
4. **Typed mapping (write)** — model → node tree; canonical emission
   matching CS6 byte layout (field order matters — derive from corpus).
5. **Compile-fail completeness** — a mapping trait/table that fails to
   compile when a `doc` struct gains a field without a format mapping
   (photocraft trick); demonstrated by a `compile_fail` doctest or
   xtask check.
6. **Round-trip floors rise** — write(parse(x)) byte-identity floor
   above M2's echo-only floor; `xtask corpus --report` per-file status;
   fuzz the typed layer (malformed trees → typed errors, no panics).
7. **wasm + layers** — `doc` L1 depends only on L0 (`xtask layers`
   enforces); wasm build green.

**Verification (milestone gate):** corpus round-trip floor met; unknown
preservation proven; coverage report shows corpus node kinds mapped or
bagged (none dropped); compile-fail demonstrated; `xtask ci` green.

**Out of scope:** rendering (M4), editing semantics/undo (M5), Auto Shape
scripting (M6).

---

## Addendum: structural spine + mapping decisions (2026-10-10)

From a structural reading of the committed research dictionary
(`docs/research/fireworks-png-nodes.md`) and M2 parser behavior. Our own
description; the dictionary is the attributed reference.

### What the tree actually nests

```
Root
├── MKBv  Document node (shared definitions + reduced page-1 copy)
│   └── LYLv  document-level layer list (master/shared content)
└── PDCv* Pages — one per page, same shape as the document node

Page/Document body: VRSi/VRMi version, FRCi frame count (x2 on pages),
canvas (XLCf YLCf WIDf HITf RESf RSUi BGCi), definition lists (PLLv
brushes, FLDv fills, GLDv gradients, EFDv effects, PATv patterns),
EXPv export options, MSLv symbol definitions, LYLv layers, GRDv grid,
GDSv guides, onion-skinning (OSBi/OSAi).

LYLv → LAYv+ layers; LSAv* shared-layer links; MPLv master-page link.
LAYv → BKGb background, LNMs name, DISb display, MSLb special,
       MCLb frame-changing, per-frame content in CLLv (cell list)
       → CELv one cell per frame/state → ELMv elements.

ELMv (object union): PTHv* path · TXTv* text · GRPv* group ·
LSLv? sub-layer · URLv* slice/hotspot · IMGv* image · ALSv* symbol
instance. (Order inside ELMv is the z-order within the cell.)

PTHv  → PBLv bezier source list (PBPv: ISCb closed, PPCi count,
        PBTv* points) + PCLv derived contour lists + optional PRIv
        primitive source (MTXv 3x3 matrix + RCTv rect + RDSf radius)
        + style plumbing (PATv pattern, BRPi brush placement, fill
        points PS/PE/PF, EOFb even-odd).
GRPv  → GRTi type, ELMv children, OPAi opacity (0..1000), BLDi blend,
        MNAb mask-not-appear (mask group marker), OBNs name.
IMGv / MSKv → TILv tile-id list → mkBT tiles; WPXi/HPXi/TSZi geometry.
ALSv  → symbol instance referencing MSTv by MSNi.
MSTv  → symbol definition (page-like: LYLv, canvas fields, MSNi id).
GDSv  → GDHv/GDVv guide position lists, GDLb locked, GDKi colour.
TXTv  → bounds (LFTf..BOTf) + TFSv rich-text runs (deep engine: M6).
```

### Mapping decisions

1. **Mapping crate placement.** The model⇄tree mapping lives in the
   `doc` crate (L1), which depends on `fwpng` (L0). `fwpng` stays a
   zero-workspace-dep standalone crate (M2 gate). The roadmap's "fwpng
   typed layer" names the *concern*; `doc::fwmap` is where it lands.
2. **Registry.** Flat, document-wide object registry (generational
   slots, Graphite-style). Structural lists (pages → layers → cells →
   elements, group children) store `ObjectId`s; z-order is list order;
   parent/child edges are derivable.
3. **Unknown preservation.** Anything outside the mapped spine is kept
   verbatim in a sidecar bag keyed by structural path (parent FourCC +
   child index), re-emitted at write time. Lossless round-trip never
   waits on semantics (PLANNING §6 strategy (b)). Task-3 test: mutate a
   modeled field, rewrite, unknowns byte-identical.
4. **Compile-fail completeness.** The writer constructs every model
   struct with a full struct literal (no `Default`/`..rest`); every
   reader match over object kinds has no wildcard arm. A new field or
   kind without a mapping breaks the build together with the
   all-fields round-trip test.
5. **States = frames.** The page's frame count indexes the per-layer
   cell lists: cell i of every layer belongs to state i. M3 stores
   cells on layers; the doc-level state list is derived.
6. **Text and effects are scaffolds in M3.** TXTv maps bounds; the
   TFSv run subtree is preserved verbatim against the object. EFLv
   effect lists are preserved, not interpreted (deep text = M6,
   filters = M7).
7. **Opacity/blend scale.** OPAi is 0..1000 (1000 = opaque); blend
   modes are ints in the M1 mode table.
8. **mkBS.** The legacy chunk carries MKBv only (no PDCv). The reader
   uses the same document mapper; the corpus battery round-trips both
   trees canonically.
9. **Color-int layouts** (RGBA/ABGR/ARGB-by-context) are NOT resolved
   in M3 — no real-file corpus exists on this host yet. Colors
   encountered on the mapped spine stay in the unknown bag; task 2's
   `docs/research/color-layouts.md` lands when corpus evidence exists.

