# M14 — Live-filter parity

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** take the live-effect stack from "works" to "matches the
original": every CS6 live-filter family at parameter-exact behavior.
M7 built the kernel crate and a working subset (brightness/contrast,
levels, invert, HSL, blur, unsharp, noise, find-edges) with compose
interpreting live stacks by `EffectMoaID` and passing unknown GUIDs
through unfiltered — that passthrough is the honesty debt this milestone
retires. The M13 filter-pipeline pass supplies the object protocol
(property bags addressed by named atoms, the interpolate-toward
virtual, the `St*Processor` kernel layer with invert and add-noise
decoded); the `.jsf` behavioral corpus and the CS6 PNG-fallback
renders in the corpus supply the semantics and the oracles. **Gate:**
the unknown-effect GUID passthrough floor falls to zero over
corpus + harvested files; per-filter goldens green; `xtask ci` green
with the wasm ceiling holding; scorecard M14 row done.

**Inputs:** `docs/research/fw-code-notes.md` (M13 filter pass: property
protocol, interpolation semantics, kernel family map, invert/add-noise
decodes with parity verdicts); `docs/research/fw-ui-evidence.md` (the
Filters menu and Live Filters panel tables); the CS6 `Configuration/`
tree's `.jsf` scripts (readable behavioral specs for per-effect
parameter names/defaults); the M7 kernel discipline
(`crates/filters`: pinned color order, integer-exact proofs);
compose's live-stack interpretation (`EffectMoaID` + internal-name
fallback); the corpus fixtures and their CS6-render `.render` pins
(PLANNING §8's "render oracle"); PLANNING risk 11 (Wine is a force
multiplier, not a dependency — the harness stays optional hardening).

## Design line: enumerate the surface first; every kernel earns a proof

The CS6 live-filter surface is enumerable from evidence that is already
in hand or one extraction away: the menu tree names every effect, the
`.jsf` command scripts name every parameter with defaults, the RTTI
filter classes carry the property slots (the M13 pass read the pattern
on six classes), and compose's fallback table lists every GUID the
corpus has hit. That enumeration is written FIRST as a committed
derived table, and it is the completion checklist for everything after
— an effect family is not done because it renders; it is done when its
row in the table carries a matching kernel, pinned parameters, and a
golden.

Kernels keep the M7 contract: integer-exact, pinned color order,
deterministic, proven (`filters` tests hold the proofs; compose
composes). Where CS6's exact arithmetic is observable (corpus renders,
the two decoded kernels) it is matched; where it is not observable the
substitution is explicit in the kernel docs, as SplitMix64-for-MINSTD
was. The shadow/glow/emboss family is composite (alpha silhouette +
blur + offset + blend against the source), so its work is mostly
composition of proven pieces, not new pixel math; the same table-walk
SIMD reuse CS6 gets from `CBlurToolQuad3/4/6` is deliberately not
reproduced (recorded M13 verdict).

The oracle hierarchy is unchanged: CS6's own flattened render (the PNG
fallback of a corpus file, extracted as the `.render` pin) beats our
compose render, which is the deterministic reference for everything
else. A family with no corpus render gets a determinism-pinned golden
through the engine registry (the M7 `filtered` pattern) — the floor
that matters is the GUID passthrough: no live filter CS6 ships, and no
GUID observed in any corpus or harvested file, may ride through
unfiltered.

## Tasks

1. **Effect-surface enumeration** — every CS6 live filter: menu family,
   name, `EffectMoaID`/GUID, `.jsf` entry points, parameter
   names/types/defaults/ranges (`.jsf` + RTTI property slots + the M13
   interpolation evidence), current FireCraft status
   (kernel-exact / kernel-substituted / missing). Output: the filter
   table in `docs/research/fw-code-notes.md` (committed derived table)
   — the authority and completion checklist for tasks 2–5.
2. **Adjust Color + Sharpen + Stylize completion** — the missing
   scalar/convolution kernels: curves (knot interpolation), color fill,
   auto levels (histogram stretch per the `StHistogramRGBProcessor`
   family), motion blur, zoom blur, blur/blur-more/sharpen/sharpen-more
   presets, convert to alpha. Integer-exact proofs per kernel; registry
   parameter tables matching the task-1 rows; `filter.setParam`
   coverage for each.
3. **Shadow/glow/emboss family** — drop/inner shadow, outer/inner glow,
   raised/inset emboss as composite effects over the proven blur and
   blend pieces (alpha silhouettes, offsets, layer-order semantics per
   CS6's stack order). Same proof discipline: the composites are
   pinned by small-geometry proofs, not pixel-luck.
4. **compose + floor** — every task-1 family interpreted by
   `EffectMoaID` (and internal name) in the live stack; the
   unknown-GUID passthrough floor becomes a zero-enforced test over
   corpus + harvested files (a new GUID in a future file still fails
   loudly — the floor is over the enumerated surface, not a promise
   about Photoshop plugins).
5. **Goldens** — extend the corpus with filter-bearing CS6 files
   (harvest per PLANNING risk 11; Wine harness optional, not blocking):
   per-family golden fixtures through the engine registry,
   CS6-preview-matched where a `.render` pin exists, determinism-pinned
   otherwise; floors rise.
6. **Shell** — the Filters/Live Filters panel driven by the task-1
   table: per-family parameter rows, defaults, live preview through
   compose; the CS6 menu's Filter submenu mapped to the same registry
   ids the panel uses.

## Verification

- Enumeration: every effect row names its evidence (menu source, `.jsf`
  file, RTTI class); no row without a source.
- Kernels: integer-exact proofs per the M7 pattern; `cargo test
  -p filters` gates them; new commands fuzz-armed and in the
  call-table coverage test.
- Floor: the passthrough-zero test is part of `xtask ci`; a corpus or
  harvested file with an unimplemented observed GUID fails the gate.
- Goldens: `xtask` golden gates (CS6-preview-matched and
  determinism-pinned) green; floors recorded in the generated report.
- Shell: `cargo test -p app` structure tests for the panel; `xtask
  wasm` keeps the bundle compiling; `xtask dist --web` keeps the
  14 MB ceiling; `xtask perf` budgets untouched.
