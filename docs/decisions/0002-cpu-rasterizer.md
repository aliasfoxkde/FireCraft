# 0002 — CPU rasterizer: `vello_cpu`

- **Status:** Accepted (M4 spike, 2026-10-10)
- **Deciders:** Michael Kinney, Claude
- **Scope:** the `compose` crate (L3 deterministic render oracle) and any
  golden-image verification built on it.
- **Supersedes:** none · **Superseded by:** none

## Context

PLANNING §7.3 pins the render strategy: CPU first, GPU second, always
identical. `compose` must be deterministic (same input → byte-identical
pixels — it is the oracle every test and the golden-image suite compare
against), pure Rust (the whole L0–L3 stack builds for
`wasm32-unknown-unknown`), and cover the Fireworks surface: even-odd
fills, strokes, per-mille opacity layers, the documented blend modes,
gradients, clip layers, and (later M7) filter kernels.

Candidates: `vello_cpu 0.3` (Linebender CPU renderer, SIMD, shares the
peniko/vello_common paint model) and `tiny-skia` (mature, but a smaller
blend-mode subset and a different paint model we would have to extend).

## Spike

Throwaway crate outside the workspace (`/var/tmp`, deleted after): with
`default-features = false, features = ["std", "u8_pipeline"]`,
rendered a closed triangular path through `RenderContext::fill_path`
under `set_transform` + `Fill::EvenOdd` + a solid peniko `Brush`, then
`flush()` + `render()` into a `Pixmap` and asserted the center pixel.
Native run produced the expected premultiplied RGBA8;
`cargo check --target wasm32-unknown-unknown` passed.

## Decision

**Adopt `vello_cpu 0.3`** with default features off (`std`,
`u8_pipeline`). Rationale:

- Fill/stroke/transform/fill-rule/opacity-layer/blend-mode/gradient API
  maps 1:1 onto the Fireworks compose pipeline (incl. `push_layer` with
  clip + blend + opacity — exactly layer/cell/group semantics).
- peniko `Brush`/`BlendMode` covers the 16 W3C mix modes (verified
  against peniko 0.6.1 during the compose build — *not* the 27-mode
  family this ADR originally claimed; corrected 2026-10-10). Fireworks
  `BLDi` values with a peniko formula map directly (multiply,
  difference, overlay, screen, darken, lighten, hue, saturation, color,
  luminosity, exclusion, hard-light, soft-light, color-dodge,
  color-burn); the rest — dissolve, the extended burn/light family,
  invert, erase, tint, average, negation, reflect — fall back to Normal
  in `compose::convert::blend_mode` until M7 adds compositing passes
  (our own M1 `color` crate already implements the full 27-mode set and
  remains the reference for those passes).
- Pure Rust, wasm-clean (verified by the spike), no C FFI.

## Consequences

- `vello_cpu` re-exports its own `kurbo` (0.13) and `peniko`; the
  workspace `geom` builds on kurbo 0.11, so `compose` converts paths by
  re-emitting path elements through `vello_cpu::kurbo::BezPath` at the
  boundary. The conversion is pure and unit-tested.
- `multithreading` stays off: single-threaded execution keeps renders
  deterministic by construction (the oracle's core property).
- `png`/`text` features stay off: decode is `fwpng`'s job; text layout
  arrives with M6 via its own stack.
- `Filter` support exists in `push_layer` (M7 will use it); M4 uses only
  clip/blend/opacity layers.
