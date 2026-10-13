# FireCraft — Roadmap

> Canonical milestone list. Strategy and rationale: [`../../PLANNING.md`](../../PLANNING.md).
> Tickets per milestone live beside this file (`M0-*.md`, `M1-*.md`, …).
> Later milestones get tickets when the previous milestone lands — details
> this far out would be fiction.

**Conventions.** Each milestone ends with a *gate* — a machine-checked
verification, not a vibe. Floors (`FLOOR` consts, round-trip percentages,
golden-image tolerances) may only rise. Every milestone appends a devlog
entry (`log/devlog.md`, gitignored) as the session-handoff record.

---

## M0 — Bootstrap & method
**Goal:** the repo becomes a working Rust workspace with the full method
installed, before any Fireworks code exists.

- Workspace scaffold: `crates/` `apps/` `xtask/` `web/` `docs/` `assets/`
  `packaging/`; edition 2024; workspace lints (`unsafe_code = "forbid"`,
  no-unwrap/panic policy); MIT OR Apache-2.0 dual license + `NOTICE` +
  trademark disclaimer in README.
- `xtask` with gates: `layers` (dependency-direction enforcement),
  `wasm` (workspace builds for `wasm32-unknown-unknown`; headless test
  subset via `wasm-bindgen-test`), `ci`, `panic_hunt` harness (empty until
  commands exist), `corpus` (fetch/check pinned+sha256), `parity` +
  `scorecard` generators (empty tables, `enforce = false` until M4).
- Root docs: `AGENTS.md` (operating manual — see PLANNING §11.1),
  `docs/ATTRIBUTION.md` (exists — extend), devlog + plan dirs gitignored.
- Placeholder crates with real manifest wiring but empty libs are NOT
  allowed (no placeholder code) — M0 creates exactly the crates M1/M2 need:
  `geom`, `color`, `raster`, `fwpng`.

**Gate:** `cargo xtask ci` green (fmt, clippy `-D warnings`, test, layers,
wasm) on the four L0 crates' skeletons-with-tests (each ships a real
smoke test, not a stub).

## M1 — L0 foundations + scripting-engine spike
**Goal:** geometry, color/blending, and tiled rasters solid enough to build
the document model on; the wasm scripting decision made.

- `geom`: affine transforms, béziers (kurbo), stroke outlines, hit testing
  primitives; property tests + `cargo-fuzz` anchors.
- `color`: pixel formats (8/16/32-bit, premultiplied), the 27 Fireworks
  blend modes as pure functions, sRGB/linear handling; golden values from
  spec + CS6-exported reference swatches.
- `raster`: copy-on-write 128×128 tile store (matches `mkBT` tile size),
  allocation caps on input-derived sizes.
- **Spike:** `rquickjs` vs `boa_engine` — must load a real shipped `.jsf`
  (local tree), expose a trivial host object, and build+run for
  `wasm32-unknown-unknown`. Decision recorded in `docs/decisions/`.

**Gate:** `cargo xtask ci` green incl. fuzz smoke targets; spike decision
doc merged; blend-mode table has one test per mode.

## M2 — `fwpng`: Fireworks PNG read
**Goal:** parse any Fireworks PNG in the wild without panicking; extract
preview, tiles, and the full node tree.

- PNG container with custom private-chunk layer (unknown chunks preserved
  verbatim); `prVW` thumbnail decode; `mkBT` tile decode (incl. split
  streams, grayscale variant); `mkTS`/`mkBS` node-tree parser → generic
  node tree (typed i/f/b/s/v values); `mkBF` passed through.
- `firecraft-corpus` bootstrap: public sample files + first Wine-generated
  batch (if harness available); sha256 manifest; `xtask corpus`.
- Differential harness: our parser vs png-analyser's output on corpus
  samples (node counts/kinds must agree at a recorded floor).
- Publishable standalone crate: zero workspace deps.

**Gate:** 100% of corpus parses panic-free (fuzz + corpus test); round-trip
serialize(parse(x)) is byte-identical on ≥ recorded `FLOOR`% of corpus;
`fwpng` builds on wasm; docs/decisions entry for any format surprises.

## M3 — `doc` model + format mapping + round-trip writer
**Goal:** typed document model fully mapped to the node tree, writing
losslessly.

- `doc`: pages/states/layers/objects, paths/text/primitives/groups/masks/
  guides/export-settings/symbols scaffolding; flat registry + derived
  indexes (Graphite pattern).
- `fwpng` typed layer: model ⇄ node-tree mapping; **compile-fail when a
  doc field is unmapped**; unknown-node preservation into a sidecar
  "unknown" bag re-emitted on write.
- Round-trip floors rise (corpus byte-identity), now including rewritten
  (not just echoed) trees where mapping exists.

**Gate:** corpus round-trip floor ≥ recorded target; `xtask layers` shows
`doc` at L1 depending only on L0; unmapped-field compile-fail demonstrated
in a test.

## M4 — CPU compositor
**Goal:** render any corpus document to pixels, deterministically, close to
CS6's own flattened export.

- `compose`: fill/stroke/gradient/pattern pipelines, blend modes from
  `color`, layer masks, Web Layer semantics, canvas states; filter
  sub-set needed by corpus docs (shadows/glows, blurs).
- Golden-image suite: compose(doc) vs CS6 export, pixel-diff floors via
  `image-compare`; `xtask scorecard` starts `enforce = true` on render.

**Gate:** golden-image floors met on corpus; deterministic (same input →
byte-identical output); benches recorded (`criterion`) as baseline.

## M5 — Editor shell (native + web)
**Goal:** a usable app: open → inspect → edit → save `.fw.png`, in the
browser and on desktop.

- `engine`: session, everything-is-a-command registry, delta undo with
  transactions; platform-services traits (fs/dialogs/clipboard) injected.
- `app`: egui/eframe shell — canvas (pan/zoom, HiDPI), tools (pointer,
  subselect, rect/ellipse/line, pen basics, zoom/hand), panels (layers,
  pages, states, properties), menus from command catalog.
- `web/`: trunk build of the same shell; browser file open/save; basic
  postMessage bridge (`openDocument`, `exportPng`).
- `app-cli`: `firecraft-cli info|convert` (headless fwpng utilities).

**Gate:** same operations scripted via CLI and UI produce identical
documents; `panic_hunt` fuzzes every registered command; `xtask wasm` ships
the web bundle; undo/redo survives session-restart journal replay.

## M6 — Vector depth
Path ops (union/intersect/punch/crop — bespoke boolean spike; simplify,
expand stroke, inset), freeform/reshape tools, Auto Shape tools + library
shapes ported from the `.jsf` behavior specs onto our scripting host,
9-slice editing, align/distribute. **Gate:** scripted shapes reproduce CS6
geometry on corpus cases; `.jsf`-authored shapes from the local tree render
correctly.

## M7 — Bitmap + full live filters
Selections (marquee/lasso/wand, feather), retouch tools, complete filter
catalog (levels/curves/hue-sat/noise/find-edges/bevel/emboss…), Photoshop
Live Effects parameter mapping from `EFDv` research, text-on-path, full
text engine attributes. **Gate:** per-filter golden images vs CS6 at
recorded tolerances; filter stack composes in any order with correct
document-model round-trip.

## M8 — Import/export pipeline
PSD (ag-psd spike first), SVG in/out (usvg/resvg), JPEG/WebP/AVIF/TIFF/
BMP/WBMP targets, animated GIF → states, optimization settings (palettes,
dithering, selective JPEG), batch processing. **Gate:** per-format
round-trip + oracle suites; PSD fidelity checklist vs documented CS6
behavior (§3).

## M9 — Scripting & automation
Full `fw`/`dom` host-object API, `.jsf` command loading (app/user/virtual
FS), History-panel replay ("Save Steps As Command" equivalent), batch
scripts, headless `run-script`, JSON control channel + MCP server.
**Gate:** the shipped CS6 `.jsf` corpus (local tree, as behavior tests)
runs and produces expected DOM effects; every command reachable via CLI,
UI, and MCP with the same ids.

## M10 — Symbols, libraries, components
Graphic/button/animation symbols, symbol editor, document library
import/export, rich/component symbols (`Widget.opCode` scripts), 9-slice
symbols, original Common Library assets (re-created; ATTRIBUTION rows).
**Gate:** symbol round-trip floors; rich symbols from CS6 docs render and
stay editable.

## M11 — Web export
Slices/hotspots/image maps, export workflows (HTML+Images, CSS+Images,
CSS Sprites, Pages/States/Layers To Files), behaviors (rollover/swap),
state animation → animated GIF, CSS Properties extraction. **Gate:**
exported HTML/CSS/images from corpus docs match CS6's structure at a
recorded floor (normalized diff).

## M12 — Parity hardening & release matrix
`xtask parity`/`scorecard` sweep with `enforce = true`, perf budgets on
cold start / pan-zoom latency / save time, memory ceiling tests,
packaging (Linux tar/AppImage/deb, macOS universal dmg, Windows installer +
portable, CLI archives), **web bundle release** + WebOS embedding guide
(iframe/postMessage bridge spec), i18n pass, security review of parsers
(fuzz corpus merge), 1.0 docs (book/).
**Gate:** full release matrix produced by `cargo xtask release` in CI;
zero known panic paths (fuzz long-run clean); scorecard parity floor met.

## M13 — Deep RE evidence & CS6 shell parity
The §5.5 binary-analysis passes the original request named and M0–M12
never ran: Ghidra (headless, driven through REA) over `Fireworks.exe` —
PNG-writer node semantics (the mkTS unknowns), chunk serializers, the
filter/effect pipeline, and the menu/command/UI resource tables. Findings
land as committed derived-spec notes; raw dumps stay uncommitted; corpus
stays the gate. The editor shell then reaches CS6 parity: menu structure,
vertical Tools panel with original icon art (ATTRIBUTION rows), Properties
inspector bottom-docked, Pages/States/Layers/Library/History/Optimize
stack, dark chrome, the CS6 accelerator map, and per-tool cursors, all
pinned by structure tests. **Gate:** every §5.5 target carries a
changed-or-confirmed verdict in a committed note; the shell implements
the extracted tables; `xtask ci` green with the wasm bundle ceiling
holding; scorecard M13 row done.

## Post-M13 fidelity series (user-directed scope: full-featured CS6 parity)

The standing scope is the *whole application* at parity with Fireworks
CS6 — reverse-engineered behavior, open-source implementation, then
enhancements over time (HEVC/AVIF and beyond). M0–M12 built functional
coverage; M13 stands up the RE evidence layer and the shell; the series
below takes each subsystem from "works" to "matches the original" with
the same gate discipline. Details stay unwritten until predecessors land.

## M14 — Live-filter parity
Every CS6 live-filter family at parameter-exact behavior, grounded in the
M13 filter-pipeline pass and Wine-oracle goldens (`.jsf` batch → golden
renders): full Adjust Color set, complete blur family, shadow/glow/bevel
families, stylize, sharpen, noise. **Gate:** unknown-effect GUID
passthrough floor falls to zero; per-filter goldens green.

## M15 — Tool-surface parity
Complete tool semantics per the extracted tables: full marquee/lasso/wand
selection model (modifier arithmetic, feather, edges), brush/eraser/
retouch engine depth, path redraw/freeform/reshape/scrubber, gradients,
style model. **Gate:** interaction suite + oracle diffs green.

## M16 — Panel & document parity
Complete panel set (Optimize, Color Mixer, Swatches, Styles, Info,
Behaviors, Align, Auto Shape/Path panels, onion skinning) and document
surfaces (shared layers, nested pages, state animation model).
**Gate:** structure tests pin every panel against the extracted tables.

## M17 — Automation parity
89/89 shipped `.jsf` scripts headless (floor rises from 19), the full
`fw`/`dom` API surface from the RE'd inventory, batch processing and the
Project Log. **Gate:** script-corpus floor at 89, byte-exact DOM effects.

Enhancements (HEVC, additional formats, performance) open after the
parity series closes — enhancements on top of parity, never instead of.
