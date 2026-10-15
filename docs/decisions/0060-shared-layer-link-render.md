# Decision 0060 — shared-layer links render their definitions

**Debt closed.** A link layer (`LSAv`, decision 0022) carried no
cells, so the compositor walked it as an empty layer — shared-layer
content never appeared in any render or export.

## Design

- **The page walk resolves the link in place.** `walk_layers` gains
  `resolve_links`; at a link layer it looks up
  `document.shared_layer_defs` by the link's `UIDs` and recurses into
  the definition's layer body — the shared content renders exactly
  where the link sits in the page's z-order, bottom-first with the
  page's own layers.
- **Both sides' flags gate.** The link layer's own `display` (and
  `visibility_per_frame`) hides the shared content on that page only;
  the definition body's flags gate inside the recursion and hide it
  everywhere — two independent switches, like CS6's page-level layer
  visibility over a shared layer.
- **Links never nest.** The recursion calls back with
  `resolve_links: false`, so a definition body's own `shared_link`
  stays inert. CS6 shares at page level only; the inert rule also
  rules out uid cycles by construction, and a symbol-definition walk
  (the other `walk_layers` caller) passes `false` for the same
  reason — a symbol's content is its own.
- **A dangling uid draws nothing.** The stale-reference walker rule:
  a link naming an absent definition skips silently instead of
  failing the render (CS6 renders the page).
- **Renders and exports inherit the fix.** Every entry point goes
  through `render_inner`'s walk — full render, `render_transparent`
  (GIF/PSD export surfaces), flatten bakes (`render_subset` renders
  explicit ids and is untouched by definition).

## Consequences

- Registry unchanged; parity table unchanged (the row was already C
  with a recorded deviation — decision 0022). No corpus fixture
  carries `LSAv`, so goldens are unaffected.
- Tests: +4 (compose integration) — a link renders the shared
  content; the link layer's and the definition's display flags gate
  independently; a dangling uid draws nothing and a self-referencing
  definition stays inert (no cycle); the shared content keeps the
  link's z-position under a later opaque layer.
- Remaining Phase 9 debts: pattern-authoring command, importFile
  resample quality.
