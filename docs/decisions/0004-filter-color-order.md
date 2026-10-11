# 0004 — Filter color order and kernel conventions

- **Status:** Accepted (M7 kernel spike, 2026-10-11)
- **Deciders:** Michael Kinney, Claude
- **Scope:** the `filters` crate (L2) — every point op, convolution,
  and noise kernel — and the contract `compose` (L3) will bridge to in
  M7 task 4.
- **Supersedes:** none · **Superseded by:** none

## Context

A filter kernel can run on straight-alpha RGB, premultiplied RGB, or
flip per op. The choice is invisible until translucent pixels meet a
neighborhood operation: convolving straight colors bleeds black halos
out of transparent regions (the classic premultiplied-alpha bug), and
point ops like brightness genuinely want straight values (premultiplied
brightness darkens by opacity, not by intent). Fireworks' live effects
must survive undo, journal replay, and the fuzzers bit-identically, so
each kernel also needs a pinned rounding rule, not just a color order.

## Decision

**`Surface` stores premultiplied RGBA8** — `compose::Render`'s native
form — and the rules per op class are:

1. **Convolutions (box/gaussian blur, unsharp, find edges, emboss) run
   on premultiplied channels directly.** A convolution is a weighted
   average; averaging premultiplied colors is exactly what keeps colors
   valid across transparent edges. Alpha participates in blur as a
   plain channel (so a soft edge stays soft); find edges and emboss
   keep the source alpha (they detect color structure — zeroing alpha
   would hide their own output).
2. **Point ops round-trip per pixel:
   unpremultiply → op → premultiply** (`Surface::map_straight`), so
   brightness of a 50%-alpha pixel brightens its color, not its
   coverage. Cost: the stored premul u8 quantizes the straight value,
   so a point op on a translucent pixel can drift 1 LSB per pass —
   accepted, deterministic. Chained point ops on translucent pixels
   accumulate that drift; **the identity fast-path is the caller's
   job** (a filter with neutral parameters must skip the kernel, the
   way `add_noise(amount=0)` does internally). Opaque pixels are
   exact: premultiply is lossless at a=255, so the M4 integer-exact
   discipline holds where it matters most.
3. **Rounding is pinned in one place** (`div_half`): add-half for
   non-negative numerators, symmetric half-away-from-zero for signed
   ones. Accumulators are `u64` (blur) / `i64` (everything else) — no
   intermediate `f32`. Exceptions that legitimately use `f64`, and
   make **no integer-exactness claim, only determinism**:
   `levels` when `gamma ≠ 1.0`, all of `hsl`, and the gaussian's
   *radius selection* (`boxes_for_gauss` below).
4. **Gaussian = three box passes** (Kutskir's cascade). Box widths come
   from the public-domain `boxesForGauss`: ideal width
   `sqrt(12σ²/n + 1)` forced odd, `m = round(mIdeal)` boxes widened by
   2, radii `(width−1)/2`. The cascade's minimum box width is 3, so
   any `sigma > 0` blurs at least radius 1 — a no-op is `sigma <= 0`,
   an exact copy. Pinned radii: σ=2 → [2,2,1], σ=5 → [5,5,4],
   σ=8 → [8,8,7], σ=0.1 → [1,1,1].
5. **Blur sums are direct per-pixel sums over the clamp-to-edge
   window**, not a sliding window. O(w·h·r) is deliberate: a
   sliding-window rewrite changes which bytes you subtract, and any
   future one must first prove byte-equality against the direct sums
   (the determinism tests are that gate).
6. **Noise is explicit-seed SplitMix64**, stream consumed in fixed
   row-major order; monochrome mode draws one delta per pixel, color
   mode three (the stream position therefore depends on the flag —
   part of the pinned behavior). Deltas are `draw % (2·span+1) − span`.
   Noise is a point op (rule 2), so the premultiplied invariant
   survives it.
7. **Brightness/contrast is one fused LUT**, contrast first:
   `out = round((in−128)·(100+c)/100) + 128 + round(b·128/100)`,
   signed rounding applied to the contraction as a whole (splitting it
   into a difference of two roundings drifts a level).
8. **Whole-image `Surface`, not `raster`'s tile store.** Kernels need
   neighborhoods across tile borders; `raster` is page storage with
   dirty tracking, the wrong abstraction for a `&Surface -> Surface`
   kernel. `compose` bridges `Render ↔ Surface` at the L3 boundary
   (M7 task 4); `filters` does not depend on `raster`.

## Consequences

- The catalog ships with proofs, not vibes: point-op LUTs are pinned
  by exact-value tests; every kernel has a determinism test (same
  bytes in → same bytes out); idempotence is pinned where the op
  claims it (invert twice, posterize re-application).
- `posterize` range is 2..=256 with 256 the exact identity — 255
  levels mathematically *cannot* represent 256 distinct values, so
  "identity at max levels" needed the inclusive-256 range.
- Any engine/journal payload that carries noise must carry the seed
  (M7 task 3 payload rule); this doc is the reason.
- Parity to CS6's exact filter math remains corpus-bounded (M7 ticket
  scope line): these pins define *our* determinism; when a CS6 oracle
  lands, per-filter similarity is measured against these same kernels,
  not redefined.
