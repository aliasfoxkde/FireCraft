# FireCraft — Post-MVP Product and Engineering Roadmap

> **Document type:** strategic product plan and post-MVP feature specification.
> **Status:** proposed direction; not an implementation-status claim.
> **Created:** 2026-11-23.
> **Intended audience:** project owner, maintainers, implementation agents, contributors, and integration partners.
> **Canonical repository:** https://github.com/aliasfoxkde/FireCraft
> **Execution authority:** this document does not replace [TASKS.md](TASKS.md), [NEXT.md](NEXT.md), [CS6-PARITY.md](CS6-PARITY.md), or the current release decisions. It becomes actionable only after the current local development work has been reconciled and the owner approves the resulting task breakdown.
> **Companion:** [POST-MVP-AGENT-HANDOFF.md](POST-MVP-AGENT-HANDOFF.md)

---

## 1. Executive summary

FireCraft should evolve from a Fireworks-inspired vector/bitmap editor into an **open, cross-platform creative production studio**: one environment in which people can create and edit graphics, assemble multi-screen designs, define reusable systems, prototype interactions, produce optimized assets, and deliver reliable handoff artifacts to developers and other applications.

The objective is not to recreate Photoshop, Illustrator, or Figma independently. It is to deliver the workflows that naturally cross those products' boundaries while preserving the speed and directness that made Fireworks useful.

The intended product proposition is:

**Create. Edit. Design. Prototype. Automate. Deliver.**

FireCraft should retain the original Fireworks strengths:
- Direct manipulation of vector and raster content in one document.
- Pages, states, layers, symbols, slices, hotspots, and export-oriented workflows.
- A compact, keyboard-driven creative environment.
- Fast iteration between editing and production output.
- A document format that preserves editable work rather than only a flattened preview.

It should extend those strengths with:
- A robust typography, brush, style, resource, and color-management system.
- A modern document model for pages, artboards, components, variants, and design tokens.
- Responsive design, interactive prototyping, and motion workflows.
- High-fidelity import/export and an explicit interoperability contract.
- Professional asset generation, optimization, and batch processing.
- Developer handoff, CLI, automation, MCP, plugins, and documented APIs.
- An accessible in-app Help and community experience.
- Optional AI assistance and collaboration built on safe, reversible operations.
- A compelling, evidence-led public product website, independently deployed through Cloudflare Pages from the same repository.
- Strong data integrity, performance, portability, and long-term document preservation.

This roadmap is deliberately broader than the next release. It is a product direction and feature-completeness specification. It must be converted into small, verifiable tasks only after the current MVP work is merged or otherwise reconciled.

## 2. Repository baseline and scope boundaries

### 2.1 Remote baseline observed for this plan

This plan was prepared against the public repository's main branch at:

- Commit: **6c68c9e36983b2aa3b38439502188a76cde52e9e**
- Workspace version documented by the README: **0.2.5**
- README project status: active development; not a claim of complete Fireworks CS6 workflow parity.
- Existing roadmap authorities: [TASKS.md](TASKS.md), [NEXT.md](NEXT.md), [CS6-PARITY.md](CS6-PARITY.md), [FEATURES.md](FEATURES.md), [PROGRESS.md](PROGRESS.md), [PLANNING.md](PLANNING.md), and [AGENTS.md](../AGENTS.md).

The repository baseline is a remote snapshot, **not the developer's current local working tree**. The project owner has stated that substantial UI/UX and feature work remains local and is still being implemented. This plan intentionally does not classify that local work as missing, complete, or obsolete.

Before execution, the agent must inspect the actual working tree, current branch, uncommitted changes, local-only branches, open PRs, task ledger, and recent session handoff. Preserve local work. Do not reset, clean, stash, overwrite, or replace it merely to match this remote snapshot.

### 2.2 Existing foundation to preserve and extend

The remote README and existing documentation describe a substantial foundation, including:

- Rust workspace with explicit crate layering and a shared document/engine architecture.
- Native Fireworks PNG document read/write, including preservation of unknown document nodes/chunks.
- Vector path and primitive editing, bitmap editing and retouching, text, symbols, masks, live effects, pages, states, layers, slices, and hotspots.
- Raster and vector import/export workflows, including PNG, JPEG, GIF, BMP, TIFF, WebP, WBMP, SVG, and PSD-related workflows as documented by the current feature inventory.
- A central command registry used by the editor, CLI, scripting, browser bridge, journal/replay, and MCP server.
- An ExtendScript-compatible scripting host and headless processing paths.
- Native desktop and WebAssembly/browser targets.
- A command palette, keyboard shortcuts/help surface, preferences, autosave/recovery, accessibility work, animation timeline work, and ongoing CS6 shell-fidelity effort.

These are existing project capabilities or documented work surfaces, not a warranty that every operation is equally mature in every frontend or every format. Use current source, tests, generated evidence, and the current parity matrix to determine actual status.

### 2.3 Explicit non-goals

Unless the owner approves a separate decision, this roadmap does not require FireCraft to:

- Reimplement every feature of Photoshop, Illustrator, InDesign, Figma, or a full desktop-publishing suite.
- Become a full web page builder or production application IDE.
- Promise perfect import/export fidelity for proprietary formats whose specifications or semantics are unavailable.
- Convert arbitrary designs into production-ready framework code without human review.
- Require user accounts, cloud storage, telemetry, hosted AI, or collaboration for ordinary local editing.
- Introduce a large server backend merely to support community issue reporting.
- Replace the existing native document format without a demonstrated technical need and a migration strategy.
- Break the WASM target or weaken current clean-room, security, dependency, lint, fuzz, or quality policies to accelerate feature delivery.

## 3. Product principles and design constraints

Every proposal and implementation should satisfy these principles.

### 3.1 One editable source of truth

A document should remain an editable project, not become a pile of flattened exports. Vector objects, raster layers, text, effects, masks, components, layout information, animation, prototype interactions, and export configuration should remain independently inspectable and editable where the format supports them.

### 3.2 Preserve the Fireworks workflow

Modern capabilities should extend rather than obscure the original workflow. The application should remain fast to open, easy to understand, keyboard-friendly, and efficient for single-image editing. Advanced design and prototyping functionality should be available through focused workspaces and panels rather than forced into every user's default workspace.

### 3.3 Shared engine, multiple frontends

The UI, CLI, scripting runtime, MCP server, plugins, and browser host should use one stable command/automation authority. Do not reimplement document semantics independently in each surface.

### 3.4 Local-first and offline-capable

Opening, editing, saving, rendering, and exporting local documents must not require sign-in or network access. Cloud collaboration, community, remote AI, and synchronization are optional services with explicit permissions.

### 3.5 Reversible operations

Potentially destructive edits, conversions, color transformations, relinking, bulk operations, AI actions, and agent-driven changes must support preview, undo, recovery, or a clearly documented irreversible boundary.

### 3.6 Honest compatibility

Opening a format is not equivalent to preserving it. For every format, document which features remain editable, which are transformed, which are flattened, and which are rejected. Surface losses before the user commits to an export or overwrite.

### 3.7 Evidence before claims

A feature is not complete because a command exists, a panel renders, a build succeeds, or an agent says it works. Completion requires tests and observable end-to-end evidence appropriate to the feature.

### 3.8 Cross-platform without platform fiction

Desktop, browser/WASM, and WebOS embedding are meaningful targets, but support claims must be based on builds, installs, launch tests, input behavior, file access, and functional smoke tests on the target environment.

### 3.9 Open ecosystem, controlled architecture

Community contributions and plugins should expand the product without making the core unstable. The maintainers retain architecture, security, quality, and release authority.

## 4. Product capability model

The product should be organized into complementary workspaces. The names are a proposed information architecture, not a mandate to redesign the current UI before its in-progress work lands.

| Workspace | Primary responsibility |
|---|---|
| Graphics | Vector drawing, raster editing, retouching, masks, effects, typography, illustration |
| Design | Pages, artboards, responsive layouts, components, variants, tokens, design systems |
| Prototype | Screen flows, interactions, variables, overlays, transitions, motion previews |
| Assets | Resource libraries, icon packages, sprites, atlases, slices, export regions, optimization |
| Handoff | Measurements, CSS/SVG properties, design tokens, asset dependencies, specifications |
| Automation | Command palette, action recording, CLI, scripts, MCP, batch pipelines, plugins |
| Community | Help, documentation, issue reporting, feature requests, discussions, roadmap, contribution |
| Diagnostics | Document health, compatibility, performance, accessibility, color and font checks |

Do not implement all workspaces as separate applications. They should be views over the same document model and shared services.

---

# 5. Workstream specifications

Each workstream describes the desired product behavior and its acceptance conditions. The labels are stable planning IDs, not GitHub issue numbers. The execution agent should map them into appropriately sized repository tasks after the baseline audit.

## 5.1 Reliability, document integrity, and performance

**Priority: P0 foundation; continuous across all releases.**

### REL-01 — Document integrity and loss prevention

Requirements:
- Autosave with configurable intervals and a clear indication of pending/saved state.
- Recovery generations independent of the primary document file.
- Crash and abnormal-exit recovery with an explicit recovery chooser.
- Transactional document writes; never truncate the last known-good file before a replacement is safely written.
- Atomic replacement on filesystems that support it, with a documented safe fallback elsewhere.
- Browser persistence that uses available durable storage with quota/error reporting and download fallback.
- Versioned serialization and explicit migrations.
- Integrity checks for document structure, references, dimensions, counts, and required metadata.
- Safe handling of interrupted writes, disk-full conditions, permission errors, and unavailable linked assets.
- Optional user-controlled backup and project packaging.
- Recovery of undo/history state where feasible without making document opening depend on a fragile session journal.
- A clear warning before overwriting a file when the source document changed externally.

Acceptance:
- Fault-injection tests cover interruptions during save, write failure, disk-full simulation where practical, invalid paths, and malformed documents.
- The previously saved document remains readable after every failed replacement attempt.
- Crash recovery restores the most recent valid recovery generation and identifies its timestamp/source.
- Open → edit → save → close → reopen preserves semantic document content.
- No save operation reports success before durable completion.
- Autosave and recovery remain bounded in disk use and do not silently overwrite user-selected backups.

### REL-02 — Large-document performance and memory management

Requirements:
- Establish measured performance budgets before optimization.
- Lazy decode/load for large raster assets and expensive subtrees.
- Tile-based or region-based raster processing where appropriate.
- Incremental rendering and dirty-region invalidation.
- Cache keys based on content/version and correct invalidation on edits.
- Bounded caches with deterministic eviction and observable memory accounting.
- Avoid whole-document recomputation for local edits.
- Virtualize large layer, page, font, brush, and resource lists.
- Background work for export, thumbnail generation, analysis, and expensive filters.
- Cancellation and progress for long-running operations.
- Avoid UI-thread blocking during decode, import, conversion, and batch work.
- GPU resource lifecycle management and deterministic CPU fallback.
- A diagnostic report identifying unusually large assets, expensive effects, complex paths, and cache pressure.

Acceptance:
- A versioned benchmark corpus covers small, ordinary, large, and stress documents.
- Measure startup, open, first render, pan/zoom, selection, common edits, save, reopen, export, peak memory, and cache behavior.
- Publish before/after measurements with same host, build profile, corpus, and source revision.
- Stress tests verify no unbounded growth during repeated open/edit/undo/export/close cycles.
- Rendering and editing remain correct when GPU initialization fails or falls back.
- Performance targets are set from observed user workflows and hardware classes, not invented after implementation.

### REL-03 — Robustness and hostile-input handling

Requirements:
- Treat imported files, dimensions, color profiles, fonts, metadata, plugin output, script input, and IPC/MCP requests as untrusted.
- Apply allocation caps and checked arithmetic to input-derived sizes.
- Keep parsers panic-free and add fuzz anchors for new file-format and document surfaces.
- Recover from a failed effect, decoder, font, plugin, or GPU path without corrupting the document.
- Use typed, actionable errors with enough context for troubleshooting but without exposing sensitive paths or contents by default.

Acceptance:
- Malformed-input corpus and fuzz/property tests cover the relevant parser and serializer surfaces.
- Every discovered crash becomes a deterministic regression test.
- Failed operations leave the document unchanged unless partial application is explicitly designed and reported.

## 5.2 Interface, Help, keyboard, and command discovery

**Priority: P1; should follow or integrate with the current shell/UI work.**

### UX-01 — Help Center

Build a searchable Help Center embedded in FireCraft. It should be useful offline for bundled core documentation and able to open online references when available.

Sections:
- Welcome and getting started.
- Workspace and interface guide.
- Tool reference.
- Menus and command reference.
- Keyboard shortcuts and keymap customization.
- Pages, states, artboards, symbols, components, and resources.
- Vector and bitmap editing.
- Typography, fonts, brushes, styles, and color management.
- Import/export format support and known limitations.
- Optimization, batch workflows, and asset generation.
- Prototyping and animation.
- CLI, scripting, MCP, plugins, and APIs.
- Troubleshooting, recovery, and diagnostics.
- Accessibility and platform-specific behavior.
- Release notes, migration notes, and known issues.
- Community, issue reporting, and contribution guides.

Requirements:
- Contextual help from tools, panels, menus, dialogs, and error messages.
- Search by user language, not only internal command names.
- Deep links to a topic or command.
- A consistent documentation source so command names, tool labels, shortcuts, and help do not drift.
- Version-aware guidance for features that differ by release.
- Offline fallback with a clear marker when a topic requires current online documentation.

Acceptance:
- Every shipped tool, menu command, major panel, and supported file workflow has a help entry or an explicit reason why it does not.
- Help search returns useful results for common synonyms and task descriptions.
- Contextual help opens the correct topic.
- Documentation build/link checks are part of CI where practical.

### UX-02 — Universal command and content discovery

Extend the current command palette; do not replace existing fuzzy search, recents, saved-command, or command-registry work.

Searchable domains:
- Commands and actions.
- Tools and tool variants.
- Panels, menus, and settings.
- Keyboard shortcuts.
- Help topics and tutorials.
- Recent documents and operations.
- Fonts, brushes, styles, colors, symbols, components, and assets.
- Plugins and automation tools.
- Document objects and named resources, when contextually appropriate.

Search methods:
1. Exact ID and label.
2. Prefix and token matching.
3. Fuzzy matching with typo tolerance.
4. Synonyms and aliases.
5. Contextual ranking based on selection, workspace, and current task.
6. Optional semantic/natural-language retrieval over command metadata and documentation.

Requirements:
- Use a single command metadata registry where practical: stable ID, display name, description, aliases, category, shortcut, parameters/schema, context, capability, destructive-risk classification, and documentation link.
- Search result ranking must be explainable and deterministic enough to test.
- A command with required parameters should expose a guided parameter form or a clear next step.
- Distinguish disabled, unavailable, destructive, and unavailable-on-this-platform results.
- Allow users to pin commands, inspect shortcuts, and view command documentation.
- Semantic search must be optional and must not make basic search depend on a remote service or model.
- Add keyboard-only navigation and accessible result announcements.

Acceptance:
- Exact, fuzzy, synonym, contextual, and semantic test sets have measurable expected result ordering.
- Every command exposed in search has a valid invocation path or a clearly declared capability gap.
- Search results never advertise a command that is not available in the current build/context.

### UX-03 — Keybinding system

Extend current keymap sets and generated shortcut documentation.

Requirements:
- Per-command rebinding.
- Conflict detection and resolution.
- Context-sensitive bindings with visible precedence rules.
- Workspace-specific profiles.
- Import/export of keybinding profiles.
- Reset to Fireworks/FireCraft defaults.
- Search shortcuts by command, key sequence, and user intent.
- Show shortcut hints in menus, tooltips, command search, and Help.
- A discoverable overlay that displays the current context's most useful shortcuts.
- Correct behavior on Windows, macOS, Linux, and browser keyboard conventions.
- Do not trigger tool shortcuts while typing in text controls or in contexts that own the keystroke.

Acceptance:
- No active binding collision is silently accepted.
- Generated shortcut tables are derived from the actual dispatch system.
- Automated tests cover focus contexts, modifier combinations, operating-system conventions, and multi-stroke bindings if supported.
- Users can recover from a bad profile without losing the ability to open Preferences.

### UX-04 — Workspace and interaction polish

Requirements:
- Complete the currently in-progress shell-fidelity work before broad redesign.
- Consistent selection, focus, snapping, drag, resize, modal, panel, docking, tab, and context-menu behavior.
- Persisted workspace layouts with reset-to-default.
- Responsive behavior at common desktop sizes and high-DPI scales.
- Contextual inspector fields that are selection-kind aware.
- Clear empty, loading, disabled, error, and unsupported states.
- Command discoverability without requiring users to memorize menu hierarchy.
- Optional simplified and advanced workspace presets.
- Respect reduced-motion preferences and user font/scaling preferences.

Acceptance:
- Fresh screenshots and real browser interaction tests are captured for representative empty, vector, bitmap, text, multi-selection, page, state, and export workflows.
- UI snapshots are treated as evidence for exact build hashes, not generic proof.
- Keyboard-only completion tests cover common editing and export workflows.

## 5.3 Color management, print, and image precision

**Priority: P1 for professional graphics workflows.**

### COLOR-01 — Color-management foundation

Requirements:
- Explicit document, working, display, and export color-space concepts.
- ICC profile identification, embedding, assignment, conversion, and preservation where supported.
- sRGB and Display P3 workflows.
- CMYK document and image workflows for print-oriented use.
- Grayscale, RGB, CMYK, and Lab representation/conversion paths as technically supported.
- Rendering intents and black-point compensation where the color-management library supports them.
- Soft-proof preview using a selected output profile.
- Gamut warning and out-of-gamut visualization.
- Profile mismatch warnings on import and paste.
- Clear distinction between assigning a profile and converting pixel values.
- Profile-aware raster import/export and PDF output.
- Web export defaults that explicitly convert to the intended web color space rather than silently relabeling pixels.
- Preserve source profile metadata on linked/embedded assets where possible.
- Document color settings should be inspectable and serializable.

### COLOR-02 — Precision and compositing

Requirements:
- Evaluate 8-bit, 16-bit, and higher-precision per-channel editing support based on actual engine and dependency constraints.
- Maintain correct alpha handling and linear-light operations where appropriate.
- Define color interpolation behavior for gradients and blend modes.
- Avoid unexpected profile changes across CPU/GPU rendering paths.
- Add reference test swatches, alpha edges, gradients, blend modes, and profile conversion fixtures.
- Support transparent, indexed, grayscale, and high-bit-depth inputs with explicit fidelity notes.

### COLOR-03 — Print and production output

Requirements:
- CMYK output when supported by the selected export format.
- PDF export with explicit output intent/profile handling.
- Evaluate PDF/X variants against a clear scope rather than advertising generic PDF/X support.
- Bleed, trim, crop marks, document dimensions, DPI/PPI, and physical units where appropriate.
- Spot-color handling only if the document model and output path can preserve it; otherwise warn and convert explicitly.
- Print preview and color proof.
- Per-page and per-artboard output settings.

Acceptance for COLOR work:
- A documented color pipeline and dependency decision precede broad implementation.
- Golden tests cover known RGB/CMYK/Lab conversions, alpha, gradients, blending, and round trips.
- Profiles are embedded, preserved, or intentionally converted with a user-visible explanation.
- CPU and GPU output remain within a defined measured tolerance.
- No claim of professional print support until real PDF and physical output workflows have been validated.

## 5.4 Typography, fonts, brushes, styles, and resources

**Priority: P1. These systems support both general graphics and modern design workflows.**

### TYPE-01 — Font and typography manager

Requirements:
- Platform-aware installed-font discovery and refresh.
- Search, filtering, favorites, collections, tags, and recently used fonts.
- Family/style grouping and clear identification of variable fonts.
- Live previews using selected document text.
- Variable font axes (for example weight, width, optical size, and custom axes where available).
- OpenType features: kerning, ligatures, stylistic sets, alternates, and localized forms where supported.
- Glyph browser with Unicode search and insertion.
- Character and paragraph styles.
- Text box sizing, wrapping, columns if supported, line-height, tracking, kerning, baseline shift, paragraph spacing/indents, and text-on-path.
- Multilingual shaping, bidirectional text, script-specific shaping, fallback fonts, and appropriate complex-script behavior.
- Missing-font and substitution reports.
- Font substitution preview before applying.
- Document font report and packaging manifest.
- Font embedding only when licensing and format rules permit; never silently embed restricted fonts.
- Browser/WASM font access through explicit browser-supported mechanisms; do not assume unrestricted host font filesystem access.

Acceptance:
- Tests cover font discovery, missing fonts, substitutions, variable axes, OpenType shaping, Unicode, RTL, and complex scripts.
- Test fixtures cover representative scripts and mixed-font paragraphs.
- Font availability changes do not silently alter the saved source text or overwrite the original font metadata.
- Export warns about missing or substituted fonts when appearance may change.

### RES-01 — Brush engine and brush library

Requirements:
- Central brush preset library with categories, tags, favorites, previews, import/export, and user libraries.
- Brush shape, spacing, size, hardness/edge, aspect, angle, scatter, rotation, jitter, and texture controls as supported by the underlying engine.
- Pressure, tilt, velocity, and barrel rotation where supported by hardware and platform APIs.
- Stabilization/smoothing and stroke interpolation.
- Pattern, texture, stamp, scatter, and vector brush families where appropriate.
- Eraser and retouch tools share predictable dynamics and pressure settings.
- Brush previews that show realistic stroke behavior, not just a static icon.
- Resource versioning and missing-brush warnings.
- Brush pack metadata including license, author, source, and compatibility.

Acceptance:
- Brush rendering is deterministic under pinned test settings.
- Input-device capabilities are detected and exposed honestly.
- Mouse/touch fallback remains functional when pen pressure is absent.
- Brush libraries can be imported/exported without corrupting documents or silently replacing existing presets.

### RES-02 — Styles, fills, effects, swatches, and resource manager

Requirements:
- Unified resource browser for styles, fills, strokes, gradients, patterns, textures, swatches, brushes, symbols, components, and assets.
- Reusable appearance styles with editable properties.
- Global colors and linked palette values.
- Theme palettes and semantic color tokens.
- Gradient and pattern libraries.
- Search, tags, previews, favorites, folders, import/export, duplicate, rename, and version metadata.
- Linked versus embedded resource semantics.
- Dependency and usage information: where each resource is referenced.
- Safe rename/delete behavior with usage counts and optional replacement.
- Importers for compatible external resource formats only when the specification and license allow it.

Acceptance:
- Resource edits update linked consumers predictably.
- Deleting a used resource requires an explicit replacement or unlinking decision.
- Resource package manifests include attribution/license metadata.
- Resource search works from the command palette and the resource browser.

### RES-03 — Asset browser and dependency graph

Requirements:
- Browse embedded and linked images, textures, fonts, symbols, components, styles, and generated assets.
- Find broken references and relink with preview.
- Detect duplicates by content hash while preserving intentionally separate instances.
- Show asset dimensions, color profile, file size, type, usage locations, and export settings.
- Identify unused assets and offer review before cleanup.
- Display relationships between source objects, components, design tokens, and generated exports.
- Support project packaging with dependencies and compatibility metadata.

Acceptance:
- Relinking preserves intended object transforms and visual result.
- Duplicate detection is content-based and does not delete without approval.
- Packaged projects open without missing external dependencies when all referenced resources are included.

## 5.5 Pages, artboards, components, states, and design systems

**Priority: P1/P2. Requires a document-model decision before substantial serialization changes.**

### DOC-01 — Pages and artboards

First define the semantic distinction. Do not silently equate a Fireworks page with a Figma-style artboard.

Proposed model:
- **Document:** project-level identity, libraries, profiles, and project metadata.
- **Page:** major workspace or document section, preserving Fireworks-style page behavior.
- **Artboard:** bounded design surface with its own dimensions, background, viewport preset, layout settings, and export identity.
- **Layer/object tree:** editable content associated with a page and, where appropriate, an artboard.
- **State/frame:** legacy Fireworks state or animation-frame semantics; keep distinct from responsive variants and component states.
- **Component state/variant:** a design-system property of a reusable component, not a replacement for document animation frames.

Requirements:
- Multiple artboards per page where appropriate.
- Artboard creation, duplication, rename, reorder, resize, background, clipping, and export.
- Common device and custom viewport presets.
- Artboard-level rulers, guides, grids, and constraints.
- Artboard labels and navigator.
- Copy/paste between artboards with explicit coordinate behavior.
- Artboard templates and responsive sibling relationships.
- Preserve existing Fireworks page/state round-trip behavior.
- Avoid serializing new semantics into legacy nodes without a compatibility decision.

Acceptance:
- A written document-model decision describes pages, artboards, states, frames, components, and variants.
- Migration and unknown-node preservation are tested.
- Multiple artboards render and export independently.
- Existing .fw.png corpus fixtures retain their previous behavior.

### COMP-01 — Components, symbols, variants, and overrides

Requirements:
- Build on existing symbol definitions and instances rather than creating a second unrelated component system.
- Reusable definitions with nested content.
- Instances with stable references.
- Exposed properties for text, color, visibility, image fills, and other safe overrides.
- Variants for size, style, state, and configuration.
- Component states such as default, hover, active, focused, disabled, loading, and custom states.
- Instance overrides that survive definition updates where compatible.
- Explicit detach, reset, and relink workflows.
- Nested components and slot/content areas.
- Component usage search and dependency visibility.
- Clear behavior when a definition is missing or incompatible.
- Reusable libraries across documents.

Acceptance:
- Changing a definition updates instances without erasing valid instance-specific overrides.
- Undo/redo covers definition and instance changes.
- Serialization and migration tests cover nested/overridden components.
- No silent flattening during native save/reopen.

### DESIGN-01 — Design system and variables

Requirements:
- Typed tokens for color, typography, spacing, sizing, radii, borders, shadows, and other supported values.
- Semantic token aliases and modes/themes such as light/dark.
- Variable collections and scoped tokens.
- Component libraries and shared styles.
- Import/export of token formats such as JSON and supported Design Tokens Community Group formats when the spec and implementation support them.
- CSS custom-property export.
- Token usage graph and change impact preview.
- Detect hard-coded values that should reference tokens.
- Design-system documentation and version notes.
- Document/library linking and conflict handling.

Acceptance:
- Tokens resolve deterministically.
- Changing a token updates its consumers and can be undone.
- Exported tokens round-trip through supported schemas.
- Missing or renamed tokens create explicit diagnostics instead of silently falling back.

### DESIGN-02 — Responsive layout

Requirements:
- Constraints and anchors.
- Auto-layout stacks and groups.
- Gap, padding, alignment, wrapping, sizing modes, min/max sizes.
- Flexbox-like layout semantics.
- CSS Grid-like rows, columns, tracks, and gaps.
- Breakpoints and responsive variants.
- Intrinsic/content sizing where feasible.
- Nested scroll containers, clipping, sticky/fixed positioning where supported.
- Layout preview at multiple viewport widths.
- Inspection of the underlying layout model and CSS mapping.

Acceptance:
- Layouts reflow at tested viewport sizes without manual object repositioning.
- Constraints are serialized and survive reopen.
- CSS/HTML handoff reports unsupported properties instead of emitting misleading output.
- Do not market the feature as a browser layout engine; define the supported subset.

## 5.6 Unified vector and raster editing expansion

**Priority: P1/P2; prioritize coherent workflows over tool-count inflation.**

### EDIT-01 — Vector depth

Potential enhancements:
- Robust node and handle editing.
- Pen, curvature, reshape, freeform, and pencil workflows.
- Boolean operations with stable topology.
- Simplify, smooth, offset, inset, expand stroke, and outline stroke.
- Variable-width strokes and pressure-sensitive vector paths.
- Shape builders and parametric/autoshape editing.
- Mesh/gradient improvements where feasible.
- Live corners and editable shape parameters.
- Pattern and vector brushes.
- Text on paths and path-aware layout.
- SVG-native object inspection.

Acceptance:
- Geometry operations use property tests, adversarial shapes, and deterministic fixtures.
- Degenerate/self-intersecting inputs fail safely or produce documented results.
- Visual tests cover zoom, transforms, stroke joins/caps, fills, and clipping.

### EDIT-02 — Raster depth

Potential enhancements:
- Layer and pixel masks with clear resolution semantics.
- Selection improvements: feather, grow/shrink, smooth, invert, save/restore, color range.
- Clone, healing, content-aware selection/removal where technically realistic.
- Transform, crop, perspective, and non-destructive adjustment workflows.
- Adjustment layers and filter stacks with reorder/disable/parameter editing.
- Channel inspection and alpha editing.
- High-bit-depth workflows where supported.
- Brush dynamics and tablet input.
- Blend-mode and color-space correctness.

Acceptance:
- Raster edits are undoable and preserve alpha.
- Filters and adjustments do not destructively flatten source data unless the user requests it.
- Memory and render cost are measured for large images.
- The UI accurately distinguishes destructive pixel edits from live/nondestructive operations.

## 5.7 Import, export, compatibility, and interoperability

**Priority: P0/P1 because trust in creative files is foundational.**

### IO-01 — Format capability registry

Create a machine-readable format registry that records:
- Extension and media type.
- Import/export support.
- Supported feature mapping.
- Editable, transformed, flattened, reference-only, or rejected behavior.
- Color-space and bit-depth support.
- Metadata and profile handling.
- Animation/page/layer behavior.
- Known limitations.
- Test fixture coverage and last verification.
- Platform/dependency requirements.

Expose these capabilities in file dialogs, Help, CLI, and developer documentation.

### IO-02 — Interchange targets

Assess and prioritize formats by user demand and actual implementation feasibility:
- Fireworks PNG native format and compatibility.
- PNG, JPEG, GIF (including animated), BMP, TIFF, WebP, WBMP.
- AVIF and JPEG XL where decoder/encoder/platform constraints permit.
- SVG and SVGZ.
- PDF, including print-oriented variants only after proof.
- PSD and PSB, with layered fidelity clearly described.
- EPS and AI-compatible workflows only where the format path is defensible.
- OpenRaster or other open layered raster interchange if it offers a meaningful workflow.
- Animation formats such as APNG and animated WebP/AVIF where codecs support them.
- Platform icon formats and asset packages.

Do not add extensions to a supported-format list until the import/export path has round-trip or documented fidelity tests.

### IO-03 — Non-destructive compatibility and round trips

Requirements:
- Preserve layers, groups, vectors, text, masks, effects, blend modes, profiles, and metadata where possible.
- Maintain unrecognized/unsupported metadata when safe and structurally valid.
- Show a conversion report before destructive flattening or loss of editability.
- Keep the original file untouched unless the user explicitly overwrites it.
- Provide a preview and report of unsupported features.
- Maintain a corpus of real-world and synthetic documents.
- Compare both rendered pixels and semantic object structure.

Acceptance:
- Every supported format has a compatibility matrix and representative fixtures.
- Import/export is tested in both directions where meaningful.
- Known loss cases are asserted and reported, not ignored.
- No format is advertised as fully editable based on a flattened preview.

## 5.8 Modern export, optimization, and production assets

**Priority: P1.**

### EXPORT-01 — Export pipeline

Requirements:
- Export pages, artboards, states, layers, selections, named regions, and individual assets.
- Multiple formats/resolutions per source.
- 1x/2x/3x/custom scale factors.
- Export presets and named recipes.
- Batch export with progress, cancellation, error summary, and retry.
- Consistent naming templates and output directory rules.
- Preserve metadata and color profiles according to the chosen output profile.
- Compare original/optimized output visually and numerically.
- Compression controls, palette reduction, dithering, transparency, matte, progressive modes, and format-specific options.
- SVG optimization/minification with optional readable output.
- Avoid unnecessary recomputation using content hashes.
- Watch/rebuild mode for development pipelines where safe.
- Export manifests documenting source, settings, output hashes, dimensions, and warnings.

Acceptance:
- Same source, settings, and pinned encoder produce deterministic outputs where the codec permits.
- Export failures do not leave apparently complete partial packages.
- The manifest can identify stale or changed outputs.
- Visual and file-size comparisons are available before finalizing an optimization preset.

### ASSET-01 — Icon and application asset studio

Requirements:
- Create one master icon and generate multiple platform sizes and variants.
- Windows ICO, macOS ICNS, Linux icon themes, Android mipmaps/adaptive icon resources, and iOS asset catalog outputs as platform requirements permit.
- Safe-area, padding, mask/shape previews, background variants, and light/dark variants.
- Generate favicon and web app icon packages.
- Export source SVG plus raster resolutions.
- Validate required sizes, alpha, dimensions, and platform naming.
- Package outputs with a manifest and preview sheet.

Acceptance:
- Each platform package is validated against its documented structure.
- Generated outputs are derived from a single source of truth.
- No platform support claim is made until the package has been consumed by a real target application/toolchain.

### ASSET-02 — Sprite and texture atlas generator

Requirements:
- Pack selected assets or animation frames into atlases.
- Configurable atlas size, power-of-two options, padding, extrusion, rotation, trimming, and transparent bounds.
- Deterministic MaxRects/skyline or similarly justified packing algorithm.
- Per-sprite pivot, origin, scale, and frame duration.
- Export atlas image and metadata in documented JSON/XML/game-engine-friendly schemas.
- Detect overflow and offer split-atlas options.
- Preview packed/unpacked layouts and wasted space.
- Rebuild only affected atlases when source assets change.

Acceptance:
- Packing is deterministic for the same source/order/settings.
- Metadata coordinates and image pixels are validated together.
- Padding/extrusion prevents texture bleeding in test fixtures.
- Oversized inputs are refused safely.

### BATCH-01 — Batch graphics production

Requirements:
- Batch conversion, resize, crop, format optimization, color conversion, watermark, rename, and export.
- Saved processing recipes.
- Parameterized operations and safe input/output globs.
- Dry-run and preview mode.
- Collision and overwrite policy.
- Per-file result log and machine-readable summary.
- Cancellation and resumability for long jobs.
- Headless use through CLI and optional GUI queue.
- Same operation implementation exposed through the shared command/pipeline API.

Acceptance:
- Batch outputs match equivalent single-document operations.
- Failure in one input does not silently corrupt or prevent reporting on other inputs.
- Dry-run lists intended outputs and destructive actions.
- CI tests cover naming collisions, invalid files, interrupted jobs, and large queues.

## 5.9 SVG and animation laboratory

**Priority: P2.**

### SVG-01 — Advanced SVG laboratory

Requirements:
- Inspect and edit SVG source alongside visual rendering.
- Object/path/style hierarchy.
- ViewBox and coordinate inspection.
- Editable gradients, masks, clipping paths, filters, text, and symbols where supported.
- SVG cleanup, minification, path simplification, ID normalization, and accessibility metadata.
- Animation authoring and inspection where supported.
- Import diagnostics for unsupported elements and CSS.
- Preserve original source when opening/editing complex files.
- Preview across browser engines when practical.

Acceptance:
- SVG round-trip corpus covers real-world gradients, clipping, masks, text, filters, viewBox, CSS, and namespaces.
- Unsupported features are reported with source location/context.
- Sanitization protects against scripts, external resource loading, and unsafe links when previewing untrusted SVG.

### ANIM-01 — Animation timeline and motion authoring

Extend the existing states/timeline functionality; keep Fireworks frame/state semantics distinct from modern motion timelines.

Requirements:
- Timeline tracks and keyframes.
- Position, size, rotation, opacity, color, and supported effect properties.
- Easing curves and spring-like interpolation.
- Motion paths.
- Component-state transitions.
- Timeline groups and reusable animation presets.
- Onion skinning where useful.
- Playback, scrubbing, loop, frame rate, and duration controls.
- Reduced-motion preview.
- Export GIF, APNG, animated WebP/AVIF, and video only when a tested encoder is available.
- Export frame sequences and animation metadata.
- Avoid conflating animation keyframes with responsive component variants.

Acceptance:
- Timeline playback is deterministic for fixed settings.
- Exported frame sequence matches preview within a defined tolerance.
- Timeline edits participate in undo/redo and document serialization.
- Long animations are memory-bounded and cancellable.

## 5.10 Interactive prototyping

**Priority: P2; depends on stable pages/artboards/components.**

### PROTO-01 — Prototype interaction model

Requirements:
- Screen/artboard navigation.
- Triggers: click/tap, hover, press, keyboard, scroll, and supported lifecycle events.
- Actions: navigate, back, overlay open/close, scroll to, set variable, toggle, focus, and supported state changes.
- Component interactions and state transitions.
- Variables and typed values.
- Conditional branches and comparisons.
- Input controls and validation states.
- Overlays, menus, dialogs, tooltips, and drawers.
- Transition effects and timing.
- Device viewport and orientation previews.
- Shareable/exportable prototype packages, with local/offline preview.
- Interaction graph and broken-link diagnostics.
- Accessibility metadata and keyboard/focus behavior.

Acceptance:
- Prototype runtime is isolated from the editor's document mutation state unless a deliberate authoring action is made.
- Interaction graph validates targets, variables, and unreachable screens.
- Click/tap and keyboard paths are tested.
- Exported prototype remains usable without the authoring application where the packaging contract says it should.
- Prototype actions are not misrepresented as production application code.

## 5.11 Developer handoff and design-to-code

**Priority: P2.**

### DEV-01 — Inspect and handoff mode

Requirements:
- Inspect selected elements, groups, components, and artboards.
- Geometry, alignment, spacing, padding, gaps, constraints, and hierarchy.
- Typography, font, weight, line-height, tracking, OpenType options, and font dependency.
- Color and profile values with HEX, RGB, HSL, and supported CMYK/Lab readouts.
- Fill, stroke, radius, shadow, effects, opacity, and blend properties.
- SVG source and asset dimensions.
- Design token references.
- Component properties, variants, and overrides.
- Export selected assets and copy asset references.
- Responsive viewport inspection.
- Contrast and accessibility annotations.
- JSON design specification export.
- CSS declarations and supported layout mapping.
- Optional framework-oriented examples, clearly labeled as generated guidance rather than guaranteed production code.

### DEV-02 — Design-to-code contract

Requirements:
- Define a framework-neutral semantic design representation first.
- Keep CSS, HTML, React, Vue, Angular, and other code generators as adapters over that representation.
- Report unsupported visual/layout features instead of silently approximating them.
- Preserve stable IDs and source references to document objects.
- Generate clean design tokens and asset manifests.
- Support copy-to-clipboard and exportable handoff packages.
- Provide a validation preview for generated CSS/layout where possible.

Acceptance:
- Generated properties match the document model.
- Unsupported mappings have explicit warnings.
- Generated artifacts can be regenerated without manually editing the source design.
- No claim of arbitrary design-to-production-code parity.

## 5.12 Accessibility inspector and accessible authoring

**Priority: P1/P2; continuous.**

### A11Y-01 — Accessibility inspector

Requirements:
- Contrast calculations using the applicable color profile/relative luminance basis and clearly documented assumptions.
- Text size and legibility checks.
- Touch target sizing.
- Focus order and visible focus state.
- Keyboard reachability and trap detection.
- Accessible names/labels for controls and exported assets.
- Alt text and decorative-image distinction.
- Semantic grouping and hierarchy.
- Reduced-motion and animation warnings.
- Color-only information warnings.
- Language and direction metadata.
- Document and prototype accessibility annotations.
- Exportable report with severity, affected object, reason, and suggested remediation.
- Ability to suppress a warning with a documented reason.

### A11Y-02 — Accessible application UI and canvas representation

Requirements:
- Audit menus, dialogs, panels, tools, command search, inspector, and canvas object navigation.
- Make selection and layer hierarchy discoverable to assistive technology.
- Provide an object navigator/list as a semantic alternative to visual-only canvas interaction.
- Provide keyboard equivalents for critical actions and visible focus.
- Test native desktop screen-reader paths.
- Revisit the documented browser/WASM AccessKit limitation in [decision 0067](decisions/0067-wcag-aaa-web-host.md). Do not claim web screen-reader support until an actual accessible tree or equivalent semantic interface is available and verified.
- Use criterion-by-criterion conformance reporting; do not claim blanket WCAG conformance from a contrast check.

Acceptance:
- Automated tests cover semantic labels, keyboard navigation, contrast, focus, and report generation.
- Manual assistive-technology test receipts identify OS, screen reader, version, host, and tested workflows.
- Web limitations remain explicit until an end-to-end browser assistive-technology test passes.

## 5.13 CLI, scripting, MCP, and automation

**Priority: P1/P2; preserve current shared command architecture.**

### AUTO-01 — Shared automation contract

Extend, do not replace, the existing command registry, CLI, scripting host, browser bridge, and MCP server.

Requirements:
- Stable, versioned command IDs and typed parameter schemas.
- Capability discovery by application version, platform, and document context.
- Structured error codes and machine-readable results.
- Progress and cancellation for long operations.
- Transaction boundaries and rollback where possible.
- Read-only and mutation capability scopes.
- Document/session selection and concurrency rules.
- Command deprecation policy and compatibility aliases.
- Audit trail for automation-originated operations.
- Deterministic headless render/export interface.
- JSON output mode and stable CLI exit-code contract.
- Safe path handling and explicit read/write roots for automation.

### AUTO-02 — MCP expansion

Expose appropriate tools, resources, and prompts for:
- Inspect document/pages/artboards/layers/components.
- Search fonts, brushes, styles, and assets.
- Inspect format compatibility and document health.
- Create/edit vector and raster objects.
- Modify text, styles, colors, and tokens.
- Manage artboards and components.
- Validate accessibility and layout.
- Build/export asset packages.
- Run batch pipelines.
- Inspect prototype graphs and animation metadata.
- Export previews and handoff specifications.
- Read command/tool/help metadata.

Safety requirements:
- Read-only by default for newly connected agents.
- Explicit write permission and scoped read/write roots.
- Human confirmation for destructive, bulk, overwrite, external-network, or irreversible actions.
- Preview changes before applying when practical.
- One transaction per meaningful agent operation.
- Undo and rollback support.
- Clear user-visible indication when an agent edits a document.
- Never expose arbitrary filesystem access or secrets through MCP.
- Keep the protocol provider-independent.

### AUTO-03 — CLI and action pipelines

Requirements:
- Stable verbs for inspect, validate, convert, render, export, optimize, package, batch, and report.
- Machine-readable JSON and useful human-readable output.
- Nonzero exit codes on errors, stderr diagnostics, and pipe-safe stdout.
- Dry-run, progress, cancellation, and output manifests.
- Saved reusable action pipelines.
- Parameter validation and versioned schemas.
- Headless operation without loading a desktop UI.
- CLI/API documentation generated or validated against actual command schemas.

Acceptance:
- CLI and MCP operations produce equivalent engine/document results to corresponding GUI commands.
- Contract tests protect command schemas and exit codes.
- Automation operations are replayable and diagnosable.

## 5.14 Plugin ecosystem and extensibility

**Priority: P2/P3.**

### PLUG-01 — Plugin API and SDK

Requirements:
- A documented plugin manifest with ID, name, version, author, license, compatibility range, entrypoint, capabilities, permissions, and dependencies.
- Versioned public extension API.
- Hooks for tools, commands, panels, inspectors, import/export adapters, resource providers, and batch steps where safe.
- UI extensions isolated from core state mutation unless using approved command APIs.
- Sandboxed execution for untrusted plugins, preferably WASM or another constrained runtime suitable for the target.
- Permission prompts for filesystem, network, document mutation, clipboard, and external process access.
- Plugin signing or integrity verification policy.
- Dependency and vulnerability reporting.
- Enable/disable, update, rollback, and safe mode.
- No silent loading of arbitrary native libraries from untrusted folders.
- Compatibility diagnostics for plugins after application updates.

### PLUG-02 — Plugin distribution and community catalog

Requirements:
- Community-maintained catalog metadata.
- Search, tags, compatibility, release history, license, permissions, and maintainer info.
- Install from a local package and configured sources.
- Review permissions before activation.
- Clearly distinguish official, verified community, and unverified plugins.
- Report malicious or abandoned plugins.
- Reproducible package hashes and release provenance where possible.

Acceptance:
- A sample third-party plugin can add a command and a panel without modifying the FireCraft core.
- Incompatible or malicious packages are rejected safely.
- Plugin failure cannot corrupt the document or crash the application.
- Plugin APIs have compatibility tests and documented deprecation policy.

## 5.15 Open-source community, feedback, and in-app issue tracking

**Priority: P1 for community experience; implementation should remain lightweight.**

FireCraft is already an open-source project. The objective is to make contribution and feedback easier, not to create a second issue-tracking system that competes with GitHub.

### COMMUNITY-01 — In-app Community Center

Proposed sections:
- Help and tutorials.
- Report a bug.
- Request a feature/change.
- Discussions and workflow questions.
- Known issues and workarounds.
- Public roadmap and milestones.
- Releases and changelog.
- Contribute code, translations, documentation, tests, and resources.
- Security and responsible disclosure guidance.

### COMMUNITY-02 — Bug-report templates and diagnostic capture

Bug report fields:
- Short summary.
- Product version/build hash.
- Operating system and version.
- Application target (desktop/browser/WASM/WebOS).
- Relevant CPU/GPU/driver/runtime details.
- Area/category.
- Expected result.
- Actual result.
- Reproduction steps.
- Frequency and severity.
- Related document/operation type.
- Optional screenshot, sample file, crash dump, or logs.

Diagnostics:
- Generate a previewable report.
- Redact tokens, credentials, personal paths, usernames, and sensitive environment values.
- Do not automatically upload artwork, documents, screenshots, or original filenames.
- Ask for explicit consent before attaching files or sending diagnostic bundles.
- Support copying a Markdown report when online submission is unavailable.

### COMMUNITY-03 — Feature request/change request templates

Fields:
- Problem/opportunity statement.
- Intended user and workflow.
- Current workaround.
- Proposed behavior.
- Expected benefit.
- Scope and affected workspaces.
- Alternatives and trade-offs.
- Mockups/examples.
- Compatibility or migration impact.
- Optional willingness to test or contribute.

Use a separate template for documentation, translation, resource/library contribution, and developer API requests where appropriate.

### COMMUNITY-04 — GitHub as the canonical backend

Preferred initial design:
- GitHub Issues remains the canonical bug/feature tracker.
- GitHub Discussions remains the canonical open-ended community forum where enabled.
- GitHub Projects or an equivalent public roadmap remains the source for project status.
- FireCraft provides an in-app, low-friction client for searching, submitting, following, and viewing these records.
- Do not create a parallel issue database unless there is a documented need and a migration/synchronization design.
- Do not embed a maintainer's personal GitHub token in the app.
- Use a secure user-authorized OAuth/device flow or a narrowly scoped public-service broker if direct API submission requires authentication.
- When auth or network access is unavailable, allow report copy/export and open the canonical web issue form.

### COMMUNITY-05 — Intelligent search and duplicate detection

Requirements:
- Search open, closed, and recently resolved issues before submission.
- Exact phrase, fuzzy, synonym, category/subsystem, and optional semantic similarity.
- Suggest likely duplicates with title, status, labels, last update, and link.
- Let users add context or subscribe to an existing issue rather than creating a duplicate.
- Show confidence and why a match was suggested.
- Never automatically close, merge, or alter an issue because of similarity.
- Rate-limit and cache responsibly; degrade gracefully when the API is unavailable.

### COMMUNITY-06 — Public status and roadmap

Requirements:
- Visible lifecycle states: submitted, triage, needs information, accepted, planned, in progress, testing, released, deferred, declined, duplicate, and cannot reproduce.
- Changelog/release link when resolved.
- Subscribe/follow and notification preferences.
- Votes/reactions as demand signals, not an automatic priority algorithm.
- Clear maintainer notes explaining decisions for declined/deferred requests.
- Public roadmap that distinguishes committed work from ideas and exploratory research.

### COMMUNITY-07 — Open-source project health

Preserve and improve the existing license and project governance rather than casually changing them:
- Apache-2.0 license and attribution policy remain authoritative unless the owner approves a change.
- Keep clean-room boundaries explicit.
- Maintain contribution guide, code of conduct, security policy, issue templates, pull-request templates, and maintainer decision process.
- Publish architecture decisions for consequential dependencies, document changes, command API changes, and format behavior.
- Provide reproducible builds, release checksums, SBOMs, and provenance/signing as the release infrastructure supports them.
- Encourage community contributions in code, tests, docs, translations, accessibility, sample files, plugins, and tutorials.
- Publish a stable native-format and automation contract where practical.

Expected community benefits:
- Lower barrier to reporting problems and contributing.
- More hardware/platform coverage.
- Broader format interoperability and plugin innovation.
- Independent review and security analysis.
- Long-term access to user documents and the ability to maintain forks.
- Better educational, research, and downstream integration opportunities.
- Reduced vendor lock-in and dependence on a single contributor.

Acceptance:
- A user can search existing issues and submit a well-formed report without manually navigating GitHub.
- The canonical issue URL and status are shown after submission.
- Offline and unauthenticated fallbacks work.
- Private security reports are routed to the security policy rather than exposed as public issues.
- Diagnostic submission is opt-in and inspectable.

## 5.16 Collaboration, versions, and project history

**Priority: P3 after stable document identities and local-first semantics.**

### COLLAB-01 — Local versioning and visual history

Requirements:
- Named checkpoints and milestones.
- Compare current document against a selected checkpoint.
- Visual diff and structural diff.
- Revert selected objects or full document where safe.
- Branch/fork a design direction.
- Document change summary and source metadata.
- Exportable project history.
- History retention and storage limits.
- Link assets and references to the revision they depend on.
- Clear behavior when edits occur in another process or from another copy of the document.

### COLLAB-02 — Multi-user collaboration (optional service)

Requirements:
- Define stable object identities and conflict semantics before implementing real-time collaboration.
- Evaluate CRDT/OT or another suitable merge design against the document graph and high-frequency canvas edits.
- Presence, selection, comments, and review markers.
- Permissions and share scopes.
- Conflict resolution for concurrent style, text, geometry, and component changes.
- Offline edits and later reconciliation if supported.
- Optional self-hosted service or provider-neutral protocol.
- Local-only projects remain fully usable.
- End-to-end encryption and data-retention policy if hosted synchronization is offered.

Acceptance:
- No collaborative feature ships with undefined conflict behavior.
- Concurrent edits are tested for convergence and data loss.
- Users can export their complete document and history without being locked into a hosted service.

## 5.17 AI-assisted editing and agent workflows

**Priority: P3; optional and reversible.**

### AI-01 — AI-assisted creative operations

Potential capabilities:
- Background removal and selection assistance.
- Object cleanup and removal.
- Generative fill/extension.
- Image enhancement, denoise, upscale, and restoration.
- Raster-to-vector tracing assistance.
- Palette and visual-style suggestions.
- Layout and composition proposals.
- Icon and asset variations.
- Accessibility suggestions.
- Document cleanup and design-lint recommendations.
- Natural-language command discovery and multi-step editing plans.
- Contextual Help and tutorial recommendations.

Design constraints:
- AI is an optional adapter, not a dependency of basic editing.
- Support local inference where feasible and useful.
- Remote providers are opt-in and have clear data handling disclosure.
- Never transmit a document or selected content without a user-authorized action.
- Present previews and a clear before/after comparison.
- Apply accepted changes as undoable command transactions.
- Keep the original source content recoverable.
- Record provenance for generated/modified assets where useful.
- Make provider/model settings explicit.
- Do not make unsupported claims about local model performance or output licensing.
- Respect document privacy, resource limits, and platform constraints.

### AI-02 — Agent collaboration

Requirements:
- Use the shared CLI/MCP/automation contract, not a parallel hidden agent API.
- Agents can inspect before mutating.
- Scope read/write roots and document permissions.
- Require explicit confirmation for destructive or external actions.
- Group related edits into named undoable transactions.
- Provide a human-readable change summary and affected-object list.
- Support preview, apply, rollback, and audit.
- Agents may generate tests, documentation, tokens, and asset manifests as well as visible artwork.

Acceptance:
- A supported MCP client can inspect a document, propose a change, preview it, apply it with permission, and undo it.
- An agent cannot access arbitrary files or transmit user content outside the granted scope.
- A failed agent action leaves the document unchanged.

## 5.18 Cross-platform delivery and integration

**Priority: continuous; release certification is evidence-driven.**

### PLATFORM-01 — Platform matrix

Maintain separate readiness for:
- Windows x86_64 and ARM64 where supported.
- macOS Apple Silicon and Intel where supported.
- Linux x86_64 and ARM64 where supported.
- Browser/WASM.
- WebOS embedding and host integration.
- Optional Android or other targets only after a scoped feasibility decision.

For every platform, distinguish:
- Source-compiles.
- Package-builds.
- Installs.
- Launches.
- Opens/edits/saves.
- Export/clipboard/file dialogs/input tested.
- Published.
- Certified for the declared scope.

Do not call a packaging recipe a supported platform.

### PLATFORM-02 — OS integration

Requirements:
- Native file open/save dialogs and drag/drop.
- Correct file associations and MIME types.
- Clipboard import/export and image copy/paste.
- High-DPI scaling and multi-monitor behavior.
- Color profile/display integration where available.
- Pen/tablet input and pressure where platform APIs support it.
- Font enumeration and fallback.
- Application menus, shortcuts, and accessibility.
- Update/uninstall behavior and user-data preservation.
- Browser storage and download fallback.
- WebOS host API/version compatibility.

### PLATFORM-03 — Release and ecosystem integration

Requirements:
- Reproducible artifact build from a known commit.
- Checksums, manifests, SBOM, provenance, and signatures as configured.
- Release notes and upgrade/migration guidance.
- Native target install/launch smoke tests.
- Test against the project's WebOS host contract.
- Versioned public SDKs and APIs for embedding/integration.
- Explicit compatibility windows and deprecation policy.

## 5.19 Documentation, education, and localization

**Priority: continuous.**

Requirements:
- User guide and feature tutorials.
- Short task-oriented recipes.
- Sample documents that exercise major features.
- Format compatibility guide.
- Developer and plugin SDK documentation.
- CLI/MCP examples and schemas.
- Keyboard and accessibility documentation.
- Localization-ready UI strings and pluralization.
- Translation contribution process.
- In-app What's New and migration summaries.
- Release-specific known limitations.

Acceptance:
- Documentation is built or link-checked in CI.
- New user-facing features include documentation in the same delivery.
- Screenshots identify the exact build/version they represent.
- Localization does not concatenate translated fragments or assume English string order.

---

## 5.20 Product marketing website and project presence

### SITE-01 — Public FireCraft marketing website

**Priority: P1.** This is a public product-discovery, adoption, download, and contribution surface. It may proceed as a separately owned static-site track after the current editor baseline is reconciled; it must not destabilize the editor, its WebAssembly build, or its Pages deployment.

**Existing foundation:** the inspected repository snapshot already contains a standalone static site under the site directory (HTML, CSS, JavaScript, and approved brand assets), plus a root Wrangler configuration that names firecraft-site and points at the site output. The existing browser editor deployment is documented separately as firecraft-web and uses dist/web. These paths must remain separate. Re-audit the actual local tree and Cloudflare dashboard before changing either.

**Objective:** provide a compelling, credible public home for FireCraft that helps a visitor understand the product, see authentic editor workflows, try or download the current release, assess maturity, and contribute.

**Required experience:**
- Clear product positioning: an open-source creative editor combining vector and bitmap workflows, inspired by classic Fireworks and built on a modern Rust foundation.
- Strong first-screen explanation and primary actions: try the browser build when verified, download a supported release, see real screenshots, and explore the source.
- Feature pages or sections that distinguish Available, Preview, In development, and Planned capabilities.
- Authentic screenshots and gallery material captured from identified current builds, with captions, alt text, and evidence dates.
- Current release/download discovery driven by published GitHub release assets, checksums, and manifests, with robust fallback behavior.
- Getting-started guidance, platform availability, format-fidelity limitations, project/community information, contribution paths, license, and attribution.
- Responsive design for desktop, tablet, and mobile web viewports; accessible navigation and keyboard operation.
- Search metadata, canonical/social metadata, sitemap, robots policy, a useful not-found response, and appropriate security/cache headers.

**Content integrity:**
- Every externally visible capability, format, platform, performance, and privacy claim must link internally to current source/test/release evidence.
- Do not market roadmap items as shipped features.
- Do not call FireCraft a complete Photoshop/Figma/Fireworks replacement or claim format/platform parity without evidence.
- Do not show planned installers as downloads. Availability must be derived from current published release assets.
- Distinguish real product screenshots from mockups or concept illustrations.
- Keep public copy aligned with FEATURES.md, CS6-PARITY.md, TASKS.md, release notes, and the verified build.

**Technical and deployment requirements:**
- Begin with the existing site directory and improve it incrementally; do not create a duplicate marketing application or replace it with a framework starter without a measured reason.
- Keep the site static and independent from the Rust editor workspace, editor WASM build, and editor runtime. No backend, account system, database, or Worker is required for the initial site.
- Preserve the approved FireCraft brand asset and existing Apache-2.0/attribution rules. Do not copy assets, UI artwork, copy, or code from benchmarked commercial products.
- Audit the current GitHub release API client for safe DOM construction, URL allowlisting, rate-limit/network/error handling, prerelease handling, and a working Releases-page fallback.
- Keep secrets and privileged tokens out of all browser assets and public build variables.
- Avoid unapproved trackers, session replay, advertising pixels, forms, and third-party script/font requests. Any analytics requires an explicit privacy decision and disclosure.
- Maintain a separate Cloudflare Pages project for the public marketing site (the inspected config names firecraft-site) and preserve the existing editor project firecraft-web. Confirm actual project state and deployment mode before using Wrangler or dashboard controls.
- If Git integration is selected, use monorepo root/build settings and build-watch paths scoped to the site and its actual shared dependencies. If the existing project uses direct upload, do not migrate deployment mode without a separate owner-approved decision.
- Preview and production deployment must be independent of the editor's dist/web deployment. Production deploy requires explicit owner approval and a receipt identifying source SHA, project, deployment URL, and rollback target.

**Acceptance criteria:**
1. The existing site is audited and improved rather than duplicated.
2. The principal visitor journeys work from a clean browser on desktop, tablet, and phone.
3. Every product claim maps to verified implementation/release evidence and clearly states maturity.
4. Screenshots and third-party resources have known provenance, rights, accessible alternatives, and optimized delivery.
5. Release downloads reflect only real published assets; malformed responses, rate limiting, network outage, and missing assets degrade safely.
6. All routes work on direct navigation and refresh; site output is static and Pages-compatible.
7. Automated and manual accessibility checks are recorded; WCAG 2.2 AA is the target, not a claim based on score alone.
8. Mobile Lighthouse Performance is at least 90 and Accessibility, Best Practices, and SEO are at least 95 on recorded representative runs, or an exception is documented with remediation.
9. SEO metadata, sitemap, robots, not-found behavior, security headers, caching, and external links are validated.
10. Site changes cannot alter the editor deployment target or its production configuration.
11. Preview deployment is reviewed; production launch is owner-approved and verified independently.
12. README, documentation index, and operational handoff point to the actual public site only after its production URL is verified.

**Detailed specification:** [MARKETING-SITE-ROADMAP.md](MARKETING-SITE-ROADMAP.md). The existing static implementation and Cloudflare deployment topology are the starting point, not proof of launch status.

**Benchmark inputs:** [creative-app-benchmark.md](research/creative-app-benchmark.md) informs the product-feature presentation and differentiation, but benchmark applications are not runtime dependencies or sources of copied code/assets.

---

# 6. Prioritization and dependency order

This roadmap is not a mandate to implement every workstream in sequence. The project should use parallel specialist work only where ownership boundaries are clean and shared interfaces have been agreed.

## 6.1 Priority definitions

| Priority | Meaning |
|---|---|
| P0 | Prevent data loss, resolve release blockers, preserve architectural integrity, or complete the current MVP baseline. |
| P1 | High-value everyday usability, reliability, format trust, typography/resources, and production workflows. |
| P2 | Major modern design, prototype, handoff, and extensibility capabilities. |
| P3 | Optional collaboration, advanced AI, hosted/community services, and later ecosystem expansion. |
| Research | Requires a spike, dependency assessment, user evidence, or architecture decision before committing to implementation. |

Priority is not a promise of delivery date. Reprioritize with evidence and explicit maintainer approval.

## 6.2 Recommended execution sequence

### Gate 0 — Finish and reconcile current development

- Let the current integration work complete.
- Inspect local changes and all relevant branches/PRs.
- Reconcile local UI/UX and feature work with main.
- Re-run current task and parity ledgers against the merged tree.
- Close or re-scope the current MVP blockers.
- Do not start parallel post-MVP feature implementation while the shared document model, shell state, or current branch integration is unsettled.

**Exit:** owner-approved MVP baseline, clean handoff, current task ledger and parity evidence refreshed.

### Track M — Public product website

This track begins only after Gate 0 has reconciled the current local editor work and deployment ownership. It can be implemented independently of core editor work when the site directory and release interfaces have clear ownership.

- SITE-01: audit and elevate the existing static site; do not scaffold a duplicate.
- Verify the current Pages topology: marketing site firecraft-site versus editor firecraft-web; inspect each project's real deployment mode, production branch, domain, and output path.
- Build content from the audited current product state, authentic screenshots, published release assets, and verified documentation.
- Improve responsive design, accessibility, SEO, privacy, safe release metadata, and performance.
- Use Pages preview deployments and browser evidence before production.
- Obtain explicit owner approval before production deployment and update the operational handoff with the verified deployment recipe.

**Exit:** a credible, accessible, responsive public site with correct downloads and evidence-backed claims, independently deployed and verified without changing the editor deployment.

### Wave 1 — Production confidence and discoverability

- REL-01/02/03: persistence, data integrity, and performance audit.
- IO-01/03: format capability matrix and round-trip regression suite.
- UX-01/02/03: Help Center, universal discovery, and complete keybinding editor.
- UX-04: finish in-progress UI/UX polish.
- A11Y-01/02: accessible UI and object navigation plan.
- PLATFORM-01: honest per-target readiness baseline.

**Exit:** reliable daily editing, predictable recovery, documented support matrix, discoverable commands/tools, and measured performance.

### Wave 2 — Professional creative foundation

- COLOR-01/02/03: color management, profiles, CMYK, precision, and print.
- TYPE-01: font and typography manager.
- RES-01/02/03: brush, style, swatch, resource, and asset manager.
- EDIT-01/02: targeted vector/raster gaps found by use and parity evidence.

**Exit:** mature typography/resources/color workflows and documented fidelity.

### Wave 3 — Asset production

- EXPORT-01: reliable export recipes and optimization.
- ASSET-01: icon/application asset studio.
- ASSET-02: sprite and texture atlas generator.
- BATCH-01: robust batch processing.
- SVG-01: advanced SVG inspection and optimization.

**Exit:** repeatable asset-production pipelines with manifests and measurable output quality.

### Wave 4 — Design system and interactive design

- DOC-01: formal page/artboard semantics and serialization decision.
- COMP-01: components, variants, and overrides.
- DESIGN-01/02: tokens, libraries, and responsive layout.
- ANIM-01: extend existing animation timeline.
- PROTO-01: interactive prototype model and preview.

**Exit:** users can author a multi-screen responsive design system and demonstrate an interactive prototype without flattening source content.

### Wave 5 — Developer and automation ecosystem

- DEV-01/02: handoff inspector and framework-neutral design specification.
- AUTO-01/02/03: robust versioned automation contract and CLI/MCP expansion.
- PLUG-01/02: plugin SDK, sandbox, and catalog.
- PLATFORM-02/03: OS integrations, SDK, packaging, and release evidence.

**Exit:** designers and developers can use FireCraft in repeatable workflows, and external tools can integrate through documented contracts.

### Wave 6 — Open community and advanced intelligence

- COMMUNITY-01 through COMMUNITY-07.
- COLLAB-01/02.
- AI-01/02.
- Advanced integrations and optional hosted services.

Some community work can begin earlier: issue templates, roadmap hygiene, and contributor documentation should be maintained continuously. The in-app client should be built after its backend and privacy design are settled.

**Exit:** a sustainable open-source ecosystem with useful in-app participation, safe extensibility, optional collaboration, and optional AI.

## 6.3 Dependency graph

Key dependencies:
- REL and IO foundations precede broad format expansion, collaboration, and AI editing.
- Color profiles and typography precede dependable print, export, and developer handoff.
- Resource management supports brushes, styles, components, and shared design systems.
- Page/artboard semantics precede responsive design and prototype flow modeling.
- Components/variants precede component-level prototype states.
- Animation timeline and prototype model should remain distinct but interoperable.
- A versioned command schema and stable object identity precede broad plugin/MCP automation.
- Permission scopes and document transactions precede write-capable external agents.
- Stable IDs and merge semantics precede real-time collaboration.
- GitHub API/auth and privacy decisions precede direct issue submission from the application.
- Format manifests and export recipes precede a trustworthy icon/sprite/batch production suite.

---

# 7. Cross-cutting engineering requirements

Every workstream must satisfy the repository's existing engineering rules in [AGENTS.md](../AGENTS.md), plus the following feature-specific requirements.

## 7.1 Preserve the architecture

- Extend the existing document model and command registry.
- Avoid duplicate implementations in UI, CLI, scripts, and MCP.
- Add new crates only when the existing layering justifies them.
- Register new workspace crates in the enforced layer map in the same change.
- Keep lower-level crates suitable for WASM unless a documented target-specific feature gate is approved.
- Maintain a deterministic CPU/reference path for tests when GPU output is nondeterministic.
- Treat UI as a client of the engine, not a second document authority.

## 7.2 Document format evolution

Before changing native serialization:
- Write an architecture decision.
- Define the semantics and versioning strategy.
- Preserve unknown fields/nodes where safe.
- Define downgrade/forward-compatibility behavior.
- Add real-world and synthetic fixtures.
- Add migration tests.
- Verify open/save/reopen and rendering.
- Avoid encoding modern component/artboard semantics into legacy nodes whose meaning differs.

## 7.3 Color and rendering correctness

- Keep color-space/profile metadata explicit.
- Do not confuse profile assignment with conversion.
- Define alpha/premultiplication behavior.
- Test CPU/GPU consistency.
- Test web, desktop, and export color paths.
- Pin and document color-management library dependencies and license/target implications.

## 7.4 Error handling and security

- No production panics, unchecked input-derived allocations, or unbounded work.
- Treat external files, SVG, PSD, fonts, plugins, scripts, IPC, MCP, and web content as untrusted.
- Sandbox plugins and scripts according to their actual capability.
- Use typed errors and stable machine-readable error codes.
- Redact sensitive diagnostics.
- Never upload user art or documents without explicit opt-in.
- Do not include credentials or personal tokens in source, logs, support bundles, or public issues.

## 7.5 Performance and quality evidence

Use:
- Unit and integration tests.
- Property-based tests.
- Fuzz anchors for hostile-input surfaces.
- Golden-image tests.
- Semantic document round-trip tests.
- Cross-format interoperability fixtures.
- Browser tests for web UI and bridge changes.
- Benchmarks with repeatable corpus and hardware metadata.
- Manual target-native testing for OS integrations, color, pen input, and assistive technologies.

Record:
- Source SHA.
- Build profile and toolchain.
- OS/hardware.
- Test command.
- Fixture/corpus version.
- Measured result.
- Known limitation.

---

# 8. Definition of done

A work package is complete only when all applicable criteria are satisfied.

1. **Scope:** user problem, desired behavior, non-goals, and compatibility impact are documented.
2. **Architecture:** design decision exists for document-model, serialization, public API, dependency, or security changes.
3. **Implementation:** real functionality exists; no TODOs, placeholders, mocked success, or unsupported claims remain in shipped paths.
4. **UI:** the operation is discoverable, usable, keyboard-operable, and has complete empty/loading/error/disabled states where relevant.
5. **Consistency:** GUI, CLI, MCP, scripting, and export surfaces agree when they represent the same operation.
6. **Data integrity:** save/reopen, undo/redo, migration, and failure behavior are tested.
7. **Compatibility:** format behavior and fidelity limitations are documented.
8. **Security/privacy:** input validation, sandboxing, consent, and redaction are reviewed.
9. **Performance:** costs are measured for large/representative inputs.
10. **Testing:** relevant focused tests, full repository gate, and target-specific browser/platform checks pass.
11. **Documentation:** user guide, command reference, format matrix, and release notes are updated.
12. **Evidence:** results identify the commit/build and environment.
13. **Task tracking:** ticket and live ledger are updated; dependencies and follow-ups are explicit.
14. **Review:** maintainers approve the change; no feature is marked complete merely because an agent reports it.

## 8.1 Required quality gates

Follow current repository instructions and the active CI contract. At the current baseline, the repository documents:

- `cargo run -p xtask -- ci` as the full standard gate.
- `cargo run -p xtask -- ci --full` for the additional fuzz compile check where available.
- `cargo run -p xtask -- wasm` or the relevant CI portion for WASM-sensitive work.
- `cargo run -p xtask -- corpus` for format/corpus changes.
- `cargo run -p xtask -- reachability` for command-to-shell reachability changes.
- `cargo run -p xtask -- dist --web` plus a real browser smoke for web/shell/bridge changes.
- The project-specific release and platform procedures in [TOOLCHAINS.md](TOOLCHAINS.md), [RELEASE-ENGINEERING-PLAN.md](RELEASE-ENGINEERING-PLAN.md), and [RELEASING.md](RELEASING.md).

The exact command set may evolve. Re-read [AGENTS.md](../AGENTS.md) and the current CI definitions before running commands. Do not override repository safety rules or use a broader test command that the repository explicitly disallows.

---

# 9. Risks and decision gates

| Risk | Mitigation / decision required |
|---|---|
| Scope expansion delays MVP | Finish local work and stabilize; turn strategic work into small prioritized tickets. |
| Duplicate functionality already exists locally | Re-audit local and merged code before creating implementation tasks. |
| Document model becomes overloaded | Define page/artboard/state/component semantics in an ADR before serialization changes. |
| CMYK support is superficial | Design a profile-aware color pipeline and validate actual print exports. |
| Format support is overstated | Maintain per-format capability/fidelity matrix and round-trip fixtures. |
| Font behavior differs by OS/browser | Isolate font discovery and shaping behind platform-aware services; test scripts and fallback. |
| Semantic search adds latency/dependency | Keep exact/fuzzy search first-class; semantic ranking optional and local/offline-capable. |
| UI complexity overwhelms Fireworks workflow | Use focused workspaces and progressive disclosure; preserve a fast graphics workspace. |
| Plugins create attack surface | Versioned API, sandbox, explicit permissions, package integrity and safe mode. |
| MCP agents damage documents | Read-only default, scoped roots, preview, confirmation, transactions, undo, audit. |
| Community feature duplicates GitHub | GitHub remains canonical backend initially; FireCraft is a client. |
| User diagnostics leak private work | Inspectable report, redaction, opt-in attachments, no document upload by default. |
| Collaboration creates merge/data loss | Stable object identity and explicit conflict model before real-time editing. |
| AI introduces privacy/quality problems | Optional provider adapters, consent, preview, reversible commands, local option. |
| Web/WASM dependency breaks | Preserve pure-Rust/WASM constraints or isolate target-specific features and fallbacks. |
| Cross-platform claim outruns testing | Require native build/install/launch/smoke receipts for each declared platform. |

Decision documents should be created for consequential choices, especially:
- Color-management model and dependency.
- Document model for artboards/components/variants.
- Format capability/fidelity contract.
- Plugin runtime and permissions.
- Command/API schema versioning.
- MCP write-scope and transaction model.
- GitHub authentication and issue-submission path.
- Collaboration merge model.
- AI provider/local-model boundary.

---

# 10. Metrics and product health

Track a small set of metrics that measure user outcomes, not vanity feature counts.

| Metric | What it demonstrates |
|---|---|
| Data-loss incidents | Whether persistence/recovery is trustworthy |
| Crash-free editing sessions | Application stability |
| Open/save success on corpus | Document and format reliability |
| Round-trip fidelity | Preservation of editable content |
| Large-document latency and memory | Predictable scale |
| Export correctness and reproducibility | Production trust |
| Command discovery success | Whether users can find features |
| Keyboard coverage | Productivity and accessibility |
| Accessibility audit completion | Inclusive usability |
| Platform install/launch/smoke receipts | Honest cross-platform support |
| Plugin API compatibility | Ecosystem stability |
| Public site task completion, accessibility, performance, and download integrity | Product discovery and launch quality |
| Issue triage and resolution time | Community responsiveness |
| Duplicate issue rate | Quality of in-app issue search |
| Contribution onboarding success | Open-source project health |
| CLI/MCP contract stability | Agent and automation reliability |

Metrics should be opt-in or derived from local test/benchmark results by default. Do not add covert product telemetry. Any remote telemetry requires a separate privacy decision, disclosure, and opt-in.

---

# 11. Release strategy and scope discipline

Use staged, independently useful releases rather than one large feature-completion milestone.

Suggested release groupings:
- **Post-MVP reliability release:** data integrity, performance, compatibility reporting, and UX discoverability.
- **Professional creative release:** typography, resource management, color management, print, and export.
- **Asset production release:** icon studio, atlas generator, batch pipelines, and SVG laboratory.
- **Modern design release:** artboards, design systems, responsive layouts, prototypes, and animation.
- **Developer ecosystem release:** handoff, public APIs, CLI/MCP depth, and plugins.
- **Community and intelligence release:** integrated community feedback, collaboration, and optional AI.

These are planning groupings, not version promises. Release scope is decided by the owner and the live task board, with the repo's existing evidence-driven release rules.

## 11.1 Do not gate useful foundational features on speculative services

Typography, color management, local editing, document health, batch export, and CLI workflows should work without user accounts or hosted services. Collaboration and remote AI may be optional extras.

## 11.2 Maintain backward compatibility

- Preserve existing .fw.png documents.
- Version any new document semantics.
- Keep migration/rollback options.
- Avoid requiring a network service to access local files.
- Document the compatibility window for CLI, MCP, plugin, and host APIs.
- Preserve old behavior or provide an explicit migration where changing command semantics.

## 11.3 Do not use feature counts as a proxy for maturity

A new panel, command, format extension, or AI tool is not a success metric by itself. Measure the end-to-end user workflow and the correctness of its output.

---

# 12. Sources of truth and related documents

Use these documents together:

- [AGENTS.md](../AGENTS.md) — engineering rules, clean-room boundary, CI, and agent workflow.
- [README.md](../README.md) — current public product description and entry points.
- [docs/README.md](README.md) — documentation map.
- [TASKS.md](TASKS.md) — canonical active task ledger.
- [NEXT.md](NEXT.md) — active phase roadmap and acceptance criteria.
- [CS6-PARITY.md](CS6-PARITY.md) — current user-visible Fireworks parity inventory.
- [FEATURES.md](FEATURES.md) — implemented feature surface and verification gates.
- [GAPS.md](GAPS.md) — historical gap audit; not a live status page.
- [PROGRESS.md](PROGRESS.md) — dated progress snapshot and evidence.
- [PLANNING.md](PLANNING.md) — architecture, document format, and original product strategy.
- [HANDOFF.md](HANDOFF.md) — current operational handoff.
- [PRODUCTION-READINESS-PLAN.md](PRODUCTION-READINESS-PLAN.md) — production reliability and engineering cleanup.
- [RELEASE-ENGINEERING-PLAN.md](RELEASE-ENGINEERING-PLAN.md) and [TOOLCHAINS.md](TOOLCHAINS.md) — release/platform evidence.
- [decision 0067](decisions/0067-wcag-aaa-web-host.md) — documented accessibility limitations of the current web canvas.
- [decision 0006](decisions/0006-psd-fidelity.md) — PSD compatibility contract and known fidelity limits.
- [MARKETING-SITE-ROADMAP.md](MARKETING-SITE-ROADMAP.md) — website content, UX, technical architecture, Cloudflare Pages deployment, verification, and launch gates.
- [research/creative-app-benchmark.md](research/creative-app-benchmark.md) — feature-transfer research for Krita, Photopea, and Pixlr, with clean-room and licensing boundaries.

**Authority rule:** current source, tests, and generated evidence outrank dated documentation. TASKS/NEXT remain the execution authority for active work. This post-MVP plan is a strategic backlog until the owner approves a concrete task sequence.

---

# 13. Final product definition

FireCraft is successful when a user can move from a blank canvas to a complete, editable creative deliverable in one coherent environment:

1. Create or import vector artwork, photography, and typography.
2. Organize it into pages and artboards.
3. Reuse content through components, variants, styles, and design tokens.
4. Build responsive layouts and interactive prototypes.
5. Inspect accessibility, fonts, colors, profiles, and document health.
6. Generate optimized icons, sprite atlases, images, and production packages.
7. Hand off precise assets, tokens, measurements, and design specifications to developers.
8. Automate the workflow from the UI, CLI, scripting, MCP, and plugins.
9. Preserve the editable source document and its history.
10. Share feedback, contribute improvements, and keep using the application offline without vendor lock-in.

The ambition is not a bigger copy of a legacy application. It is a dependable, open, extensible creative production platform that preserves Fireworks' best ideas and makes them useful in modern workflows.
