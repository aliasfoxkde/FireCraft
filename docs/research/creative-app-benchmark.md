# Creative-app benchmark: Krita, Photopea, and Pixlr

> **Purpose:** identify product patterns worth independently implementing in FireCraft.
> **Status:** research and product guidance, not a claim that any benchmarked capability exists in FireCraft.
> **Reviewed:** 2026-11-23.
> **Related:** [Post-MVP Product and Engineering Roadmap](../POST-MVP-ROADMAP.md), [Marketing Site Roadmap](../MARKETING-SITE-ROADMAP.md).

## 1. Executive assessment

These products are useful references because they excel in different parts of the creative workflow:

- **Krita** is the strongest reference for professional painting, brush dynamics, stylus/tablet input, resource management, and color-managed artistic work.
- **Photopea** is the strongest reference for browser-native layered image editing, broad file interoperability, non-destructive object/filter concepts, and scriptable host integration.
- **Pixlr** is the strongest reference for approachable quick actions, guided image cleanup, low-friction editing, and optional AI-assisted transformations.

FireCraft should not become a clone of any one product. Its opportunity is to combine the parts that fit its existing strengths: vector and bitmap content in one editable document, a compact Fireworks-style workflow, reliable export, a local-first Rust engine, browser/native surfaces, and automation.

This benchmark is a source of product requirements and interaction ideas. It is not permission to reuse another product's code, artwork, icons, copy, proprietary assets, or protected implementation details.

## 2. Product comparison

| Product | Strongest reference areas | FireCraft opportunity | Boundary |
|---|---|---|---|
| Krita | Brush engines, pressure/tilt sensors, tablet diagnostics, popup palette, canvas-only mode, symmetry, assistants, resource tags, soft proofing | Professional brush and stylus workflows; a resource manager; touch-aware workspace; fast contextual controls; color-managed preview | Krita is GPL-3.0. Do not transplant application code or assets into the Apache-2.0 FireCraft project without an explicit licensing decision. |
| Photopea | Browser editing, PSD/PSB and other formats, layers/masks/channels, Smart Objects, Smart Filters, scripting, embedding and messaging API | Stronger non-destructive asset workflows; format-fidelity tests; a scriptable editor host contract; browser-specific UX | Photopea is proprietary, not open source. Its public API does not make its implementation open source or grant rights to copy it. Avoid making it a required runtime dependency. |
| Pixlr | Guided quick edits, masks/selections, background/object removal, generative fill/expand, upscaling, mobile editing | Discoverable quick actions and previewable operations; a fast path for common image tasks; optional AI adapters later | Pixlr is a proprietary service/product. Study public functionality and independently implement workflows; do not copy assets, code, UI artwork, text, or branding. |

## 3. Krita patterns to evaluate

### 3.1 Brush engine and preset library

Krita demonstrates that a brush system is more than size, hardness, and color. Its documented engine families include pixel, bristle, chalk, clone, color-smudge, deform, filter, grid, hatching, MyPaint, particle, shape, sketch, spray, and tangent-normal brushes.

FireCraft should evaluate a staged brush model:

1. **Reliable baseline:** pressure-aware size and opacity, spacing, hardness, angle, roundness/aspect, scatter, rotation, and stroke smoothing.
2. **Preset workflow:** searchable categories, tags, favorites, previews generated from representative strokes, import/export, user libraries, and missing-resource diagnostics.
3. **Advanced dynamics:** tilt, velocity, direction, rotation, texture, dual brushes, bristle behavior, and specialized engines only where the renderer can support them predictably.
4. **Reproducibility:** pin brush parameters and relevant engine versions in the document/resource metadata so reopening a document does not silently change its appearance.

Do not promise parity with Krita's brush-engine catalogue. Add engines only when they have deterministic tests, useful UX, and acceptable CPU/GPU behavior.

### 3.2 Stylus, tablet, and touch input

Treat pen input as a first-class input domain rather than an incidental mouse event. Evaluate:

- Pressure, tilt, azimuth, rotation, barrel buttons, eraser end, and device identity where the platform exposes them.
- User-adjustable pressure curves and minimum/maximum pressure thresholds.
- A tablet diagnostic view that reports the events actually received by the application.
- Input smoothing/latency controls with clear trade-offs.
- Stylus detection and separate touch gestures for viewport navigation, zoom, rotate, and canvas interaction.
- A tablet workspace with larger targets, collapsible docks, canvas-first controls, and a configurable popup palette.
- Per-device profiles that do not leak into other devices or documents.

Test native desktop and browser/WASM input separately. Do not infer stylus support from a successful mouse test or from a browser API being available. Record real devices, OS/browser versions, event fields, latency observations, and failure modes. Phone-sized full-editor support is not a prerequisite for a strong tablet experience.

### 3.3 Popup palette and canvas-only workflow

Krita's popup palette is a useful interaction reference: frequently used colors, brush presets, color history, canvas rotation, zoom, mirroring, and workspace actions are available close to the canvas.

For FireCraft, prototype a configurable radial or compact contextual palette, but preserve conventional menus and keyboard access. The palette must be operable by mouse, pen, touch, and keyboard; it must not hide essential commands behind a gesture that is unavailable to some users.

A canvas-only mode should hide configurable chrome while preserving a visible way to exit, restore panels, and reach essential status information. Respect reduced-motion preferences and persist the workspace choice.

### 3.4 Painting aids and color management

Evaluate symmetry/mirror tools, perspective and drawing assistants, rulers/guides, tiled wrap-around painting, canvas rotation, soft proofing, out-of-gamut warnings, rendering intents, and black-point compensation.

Color management requires a defined pipeline and test profiles, not just a profile selector in the UI. Keep display preview, source profile, conversion, and export behavior distinct. Use the existing color-management workstream and architecture decision process before selecting dependencies or changing document serialization.

## 4. Photopea patterns to evaluate

### 4.1 Non-destructive source assets

Photopea's Smart Object concept is a useful specification for an embedded or linked source asset that can be transformed without repeatedly degrading its source pixels. Multiple instances can share source content while carrying independent transforms; editing the source updates its instances. Smart Filters provide an editable filter stack rather than baking each effect into pixels.

For FireCraft, evaluate a distinct **Smart Asset** model:

- Embedded source content for reliable portability.
- Optional linked source content with explicit missing/relink state.
- Independent transforms for each placed instance.
- A visible source-edit action and clear indication of shared instances.
- Non-destructive filter stacks with reorder, enable/disable, parameter editing, masks, and preview.
- Explicit warnings when export or format conversion must rasterize unsupported semantics.

Do not conflate Smart Assets with design Components. Smart Assets represent source content and nondestructive placement; Components represent reusable design structures and behavior. See the existing asset/resource and component workstreams before changing the document model.

### 4.2 File interoperability as a tested contract

Photopea's broad format list is a useful benchmark, not a target checklist to copy blindly. Format support should be ranked by user value and tested using representative fixtures.

For every format, distinguish:

- Can open and render.
- Can edit a documented subset.
- Can preserve layers, masks, text, paths, profiles, and effects.
- Can save back to the same format.
- Can round-trip without unacceptable loss.
- Must flatten or substitute unsupported content.
- Must reject unsafe or unsupported features.

A file extension appearing in a file picker is not proof of useful compatibility. Keep the format capability registry, corpus fixtures, fidelity tests, and warning UX authoritative.

### 4.3 Script and host integration

Photopea's scripting and messaging model is a reference for an editor that can be embedded or driven by a host. FireCraft already has a shared command registry, CLI, scripting, browser bridge, and MCP surfaces. Extend those existing contracts instead of inventing a separate web-only editing API.

Any host API should be versioned, permission-scoped, documented, testable, and unable to access arbitrary local files or privileged secrets without explicit user authorization.

## 5. Pixlr patterns to evaluate

### 5.1 Quick actions and guided editing

Pixlr is a useful interaction benchmark for making frequent tasks discoverable without requiring users to understand every panel. FireCraft should evaluate a Quick Actions surface for operations such as:

- Remove background or isolate a subject, when a dependable implementation exists.
- Crop, resize, rotate, straighten, and convert.
- Adjust brightness, contrast, levels, curves, hue, saturation, and color balance.
- Remove blemishes, heal, clone, blur, sharpen, and denoise.
- Expand canvas and fit artwork to common target dimensions.
- Export selected layer, selection, page, or asset with a known preset.

Each action must be a real workflow over the existing engine, not an isolated duplicate implementation. Show the expected result before applying destructive changes; support undo and clear cancellation. The user should be able to discover the underlying detailed controls when desired.

### 5.2 AI-assisted operations

Background segmentation, generative fill/expand, super-resolution, and layer extraction may be valuable, but they should be later, optional capabilities. Separate deterministic local editing from provider-backed AI. Require:

- Explicit user initiation and clear disclosure of local versus remote execution.
- Preview and approval before document mutation.
- No upload of document pixels without informed consent.
- A documented provider/model and cost boundary where applicable.
- Failure handling that preserves the original document.
- An offline or non-AI alternative for core editing tasks.

Do not make AI services necessary to open, edit, save, export, or use FireCraft.

## 6. Recommended FireCraft priority

| Priority | Work | Rationale |
|---|---|---|
| P1 | Pressure/tilt input audit; tablet diagnostics; brush presets and resource browser; popup palette; canvas-only and tablet workspace | High-value professional workflow improvements that reinforce FireCraft's creative-editor identity. Audit existing local work before creating tasks. |
| P1 | Non-destructive adjustment/filter UX and Smart Asset architecture spike | Improves editability and reduces destructive raster workflows; depends on a safe document-model decision. |
| P1 | Format capability/fidelity reporting and round-trip corpus improvements | Trustworthy interoperability is more important than a long unsupported-format list. |
| P2 | Advanced brush engines, assistants, symmetry, wrap-around painting, advanced selections, and richer masking | Valuable after baseline input, rendering, and resource infrastructure are stable. |
| P2 | Quick Actions and guided workflows over existing commands | Improves discoverability without creating a second editing engine. |
| P3 | Optional local/provider-backed AI segmentation, fill, expansion, upscaling, and layer extraction | Potentially useful, but requires separate privacy, performance, licensing, and provider decisions. |
| Research | Android/tablet/mobile packaging and adaptive shell | Use actual device/browser test evidence; do not equate upstream app availability with FireCraft platform readiness. |

This table is a prioritization proposal only. The baseline audit determines whether any item is already implemented or in progress.

## 7. Clean-room and licensing constraints

- FireCraft's source is Apache-2.0. Krita's application is GPL-3.0; copying or adapting its GPL-covered code into FireCraft can create incompatible distribution obligations. Do not do so without an explicit project licensing decision and legal review.
- Do not assume every individual Krita file, brush pack, preset, or bundled resource has the same license. Verify the exact license and redistribution terms for every third-party asset or dependency.
- Photopea is proprietary. Its public scripting or embedding API is not source-code permission and does not grant rights to reproduce its internals, brand assets, or UI artwork.
- Pixlr is proprietary. Do not reverse engineer, scrape, copy, or redistribute its software, assets, text, branding, or protected implementation.
- Use public product behavior and documentation to identify user problems and independently design FireCraft's own solution. Record provenance and licenses for every third-party dependency or shipped resource in the existing attribution ledger.
- Do not decompile any Adobe software or import Adobe code/assets. The repository's AGENTS.md clean-room policy remains authoritative.

## 8. Primary references

### Krita

- [Krita source repository and license](https://github.com/KDE/krita)
- [Krita Brush Engines manual](https://docs.krita.org/en/reference_manual/brushes/brush_engines.html)
- [Krita Tablet Settings](https://docs.krita.org/en/reference_manual/preferences/tablet_settings.html)
- [Krita Popup Palette](https://docs.krita.org/en/reference_manual/popup_palette.html)
- [Krita Canvas-Only Mode](https://docs.krita.org/en/reference_manual/canvas_only_mode.html)
- [Krita Soft Proofing](https://docs.krita.org/en/reference_manual/preferences/color_management_settings.html)
- [Krita Android FAQ](https://krita.org/en/about/faq/)

### Photopea

- [Photopea introduction](https://www.photopea.com/learn/)
- [Supported file formats](https://www.photopea.com/learn/opening-saving)
- [Smart Objects](https://www.photopea.com/learn/smart-objects)
- [Smart Filters](https://www.photopea.com/learn/smart-filters)
- [Scripting](https://www.photopea.com/learn/scripts)
- [Photopea API](https://www.photopea.com/api/)
- [Photopea privacy policy](https://www.photopea.com/privacy.html)

### Pixlr

- [Pixlr editor](https://pixlr.com/editor/)
- [Pixlr features](https://pixlr.com/)
- [Pixlr terms](https://pixlr.com/terms-of-use/)
- [Pixlr mobile apps](https://pixlr.com/mobile/)

## 9. How to use this benchmark

1. Read this document alongside AGENTS.md and the current roadmap.
2. Map each proposed capability to existing source, tests, local branches, PRs, and the live task ledger.
3. Mark existing work accurately; do not create duplicate tasks.
4. For any new architecture or document-format semantics, write an ADR/spike before broad implementation.
5. Implement a small end-to-end workflow with evidence before adding more engine variants or format labels.
6. Update this benchmark when a source behavior, license, or strategic recommendation changes.
