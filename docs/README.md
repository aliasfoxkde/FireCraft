# FireCraft documentation

This page is the documentation map. Start with the overview that matches your goal, then follow its links into the detailed reference.

## Choose a path

### I want to use FireCraft

1. [Usage](USAGE.md): install/build, CLI examples, keyboard shortcuts, and browser embedding.
2. [Keyboard shortcuts](SHORTCUTS.md): the published binding table, generated from the code.
3. [Features](FEATURES.md): implemented surfaces and their verification gates.
4. [Public roadmap](PUBLIC-ROADMAP.md): shipped capabilities and planned work in user-facing language.
5. [Product and UX roadmap](PRODUCT-UX-ROADMAP.md): phased plan honoring Fireworks workflows while adding modern features and release criteria.
6. [WASM delivery and extension package plan](tasks/T31-modular-packaging-and-wasm-size.md): measured bundle optimization, genuinely lazy features, compression and a cross-platform extension boundary.
7. [Tutorials](wiki/Tutorials.md): practical editor workflows.
8. [Release engineering](RELEASE-ENGINEERING-PLAN.md): standalone platform matrix, artifact evidence, SDK distribution and shared CI adoption.
9. [WebOS embedding](WEBOS.md): current host page contract and browser bridge.
10. [Platform integration](WEBOS-PLATFORM-INTEGRATION.md): future versioned SDK, command/capability contract and WebOS interoperability.

### I want to understand project status

1. [Progress](PROGRESS.md): current verified snapshot and release/build evidence.
2. [Task board](TASKS.md): the single current list of active, queued, blocked, and completed tasks.
3. [Next](NEXT.md): detailed phased roadmap and acceptance criteria.
4. [CS6 parity matrix](CS6-PARITY.md): the user-visible inventory — every CS6 workflow and UI area, classified with evidence.
5. [Published Wiki](https://github.com/aliasfoxkde/FireCraft/wiki): concise user and contributor navigation; the source is [`wiki/Home.md`](wiki/Home.md).

### I want to understand the post-MVP product direction

1. [Post-MVP Product and Engineering Roadmap](POST-MVP-ROADMAP.md): detailed product vision, feature specifications, dependencies, priorities, architecture constraints, and acceptance criteria.
2. [Product and UX Roadmap](PRODUCT-UX-ROADMAP.md): phased product plan centered on Fireworks CS6 workflows, modern UX and verifiable delivery.
3. [WASM delivery and extension packages](tasks/T31-modular-packaging-and-wasm-size.md): modular delivery and plugin architecture with measurable size and startup criteria.
4. [Post-MVP Agent Handoff](POST-MVP-AGENT-HANDOFF.md): repository audit, local-work reconciliation, task conversion, and execution rules.
5. [Marketing Site Roadmap](MARKETING-SITE-ROADMAP.md): site content, responsive UX, release downloads, privacy, SEO, accessibility, deployment and launch gates.
6. [Post-MVP Baseline Audit](POST-MVP-BASELINE-AUDIT.md): source-backed status of roadmap workstreams and unreconciled local work.
7. [Post-MVP Execution Plan](POST-MVP-EXECUTION-PLAN.md): merge-safe gates, ticket rosters, verification and handoff.

### I want to research product direction

1. [Creative-app benchmark](research/creative-app-benchmark.md): cross-product synthesis from Krita, Photopea, Pixlr and Graphite.
2. [Graphite product benchmark](research/graphite-product-benchmark.md): current alpha capabilities, hybrid node/layer workflow, transferable patterns and FireCraft fit.
3. [BonBon / Libreworks benchmark](research/bonbon-product-benchmark.md): current interaction and feature patterns from the Linux-focused C++ editor.
4. [Fireworks command library benchmark](research/fireworks-command-library-benchmark.md): user-workflow and command-discovery ideas from a Fireworks support repository.
5. [Emberframe benchmark](research/emberframe-product-benchmark.md): sample-document onboarding, export comparison and candid scope communication.
6. [fwlib benchmark](research/fwlib-extension-benchmark.md): document metadata, layer APIs, preferences and extension SDK ideas.
7. [Fireworks Console benchmark](research/fwconsole-benchmark.md): an in-app script diagnostic console, distinct from an OS terminal.
8. [Fireworks JS errata review](research/fwjs-errata-compatibility.md): a clean-room `.jsf` compatibility test matrix based on observed host quirks.
9. [fwrequirejs benchmark](research/fwrequirejs-extension-modules.md): module boundaries, dependency isolation and extension versioning.
10. [UIWORK FireworksExtensions benchmark](research/uiwork-fireworks-extensions-benchmark.md): task-focused commands and persistent panel patterns.
11. [Taskbar/launcher menu research](research/taskbar-context-menus.md): Windows Jump Lists, macOS Dock menus and Linux launcher limits.

### I want to contribute or use an AI agent

1. [`../AGENTS.md`](../AGENTS.md): repository rules, clean-room boundary, commands, and workflow.
2. [Handoff](HANDOFF.md): short orientation and session closeout procedure.
3. [Architecture and format plan](PLANNING.md): technical design and file format detail.
4. [Decisions](decisions/): durable design choices; [research](research/): observed format behavior.

## Reference shelves

| Level | Documents | Use them for |
|---|---|---|
| Overview | [README](../README.md), this page, [Wiki](wiki/Home.md) | Project purpose and navigation |
| User guide | [USAGE](USAGE.md), [SHORTCUTS](SHORTCUTS.md), [FEATURES](FEATURES.md), [WEBOS](WEBOS.md) | Operating the app and understanding its surface |
| Current state | [PROGRESS](PROGRESS.md), [TASKS](TASKS.md), [NEXT](NEXT.md), [CS6-PARITY](CS6-PARITY.md) | What is true now and what comes next |
| Engineering | [HANDOFF](HANDOFF.md), [PLANNING](PLANNING.md), [TOOLCHAINS](TOOLCHAINS.md), [RELEASING](RELEASING.md) | Building, architecture, and release work |
| Evidence | [decisions](decisions/), [research](research/), [generated parity and scorecard](planning/generated/) | Specific technical claims and their basis |
| Product direction | [Product/UX roadmap](PRODUCT-UX-ROADMAP.md), [Post-MVP roadmap](POST-MVP-ROADMAP.md), [baseline audit](POST-MVP-BASELINE-AUDIT.md), [execution plan](POST-MVP-EXECUTION-PLAN.md), [agent handoff](POST-MVP-AGENT-HANDOFF.md), [marketing site roadmap](MARKETING-SITE-ROADMAP.md) | Strategic scope, verified per-workstream status, public website plan, and the gated ticket sequence; not live task status |
| Research | [creative app benchmark](research/creative-app-benchmark.md), [Graphite benchmark](research/graphite-product-benchmark.md) | Feature-transfer research and licensing constraints for external creative-app references |
| History | [SESSIONS](SESSIONS.md), [HISTORY](HISTORY.md), [closed milestones](planning/firecraft/), [release notes](releases/) | How the project arrived here; not live task status |

## Source-of-truth rules

- `docs/PROGRESS.md` is the dated status snapshot.
- `docs/TASKS.md` is the live task ledger. Detailed acceptance criteria belong in `docs/tasks/` and `docs/NEXT.md`.
- `docs/SESSIONS.md` and `docs/HISTORY.md` are historical. Do not infer current work from an old entry.
- `docs/planning/generated/` is generated output; use its generator rather than editing derived counts by hand.
- `source/` and Adobe-derived local extracts are reference-only and must never be committed.
