# Decision 0046 — canvas trim + delete-all (P rows → C)

**CS6 surface.** Three dom verbs:

- `dom.setDocumentCanvasSizeToDocumentExtents(bGrowCanvas)`
  (Fireworks 3) — "Calculates the size of all the items in the
  document and resizes the document canvas to that size. This action
  is the same behavior as Modify > Trim Canvas." `bGrowCanvas`
  false: the canvas "only shrinks".
- `dom.setDocumentCanvasSizeToSelection()` (Fireworks 3) — "Calculates
  the size of all the items in the selection and resizes the document
  canvas accordingly." No grow argument.
- `dom.deleteAllInDocument()` (Fireworks MX) — "Deletes all of the
  objects in the document."

## Design

- **The canvas rect moves over the content; objects do not move.**
  The wire carries the canvas origin (`XLCf`/`YLCf`) next to its
  size (`WIDf`/`HITf`), and Trim Canvas is exactly "move+resize the
  canvas rect to hug the drawing". Both verbs set
  `x/y = bounds origin`, `width/height = bounds size`; every object
  keeps its world coordinates (unlike `setDocumentImageSize`, nothing
  scales). The whole `PageInfo` is captured, so undo restores the
  full canvas state.
- **FW3's "document" is one page.** The verb predates multi-page
  documents; Trim Canvas in CS6's UI is page-scoped. Both verbs take
  an explicit page index (the `document.rotate` precedent — no
  current-page state) and compute the extents of that page's items
  (`content_bounds`, which recurses through groups and instance
  transforms).
- **`bGrowCanvas` false clamps per dimension** to the current
  canvas size — "only shrinks" means each of width/height takes
  `min(extents, current)`. The origin still moves (the flag speaks
  to size only). The selection variant always grows or shrinks.
- **Bounds hygiene:** zero-extent bounds floor at one pixel per side
  (a single-point path trims to the 1×1 setCanvasSize minimum);
  sizes beyond 65535 refuse like `page.setCanvasSize`; non-finite
  bounds refuse. An empty page is the documented no-op (nothing to
  trim to — `Delta::Noop`); an empty selection refuses (CS6 has
  nothing to calculate from).
- **`deleteAllInDocument` folds `object.delete`.** One pre-flight
  sweep collects every top-level element id of every cell on every
  page (000557d: nothing mutates before the walk completes), then
  each id goes through the exact `object.delete` semantics — detach
  with container inverse, registry removal, `Removed` object
  inverse — folded into one `Delta::Group`. Group children ride the
  standing group-delete contract (the shell is removed; children
  stay registered — the deep-clone debt covers the walk). An empty
  document is the no-op.

## Consequences

- Registry 215 → **218**; parity 242 C / 29 P / 0 M / 85 N → **245 C
  / 26 P / 0 M / 85 N** (three flips).
- `content_bounds` stays the single bounds authority (the align/
  distribute walk); no new geometry code. `union_bounds` became
  `pub(crate)` for the selection variant.
- Shared-layer links: the sweep deletes shared content exactly once,
  at its owning page (links are references, not copies).
