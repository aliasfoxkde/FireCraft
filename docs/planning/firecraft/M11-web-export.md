# M11 — Web export

> Roadmap: [ROADMAP.md](ROADMAP.md) · Strategy: [../../PLANNING.md](../../PLANNING.md)

**Objective:** FireCraft gains CS6's web layer and export workflows:
slices (rect/ellipse/polygon) and hotspots through the registry on a
per-page Web Layer (`URLb`/`MSLb`), behaviors (`BEVv` — simple
rollover, swap image, nav bar; popup menu `BETi` 9 preserved), slice
auto-naming, and the export workflows — Images Only (per-slice files),
HTML+Images (the classic shim table), CSS+Images (absolutely-positioned
layers), CSS Sprites, Pages/States/Layers To Files — plus states →
animated GIF and the CSS Properties extraction CS6 ships as a panel.
**Gate:** exported HTML/CSS/images from corpus docs match the CS6
structure at a recorded floor (normalized structural diff); slice
corpus floor + round-trip floors rise.

**Inputs:** the model seeds (`doc::SliceObject` = `URLv` with its
per-slice `EXPv` override — already round-trips; `Layer::has_url` +
`is_special` = the Web Layer; `ExportOptions` = `EXPv`); the node
dictionary (`URLv`/`URSi`/`TSLi`/`BEVv`/`BEHv`/`BETi` 9/`CBAi`/
`CBHi`/`CBRi`/`CBVi`/`CHVi`/`CVVi`/`CBTb`/`FILs`/`TDTs`/`URLb`/
`MSLb`); the CS6 `Configuration/HTML Code/` templates as *behavioral
specs* (clean-room readable: `Dreamweaver/SLICES.XTT` = the HTML+
Images table structure incl. shim cells, `name_rX_cY` naming, usemap
emission, `MM_nbGroup`/`MM_swapImage` hooks; `CssLayers.htt` = the
CSS+Images structure); the M8 io codecs (PNG/GIF/JPEG encoders, the
animated-GIF browser-disposal path); the M5+ shell export dialog
(2-Up preview) and command registry.

## Design line: slices are objects on a special layer; export is a
pure function of the document

The Web Layer is a real layer per page (`has_url` + `is_special`,
blocks deletion, holds slices + hotspots) — not a parallel tree. A
slice is `SliceObject` in the registry like any object, so create /
mutate / undo / replay / fuzz all come free through the registry, and
the file format already round-trips it. Behaviors ride `BEVv` on the
slice: FireCraft models the HTML-facing kinds the templates consume
(swap-image / nav-bar / simple rollover, typed) and preserves
everything else (popup menus, unknown types) verbatim — lossless, no
speculative mapping. Export workflows are pure functions
`(&Document, options) -> {text files, images}` in a `webexport`
crate: no engine mutation, no I/O side effects, deterministic bytes —
so goldens can pin structure and the shell/CLI/scripting all call the
same entry points. Auto-naming is the documented element scheme
applied at export time (never stored on the slice). The normalized
gate: CS6's exporter is the oracle for *structure* (table shape, tag
order, attribute presence), recorded as a floor on normalized-diff
distance for our fixtures — not byte identity, which Adobe's
whitespace and quoting make meaningless.

## Tasks

1. **Web layer + slice commands** — page provisioning carries a Web
   Layer (`page.add` creates one; `MSLb` protects it from deletion /
   reorder); `slice.create` (rect/ellipse/polygon), `hotspot.create`,
   `slice.setLink` (URL/alt/target), `slice.setType` (foreground/
   background), `slice.setBackground` (repeat/attachment/position/
   color — the cell-background family), `slice.setExport` (per-slice
   `EXPv` override); journal/undo/replay for all; fuzz arms in the
   driver. **Stopping condition:** a doc with slices round-trips
   byte-comparably through the M3 battery rules and every new command
   passes the four fuzz invariants.
2. **Behaviors (`BEVv`)** — model + reader/writer: typed
   swap-image / nav-bar / simple-rollover kinds, `BETi` raw
   preservation for popup menu (9) and anything unobserved, unknown
   `BEHv` children bagged verbatim; `slice.addBehavior`/
   `removeBehavior`; corpus files carrying `BEVv` parsed without
   panic. **Stopping condition:** a behavior-bearing file round-trips
   byte-comparably and behaviors survive `slice.setLink` edits.
3. **HTML+Images + Images Only export** — `webexport`: per-slice
   region rasters through compose (slice rect ∩ page, slice's `EXPv`
   override → M8 encoders), the shim table structure from the
   SLICES.XTT spec (row/column scan of slice bounds, colspan/rowspan
   for named cells, 1px shim edges), `<img name="doc_rX_cY">` cells,
   `usemap` emission for ellipse/polygon hotspots, behavior hooks
   (`MM_swapImage`, `MM_nbGroup`) wired from the model; slice
   auto-naming (the 8-element scheme, export-time only).
   **Stopping condition:** a sliced doc exports HTML+images that
   render the same pixels the composer draws; the HTML's structure
   (tag sequence, not whitespace) matches the template-derived
   reference at the recorded floor.
4. **CSS+Images, CSS Sprites, To Files, states→GIF, CSS Properties**
   — CSS+Images (absolutely-positioned divs + background images per
   CssLayers.htt structure); CSS Sprites (one sheet + jQuery-facing
   CSS offsets, the `fw.exportCSSSpriteForJQuery` analog); Pages/
   States/Layers To Files batch naming; states → animated GIF through
   the M8 codec; CSS Properties extraction per object (geometry,
   fill → background, radius, shadow) as a text artifact.
   **Stopping condition:** each workflow's output is deterministic
   (double-run byte-equal) and pinned by golden fixtures.
5. **Shell + CLI surfaces** — Web Layer panel (slice list, link
   editing); slice tool in the tool group (drag → `slice.create`);
   export dialog gains the workflow picker with per-workflow options;
   app-cli `export-web` command; MCP/CLI dispatch through the same
   registry ids. **Stopping condition:** the shell's apply path
   drives slice-create → link → export end to end; one-command battery
   covers the new ids (GUI ≡ channel ≡ MCP byte-equal).
6. **Gates** — corpus fixture with slices pinned (`sliced.fw.png`
   through the engine registry); normalized structural-diff floor for
   exported HTML/CSS; parity floors (export workflows, slice corpus)
   + scorecard M11 row; decisions doc (web-layer model + export
   purity + auto-naming provenance); devlog. **Stopping condition:**
   `xtask ci` green via GitForge; M11 scorecard row → done with gate
   evidence.

## Non-goals (M11)

- Pop-up menu *rendering* (CS6's `mm_menu.js` generation) — `BETi` 9
  payloads stay preserved verbatim; typed menu export needs observed
  corpus evidence we don't have yet.
- jQuery Mobile theme export (CS6 addition, deprecated surface) —
  tracked behind M12 with the rest of the release matrix.
- Image optimization palettes beyond the M8 codec set (`.act` palette
  files) — `EXPv` palette settings stay recorded-not-applied.
- The `mkBT` writer + TileBank host bridge (M7-owed debt) remains
  tracked separately.
