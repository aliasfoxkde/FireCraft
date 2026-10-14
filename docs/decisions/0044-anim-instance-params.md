# Decision 0044 — `symbol.setAnimInstance*` (P rows → C)

**CS6 surface.** Seven animation-parameter dom verbs over symbol
instances:

- `dom.setAnimInstanceNumFrames(numFrames)` (FW4) — "the number of
  frames through which the symbol animates".
- `dom.setAnimInstanceLoopCount(loopCount)` (FW3, deprecated in 4
  "in favor of `dom.setAnimInstanceNumFrames`").
- `dom.setAnimInstanceOffsetDist(offsetDistPt)` — "a point that
  specifies the distance the animation moves in pixels"
  (`{x:100, y:25}` moves right 100, down 25).
- `dom.setAnimInstanceRotationAmount(rotationAmount)` — clockwise
  degrees across the span; 720 = two clockwise turns, negative
  counter-clockwise.
- `dom.setAnimInstanceScaleAmount(scaleAmount)` — positive percent
  (50 = half size, 200 = double, 100 = none).
- `dom.setAnimInstanceStartEndOpacity(startOpacity, endOpacity)` —
  "float values between 0 and 100".
- `dom.setAnimInstanceStartFrame(startFrame)` (FW3, deprecated in 4
  "in favor of placing the animation symbol on the frame in which it
  should start").

## Design

- **The `ANNi` family already models five of the seven.** The
  doc-model carrier (`SymbolAnimation`: `ANNi` frames, `ANXf`/`ANYf`
  shift, `ASCf` scaling, `ANRi` rotation, `ASOi`/`AEOi` opacity)
  round-trips the wire today; these verbs are its missing command
  surface. One walker (`edit_animation_instances`) serves all six:
  instances in the selection only (the verbs address "the selected
  instances of multi-frame image symbols"; other members skip), a
  plain instance **materializes** the carrier with the Objects-panel
  defaults (one frame, no shift, 100% scale, no rotation or opacity
  ramp — the `setFillEdgeMode` materialize-when-absent precedent),
  empty selection refuses, instance-less selection is `Noop`.
- **Loop count rides `ANNi`** — Adobe's own deprecation note names
  `setAnimInstanceNumFrames` as the FW4 replacement, so the FW3
  spelling writes the same anchor. Both counts refuse < 1.
- **Fractional rotation rounds.** The dom takes a float; the wire
  field `ANRi` is integral — rounding is the recorded substitution
  (720.5 → 721).
- **Opacity converts 0–100 → per-mille** (`ASOi`/`AEOi` slots store
  per-mille like every opacity in the model): `×10`, rounded.
- **`setAnimInstanceStartFrame` stays partial.** The FW4 format has
  no start-frame field (the dictionary's `ANNi` family is complete:
  ANNi/ANXf/ANYf/ASCf/ANRi/ASOi/AEOi — no loop or start-frame node),
  and inventing one would fabricate wire format. The verb refuses
  pointing at the documented FW4 substitute (place the instance on
  its start frame).
- **Gates** (all pre-validate before any capture): counts ≥ 1,
  offset finite, rotation finite, scale finite > 0, opacities finite
  within 0..=100.

## Consequences

- Registry 206 → **212**; parity 233 C / 38 P / 0 M / 85 N → **239 C
  / 32 P / 0 M / 85 N** (six flips; `setAnimInstanceStartFrame`
  stays P with an updated note).
- The animation instance is now fully command-driven: place → tween
  (0030) → tune via these verbs; the render side still ignores the
  carrier (frames render statically — recorded, no parity pressure
  until the animation renderer lands).
