# FireCraft product and UX roadmap

**Status:** proposed, evidence-led plan. This is a product plan, not a release promise. Live implementation status belongs in [TASKS.md](TASKS.md), verified availability in [FEATURES.md](FEATURES.md), and parity evidence in [CS6-PARITY.md](CS6-PARITY.md).

FireCraft should keep the fast, keyboard-driven hybrid raster/vector workflow that made Fireworks useful while improving reliability, discoverability, portability, automation and interoperability. The goal is a familiar creative workspace with modern, optional capabilities—not an attempt to reproduce every feature of several competing products.

## Product principles

1. **Preserve the Fireworks feel:** direct canvas editing; compact tools; pages, layers, states, symbols, slices and export in one workflow; predictable menus and shortcuts.
2. **Keep one document authority:** native desktop, WebAssembly, CLI, scripting, MCP and extensions should route document changes through shared engine commands.
3. **Make actions discoverable:** commands need readable names, descriptions, aliases, shortcuts, state/requirements and useful Help links.
4. **Keep edits reversible:** previews, undo, recovery and clear overwrite/loss warnings for destructive actions and conversions.
5. **Be honest about status:** distinguish implemented, partial, experimental and planned behavior with observable evidence.
6. **Local-first by default:** ordinary editing must work offline; network, AI, shell and external submission actions are explicit.
7. **Clean-room compatibility:** learn from public behavior and documentation only; do not copy proprietary code/assets or reproduce unsafe legacy defects.

## Phased plan

### Phase 0 — reconcile and establish evidence (P0)

- Preserve active worktrees, branches, stashes and agent-owned experiment changes; record their owner, base, dirty paths and relationship to open PRs.
- Reconcile each open PR's head against the local branch, review comments, CI failures, conflict state and duplicated scope before selecting changes to integrate.
- Establish a menu/command/panel census, a short list of representative workflows, baseline startup/render/memory measurements and platform build receipts.
- Convert claims into tickets with reproduction steps, acceptance criteria and an evidence owner. Keep historical metrics labeled as historical.

**Exit:** task board agrees with source and PR state; no active work is overwritten; baseline and acceptance paths are recorded.

### Phase 1 — finish the reviewable Windows shell (P0)

- Integrate the experiment's useful layout, tool and panel behavior only after line-level review; resolve compile/runtime/CI findings before calling it merged.
- Match CS6's compact tool grouping, panel title-bar collapse, selection-aware Properties, consistent icons, typography, spacing and resize behavior.
- Make floating canvases independent, draggable windows with reliable drag/drop and focus restoration. Fix Tab canvas-focus toggling and verify clipboard copy/paste across native and web contexts.
- Make the splash a real borderless floating startup surface, with no dark application frame behind it, useful scale, no fake progress, and a short predictable display.
- Refine native file dialogs: clear format filters, extension behavior, overwrite confirmation, recent locations, import placement and export options. Record platform-specific constraints.
- Build a Windows x86_64 review binary from a named commit; record digest and build environment. Ask the reviewer to launch it on Windows 11 and report open/edit/save/export results before release claims.

**Exit:** focused native workflows work; Windows artifact is reproducible and identified; browser smoke passes for shared shell changes; platform failures have owners.

### Phase 2 — Help, command discovery and feedback (P1)

- Make Help a useful hub: keyboard map, feature/status inventory, tutorials, roadmap, release notes, feedback and issue reporting.
- Expand the command palette from ID-only subsequence matching to names, descriptions, aliases in user language, category, shortcut and contextual requirements. Support actions such as “make text bold”, “export transparent PNG” and “show layers”; preserve exact command targeting and stable ranking.
- Add readable command descriptions and empty-state suggestions; recent/favorite commands and keyboard-only operation; hide or disable impossible actions with a reason where safe.
- Provide in-app bug/feature/change reporting via GitHub issue templates: choose type, fill version/platform automatically, preview what will be sent, and open the external tracker. Never submit silently.
- Show current version, release notes and update availability; provide a simple link to verified downloads. Any future updater must verify signed or otherwise authenticated metadata, explain restart/rollback and preserve user documents.

**Exit:** Help links work, every palette result is understandable, issue report flow is reviewable, and release/update information is versioned.

### Phase 3 — daily editing completeness (P1)

- Validate menus, options, shortcuts and panel actions against documented CS6 behavior; record each difference in the parity matrix with status and reason.
- Improve styles and layers: inspect/edit named styles, apply/redefine/scale safely, move/reorder/group layers, explain unsupported blend/effect cases, and ensure undo/history semantics.
- Add a sample design page/document with guided exercises for vector, raster, text, symbols, layers, slices and web export. Include attribution and a reset path.
- Refine dialog consistency, file type selectors, color/font controls, focus order, accessibility labels and keyboard navigation.
- Improve icon family consistency and sizing; audit density and contrast at supported UI scales and HiDPI settings.

**Exit:** key beginner and daily workflows have reproducible end-to-end evidence, parity differences are explicit, and sample content ships with usage guidance.

### Phase 4 — recovery, settings and performance (P1)

- Audit autosave and recovery as implemented today. Set a 15-minute default only if it fits existing persistence, privacy and data-loss behavior; make interval, location, retention and disable controls understandable.
- On startup, offer recovery only for valid newer snapshots; show document name/time and let users restore, discard or open a copy. Never silently replace the last saved file.
- Choose sensible defaults for workspace layout, autosave, update checks, recent files, shortcuts and export behavior; persist with schema versioning and migration.
- Benchmark startup, large file open/save, command latency, canvas frame time, memory and WebAssembly bundle. Optimize measured bottlenecks and keep regression budgets.
- Establish a reproducible web-delivery size report for raw WASM, gzip/Brotli transfer bytes, generated JavaScript, total initial download, and deferred feature chunks. Keep raw-byte and compressed-transfer budgets distinct; measure startup and feature-use latency as well as size.
- Split web code only where the host can genuinely defer loading it. Keep core editing, document opening and recovery in the initial module; evaluate optional import/export codecs, tutorials, automation and advanced panels as lazy modules. A separate file does not reduce first-load bytes if it is eagerly fetched.
- Keep Rust release optimization, LTO, symbol/debug stripping and Binaryen optimization evidence-based. Do not obfuscate source identifiers: that does not materially shorten compiled Rust/WASM and makes diagnostics worse. Minify generated JavaScript and use Brotli/gzip at delivery where the host supports the matching content-encoding contract.
- Design native optional features as separately versioned packages or processes only where measurements justify the install/runtime cost. Avoid Rust dynamic-library plugins as a public ABI; prefer a versioned capability-based extension protocol, with WebAssembly sandboxing as a cross-platform option.

**Exit:** recovery scenarios pass fault-injection/manual checks, settings migrate safely, and performance changes have before/after evidence.

### Phase 5 — scripting and intelligence workspace (P2)

- First deliver an Automation Console for existing scripts/commands: bounded output, errors with line context, cancellation, timings, safe document inspection and undo-aware mutations.
- Design an optional Intelligence panel for provider/model configuration, skills, scoped memory, scheduled jobs, run history and per-action approvals. Keep secrets out of documents/logs and allow offline-only use.
- Treat an integrated system terminal as a separate opt-in panel with explicit working directory, process lifecycle, permission boundaries and no implied access to document scripting. Reuse the bottom Properties group only if space and lifecycle behavior remain clear.
- Evolve scripting into a versioned extension SDK: namespaced preferences, documented file dialogs, font/format metadata, command registration, capability declarations and migration rules. Preserve stable module boundaries; do not transplant legacy AMD/RequireJS infrastructure.
- Make the extension SDK load optional, signed/versioned packages through explicit capabilities and compatibility checks. Specify lifecycle, permissions, resource limits, error isolation, update/rollback, and native/WASM parity before shipping third-party extensions.
- Add observed `.jsf` compatibility fixtures for quirks documented by fwjs-errata only where safe and relevant; reject or correct behaviors that corrupt documents or crash.

**Exit:** capabilities and data access are explicit; console operations are bounded and reversible; extension APIs are documented and versioned.

### Phase 6 — release and public project quality (P1)

- Maintain a public feature list, roadmap, tutorials, known limitations and changelog with links from README and in-app Help.
- Produce platform binaries only with build, launch and basic edit/export receipts for the actual OS; distinguish available recipes from tested artifacts.
- Reconcile GitHub issues, pull requests, branches and experiment work only after ownership and divergence are known. Resolve conflicts in isolated branches and keep provenance.
- Deploy the Cloudflare marketing page and WebAssembly application only from a named, validated commit; record asset digests and deployment identity.
- Refresh wiki pages and contributor instructions from current source; remove stale claims and preserve historical records as historical.
- Add a native taskbar/Dock launcher menu. Share one privacy-aware recent/pinned document model, but use platform-specific adapters: Windows Jump Lists, macOS Dock menu, and Linux desktop actions plus the desktop's recent-document integration. Make app-wide tasks available while closed, and route document selections through the same open/dirty-state flow as the GUI.

**Exit:** release artifacts and web deployment map to source revisions; docs match evidence; repository cleanup does not discard active work.

### Phase 6.1 — taskbar and launcher integration (P1)

- Make **File → Open Recent** real first, with most-recent-first ordering, a clear-list action, missing-file handling and persistence. Add a pin/favorite list with explicit pin/unpin and stable order; keep recent and pinned distinct.
- Define one platform-neutral launcher model: New, Open, recent documents, pinned documents, currently open documents where supported, Settings/Help, and Quit. Keep the core app command IDs and launch arguments stable.
- **Windows:** use the desktop Jump List API and AppUserModelID. Populate recent/custom destinations and static tasks (New/Open); honor destinations removed by the user and gracefully skip missing files. Let Windows own pinning semantics for Jump List destinations. Validate packaged and unpackaged distribution behavior on Windows 11.
- **macOS:** provide a Dock menu using AppKit's Dock-menu hook, with useful current/recent documents and app-wide actions. This needs a native application delegate integration point; verify bundle, event-loop and activation behavior on macOS.
- **Linux:** install desktop-entry actions for stable app-wide commands and add opened documents to the desktop recent-document store. Check behavior under GNOME, KDE and Wayland/X11 environments; desktop implementations differ, and not all expose a custom dynamic right-click menu.
- Pass document paths safely through OS launch arguments; handle spaces, Unicode, duplicate launches, cold start, single-instance activation, deleted files and dirty current documents.
- Do not make notification-area tray support part of the taskbar feature. Keep an optional system-tray icon/menu as a separate follow-up with its own lifecycle and accessibility review.

**Exit:** each supported desktop environment has a manual receipt covering menu visibility, New/Open, recent/pinned semantics, launch while app is closed/open, missing documents, removal/pin behavior, and interaction with unsaved edits. Unsupported launchers fail gracefully and remain fully usable through the app's File menu.

### Phase 7 — selectively broaden creative capability (P2/P3)

- Study Krita, Photopea, Pixlr, Graphite and BonBon for brush dynamics, resource management, non-destructive graph workflows, mobile interactions and raster workflows; prototype in FireCraft's shared architecture and evaluate licensing before any reuse. Keep Graphite's layer-first/procedural hybrid optional and limited to a measured operation family rather than replacing FireCraft's document model ([Graphite benchmark](research/graphite-product-benchmark.md)).
- Define color management and high-bit-depth/HDR interchange before adding Lightroom-like correction; document gamut, precision and export behavior.
- Explore frame/timeline editing and CorridorKey 2.0 only after a standalone integration and file-format decision.
- Consider mobile support based on a focused interaction prototype and real device acceptance, not desktop UI scaling alone.

**Exit:** every expansion has a user need, architecture fit, licensing review, file compatibility plan and measurable acceptance criteria.

## Reference insights captured

- [BonBon](research/bonbon-product-benchmark.md): shortcut migration, tooltips, paste-at-pointer and Linux release packaging.
- [Emberframe](research/emberframe-product-benchmark.md): honest “not in this version” scope, sample documents and export comparison.
- [Fireworks command library](research/fireworks-command-library-benchmark.md): workflow-oriented command names and batch export ideas.
- [fwlib](research/fwlib-extension-benchmark.md): metadata, layer helpers, preferences and extension SDK seams.
- [fwconsole](research/fwconsole-benchmark.md): editor-integrated script diagnostics, distinct from a system terminal.
- [fwrequirejs](research/fwrequirejs-benchmark.md): stable extension module boundaries and isolation, without adopting obsolete loader code.
- [fwjs-errata](research/fwjs-errata-compatibility.md): observable compatibility fixtures with unsafe legacy defects excluded.
- [UIWORK FireworksExtensions](research/uiwork-fireworks-extensions-benchmark.md): small task-specific commands and persistent panels as distinct extension shapes.
- [Taskbar and launcher menu research](research/taskbar-context-menus.md): Windows Jump Lists, macOS Dock menus and Linux desktop actions/recent documents.
- [WASM and extension packaging plan](tasks/T31-modular-packaging-and-wasm-size.md): initial-load budgeting, truly deferred modules, delivery compression, and a safe cross-platform extension boundary.

## Task conversion and status

The live board in [TASKS.md](TASKS.md) is authoritative for owners and state. Each phase should be split into small tickets with prerequisites, affected surfaces, non-goals, acceptance steps, test/browser/platform requirements, and documentation updates. Do not label an item complete because it is described here or because a branch/PR exists.
