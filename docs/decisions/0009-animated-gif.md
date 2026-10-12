# 0009 — Animated GIF ↔ states: compositing, timing, and palette policy

- **Status:** Accepted (M8 task 7, 2026-10-11)
- **Deciders:** Michael Kinney, Claude
- **Scope:** `io::gif` (L4) — the animated-GIF importer's compositing
  semantics, the exporter's palette and disposal policy, where frame
  timing lives in the model, and the `compose` Image-object arm this
  task landed to make it possible.
- **Supersedes:** none · **Superseded by:** none

## Context

The M8 ticket (task 7) demands animated GIF import (GIF89a frame loop
→ one state per frame, disposal respected, delays in the doc's
per-state record) and export (states → animated GIF, global palette
with local fallback, loop count, per-frame delay, `AACb`/`AADb`
honored). There is no animated corpus in `corpus/fixtures` (all
fixtures are own-generated) and the CS6 `Configuration/` scripts carry
zero `frameDelay` hits, so "CS6 behavior" here is grounded in: the
GIF89a specification's own compositing model (what every browser
does), the `gif` 0.14 decoder/encoder semantics, and `color_quant`
1.1 (PLANNING §7.6 stack).

## Decisions

### 1. Import compositing is browser semantics

Each frame draws over the screen (source-over); fully transparent
source pixels leave what is beneath; state *i* captures the screen
**after** frame *i* drew. Disposal applies when the *next* frame
starts: `Keep`/`Any` leave the screen (Any is treated as Keep with a
report note — the spec says "no disposal specified", and Keep is what
players do); `Background` clears the frame's own rect to transparent;
`Previous` restores a snapshot taken **before** the frame drew. A
frame whose rect leaves the logical screen is a `Decode` refusal
(per-spec, and the decoder hands us the numbers to check it exactly).

The frame loop is bounded before decoding: the tile bank holds 4096
tiles, one image per state needs ⌈w/128⌉·⌈h/128⌉ of them, so the
state ceiling is `min(4096 / canvas_tiles, 4096)` — checked before any
frame decodes, so hostile declarations fail fast.

### 2. Timing home: `PageInfo::frame_durations`, no `FRVi` invention

Per-state delays live in `PageInfo::frame_durations: Vec<i64>` in
**hundredths of a second** — the GIF delay field's unit *is* 10 ms, so
values pass through unchanged on both sides. The dictionary's `FNDi`
(Frame Delay) is the same unit. The ticket's "map to `FRVi`-style
timing if the model carries it" resolves to: it doesn't (the M3 model
has no timing node), so this vec is the doc's per-state record and the
`EXPv`/`FNLv` persistence mapping — the on-disk spelling — lands with
task 8's `EXPv` work. Until then documents round-trip timing through
the CLI but a canonical `convert` does not preserve it.

**Export default: 10 cs (100 ms)** when a state outruns the record.
The CS6 panel default was not observable (zero `frameDelay` hits in
the source tree; no animated corpus) — **revisit against real-corpus
`FNDi` evidence in task 8**. Recorded values clamp to 0..=65535.

### 3. Export palette policy: global-first with a fidelity bar, local fallback

One global palette trains on **every state at once** (`color_quant`,
sample factor 1 — deterministic; slot 0 reserved for transparency and
never matched by the nearest scan). Global mode is kept when every
state's mean absolute per-channel error against its indexed form is
≤ 8.0; otherwise the whole animation falls back to per-frame local
palettes (`Frame::from_rgba_speed`) for full per-frame fidelity.

Rationale: `crop` and `diff` need a *stable index space* across
frames, which only the global palette provides. Diff is therefore
disabled in local mode — an indexed-space delta across varying
palettes is meaningless. In global mode, mean-error ≤ 8 keeps
primaries visually intact (observed: sparse two-color screens can
miss a pure primary by ~14 levels on one channel while the mean stays
≈1 — NeuQuant spreads 255 neurons across the cube) while unlocking
crop/diff everywhere else.

### 4. `AACb` crop, `AADb` diff, and the diff-seed disposal

`crop` trims each frame to its ink bounding box (pixels ≠ the
transparent index) with left/top offsets; a fully transparent state
stays a 1×1 transparent frame at the origin — GIF frames are never
empty. `diff` sends pixels whose global index equals the previous
frame's to transparent slot 0 (an indexed-space delta) and — the bug
the battery caught in the first draft — **every frame including the
seed gets `Keep` disposal**: a `Background` seed clears the screen the
very next frame's unchanged pixels rely on, silently corrupting the
chain from frame 1 on. Non-diff frames use `Background` (self-contained
screens). The documented trade, same as every diff-writing exporter: a
state that *clears* a previously opaque pixel cannot be expressed
through Keep — the pixel stays.

Loop count rides `GifOptions::repeat` (`None` = NETSCAPE infinite);
the extension is written for both modes.

### 5. The compose Image arm (the M7-owed gap this closed)

Images now render: the `TILv` grid assembles into a full-canvas
premultiplied buffer through the same `effects::plan` → hash →
`apply_filtered` → `paint_surface` path as groups. Pinned by
`compose/tests/image_render.rs` (11 tests): exact extent placement,
row-major multi-tile grids, negative-origin clipping, matte
substitution, opacity compositing (integer-exact `OPAi=200` over
white), live-effect routing, per-state cell isolation, and the
refusals. Two byte-order facts recorded here because they diverge:

- **`TMCi` matte raws are ARGB** per the dictionary (a|r|g|b), unlike
  fill/stroke raws which are ABGR. The matte decoder is the one place
  this is spelled; tests pin it.
- **Missing `TIDi` is a document defect, not a skip**: render refuses
  with `TileBankMiss` naming the tile. `app-cli`'s `open()` still does
  not bridge `mkBT` into the bank (M7-owed debt), so real documents
  with images fail *loudly* instead of silently dropping bitmaps.
  `TSZi` 256 refuses loudly too (`TileSource`) — bank slots are fixed
  128².

### 6. Static vs animated routing

`app-cli import` sniffs GIF8 + frame count > 1 → the animated
importer; single-frame GIFs keep the static raster path (unchanged
from task 2). `app-cli export *.gif` writes full frames on an
infinite loop with default options — the `EXPv` crop/diff wiring is
task 8's.

## Consequences

- The ticket's stopping condition is met by
  `crates/io/tests/gifanim.rs` (17 tests): the golden pin (frame
  count + delays + per-frame ink anchors), import→export→import
  equality (frames, delays, ink within quantization tolerance,
  transparency exact), all four disposal methods, crop rects +
  placeholder frames, diff transparency + compositing back to the
  original states, loop extension + determinism, and hostile
  refusals — plus the `gifanim` fuzz arm (libfuzzer target + stable
  anchor over an in-code 3-frame seed, driving the full
  decode → render-all-states → re-encode both option paths →
  re-decode loop).
- `compose` gains a dependency on `raster` (same workspace, layer-legal).
- `save()`'s fresh-stream path still emits no `mkBT` (M7-owed), so an
  imported animation saved as `.fw.png` carries the model without
  pixel payloads — unchanged from tasks 2/6, closed by the same
  future bridge.
