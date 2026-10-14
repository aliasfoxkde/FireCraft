# Decision 0026 — selection-to-frames fans

Date: 2026-10-14
Closes: HANDOFF §4 Phase 2 — parity rows `duplicateSelectionToFrames`,
`duplicateSelectionToFrameRange`, `distributeSelectionToFrames` (and the
`cuplicateSelectionToFrames` typo alias) flip `M` → `C`.

## Wire / model

CS6 aims all three verbs at "the current frame" — editor state the
engine does not carry. Every command names its source frame explicitly
in the payload, the same resolution `frame.add` uses for its
current-relative locations and `frame.delete` documents in reverse
("the engine has no active frame"). The acted-on set is the object
selection filtered to top-level items owned by a cell of that
page+frame; group children are skipped (fan the group by selecting the
group). An empty acted-on set is `Noop`, as is `distribute` with a
single item (documented).

The fans reuse `cell.rs` machinery: `check_closure` before any mint (a
refused fan leaves nothing behind), `clone_element` per (item, target)
with a fresh remap table (every copy independent), `splice_cells` for
frame additions (`FRCi` counts + aligned duration entries, matching
`frame.add`).

## Commands

- `selection.duplicateToFrames {page, frame, which}` — CS6
  `dom.cuplicateSelectionToFrames` (the original's own typo stays in
  the parity matrix; the command carries the clean name). `all` covers
  every existing frame *including* the source (an in-place clone — the
  text documents no exclusion, unlike the range verb); `previous`
  underflows to `Noop` at frame 0; `next` past the last frame adds one
  (the "end … does not add a new frame" note implies the others may);
  `end` never adds.
- `selection.duplicateToFrameRange {page, frame, first, last}` —
  inclusive; `-1` keeps its CS6 meaning and names the payload's source
  frame. Spanning ranges skip the source ("duplicates are not placed
  on that frame"); `first == last` copies onto that frame even when it
  is the source (the documented exception). Refusals: indices below
  `-1`, `first > last`, and any bound past the last frame — range fans
  never create frames (the text grants creation only to `distribute`
  and `next`).
- `selection.distributeToFrames {page, frame}` — moves the items onto
  consecutive frames from the source frame ("the first item goes to
  the current frame"), adding frames when the run outruns the page.
  Item 0 stays; the rest move within their own layer (identity
  preserved, so the selection stays valid untouched).

Copies append to the target cell (top of z-order) — a recorded
deviation from `object.duplicate`'s above-original placement: the fan
has no single "the copy" to sit above. Move z-order appends likewise.

## Deltas

Duplicates: `Delta::Group([PageBody, Created…])` — the pre-command
page covers cell lists, added frames, and `FRCi` in one capture;
mints retract LIFO under undo. Distribute: `Delta::PageBody` alone (no
mints — ids are preserved).

## Deviations

- Source frame explicit (above) — CS6 reads editor state.
- No `-1`-only surface: the sentinel maps to the payload frame.
- `edgemode`-style extras: none in this family; CS6's silent points
  (all-includes-source, range refusal bounds, append z-order) are the
  simplest readings of the text, recorded here. No `.jsf` usage exists
  in the Configuration tree for any of the three verbs, so the API
  text is the sole authority — no corpus oracle.

## Consequences

Frame-animation workflows (select on frame 1, fan to N) are now
command-drivable; the scripting host mirrors them as plain command
queuing (no `fw.selection` changes — the fans leave the selection
alone, per the silent text).

## Verification

Engine unit tests (7 new): all-frames coverage with group closures and
exact undo; range skip + single-frame exception + refusals; next-past-
the-end frame add with undo; previous underflow and empty-selection
no-ops; distribute across the page edge with frame add, id
preservation, and exact undo; single-item no-op. Fuzz: the three ids
in `CALLS` with `which`/range payloads; CALLS==catalog pin holds
(registry 158 → 161).
