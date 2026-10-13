# M15 — Tool-surface parity

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** take the tool surface from "works" to "matches the
original": every CS6 tool's semantics at modifier-exact behavior.
M6 built the vector tools (shape/text/line drag-create, align/
distribute, booleans, Auto Shape library), M7 the bitmap tools
(marquee/lasso/wand selection, feather, fill/erase/move-pixels,
retouch: blur/smudge/clone/eraser/red-eye), and M13 the tool strip
shell (11 glyphs, cursors, zoom/pan interaction parity) — but the
CS6 tool strip is larger than what FireCraft ships, and the shipped
tools are coarse: selections have no modifier arithmetic (shift adds,
alt subtracts, intersection), no edge treatment beyond a single
feather command; the brush/eraser/retouch family has no size/edge/
pressure model; the vector reshape tools (Freeform, Reshape Area,
Path Scrubber, Redraw) do not exist; gradient fills are the file
format's `FPLv` subtree — read and preserved verbatim, never rendered;
the style model (`CSTs`/`STYs`) is likewise pass-through. This
milestone enumerates the full tool surface first, then retires the
gaps family by family under the M14 discipline. **Gate:** the
enumeration table's every row carries a matching implementation or an
explicit recorded substitution; interaction suite green; oracle diffs
(corpus `.render` pins) green where a pin exists; `xtask ci` green
with the wasm ceiling holding; scorecard M15 row done.

**Inputs:** the M13 UI-resources pass (`docs/research/fw-ui-evidence.md`:
tool strip roster, keymap, 449 `RT_DIALOG` layouts — the tool option
dialogs carry each tool's parameter slots); the M13 Ghidra project
(persistent, `/nas/Temp/tmp/firecraft-re` — RTTI tool classes for the
selection/brush/reshape engines); the CS6 `Configuration/` tree's
`.jsf` scripts (Auto Shape Tools + Commands/Selection are already in
the extracted corpus); the existing tool implementations
(`crates/engine/src/catalog/bitmap.rs` + `retouch.rs` commands,
`crates/vector` geometry, `crates/app/src/shell.rs` drag model); the
file-format spec for fill/style subtrees (`PLANNING.md` §6,
`docs/research/fireworks-png-nodes.md` — `FPLv`/`TXBv`/`TXFv`,
`CFTs`, style containers); corpus fixtures with `.render` pins
(PLANNING §8 oracle); PLANNING risk 11 (Wine harness optional —
oracle hierarchy unchanged: corpus render beats compose, compose is
the deterministic reference otherwise).

## Design line: enumerate the tools first; every tool earns its modifiers

CS6's tool strip is enumerable from evidence in hand or one extraction
away: the resource DLL's tool icon bins + `MMPG` panel groups name the
roster and flyouts, the keymap names the one-key selects (already
extracted), the `RT_DIALOG` layouts carry per-tool option dialogs
(marquee style/feather edge, wand tolerance/contiguous, brush
size/edge, lasso edge), the RTTI classes carry the engine slots
(selection combine modes, brush dynamics, reshape pressure), and the
`.jsf` corpus shows scripted invocations with real parameter shapes.
That enumeration lands FIRST as a committed derived table in
`docs/research/fw-code-notes.md` (extending the M13 tool verdicts) —
the completion checklist for everything after. A tool is not done
because it drags; it is done when its row carries modifier
arithmetic, parameter slots matching the dialog table, and a test.

Modifier arithmetic is the shared spine and lands before any
per-tool depth: selection commands gain combine modes
(replace/add/subtract/intersect — the CS6 shift/alt-shift/alt-
ctrl-shift reads), drag tools gain the constraint keys (shift square/
axis-lock, alt from-center) as one documented matrix applied in the
shell drag model, not per-tool ad hoc. Bitmap selections become a
first-class mask channel (the feather/edge treatments operate on the
mask, and `bitmap.movePixels` already proves the stamp path); brush/
eraser/retouch gain the size/edge/pressure triple with the same
integer-exact contract as the M7 kernels. Vector reshape
(freeform/reshape-area/path-scrubber/redraw-path) works on the
existing `vector` crate's point/segment model — new geometry, no new
format. Gradients and styles are format work first: interpret the
`FPLv`/`TXBv`/`TXFv` subtrees into a fill model compose can render
(linear/radial/contour/conical, stops, opacity, dither — the same
enumerate-pin-prove ladder), then surface them in the panel; styles
deserialize into named property bundles over the existing fill/
stroke/effect model.

## Tasks

1. **Tool-surface enumeration** — every CS6 tool and flyout: roster
   position, keymap binding, option-dialog parameters (the `RT_DIALOG`
   extraction), RTTI engine class, current FireCraft status
   (parity / substituted / missing). Output: the tool table in
   `docs/research/fw-code-notes.md` (committed derived table) — the
   authority and completion checklist for tasks 2–6.
2. **Modifier arithmetic + selection combine modes** — the shared
   constraint matrix in the shell drag model (one documented table,
   structure-tested); selection commands gain combine modes over a
   first-class mask (replace/add/subtract/intersect, feather/edge
   treatments on the mask), journaled, undoable, fuzz-armed, in the
   call-table coverage test.
3. **Brush/eraser/retouch depth** — the size/edge/pressure model over
   the proven stamp kernels (per-dab integer-exact, deterministic);
   tool option payloads match the enumeration rows; retouch tools
   take the same model.
4. **Vector reshape family** — freeform push, reshape-area, path
   scrubber (variable width), redraw path: geometry in `crates/vector`
   with proofs, registry commands journaled + undoable, shell tools
   with cursors.
5. **Gradient fills** — the `FPLv`/`TXBv`/`TXFv` subtree decoded into
   a fill model (kinds, stops, opacity, dither) with corpus
   round-trips byte-exact; compose renders linear/radial first
   (contour/conical as pinned substitutions where CS6's math is not
   observable); gradient tool + Fill panel rows; goldens.
6. **Style model** — style containers deserialize into named bundles
   over fill/stroke/effect; the Styles panel lists/applies them;
   corpus round-trips byte-exact; Common Library gains the CS6 style
   set only if permissively re-derivable (asset rule — never copied).

## Recorded gaps (task-5 close, 2026-10-13)

- **compose never paints the canvas background.** `canvas.background_raw`
  rides the model and `webexport` emits it (`bg_hex`), but
  `compose::render` leaves the canvas transparent — the bare pixel in
  the gradient golden pins that. Painting it is a compose change with
  golden re-bless + anchor rewrites (every fixture's bare pixel moves);
  its own task, alongside the SVG `background` attribute.
- **SVG import drops gradient paint servers** (`gradients_dropped`
  counter + notes; the round-trip gate skips gradient-painted
  documents on that recorded limitation). Import of
  `linearGradient`/`radialGradient` back into the `FPLv` fill model
  is open — the exporter side is exact (`svg-golden`: max Δ 2 on the
  gradient fixture).
- **Gradient fill substitutions are pinned, not observed**: elliptical
  renders as a circle (the third handle's squash needs an affine the
  radial brush does not carry); satin/pinch/folds/rectangular/bars/
  ripple/waves/web-dither/pattern/contour render as the solid `FCLi`
  colour; the conical sweep starts along the fill vector and wraps a
  full turn (CS6's zero-angle basis is not observable); `FSHi` value 3
  reads as Radial (ambiguous against the dictionary); `FCLi` rides the
  first RGB stop; stop interpolation is unpremultiplied (`FG0v`/`FG1v`
  are independent 1-D ramps — the premultiplied default drags a
  transparent end stop black); `path.setFillColor` collapses an
  existing gradient definition to solid (resolved from the documented
  model in `docs/decisions/0018-setfillcolor-solid-collapse.md`; the
  Wine-oracle playback remains the named falsifier).

## Verification

- Enumeration: every tool row names its evidence (icon bin/keymap/
  dialog/RTTI/`.jsf`); no row without a source.
- Modifiers: the constraint matrix and combine modes are
  structure-tested; every selection command carries a combine-mode
  refusal test; fuzz arms extended (selection-mutation cases).
- Kernels: integer-exact proofs per the M7 pattern; `cargo test -p
  filters` and `-p vector` gate them; new commands in the call-table
  coverage test.
- Goldens: corpus round-trips byte-exact for gradient/style files;
  `xtask golden` pins green (CS6-matched where `.render` exists,
  determinism-pinned otherwise); floors rise only with new pins.
- Shell: `cargo test -p app` structure tests for the new tools and
  panels; `xtask wasm` keeps the bundle compiling; `xtask dist --web`
  keeps the 14 MB ceiling; `xtask perf` budgets untouched.
- Close: scorecard M15 row + devlog entry; commits conventional;
  GitForge sweep green.
