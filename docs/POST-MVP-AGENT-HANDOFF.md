# FireCraft — Post-MVP Agent Handoff and Execution Contract

> **Purpose:** instructions for the AI/coding agent that begins post-MVP planning after the current FireCraft development work is complete.
> **Read first:** [AGENTS.md](../AGENTS.md), [TASKS.md](TASKS.md), [NEXT.md](NEXT.md), [HANDOFF.md](HANDOFF.md), [PROGRESS.md](PROGRESS.md), [CS6-PARITY.md](CS6-PARITY.md), [FEATURES.md](FEATURES.md), and [POST-MVP-ROADMAP.md](POST-MVP-ROADMAP.md), [MARKETING-SITE-ROADMAP.md](MARKETING-SITE-ROADMAP.md), and [research/creative-app-benchmark.md](research/creative-app-benchmark.md).
> **This is not authorization to start implementation immediately.** The first deliverable is a verified baseline and a deduplicated task plan.

---

## 1. Mission

Convert the strategic scope in [POST-MVP-ROADMAP.md](POST-MVP-ROADMAP.md) into an evidence-backed, prioritized, implementation-ready backlog. Include the independent public-site workstream and the creative-app benchmark, but audit the current state before creating tickets.

The project owner has stated that substantial UI/UX and feature work is still local/in progress. Do not assume the public main branch contains all current work. Do not discard, overwrite, reset, clean, stash, or replace any local changes to make the repository match a remote snapshot.

**Primary goal:** identify what already exists, what is incomplete, what is only available through the engine/CLI/MCP but not through a polished GUI workflow, what is still in progress locally, and what is genuinely new work.

## 2. Mandatory preflight

Before making code changes or creating implementation tasks:

1. Confirm repository root and current branch.
2. Run git status and inspect all staged, unstaged, untracked, and ignored-but-relevant local work. Do not expose or commit secrets, local source/reference installs, generated binaries, or private artifacts.
3. Inspect local-only branches, remotes, tracking configuration, and worktrees.
4. Inspect all open PRs, their review/CI state, and likely merge order.
5. Read the current untracked session log at log/devlog.md if present.
6. Read the current AGENTS.md and verify its command, branch, commit, push, remote, and test rules.
7. Read TASKS.md, NEXT.md, HANDOFF.md, PROGRESS.md, CS6-PARITY.md, FEATURES.md, PLANNING.md, PRODUCTION-READINESS-PLAN.md, and the current relevant task tickets.
8. Compare current source and generated evidence to every relevant status statement. Treat dated counts as historical until regenerated.
9. Identify whether any work described in the strategic plan is already implemented in local branches or uncommitted files.
10. Inspect the existing site/ source, root wrangler.toml, README marketing links, docs/HANDOFF.md deployment recipe, Pages project topology, and currently deployed URLs/revisions. Confirm whether firecraft-site exists and how it deploys; do not assume dashboard state from configuration alone.
11. Preserve current work and report conflicts before touching shared files.

**Stop condition:** if current work is uncommitted or integration is unstable, do not begin a competing feature branch or edit shared implementation files. Produce the audit and proposed merge-safe sequence first.

## 3. Required baseline audit deliverable

Create a dated baseline report under docs, using the repository's existing documentation conventions. Suggested name: docs/POST-MVP-BASELINE-AUDIT.md.

For every workstream ID in the roadmap, assign exactly one current status:

- **VERIFIED IMPLEMENTED** — implementation and relevant tests/evidence exist in the current integrated tree.
- **IMPLEMENTED, UX INCOMPLETE** — engine/command/API exists but the user-facing workflow is incomplete or undiscoverable.
- **PARTIAL** — a real but incomplete implementation exists.
- **LOCAL / NOT MERGED** — implementation exists in local work but is not part of the selected integration baseline.
- **IN PROGRESS** — active work is currently being implemented.
- **MISSING** — no substantive implementation exists after searching source, docs, branches, and issues.
- **BLOCKED** — a dependency, architecture decision, or platform capability prevents safe implementation.
- **DEFERRED** — intentionally outside the approved near-term scope.

For each status record:
- Evidence: file paths, symbols, tests, UI route, command ID, docs, PR, or branch.
- Exact limitation or remaining behavior.
- Whether the functionality works in desktop, web/WASM, CLI, scripting, and MCP.
- Relevant issue/task/decision IDs.
- Dependencies and owners.
- Recommended next action.

Do not classify a feature as complete because a command ID exists. Verify a real workflow at the appropriate surface.

For SITE-01, audit the existing static site and its live deployment as separate evidence sources. Verify every feature/platform/format claim against a current build; verify screenshots against their source build; verify download URLs against published release assets; and verify the Pages project, deployment mode, production branch, domain, output directory, and deployed SHA. Mark unknown dashboard or DNS state UNVERIFIED rather than inferring it from wrangler.toml.

### 3.1 Mandatory audit categories

At minimum, audit:

- Core editor and document model.
- Fireworks PNG read/write and migration.
- Vector editing and geometry operations.
- Raster editing, selections, masks, filters, and retouching.
- Pages, states, symbols, and nine-slice behavior.
- Current artboard/component/variant semantics.
- Animation timeline and current state model.
- Import/export and format-fidelity matrix.
- Color models, ICC handling, CMYK, and print output.
- Fonts, typography, text layout, and font discovery.
- Brushes, styles, swatches, patterns, and resource management.
- Existing command palette, fuzzy search, aliases, recents, and saved commands.
- Help, F1, shortcut viewer, keymap sets, and rebinding.
- Autosave, recovery, undo/redo, save integrity, and external modification.
- Performance, large documents, memory behavior, CPU/GPU parity.
- Accessibility on desktop and browser.
- CLI, scripting, browser bridge, and MCP.
- Batch processing, export presets, icon generation, and sprite workflows.
- Plugin architecture and API stability.
- Community reporting, GitHub integration, and public roadmap.
- Collaboration, version history, and AI-assisted editing.
- Platform builds, installers, WebOS integration, and target-native evidence.
- Public marketing website, its existing static implementation, content/claim accuracy, asset provenance, accessibility/SEO, release downloads, and separate Cloudflare Pages deployment.

### 3.2 Specific repository evidence rules

- Use source and tests as the implementation authority.
- Use generated parity/scorecard output only when regenerated from the same source SHA.
- Use CS6-PARITY.md for user-visible parity, not DOM verb counts.
- Use FEATURES.md as an inventory, not proof of every GUI workflow.
- Use GAPS.md as historical research, not a live work list.
- Use TASKS.md and NEXT.md for current active work.
- Check local unmerged work before creating new tickets.
- Do not duplicate existing GitHub Issues; search before creating any.
- Do not rewrite historical progress to imply that a later implementation existed earlier.
- If evidence is unavailable, mark the status **UNVERIFIED** in the audit narrative and explain what must be measured rather than guessing.

## 4. Reconcile the existing local development before planning new work

The owner explicitly says substantial UI/UX and feature work is still local. This takes precedence over any stale remote documentation snapshot.

Produce a merge/integration inventory:
- Branch and worktree.
- Intended objective.
- Files/modules affected.
- Tests already run and exact result.
- Dependencies on other branches.
- Conflicts with main or other local branches.
- Whether it duplicates an existing issue/task.
- Recommended merge order.
- Remaining acceptance criteria.

Do not merge local work simply because it appears related. Verify that the implementation uses the existing document/engine/command architecture and does not introduce duplicated implementations or regressions.

After integration:
- Refresh TASKS.md, NEXT.md, CS6-PARITY.md, FEATURES.md, and PROGRESS.md only where the current rules require it.
- Re-run generated parity/scorecard/reachability/corpus evidence as applicable.
- Preserve the history and evidence of completed milestones.

## 5. Convert the strategic roadmap into executable tickets

Do not dump the entire roadmap into TASKS.md as one giant task. Create small task tickets using the established format and conventions under docs/tasks/ and docs/planning/firecraft/.

Every ticket must include:

1. Stable task ID.
2. Title and user problem.
3. Priority and why.
4. Status and dependencies.
5. Scope and explicit non-goals.
6. Current implementation/evidence audit.
7. Files/crates/surfaces likely affected.
8. Architecture/API/document-format decisions required.
9. Implementation steps small enough to review.
10. Acceptance criteria written as observable behaviors.
11. Automated test plan.
12. Manual/browser/platform test plan where applicable.
13. Performance and memory considerations.
14. Security/privacy considerations.
15. WASM and cross-platform impact.
16. Documentation and migration impact.
17. Risks, rollback, and stopping condition.
18. Evidence required to close the task.

Use the workstream IDs in POST-MVP-ROADMAP.md as parent identifiers. For creative-app benchmark ideas, map Krita/Photopea/Pixlr patterns to existing source and tests before proposing new functionality; follow the license and clean-room boundaries in research/creative-app-benchmark.md. For the public site, use SITE-01 and MARKETING-SITE-ROADMAP.md. Search existing issues and task tickets before creating any child ticket. Create a limited number of tickets per iteration; do not pre-author detailed implementation tickets for every hypothetical future feature before dependencies are understood.

## 6. Recommended sequencing

Use this order unless the audit provides evidence for a better one:

### Gate A — Current work complete and merged

- Integrate the current local UI/UX/feature work.
- Validate merge order and branch cleanup.
- Establish a clean, tested MVP baseline.
- Reconcile current docs and generated evidence.

### Track M — Public marketing site (separate deployment)

After Gate A has stabilized the integrated editor baseline, audit and improve the existing site/ application as a separately owned static-site track. The remote snapshot already contains site/index.html, site/styles.css, site/app.js, site/assets/, and a root Wrangler configuration naming firecraft-site; do not scaffold a second site.

Required order:
1. Audit current local site work, current site content, screenshots, brand asset provenance, README links, and all site-related branches/PRs.
2. Verify Cloudflare project topology and deployment mode. Keep firecraft-site (marketing) separate from firecraft-web (editor output dist/web). Never repoint one project at the other's output.
3. Produce a feature-claim evidence matrix and screenshot refresh plan before rewriting copy.
4. Create/reconcile SITE-01 tickets for content, responsive design, gallery, downloads, SEO, accessibility, privacy, and deployment.
5. Implement in small site-only slices. Preserve the static architecture unless a measured requirement justifies a framework.
6. Run site-specific automated and manual browser checks. A Rust CI pass is not a substitute for site/browser testing.
7. Deploy preview and obtain review. Production deployment requires explicit owner approval, a verified project/domain, and a deployment receipt.
8. Add the verified site deployment procedure to docs/HANDOFF.md and update public links only after production verification.

Keep this track independent of editor/WASM release work. Parallelize only after file ownership and Pages project boundaries are clear.

### Gate B — Reliability and usability

- Data integrity, recovery, and large-document performance.
- Format support matrix and compatibility warnings.
- Help Center, command discovery, and complete keybinding customization.
- Accessibility audit and platform readiness.

### Gate C — Professional creative foundation

- Color management and print.
- Font/typography management.
- Brushes, styles, swatches, patterns, and resource manager.
- Targeted vector/raster editing gaps.

### Gate D — Asset production

- Export pipeline.
- Icon/application asset studio.
- Sprite/texture atlas generator.
- Batch graphics production.
- SVG laboratory.

### Gate E — Modern design platform

- Document-model decision for pages/artboards/states/components.
- Components, variants, design tokens, and libraries.
- Responsive layout.
- Animation and interactive prototyping.

### Gate F — Developer and automation ecosystem

- Developer handoff.
- Versioned CLI/command contract.
- MCP capability and security expansion.
- Plugin SDK and sandbox.

### Gate G — Community and advanced features

- In-app Community Center and GitHub integration.
- Issue templates, intelligent duplicate search, public status.
- Local version history and visual diff.
- Collaboration and AI only after architecture, identity, permission, and privacy decisions.

This is a dependency-aware default, not a date-based commitment. Parallelize only where module ownership and API boundaries are stable.

## 7. Non-negotiable implementation rules

- Read and follow AGENTS.md.
- Preserve clean-room policy and all attribution requirements.
- Do not commit Adobe binaries, assets, proprietary source, or local reference installs.
- No placeholders, mock success, dead-end controls, or untracked TODOs in shipped paths.
- No duplicate implementations of an existing engine command or subsystem.
- No production panic/unwrap/expect/unimplemented/unreachable or unsafe code.
- Do not add a dependency without checking license, MSRV, native and WASM behavior, compile time, binary size, and security posture.
- Preserve the WASM build; isolate native-only dependencies behind explicit features and provide a documented fallback when appropriate.
- Extend the central command registry rather than creating separate GUI/CLI/MCP semantics.
- Keep the native document model and serialization backward-compatible or provide a tested migration.
- Treat imported files, SVG, PSD, fonts, plugins, scripts, IPC, MCP, and remote responses as untrusted.
- Keep local editing, opening, saving, and exporting independent of network services.
- Do not add telemetry or upload user documents without an explicit privacy decision and opt-in.
- Do not send user documents to remote AI providers without explicit consent.
- Use transaction/undo/preview for bulk, destructive, or AI/agent-driven modifications.
- Keep GitHub as the canonical issue backend initially; do not introduce a parallel tracker without a written decision.
- Do not mark a platform supported without target-native build/install/launch/smoke evidence.

## 8. Testing and verification contract

Re-read AGENTS.md before selecting commands. At the current documented baseline, the common gate is:

~~~sh
cargo run -p xtask -- ci
~~~

Use focused crate/test commands during development and the full gate before finalizing. For relevant work, include:
- Corpus and round-trip verification for document/file-format changes.
- Fuzz anchors and hostile-input tests for parsers and external input.
- Golden-image and CPU/GPU comparison for rendering/color changes.
- Reachability and command-registry checks for new commands.
- Real browser testing against the served optimized dist/web bundle for web/shell/bridge changes.
- Native platform tests for font discovery, color profiles, pen/tablet input, file dialogs, clipboard, and screen readers.
- Benchmarks for performance claims.
- CLI/MCP contract tests and permission tests for automation.
- Plugin sandbox and malicious-package tests for extensibility.
- Privacy/redaction tests for diagnostics and issue submission.
- Assistive-technology tests with OS, tool version, and workflow receipts.
- For site/ changes: validate static output, metadata, links, sitemap/robots, release API fallback and URL allowlisting; run Playwright at desktop/tablet/mobile viewports; check keyboard, focus, reduced motion, text scaling, console/network errors, and axe/manual accessibility; record Lighthouse results. Deploy and verify a Pages preview independently from the editor build.

Never treat a green compile as proof that a workflow works.

## 9. Required handoff when the plan is ready for implementation

Deliver:

1. **POST-MVP-BASELINE-AUDIT.md** — source-backed implementation matrix and local-work reconciliation.
2. **Prioritized ticket list** — parent workstream, dependencies, and acceptance gates.
3. **Updated task board proposal** — avoid editing the live ledger until scope is owner-approved.
4. **Architecture decisions** — only for real consequential choices discovered during audit.
5. **Verification plan** — exact commands and manual evidence by workstream.
6. **Risk register** — blockers, unresolved decisions, and platform constraints.
7. **Next-agent handoff** — exact first ticket, files owned, required tests, and stopping condition.

Before beginning implementation, provide the owner with a concise summary:
- What is already implemented.
- What local work remains to merge.
- What is incomplete or missing.
- What should be done first and why.
- What work is blocked by architectural decisions.
- What scope is explicitly deferred.

## 10. Stop conditions

Stop and report rather than guessing when:
- The current working tree contains uncommitted changes you cannot safely attribute.
- Another agent is modifying the same files or document model.
- The repository's canonical task board conflicts with current source/evidence.
- A proposed feature would require an incompatible document-format change without an approved migration plan.
- A new dependency breaks or materially threatens WASM/native compatibility without an approved alternative.
- A format's semantics are unknown and implementation would risk silent data loss.
- A user-facing claim cannot be tested on the target platform.
- An issue appears to duplicate an existing one but the relationship is uncertain.
- A community integration would require storing privileged tokens in the client.
- The actual marketing Pages project, deployment mode, domain, or production source revision cannot be verified.
- Production-site deployment or DNS changes would require owner approval or account access the agent does not have.
- A plugin or AI integration cannot be adequately permissioned and sandboxed.

A partial audit with clear evidence and blockers is better than a large speculative implementation.

## 11. Definition of a successful post-MVP handoff

The handoff is successful when another agent can select the first approved ticket and implement it without:
- Re-discovering the repository architecture.
- Repeating work already present locally or on another branch.
- Guessing about current feature status.
- Duplicating commands or subsystems.
- Making unreviewed document-format assumptions.
- Weakening current quality gates.
- Claiming unsupported file/platform compatibility.
- Losing user data or privacy boundaries.

The strategic roadmap defines the direction. The audit defines the truth. The task tickets define the work. The evidence defines completion.
