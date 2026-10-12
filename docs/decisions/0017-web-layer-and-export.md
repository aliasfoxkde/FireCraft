# Decision 0017 — Web layer model, export purity, and naming provenance

Date: 2026-10-12 · Milestone: M11 · Status: accepted

## Context

CS6 models web output as *slices*: objects on a per-page Web Layer that
partition the canvas for export. The wire format gives the model no
help — `URLv` carries slices and hotspots alike with no slice-vs-hotspot
bit, behaviors are an untyped `BETi` list whose payloads the dictionary
only partially documents, and CS6's exporter emits HTML whose Adobe
whitespace/quoting is noise. FireCraft needed a model that round-trips
losslessly and an exporter whose output can be gated against the CS6
`HTML Code/` templates (behavioral specs, clean-room readable) without
chasing byte identity.

## Decision

**1. The Web Layer is a real layer; a slice is a real object.** Every
page provisions one Web Layer at creation (`Layer::has_url` +
`is_special`, the `URLb`/`MSLb` flags); it blocks deletion and reorder.
Slices and hotspots are `SliceObject`s in the registry like any object,
so create/mutate/undo/replay/fuzz all come free through the registry,
and the file format already round-tripped them before M11 wrote a
single export line. The slice/hotspot distinction rides `TSLi`
presence, not a new bit: `TSLi`-less objects are hotspots — map-only
areas at export (`<area>` with no image file), exactly what a `URLv`
without `TSLi` means on the wire.

**2. Behaviors: typed where the templates consume, raw everywhere
else.** `BETi` kinds the HTML-facing templates actually read (swap
image 2, button down 4, swap restore 5, button highlight 6, button
restore 7) are modeled and drive the `MM_swapImage`/`MM_nbGroup` hook
emission. Their parameters (swap target file, nav group) ride the
slice's `DTAv` custom-data map (`slice.setData`) — the same `DCEv`
key/value machinery symbol instances already carry — under documented
keys (`swapTarget`). Popup menu (kind 9) and anything unobserved stay
raw (`SliceBehavior { kind }`) and are preserved verbatim; lossless
beats speculative mapping.

**3. Export is a pure function of the document.** Every workflow is
`(&Document, options) -> {text files, images}` in the `webexport`
crate: no engine mutation, no I/O side effects, deterministic bytes
(pinned by double-run byte-equality tests). The GUI export dialog, the
`firecraft-cli export-web` verb, and MCP/CLI dispatch all funnel into
`app_cli::ops::export_web` → the same functions — one entry point,
GUI ≡ channel ≡ MCP byte-equal by construction (battery-tested). This
is what makes goldens possible: same input, same bytes, forever.

**4. Auto-naming is export-time, never stored.** The SLICES.XTT
`CellName` scheme (`naming::cell_name`) applies at export: a named
slice exports under its cleaned name; anything else falls back to
`n{doc}_{row}_{col}` (`_r_c` suffix only on multi-cell grids).
Cleaning drops every character outside `[A-Za-z0-9_\-.:]` and prefixes
a leading digit/punctuation with `n`; a name that cleans to nothing
falls through to the auto scheme. The slice object never stores the
exported name — renaming a slice is a model edit, not a file rename.
A slice's per-slice `EXPv` override chooses its codec; slices without
one export PNG so the exported pixels are exactly the composer's (the
pixel-identity floor holds by construction).

**5. The gate is structural, not byte-identical.** Adobe's whitespace,
quoting, and entity choices make byte identity meaningless, so
`xtask web-golden` pins the *normalized structure* of the exported
HTML/CSS: tags in document order with attribute names sorted per tag,
CSS selectors with property names (values elided), `<style>` bodies
re-normalized through the same CSS normalizer, plus the emitted
file-name set (auto-naming is structure). Floor: structural distance 0
against the blessed `<fixture>.webpin` — the shape the emitter
committed to, derived from the template spec. Re-bless on purpose,
diverge = gate failure.

## Consequences

- The M11 corpus fixture (`sliced.fw.png`) is built through the engine
  registry — linked rect slice with a swap behavior + preserved popup
  kind, ellipse and polygon slices, a link hotspot — and pins the
  whole path: corpus round-trip, golden render, web structural pin,
  SVG goldens.
- Symbol instances are **ineligible** for the SVG pixel-golden loops
  (the export report's notes are the eligibility oracle): compose draws
  instances (decision 0014) but the SVG drops them, so the comparison
  would fail by construction. This closed an M10-latent gate breakage:
  `rich.fw.png` had made `svg-golden`/`svg-roundtrip` fail the moment
  either was next run; the GitForge pipeline does not execute those
  gates, so only this milestone's full-gate sweep surfaced it.
- The exporter treats a hotspot's `<area>` as anchored to the cell
  containing its center (coordinates relative to that host image's
  origin) — one area per hotspot, not a per-region clip. Recorded here
  because it is a deliberate simplification of CS6's cell scatter,
  revisitable only with corpus evidence of per-region duplication.
- Popup-menu *rendering* (`mm_menu.js` generation) stays a non-goal:
  `BETi` 9 payloads survive verbatim; typed menu export needs observed
  behavior the templates do not specify.
