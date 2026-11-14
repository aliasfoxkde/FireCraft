# 0114 — The Properties panel carries the element mask

Date: 2026-11-12 · Issue: [#136](https://github.com/aliasfoxkde/FireCraft/issues/136) ·
Status: accepted

## Context

CS6's Modify ▸ Mask menu carried the mask's own verbs: mask the
selection, enable or disable the mask, link or unlink it, apply or
discard it. The shell bound those through the menu, but the engine
carried four more mask verbs with no door at all:

- `mask.moveBy` — move the *mask*, leaving the content it hides
  where it is;
- `mask.moveContentsBy` — move the *content*, leaving the mask in
  place;
- `mask.setMode` — convert the mask between CS6's raster (a
  grayscale bitmap) and vector (a shape) form;
- `mask.setShowAttrs` — show or hide the mask's own fill and stroke.

The two moves are the ones a daily driver reaches for: a mask that
does not line up with the artwork is corrected by nudging the mask
or the content, not by rebuilding the mask.

## Decision

- **The Properties panel grows a Mask section**, shown only when the
  selection actually carries an element mask. Every mask verb
  refuses on an unmasked object, so an always-present section would
  only offer dead buttons.
- **The panel asks the engine who owns the mask**
  (`element_mask_state`) rather than re-deriving the ownership rule
  in the shell; the engine's rule is the one the commands apply.
- **The section carries the two nudges as arrow rows** — the mask's
  own and the content's — so the two doors are visibly distinct.
- **The conversion button follows the mask's kind**: a raster mask
  offers "mask to path", a vector mask offers "mask to image".
- **The attributes button is its own inverse** — "hide mask
  attributes" / "show mask attributes" — and writes `MNAb`.

## Evidence

- `mask.moveBy` refuses when no target carries a mask and translates
  the mask child; `mask.moveContentsBy` leaves the mask in place and
  is a no-op when nothing matched. The two verbs are therefore not
  interchangeable, and the panel's own test asserts that the mask
  nudge leaves the content alone and the content nudge leaves the
  mask alone.
- `mask.setMode` needs the matching kind ("mask to image" needs a
  path mask, "mask to path" needs an image mask) and rewrites the
  mask child and the group's `GRTi` to follow.
- `mask.setShowAttrs` is the one mask verb that also accepts a
  *disabled* mask — the `GRTi` 0 structure — so the panel offers it
  for a disabled mask too, and offers the conversion and the nudges
  only for an enabled one.
- The reachability gate measured all four commands as carried by the
  engine and unbound by the shell.

## The engine's uniform mask read its coverage through the wrong view

Binding the family exposed a defect under it. `mask.add` mints a
uniform mask as a **solid matte** — the file's flat color for the
tile, with no payload — and the engine documents that matte as the
"reveal all" mask: opaque. But `mask_gray_plane` read the mask's
pixels through `gather`, the *editable-pixels* view, which skips a
matte slot entirely. The renderer bakes a matte to the color it
stands for (decision 0034), so the mask the user sees and the mask
the engine measured disagreed:

- `mask.setMode` "mask to path" found no coverage and refused on
  the very masks `mask.add` mints;
- `mask.remove` "apply" baked an all-zero coverage into the
  content — a "reveal all" mask erased the artwork it covered.

The fix reads the coverage through `assemble_rgba`, the renderer's
view, which is what the clipboard's own mask reader
(`clipboard::mask_coverage`, decision 0058) already does. The engine
keeps both views: `gather` stays the editable-pixels view, and a
matte still occupies its grid slot there.

## Consequences

- Four more engine commands are reachable from the shell's designed
  surface, and the mask family is fully bound: the measurement moves
  from 224 bound commands of 253 to **228 of 253** (88.5 % → 90.1 %).
- A mask can be aligned with its content, converted between CS6's
  two mask forms, and its own attributes hidden or shown — all from
  the panel that owns the object.

See also decisions [0104](0104-the-shells-selection-verbs-go-through-the-engine.md),
[0111](0111-the-symbol-panel-carries-the-symbol-family.md) and
[0113](0113-the-filters-panel-bakes-and-renames.md) — the same week's
other steps in closing the measured surface gap.
