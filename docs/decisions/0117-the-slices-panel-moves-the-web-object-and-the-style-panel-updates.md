# 0117 — The Slices panel moves the web object, the Styles panel updates the named style

Date: 2026-11-12 · Issue: [#136](https://github.com/aliasfoxkde/FireCraft/issues/136) ·
Status: accepted

## Context

After the panel bindings of decisions 0114–0116, the reachability gate
measured fourteen engine commands the shell's designed surface still
could not reach. Five of them are properties a user reaches for
routinely, and the panels that own them had no way to say so:

- the **web object's rectangle** — `hotspot.setRectangle`, CS6's
  `dom.setHotspotRectangle`;
- the **background slice's knobs** — `slice.setBackground`
  (`CBTb` and the cell-background family);
- the **selection as a web object** — `hotspot.copyToHotspot`, CS6's
  `dom.copyToHotspot`;
- the **named style** — `style.redefineByName`, CS6's
  `redefineStyleByName`;
- the **bitmap's channel** — `clipboard.pasteFromChannelToChannel`.

The rest were counted for a reason, and the reasons are recorded
below rather than papered over with a button that does nothing.

## Decision

- **The Slices panel's inspector carries the web object's rectangle.**
  Four fields (left, top, right, bottom) and two buttons: *Move there*
  moves the object, *Copy there* copies it and leaves the copy
  selected (`hotspot.setRectangle`). The verb addresses the
  document's selection, so the object is selected before it runs.
- **The Slices panel's inspector carries the background knobs** — the
  background flag, the repeat mode, the attachment, and the horizontal
  and vertical position with their values. Each control writes the one
  field the user changed: the engine's fields are tri-state, so an
  absent field leaves the stored value alone and the file keeps its
  own record of which knobs were ever set (`slice.setBackground`).
- **The Slices panel's header carries *Selection → hotspot* and
  *Selection → slice*.** The selection becomes a web object of its
  own (`hotspot.copyToHotspot`). CS6 asks what to do when several
  objects are selected; the headless path has no one to ask, so the
  panel states its choice — one web object per selected object.
- **The Styles panel gains *Update Named*.** The existing style of the
  typed name takes the selection's appearance (`style.redefineByName`).
  It never creates: a name with no style is refused, where *New From
  Selection* defines or replaces one.
- **The Properties panel's bitmap section gains a Channels section.**
  The clipboard's channel lands in the image's own
  (`clipboard.pasteFromChannelToChannel`) — the habit CS6 users have of
  moving an alpha channel from one bitmap to another. The verb writes
  onto the single selected image, so the image is selected first.

## What stays out, and why

The remaining nine commands are not bound by this decision, and each
one has a reason that is a design question rather than an oversight:

- **`object.resize`** — the transform path refuses images, and the
  Properties panel's X/Y/W/H strip already moves *and* resizes the
  selection. The engine's verb only scales in place, and no
  user-facing action yet distinguishes "scale in place" from "set the
  bounds" — the CS6 *Transform ▸ Scale ▸ To…* dialog that would is
  not built. A second W/H row in the same panel would be a near-duplicate
  of the first, which is the shape of a fake binding.
- **`object.setGroupType`** — the engine accepts only the plain group
  type and refuses the mask-group spellings, so the only action it
  could carry would leave the document unchanged.
- **`document.deleteAllInDocument`** — destructive, with no CS6 menu
  row of its own. The command palette carries it; a panel button that
  empties a document needs a confirmed action that does not exist yet.
- **`autoshape.rect` and `autoshape.redraw`** — the Auto Shapes
  library has no rounded or chamfered rectangle entry yet, and the
  redraw has no faithful source: FireCraft bakes a shape's geometry at
  insert, so the parameters a redraw would re-run are not persisted
  anywhere. Re-generating from the panel's current parameters would
  move the shape rather than redraw it.
- **`path.warp` and `path.cropSelection`** — both are canvas gestures
  (the push tool and the crop tool), not panel fields. They need the
  drag interaction the canvas owns.
- **`slice.appendPoint` and `hotspot.appendPoint`** — appending a
  point to a polygon web object needs a polyline editor the shell does
  not have.

The gate's ratchet stays where the measurements put it; these nine
remain measured gaps with reasons, not silent ones.

## Consequences

- Five more engine commands are reachable from the shell's designed
  surface: the measurement moves from 239 bound commands of 253 to
  **244 of 253** (94.5 % → 96.4 %).
- A web object can be moved and copied by rectangle, carry its
  background knobs, and be made from an ordinary selection; a named
  style can be updated in place; a bitmap can receive a channel.

See also decisions [0104](0104-the-shells-selection-verbs-go-through-the-engine.md),
[0113](0113-the-filters-panel-bakes-and-renames.md),
[0114](0114-the-properties-panel-carries-the-mask.md),
[0115](0115-the-pages-panel-marks-the-master-and-the-layers-panel-shares.md) and
[0116](0116-the-panel-edits-the-rectangle-the-pixel-mask-and-the-pattern.md) —
the same week's other steps in closing the measured surface gap.
