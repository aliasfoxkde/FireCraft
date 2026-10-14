# Decision 0022: shared-layer model (`LSMv`/`LSEv`/`LSAv`) and the sharing commands

Date: 2026-10-13 · Status: accepted · Closes the deferral recorded in
[decision 0021](0021-layer-page-structure.md).

## Context

CS6 shares a layer across pages: the definition lives once, every page
shows a reference. The wire model (dictionary, the only authority — no
corpus fixture carries these nodes, so the observational dictionary
stands unopposed; nothing to arbitrate):

- `LSMv` (child of `MKBv`; also legal under `PDCv`, "appears to always
  be empty" there) — holds `LSEv`*.
- `LSEv` = `UIDs` (string UUID) + `ISPb` (true: primary definition) +
  `LAYv` (the layer body).
- `LSAv` (child of a page's `LYLv`) = `UIDs` + `ISPb` (false: a
  reference) — a layer-list slot that carries no layer body.

Until now both nodes were bagged verbatim (lossless, opaque) and
`Document.shared_layers` remained MKBv's legacy page-1 copy — no
sharing semantics existed (0021).

## Model

- `SharedLayerLink { uid: String, is_primary: bool }` — the `LSAv`
  payload, stored on `Layer.shared_link: Option<_>`. A link layer
  carries no cells: content resolves through the definition. This keeps
  the layer list a `Vec<Layer>` (every consumer unchanged) instead of
  introducing an entry enum through the whole engine.
- `SharedLayerDef { uid: String, is_primary: bool, layer: Layer }` —
  one `LSEv`, in `Document.shared_layer_defs: Vec<_>` (the typed
  `LSMv`). `shared_layers` keeps its legacy meaning.
- UIDs are modeled as `String` (the node is a string UUID). Minted ids
  are UUID-shaped and collision-checked against existing defs and
  links.

## Commands (registry 145 → 148)

CS6 API (Extending Fireworks), engine mapping:

- `dom.setLayerSharing(layerIndex, sharedStatus,
  bUnshareCopiesToAllFrames, bWarnUser)` → `layer.setSharing { page,
  layer, shared, unshare_copies_to_all_frames? }`. `shared=true`
  converts the layer document-wide: mint one `LSEv` from the layer's
  cells, replace the slot with a link, insert links at the same index
  (clamped) into every other page. `shared=false` unshares everywhere:
  each page's link becomes a real layer cloned from the definition —
  flag true fills the page's frame count by repeating the definition's
  cells, false puts content on frame 0 only (CS6: "items are placed
  only on the current frame"); the definition is removed. `bWarnUser`
  is UI-only and dropped. The special layer (`MSLb`, Web Layer) cannot
  be shared.
- `dom.shareLayerToPages(layerNum, addToPages, deleteFromPages)` →
  `layer.shareToPages { page, layer, add_to_pages?, delete_from_pages? }`.
  CS6 takes comma-separated page *names* (UI-era); the engine is
  index-based everywhere, so the payload takes page *indices* —
  recorded deviation. Adds/removes that page's link slot; idempotent
  adds and deletes that change nothing are a `Noop`.
- `dom.detachSharedLayer(layerNum, pageNum)` → `layer.detachShared
  { page, layer }`. One page's link becomes a private copy (frame count
  filled as above); the definition and every other page's link stay.
  CS6's sub-layer auto-detach is moot: `LYLv` is flat (0021).

Deltas: new list-capturing `Delta::SharedLayers { before }` for the
definition list, plus per-page `Delta::Layers` for every touched page
list, composed as a `Group` (LIFO undo).

## Consequences and recorded debts

- Link layers render as empty: resolving shared content at render time
  needs a link-resolution pass in `fwpng` (recorded debt, shared layers
  were invisible on the wire until now, so no render regressed).
- `layer.rename`/`setDisplay`/`setLocked` on a link layer edit the
  link, not the definition — matching the wire (an `LSAv` carries
  nothing to edit). Definition edits go through unshare/re-share.
- No corpus oracle exists for the encoding; the synthetic round-trip
  fixture (write → read → byte-equal) is the gate, per M3 rule.
