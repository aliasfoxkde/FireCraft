# 0005 — Live-effect rendering: identity, margins, and the content cache

- **Status:** Accepted (M7 task 4, 2026-10-11)
- **Deciders:** Michael Kinney, Claude
- **Scope:** `compose` (L3) — how an object's `EFLv` stack becomes
  pixels; the `filters` bridge, the effect cache, and the golden
  fixtures that pin it.
- **Supersedes:** none · **Superseded by:** none

## Context

M7 task 4 wires live filters into composition (PLANNING §7.3: "live
filters compose lazily with caching keyed on content hashes"). Three
policy questions had to be settled before any code: which identity
selects a kernel when a stack names an effect, how a filter's spread
past the object's geometry is bounded, and what a cache entry may
safely be keyed on.

## Decision

### 1. Identity: `EffectMoaID` GUID primary, `INMs` name fallback

The dictionary's `DCEv "EffectMoaID"` table (lines 1486–1565) is the
authoritative effect identity; the interpreter matches the GUID with
braces and case normalized. Stacks without a GUID — synthetic
`filter.add` payloads, hand-written files — fall back to the internal
name, lowercased alphanumerics only. A GUID that is present but
unknown is **never guessed at**: the effect is skipped (counted), the
object renders unfiltered through it. Totality beats coverage — a
shadow we cannot draw must not cost the whole render.

### 2. Uninterpretable ≠ failed

Missing/unparseable required parameters, unknown identities, hidden
effects (`EffectIsVisible` false), and kernel gaps (unsharp with a
non-zero threshold, per-channel levels, curves with raw grab points)
all skip that one effect. `render` still succeeds. The count rides in
the interpreter's `Plan::skipped` for diagnostics; nothing in the
compose contract reports filter gaps as errors.

### 3. Margins: full-canvas offscreen, spread bounded by the canvas

A filtered object rasterizes **alone into a full-canvas offscreen
context** — not a bounding-box with margin. A blur's spread past the
geometry therefore survives for free, and the "margin" question
collapses to the kernel's clamp-to-edge behavior at the canvas border
(decision 0004). Cost: one canvas-sized render + kernel pass per
filtered object per cache miss; the cache makes that once per content,
not once per frame. A bbox-scoped offscreen with explicit margin
bookkeeping is the recorded future optimization if profiling demands
it.

The filtered raster blits back as a 1:1 canvas-sized image paint
(`ImageSource::Pixmap`, nearest sampling) **inside the object's
existing opacity/blend layer**, so the filtered and unfiltered paths
differ only by the filter math — one uniform composite mechanism.

### 4. Cache keys: content for paths, pixels for groups

- **Paths** key on rendering content: geometry (anchors/controls as
  bit patterns), paints, even-odd, fill-over-stroke, the kernel plan,
  and the canvas size. Blend/opacity are excluded — they wrap the
  filtered image and never touch the cached bytes, so an opacity scrub
  must not re-filter. A hit skips both the offscreen render and the
  kernels.
- **Groups** key on the already-rendered base pixels: children can be
  edited under a stable id, so ids are not a content identity. The
  offscreen render is unavoidable for groups anyway (it *is* the filter
  input); only the kernel pass is saved.
- Hash is FNV-1a 64 over a versioned domain tag. Not cryptographic;
  the failure mode of a collision is one stale frame, not corruption,
  and real sessions hold orders of magnitude fewer than 2⁶⁴ distinct
  filtered rasters. Entries cap at 4096; overflow clears the map.

### 5. Add Noise determinism without payloads

`add_noise` needs a seed but the `EPSv` property set carries none.
The seed derives by FNV-1a over the effect's own bytes (internal name,
then every property key/value; raw node values contribute their
fourcc). Same stack → same noise, across renders, platforms, and
journal replays — no new payload slot, no clocks.

### 6. Golden fixture

`corpus/fixtures/filtered.fw.png` carries one shape per interpreted
kernel family (gaussian blur, brightness lift, invert) plus a
dropShadow whose GUID is beyond the catalog — the same image pins
filtering and unfiltered pass-through. Stack-order proofs live in
`crates/compose/tests/compose_proof.rs` (A∘B ≠ B∘A pinned by value;
commuting point ops pinned byte-equal).

## Consequences

- The interpreter catalog (10 effects) is additive: widening it never
  changes existing pins because every effect is skipped-not-guessed.
- CS6 similarity for interpreted effects is still report-only until
  real CS6 exports land (M7 scope line); the enforced floors are
  self-pins (determinism, LUT arithmetic, order properties) — honest
  by construction.
- Nested stacks (a child filtered inside a filtered group) compose by
  construction; the nested-inverts-cancel proof pins the identity.
