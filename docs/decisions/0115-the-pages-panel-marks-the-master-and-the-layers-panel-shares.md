# 0115 — The Pages panel marks the master page and the Layers panel shares layers

Date: 2026-11-12 · Issue: [#136](https://github.com/aliasfoxkde/FireCraft/issues/136) ·
Status: accepted

## Context

The reachability gate measured two families the engine could execute and
the shell's designed surface could not reach:

- the **master page** — `page.setMaster` (CS6's `dom.setMasterPage`),
  `page.unsetMaster` (`dom.unsetMasterPage`) and `page.addMasterLayer`
  (`dom.addMasterPageLayer`);
- the **shared layer** — `layer.setSharing`
  (`dom.setLayerSharing`), `layer.shareToPages`
  (`dom.shareLayerToPages`) and `layer.detachShared`
  (`dom.detachSharedLayer`).

The master page is the document's shared base: one page's content stands
behind every other page's. The shared layer is the other half of the
same idea — one definition, a link per page, so a change to the layer
shows everywhere at once. Both are how a Fireworks user avoids
re-drawing the same furniture on every page.

## Decision

- **The Pages panel carries the master as a star per page.** A filled
  ★ marks the master; ☆ designates the page. The engine allows one
  master per document, so designating a second demotes the first.
- **The Pages panel gains a Master page layers section** — the
  master's own layer list, with a button that adds a layer to it
  (`page.addMasterLayer`). The engine keeps those layers on the master
  page itself, so the section reads that page's list; the Layers panel
  edits them when the master page is the current page.
- **The Layers panel gains a Sharing section.**
  - *share the active layer* — one definition, a link on every page
    (`layer.setSharing`, CS6's "share to all pages");
  - a **page toggle per shared layer** — adds or removes that page's
    link (`layer.shareToPages`, the index-based deviation decision 0022
    records);
  - *unshare* — every page's link becomes that page's own copy. CS6
    asks whether the copies go to every state; the copies fill the
    page's frame span;
  - *detach here* — this page alone gets its own copy; the definition
    and the other pages keep theirs (`layer.detachShared`).
- **The last link cannot be removed.** The command addresses the
  definition *through* a link slot, so a document whose only link went
  away would keep an orphaned definition that nothing can address. The
  toggle is disabled on the last link and says why; *unshare* is the
  way out.
- **A link slot carries no name of its own**, so the Layers panel
  resolves the row's label from the definition.

## The master page's layers stand behind every other page's

The engine modelled the master page as a flag and a layer list; the
renderer walked only the page's own list, so the shared base drew
nothing. The compose walk now visits the master page's layers first —
behind the page's own, which paint over it. A master page with fewer
states than the page repeats its last one, so the shared base never
vanishes on a later state.

## Three layers minted a display flag of `false`

Binding the family exposed a defect under it. `Layer`'s derive-default
is `display: false` — CS6's *smart-reference display* — and three
places minted a layer without overriding it:

- `layer.setSharing` minted the link slots, both the owner's and the
  other pages';
- `layer.shareToPages` minted the link it inserted;
- `page.addMasterLayer` minted the master's layer.

The renderer's walk gates a link's definition on the link's own display
flag, so every shared layer and every master layer rendered *nowhere* —
the feature was reachable and invisible. The reader had the same
default for a link slot, so a saved document lost its shared content
on the way back in. All four now mint a display layer, the way
`layer.add` already did.

The link slot's *visibility* is a different matter: `LSAv` carries only
the uid and the primary flag (decision 0022), so a page's link
visibility does not survive a save. The reader therefore defaults a
link to visible, and the loss is recorded rather than papered over.

## Consequences

- Six more engine commands are reachable from the shell's designed
  surface: the measurement moves from 228 bound commands of 253 to
  **234 of 253** (90.1 % → 92.5 %).
- A document can have a shared base and shared layers — and both
  actually draw.

See also decisions [0104](0104-the-shells-selection-verbs-go-through-the-engine.md),
[0111](0111-the-symbol-panel-carries-the-symbol-family.md),
[0113](0113-the-filters-panel-bakes-and-renames.md) and
[0114](0114-the-properties-panel-carries-the-mask.md) — the same week's
other steps in closing the measured surface gap.
