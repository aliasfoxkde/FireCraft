# M1 — L0 foundations + scripting-engine spike

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** geometry, color/blending, and tiled rasters solid enough to
build the document model on; the wasm-compatible scripting engine chosen by
experiment, not taste.

**Inputs:** PLANNING §7.3/§7.6 (stack table), CS6 reference material for
blend-mode ground truth (27 modes; CS6-exported swatch docs via corpus or
Wine harness), shipped `.jsf` files in the local tree for the spike.

**Authority:** ROADMAP M1 gate; deviations → `docs/decisions/`.

## Tasks (atomic)

1. **`geom`** — affine transform type (kurbo-based), bézier path wrapper,
   stroke-outline request (delegate to kurbo), transform composition,
   bounding boxes, point/rect hit-test helpers. Property tests (inverse
   round-trip, associativity); fuzz anchor: transform from random f64
   fields must not panic. **Stopping condition:** API used by a doc-level
   smoke test compiles.
2. **`color`** — pixel format enum (rgba8/rgba16/rgba32, premultiplied
   variants), premultiply/unpremultiply, sRGB↔linear, **27 blend modes as
   pure functions** with one test per mode against reference values
   (source: W3C compositing spec for the standard 16 + measured CS6
   swatches for Fireworks-specific ones like Invert/Tint/Erase — record
   provenance per test). Allocation-free hot paths; no panics (checked
   conversions).
3. **`raster`** — COW tile store, 128×128 tiles, `Arc`-based clone-on-write,
   iteration over dirty tiles, allocation caps on width/height derived
   from input (hostile-input rule). Tests: COW semantics (writer doesn't
   mutate shared tiles), cap enforcement returns errors not panics.
4. **Wine corpus harness (optional-but-first)** — script to launch the
   portable CS6 under Wine, run a generated `.jsf` that draws deterministic
   swatch/tile documents, save `.fw.png` + flattened exports. If Wine
   can't run CS6 on this host, record the blocker in devlog and fall back
   to public sample files for M2. **Stopping condition:** either ≥3
   generated corpus files with sha256 manifest, or a documented fallback.
5. **Scripting spike** — two throwaway bench crates (not workspace members)
   embedding `rquickjs` and `boa_engine`: load a real shipped `.jsf` from
   the local tree, provide a trivial `fw` host object, execute
   `InsertSmartShapeAt` handlers, measure binary size and wasm32 build
   feasibility (QuickJS-NG via cc to wasm; boa native wasm). Write
   `docs/decisions/0001-scripting-engine.md` with the choice + evidence.
   **Stopping condition:** decision doc merged; loser crate deleted.
6. **Fuzz anchors wired** — `cargo-fuzz` targets for geom transforms and
   color conversions registered under `fuzz/`; `xtask ci --fast` skips
   long fuzz, full run executes 60s each.

**Verification (milestone gate):** `xtask ci` green incl. wasm; blend-mode
test table complete (27 rows); spike decision doc merged; corpus harness
produced files or documented fallback; `cargo bench` baseline recorded.

**Out of scope:** document model (M3), any PNG parsing (M2), GPU (M4+).
