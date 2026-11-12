# Editor and Layout

FireCraft uses one editing model across its native desktop and WebAssembly shells. The user-visible controls are being aligned with Fireworks CS6 while retaining FireCraft's modern document engine and command registry.

## Canvas and tools

The canvas supports document navigation, object selection, vector editing, bitmap tools, path and gradient manipulation, transforms, guides, grid, snapping, zoom and selection-aware context menus. The CS6-oriented tool strip and flyout model are implemented. See [Usage](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/USAGE.md) for shortcuts and [Features](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/FEATURES.md) for the current feature surface.

## Panels

The shell has a persistent, resizable right-side panel dock with a scrolling stack of collapsible panels. The panel inventory includes Properties, Layers, Pages, States, Swatches, Color Mixer, Info, History, Optimize, Symbols, Styles, Common Library, CSS Properties, Auto Shapes, URL, Behaviors, Find and Replace, Slices, and Web Layer. Panels can be toggled from the Window menu; availability and fresh-document defaults are defined in the app.

The panel inventory and tool strip are implemented, but that does not mean every inspector field is complete. Bitmap transform and fill/stroke behavior remain partial/evidence-gated, and the CS6 chrome/Main toolbar audit still has follow-up work. Optimize/timeline changes are in open [PR #118](https://github.com/aliasfoxkde/FireCraft/pull/118); the mask-stack extension is in [PR #113](https://github.com/aliasfoxkde/FireCraft/pull/113).

See the [CS6 parity matrix](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/CS6-PARITY.md), [task board](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/TASKS.md), and [Phase 22](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/NEXT.md#phase-22--cs6-shell-fidelity-p0-the-looks-and-acts-like-fireworks-phase) for exact status and evidence.

## Visual references

Repository screenshots are dated build evidence, not guaranteed captures of the latest main. The [T22 screenshot audit](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/tasks/T22-screenshot-ui.md) records tested viewports, states and limitations. Clean-room reference notes are in [fw-ui-evidence.md](https://github.com/aliasfoxkde/FireCraft/blob/main/docs/research/fw-ui-evidence.md).
