# 0122 — The selective JPEG holds the mask in the document and the second quality in the export settings

Date: 2026-11-16 · Tickets: [T7](../tasks/T7-optimize-panel.md) ·
Status: accepted

## Context

CS6's *selective JPEG* compresses one region of a JPEG at a
different quality from the rest of the image. Its reference manual
("Selectively compress areas of a JPEG") describes the workflow: mark
an area with a marquee, save that selection as a JPEG mask
(`Modify ▸ Selective JPEG ▸ Save Selection as JPEG Mask`), then turn
on **Enable Selective Quality** in the Optimize panel and enter a
value. The manual's own example shows the *selected* area at quality
90 while the *unselected* area sits at 50, and it states the rule
plainly: entering a low value compresses the selected area more than
the rest of the image. So the entered value belongs to the **masked**
region, and the rest of the image keeps the main Quality slider's
value. The manual also carries two flags FireCraft does not yet model
(*Preserve Text Quality*, *Preserve Button Quality*) and an overlay
colour that affects only the on-screen overlay, never the output.

The CS6 binary's own property table names the same settings
(`jpegSelQuality`, `jpegSelQualityEnabled`, `jpegSelPreserveButtons`,
`jpegSelPreserveText`) beside `jpegQuality`, `jpegSmoothness` and
`jpegSubsampling`. Those names are the behavioral authority for this
decision. (The manual text and the binary strings are working notes
under `/var/tmp/`, per the repository's `source/` rules; they are not
committed.)

Two questions had to be answered before anything could be built.

**Where the mask lives.** FireCraft already has it:
`doc::Document::jpeg_mask` holds a `SelectionMask` — one coverage byte
per image pixel — and `mask.saveJPEG` / `mask.restoreJPEG` /
`mask.clearJPEG` in the engine's mask catalog move the selection in
and out of it. The mask is editing state and never file content,
because the format has no node for it: CS6 keeps the mask in the
document too, and there is no `EXPv` child that carries a bitmap.

**Where the second quality lives.** This build has no home for it.
`ExportOptions` is the mapped `EXPv` record, and its JPEG children are
`JPQi` (quality), `JPSi` (smoothing), `JPOi` (colour) and `JPPb`
(progressive). The dictionary documents no node for CS6's selective
quality, and no corpus file has been observed carrying one. The
value is therefore a **FireCraft-side setting**: recorded in
`ExportOptions`, honored at export time, and not yet written to the
file. That is the C4.01 fallback — the feature ships on a
FireCraft-side setting whose wire home waits for corpus evidence,
rather than waiting for that evidence first.

The format constrains the implementation more than the model does. A
JPEG scan carries **one quantization table per component**, so a
single scan cannot hold two qualities; the encoder would need a
different table per 8×8 block to do what CS6's panel describes. The
`jpeg-encoder` crate offers no per-block tables.

## Decision

- **The mask stays where it is.** `Document::jpeg_mask` is the mask;
  the export reads it. The panel's *use current selection* knob lets
  the document's current selection stand in for the saved mask, which
  is the one step of CS6's workflow FireCraft can fold into a single
  control.
- **The second quality is a FireCraft-side `ExportOptions` field.**
  `jpeg_selective_quality` carries the entered value,
  `jpeg_selective_enabled` carries CS6's *Enable Selective Quality*
  checkbox, and `jpeg_selective_use_selection` carries FireCraft's
  mask source. None of the three has a proven node, so the writer
  emits none of them: the value is honored at export time and is lost
  when the document is saved. The fields say so in their doc comments.
- **The encoder composites the two regions.** The image is encoded
  once at the main quality and once at the selective quality, both
  are decoded back, and the two decoded images are blended per the
  mask's coverage (the masked pixels come from the selective encode,
  the rest from the main one). The blend is then encoded once more,
  at the **better** of the two qualities, so the region that was
  compressed well is not degraded a second time by the final pass.
- **The main quality covers the unmasked region; the selective
  quality covers the masked one.** That is CS6's rule, and the tests
  pin it in both directions: a high entered value lifts the masked
  region's PSNR, a low one drops it.
- **The Preserve Text / Preserve Button flags are not modeled.** They
  change which pixels count as masked, FireCraft would need to know
  which objects are text and which are button symbols, and CS6's own
  rule for that union is unobserved. Until it is, the mask is the
  user's.

## Consequences

- The behavior CS6's manual describes is reproducible here, and the
  test that proves it is a measurement rather than a shape check: the
  masked region's PSNR against the original rises with the entered
  quality and the unmasked region's stays at the main quality's
  level, within the tolerance the composite's extra generation costs
  (a few dB — the blend is encoded once more than CS6 encodes its
  regions).
- The bytes are not CS6's bytes. The file carries the two qualities
  approximately: the well-preserved region pays one extra generation
  and the compressed region pays the second one, and the file size
  lands between the two uniform encodes rather than exactly on either.
  A corpus observation of a CS6-written selective JPEG would still be
  needed to pin CS6's own layout.
- The setting does not survive a save. The three `ExportOptions`
  fields are FireCraft-side until a node is pinned; the mask never
  survived a save either, so the pair is consistent — but a document
  that is saved and reopened loses the selective quality.
- The two flags CS6 offers for text and buttons stay unimplemented,
  and the panel does not offer them: an unimplemented knob is worse
  than a missing one.

See also decision
[0010](0010-expv-export-options.md) — the `EXPv` mapping this
decision's fields sit beside, and the slice-level `EXPv` that carries
per-slice export overrides.
