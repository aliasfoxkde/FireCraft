# FireCraft 1.0 — End-to-End Execution, Distribution, Documentation, and GitHub Closeout Plan

> **Document type:** phase-gated implementation and launch work order.
> **Prepared:** 2026-11-26.
> **Status:** expanded owner-requested scope; this is a plan, not a claim of completed implementation.
> **Canonical repository:** https://github.com/aliasfoxkde/FireCraft
> **Parent handoff:** [FireCraft 1.0 Consolidation and Release Handoff](FIRECRAFT-1.0-CONSOLIDATION-AND-RELEASE-HANDOFF.md).
> **Product strategy:** [Product Expansion Roadmap](FIRECRAFT-PRODUCT-ROADMAP-AND-EXPANSION.md).
> **Release authorities:** [Release Engineering Plan](../RELEASE-ENGINEERING-PLAN.md), [Releasing](../RELEASING.md), [Artifact Matrix](../release/ARTIFACT-MATRIX.md), and [T27 Cross-Platform Release](../tasks/T27-cross-platform-release.md).
> **Marketing:** [Marketing Site Roadmap](../MARKETING-SITE-ROADMAP.md).
> **Live task authority:** [TASKS.md](../TASKS.md). This document is not a second task board.

---

## 0. Executive mandate

Deliver a coherent, polished, supportable FireCraft 1.0 product and a professional public release. Work is not complete when branches are merged or the application compiles. It is complete when a new user can discover FireCraft, download or open a ready-to-use build, install it on a declared supported platform, complete core creative workflows, save/reopen work safely, export usable assets, find documentation, report problems, and receive fixes through a controlled community/release process.

The final launch must include:

1. One canonical FireCraft repository, one integrated GUI, one document/command authority, and auditable source history.
2. The strongest UI enhancements from the local CraftAnything experiment reconciled with current FireCraft development.
3. A stable, practical editor MVP with honest feature and compatibility claims.
4. A completed landing page deployed to its dedicated Cloudflare Pages project.
5. In-app feedback with user-controlled diagnostics and a secure GitHub issue workflow.
6. A comprehensive, versioned GitHub release with ready-to-use WASM, Windows, macOS, and Linux packages, plus CLI, source, checksums, SBOM, provenance, and release notes.
7. Refreshed README, docs, wiki, release notes, contribution information, support/security guidance, and templates.
8. An audited issue/PR/branch/worktree inventory with every ticket resolved, superseded, consolidated, deferred with an owner/next action, or intentionally retained.
9. Reproducible CI, target-native test evidence, artifact verification, staging promotion, rollback, and human-approved production publication.

**Quality principle:** target the majority of workflows users actually expect, not a vanity feature count. Do not advertise unverified features or platforms. Avoid delaying 1.0 for unrelated speculative work once the approved release matrix is satisfied.

## 1. Authority and scope reconciliation

### 1.1 Canonical documentation authorities

| Document | Role |
|---|---|
| AGENTS.md | Repository rules, clean-room boundary, test commands and agent workflow |
| docs/TASKS.md | Sole live task ledger |
| docs/NEXT.md | Phase plans and acceptance criteria |
| docs/HANDOFF.md | Agent orientation and operating runbooks |
| docs/PLANNING.md | Core architecture and document format |
| docs/FEATURES.md | Verified user-visible feature surface |
| docs/CS6-PARITY.md | User-visible compatibility/parity inventory |
| docs/PROGRESS.md | Dated factual snapshot and receipts |
| docs/RELEASE-ENGINEERING-PLAN.md | Cross-platform artifact targets and packaging/certification |
| docs/RELEASING.md | Actual release commands and current automation |
| docs/release/ARTIFACT-MATRIX.md | Per-artifact evidence state |
| docs/tasks/T27-cross-platform-release.md | Release work package |
| docs/MARKETING-SITE-ROADMAP.md | Landing page and Cloudflare Pages |
| docs/POST-MVP-ROADMAP.md | Detailed strategic product backlog |
| docs/wiki/ | Version-controlled wiki source pages |

Where documents disagree, inspect code, CI receipts, current issue descriptions, actual packages, and deployed output. Record the resolution in a dated audit report. A stale paragraph is not evidence.

### 1.2 Reconcile accepted release-scope decisions

The repository has accepted decision 0075 (2026-11-17), which narrows an initial MVP candidate to web/WASM, Linux x86_64 tar/DEB, CLI, source, SBOM, checksums, and matching deployed web hashes. The long-term release plan covers more platforms. Decision 0070 defers RPM. These repository decisions must not be silently ignored.

The owner has now explicitly requested a more comprehensive public 1.0 release with ready-to-use browser/WASM, Windows, macOS, and Linux packages including AppImage, Flatpak, DEB, and related formats. Therefore:

1. Review decision 0075 and decision 0070 against the current release matrix and receipts.
2. Prepare a new release-scope decision that explicitly supersedes/amends the narrow 1.0 cut and re-evaluates RPM.
3. State mandatory 1.0 packages, optional architectures, deferred targets, signing requirements, and evidence requirements.
4. Preserve prior decisions as historical records; do not rewrite them to conceal earlier scope.
5. Update TASKS, T27, ARTIFACT-MATRIX, RELEASING, README, wiki and site copy to agree with the new approved decision.

The owner has authorized preparing this expanded scope. It does not authorize claiming a package has already been built or certified.

### 1.3 PhotoCraft benchmark

Use:
- [PhotoCraft releases](https://github.com/storytold/photocraft/releases)
- [PhotoCraft release guide](https://github.com/storytold/photocraft/blob/main/docs/releasing.md)
- [PhotoCraft repository/download overview](https://github.com/storytold/photocraft)

The latest release checked for this plan was PhotoCraft v0.23.1, published 2026-11-12. Its GitHub release exposed 54 assets, including Windows x64/ARM64 MSI and portable ZIPs, macOS universal DMG, Linux x86_64/aarch64 AppImage, DEB, Flatpak, RPM and tar.gz, browser ZIP, CLI, checksums, and detached signature files. Its release guide documents draft-first publication, platform signing, package verification, and native runtime testing.

Use it as a model for breadth, asset naming, user download ergonomics, release automation, integrity material, and per-target testing. Do not copy PhotoCraft code, credentials, assets, packaging manifests, or unreviewed assumptions. Adapt patterns to FireCraft's architecture, license, clean-room boundary and real supported targets.

---

## 2. Phase 0 — Preserve, inspect the experiment, and establish the baseline

**Goal:** know exactly what exists before integrating or deleting anything.

### 2.1 Local CraftAnything experiment audit

The owner identifies this directory as a key UI source:

**/nas/Temp/repos/FireCraft-CraftAnything-Experiment**

At plan preparation (2026-11-26), the path was unavailable. On 2026-11-27 it was available and the current `main`/v0.3.5 source, release evidence, architecture seams, and UI behavior limits were reviewed. The PR #189 integration is present in the current release lineage. That establishes the current product baseline; it does not prove every experiment-only asset or commit has been reconciled. Complete the artifact-by-artifact comparison below before archiving any experiment branch or worktree. The [dated audit](https://github.com/aliasfoxkde/FireCraft-CraftAnything-Experiment/blob/docs/post-v035-audit/docs/AUDIT-2026-11-27.md) and [task ledger](https://github.com/aliasfoxkde/FireCraft-CraftAnything-Experiment/blob/docs/post-v035-audit/docs/TASKS.md) in the canonical repository record the verified release and remaining review limits.

Run and record:
- Test that the directory exists.
- Record repository root, HEAD SHA, branch, remotes, tags and commit graph.
- Capture clean/dirty state, staged/unstaged/untracked/ignored files, stashes, submodules and LFS.
- Inventory UI modules, panels, toolbars, icon/font/image assets, theme resources, tests, screenshot fixtures, docs, build scripts and dependencies.
- Identify unique work, duplicated work, work already merged into FireCraft, and changes overlapping the active newest worktree or PRs #186/#187.
- Record source provenance and license/attribution for all assets and dependencies.
- Map each useful change to an intended FireCraft file/module and corresponding verification.

**Preservation rules:** begin read-only; do not run destructive cleanup, reset, stash-pop, overwrite, rebase or force-push in the experiment repository. Create a verified backup/reference before importing. Preserve the experiment until the integrated product is validated and the owner approves archival. Follow FireCraft clean-room/IP rules: no Adobe source, application artwork, proprietary icons/fonts or restricted material enters the repo.

If the path becomes inaccessible, report the blocker and request an accessible mount or owner-provided archive. Do not invent its content or silently skip it.

### 2.2 FireCraft repository inventory

Capture current:
- git status, all worktrees, local/remote branches, stashes, remotes, tags and unique commits.
- Agent sessions, dirty diffs, local notes, untracked files and ownership conflicts.
- All open/closed PRs, complete patches, review comments, check suites and merge ancestry.
- All open issues and linked task tickets.
- CI pipeline status and recent failing runs.
- Current release tags/assets and artifact-matrix receipts.
- Cloudflare Pages project names, domains, deployment history, current deployed commit and asset hashes.

Create a dated, machine-readable branch/worktree/PR inventory and human-readable reconciliation report under docs/integration/.

### 2.3 PR reconciliation

At the 2026-11-26 GitHub snapshot, the open PR search returned:

| PR | Branch | Required action |
|---|---|---|
| [#186](https://github.com/aliasfoxkde/FireCraft/pull/186) | temp/consolidation | Inspect consolidation, WASM and production-readiness changes; record unique content |
| [#187](https://github.com/aliasfoxkde/FireCraft/pull/187) | feat/experiment-merge | Inspect experiment UI changes; record unique content |
| [#188](https://github.com/aliasfoxkde/FireCraft/pull/188) | docs/firecraft-v1-consolidation-handoff | Review documentation package; keep separate from code integration |

Earlier comparison showed #186 and #187 have diverged histories. Re-run the comparison against current heads. Never merge them sequentially just because GitHub labels both mergeable. Review actual patches, commit ancestry, overlaps, test results and the local experiment.

For every PR, record retained, cherry-picked, reimplemented, superseded, deferred or rejected content and resulting commit/PR references. Close a superseded PR only after its unique changes are accounted for and the replacement passes checks.

### 2.4 Phase 0 exit

- Experiment exists or its access blocker/alternate path is recorded.
- Experiment HEAD, branch, dirty state, unique changes and asset/license inventory recorded.
- Every local worktree has an owner and disposition.
- Every open PR has a file/behavior/commit reconciliation.
- Branches with unmerged commits are protected.
- Integration sequence approved.
- No unknown local work or overlapping agent ownership remains.

---

## 3. Phase 1 — Consolidate repository and architecture

**Goal:** one product, one source tree, one engine/command authority and one release path.

### Work packages

1. Create an integration branch from current approved main.
2. Merge/reimplement shared engine and document-model changes.
3. Integrate the best GUI shell, panels, interactions, toolbars, icons, menus, context menus and preferences.
4. Integrate import/export and file-compatibility changes.
5. Reconcile WASM/browser bridge and hosting.
6. Reconcile CI, tests, packaging and release engineering.
7. Consolidate docs and website without mixing independent deployment outputs.

Preserve one document model and command registry across native GUI, WASM, CLI, scripting, browser bridge and MCP. Avoid duplicate panels/tool registries, codecs, icon systems, UI frameworks or command adapters. Keep target-specific code at platform boundaries. Preserve undo/redo, transaction semantics, autosave/recovery, deterministic file operations and command journaling.

Clean stale code, dependencies, feature flags, scripts, generated files, metadata, screenshots, workflows and orphaned docs only after checking all call sites, tests, CI, release workflows and inbound links.

### Exit criteria

- One canonical UI and source tree.
- All imported changes have provenance.
- No duplicated engine/document/command implementation without an explicit decision.
- Full canonical CI, browser and WASM gates pass.
- Existing lint/coverage/parity floors are preserved.
- No secrets, restricted assets, unexplained dependency or license additions.
- Integration report maps every source PR/experiment commit to final commits.

---

## 4. Phase 2 — UI integration, refinement and MVP behavior

**Goal:** use the experiment as a primary UI reference while preserving FireCraft semantics.

### Required UI comparison

Compare experiment and current app in:
- Application frame, title/menu bars, toolbars/flyouts, iconography, themes, typography, spacing, hover/selected/disabled states.
- Canvas transparency, infinite canvas, zoom, drag-to-navigate, guides, rulers, grid, snapping, smart guides, handles, selection outlines and high-DPI.
- Tabs, unsaved state, multi-document workflow, recovery, close/quit prompts and layout persistence.
- Selection, transform, shapes, Bézier/path editing, text-in-place, raster tools, crop, gradients, masks and context menus.
- Tools, Layers, Properties/Inspector, Optimize, Pages/States, Timeline, History, Common Library, CSS Properties, Slices, Find/Replace.
- Drag/drop, clipboard, file dialogs, shortcuts, command palette, tooltips, dialogs, preferences and error states.
- Keyboard focus, accessibility semantics, contrast, screen-reader support where available.
- Browser/native differences and responsive behavior.

For each item record experiment behavior, current behavior, intended behavior, module/command dependencies, test coverage, screenshots and disposition.

### Visual system

Establish one original FireCraft visual language: approved logo, complete icon family, consistent sizes/strokes, typography, spacing, borders, color tokens, theme states, focus indicators and tooltips. Do not leave generic substitute icons or inconsistent toolbar treatments as the final UI. Review screenshot changes manually before updating baselines.

### Functional requirements

Every visible control must dispatch a valid engine/registry command or be visibly disabled/unavailable. UI mutations must preserve undo/redo. Dialogs must be cancellable and keyboard accessible. Panel layouts/preferences must persist and recover from malformed state. Native and WASM behavior should match where capabilities are shared.

### Core acceptance workflows

Launch → create → vector edit → add raster → text → layers → save → close → reopen; import supported formats; transform/align/group/mask; undo/redo; autosave/recovery; export actual bytes; panel persistence; keyboard-only workflow; browser file APIs/download/drag-drop/postMessage bridge.

**Exit:** a polished unified interface, complete P0 journeys, regression tests, current screenshots, no known P0 data-loss defect and no inert controls in the declared 1.0 surface.

---

## 5. Phase 3 — Marketing landing page and Cloudflare Pages

The existing site is the starting point. Do not create a competing site or point the WASM app deployment to marketing output.

### Required public journeys

- Home → try browser version when verified / download desktop.
- Home → real screenshots and feature detail.
- Features → clear availability and limitations.
- Downloads → correct artifact, architecture, checksum/signature state and install guide.
- New user → getting started, sample project and troubleshooting.
- Community → feedback, issue tracker, contribution, license and security contact.

### Required content and refinement

Product promise, differentiators, current release, real screenshots with build/date context, Available/Preview/In Development/Planned labels, compatibility matrix, supported platform matrix, privacy/local-first statement, accessibility statement, downloads and contribution links.

### Cloudflare boundary

Marketing source is site/ and historically used the Pages project firecraft-site. WASM output is dist/web and historically used firecraft-web. Verify project names and current deployments before use. Keep separate projects, domains, output directories, workflows, environment variables, credentials and rollback.

Set up PR previews, staging, protected production promotion, scoped deploy credentials, deploy receipts and rollback. Verify deployed bundle hashes/commit identity, not just Wrangler command success.

### Acceptance

Responsive layouts, keyboard/focus/contrast, semantic structure, reduced motion, SEO/canonical/OG metadata, sitemap/robots/favicon/404, link checks, working downloads/fallbacks, privacy/security headers, CSP, optimized assets and real-browser QA. No analytics/tracking by default. Any analytics requires explicit privacy review and disclosure.

**Exit:** staging site journeys pass, an authorized human promotes production, rollback is tested, and deployed commit/Pages deployment IDs are recorded.

---

## 6. Phase 4 — README, docs, wiki and contributor experience

**Goal:** make public-facing material consistent, accurate and understandable.

### README refactor

The root README should be a concise project front page, not the entire technical manual. It should contain:
1. Product name, logo, short value proposition and independence/trademark notice.
2. Current release/status and supported-platform badges only when evidence supports them.
3. Primary links: Try Web, Download, Getting Started, Features, Docs, Roadmap and Contribute.
4. What FireCraft is and why it is different.
5. Real application screenshots/workflow visual with build context.
6. Verified highlights and current limitations.
7. Download table linked to actual current GitHub release assets.
8. Quick-start for users and separate developer build instructions.
9. Privacy/local-first statement that reflects real behavior.
10. Links to wiki/docs/issues/security/contribution/release history.
11. License.

Move exhaustive feature tables, architecture, command references, long status history and platform installation details into dedicated docs. Preserve useful information and links. Remove stale feature counts, inaccurate platform claims, dead screenshots, aspirational functionality presented as implemented and duplicate instructions.

### Documentation inventory and reconciliation

Inventory all Markdown, generated docs, decisions, issue templates, wiki pages, workflows and site content. Classify as canonical/current, canonical/stale, historical, generated, duplicate/superseded or unowned/dead. Audit relative links, anchor links, case sensitivity, old PR/issue/branch references, package names, dates and domains. Regenerate generated material through its generator. Keep historical receipts labeled and immutable.

### Wiki source and live wiki

The repo has a docs/wiki source tree with Home, Getting Started, Project Status, Parity Roadmap, Editor and Layout, Documents and Formats, Browser and WebOS, Distribution and SDK, Automation, Architecture, Technical References, Troubleshooting, Contributing, Sidebar and Footer.

Audit every source page and the actual published GitHub Wiki independently. Verify that the Wiki feature is enabled and determine the publication/synchronization method; do not assume docs/wiki automatically publishes itself. Update content for onboarding, tools/panels, native/browser installs, every published package format, file compatibility, CLI/MCP, troubleshooting, feedback, contributions, privacy/security and release status. Use the wiki for concise user-facing guidance and docs/ for versioned engineering specifications. Add an explicit canonical-source and wiki-publish/check process.

### Contributor experience

Audit and improve existing CONTRIBUTING, CODE_OF_CONDUCT, SECURITY, issue templates and PR template. Ensure GitHub surfaces them. Provide development setup, architecture map, build/test/browser/package commands, clean-room/IP and attribution rules, review expectations, support policy, vulnerability reporting, release policy and contributor onboarding.

### Exit

README concise and current; wiki source and live publication synchronized; links resolve; feature/platform/download claims have evidence; support/privacy/security/installation instructions match reality.

---

## 7. Phase 5 — Full native, Linux and WASM distribution

**Goal:** users can download ready-to-use binaries and install without building Rust source themselves.

The technical authorities are RELEASE-ENGINEERING-PLAN, RELEASING, ARTIFACT-MATRIX and T27. Extend current mechanisms rather than creating duplicate packagers. Existing docs identify a working web and Linux x86_64 tar/CLI/DEB path, while Windows, macOS, AppImage and Flatpak still require target build/runtime receipts. Signing/provenance and deployment reconciliation are also known gaps. Re-fetch current receipts before final claims.

### 7.1 Required 1.0 candidate matrix

Any mandatory artifact formally excluded needs a maintainer-approved decision plus README/site limitation. It must not silently disappear.

| Target | Ready-to-use deliverables | Required evidence |
|---|---|---|
| Web/WASM | Versioned static firecraft-web ZIP with HTML, JS, WASM and required assets; deployed browser application | Clean extraction/static server; browser matrix; open/edit/save/export/bridge smoke; deployed bytes match release |
| Windows x64 | Portable ZIP and signed installer (.exe or .msi; choose primary installer, publish both only if maintainable); CLI and optional symbols | Native MSVC build; clean Windows VM install/launch; open/edit/save/export; upgrade/uninstall/reinstall; spaces/non-ASCII paths; signature verification |
| macOS | Universal arm64+x86_64 DMG/app; standalone CLI ZIP; optional PKG for managed deployment | Both slices verified; native install/launch; core workflows; code-signing, hardened runtime, notarization/stapling and Gatekeeper validation |
| Linux x86_64 | AppImage, DEB, Flatpak bundle, RPM after revisiting decision 0070, tar.gz; CLI archive | Package build and inspection; clean distro install; launch; open/save/export; upgrade/uninstall; AppImage runtime; Flatpak sandbox/portal; RPM verification |
| Linux aarch64 | AppImage, DEB, Flatpak, RPM, tar.gz and CLI where supported | ARM64 build plus ARM64 VM/device/runtime smoke; cross-compilation alone is not support evidence |
| CLI/automation | Standalone binaries/archives for supported native targets | Version/help, import/export/convert, document operations, permissions, streams and exit-code smoke |
| Source | Exact-tag source archive and GitHub-generated source ZIP/tar.gz | Clean extraction, locked build/tests, license/attribution/dependency audit |
| Integrity | SHA256SUMS, signed checksum file, manifest, CycloneDX SBOM, provenance/attestation and artifact signatures | Independent verification against every published asset |

Assess as additional/gated targets: Windows ARM64/x86, Linux Arch package, AppImage zsync, FreeBSD, Android APK/AAB and Flathub listing. Android requires a native feasibility decision; WASM is not an Android native app.

### 7.2 Example release asset naming

Use one deterministic versioned naming convention. Example patterns:

- firecraft-<version>-windows-x64-setup.exe
- firecraft-<version>-windows-x64-portable.zip
- firecraft-<version>-windows-x64-symbols.zip
- firecraft-<version>-macos-universal.dmg
- firecraft-cli-<version>-macos-universal.zip
- firecraft-<version>-linux-x86_64.AppImage
- firecraft-<version>-linux-x86_64.deb
- firecraft-<version>-linux-x86_64.rpm
- firecraft-<version>-linux-x86_64.flatpak
- firecraft-<version>-linux-x86_64.tar.gz
- firecraft-cli-<version>-linux-x86_64.tar.gz
- firecraft-<version>-linux-aarch64.<format>
- firecraft-web-<version>.zip
- firecraft-<version>-source.tar.gz
- firecraft-<version>-sbom.cdx.json
- release-manifest.json, SHA256SUMS.txt and SHA256SUMS.txt.asc

The final manifest is authoritative. Asset names, architecture labels, package IDs, MIME types, desktop entries, product identity, version and website links must agree.

### 7.3 Windows

Build on a Windows MSVC runner; GNU cross-builds and Wine are preview aids only. Produce stable portable ZIP and installer with publisher/product/version metadata, install scope, Start Menu shortcuts, documented portable data behavior, correct file associations only where implemented, and upgrade/uninstall behavior. Verify CLI and GUI subsystems, runtime dependencies, paths containing spaces/non-ASCII, restricted permissions and clean VM installs. Sign user-facing binaries/installers with Authenticode when configured and verify signatures/timestamps. Unsigned artifacts must be labeled accurately.

### 7.4 macOS

Build Intel and Apple Silicon binaries on a supported macOS host; join as universal where feasible. Assemble a polished DMG with correct app metadata, icons and Applications shortcut. Sign nested binaries/bundles correctly, use hardened runtime and minimal entitlements, notarize/staple when configured, and verify via codesign/stapler/Gatekeeper. Test install, first launch, core editing, replacement/upgrade and removal on supported macOS versions. Keep signing/notarization credentials out of the repository.

### 7.5 Linux

**AppImage:** stage a predictable FHS layout with desktop entry, icons, MIME metadata, licenses and binaries; pin/verify packager/runtime; test normal launch and extract-and-run, FUSE behavior, dynamic libraries and glibc floor.

**DEB:** validate package identity/dependencies, desktop entry, icons, MIME types and CLI. Test clean install, launch, open/edit/save/export, upgrade, remove, purge and reinstall on supported Debian/Ubuntu hosts. Validate dependency declarations against actual binary requirements.

**Flatpak:** use immutable tag/source reference; validate runtime/SDK, app ID, permissions, portals, file chooser, filesystem access, GPU, fonts, desktop metadata, update and removal. Build and install a single-file bundle in a clean user installation. Flathub submission is a separate review, not the same as providing a bundle.

**RPM:** revisit the existing deferral based on the expanded owner request. If adopted, produce package metadata/dependencies and test install/upgrade/remove on Fedora/RHEL/openSUSE-compatible images. If still deferred, record why and update decision 0070 or successor; README/site must not imply an RPM exists.

**tar.gz:** stable directory layout, binaries, licenses, documentation and required assets, with dependency and PATH instructions.

### 7.6 WASM

Produce a self-contained versioned static ZIP suitable for download/self-hosting, with no developer-machine paths. Document browser support, storage, file picker, drag/drop, clipboard, downloads, WebGPU/WebGL fallback and security restrictions. Test from the extracted ZIP using a local static server and real browser. Run core open/edit/save/export/bridge E2E on release bytes and enforce size/load budgets. Deploy identical release bytes to the separate WASM Pages project and reconcile served assets against the tag. Claim PWA support only after manifest, icons, service worker, offline and update-cache tests pass.

### 7.7 Updates and artifact evidence

Consider a user-disableable update check against public GitHub release metadata, no user/document data, no embedded token, manual check option, and no silent installation/data overwrite. Package-managed Linux updates should direct users to their package manager.

Each artifact progresses independently through:
**Recipe verified → Built → Package inspected → Installed → Launched → Smoke tested → Signed/attested → Published → Downloaded and independently verified.**

Never compress these into a single "supported" status. Every receipt names SHA, version, host, OS, architecture, toolchain, result, warning and date.

---

## 8. Phase 6 — GitHub release automation and publication

**Goal:** publish a coherent release from one exact approved tag.

- Stamp version consistently across workspace/package metadata, CLI, app About dialog, WASM metadata, installers, manifests and release notes.
- Create a protected/annotated tag at the exact reviewed source SHA.
- Build in fresh isolated jobs using pinned toolchains and reviewed workflow/action dependencies.
- Generate per-platform receipts with host, runner, toolchain, architecture, artifact sizes/hashes and test results.
- Aggregate job fails closed unless mandatory assets are present exactly once, non-empty, correctly versioned, from the same tag/SHA, package-inspected and runtime-tested.
- Generate SHA-256 checksums for every release asset, signed checksum manifest, release JSON manifest, SBOM and provenance/attestation.
- Sign Windows binaries/installers; sign/notarize macOS app/DMG; define Linux signature policy and public verification instructions. Secrets stay in protected environments.
- Create draft GitHub Release/prerelease; never auto-publish production.
- Download draft assets from GitHub and independently hash/inspect them; test representative clean installations.
- Review release notes with changes, supported platforms, package choices, install/update/uninstall, known limitations, checksums, signature state and links to merged issues/PRs.
- Verify website downloads and browser release assets.
- Obtain explicit owner go/no-go, publish release and deploy matching website/WASM.
- Independently download public assets again and verify manifest/signature/provenance. Never silently replace bytes on a published tag.

---

## 9. Phase 7 — Audit every GitHub issue, PR and branch

**Goal:** close tickets because work is actually resolved, not to make the issue count look clean.

### 9.1 Initial open-issue inventory

GitHub search for open issues returned 31 results at 2026-11-26. This is a baseline seed, not a final audit.

| Issue | Initial reconciliation focus |
|---|---|
| #1 | ArtCraft-candidates incubation proposal; obtain explicit owner decision |
| #3 | Windows feature request; cross-reference #31/#96 |
| #23 | Optimize panel; old body references PR #118, recheck current behavior/export acceptance |
| #24 | Animation timeline; old body references PR #118, verify GIF order/delay/onion-skin/playback |
| #26 | Phase 22 keymap; verify current parity and command dispatch |
| #27 | Phase 22 toolbar; verify current toolbar and reachability evidence |
| #31 | Windows/macOS release receipts; reconcile granular T27 tickets |
| #36 | CI baked Cargo registry/lockfile regression |
| #37 | GitForge runner failure-path issue; confirm ownership and external dependency |
| #42 | T27 baseline release build receipts; verify receipt completeness |
| #45 | DEB desktop integration and install lifecycle |
| #48 | Old PR #8 task; body says PR closed unmerged and findings moved to other issues |
| #49 | AppImage build |
| #50 | AppImage inspection/launch |
| #51 | AppImage clean-host smoke |
| #52 | Flatpak bundle build |
| #53 | Flatpak sandbox install/runtime smoke |
| #54 | RPM deferral; revisit decision 0070 |
| #55 | Linux ARM64 feasibility/build target |
| #59 | Windows portable ZIP clean-VM smoke |
| #60 | Windows installer upgrade/uninstall/reinstall |
| #61 | Windows ARM64 assessment |
| #62 | macOS DMG script and architecture audit |
| #63 | macOS app bundle/DMG metadata |
| #64 | macOS native install/launch |
| #65 | Android feasibility decision |
| #66 | Android native port, gated by #65 |
| #80 | Quality ratchet, edge tests, missing docs |
| #95 | AVIF decode scope/WASM budget |
| #96 | Windows MSVC build and installer |
| #126 | GitForge CI throughput; confirm whether issue belongs to FireCraft or infrastructure repo |

The open-PR search also returned #186, #187 and #188. Refresh all states/details when work begins.

### 9.2 Issue dispositions

For every open issue, inspect full body, comments, labels, owner, milestone, linked PRs/tasks, current code, tests and decisions. Assign exactly one disposition:
- **Resolved:** merged implementation and acceptance evidence; close completed with commits/tests.
- **Duplicate:** move unique criteria into a canonical issue, then close with link.
- **Superseded:** identify successor issue/PR/task and prove it owns all remaining scope.
- **Not planned:** owner rejects scope; document rationale, close as not planned.
- **Deferred:** valid work but not 1.0; keep open or move to approved backlog with owner/priority/review trigger.
- **Blocked:** keep open with blocker evidence, owner and next action.
- **Active:** retain bounded objective, acceptance, tests and accountable owner.

Do not mass-close. A PR existing, a recipe compiling, a screenshot, or a stale ledger status does not prove resolution.

### 9.3 Priority consistency checks

- #23/#24 still describe earlier PR #118 work while current docs describe portions of Phase 20 as landed; verify every listed criterion before closure.
- #3/#31/#96 overlap Windows request/build/receipt obligations; consolidate without losing user demand or package acceptance.
- #48 refers to PR #8 closed unmerged and findings moved elsewhere; verify replacement issues/receipts.
- #49–#53 form package dependency chains: build → inspect/install → runtime smoke.
- #54 must agree with the new RPM decision.
- #62–#64 separate architecture build, bundle/DMG metadata and native functional tests.
- #65/#66 must remain gated until Android feasibility and scope are explicitly decided.
- #37/#126 may be infrastructure-owned; link/transfer rather than leave external work orphaned.
- #1 requires an owner decision.
- #45 must validate the fixed DEB in a clean environment; old package receipts do not certify new bytes.

### 9.4 Tracker and branch hygiene

Normalize title, labels, priority/platform/status, milestone, assignee, task IDs, acceptance checklists and cross-links. Every open issue needs a current status, owner/next action and review trigger. Update TASKS and its ticket in the same implementation PR. Keep versioned audit history.

For PRs, inspect complete diff, CI, reviews, threads, commits and linked tickets; then merge, request changes, retarget, close with replacement reference or retain intentionally. Resolve review threads only when addressed.

Classify branches as active, integrated, superseded, archived reference, release/tag or abandoned. Delete only after checking unique commits, active worktree/agent ownership and owner approval. Preserve tags and releases. No blanket branch deletion, destructive clean, force-push or indiscriminate worktree pruning.

### 9.5 Exit criteria

Every issue in the final inventory has a documented disposition. Resolved/duplicate/superseded/not-planned tickets are closed with explanation and links. Deferred/blocked/active items remain open with useful next actions. Every PR is integrated, closed with a replacement reference, or intentionally retained. No stale open ticket points to an obsolete PR as if it were in flight. TASKS, tickets, release board and progress docs agree. Commit an issue/PR/branch closeout report.

---

## 10. Phase 8 — In-app feedback and agent improvement loop

Offer Report Bug, Request Feature, Suggest Change, Ask Question/Docs Issue, Known Issues/Release Notes, and offline copy/export diagnostics.

Use a versioned diagnostic envelope such as firecraft-diagnostic/v1. Include only useful approved metadata: FireCraft version/build/channel, native/WASM, OS/distro/architecture, browser/version, runtime/UI version, optional GPU/driver and bounded sanitized logs.

Never include artwork, document content, original file paths/names, project names, usernames, hostnames, machine IDs, location or secrets by default. Screenshots/logs/crash dumps require opt-in, preview, redaction and size limits. Provide an offline path and explain when GitHub issues are public.

Prefer a narrow Cloudflare Worker/GitHub App with minimum Issues permission, payload validation, rate limiting and server-side credentials. Never embed a maintainer token in native/WASM/website clients. If broker is not ready, use copy/prefilled issue fallback.

Flow:
**User report → deduplicate/triage → isolated agent worktree → reproduce → fix/regression test → PR → CI → human review → staging → reporter/cohort acceptance → human-approved production.**

Agents cannot close their own issue, merge their own PR, or promote to production. Desktop rollout uses opt-in beta artifacts/channels, not a fake percentage switch. WASM uses staging then controlled promotion.

---

## 11. Phase 9 — Staging, external validation and release-candidate certification

Separate local development, PR previews, internal integration, public staging, desktop beta/prerelease, and production. Keep credentials, domains, app IDs, update channels and rollback independent.

Target 5–10 external testers across supported platforms where possible. Provide install/quick-start instructions, sample documents, feedback and known limitations. Validate install/update/uninstall, create/open/save/reopen/export, compatibility, performance/stability, discoverability and privacy comprehension.

Stop publication for P0 data loss/corruption, core crash, tag/SHA mismatch, missing artifact, failed install/launch on an advertised platform, incorrect dependency floor, broken upgrade/uninstall, invalid integrity files, WASM deployment mismatch, incorrect website download, critical security/privacy issue, or unresolved P0/P1 without an approved workaround.

P2/P3 limitations may ship only if documented and accepted by the owner.

---

## 12. Phase 10 — Final GitHub 1.0 launch and repository closeout

Before publication:
- README/wiki/docs and site accurately reflect the release.
- GitHub release includes all mandatory assets/integrity files.
- Release notes/changelog and support/known-limitations are complete.
- Issue/PR/branch audit report is committed.
- Website and WASM deployment point to the same published release.
- Contribution, support, security, privacy and installation instructions are live.

Publication:
1. Freeze approved source SHA and version/tag.
2. Run complete tag pipeline.
3. Verify draft release assets and independently download/hash them.
4. Install/test packages on clean target environments.
5. Verify landing page and browser app against release assets.
6. Update artifact matrix, release receipt, README, wiki, site and changelog with actual evidence.
7. Obtain owner approval and publish GitHub Release.
8. Promote production marketing site and WASM; record deployed SHA/hashes/Pages IDs.
9. Verify public release from a clean browser and independent host.
10. Close completed/superseded tickets and PRs; leave deferred items open.
11. Archive superseded experiment/integration work only after owner approval.

Final handoff includes exact source SHA/tag/version, GitHub Release URL, every asset name/size/hash, OS/browser support matrix, CI run IDs, install test environments, Pages URLs/deployment IDs, issue closeout totals, known limitations and remaining deferred work. Keep built, installed, launched, smoke-tested, signed, published and certified states separate.

---

## 13. Master phase exit matrix

| Phase | Deliverable | Exit gate |
|---|---|---|
| 0 — Preserve/audit | Experiment/worktree/branch/PR/issue baseline | No unknown or destroyed work |
| 1 — Consolidate | One repo and architecture | Source map complete; CI/browser green |
| 2 — UI/MVP | Polished editor | Core journeys and regression tests pass |
| 3 — Website | Cloudflare landing page | Deployed staging, acceptance and rollback verified |
| 4 — Docs/wiki | README/docs/wiki/contributor refactor | Links and claims audited |
| 5 — Packaging | WASM/native/Linux/CLI/source/integrity assets | Target-native receipts for all mandatory artifacts |
| 6 — GitHub release | Draft-first release automation | Downloaded assets independently validate |
| 7 — Tracker closeout | Issues/PRs/branches reconciled | Every ticket has intentional disposition |
| 8 — Feedback | Privacy-safe in-app intake | Actionable canonical issues created |
| 9 — Staging | External release candidate | No P0/P1 blocker; all advertised targets pass |
| 10 — Launch | Published 1.0 and clean handoff | Public release verified and rollback recorded |

## 14. Agent operating rules

- Read AGENTS.md and follow its clean-room and quality rules.
- Re-fetch issue/PR/tag/deployment states when starting; this document is a dated snapshot.
- Inspect the experiment path directly. If inaccessible, report a blocker and request an accessible mount/archive.
- Do not delete or rewrite experiment, PR, issue or branch history to make the repository appear tidy.
- Use separate PRs where independent review is valuable.
- Update TASKS and the owning task ticket in the same implementation PR.
- Record exact commands, host, SHA and date for every test/receipt.
- Keep signing, notarization, Cloudflare and GitHub credentials out of the repo.
- Do not claim package/OS/browser support from a recipe or cross-compile alone.
- Stable GitHub Release publication is a human-approved action.

## 15. Baseline observations requiring reconciliation

At the 2026-11-26 plan preparation snapshot:
- README described v0.2.5 as the latest published release, dated 2026-11-03.
- The repository has substantial release engineering documentation, a detailed artifact matrix, a WASM distribution path, Linux x86_64 tar/CLI/DEB packaging, source/SBOM/checksum/manifest machinery and draft-release automation.
- The same docs identify Windows, macOS, AppImage and Flatpak as needing actual target build/runtime receipts; signing/provenance and public deployment reconciliation are known issues. The release guide and artifact matrix contain historical snapshots/rehearsal evidence that must be reconciled to exact workflow runs and public release assets.
- docs/wiki/ has 15 version-controlled pages/partials. The live GitHub Wiki publishing state must be checked independently.
- Open issue search returned 31 issues and open PR search returned #186, #187 and #188.
- The CraftAnything experiment directory was not mounted in the documentation-authoring environment; Phase 0 inspection remains outstanding.

These observations are not completion claims. Replace them with new dated evidence when execution begins.
