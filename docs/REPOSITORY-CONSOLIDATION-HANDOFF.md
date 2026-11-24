# FireCraft — Repository Consolidation Agent Handoff

> **Read this first after AGENTS.md.**
> **Master plan:** [REPOSITORY-CONSOLIDATION-AND-MODERNIZATION-PLAN.md](REPOSITORY-CONSOLIDATION-AND-MODERNIZATION-PLAN.md).
> **Snapshot baseline:** main 4ff8f786e365c8b7d7609e435424c9c746e61d6d, 2026-11-24. Refresh before action.
> **This handoff authorizes audit and planning first; it does not authorize destructive cleanup, license changes, production deploys, or blind merges.**

## Mission

Consolidate FireCraft's branches, PRs, local development work, code structure, feature backlog, and release process. Recover valuable work, remove true duplication and generated debris, refactor high-coupling modules safely, polish existing workflows, and then adopt high-value creative features from other products where they fit the project.

The expected result is one coherent, tested application and an understandable repository—not an indiscriminate merge of every branch or a port of entire competitor applications.

## Mandatory operating contract

1. Follow AGENTS.md. It is the operating authority.
2. Start from current main, then refresh every remote, branch, PR, issue, check run, and local worktree. Do not use this document's snapshot as live truth.
3. Preserve all staged, unstaged, untracked, ignored-but-relevant, and local-only work.
4. Identify every active agent/worktree and assign file ownership before shared edits.
5. No hard reset, clean, force push, bulk branch delete, or bulk worktree prune during the initial audit.
6. No broad git add -A when generated or other-agent files may exist.
7. One objective per PR; split cross-cutting PRs by behavior and verification boundary.
8. Never merge a PR whose checks are skipped/stale and represent that as passed.
9. Keep Apache-2.0 as the default project licensing posture. Do not copy GPL/proprietary code into FireCraft without explicit licensing approval and legal review.
10. Do not copy Adobe code, binaries, artwork, source, or local reference-install data.
11. Keep the same document model and command registry as the authority for GUI, CLI, scripting, browser, and MCP.
12. Any web/shell change receives real-browser verification. Any data-model/file-format change receives migration, corpus, unknown-field preservation, and fuzz coverage.
13. Production deployment, DNS, domain changes, Cloudflare project mode changes, or publication need explicit owner approval.
14. TASKS.md remains the single live task ledger. Do not create another board.

## Phase-by-phase execution

### Phase 0 — Freeze, preserve, and inventory

- Record repo root, current branch/HEAD, remotes, status, ignored files, local branches, tags, and worktrees.
- Ask each active agent for branch, SHA, worktree, dirty files, files owned, and next commit.
- Inventory local-only commits and compare them with every remote.
- Refresh open PRs, base/head SHAs, changed files, review comments, and CI/check status.
- Record branch ancestry and changed-file overlap.
- Mark anything uncertain UNVERIFIED.
- Stop before code changes if ownership or preservation is unresolved.

**Required output:** current baseline/integration inventory with exact SHAs and owner/disposition for every active worktree and branch.

### Phase 1 — PR triage and integration plan

Refresh these items from current GitHub state:

- **#182:** already merged in the observed snapshot. Do not repeat the old merge instruction.
- **#184:** five-file deployment receipt PR; resolve review comments (HTML beacon checksum exception, status vocabulary, manual Pages deploy fact, live hash comparison), verify evidence, then merge if still clean.
- **#178:** preserve the slice-behavior export fix, but extract it from the release-doc and release-tooling changes. Fix exact-name priority and safe JS/HTML path encoding; rebase and run focused tests.
- **#181:** 243-file/96-commit mixed PR in the observed snapshot with conflicts and a failed action-pin gate. Decompose into focused PRs; remove generated output; reassess the obsolete CDN/proxy design against same-origin release evidence. Do not merge wholesale.
- **feat/post-mvp-execution:** contains the marketing site implementation. Preserve the site work, but reconstruct a site-only PR on current main, excluding documents already merged in #182.
- **review/post-mvp-execution:** duplicate SHA with feat/post-mvp-execution in the observed snapshot. Check worktree ownership before retiring it.

Create a PR decision matrix with objective, unique changes, overlap, current checks, review findings, tests, owner, and disposition.

**Required output:** a small ordered integration queue. Do not create feature tickets until the current PRs/branches have been reconciled.

### Phase 2 — Recover unique feature work and consolidate branches

Audit Adam7 PNG, selective JPEG, inspector field sets, parity hardening, missing-docs campaign, release provenance, CI cache workaround, WASM budget, and historical release branch.

For each branch:
- list unique commits and files;
- verify whether the implementation/test/decision already exists on main;
- identify any local worktree/active owner;
- extract only the useful patch;
- create one focused PR per concern;
- preserve attribution, tests, and decision history;
- delete the branch only after the deletion gate in the master plan passes.

Do not chase a target branch count. The goal is zero lost work and one active branch per owned task.

### Phase 3 — Repository hygiene

Audit and safely classify:
- target/windows experiment binaries;
- web/dist generated JS/WASM/EXE;
- tracked session log files;
- duplicate screenshots and images;
- broken references to old asset names;
- invalid PNGs, stale or duplicated logo exports;
- local caches, node_modules, test-results, Playwright output;
- generated docs and reports;
- unused/duplicated Cargo dependencies;
- docs that describe merged PRs as still open.

Remove only what is proven generated, duplicated, obsolete, or inappropriate for source control. Check references, hashes, manifests, provenance, and packaging consumers first. Keep canonical assets and required test fixtures.

### Phase 4 — Structural code refactor

Do not start by splitting every large file. Re-measure current production/test boundaries and preserve behavior.

Priority candidates:
1. panels.rs: split by panel responsibility (properties, layers, pages/states, history, styles/resources, fill/stroke, help/shared).
2. shell/mod.rs: extract tests first, then move coherent responsibilities to modules.
3. editor.rs: split pointer/gesture, selection/transforms, vector interaction, bitmap interaction, text interaction, and viewport logic.
4. xtask/src/dist.rs: split native/web assembly, manifests, SBOM/provenance, verification, and smoke orchestration.
5. menus.rs: separate tests or declarative data only if ownership improves; much of its size is static menu data.
6. panels_web.rs: keep platform-specific UI isolated while reusing shared engine/command semantics.

Use small PRs. Preserve visibility and layering rules. Never create duplicated editing logic or direct document mutation paths in UI panels.

### Phase 5 — Finish and polish existing features

Audit end-to-end workflows across document open/save/recovery, vector/bitmap editing, text, masks, effects, pages/states/layers, symbols, slices/hotspots, export, CLI, scripting, browser bridge, and MCP.

For each workflow record:
- exact user steps and expected visible result;
- undo/redo behavior;
- save/reopen behavior;
- export output and known fidelity limits;
- relevant tests and browser/native evidence;
- status and outstanding work.

Reconcile open issues with current main and the task ledger. Close genuinely complete issues with proof; rewrite partial issues to contain only remaining work; never close based solely on a source symbol or PR existing.

### Phase 6 — Competitor feature research and selective reuse

Benchmark feature/workflow ideas from:
- Krita: brush engines, stylus/pressure/tilt, tablet diagnostics, popup palette, resource browser, soft proofing.
- GIMP/GEGL/babl: raster selection/transform, filter operations, tiled raster architecture, color conversion.
- Inkscape/lib2geom: vector node editing, path operations, snapping, SVG fidelity.
- Photopea: Smart Objects, Smart Filters, adjustment layers, broad PSD workflows, scripting and host messaging.
- Pixlr: quick actions, guided image editing, subject/background operations, optional AI.
- Penpot: components, design tokens, prototyping, handoff and collaborative design concepts.
- SVG-Edit: browser SVG interaction and path editing.

Use behavior/docs as product research. Reuse source code only after exact file-level license, dependency, wasm/platform, security, maintenance, and architecture review:
- Krita and GIMP are GPL; do not transplant GPL-covered application code into the Apache-2.0 project without an explicit project licensing decision and legal review.
- Inkscape is GPL; its code is not an Apache-compatible default source.
- Penpot's repository metadata identifies MPL-2.0; file-level obligations require explicit review.
- SVG-Edit identifies MIT; individual reusable routines may be candidates after commit/file/dependency inspection and attribution.
- Photopea and Pixlr are proprietary; no code/assets/copying/decompiling/scraping. Their public APIs do not license their internal implementation.
- Third-party files/assets/fonts/brush packs carry their own licenses; never infer them from the application's license.

For each adopted feature: write a user problem, compare current FireCraft support, define UX/engine/doc-model/performance needs, write ADR if persistent semantics change, implement one end-to-end workflow, test, document, and update attribution.

### Phase 7 — Reliability, format, performance, security

Reuse PM-01 through PM-08 and existing tickets. Prioritize:
- atomic saves and recovery fault injection;
- format capability registry and explicit loss warnings;
- representative import/export corpus;
- performance measurements by SHA/host/profile/document;
- wasm size/startup measurement;
- bounded hostile-input handling and fuzz anchors;
- MCP/script permissions and data access;
- keyboard/accessibility and browser parity.

Do not change budgets or generated floors without understanding and recording the new measurement basis.

### Phase 8 — Marketing website

Keep the existing static site implementation. Rebase/reconstruct from feat/post-mvp-execution into a focused site PR. Verify:
- real screenshots and product claim matrix;
- release metadata allowlist/fallback and actual published download assets;
- desktop/tablet/mobile UI and accessibility;
- SEO, sitemap, robots, CSP/headers/cache;
- Pages project separation and preview deploy;
- current project mode/domain/output from actual Cloudflare configuration.

Marketing site is site/ → firecraft-site. Editor is dist/web → firecraft-web. Never repoint one to the other. Production deployment and DNS changes are owner-gated.

### Phase 9 — Release and platform evidence

Reconcile existing T27 tickets, artifact matrix, execution board, and release receipts. A build recipe is not a verified artifact; an artifact is not an install; an install is not a successful launch. Publish platform support only with native build/install/launch/smoke evidence. Verify release hashes against the live editor deployment and keep draft release creation separate from production publication.

### Phase 10 — Closeout and maintenance

Before each session ends:
- update TASKS.md and its ticket;
- regenerate applicable scorecard/parity/corpus evidence;
- update PROGRESS/HANDOFF as required;
- record the exact SHA and test/browser receipts;
- identify open blockers and next owner;
- ensure log/devlog.md stays untracked;
- push in AGENTS.md remote order.

## Initial command checklist

Run in each relevant worktree. Do not run destructive commands during initial inventory.

~~~bash
git status --short --branch
git rev-parse HEAD
git remote -v
git branch -vv
git worktree list --porcelain
git log --oneline --decorate --graph --all -80
git status --ignored --short
~~~

Before commits use current AGENTS.md commands. At the observed baseline, the canonical full gate is:

~~~bash
cargo fmt --all
cargo run -p xtask -- ci
~~~

Use focused crate tests as appropriate. Do not use cargo test --workspace as a shortcut. For any shell/web change, execute the real-browser workflow in HANDOFF.md §6.2 against the exact built dist/web. Do not claim that a skipped check passed.

## Required deliverables before feature work starts

1. Current branch/PR/worktree/agent ownership inventory.
2. PR-by-PR disposition matrix and integration order.
3. Unique branch commit recovery table.
4. Repository artifact/asset/attribution cleanup report.
5. File-level architecture/refactor map with tests and dependencies.
6. Updated issue/task deduplication map.
7. Feature-transfer/license matrix for the selected competitor ideas.
8. A small first execution batch with owners, files, acceptance tests, and stopping conditions.

## Agent stop conditions

Stop and report if local/agent ownership is unclear, any unique work could be lost, a PR has skipped/stale validation, a proposed code source has an unclear or incompatible license, a document-format change could discard unknown data, a deployment would change production configuration without owner approval, or a feature cannot be verified on its target platform.

Do not start speculative feature implementation, mass cleanup, mass branch deletion, broad module rewrites, or license changes before the eight deliverables above exist and have been reviewed.
