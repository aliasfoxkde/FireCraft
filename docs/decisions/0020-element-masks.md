# 0020 — Element masks are mask-structured groups; pixel masks stay planes

- **Status:** Accepted (post-roadmap gap close, 2026-10-14)
- **Deciders:** Michael Kinney, Claude
- **Scope:** `doc::model` (`GroupObject`, `Document`), `doc::fwmap`
  (`LNKb`), `engine::catalog::mask` (new family), the `bitmap.*`
  pixel-mask rows, and the parity matrix's mask rows.
- **Supersedes:** the "element/pixel/JPEG masks (~14 verbs)" gap note in
  the parity matrix · **Superseded by:** none

## Context

The mask family is the largest remaining DOM gap (18 M rows, 1 P).
The model already carries half the format truth: `GroupObject.group_type`
(`GRTi`: 0 regular, 1 grayscale mask, 2 shape mask — `ELMv[0]` = mask,
`ELMv[1]` = masked) and `GroupObject.mask_hidden` (`MNAb`). Missing:
`LNKb` (mask-group link) and every command that manipulates masks.

## Evidence

1. **Format dictionary**: mask groups are plain `GRPv` with `GRTi`
   1 (grayscale) or 2 (shape); `ELMv[0]` is the mask, `ELMv[1]` the
   masked content. `LNKb` on `GRPv`: "for mask groups, controls whether
   the mask moves when the content is moved. Never present with value
   `true`; `true` is implied by its absence" — `false` = unlinked.
   `MNAb` (modeled as `mask_hidden`) hides the mask's own fill/stroke.
   No JPEG-mask node exists (`JPQi`/`JPSi`/`JPOi`/`JPPb` are export
   options), so the "Selective JPEG mask" is editing state, like
   `Document.selection`.
2. **CS6 signatures** ("Extending Fireworks"): `addElementMask(mode)`
   modes "reveal all" / "hide all" / "reveal selection" / "hide
   selection" (the selection modes degrade to the all modes without a
   pixel selection); exactly one element must be selected, and an
   existing element mask is replaced. `enableElementMask(enable, …)`
   throws when there is no mask. `linkElementMask(…, bLink)` throws on
   no-mask. `moveElementMaskBy(delta)` moves the masks of every masked
   element in the selection and throws if none has one.
   `moveMaskGroupContentsBy(delta)` moves the content, not the mask.
   `removeElementMask(whatIfElementIsAnImage)`: "apply" / "discard" /
   "ask". `setElementMaskMode(mode)`: "mask to image" / "mask to path".
   `setElementMaskShowAttrs(bShow)`: vector-mask fill/stroke visibility.
   `convertPathToMarquee(mode, featherAmount)`: "hard edge" /
   "antialias" / "feather". `setPixelMask(mask, howToCombineMasks)`:
   replace / add / subtract / intersect (deprecated alias of
   `setSelectionMask`, which the select family already implements).
   `getPixelMask` is a deprecated read-only accessor. The
   `clipPasteAsMask` / `clipPasteInside` pair is clipboard-dependent.
3. **Engine machinery in place**: `Delta::Selection` (plane undo),
   `engine::mask::{fold, stamp_polygon, feather}` (plane combine and
   build), `catalog::object::{find_owner, detach, attach_cell,
   translate_object}` (group surgery and mask moves), and
   `vector::flatten` (path → polyline) for rasterization.

## Decision

**Element masks are mask-structured groups, manipulated as such.**
A mask group is `GRTi` 1 or 2 with `ELMv = [mask, content]`; every
element-mask command resolves its target through `find_owner` to the
owning group. Command surface (engine names, one per CS6 verb):

| Command | CS6 verb | Application |
|---|---|---|
| `mask.add` | addElementMask | build the mask image (uniform white/black, or selection-alpha gray for the selection modes), wrap content in `GRTi` 1 group at the content's owner position; replaces an existing mask by building fresh |
| `mask.enable` | enableElementMask | toggle the owning group `GRTi` 0 ↔ derived-from-mask-kind (Image → 1, Path → 2); `GRTi` 0 with mask-first child order *is* the disabled mask (flagged design point below) |
| `mask.link` | linkElementMask | set `mask_linked` (`LNKb`) on the owning group |
| `mask.moveBy` | moveElementMaskBy | translate the mask child (`ELMv[0]`) of each masked target via `translate_object`; error when no target is masked |
| `mask.moveContentsBy` | moveMaskGroupContentsBy | translate the content child (`ELMv[1]`), or the element itself when the target is masked content |
| `mask.remove` | removeElementMask | ungroup, keep the content at the group's owner position; "apply" first multiplies the image content's alpha by the mask plane (bitmap or rasterized-vector mask); "discard" drops it; "ask" refused (no dialogs) |
| `mask.setMode` | setElementMaskMode | "mask to image": rasterize the path mask (`vector::flatten` + `stamp_polygon`) into a fresh image; "mask to path": threshold the mask gray at 128 and trace contours; either way the mask child is replaced and `GRTi` follows the kind |
| `mask.setShowAttrs` | setElementMaskShowAttrs | `mask_hidden = !show` (`MNAb`) |
| `mask.pathToMarquee` | convertPathToMarquee | rasterize the path into a selection plane over the target image; "feather" runs the plane through `feather(radius)`; "hard edge"/"antialias" stamp binary (recorded: no AA sampling) |
| `mask.marqueeToPath` | convertMarqueeToPath | trace the active selection plane (marching squares, ≥128) into a path of linear contours, minted top of the current cell |

**Pixel-mask rows stay planes.** `bitmap.movePixelMaskBy` translates
`Document.selection` (edge-clipping shift); `bitmap.setPixelMask`
folds a caller-supplied plane through replace/add/subtract/intersect
(`fold`, null mask = clear). Both capture `Delta::Selection`.

**The JPEG mask is a second editing-state slot.** `Document.jpeg_mask:
Option<SelectionMask>` beside `selection` (same comment contract: never
file content, the writer never reads it), with a parallel
`Delta::JpegMask` capture. `mask.saveJPEG` / `mask.restoreJPEG` /
`mask.clearJPEG` move the current selection in and out of it.

**Model addition: `GroupObject.mask_linked: Option<bool>`.** `None` =
node absent = linked (the dictionary's implied default); `Some(false)`
writes `LNKb false` between `MNAb` and `VISb` (dictionary order).
`Option` keeps old serialized documents deserializing unchanged.

**Refusals.** Not-one-target (CS6 throws): `mask.add` / `mask.enable` /
`mask.link` / `mask.setMode`. No owning mask group: enable/link/setMode/
remove/moveBy (CS6 throws). No active selection: `mask.marqueeToPath`,
`bitmap.movePixelMaskBy`, and the JPEG restore/clear. Dialog values
("ask") refuse with named detail. `bEnterMaskEditMode` and
`selectAndEnterPaintModeIfPossible` are UI-mode flags — accepted and
ignored, recorded here as unmodeled surface. `clipPasteAsMask` /
`clipPasteInside` stay M until a clipboard surface exists.

**Undo.** Group surgery captures container `Mutated` inverses; freshly
minted objects (mask images, group shells, replacement masks, traced
paths) each carry a `Created` inverse pushed **in mint order** — the
LIFO retraction rule from the frame family applies to every minting
command in this family. Plane ops capture `Delta::Selection` /
`Delta::JpegMask`.

## Falsification path

A Wine-oracle playback scripting `addElementMask("reveal all")`,
`linkElementMask(…, false)`, `setElementMaskShowAttrs(false)`, and
`enableElementMask(false)`, then diffing the written `GRPv`. Two
commitments are corpus-gated: (a) **disabled-mask representation** —
this design encodes it as `GRTi` 0 with the mask-first child order
preserved; if CS6 instead writes a dedicated node (or drops the group),
the corpus wins; (b) `enableElementMask`'s re-enable mode re-derives
grayscale-vs-shape from the mask child's kind, not from stored state.
`convertMarqueeToPath`'s tracer and `mask.setMode`'s threshold are
lossy by nature; CS6's own tracing is too — byte-level equality is not
the gate, structural equivalence is.

## Consequences

- Parity rows move: `addElementMask`, `enableElementMask`,
  `linkElementMask`, `moveElementMaskBy`, `moveMaskGroupContentsBy`,
  `removeElementMask`, `setElementMaskMode`,
  `setElementMaskShowAttrs`, `movePixelMaskBy`, `setPixelMask`,
  `saveJPEGMask`, `restoreJPEGMask`, `clearJPEGMask`,
  `convertPathToMarquee`, `convertMarqueeToPath` → C (15);
  `getPixelMask` → N (deprecated accessor); `clipPasteAsMask` /
  `clipPasteInside` stay M (clipboard); `moveSelectionMaskBy` stays P.
- Engine call table 124 → 139. Recorded limitations: no
  anti-aliasing in path→plane stamping; "apply" on a vector mask
  rasterizes with the same binary stamp; dialog flows ("ask") refuse.
- The fuzzer gains the plane ops and the attribute/flip commands
  (`link`, `enable`, `setShowAttrs`, `moveBy`, `moveContentsBy`,
  pixel-mask ops, JPEG stash); `mask.add` / `mask.remove` /
  `mask.setMode` / the converters are structure-minting and join the
  fixture suite instead (mint-order undo assertions mandatory).
