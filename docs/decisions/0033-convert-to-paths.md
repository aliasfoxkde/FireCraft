# Decision 0033 — convertToPaths (text → glyph outlines)

**Date:** 2026-10-14 · **Phase:** 8.4 · **Status:** implemented

## Verb (CS6 API text)

- `dom.convertToPaths()` (Fireworks 3), no arguments, returns
  nothing: "Converts the selected text items into editable paths."

## Command

`text.convertToPaths {}` (selection-driven) in `catalog/text.rs`.
The engine now depends on the `text` crate (tier 3 → tier 2, the
compose precedent) so the command can run the layout engine.

## Design

**In-place variant swap.** Each selected text object's registry slot
becomes a `PathObject` under the *same id* — z-order, cell membership,
and any selection membership survive untouched. Undo restores the
`TextObject` whole (`ObjectInverse::Mutated` captures the entire
object, so a variant swap is delta-safe; this is the first command to
change an object's kind in place).

**The renderer's synthetic path is the contract.** `render_text`
already lowers text by running `glyph_contours` and painting the
result as a synthetic path (default fill opaque black, `FCLi`
override, no stroke, nonzero winding). The converted path is exactly
that synthetic object plus the text object's display fields (name,
opacity, blend, visibility, lock, effects). Consequence — proved by a
golden test: `render_subset` pixels are **byte-identical** before and
after conversion.

**Geometry source.** `text::TextEngine::glyph_contours` returns
world-space contours (frame origin and `MTXv` transform baked in by
`finish`), unhinted outlines as pure curve scalings — the same
platform-stable bytes the renderer uses. Glyph counters (the holes in
`o`, `e`) ride as extra contours with opposite winding; `even_odd`
stays `false` because font outlines fill nonzero, matching both the
renderer and the fonts.

**Ink-less text.** Empty runs produce no contours; the object leaves
nothing behind and is removed (detach + registry remove +
`Removed`), the same empty-result contract as `path.boolean` and
`path.cropSelection`.

## Deviations (recorded, not hidden)

1. **Selection-driven scope** — the dom verb is argument-free; the
   CS6 tool works on the selection. Non-text members refuse the whole
   command (cropSelection consistency).
2. **One path per text item** — CS6's wording ("text items into
   editable paths") reads as one converted object per item; we do not
   explode per-glyph objects. Per-glyph splitting remains available
   through `path.splitPaths` on the converted result.
3. **No corpus oracle** — no `.jsf` usage of the verb in the CS6
   Configuration tree; the API text plus the renderer's own behavior
   are the authority.
4. **Per-run styling collapses** — `glyph_contours` lays out over the
   mapped style set (`TFSv`), exactly as the renderer does; a
   multi-run object with divergent run styles converts with the
   mapped appearance, and per-run colors beyond the mapped fill are
   already a recorded renderer limitation (not new here).

## Consequences

- Parity 202 C / 45 P / 26→25 M / 84 N; the text family now covers
  create/edit/attach/detach/convert.
- Converted paths are editable with the whole vector surface
  (knife, split, booleans, point edits) — the point of the verb.
- The pixel-identity golden (`convert_to_paths_paints_like_the_text_did`)
  pins the renderer contract: if `render_text` and the conversion
  ever diverge, that test fails.
- Engine → text edge added; `Cargo.lock`/`fuzz/Cargo.lock` moved
  (image re-ship required before push).
