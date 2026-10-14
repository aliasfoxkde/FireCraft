# Decision 0039 — clipboard surface + the paste-mask trio

**Date:** 2026-10-14 · **Phase:** 8.6/6.6 · **Status:** landed —
`clipboard.copy` / `clipboard.cut` / `clipboard.paste` /
`clipboard.pasteAsMask` / `clipboard.pasteInside` /
`clipboard.pasteFromChannelToChannel`

## Verbs (CS6 API text)

- `dom.clipCopy()` — Fireworks 3. "Copies the selection to the
  clipboard." No arguments.
- `dom.clipCut()` — Fireworks 3. "Cuts the selection to the
  clipboard." No arguments.
- `dom.clipPaste({whatIfResolutionDifferent},
  {whatIfPastingIntoElementMask})` — Fireworks 3/4. "Pastes the
  clipboard contents into the document."
- `dom.clipPasteAsMask(whatIfResolutionDifferent, masktype,
  maskReplaceOptions)` — Fireworks 4. "Pastes the clipboard contents
  into the document as an element mask. Only one element can be
  selected … Fireworks throws an exception" otherwise, "and also if
  there is nothing on the clipboard." `masktype` "image" / "vector" /
  "ask"; "If the clipboard contains a single image, it is pasted as
  an image mask, even if you pass vector." `maskReplaceOptions`
  "replace" / "add" / "ask".
- `dom.clipPasteInside({whatIfResolutionDifferent})` — Fireworks 3;
  the manual retires it in Fireworks 4 "in favor of
  dom.clipPasteAsMask()".
- `dom.clipPasteFromChannelToChannel(fromChannel, toChannel)` —
  Fireworks MX. "Pastes the specified color channel on the clipboard
  into each of the RGB channels of a new image or into the specified
  channel of the selected image, if any." Without a single-bitmap
  selection "a new opaque bitmap is created and the fromChannel is
  pasted in to all three color channels … resulting in a grayscale
  image." Examples: `("red", "red")`, `("green", "alpha")`.

`.jsf` usage in the CS6 Configuration tree: none for the mask trio;
`clipCopy`/`clipCut`/`clipPaste` appear only inside menu-command
shells. The API text is the authority.

## Design

- **Model:** `Document.clipboard: Option<Clipboard>` with
  `Clipboard { elements: Vec<ObjectData> }` — deep object clones in
  capture order, group children captured after their parents so paste
  can mint bottom-up. Editing state, never file content: the format
  has no node for it and the writer never reads the field (the
  `selection` / `jpeg_mask` / `saved_selections` class). Tile data
  rides by reference: copies share bank tile ids, and the fresh-mint
  law (scatter never edits shared tiles in place) keeps them pristine
  for as long as the clipboard holds them — the standing bank-GC debt
  covers their eventual release.
- **Delta:** `Delta::Clipboard { before: Option<Box<Clipboard>> }` —
  whole-clipboard capture, mirroring `Delta::SavedSelections`.
- **`clipboard.copy`** `{}` — deep-captures `object_selection` into
  the clipboard (selection order). Empty selection refuses
  (`InvalidState`; deviation 2). Groups capture their whole subtree.
- **`clipboard.cut`** `{}` — copies, then deletes the selected ids
  through the `object.delete` machinery (detach + `Removed`). An id
  that went stale mid-cut because an ancestor in the same selection
  was deleted first is skipped (its removal rode the ancestor).
- **`clipboard.paste`** `{page, layer, cell}` — mints the clipboard
  elements bottom-up (children first), rewrites each group's children
  through the minted-id map, and attaches the top-level elements to
  the named cell in stored order. Paste-in-place: clones carry their
  world position. Explicit addressing per decision 0037's deviation 2.
- **`clipboard.pasteAsMask`** `{mask_type?, mask_replace_options?}` —
  operates on the object selection (CS6's throw-for-not-exactly-one
  is kept: the command refuses unless `object_selection` holds
  exactly one live id). The clipboard must hold exactly one
  mask-kind element (Image → `GRTi` 1 grayscale, Path → `GRTi` 2
  shape; anything else refuses). The target is detached from its
  container, the mask element is minted fresh, and a mask-structured
  group `[mask, content]` takes the target's owner position — the
  `mask.add` structure, with clipboard content as the mask. When the
  target already sits in a mask group, `mask_replace_options`
  "replace" (default) swaps that group's mask child; "add" and "ask"
  refuse (deviation 5). `mask_type` "ask" refuses; "image" against a
  path clipboard refuses (no rasterizer hook — deviation 4); the
  image-beats-"vector" rule falls out of the kind check.
- **`clipboard.pasteInside`** `{}` — the retired spelling: the
  pasteAsMask core with every option defaulted.
- **`clipboard.pasteFromChannelToChannel`** `{from_channel,
  to_channel, page?, layer?, cell?}` — channels name
  "red"/"green"/"blue"/"alpha". The clipboard's single element must
  be an image. With exactly one image selected: the clipboard's
  channel plane overlays the target's channel, top-left anchored,
  over the shared extent (no resample — deviation 3); the target
  re-tiles over fresh ids (`Mutated` + `Bank`). Otherwise the
  channel plane becomes a new opaque grayscale image (RGB = the
  channel byte everywhere, alpha 255) placed by the explicit
  page/layer/cell.

## Deviations (recorded, not hidden)

1. **Single-document.** `whatIfResolutionDifferent` (all three paste
   verbs) exists for cross-document resolution mismatch; the engine
   is single-document and never resamples (the resample pass is the
   standing 6.4 debt). `whatIfPastingIntoElementMask` is
   mask-edit-mode UI posture — the engine has no mask-edit mode.
2. **Empty/multi selection refuses copy and cut.** CS6's script
   surface does not document the empty-selection case; the engine
   refuses rather than silently keeping a stale clipboard.
3. **No resample.** Channel paste overlays top-left over the shared
   extent; a smaller clipboard clips, a larger one clips to the
   target. Resolution-mismatch dialogs are shell concerns.
4. **No path→image rasterization.** `mask_type: "image"` against a
   path clipboard refuses instead of rasterizing the path; the
   compose crate's raster is golden-pinned and not wired into
   command-land minting yet.
5. **`mask_replace_options` "add" refuses.** Mask-union (folding two
   mask images, or Boolean-op-ing two mask paths) is real work, not
   glue; recorded as debt until a corpus fixture demands the exact
   fold semantics. "ask" refuses everywhere (dialogs are shell).
6. **No OS clipboard.** The clipboard is per-document engine state;
   cross-application paste is the shell's job (serialize → system
   clipboard → deserialize), out of the DOM's reach.

## Consequences

- Parity: three M rows flip M→C and three N rows flip N→C
  (**221 C / 45 P / 5 M / 85 N** at this batch's close); registry
  189 → 195.
- The N-class rationales for clipCopy/clipCut/clipPaste ("app
  session") are retired: the DOM verbs are engine commands; the OS
  bridge remains shell territory (deviation 6).
- Cut of a group still orphans its registered children when they are
  not themselves selected — the standing deep-clone debt (Phase 9)
  now also covers delete/cut.
- Remaining M after this batch: the `filterSelection` bake trio
  (4 rows, one a CS6-typo duplicate) and `setDocumentImageSize`
  (rides the resample pass).
