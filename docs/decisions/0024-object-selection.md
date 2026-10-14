# Decision 0024 — object selection model & the selection.* commands

Date: 2026-10-14
Closes: HANDOFF §4 Phase 1.1 + 1.2 — parity rows `selectAll`,
`selectInverse`, `selectAllOnLayer` flip `M` → `C`;
`selectNone` gains the object-mode mapping next to
`bitmap.selectNone`.

## Model

`Document.object_selection: Vec<ObjectId>` — CS6's `fw.selection`.
Editing state, never file content (the format has no selection node;
the writer never reads the field). Canonical order is the document
walk order (page, layer, cell, element position — bottom-first, the
model's storage order), applied by every setter, so results are
order-deterministic regardless of pick order. Ids unreachable from a
cell (group children, until 1.3 lands) order by registry slot.

Distinct from `Document.selection` (the bitmap-mode pixel mask). CS6
keeps the two modes mutually exclusive in the UI; the model carries
them independently.

## Commands (`catalog/selection.rs`)

- `selection.set {ids}` — validated, deduplicated, canonicalized.
  Stale/wrong-generation ids refuse.
- `selection.selectAll {page, frame}` — CS6 `dom.selectAll()`: every
  element of the page in the frame ("all elements on all layers";
  the single-layer-editing variant is a UI concern).
- `selection.selectInverse {page, frame}` — complement within the
  page+frame scope. The CS6 text documents only the bitmap-mode
  case; the object-mode complement is the Select > Inverse menu
  behavior (recorded deviation).
- `selection.selectAllOnLayer {page, layer, frame,
  remember_selection?, toggle_selection?}` — CS6
  `dom.selectAllOnLayer(layerIndex, bRememberSelection,
  bToggleSelection)` verbatim: replace by default, append with
  remember, toggle membership with remember+toggle.
- `selection.selectNone {}` — clears the object selection; the pixel
  mask is untouched. Noop when already empty (as are select* results
  that change nothing — the journal stays clean).

Scope rules: layers shorter than `frame` contribute nothing; a frame
beyond every layer's cell list refuses. The documented CS6 text puts
no hidden/locked gating on select*; the engine selects everything in
scope (UI-level gating is a view concern — recorded deviation).

## Scripting wiring

The host verbs (`dom.selectAll`, `selectAllOnLayer`, `selectInverse`,
`selectNone`) queue the matching command **and** mirror the view
immediately, so mid-run reads agree with the queued truth. `fw.selection
= [...]` assignments remain plain view state — the command surface is
the durable, undoable path. The projection seeds `fw.selection` from
`document.object_selection` when the session list is empty, so a
selection set through commands before the run is visible. At drain, a
run that applied a `selection.*` command reconciles the host view
from the model (a later creation's select-what-it-created still
outranks it).

## Verification

Engine unit tests (7): canonicalization, dedupe, stale-id refusal,
scope/inverse/toggle/none semantics, undo round-trips, Noop hygiene.
Fuzz: all five ids in CALLS with payload builders; the
CALLS==catalog pin holds (registry 150 → 155). Scripting:
`mutate.rs` equivalence baselines now include the queued selection
command the scripts issue.
