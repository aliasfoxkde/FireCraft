# Decision 0035 — view flags (setShowGrid family) are session state

**Date:** 2026-10-14 · **Phase:** 4 · **Status:** resolved — rows flipped
to `N` with evidence; flags hosted in the scripting session

## Verbs (CS6 API text)

- `dom.setShowGrid(bShow)` — "If true, the grid is visible; if false,
  the grid is not visible."
- `dom.setShowGuides(bShow)` — "If true, the normal guides are
  visible…"
- `dom.setSnapToGrid(bSnap)` — "If true, the tools snap to the grid…"
- `dom.setSnapToGuides(bSnap)` — "If true, the tools snap to all
  guides…"

All four: Fireworks 3, Document object, no return value.

## The finding (Phase 4.1 corpus hunt)

**No wire home exists — proven three ways:**

1. **The dictionary is exhaustive on the candidates.** `GRDv` (grid)
   carries exactly `GOXf`/`GOYf`/`GSXf`/`GSYf`/`GCLi` — geometry and
   colour, no show/snap boolean. `GDSv` (guides) carries
   `GDLb`/`GDKi`/`GDHv`/`GDVv` — locked, colour, positions, no
   visibility flag. The complete `MKBv` and `PDCv` child lists
   (checked node by node) contain no view/preferences node at all;
   no dictionary fourcc contains "snap", "show", or "view".
2. **CS6's own scripts never use them.** A case-relaxed sweep of the
   entire CS6 `Configuration/` tree (89 `.jsf` scripts, Commands,
   Panels) finds zero occurrences of the four verb names — nor any
   grid-visibility API. Adobe's shipped automation treats them as
   outside the document.
3. **The API text reads as view toggles**, not document mutations:
   "the grid is visible", "the tools snap" — session UI posture, the
   same class of state Fireworks keeps in its user preferences file.

Per the Phase 4 rule, no wire node was invented. The four rows flip
to `N` (parity 203 C / 45 P / 20 M / 88 N).

## Implementation (Phase 4.2 — the scripting session)

Hosted exactly like `dom.changeCurrentPage` (the existing view-state
precedent):

- **State:** four booleans on the scripting engine's `HostState`
  (default off — a fresh session shows neither grid nor guides and
  does not snap, matching CS6 defaults).
- **Projection:** the document object exposes `showGrid`, `showGuides`,
  `snapToGrid`, `snapToGuides` as read properties.
- **Mutators:** `dom.setShowGrid(bShow)` and siblings validate a
  boolean argument (typed throw otherwise), write the projection in
  place (same-run reads agree), and queue a control intent
  (`__fw.showGrid` family).
- **Drain:** flag intents resolve order-independently like the page
  change — persisted into host state, no document delta, no journal
  rows, nothing for undo to touch. A failed run drops them with the
  rest of the queue (failing scripts leave everything untouched).

## Deviations (recorded, not hidden)

1. **Not engine commands.** A registry command would imply a document
   delta and a fuzz arm; these verbs have neither in CS6. The parity
   matrix tracks the command surface, so they read `N`, not `C`.
2. **Flags are per-engine, not global app state.** The app shell will
   read them out of `AttachView` when it renders grid/guides — wiring
   the app UI is future work, not a scripting-layer obligation.
3. **No persistence to files.** CS6 keeps these in user preferences
   outside the document; there is no `mkBT`-style slot to write them
   to, and inventing one is forbidden by the Phase 4 rule.

## Consequences

- The M table shrinks to 20 verbs; the guides/grid/onion *data*
  family (`page.addGuide`, `setGridOrigin`, `setGridSize`,
  `setOnionSkinning`, …) was already carried by `page.*` commands.
- `AttachView` grew four public fields — any future host (app shell,
  CLI) passes its own flags; defaults keep behavior unchanged.
- If a future corpus file ever surfaces a view-prefs node, this
  decision is the written refutation to reopen: the dictionary sweep
  date (2026-10-14, davidje13 vendored dictionary + CS6 Configuration
  tree) bounds the evidence.
