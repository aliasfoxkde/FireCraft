# Graphite product benchmark

**Reviewed:** 2026-11-26. **Purpose:** extract transferable product and engineering insights for FireCraft, while distinguishing Graphite's current alpha capabilities from its published roadmap.

Graphite is a Rust-based, open-source 2D graphics editor and engine. Its distinctive idea is a layer-and-tool editor backed by a procedural node graph: users can stay in the conventional layer view, work directly with nodes, or switch between them. The tools manipulate the graph behind the scenes, and the layer panel and graph are alternate views of one document structure. [Graphite features](https://graphite.art/features/), [repository](https://github.com/GraphiteEditor/Graphite).

## Current state, not roadmap promises

Graphite's own current [features and limitations page](https://graphite.art/learn/introduction/features-and-limitations/) identifies the application as alpha and says vector editing is its current core competency. It documents drawing/editing vector shapes and procedural operations, including node-based repetition. Raster support is growing: import images, manipulate them in the compositor, and apply non-destructive global color adjustments. The live feature page calls raster rendering at arbitrary scales experimental. Its roadmap lists other work separately, including richer brush tooling, asset libraries, batch processing, dockable panels, mobile/tablet features, and import formats. Those are plans, not evidence of current production behavior.

The Graphite repository's node-graph authoring guide describes nodes as editable operations connected into a graph. Node definitions declare categories, inputs, outputs, defaults and properties. Inputs can stay hidden from the graph until exposed, which reduces clutter while retaining precise control in a properties view. [Node graph authoring guide](https://github.com/GraphiteEditor/Graphite/blob/master/node-graph/README.md).

## Transferable product insights

### 1. Keep the familiar editor and make procedural depth optional

Graphite's layer-first tools and optional graph view show a useful gradual-disclosure pattern. FireCraft's CS6-inspired layers, properties and commands should remain the primary workflow. A node view can be an advanced alternate view for selected operations, so an artist can inspect and change effect order, parameter values and dependencies without forcing every user to learn a graph.

**FireCraft fit:** explore one bounded effects workflow first, such as an existing color adjustment or filter stack. Every graph edit must map to the same undoable engine commands and document state as the existing inspector. Provide a clear graph/property representation, stable serialization, migration behavior and sensible fallback for unsupported nodes. Do not switch the whole FireCraft document model to Graphite's model.

### 2. Model operations as inspectable, editable state

Graph nodes make operation order and dependencies explicit. This could improve FireCraft's filter and style workflows: reorderable operations, named inputs, defaults, enabled state, preview, masks and a visible result path. It also gives debugging and scripting surfaces a concrete description of how a rendered result was produced.

**FireCraft fit:** continue building the existing non-destructive masks, effects, styles and command registry first. An operation graph is justified only where the document model needs branching/reuse or where a list stops being understandable. A linear editable stack may deliver most user value with less UI and serialization complexity.

### 3. Use progressive input exposure to keep panels legible

Graphite's node definitions can expose common controls directly while keeping advanced inputs hidden until requested. This is a good general inspector pattern: show the few controls needed for the current task and offer explicit advanced controls, rather than filling every panel with seldom-used fields.

**FireCraft fit:** use this in Properties, Filters and future Intelligence panels. Preserve keyboard discoverability, screen-reader labels and a stable layout. Hiding a control must not remove its accessibility or command path.

### 4. Treat tiny interaction failures as product work

Graphite's public issue board tracks specific paper cuts alongside feature work: pasted nodes should land at the pointer, panning/zooming during drags should remain robust, aborted selections should restore prior state, and dialogs should respond to keyboard confirmation. This makes repro steps and observable behavior actionable.

**FireCraft fit:** the current focus, clipboard, tab-toggle, drag/drop and dialog concerns deserve isolated reproduction tasks with input sequences, expected state, regressions and platform-specific receipts. Do not hide such bugs under a broad “polish” task.

### 5. Keep rendering architecture behind capability checks

Graphite's project history and repository distinguish renderer and node-engine work. FireCraft can study its GPU and procedural-rendering decisions, but performance improvements must be measured on the supported native and browser backends. WASM/WebGPU and native graphics capability differences need explicit fallback paths.

**FireCraft fit:** continue using the shared rendering/engine boundary. Benchmark CPU/GPU paths with representative vector, raster and effect documents; test browser support and recovery from renderer failure before adopting a new backend or moving more work into shaders.

### 6. Organize contributions around real seams

The public Graphite repository separates editor/frontend, document, graph schema/execution, node families, shared graphics types, rendering and desktop hosting into visible areas. Its contribution labels distinguish architecture, graph-engine, graphics, performance, testing, infrastructure, desktop/web and paper-cut work. That makes ownership and review scope easier to understand in a large graphics project.

**FireCraft fit:** keep its enforced engine layers and workspace boundaries authoritative. Improve contribution tickets with exact affected crate/surface, behavior owner, fixture or screenshot evidence, platform matrix and merge dependency. Do not mirror Graphite's source tree mechanically; FireCraft's engine, FPNG preservation model and automation layers have different responsibilities.

## Suggested FireCraft work, ordered by risk

| Priority | Candidate | Acceptance evidence |
|---|---|---|
| P1 | Finish familiar non-destructive filter/mask/property flows and robust layer operations | Undo/redo, serialization round-trip, disabled/missing-feature handling, screenshot and keyboard flow receipts |
| P1 | Fix editor paper cuts as narrow tasks: clipboard, focus restoration, drag cancellation, dialog confirmation, import placement | Repro test per issue; native and browser behavior recorded separately |
| P2 | Prototype an optional graph view for one existing operation family | Same result as the layer/property workflow; graph mutations use existing command/history path; old documents load; unsupported node data is preserved or diagnosed |
| P2 | Adopt progressive control exposure and explanatory tooltips in dense inspectors | Keyboard/screen-reader access, no hidden essential action, tests for property defaults and edits |
| P2 | Benchmark renderer alternatives/fallbacks against real FireCraft corpus files | Same-commit timing, memory and visual comparisons across native and WebAssembly |
| P3 | Consider asset/node libraries, procedural brushes, batch processing or mobile input | Demonstrated user need, versioned package/format plan, licensing review, support matrix and tested platform receipts |

## Non-goals and guardrails

- Do not advertise Graphite's alpha roadmap as current functionality or adopt its whole product scope as FireCraft commitments.
- Do not make a node graph mandatory for routine vector, raster, text, layer, page or export work.
- Do not duplicate document semantics in a frontend-only graph that cannot be undone, scripted, round-tripped or used by CLI/MCP.
- Do not copy Graphite code or assets into FireCraft. Its repository offers MIT or Apache-2.0 licensing for source unless a subdirectory says otherwise; product research should remain behavior/documentation-led, and any future dependency/code reuse needs provenance and license review.
- Keep FireCraft's own document format, layer model, command registry, clean-room rules and WASM/native portability authoritative.

## Sources

- [Graphite repository](https://github.com/GraphiteEditor/Graphite) — project description, Rust implementation and license.
- [Graphite features](https://graphite.art/features/) — hybrid layer/node approach and roadmap.
- [Graphite features and limitations](https://graphite.art/learn/introduction/features-and-limitations/) — current alpha capabilities and present limits.
- [Graphite node-graph authoring guide](https://github.com/GraphiteEditor/Graphite/blob/master/node-graph/README.md) — node categories, data inputs and property exposure.
- [Graphite open contribution issues](https://github.com/GraphiteEditor/Graphite/contribute) — current examples of granular UX and robustness issues.
- [Krita layers and masks](https://docs.krita.org/en/user_manual/layers_and_masks.html) and [brush engines](https://docs.krita.org/en/reference_manual/brushes/brush_engines.html) — mature non-destructive layer and brush references.
- [Photopea Smart Objects](https://www.photopea.com/learn/smart-objects), [filters](https://www.photopea.com/learn/adjustments-filters) and [scripts](https://www.photopea.com/learn/scripts) — linked source assets, editable effects and host automation.
- [Pixlr Editor](https://pixlr.com/editor/) and [Pixlr tools](https://pixlr.com/tools/) — quick actions, selection/mask editing, batch tools and optional AI services.
