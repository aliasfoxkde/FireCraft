# M4 — CPU compositor

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** render any document the model can express to pixels,
deterministically — same input → byte-identical output — as the
reference oracle every later renderer (GPU, export preview) must match.

**Inputs:** M3 `doc` model + paint spine (task 2 below); `color` blend
modes (M1); `raster` tile store (M1); rasterizer decision
[0002](../../decisions/0002-cpu-rasterizer.md) (`vello_cpu 0.3`).

**Authority:** ROADMAP M4 gate. CS6's own flattened export is the parity
target; until real exports exist on this host, self-generated goldens
pin determinism and CS6 floors stay `enforce = false`.

## Tasks (atomic)

1. **Paint spine in `doc`** — model what PTHv carries for rendering:
   `PATv` (`BCLi` brush colour ABGR, `FCLi` fill colour ABGR, `FPLv`
   fill-pattern subtree — solid mapped, gradient/pattern bagged), the
   gradient fill points (`PSXf`/`PSYf`/`PEXf`/`PEYf`/`PFXf`/`PFYf`),
   `FOTb` fill-on-top, `BRPi` brush placement. Reader bags what it does
   not interpret (unchanged policy); writer re-emits mapped parts
   canonically; the exhaustive-match compile-fail gate keeps holding.
   **Stopping condition:** sample + corpus round-trip floors still green
   with paints modeled.
2. **`compose` crate (L3)** — page canvas from `PageInfo`, frame
   (state) selection, layer walk honoring `display`/frame-specific
   visibility, cell visibility per frame, object render in z-order:
   contours → vello paths (even-odd rule from `EOFb`), solid fills and
   strokes with per-mille opacity (`OPAi`) via layers, blend modes from
   the model via peniko. `unsafe_code = forbid`, workspace lints.
   **Stopping condition:** a document with painted paths on two layers
   with different opacities composes to the hand-computed pixels.
3. **Determinism floor** — repeat renders of the same document are
   byte-identical (hash-pinned); render the corpus documents and pin
   their output hashes as self-goldens in `corpus/fixtures/`.
4. **Golden machinery** — `image-compare`-based harness + xtask wiring
   for CS6-export comparison; floors recorded but `enforce = false`
   until real exports land in `corpus/incoming/` (recorded limitation,
   same posture as M2's provisional byte-layout items).
5. **Bench baseline** — criterion bench of compose over the sample
   document; numbers recorded in the devlog as the M4 baseline.
6. **Gates** — `xtask layers` (compose L3), `xtask wasm` (compose in
   the wasm-clean list — vello_cpu verified wasm-clean by the spike),
   corpus battery green (paint spine extension), `ci --full` green.

**Verification (milestone gate):** determinism proven (byte-identical
repeat renders + pinned goldens); hand-computed composition tests pass;
benches recorded; `xtask ci` green. CS6 golden-image floors recorded,
enforcement deferred until exports exist.

**Out of scope:** gradients as paints (pattern/texture/gradient fills
stay bagged; solid fill + gradient geometry this milestone), filters and
live effects (M7), masks as pixel content (M6/M7 — group mask flags are
honored structurally), text layout (M6), GPU path (later).
