# 0021 — Layer/page structure: flags and order over the existing model

- **Status:** Accepted (post-roadmap gap close, 2026-10-14)
- **Deciders:** Michael Kinney, Claude
- **Scope:** `engine::delta` (`PageBody`, `Pages`), `engine::catalog::page`
  (master/reorder/name rows), `engine::catalog::layer` (reorder/lock
  rows), and the parity matrix's layer/page rows.
- **Supersedes:** the "layer/page structure (~11 verbs)" gap note's
  structure rows · **Superseded by:** none for structure; the sharing
  trio is deferred with its own model requirement (below).

## Context

Seven DOM verbs are pure structure over model bits the format layer
already round-trips: page rename, page reorder, layer reorder, layer
lock, and the master-page trio. Three more (the sharing trio) look
adjacent but are not — they need the shared-layer wire model typed
first.

## Evidence

1. **CS6 signatures** ("Extending Fireworks"):
   `setPageName(index, name)`; `reorderPages(origPos, newPos)` —
   move, no copy flag; `reorderLayer(layerToMove, layerToPutItBefore,
   bMakeCopy, posInLayer, aboveBelowInto)` with `aboveBelowInto`
   0 = above / 1 = below / 2 = into (sub layer);
   `setLayerLocked(layerIndex, frameIndex, bLock, bAllLayers)` — the
   lock is **per frame**, exactly the `LCKb` inside `CELv` our model
   carries as `Cell.locked`; `setMasterPage(PageNum)` /
   `unsetMasterPage()` (CS3) — one master page per document;
   `addMasterPageLayer()` — "the document must already have a master
   page"; adds a master-page layer at the bottom of the hierarchy.
2. **Format dictionary**: `IMPb` (Is Master Page) sits on `PDCv` — a
   page flag, exclusive in practice; `LCKb` parents `CELv` (per-state
   lock, modeled as `Cell.locked`); `LLLi` (nesting level) round-trips
   but sub layers (`LSLv`) parent `ELMv`, not the page layer list —
   the page `LYLv` is flat, matching our `LayerList`.
3. **Sharing needs a model, not commands.** The wire model for shared
   layers is `LSMv` (Layer Shared Definitions, under `MKBv`) holding
   `LSEv` definitions (`UIDs` + `ISPb` + `LAYv`), referenced from page
   layer lists by `LSAv` links (`UIDs` + `ISPb`). Today the reader
   bags `LSMv` verbatim (reader.rs "no document-level model slot") and
   preserves `LSAv` links verbatim inside layer lists; the
   `Document.shared_layers` slot is the `MKBv` legacy layer list (the
   "reduced copy of page 1"), not a sharing registry. Commands that
   share/unshare/detach would have to mint and retarget invisible bag
   nodes — refused until `LSMv`/`LSEv`/`LSAv` are typed (decision
   point deferred; the rows stay M).

## Decision

**Structure commands mutate existing bits; no model change.** Seven
commands (engine names, one per CS6 verb):

- `page.setName {page, name}` — `PGNs`. Captures the page body.
- `page.reorder {from, before}` — moves the page to sit before
  `before` (compensating insert; own-slot/next-slot move is CS6's
  no-op). Captures the whole page list (list-capturing undo, the
  `Library`/`Styles` precedent).
- `layer.setLocked {page, layer, frame, locked, all_layers?}` —
  `Cell.locked` on one layer's cell for `frame`, or every layer's
  cell for `frame` when `all_layers` (CS6's `bAllLayers` ignores
  `layerIndex`). Refuses frames beyond a layer's cell count.
- `layer.reorder {page, from, before, make_copy?}` — moves the layer
  to sit before `before`. `make_copy` deep-copies the layer's cells
  through `clone_element` (fresh registry slots, groups remapped,
  tiles/symbol definitions shared) — the `frame.duplicate` machinery.
  CS6's `posInLayer` and `aboveBelowInto = 2` (into-as-sub-layer) are
  nesting operations on a flat list — refused with a nesting detail.
- `page.setMaster {page}` — sets `IMPb` and clears it on every other
  page (one master per document). Master on the already-master page
  is a no-op.
- `page.unsetMaster {}` — clears `IMPb`; absent master is a no-op.
- `page.addMasterLayer {index?}` — refuses when no master page
  exists; otherwise inserts a fresh empty layer into the master
  page's layer list, bottom-first (CS6's "bottom of the hierarchy";
  CS6's `-1` level). `index` allows another position.

Two new delta variants carry the undo: `PageBody { page, before }`
(in-place page restore — rename, master flips) and `Pages { before }`
(whole-list restore — page reorder). Master flips that touch two
pages compose two `PageBody` captures in a `Group`.

## Consequences

- Engine registry grows 138 → 145; parity moves 162C/45P/67M →
  169C/45P/60M.
- The sharing trio (`setLayerSharing`, `shareLayerToPages`,
  `detachSharedLayer`) stays M with a greppable note pointing here;
  typing `LSMv`/`LSEv`/`LSAv` is its own design (the reader/writer
  already preserve both nodes verbatim, so no round-trip risk).
- `selectAllOnLayer` stays in the selection-tooling family (needs the
  object-selection surface, not this one).
- Deleting the master page (`page.delete` is master-agnostic) leaves
  the document masterless — consistent with `unsetMaster` being a
  no-op then; CS6 prompts, we simply allow it.
