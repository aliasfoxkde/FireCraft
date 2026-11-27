# Emberframe product benchmark

**Reviewed:** 2026-11-25. **Reference:** [gregstein/emberframe](https://github.com/gregstein/emberframe). Repository README identifies Emberframe as a Rust/GPUI Fireworks look-alike, version 0.1, with Windows as the available native platform at the time reviewed. The repository reports four commits and an MIT license; confirm current state before relying on those details.

## What it demonstrates

- Its README enumerates tools, panels, vector and bitmap actions, states/animation, export comparisons, document commands and limitations in a compact feature table.
- A checked-in `examples/solstice-landing.ember` design is presented as the way to explore the UI. A real sample document plus the screenshot makes the feature claims tangible and gives first-time users something to inspect.
- Export preview has Original/Preview, 2-Up and 4-Up modes, which foregrounds Fireworks' production-oriented optimization workflow.
- The classic docked workspace and tool groups are central to the product identity; the README states that the icons are newly drawn and the implementation contains no Adobe code or artwork.
- The project calls out missing or partial behavior instead of implying full parity: Bezier curves, masks, live effects, blend modes, editable Fireworks PNG layers, and non-Windows builds are named in the README.

## FireCraft comparison and opportunities

| Emberframe pattern/state | FireCraft position | Suggested follow-up |
|---|---|---|
| Sample design file is the guided first-run artifact | FireCraft has sample/mockup imagery; a maintained editable document tutorial should be verified | Create a clean-room sample project with labeled pages/states/layers and guided exercises; verify it opens and round-trips in native and WASM builds |
| Export comparison modes are easy to see in the product story | FireCraft has an Optimize panel and export preview, but the public feature inventory should call out exact supported compare modes | Capture a current screenshot and test Original/Preview/2-Up/4-Up behavior where supported; document any missing mode |
| Short tool/panel capability inventory | FireCraft has deeper inventories and CS6 parity evidence, but status is spread across several docs | Link the user-facing feature list, parity matrix, tutorials and issue templates from Help and README |
| Explicit “not in this version” list | FireCraft distinguishes implemented, partial and evidence-gated items, but some docs are stale | Adopt a release-scoped, dated limitations list sourced from the live task ledger and current release |
| Windows review build is a first-class download path | FireCraft has release recipes but Windows artifact certification is a workstream | Build a review binary with commit/hash, then collect user launch and editing feedback before release claims |

## “Not in this version” inventory mapped to FireCraft

The Emberframe README lists these limits for its 0.1 build. They are prompts for FireCraft's audit, not assumptions that FireCraft has the same gaps.

| Emberframe limitation | FireCraft assessment from current source/docs | Plan decision |
|---|---|---|
| Free rotation, skew and distort limited; rotate is 90° and skips text/symbols | FireCraft documents a Numeric Transform dialog and scale/rotate/skew canvas handles; verify type coverage and transform parity from `CS6-PARITY.md` | Verify during Gate B; do not reimplement blindly |
| Bezier curves absent; Pen uses straight segments | FireCraft has path-node anchors and Bezier handles | Existing capability; verify with tutorial and regression evidence |
| Masks, live effects in Styles and moving selected pixels in place absent | FireCraft has a mask stack and live effects; style/effect interaction and pixel-selection move behavior need explicit parity evidence | Keep current implementation; add targeted verification only for unproven cases |
| Auto Shapes/library items click-insert only, no drag-to-canvas | FireCraft has file drag/drop and auto-shape panels, but that does not prove library-item drag/drop | Add a scoped interaction check; implement only if a current UX gap is confirmed |
| Printing and layer blend modes absent | FireCraft's menu census still has print/document gaps; layers already expose opacity/blend behavior in the feature inventory | Track printing as a distinct deferred product gap; verify layer blend modes in current UI |
| JPEG Selective quality and smoothing absent/disabled | FireCraft supports JPEG quality/matte export; inspect menu/preview fields to establish whether selective JPEG semantics are present | Record a CS6 parity row and feature decision after UI comparison |
| Fireworks PNG opens flattened (no editable layers) | FireCraft's differentiator is native editable `.fw.png` node-tree read/write with unknown-node preservation | Keep as a core differentiator; add representative fixture evidence to the public feature list |
| No macOS/Linux builds | FireCraft's support matrix is evidence-gated and currently has platform certification work | Do not infer support from a recipe; continue per-platform build/launch certification |

## Limitations and clean-room boundary

The README says Emberframe is version 0.1, has only four commits in the repository history at review time, and lacks Linux/macOS builds. This is a useful UX and product-story reference, not evidence of mature compatibility or a substitute for FireCraft's behavior audit. Treat the implementation and icons as reference-only. The public MIT license is noted, but the FireCraft project remains clean-room: transfer product ideas and independently create all code and artwork.

## Suggested tasks

1. Include sample document creation and open/save/round-trip checks in T29's GUI acceptance.
2. Add an Export Preview tutorial explaining optimize settings and compare modes.
3. Use the mapped “Not in this version” table to create only parity tasks that FireCraft does not already satisfy.
4. Review our app and marketing screenshots against the real running shell, with date, build revision, viewport and state in each receipt.
5. Keep a concise feature/limitations summary per release, generated or reconciled from the authoritative feature and task sources.
