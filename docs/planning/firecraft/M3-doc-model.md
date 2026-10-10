# M3 — `doc` model + format mapping + round-trip writer

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
