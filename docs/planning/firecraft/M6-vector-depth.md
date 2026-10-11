# M6 — Vector depth

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** real vector editing: path booleans (union/intersect/punch/
crop), path ops (simplify, expand stroke, inset), Auto Shape tools +
library, align/distribute, freeform/reshape, and 9-slice editing — all
through the M5 command registry, so undo, journal replay, and the CLI get
every new operation for free.

**Inputs:** M3 `doc` path model (format-mapped `FDIv`-family path data)
+ M5 `engine` registry/deltas; `geom` (L0) transforms; stack decisions
PLANNING §7.1 (`kurbo` 0.13 for curve math, `lyon_tessellation` 1.0 for
stroke outline); the local Adobe tree's Auto Shape `.jsf` behavior specs
(`source/` — never committed; read as specs in scratch, and used as
runtime test data only under `corpus/incoming/`).

**Authority:** ROADMAP M6 gate: *scripted shapes reproduce CS6 geometry
on corpus cases; `.jsf`-authored shapes from the local tree render
correctly.* Path booleans are bespoke work (PLANNING §7.1 names no
boolean crate — the spike in task 1 decides the algorithm).

## Scope line: .jsf behavior is ported, not executed

M9 owns the scripting host. For M6, each Auto Shape's `.jsf` behavior
(`GetControlPoints`/`SetControlPoints`/parameter drawing) is ported to a
Rust generator with the `.jsf` source as the spec; corpus cases come from
the local tree. The gate's "scripted shapes" means our parameter-driven
generators; executing the original `.jsf` on a JS host stays M9.

## Tasks (atomic)

1. **`vector` crate (L2) + boolean spike** — new `vector` crate: owns
   boolean ops (union/intersect/punch/crop), simplify, expand stroke,
   inset. First deliverable is the algorithm decision
   (`docs/decisions/0003-path-booleans.md`): flatten-and-clip (polygon
   clipping at a recorded tolerance) vs Bézier-aware clipping —
   decided by evidence on adversarial cases (overlapping loops,
   collinear/tangent edges, self-intersection, split-result
   degeneracies), not preference. **Stopping condition:** integer-exact
   or tolerance-pinned proofs for all four booleans on the adversarial
   case set — same discipline as M4's compose proofs — plus determinism
   (same input, same output bytes, every run).
2. **Curve layer: kurbo bridge** — `vector` converts between the `doc`
   path representation and `kurbo` primitives at the crate boundary;
   `doc` keeps its format-mapped representation untouched (round-trip
   floors must not move). **Stopping condition:** conversion property
   test — doc→kurbo→doc is identity on the corpus path set; kurbo
   math (arc-length, bbox, split) exercised against hand-computed
   values.
3. **Path ops: simplify, expand stroke, inset** — simplify
   (tolerance-driven reduction), expand stroke via `lyon`
   tessellation/outline with joins/caps mapped to Fireworks' recorded
   behavior, inset/offset. **Stopping condition:** golden renders
   (M4 machinery) for each op on pinned shapes; idempotence where the
   operation claims it (simplify twice at same tolerance = simplify
   once, byte-equal).
4. **Registry commands + deltas** — `path.boolean`,
   `path.simplify`, `path.expandStroke`, `path.inset`, each capturing
   the pre-op path as delta (inverse = restore original paths).
   `panic_hunt` grows these commands. **Stopping condition:** fuzz
   campaign (stable driver + 60 s libfuzzer) clean; undo/journal replay
   equivalence for boolean sequences byte-compares as in M5.
5. **Auto Shape library (13 tools)** — parametric generators ported
   from the `.jsf` behavior specs: control points, parameter schema,
   live re-generation on parameter change (Auto Shapes stay editable —
   regenerate, don't bake). **Stopping condition:** for each shape,
   rendered geometry golden-pinned; parameter round-trip
   (create → mutate → re-render) deterministic; corpus cases from the
   local tree render correctly per the gate.
6. **Align/distribute** — bbox-based commands (align left/center/right/
   top/middle/bottom, distribute spacing) over selection sets.
   **Stopping condition:** table-driven tests over canonical layouts;
   registry commands with captured transforms (undo restores positions
   exactly).
7. **Freeform/reshape + 9-slice editing** — freeform/reshape pointer
   tools mutating path geometry through the registry (per-point deltas),
   `NGSv` nine-slice guide editing as a command. **Stopping condition:**
   shell-driven edits replay byte-identically via journal (the M5
   equivalence harness, now over vector commands); 9-slice guides
   round-trip through the document format.
8. **Surfaces + gates** — shell tools wired (Auto Shape tool group,
   align panel, path ops menu), `vector` added to `WASM_CRATES` (it is
   L2 pure math: no IO, no threads), CI picks everything up via the
   pipeline. **Stopping condition:** `xtask ci` green including the new
   crate; scorecard M6 → done with gate evidence recorded in
   `log/devlog.md`; parity floors raised where the gate earned them
   (golden scripted-shape floor).

## Non-goals (M6)

Text layout and text-on-path (M7), bitmap/filters (M7), scripted
execution of `.jsf` files on a JS host (M9), symbols (M10).
