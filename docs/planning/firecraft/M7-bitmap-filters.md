# M7 — Bitmap + full live filters

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** the pixel half of the editor: bitmap-object editing
(selections with feather, retouch tools), the full live-filter catalog
as deterministic CPU kernels with the `EFDv`/`EFLv` document mapping,
and the text engine (`TXTv` runs, full attributes, text-on-path) — all
through the M5 command registry, so undo, journal replay, the CLI and
the fuzzers get every new operation for free.

**Inputs:** M3 `doc` model (IMGv bitmap objects, TXTv frame+transform
with runs bagged, EFLv currently bagged); M4 `compose`/`raster` (tiled
bitmap store, golden machinery); `color` (premultiply, blend modes);
the node dictionary's effect sections (`EFDv`/`EFLv`/`EPSv`/`DCEv` —
observational, `EPSv` parameter encodings flagged inferred); stack
decisions PLANNING §7.3/§7.6 (`filters` L2 CPU kernels, Graphite
`filter.rs` pattern, resvg as SVG-effects reference; `text` L2 on
cosmic-text 0.19); CS6 `Configuration/` `.jsf` specs for text and
filter behavior (read in scratch, never committed).

**Authority:** ROADMAP M7 gate: *per-filter golden images vs CS6 at
recorded tolerances; filter stack composes in any order with correct
document-model round-trip.*

## Scope line: parity is corpus-bounded

There is no CS6 oracle on this machine yet (`corpus/incoming/` is
empty): per-filter CS6 similarity stays report-only (the M4 golden
machinery's `cs6/` sidecar path) until real exports land, and the
*enforced* floors are the ones we can pin honestly — deterministic
kernels, integer-exact point ops, golden pins of our own renders, and
format round-trip. Recorded-vs-enforced is stated per task; nothing
"passes vs CS6" that was only compared against itself.

## Tasks (atomic)

1. **`filters` crate (L2) + kernel spike** — new crate: point and
   separable kernels over `color` buffers, gamma-aware premultiply
   order decided by evidence and recorded in
   `docs/decisions/0004-filter-color-order.md`. Catalog, deterministic
   first: brightness/contrast, levels, curves, invert, hue/saturation/
   lightness, posterize, threshold; blur (box/gaussian), sharpen/
   unsharp, find edges, emboss/bevel-style convolutions; seeded noise
   (explicit seed in every payload — replay must be bit-stable).
   **Stopping condition:** integer-exact proofs for the point ops
   (M4-compose discipline); determinism proofs for all kernels (same
   input bytes → same output bytes, every run); property tests
   (idempotence where the op claims it, e.g. invert twice).
2. **Live-filter document model + `EFLv` mapping** — `doc` gains a
   per-object effect list (the `EFLv` shape: category, internal name,
   `EPSv` properties); parameters the model can't yet interpret stay
   bagged so round-trip never waits on semantics (M3 rule). Differential
   check against png-analyser output on any corpus file that carries
   `EFLv` (corpus-gated: synthetic fixtures until real files land).
   **Stopping condition:** read→write→read equality on synthetic
   effect fixtures; unknown-parameter preservation byte-proven.
3. **Registry commands + deltas** — `filter.add`, `filter.remove`,
   `filter.setParam`, `bitmap.applyLUT`-style instant ops, each
   capturing the prior state in the delta. `panic_hunt` grows the new
   commands. **Stopping condition:** fuzz campaign clean; undo/redo +
   journal replay equivalence byte-compares over filter sequences
   (the M5 harness, now over effects).
   *Sequencing note (2026-10-11):* the `bitmap.*` instant ops land
   with task 5 — they need the `IMGv`↔tile-store pixel plumbing that
   task builds; task 3 ships the effect-list commands and arms them in
   `fuzzdrive`.
4. **Compose integration: lazy stack application** — live filters
   compose in any order under the content-hash cache (PLANNING §7.3);
   filter bounds/margin handling recorded. **Stopping condition:**
   golden pins for every filter on pinned shapes; stack-order tests
   (A∘B ≠ B∘A where the math says so, equal where it doesn't).
   *Done (2026-10-11):* `compose::effects` interprets 10 effects into
   the kernel catalog (identity: `EffectMoaID` GUID primary, `INMs`
   name fallback; unknown/hidden/missing-param → skip-and-count);
   content-hash `EffectCache` (paths key on rendering content, groups
   on base pixels); full-canvas offscreen + 1:1 image blit inside the
   existing opacity/blend layer. Stack-order proofs pin both
   directions (95 vs 159) and the commuting case byte-equal; golden
   `filtered.fw.png` pins three kernel families plus an unknown-GUID
   pass-through. Policy in `docs/decisions/0005-live-effect-render.md`.
5. **Bitmap objects + selections** — `IMGv` pixel editing through the
   `raster` tile store: marquee/lasso/wand → `Mask` (a raster alpha
   plane), feather, and the masked ops (fill, erase, move). Registry
   commands with mask-capturing deltas. **Stopping condition:**
   table-driven mask tests (feather falloff, wand tolerance);
   undo byte-exact; journal replay equivalence.
   Done (2026-10-11): `Document.tiles` (TileBank) + `Document.selection`
   (SelectionMask) in `doc`; `Delta::Tiles`/`Delta::Selection`; the
   `engine::mask` algorithms (center-rule rect/ellipse/polygon, per-channel
   tolerance wand, 3×box feather with a hand-pinned falloff table) and
   `engine::bitmap` gather/scatter (premultiplied tiles ↔ straight flat
   buffer, 4096² cap, matte slots keep their grid position, missing
   payloads refuse). Nine `bitmap.*` registry commands, armed in fuzzdrive.
   Replay/undo equivalence in `tests/bitmap_journal.rs` (3 tests);
   `Scratch` hoisted into `tests/support` so replay starts from a
   tile-loaded base.
6. **Retouch tools** — deterministic retouch over masks: blur tool,
   smudge-lite, clone (aligned), eraser, red-eye. Every tool takes
   explicit geometry (no pointer state in the engine). **Stopping
   condition:** golden pins with fixed inputs; undo byte-exact.
7. **`text` crate (L2): layout + `TXTv` runs** — cosmic-text 0.19
   layout over the `TXTv` frame; `TFSv` run attributes mapped into the
   model (font, size, fill, kerning/tracking/leading/baseline shift,
   anti-alias preset); auto-size vs fixed-width frames; a bundled OFL
   font (attribution row in the same change — asset rule). Commands:
   `text.createText`, `text.setText`, `text.setAttrs`, `text.setFont`.
   **Stopping condition:** layout goldens (pinned font + pinned string
   → pinned render); attribute round-trip through the format;
   wasm-clean (`text` joins `WASM_CRATES`).
8. **Text-on-path** — text laid along a `PTHv` contour, orientation
   and baseline behavior pinned by tests (Fireworks' recorded behavior
   from the `.jsf`/help specs). **Stopping condition:** golden pins;
   round-trip.
9. **Surfaces + gates** — shell: filter panel (live effect list),
   text tool + text properties, selection tool group; `filters` and
   `text` added to `WASM_CRATES`; parity floors raised where the gate
   earned them (per-filter golden floor); fuzz arms for the new
   commands. **Stopping condition:** `xtask ci` green; scorecard M7 →
   done with gate evidence in `log/devlog.md`.

## Non-goals (M7)

GPU rendering (later milestone; compose stays the oracle), PSD/SVG and
raster export formats (M8), scripted execution (M9), symbols (M10).
