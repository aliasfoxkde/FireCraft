# Decision 0025 — hierarchy selects, selectSimilar, and the
filterSelection reclassification

Date: 2026-10-14
Closes: HANDOFF §4 Phase 1.3 + the corrected 1.4 — parity rows
`selectParents`, `selectChildren`, `selectSimilar`,
`selectSimilarFromPoint` flip `M` → `C`; the four `filterSelection*`
rows stay `M` with corrected classification.

## The corrected premise (recorded)

HANDOFF Phase 1.4 listed `selectSimilar`/`selectSimilarFromPoint` as
attribute selects and `filterSelection`/`filterSelectionByID`/
`filterSelectionByName` as attribute filters. The CS6 API text says
otherwise:

- `dom.selectSimilar(tolerance, edgemode, featherAmt, combinemode)` —
  **bitmap mode**: "selects all pixels in the current image that are
  within the specified tolerance of the average color in the current
  pixel selection."
- `dom.filterSelection*` — **filter application**: "applies the
  specified pixel filter to the selection as a permanent action, not
  as a Live Effect." These belong to the filter family and need the
  bitmap bake pass (recorded debt); they are not selection surface.

## Commands

- `selection.selectParents {}` — CS6 `dom.selectParents()`: when
  every member of a group is selected, the members drop out and the
  group joins (partial sets stay put). Nested groups resolve
  innermost-first to a fixpoint, depth-capped at 64 against corrupt
  cycles.
- `selection.selectChildren {}` — CS6 `dom.selectChildren()`: a
  selected group expands to its members. One level per invocation,
  as documented. Dangling entries drop (deleted objects stop being
  selected, matching the projection).
- `bitmap.selectSimilar {id, tolerance, contiguous?, mode?}` — the
  average color over the active mask's selected pixels seeds the
  wand (mask-weighted, rounded half-up); the match is global unless
  `contiguous` floods. Refuses when no active selection, the plane
  does not span the target image, or the selection covers nothing.
  CS6's `edgemode`/`featherAmt` are edge softening the model applies
  separately (`bitmap.feather`) — recorded deviation.
- `selectSimilarFromPoint` — carried by the existing
  `bitmap.selectWand` (a point-seeded wand; `contiguous: false` is
  the global match).

Canonical order note: selection members not reachable from a cell
(group members after expansion) order by registry slot, strictly
after every cell-listed id — the `(walked, key, slot)` sort key in
`canonicalize` keeps the two rank spaces from colliding.

## Verification

Engine unit tests (4 new): one-level expansion, Noop when no group is
selected, all-members-required collapse with an outsider preserved,
nested collapse + one-level re-expansion, selectSimilar over the
checker fixture (average of a solid selection matches image-wide,
empty-selection refusal). Fuzz: the three new ids in CALLS;
CALLS==catalog pin holds (registry 155 → 158).
