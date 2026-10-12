# 0010 — `EXPv` export options: mapping, precedence, and refusal posture

- **Status:** Accepted (M8 task 8, 2026-10-11)
- **Deciders:** Michael Kinney, Claude
- **Scope:** `doc::ExportOptions` and its `fwmap` reader/writer arms
  (L3), `io::expv` option resolution (L4), and the consumers that
  resolve stored settings against explicit requests (`app-cli` export
  paths, the shell export dialog).
- **Supersedes:** none · **Superseded by:** none

## Context

The M8 ticket (task 8) demands `EXPv` mapped both ways with unknown
children preserved exact-path (the M3 unknown-bag rule), selective
JPEG quality, a CLI batch surface, and a shell export dialog.
`EXPv` is the dictionary node that records a document's (or page's, or
slice's) optimization settings; its children are a flat bag of typed
scalars whose meaning is known for a dozen tags and unknown for the
rest (`FNPi`, `VISb`, `OBNs`, …). The `FNLv`/`FNMv`/`FNDi` subtree
records per-frame timing for animated exports.

## Decisions

### 1. Known children map; unknown children ride the bag

Fifteen known tags map to typed `ExportOptions` fields: `FMTi`
(format), `MCTi` (matte, ABGR — the opposite byte order of the model's
`TMCi` ARGB, so it round-trips as `matte_raw` verbatim), `JPQi`/
`JPSi`/`JPOi`/`JPPb` (JPEG quality/smoothing/progressive), `GFIb`
(interlace), `GLTi` (GIF loss), `PLTi`/`PLDi`/`PLOb` (palette), `PTNi`
(transparency), `FLPi` (frame loop), `AACb`/`AADb` (animation
crop/diff), plus `FNLv` frame timing. Everything else under `EXPv`
bags with `(container path, child index)` and splices back at the
recorded indices on write. Duplicate `EXPv` nodes: first wins, later
ones bag whole. A known tag carrying the wrong payload shape is
`MapError::BadShape` — never a guess.

Splice semantics (pinned by `doc` test): mapped children fill
canonical dictionary order; unknowns interleave at their recorded
indices; mapped items beyond the original count append.

### 2. `FNLv` is all-or-nothing

Frame timing maps only when **every** `FNMv` child carries `FNDi`.
Any incomplete frame config bags the whole `FNLv` node verbatim — no
partial timing lists, because a partially-mapped timing table would
silently re-time frames that had none. `PageInfo.frame_durations`
holds hundredths-of-a-second `i64`s exactly as `FNDi` stores them (no
unit conversion at the model layer).

### 3. `FLPi` loop spellings; `Never` omits the extension

`FLPi` 0 = `Loop::Infinite` (NETSCAPE count 0), -1/4294967295 =
`Loop::Never`, n = `Loop::Times(n)` (clamped to u16 on write). A
`Never` export **omits** the NETSCAPE extension entirely — players
treat a missing extension as play-once, and writing count 0 would
mean infinite, the opposite of the setting. (Writing 0 for Never would
be a silent inversion; omission is the only faithful encoding given
GIF89a's two-state extension.)

### 4. Resolution order: explicit > page > document > codec default

Consumers resolve through `io::expv::for_page` (page export, falling
back to document export) and the per-codec option builders. An
explicit CLI flag or dialog setting wins over any stored value; a
stored page value wins over the document's; the codec default is last.
Slice-level `EXPv` (the selective-quality table under `URLv` nodes) is
mapped and round-tripped now; its consumer is the M11 web-export
pipeline, which walks per-slice settings at slice export time.

### 5. No Adam7: interlaced PNG refuses loudly

`png` 0.18 has no interlaced *encoder*. `PngOptions.interlaced` set
from `EXPv GFIb` produces `ExportError::Unsupported` naming Adam7
rather than silently writing progressive. When the codec grows an
interlaced encoder this flips to honored with a corpus pin.

### 6. Dither is a real-value feature, tested under palette pressure

Error-diffusion dither only changes output when the palette misses. A
palette that fits every color exactly (unique colors ≤ palette size)
plus zero error produces byte-identical output with and without
dithering — that is correct, not a defect. The dither pin therefore
uses a 320-unique-color image against a 256-slot palette so the
diffusion path actually runs.

## Consequences

- `ExportOptions` is total-`Option` by design: absent = not stated =
  defer down the resolution chain. The batch overlay and the dialog
  both fill only `None` fields (`stamp`), so per-file/slice recorded
  settings survive a batch run that sets only quality.
- Consumers that gained `Page.export`/`PageInfo.frame_durations`
  literals are updated wholesale; `PageInfo` still has no `Default`
  (its scalars are required — constructing one is deliberate).
- The fuzz target `expv` anchors parse → read → totality asserts →
  write → re-read equality; 60 s libfuzzer campaign: 3.26 M execs,
  0 crashes (floor 6 → 10 with gifanim/psdimport/svgimport).
