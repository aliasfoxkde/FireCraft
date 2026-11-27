# FireCraft Product Expansion Roadmap

> **Status:** strategic proposal for owner review. Planned features are not represented as implemented or committed to a release date.
> **Canonical repository:** https://github.com/aliasfoxkde/FireCraft
> **Execution authority:** [TASKS.md](../TASKS.md), [NEXT.md](../NEXT.md), and approved task tickets. This is strategy, not a second live task board.
> **Related:** [1.0 Consolidation Handoff](FIRECRAFT-1.0-CONSOLIDATION-AND-RELEASE-HANDOFF.md), [Marketing Site Roadmap](../MARKETING-SITE-ROADMAP.md), [Post-MVP Roadmap](../POST-MVP-ROADMAP.md).

---

## 1. Product thesis

FireCraft should become an open, cross-platform creative production environment that preserves the best of the Fireworks workflow: fast hybrid vector/bitmap composition, editable source documents, practical web/asset output, and a productive integrated workspace.

Its differentiators are:
- **Hybrid editing:** vector objects, raster content, text, effects, layers, pages/states, and export in one editable document.
- **Fast workflow:** discoverable tools, keyboard-first operation, coherent panels, useful defaults, and low-friction common tasks.
- **Document ownership:** local-first files, predictable save/recovery, transparent format support, no mandatory account or hosted service.
- **Cross-platform access:** native desktop and browser/WASM, with an honest support matrix.
- **Production output:** repeatable export, asset organization, batch workflows, and developer handoff.
- **Open ecosystem:** source availability, automation, documented contracts, community feedback, and eventually extensions.
- **Trust:** privacy, accessibility, reliability, clear limitations, reproducible releases, and responsive maintenance.

FireCraft need not become a complete replacement for Photoshop, Illustrator, Krita, Figma, or a desktop publishing suite. Borrow patterns only when they strengthen the core promise.

## 2. Release horizons

These are planning horizons, not calendar promises. Promote work to the live task board only after validating dependencies, evidence, cost, and owner priority.

### Horizon A — 1.0: dependable, approachable, launchable

A new user can discover FireCraft, understand its status, install or try it, create/import/edit/save/reopen/export a real project, get help, and report a problem.

1.0 includes:
- The polished existing `site/` marketing landing page deployed to its own Cloudflare Pages project.
- Verified download and browser-app entry points, authentic screenshots, accurate availability labels, getting-started content, and contributor/support links.
- In-app feedback with user-controlled diagnostics.
- Release provenance, checksums, platform evidence, recovery/data-safety tests, and clear limitations.
- A maintainable docs hierarchy, contribution guide, security/privacy contact, and community triage process.

### Horizon B — 1.x: deepen everyday creative workflows

Prioritize improvements that increase daily usefulness without changing product identity:
1. **Selection/transforms:** robust multi-select, numeric transforms, alignment/distribution, constraints, transform origins, snapping, guides, rulers, reusable selection sets.
2. **Vector authoring:** Bézier ergonomics, node types, path cleanup, boolean operations, stroke alignment/expansion, compound paths, editable effects.
3. **Raster workflow:** crop/resize, selection refinement, masks, brush/retouch ergonomics, non-destructive adjustments, layer compositing, blend modes, sampling.
4. **Typography:** font discovery/fallback, substitution warnings, stable text layout, character/paragraph controls, text-on-path where feasible, licensing policy.
5. **Document organization:** pages/states, artboards if they fit the model, symbols/components, libraries, templates, styles, naming/search/outline.
6. **Export:** presets, batch export, slices/assets, CSS/SVG/PNG/WebP, optimization preview, deterministic output, export queue and errors.
7. **Workspace:** panel presets, command palette, contextual help, shortcut customization, preferences, onboarding samples, recent files, recovery browser.
8. **Interchange:** public PSD/SVG/raster capability matrix, round-trip corpus, unsupported-feature warnings, migration tools, loss reports.
9. **Performance:** large-document profiling, incremental rendering, tile/cache policy, memory limits, background export, responsive input.
10. **Accessibility/localization:** keyboard-complete workflows, screen-reader semantics where feasible, contrast/focus audits, string extraction and translation.

Each feature needs a real user workflow, model/command design, regression tests, documentation, and a verified support label. Avoid adding panels that merely expose incomplete engine commands.

### Horizon C — 1.x/2.0: asset and production toolkit

- Icon/sprite authoring, atlas packing, density variants, naming conventions, export manifests.
- Asset slicing and batch export with deterministic naming and reproducible settings.
- SVG inspection/cleanup and controlled optimization with editable-source preservation.
- Design tokens for color, typography, spacing, effects, reusable styles, import/export.
- Component/symbol libraries with reuse, variants, overrides, migration.
- Templates for web, app, icon, and social formats.
- Asset validation for dimensions, profiles, transparency, naming, file-size budgets, and completeness.
- Developer handoff: measurements, CSS variables/tokens, bundles, previews, machine-readable manifests.
- Optional CLI pipelines using the same engine/export semantics as the GUI.

Do not turn FireCraft into a website builder by default. Keep handoff interoperable with external tools.

### Horizon D — 2.x: advanced design and professional workflows

Candidate directions gated by demand and technical validation:
- ICC profiles, working/display/export profiles, soft proofing, explicit unsupported-profile behavior.
- Advanced typography and text layout.
- Advanced masks, adjustment layers, non-destructive filters/effect stacks.
- Animation/timeline and export improvements where complete workflows are viable.
- Artboards, responsive variants, layout constraints, design-system integration.
- Advanced print/PDF only after color, font, and page-layout requirements are specified.
- More complete PSD/SVG/interchange fidelity with a public compatibility matrix.
- Large-document/high-DPI performance, GPU fallback quality, memory diagnostics.

These are not 1.0 gates unless evidence shows a workflow is essential to MVP.

### Horizon E — ecosystem and optional intelligence

- Stable plugin API with versioning, permission scopes, signing/integrity, compatibility checks, safe mode.
- Scripting SDK, CLI, MCP, browser bridge against one versioned command/capability contract.
- Local automation recipes and user-installed extensions.
- Optional AI-assisted selection, cleanup, vectorization, naming, layout suggestions, batch operations.
- Optional local-model/provider adapters; explicit consent before content leaves the device.
- Agent workflows through previewable, reversible commands and document transactions.
- Community template/preset/extension catalogue with moderation and supply-chain controls.

AI/plugins must never be required for ordinary editing. Default to local processing, least privilege, preview-before-apply, undo, and an audit trail.

### Horizon F — collaboration and hosted services, only if justified

Potential future capabilities: shareable review links/comments, version history, conflict-aware sync, real-time collaboration after stable identity/transactions/conflict semantics, team libraries, and optional hosted project spaces. Do not add mandatory accounts or cloud document storage to the local-first editor. Hosted collaboration requires a separate privacy, security, and operating-cost review.

## 3. Cross-cutting product requirements

### 3.1 Document safety
- Atomic saves where supported; clear handling of disk-full, permission-denied, and interrupted writes.
- Autosave/crash recovery with explicit recovery choices and no silent overwrite.
- Backup/version snapshots and tested retention/recovery.
- Schema migration and backward-compatibility tests.
- Round-trip tests for supported formats; preserve unsupported content where possible or warn before destructive save.
- Never silently flatten editable content during save.
- Input limits, fuzzing, decompression limits, and no panic on malformed files.

### 3.2 Usability and accessibility
- Consistent icons, terminology, spacing, focus states, tooltips, hover/pressed/disabled states, and keyboard navigation.
- Every visible control works, explains itself, or is explicitly disabled/unavailable.
- Skippable onboarding that never blocks local editing.
- Contextual help and recoverable empty/error states.
- Keyboard-only completion of core workflows; measure screen-reader/high-contrast support honestly.
- Review screenshot/golden changes rather than blindly updating baselines.

### 3.3 Performance and reliability
Benchmark representative fixtures for cold start, open/save, first paint, canvas navigation and zoom, selection/transform latency, export, memory, and WASM bundle size. Set target-specific budgets. Profile before optimizing. Handle GPU initialization failure with a viable fallback. Avoid blocking UI on large imports/exports; define progress and cancellation.

### 3.4 Compatibility and portability
Maintain a format matrix by feature, direction, fidelity, and evidence. Maintain a platform/browser matrix by artifact, OS/browser version, architecture, install, launch, smoke, signing, and certification. Compile success is not install/launch evidence. Document browser storage quotas, file APIs, downloads, lifecycle, and recovery.

### 3.5 Security and supply chain
Threat-model untrusted document/image/SVG import, archive/decompression, plugins, updates, and feedback intake. Review dependencies/advisories/licenses; generate SBOM/provenance; sign artifacts where feasible. Protect scoped CI, signing, and Cloudflare secrets. Publish vulnerability-reporting instructions. Plugin execution requires isolation and permissions.

### 3.6 Community operations
Add CONTRIBUTING, Code of Conduct, support boundaries, security reporting, issue templates/labels, maintainer ownership, release support policy, known-good sample files, and contributor onboarding. Define supported versions and EOL policy. Keep sensitive diagnostics out of public issues unless explicitly approved.

### 3.7 Typography, color, localization
Document font fallback/substitution and font licensing; define ICC/profile behavior and export fidelity. Extract UI strings and plan localization only with a maintainable review workflow.

## 4. Website as a first-class product surface

The marketing site is part of 1.0, not optional polish. See [MARKETING-SITE-ROADMAP.md](../MARKETING-SITE-ROADMAP.md). It must answer: what is FireCraft, what works now, what is different, can I try/download, how are files handled, and where can I get help/contribute?

Required journeys: home → try/download; home → feature evidence/limitations; download → correct artifact/checksum/install notes; new user → quick start/sample; support → feedback/issues; contributor → source/license/contribution/roadmap.

Keep `site/` and its Pages project separate from WASM `dist/web` and the editor Pages project. Use separate workflows, domains, environments, and rollback paths.

## 5. Feedback and product discovery

Treat feedback as discovery, not an agent command channel:
- Bug reports: repro steps, expected/actual, app/build/platform, optional sanitized attachments.
- Feature requests: problem, workaround, target workflow, frequency/impact, optional mockup; no implied commitment.
- Duplicate search and related reports before creating an issue.
- GitHub Issues remains canonical initially.
- Triage impact, reproducibility, workflow, platform, security/privacy risk, and cost.
- Agent fixes reproduce the issue, add regression coverage, pass PR review and staging.
- Reporter validation where practical; never expose private attachments publicly without explicit confirmation.
- Close the loop in release notes.

Measure reproducibility, triage/resolution age, duplicates, regressions, and reporter validation—not just submission count.

## 6. Prioritization model

Score user impact/frequency, core-product fit, workflow completeness, reliability/data risk, implementation plus maintenance cost, dependency leverage, and evidence (issues, interviews, observed workflows, benchmarks). Foundational capabilities outrank flashy breadth. Save integrity, predictable selection, export correctness, and onboarding outrank an incomplete panel.

## 7. Explicit non-goals unless separately approved

Do not make 1.0 depend on accounts, cloud document storage, subscriptions, hosted collaboration, AI, model downloads, telemetry, plugin marketplace, full parity with other creative suites, perfect fidelity for every format, mobile-native redesign, CMS/blog infrastructure, or a second engine/shell/design system.

## 8. Turning strategy into executable work

1. Audit source and active worktrees before estimating.
2. Map features to existing crates, commands, panels, tests, docs, and gaps.
3. Reuse current tickets and decisions; avoid duplicates.
4. Create bounded tickets with owner, dependencies, acceptance, tests, docs, and explicit exclusions.
5. Add approved work to `TASKS.md` and `NEXT.md`; do not bulk-import speculative ideas.
6. Publish feature claims only after verification in a published build.
7. Close work with test receipts, screenshots where relevant, docs, release notes, and handoff.
8. Revisit priorities using actual post-launch community evidence.

## 9. Product health dashboard

Track crash-free sessions/data-loss incidents; save/reopen and format round-trip success; core workflow regressions; startup/open/save/canvas/export/memory/bundle budgets; platform install/launch/smoke evidence; accessibility/keyboard coverage; issue reproducibility/triage/fix/duplicate/reporter-validation; website journey conversion only with approved privacy-preserving measurement; and artifact provenance/signature/checksum/rollback readiness.

No covert product telemetry. Remote measurement requires a separate decision, disclosure, and explicit opt-in. The app must remain useful with telemetry disabled.
