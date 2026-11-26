# FireCraft 1.0 — Consolidation, MVP, Community Feedback, and Release Handoff

> **Document type:** executable consolidation and release plan for the FireCraft 1.0 MVP.
> **Status:** proposed plan; no implementation status is implied by this document.
> **Canonical repository:** https://github.com/aliasfoxkde/FireCraft
> **Primary branch:** `main`
> **Companion strategy:** [Post-MVP Product and Engineering Roadmap](POST-MVP-ROADMAP.md)
> **Existing authorities:** [AGENTS.md](../AGENTS.md), [TASKS.md](TASKS.md), [NEXT.md](NEXT.md), [HANDOFF.md](HANDOFF.md), [CS6-PARITY.md](CS6-PARITY.md), [FEATURES.md](FEATURES.md), [PROGRESS.md](PROGRESS.md), [PRODUCTION-READINESS-PLAN.md](PRODUCTION-READINESS-PLAN.md), [POST-MVP-AGENT-HANDOFF.md](POST-MVP-AGENT-HANDOFF.md), and [MARKETING-SITE-ROADMAP.md](MARKETING-SITE-ROADMAP.md).
> **Prepared against remote snapshot:** `main` at `4ff8f786e365c8b7d7609e435424c9c746e61d6d`, 2026-11-26.
> **Owner context:** the project owner reports that the CraftAnything experiment is currently the better-feeling UI baseline and that an agent is already working in a newer branch/worktree on icons, look-and-feel, polish, and consolidation. That in-flight work is not visible in this remote snapshot and MUST be discovered and preserved before integration.

---

## 0. Executive direction

The goal is to turn the current substantial but fragmented development state into **one coherent, installable, understandable, supportable FireCraft product**, with one canonical repository and one authoritative release process.

FireCraft should preserve the promise that attracted its initial GitHub audience: a modern, open-source, cross-platform continuation of the productive Adobe Fireworks-style hybrid vector/bitmap workflow. It should not attempt to become every graphics application at once.

**FireCraft 1.0 means a dependable first product, not complete Fireworks CS6 parity.** Target the most important 80% of everyday workflows, make those workflows discoverable and reliable, document the remaining gaps honestly, and establish a rapid feedback-to-fix loop.

The 1.0 effort has six outcomes:

1. **One source of truth:** all useful work from the experiment, integration branches, open PRs, and current agent work is reconciled into the primary `FireCraft` repository. No second product repository is required for ongoing development.
2. **One coherent GUI:** the strongest current experiment UI is retained, polished, visually consistent, accessible, and regression-tested. Do not replace the current in-flight UI work with an older remote snapshot.
3. **A credible MVP:** core create/edit/save/reopen/export workflows are stable on explicitly supported targets.
4. **In-app community intake:** users can report bugs and request features/change from inside FireCraft, with useful environment metadata and optional screenshots/logs, with explicit privacy consent.
5. **A controlled improvement loop:** reports become triaged GitHub issues, then agent-owned branches/PRs, verified staging builds, opt-in cohort testing, and controlled production releases.
6. **A maintainable project:** clean task ledger, focused modules, understandable docs, reliable CI, release provenance, and no abandoned branch/worktree sprawl.

This plan is the immediate MVP execution plan. The broader product direction remains in `POST-MVP-ROADMAP.md`; do not pull its entire long-term scope into 1.0.

## 1. Non-negotiable execution rules

- Read `AGENTS.md` and obey its current repository-specific rules. This document supplements it; it does not override it.
- **Preserve the active local work.** First inspect all worktrees, branches, uncommitted/staged/untracked changes, stashes, local commits, agent sessions, and the current devlog/session handoff. Never run destructive cleanup, reset, checkout-overwrite, stash-pop, force-push, or branch deletion until ownership and contents are understood.
- Do not assume `main` is the newest or best UI. The owner explicitly identifies the active experiment/newest worktree as the current visual reference.
- Do not merge PRs by title or insertion order. Review actual file-level diffs, ancestry, tests, CI, and conflicts.
- Do not merge overlapping PRs twice or cherry-pick the same work twice.
- Do not silently drop changes merely because they are duplicated, inconvenient, or difficult to reconcile. Record every retained, superseded, or intentionally discarded change with evidence and rationale.
- Keep diffs scoped. Separate integration, GUI polish, community intake, release engineering, and unrelated long-term features into distinct PRs where possible.
- Never mark a feature complete because it compiles or has a command ID. Verify the user-visible workflow.
- Never advertise an OS/browser target as supported solely because a build script or CI compile succeeds.
- Local-first editing remains available without account creation, network access, telemetry, or community-service availability.
- Never embed a maintainer's personal GitHub token, OAuth client secret, Cloudflare secret, signing key, or other credential in the app, website, repository, or release artifact.
- Do not upload user artwork, original document names/paths, screenshots, logs, crash dumps, or machine-identifying metadata without a clear preview and explicit user consent.
- No autonomous agent may merge its own PR or promote a build directly to production. Human approval remains a release gate.

## 2. Remote baseline and immediate integration facts

The remote repository currently reports `main` at `4ff8f786e365c8b7d7609e435424c9c746e61d6d`. The README documents release `v0.2.5` (2026-11-03), a substantial Rust shared-engine architecture, native and WebAssembly frontends, Fireworks PNG document support, vector/bitmap editing, pages/layers/symbols, export, CLI/scripting/MCP, and a CS6-inspired shell-fidelity campaign. The README also explicitly warns that engine/DOM command coverage is not equivalent to complete GUI parity.

### 2.1 Known open integration PRs at plan preparation

| PR | Branch | Scope | Observed state |
|---|---|---|---|
| [#186](https://github.com/aliasfoxkde/FireCraft/pull/186) | `temp/consolidation` | Broad production-readiness consolidation: WASM CDN/glue, experiment integration, shell module extraction, wasm32 fixes, coverage/release engineering/docs | Open; mergeable at inspection; 175 files, +14,837/-1,907 |
| [#187](https://github.com/aliasfoxkde/FireCraft/pull/187) | `feat/experiment-merge` | CraftAnything experiment integration: panel enhancements, infinite canvas, text editing, smart guides, drag/drop, context menus, CS6 chrome, cross-platform CI and Playwright smoke | Open; mergeable at inspection; 58 files, +9,340/-1,432 |

**Critical: #186 and #187 are not a simple linear stack.** Both target `main` at the same base SHA, but their head histories have diverged. A GitHub compare of #187 head `399385f1f97d13d1d715bafec715be553e01a0f2` to #186 head `155ef967e72aa0b677e7d10e256adf6dce45f944` reported `diverged` (one commit unique to #186 and four commits unique to #187). #186's description says it includes experiment improvements, but that statement is not proof that every later #187 change or the current local worktree is included.

Therefore:
- Do not merge #186 and then #187 blindly.
- Do not merge #187 and then #186 blindly.
- Do not close either PR until every unique change has been accounted for.
- Build a file-level and behavior-level reconciliation matrix from the actual PR heads and the active worktree.
- Select one authoritative integration branch/PR after reconciliation. It may be a new integration branch based on the current main, not necessarily either existing branch.
- Prefer the newest, best-tested UI implementation when it is demonstrably better, but preserve unrelated production-readiness fixes from #186.
- If one PR becomes fully superseded, close it with a clear cross-reference only after the authoritative PR contains its unique value and all required checks pass.

### 2.2 Other branches visible remotely at inspection

Remote branches included:

- `audit/parity-hardening-2026-11-22`
- `climb/C4.08-selective-jpeg`
- `climb/C8.04-missing-docs-campaign`
- `feat/experiment-merge`
- `feat/inspector-field-sets`
- `feat/post-mvp-execution`
- `feat/release-provenance`
- `feat/wasm-cdn-deploy`
- `feature/adam7-png-interlace`
- `feature/issue-93-behavior-export`
- `fix/ci-fresh-workspace-crates`
- `fix/wasm-size-budget-repin`
- `fix/web-deploy-p0`
- `fix/xtask-gate-repair-2026-11-24`
- `release/v0.2.5`
- `review/post-mvp-execution`
- `temp/consolidation`

This is a discovery list, not a deletion list. The active agent may have newer branches/worktrees not represented here.

## 3. Phase 0 — Freeze, discover, and establish the real baseline

**Goal:** no work is lost, and every candidate change is understood before integration.

### 0.1 Capture repository state

Run and save a dated audit artifact (suggested `docs/integration/2026-11-26-reconciliation.md`):

```bash
git status --short --branch
git worktree list --porcelain
git branch --all --verbose --no-abbrev
git stash list
git log --graph --decorate --oneline --all -100
git remote -v
gh pr list --state open --limit 100
gh pr status
```

Also inspect:
- Each worktree's branch, HEAD, dirty state, untracked files, and owner/agent purpose.
- Agent session notes, task ledgers, devlogs, and current handoff.
- Local-only commits and branches not pushed to GitHub.
- Tags and release refs.
- CI runs, review comments, unresolved review threads, merge conflicts, and changed-file lists for every open PR.
- The experiment repository's latest state and the exact commit(s) already imported into FireCraft.
- Current Cloudflare Pages project names, build outputs, deploy workflows, and deployed commit IDs; do not infer deployed state from `wrangler.toml` alone.

Create a machine-readable inventory (CSV/JSON is acceptable) with: ref, HEAD SHA, base SHA, owner/worktree, clean/dirty, commits unique to ref, changed files, CI state, unique behavior, overlap, disposition, and evidence.

### 0.2 Reconcile every PR and branch

For each open PR:
1. Read its complete description, changed-file list, full patch, review threads, CI checks, and commit ancestry.
2. Compare its head to current `main` and to the selected integration candidate.
3. Identify unique code, unique tests, unique docs, and overlapping hunks.
4. Run targeted tests for behavior that is not covered by existing CI.
5. Assign one disposition: `INTEGRATE`, `CHERRY-PICK`, `REIMPLEMENT`, `SUPERSEDED`, `DEFER`, or `REJECT`.
6. Record why. Never use `SUPERSEDED` without naming the replacement commit/PR and proving equivalent or better behavior.

For #186/#187 specifically, compare:
- Shell/panel modules and panel inventory.
- Toolbar/icon assets, keymap, color system, menus, dialogs, dock behavior.
- Canvas zoom/pan/infinite canvas and input gestures.
- Inline text editing and cancellation semantics.
- Smart guides, lasso feedback, drag/drop overlay, context menus, quit confirmation.
- WASM glue/CDN and deploy behavior.
- Cross-platform workflows and browser smoke tests.
- Coverage gates, shell module extraction, release provenance, SBOM, audit docs, and task ledger changes.

### 0.3 Protect the active newest worktree

The owner says an agent is currently polishing icons, look/feel, consolidation, and other UX in a newer branch/worktree. Treat that as the likely visual source of truth until inspected.

- Ask the active agent to stop at a clean checkpoint, commit or document its current state, and provide the worktree path, branch, HEAD SHA, dirty diff, and current tests.
- Do not ask the agent to abandon or reset its work.
- Compare its UI against both #186 and #187.
- Preserve the strongest parts of each; do not mechanically prefer the newest commit if it regresses functionality.
- Keep a copy of the worktree/branch ref before any rebase or integration.
- Resolve ownership conflicts before parallel agents edit the same shell files.

**Phase 0 exit:** signed-off baseline report; all local work preserved; every open PR and candidate branch has a disposition; one integration plan with dependency order; no unknown dirty worktree.

## 4. Phase 1 — Consolidate into one FireCraft repository

### 4.1 Repository architecture and ownership

`aliasfoxkde/FireCraft` is the canonical product repository. Move or port useful experiment code, tests, screenshots, docs, and research into it with provenance. The experiment repo becomes a read-only historical reference after all useful material is migrated and verified; do not delete it immediately.

Keep the repository organized around existing crate boundaries and conventions. Do not introduce a second competing application shell, document model, command registry, test framework, or design system. Consolidate duplicate modules only after behavior parity is demonstrated.

Expected ownership:
- `crates/doc`, `crates/engine`, `crates/vector`, `crates/raster`, `crates/compose`, `crates/io`, `crates/fwpng`: canonical document semantics and operations.
- `crates/app`: native/WASM shell and presentation only; split large modules by cohesive responsibility.
- `crates/app-cli`, scripting, browser bridge, MCP: adapters over shared engine commands, not independent implementations.
- `xtask`, scripts, workflows: reproducible quality gates, packaging, deployment, provenance.
- `site/`: marketing website only.
- `docs/`: one canonical roadmap/task/parity/release/architecture authority; avoid stale duplicate planning documents.
- `tests/`, fixtures, screenshot baselines: shared verification corpus.

### 4.2 Integration method

1. Create a protected integration branch from the latest approved `main`.
2. Merge/cherry-pick/reimplement the selected PR changes in small logical groups.
3. Prefer preserving meaningful commit history; use a documented squash only when the original history is noisy or duplicate.
4. After each logical group, run formatting, compile, targeted unit/integration tests, and relevant GUI tests.
5. Keep integration commits bisectable. Avoid one opaque mega-commit if the change can be separated safely.
6. Re-run the full repository gate from a fresh workspace after the integration is complete.
7. Produce a merge report mapping each source PR/branch to resulting commit(s) and final status.
8. Close superseded PRs only after the report and checks are complete.

### 4.3 Branch and worktree cleanup

Cleanup is a final phase, never a prerequisite to understanding the work.

- Classify branches as active, integrated, superseded, release/tag, archived reference, or abandoned.
- Protect `main`, active release branches, and branches with unmerged commits.
- Delete a remote feature branch only after its unique commits are merged or explicitly archived and its owner confirms it is no longer active.
- Preserve tags and release refs.
- For worktrees: verify clean state, branch association, and no running agent; archive required diffs/notes; then remove the worktree and branch together only when safe.
- Do not delete the CraftAnything experiment repository as part of 1.0 cleanup. Mark it archived/read-only only after the owner approves.
- Remove stale local generated files/caches only after checking ignore rules and ensuring they are reproducible.
- Remove duplicate docs and stale screenshots only after finding inbound links and replacing them with canonical references.
- No blanket `git branch -D`, `git worktree prune`, `git clean -xfd`, or force-push sweep.

**Phase 1 exit:** one integrated source tree; one canonical UI; one task ledger; all useful experiment code migrated; branch/worktree inventory reconciled; no unresolved merge conflict; no silent loss of functionality.

## 5. Phase 2 — Define and deliver the FireCraft 1.0 MVP

### 5.1 Product promise

Recommended 1.0 positioning:

> **FireCraft is a free, open-source graphics editor that brings vector and bitmap editing together in one practical, Fireworks-inspired workspace. Create editable graphics, organize work with layers and pages, and export assets—on supported desktop platforms and in the browser.**

Use the phrase “Fireworks-inspired” rather than implying Adobe affiliation or perfect CS6 compatibility. Keep the existing trademark/independence notice. All claims must be tied to the published build and verified workflows.

### 5.2 MVP scope: prioritize complete workflows over feature count

The target is roughly 80% of the common workflows requested by the intended first audience, not 80% of every CS6 menu item. Establish that audience evidence through actual user testing and issue data.

**P0 — must work reliably for 1.0**

1. **Start and discoverability**
   - First-run welcome/open/create workflow.
   - Clear new/open/recent/save/save-as/export actions.
   - Discoverable tools, menus, keyboard shortcuts, command palette, and contextual help.
   - Recoverable errors; no silent no-op controls.
2. **Document lifecycle**
   - Create, open, save, save-as, close, reopen.
   - Correct dirty-state indicators and unsaved-change prompts.
   - Atomic/safe saves, autosave/recovery where supported, and no data loss on failed save.
   - Clear handling of malformed, unsupported, or partially compatible files.
3. **Hybrid canvas editing**
   - Import/place raster images.
   - Create/select/move/resize/rotate vector objects.
   - Bézier path creation and node/handle editing.
   - Text create/edit/format with reliable commit/cancel behavior.
   - Fill, stroke, color, opacity, gradients, alignment, arrangement, and basic transforms.
   - Selection, multi-selection, copy/paste/duplicate, undo/redo.
   - Predictable zoom, pan, fit-to-content, pixel-accurate zoom, guides/grid/snapping.
4. **Organization**
   - Layers with visibility, lock, reorder, rename, and selection behavior.
   - Pages or artboards with dependable navigation and document persistence.
   - Basic reusable symbols where already implemented and tested.
   - Panel docking/visibility/resize/reset and layout persistence; no inaccessible or permanently lost panels.
5. **Output**
   - Reliable PNG and JPEG export with dimensions and quality/transparency controls appropriate to the format.
   - SVG export for supported vector content.
   - Export must not mutate the editable source document.
   - Unsupported/lossy conversion is disclosed before export.
6. **Reliability**
   - Stable launch/close, no common crash/data-corruption path.
   - Useful error messages and logs.
   - Reasonable performance on representative small and medium documents.
   - Keyboard and pointer interactions usable without obvious focus traps.
7. **Support**
   - In-app report bug / request feature / request change.
   - Preview and consent for diagnostic metadata, screenshots, and logs.
   - Offline/copy fallback.
   - Public issue links and clear privacy notice.

**P1 — include if already close and reliable; otherwise ship shortly after 1.0**
- More advanced bitmap selection/retouch tools.
- More sophisticated symbols, states, slices, hotspots, and web optimization.
- Additional import/export formats beyond the reliable core.
- Advanced filters/live effects and animation timeline.
- CSS properties, sprite generation, HTML prototype output.
- Advanced inspector panels, color management, and professional typography controls.

**Defer from 1.0 unless the current integrated implementation is already proven**
- Full Photoshop/Illustrator/Figma feature parity.
- Full collaborative cloud editing, user accounts, cloud document storage, hosted AI, or mandatory telemetry.
- A plugin marketplace, complex extension sandbox, advanced motion design, full responsive web builder, or AI-generated production code.
- Perfect fidelity for undocumented proprietary format behaviors.
- Automatic unattended production rollout by AI.

Do not remove existing working features merely because they are not MVP-critical. Keep them available if stable; simply avoid making them release blockers or central marketing claims.

### 5.3 1.0 usability and visual polish

The owner reports the experiment UI is already close and prefers its overall look/feel. Improve that baseline rather than redesigning from scratch.

Required polish:
- One consistent icon family, stroke weight, optical sizing, alignment, and selected/disabled/hover states. Audit the current icon assets before choosing a library; do not introduce a new icon dependency merely for marketing. If adopting Phosphor, integrate it consistently, preserve license/attribution, and test native/WASM rendering.
- Cohesive color tokens, surfaces, borders, separators, typography, spacing, corner radii, and panel headers.
- Clear active tool, active document, selected object, dirty document, modal, and focus states.
- Predictable menu, toolbar, panel, canvas, context-menu, and keyboard behavior.
- Consistent light/dark themes if both are retained; no half-themed widgets.
- Correct scaling at common desktop DPI/display scale settings.
- Useful empty states and first-run guidance.
- Accessible names for icon-only controls; keyboard navigation and visible focus.
- No dead menu entries, placeholder dialogs, misleading controls, clipped text, or overlapping panels.
- Keep the UI responsive while importing, exporting, or processing large assets.

Do not optimize solely for screenshot similarity. Prioritize task completion, clarity, and interaction consistency. Keep a visual regression suite for stable layouts and separate tests for behavior.

### 5.4 Definition of Done for a feature

A feature is complete only when:
- Its user problem and acceptance criteria are explicit.
- It works through the intended GUI workflow, not just the engine/CLI.
- It uses the shared command/document architecture where applicable.
- Unit tests cover logic and edge cases.
- Integration tests cover persistence and cross-module behavior.
- GUI tests cover the interaction sequence and relevant state.
- Screenshot baselines cover meaningful visual states where useful.
- Native and WASM behavior is tested or the limitation is documented.
- Errors, undo/recovery, accessibility, and privacy implications are reviewed.
- Docs, feature map, parity matrix, task ledger, and release notes are updated.
- No placeholder, unverified claim, or hidden TODO is left in the user-facing path.

## 6. Phase 3 — In-app bug reports and feature/change requests

**Priority: P0 for the community MVP.** This is a core adoption and improvement feature, not a post-launch convenience. It should be small, privacy-safe, and built on GitHub as the canonical tracker. The existing `POST-MVP-ROADMAP.md` COMMUNITY-01 through COMMUNITY-07 sections are the longer-term specification; this section defines the 1.0 implementation slice.

### 6.1 User experience

Add a persistent, discoverable **Help / Feedback / Community** entry in the application menu and/or help panel. Provide:

- Report a bug.
- Request a feature.
- Request a change/improvement.
- Ask a usage question / open discussions.
- View known issues and workarounds.
- View releases and roadmap.
- Copy diagnostic information.
- Open GitHub project/issues.

The form should be short and contextual. Do not force users to fill in every field. Prepopulate environment metadata, then let them inspect, redact, and remove it.

**Bug report flow**
1. Select “Report a bug.”
2. Enter a concise title, what happened, expected behavior, reproduction steps, and frequency/severity where known.
3. Show detected environment/build information.
4. Offer “Capture screenshot” and “Include recent diagnostic log” as separate, unchecked options.
5. Generate a preview of the exact report and every attachment.
6. Let the user redact/remove fields or attachments.
7. Require explicit consent before network submission.
8. Submit to the FireCraft GitHub issue tracker, show the resulting issue URL/number, and offer copy/open actions.
9. If submission fails or the user is offline, save/copy a Markdown report and open the canonical GitHub issue form. Never discard the report.

**Feature/change request flow**
- What problem are you trying to solve?
- What workflow/user is affected?
- What do you do today / what is the workaround?
- What behavior would you expect?
- Why would it help?
- Optional mockup/example/screenshot.
- Optional “I can test this” or “I can contribute.”

Keep feedback separate from bug diagnostics: a feature request should not automatically attach machine metadata or logs.

### 6.2 Metadata schema

Use a versioned, structured diagnostic envelope (for example `firecraft-diagnostic/v1`) with only fields that are useful to reproduce the problem.

| Group | Suggested fields | Privacy rules |
|---|---|---|
| Product | app name, semantic version, build channel, git/build revision, build date, diagnostic schema version | Safe to include in preview |
| Target | native / browser-WASM / embedded host; target triple; release/debug | Safe to include in preview |
| OS | OS family, version, architecture, distro/version where available | Do not collect hostname, account name, machine ID |
| Browser | browser family/version, engine/version, WASM target, relevant feature support | Browser UA may be coarse-grained; show exact values before submit |
| Runtime | Rust/app runtime version if relevant, eframe/egui version, backend | Include only if available and useful |
| Graphics | GPU vendor/model and driver/API when available; renderer/backend; fallback status | Optional; do not fingerprint or enumerate unrelated devices |
| Display | viewport size, scale factor, color depth if relevant | Optional; no screen capture without consent |
| App state | active tool, selected panel/workspace, relevant command/action ID, app uptime, crash/recovery state | Do not include document content |
| Document context | file format, approximate dimensions, object/layer/page counts, file-size bucket, unsaved/recovery state | Never include original path, filename, artwork, text, or file bytes by default |
| Logs | bounded recent app log ring buffer, error IDs, sanitized backtrace | Redact secrets, tokens, environment variables, user paths, URLs with credentials |
| Attachments | screenshot, optional minimal reproduction document, optional crash dump | Unchecked by default; preview, redact/crop, size limit, explicit consent |

Rules:
- Capture only what is available through supported APIs. Unknown values remain unknown; never invent them.
- Do not use invasive fingerprinting or collect unrelated hardware/software inventories.
- Do not collect precise location, contacts, browsing history, installed applications, clipboard, or other unrelated data.
- Do not automatically upload active artwork, source files, filenames, paths, or full memory dumps.
- Screenshots can expose private artwork or personal data. Make them opt-in, previewable, crop/redact-capable, and removable.
- Log capture is opt-in, bounded (for example, last 5–10 minutes or a fixed maximum size), sanitized, and previewable.
- Provide “Copy diagnostics” and “Export report” locally, even when the user declines network submission.
- State retention and public visibility: GitHub issues are public unless a separate private security-reporting path is used. Warn before submission.
- Provide a privacy notice in-app and in the repository describing exactly what is collected and when.

### 6.3 Submission architecture

GitHub Issues remains the canonical source of truth. Do not create a separate issue database.

Preferred low-complexity design:
- The client creates a versioned Markdown/JSON report preview.
- A narrowly scoped Cloudflare Worker or equivalent public submission broker uses a GitHub App installed only on the FireCraft repository, with only the minimum Issues permission required.
- Store the GitHub App private key and installation identifiers as server-side secrets; never ship them to the client.
- The broker validates schema, request size, allowed categories, attachment count/type/size, and rate limits; it applies labels and creates the GitHub issue.
- Use abuse controls appropriate to a public endpoint (rate limiting, origin checks as defense-in-depth, challenge where needed, bounded payloads, duplicate throttling). Do not treat CORS as authentication.
- Do not log full user reports or attachment contents in the broker's request logs.
- If the project owner prefers no broker for the first release, ship a robust copy/export + prefilled browser issue-form fallback first; do not embed a PAT as a shortcut.
- Direct GitHub API submission from the desktop app is allowed only through a secure user-authorized flow; no shared maintainer credential.
- Native and WASM clients should share the same report schema and preview logic, with platform adapters for metadata/screenshot capture.

Initial GitHub labels:
- `type:bug`, `type:feature`, `type:change`, `type:question`
- `area:ui`, `area:canvas`, `area:document`, `area:import-export`, `area:performance`, `area:wasm`, `area:platform`, `area:accessibility`
- `needs-triage`, `needs-reproduction`, `good-first-issue`, `confirmed`, `regression`

Use existing repository label conventions if already established; do not create duplicate taxonomies.

### 6.4 Duplicate detection and issue lifecycle

For 1.0, keep duplicate detection simple:
- Search recent/open issues by title, category, and a few relevant keywords.
- Show likely matches before submission.
- Let the user continue anyway.
- Never auto-close, auto-merge, or silently redirect a report based on a similarity score.

Use the lifecycle already specified in the post-MVP roadmap: submitted → triage → needs information → accepted/planned → in progress → testing → released; also allow duplicate, cannot reproduce, deferred, and declined. Add a short maintainer explanation for deferred/declined requests.

### 6.5 Acceptance criteria

- A user can submit a bug report and a feature/change request from both native and WASM clients.
- The report preview is editable and attachments are individually removable.
- Metadata is accurate for each target; missing fields are omitted or marked unavailable.
- No artwork, original filename/path, credentials, or screenshot is sent without explicit opt-in.
- Submission creates the correct GitHub issue type/labels and returns a working issue link.
- Rate limits and malformed/oversized payloads are tested.
- Offline, broker outage, GitHub outage, and auth failure all preserve the report and provide copy/export/browser fallback.
- The feature does not block document editing or startup when the network is unavailable.
- Automated tests verify redaction of paths, tokens, secrets, and sensitive environment values.
- A human privacy review approves the exact metadata and screenshot/log consent language before release.

## 7. Phase 4 — GUI and quality verification

The CraftAnything experiment's strongest contribution is the move toward deterministic testing of the actual GUI. Make this a first-class permanent capability in the canonical repo.

### 7.1 Test pyramid

1. **Unit tests:** document operations, commands, transforms, serialization, metadata capture, redaction, report schema, and state reducers.
2. **Integration tests:** open/edit/save/reopen, import/export, undo/redo, page/layer persistence, recovery, and GitHub submission broker contract.
3. **Headless GUI tests:** egui widget state, keyboard/pointer gestures, menu dispatch, panel operations, focus, dialogs, and navigation.
4. **Screenshot/golden tests:** stable workspace, dark/light theme, menus, panels, canvas selection, text editing, export dialog, first-run screen, feedback form and report preview.
5. **End-to-end tests:** launch actual native/browser build, execute common workflows, verify persistence/export, capture failure evidence.
6. **Platform smoke tests:** install, launch, file open/save, input, screenshot, and clean shutdown on each claimed target.

### 7.2 Required GUI scenarios

At minimum:
- First launch → create document → draw shape → save → close → reopen.
- Import raster → resize/position → add vector overlay → export PNG.
- Create/edit Bézier path → move nodes/handles → undo/redo.
- Create/edit text → commit and cancel with keyboard shortcuts.
- Multi-select → align/distribute/transform.
- Layer visibility/lock/reorder/rename.
- Page navigation and persistence.
- Panel open/close/resize/dock/reset and layout persistence.
- Zoom/pan/fit, pixel-accurate zoom, guide/grid/snap interactions.
- Context menu and menu/toolbar command parity for common operations.
- Drag/drop supported files and invalid-file handling.
- Unsaved changes, save failures, recovery prompt, and malformed document.
- Bug report and feature request with metadata preview, screenshot consent, redaction, successful submit, and offline fallback.

### 7.3 Quality gates

Use the repository's current `cargo run -p xtask -- ci` and existing CI workflows as the source of truth; do not invent a parallel gate. Audit the gates for correctness and fresh-workspace behavior.

Required gates:
- `cargo fmt --all -- --check`
- Clippy with repository-approved warning policy.
- Unit/integration tests and coverage gates already defined by the project.
- Native release build.
- `wasm32-unknown-unknown` build and browser bundle verification.
- GUI/headless test suite.
- Browser Playwright smoke and screenshot tests where configured.
- Security/dependency/license/secret scans and SBOM/provenance checks.
- Fresh clone/workspace reproducibility.
- Release artifact install/launch/smoke test.

Coverage should be a trend and risk indicator, not a reason to write brittle tests solely to inflate a number. Add tests around high-risk document integrity, input behavior, file I/O, report privacy, and release/update paths first.

No baseline should be weakened just to make a branch green. If a gate is broken, repair the gate or document a narrowly scoped, time-limited exception with owner approval.

## 8. Phase 5 — Staging, beta cohorts, and controlled rollout

This is the foundation for the requested “dark factory” improvement loop. Automate repetitive work, but keep human approval at the trust boundaries.

### 8.1 Environments and channels

Keep environments distinct:

- **Development:** local agent worktrees and ephemeral test artifacts.
- **CI:** reproducible builds/tests for each PR.
- **Staging:** deployable candidate built from a specific commit, isolated from production.
- **Early access/beta:** opt-in users and explicitly selected testers.
- **Production/stable:** signed, immutable, published release.

For Cloudflare:
- Keep the marketing site Pages project separate from the WASM editor project.
- Keep staging and production project names/configuration separate.
- Every deployment records source commit, build workflow/run, artifact digest, and environment.
- Never let a staging deploy overwrite production assets or point the stable site at unverified WASM.
- Use a staging URL that is clearly labeled and not indexed as the stable product.

For desktop:
- Publish separate prerelease/beta artifacts and stable release artifacts.
- Do not force-update users from a beta channel to stable or vice versa without explicit channel selection.
- Updates should be opt-in initially; show release notes, version, and rollback/reinstall instructions.
- Verify package signatures/checksums and provenance before update/install.

### 8.2 Automated issue-to-PR workflow

Permitted automation:
1. New issue webhook creates a triage record and applies labels.
2. Agent summarizes issue, extracts environment details, searches duplicates, and proposes severity/area.
3. Agent asks for missing reproduction details through an issue comment; it does not invent missing facts.
4. Agent creates an isolated worktree/branch from current `main` (or approved release maintenance branch).
5. Agent reproduces the bug with the submitted minimal repro or a generated fixture; never needs the user's original artwork unless explicitly provided.
6. Agent implements the smallest defensible fix and adds regression tests.
7. Agent opens a PR linked to the issue, with reproduction, root cause, diff summary, test evidence, risk, and rollback notes.
8. CI runs the full applicable gates.
9. Human reviewer approves merge.
10. Merge produces a staging candidate, not an automatic production release.

Automation must not:
- Merge its own PR.
- Change labels/status to “fixed” without a verified release.
- Access unrelated user files or secrets.
- Upload private attachments into public issues without explicit consent.
- deploy to stable automatically.
- alter production feature flags or cohort membership without authorized approval.

### 8.3 Human validation and staged rollout

For each candidate:
1. Publish candidate to staging with a unique build identifier and commit SHA.
2. Run automated smoke and regression suites.
3. Invite the reporting user to test the fix, if they opted in to follow-up/beta testing.
4. Add a small opt-in cohort (maintainers and volunteer testers) with explicit target platform/version.
5. Verify the reported workflow and adjacent regression tests.
6. Expand gradually only when the previous cohort is healthy: suggested gates 1–5% → 10–25% → 50% → 100%, adjusted to the actual audience and distribution mechanism.
7. Keep a release owner responsible for the promotion decision.
8. Monitor crash rate, failed launches, save/export failures, issue volume, and rollback signals using privacy-respecting, opt-in telemetry or volunteered reports. Do not make telemetry mandatory.
9. If a regression appears, halt promotion, disable the affected feature flag if safe, and roll back the web deployment or publish a known-good desktop release.
10. Close the issue only when the fix is present in a stable release and the changelog/release link is attached.

For desktop applications without a reliable remote feature-flag/update service, cohort rollout means distributing signed prerelease artifacts to opt-in testers, not pretending there is a server-side percentage rollout. For WASM, use staging deployments and controlled production promotion. Do not build a complex remote feature-flag system solely to simulate percentage rollout.

### 8.4 Release artifacts and observability

Every candidate/release should retain:
- Git commit SHA and tag.
- Build workflow/run URL.
- Artifact checksums and manifest.
- SBOM and provenance/attestation where supported.
- Platform/architecture and toolchain.
- Test summary and known issues.
- Migration/document compatibility notes.
- Release notes and rollback instructions.

Collect the minimum operational signals needed to detect failures. Prefer opt-in diagnostics and aggregate counts. Never send document contents or personal file paths as telemetry.

## 9. Phase 6 — Release FireCraft 1.0

### 9.1 Platform support matrix

Create a single generated or maintained matrix with these columns: target, artifact, CI build, install test, launch test, file I/O test, GUI E2E, known limitations, support tier, evidence URL.

At minimum evaluate:
- Linux x86_64 desktop (current published release target).
- Windows x86_64 desktop.
- macOS Apple Silicon and Intel if feasible.
- Browser/WASM on current Chrome/Edge/Firefox and Safari where supported.

A platform is **Supported** only after a real artifact is installed/launched and the core workflow passes on that platform. A successful compile is **Builds**, not Supported. A platform that has only been smoke-tested is **Preview**. Keep this matrix consistent across README, website, release notes, and downloads.

### 9.2 1.0 release gates

Do not tag 1.0 until all are true:

- The canonical integration branch is merged and reproducible from a clean clone.
- No known P0 data-loss, corruption, startup, save, or export defect remains.
- P0 workflows in §5.2 pass on each claimed supported target.
- Native and WASM builds pass their required CI and manual smoke checks.
- GUI interaction and screenshot regression suite is stable and useful.
- The feedback flow is usable, privacy-reviewed, and has offline fallback.
- Install/uninstall/update instructions are verified.
- Release artifacts have checksums, manifest, provenance/SBOM as applicable.
- README, website, feature map, CS6 parity matrix, changelog, docs index, and support matrix agree.
- Known limitations and unsupported formats/platforms are explicit.
- At least a small external tester cohort has completed real workflows and provided feedback; aim for 5–10 testers across the actual supported targets, not a vanity user count.
- The owner approves the final release candidate.

### 9.3 1.0 is not a promise of 100% CS6 parity

Publish a concise “What FireCraft 1.0 does / does not do” page. Clearly separate:
- Available and verified.
- Available with limitations.
- Preview/experimental.
- Planned.
- Not supported.

Do not inflate 1.0 by counting engine commands, panel IDs, or source files. The release is judged by complete user workflows and user trust.

## 10. Website, documentation, and community launch

Use the existing `site/` and [MARKETING-SITE-ROADMAP.md](MARKETING-SITE-ROADMAP.md); do not create another marketing site or conflate the marketing Pages project with the WASM app.

Before launch:
- Refresh screenshots from the actual release candidate, not mockups.
- Show vector + bitmap editing in one real document.
- Demonstrate layers/pages/panels and a save/reopen/export workflow.
- Provide a concise video/GIF with a real workflow.
- Add a clear 1.0 feature/limitations table and platform support matrix.
- Make download buttons resolve only to verified release assets.
- Link in-app feedback, GitHub Issues/Discussions, contributing, privacy, and release notes.
- Retain the Adobe independence/trademark notice.
- Do not use fabricated testimonials, user counts, or unverified performance comparisons.

Launch sequence:
1. Existing Fireworks community: explain the problem and invite workflow feedback.
2. Existing FireCraft GitHub audience: explain the consolidated roadmap and how to test/report issues.
3. Relevant open-source/Rust/Linux/web-design communities: share the actual product and architecture, not generic promotion.
4. Respond constructively to people asking for alternatives; disclose developer affiliation and answer their question first.
5. Treat competing Fireworks-inspired projects respectfully; collaborate on compatibility and user needs where useful.

Marketing claims should be evidence-led. The product's strongest early differentiator is not “we have every feature”; it is a real, integrated Fireworks-style workflow with a fast, transparent feedback loop.

## 11. Task breakdown and execution order

Convert this plan into small tickets under the existing `docs/tasks/` conventions. Do not duplicate tasks that already exist; link to and update the existing tickets.

Suggested work packages:

| ID | Work package | Priority | Dependencies | Exit evidence |
|---|---|---|---|---|
| FC10-00 | Local/worktree/PR inventory and integration decision | P0 | none | Dated reconciliation report; every ref has disposition |
| FC10-01 | Reconcile #186/#187 and current active UI work | P0 | FC10-00 | One integration branch/PR; source-to-result mapping |
| FC10-02 | Merge experiment + production readiness changes | P0 | FC10-01 | Clean full CI, native/WASM GUI tests |
| FC10-03 | Consolidate docs/task/parity/release authorities | P0 | FC10-02 | No contradictory/stale status; links verified |
| FC10-04 | GUI icon/theme/panel polish | P0 | FC10-01 | Visual review + snapshot/interaction evidence |
| FC10-05 | P0 workflow completion and regression suite | P0 | FC10-02 | Core workflow acceptance matrix passes |
| FC10-06 | Diagnostic schema, metadata providers, redaction | P0 | architecture review | Unit tests and privacy test corpus |
| FC10-07 | In-app feedback UI and report preview | P0 | FC10-06 | Native/WASM form and offline export |
| FC10-08 | GitHub submission broker or safe issue-form fallback | P0 | FC10-07 | End-to-end issue creation; rate-limit/security tests |
| FC10-09 | Staging environment and artifact provenance | P1 | FC10-02 | Staging deploy tied to immutable commit/artifact |
| FC10-10 | Issue → agent branch/PR workflow | P1 | FC10-08, FC10-09 | Demo on a non-production test issue |
| FC10-11 | Beta cohort and release promotion checklist | P1 | FC10-09 | Human-approved candidate promotion/rollback drill |
| FC10-12 | Platform install/launch/core workflow matrix | P0 | FC10-02 | Evidence per claimed supported target |
| FC10-13 | README/site/release documentation refresh | P0 | FC10-04, FC10-12 | Claims match published artifacts |
| FC10-14 | Branch/worktree cleanup | P1 | all integration work merged | Safe cleanup report, no unmerged work lost |
| FC10-15 | 1.0 RC, external user test, final release | P0 | all P0 work | Owner sign-off and release checklist |

The owner may reorder tasks based on the active worktree and actual blockers. Do not begin the feedback backend or broad refactors before the integration baseline and current UI direction are agreed.

## 12. Required agent handoff report

At the end of each phase, the agent must report:

1. Starting and ending commit SHA.
2. Branch/worktree used and whether it is clean.
3. PRs/issues/branches merged, closed, retained, or deferred.
4. Files/modules changed and why.
5. Tests actually run, exact commands, pass/fail counts, and skipped tests.
6. Native/WASM/platform evidence and screenshots/artifacts.
7. Known regressions, limitations, risks, and unresolved questions.
8. Task ledger/parity/feature/docs updates.
9. Whether a human decision is required before proceeding.
10. Next smallest safe unit of work.

Do not report “all tests pass” without listing the commands and results. Do not claim a platform or workflow is verified without evidence.

## 13. First-run checklist for the implementation agent

- [ ] Read `AGENTS.md` and the current task/handoff/release documents.
- [ ] Inspect `git status`, all worktrees, local branches, remotes, stashes, and agent sessions.
- [ ] Locate the owner's newest active GUI polish branch/worktree and preserve it.
- [ ] Fetch current GitHub PRs, reviews, CI, changed-file lists, and commits.
- [ ] Reconcile #186 and #187; do not merge both blindly.
- [ ] Compare the experiment repo and active worktree against current `main`.
- [ ] Create a dated inventory and proposed merge order before code changes.
- [ ] Obtain owner approval for the integration baseline and any destructive cleanup.
- [ ] Build/test the selected baseline before making unrelated UI changes.
- [ ] Convert this plan into deduplicated, small tasks in the existing ledger.
- [ ] Start with the smallest integration slice and preserve a rollback ref.

## 14. Definition of project success

FireCraft 1.0 is successful when a new user can install or open a supported build, understand the workspace without a tutorial, create or open an editable document, combine vector and bitmap content, make common changes, save and reopen without data loss, export useful assets, and report a problem from inside the application with confidence about what will be shared.

The project is operationally successful when a maintainer can receive that report, reproduce it from its metadata or a deliberately shared sample, hand it to an isolated AI agent, review a tested fix, deploy it to staging, invite the reporter/opt-in testers to validate it, and promote a known-good build through a human-approved release process.

That is the 1.0 promise: **a useful graphics editor and a trustworthy, continuously improving open-source project—not an unfinished feature checklist disguised as a release.**
