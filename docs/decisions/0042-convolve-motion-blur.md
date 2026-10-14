# Decision 0042 — `convolveSelection` / `motionBlurSelection` (P rows → C)

**CS6 surface.** Two pixel-filter dom verbs the matrix classed P (via
`filter.add` carriers):

- `dom.convolveSelection(kernelWidth, kernelHeight, kernelValues,
  affectsAlpha)` — "applies the convolution filter defined by the
  values to the selection. `kernelValues`: An array of integers that
  defines the values for specific filter patterns."
- `dom.motionBlurSelection(blurType, angle, distance, samples)` —
  "`blurType`: linear/radial/zoom; `angle`: degrees, 0–359;
  `distance`: 0–400; `samples`: the number of times the original
  image is cloned and blurred to produce the desired effect."

## Design

- **Both verbs ride the 0040 bake core unchanged.** They build a
  one-op `compose::Plan` directly (no `Effect` synthesis — these are
  parameter forms, not catalog identities) and call `bake_core`:
  empty-selection refuse, per-image pre-flight, pixel-mask gating,
  `Mutated` + `Bank` deltas, byte-exact undo.
- **`KernelOp::Convolve` is the generic kernel** behind the existing
  presets: edge-clamped sampling, anchor at `((w−1)/2, (h−1)/2)`,
  raw `i64` accumulation, house `clamp255`. **No normalization** —
  the dom passes raw integer coefficients ("values for specific
  filter patterns"), so their scale is the caller's contract; a
  zero-sum kernel is legal and produces black. `affectsAlpha`
  extends the pass to the alpha channel (default: RGB only).
  `convolve3` now delegates to it (1-bit behavior change: none —
  the same math).
- **`samples` is the tap count.** `filters::motion_blur` gains the
  companion `motion_blur_taps(surface, angle, distance, taps)`;
  taps spread evenly over `0..=distance` pixels. With the effect
  carrier's `taps = distance + 1` the step is exactly one pixel —
  Live Effect output is byte-identical (the compose GUID constructor
  sets `taps: distance + 1`). `samples ≤ 1` (or `distance = 0`) puts
  every tap on the pixel: an exact copy.
- **`blurType` mapping** (CS6 gives no kernel math; these pin the
  substitutions): `linear` → `MotionBlur { angle, distance, taps:
  samples }`; `radial` → `RadialBlur { amount: distance, quality:
  samples }` (distance reads as sweep degrees); `zoom` → `ZoomBlur {
  amount: distance, quality: samples }` (distance reads as pinch
  percent). The kernels' own carrier-range clamps apply (sweep caps
  360, pinch caps 100, quality clamps 1..=100) — the engine only
  gates the dom-documented ranges: angle 0..=360 finite, distance
  0..=400, samples 0..=100, `blurType` one of the three (bogus
  refuses). Radial/zoom are center-based, so `angle` stands unused
  there.
- **Kernel sanity gates** (`convolveSelection`): sides 1..=31 per
  side (a 31×31 kernel already samples a 15-px radius, far past the
  UI's 7×7 — bigger is a payload bug), and `kernelValues.len() ==
  width × height`. Zero sides, oversize, count mismatch, and
  dimension overflow all refuse `InvalidState` before any mutation.

## Consequences

- Registry 199 → **201**; parity 226 C / 45 P / 0 M / 85 N → **228 C
  / 43 P / 0 M / 85 N**.
- The two verbs stop depending on the `filter.add` carrier note and
  carry themselves; the remaining P rows are structure/render debts
  (rasterizer, duplicate, patterns, mkBT writer, …), not pixel
  filters.
- Deviations to revisit on oracle evidence: distance→degrees/percent
  mapping for radial/zoom; the dom's angle 0–359 documented range is
  admitted up to 360 (fractional angles round in the kernel).
