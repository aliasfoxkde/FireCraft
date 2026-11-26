# FireCraft public roadmap

**Tagline:** FireCraft — Open Source Raster and Vector Graphics Editor.

This page summarizes the product direction for users and contributors. It is not a release promise. Work is considered shipped only when it is in a published release; work under development is tracked in [TASKS.md](TASKS.md), and the detailed product plan is [POST-MVP-ROADMAP.md](POST-MVP-ROADMAP.md).

## Shipped and available

- Editable Fireworks PNG documents, with preservation of unknown document data.
- Vector and bitmap editing, text, layers, pages, states, symbols, masks, live effects and web export workflows.
- Native desktop, WebAssembly, CLI, scripting, automation and MCP surfaces sharing one document engine and command registry.
- CS6-oriented menus, tool groups, dockable panels, canvas editing, command palette, shortcut sets, preferences and recovery features. See the [feature list](FEATURES.md), [shortcut reference](SHORTCUTS.md) and [CS6 parity matrix](CS6-PARITY.md) for measured detail.
- Current published release and platform evidence are listed in [release status](PROGRESS.md) and [GitHub Releases](https://github.com/aliasfoxkde/FireCraft/releases).

## In progress

- Finish integrating and validating the CraftAnything experiment-derived shell changes, including platform builds, menus, dialogs, panel behavior, focus, clipboard, splash behavior and browser interaction.
- Reconcile the open production-readiness and experiment merge lines; resolve review findings and required CI before merging.
- Improve command discovery with searchable descriptions and user-language terms, and make Help a reliable entry point for shortcuts, feature status and tutorials.
- Expand hands-on tutorials, sample documents and contribution paths for bug reports and feature requests.
- Improve document dialogs, style/layer workflows, visual consistency, pop-out canvas interaction, application update guidance and release notes.
- Add useful right-click actions to Windows/macOS app launchers and supported Linux taskbars, including recent and pinned document access where the platform supports it ([T30](tasks/T30-taskbar-launcher-menu.md)).
- Continue cross-platform release certification and deployment reconciliation for the desktop binaries, marketing site and WebAssembly app.
- Measure and reduce WebAssembly initial-download cost; split optional features only when they can load on demand. Define a secure, versioned extension package model for browser and native hosts ([T31](tasks/T31-modular-packaging-and-wasm-size.md)).

## Planned product work

### Editing and design

- Stronger brush, typography, color-management and high-bit-depth workflows.
- More capable styles, layers, masks, bitmap inspection, selection and non-destructive correction.
- Sample design documents that teach core vector, raster, layout and export workflows.
- Mobile-capable interaction and responsive workspace behavior, subject to platform and usability validation.

### Workflow and reliability

- Clear in-app update and changelog flows backed by signed/provenanced release metadata.
- Configurable, user-visible autosave cadence and recovery controls; current recovery behavior and platform limitations are documented in [Features](FEATURES.md).
- An Automation Console for running scripts and inspecting structured output, errors and document state, built on the existing scripting/command boundaries.
- A documented scripting compatibility envelope with regression fixtures for legacy JavaScript and Fireworks DOM expectations. Compatibility should preserve safe script behavior without reproducing crashes or data-corruption bugs.
- An optional system terminal and an Intelligence workspace for connected AI tools, skills, memory and scheduled jobs. Keep OS-shell execution separate from document scripting; both need clear permissions, data boundaries and safe reversible actions.
- Direct in-app bug, feature and change-request submission using the repository's issue templates, with a privacy-aware preview before opening the external tracker.

### Broader creative workflows

- Research and selectively adapt documented interaction patterns from Krita, Photopea, Pixlr and Graphite; keep FireCraft's current/roadmap status distinct and do not copy code or assets ([research synthesis](research/creative-app-benchmark.md), [Graphite assessment](research/graphite-product-benchmark.md)).
- Video, frame and timeline editing, including a possible CorridorKey 2.0 integration after a separate compatibility and architecture decision.
- Lightroom-like color correction and HDR-capable image workflows, contingent on a defined color pipeline and supported interchange formats.

## How to contribute feedback

Use [GitHub Issues](https://github.com/aliasfoxkde/FireCraft/issues/new/choose) for bugs and feature requests. Include the FireCraft version, operating system, steps to reproduce, expected and observed behavior, and a sample file only when you have the right to share it. For general questions, start with the [Wiki](wiki/Home.md). The Help menu links to the feature list, tutorials and shortcut reference.

## Planning and status links

- [Feature list](FEATURES.md): implemented behavior and evidence.
- [CS6 parity matrix](CS6-PARITY.md): behavior-by-behavior comparison and limits.
- [Live task board](TASKS.md): current task status and owners.
- [Detailed post-MVP plan](POST-MVP-ROADMAP.md): larger specifications, priorities and acceptance criteria.
- [Release status](PROGRESS.md): dated verification and publication evidence.
