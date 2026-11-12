# FireCraft Production Readiness and Engineering Cleanup Plan

> **Purpose:** systematic execution plan for agents to audit, clean up, refactor, test, polish, and prepare FireCraft for dependable public production use.
>
> **Status:** active work order; not a readiness claim.
> **Created:** 2026-11-10.
> **Scope authority:** decision 0075.
> **WASM policy:** decision 0090.
> **Cross-platform release work order:** RELEASE-ENGINEERING-PLAN.md and T27.
> **Live project ledger:** TASKS.md.

This plan is an execution contract. It does not imply that work is complete because a script exists, a branch passes a subset of CI, a cross-compiled binary exists, or an artifact was uploaded. Each agent must inspect the current tree and open PR set before starting, preserve work already in flight, and submit evidence with each handoff.

---

## 1. Mission and definition of production readiness

Prepare FireCraft as a reliable, maintainable editor that users can install, open real documents with, edit safely, save, recover, export, and upgrade without losing work.

Production readiness requires evidence in seven areas:

1. **Data integrity:** document open/edit/save/export, undo/redo, multi-document state, autosave and crash recovery preserve user data.
2. **Format fidelity:** supported Fireworks PNG structures and import/export formats are covered by deterministic corpus, golden, round-trip, and malformed-input tests.
3. **Application usability:** the shell, tools, menus, panels, keyboard navigation, dialogs, scaling, accessibility, and error states behave consistently.
4. **Runtime efficiency:** browser and desktop targets meet measured responsiveness, memory, startup, and distribution-size expectations.
5. **Reproducible builds:** artifacts come from one tagged source SHA with pinned toolchains, canonical names, manifests, and reproducible verification.
6. **Operational trust:** dependency/security scanning, SBOM, provenance, signatures, release authorization, and deployment identity match the claims made to users.
7. **Maintainability:** architecture, code, documentation, issues, and the live task board agree with the implementation.

The project must not trade correctness, document compatibility, security, or debuggability for a smaller binary or a green status badge.

## Long-horizon WebOS platform integration

FireCraft is a first-class app in the ArtCraft WebOS portfolio, not merely
an iframe demo. It also remains an independent product with its own complete
standalone distribution goals: native desktop packages, browser/PWA builds,
CLI/MCP tools, developer-facing libraries/modules, source releases, and
platform-specific packages where validation and maintenance ownership exist.
Prebuilt, standardized artifacts make downstream embedding easier, but
embedding is not the sole purpose of the release program. The near-term
release remains standalone browser/desktop quality; the larger product
contract must preserve a path to shared platform components/libraries,
app-host services, WebOS CLI/MCP routing, AI-native command discovery, and
later native installation.

The expanded delivery matrix and shared-workflow adoption plan are tracked in
[RELEASE-ENGINEERING-PLAN.md](RELEASE-ENGINEERING-PLAN.md), especially its
complete product delivery matrix and RP-09 SDK/interoperability work package.
The companion host-side roadmap is [ArtCraft-WebOS PR #36](https://github.com/aliasfoxkde/ArtCraft-WebOS/pull/36).

The FireCraft-side requirements, security boundary, library strategy, and
cross-repository test matrix are in
[WEBOS-PLATFORM-INTEGRATION.md](WEBOS-PLATFORM-INTEGRATION.md).

**Important rule for WP-01:** optimize build artifacts without deleting
required application capabilities. The native CLI and MCP transports do
not necessarily need to be linked into the browser WASM binary, but the
underlying command catalog, schemas, scripting semantics where required,
and host-callable browser bridge must remain available through the
appropriate adapters. Do not remove app-cli features based only on the
current standalone-browser call graph; measure target-specific linkage and
preserve a path for the WebOS host to discover and invoke FireCraft tools.
Shared source crates do not automatically reduce the aggregate size of
independently compiled Craft WASM apps.

## 2. Current baseline and known work in flight

This is a dated execution baseline, not a substitute for refreshing GitHub
before each session. The current live task ledger and release artifact matrix
are authoritative for task and platform status.

- **Canonical source:** `main` at `b61f198` on 2026-11-11. The latest
  published version remains `v0.2.5` (2026-11-03); the source tree has
  advanced beyond that tag.
- **Editor surface:** 253 registered engine commands; 164 are bound in the
  CS6-visible shell surface (64.82%, latest measurement 2026-11-11). The
  generated DOM matrix is 268 carried / 3 partial / 0 missing of 356 verbs.
  These are separate measurements and neither establishes complete
  end-user GUI parity.
- **Menus and panels:** 2 inert menu rows of 296; eight additional rows
  remain explicitly feature/evidence-gated. The 19.6 inert-menu wiring
  campaign is complete. The CS6 user-visible matrix is published in
  `docs/CS6-PARITY.md`.
- **Recent delivered work:** Phase 21's user-facing deliverables and Phase
  20.1/20.5–20.7 landed in the 2026-11-11 shell-lineage train (#107).
  Optimize/timeline work is in open PR #118; the mask stack is in open
  PR #113. These are not shipped until merged.
- **WASM/release policy:** the web-only Cargo `opt-level=z` and Binaryen
  `wasm-opt=z` path is scoped to Trunk; decision 0090 sets a
  25,000,000-byte investigation warning and 50,000,000-byte hard ceiling.
  The ceiling is a safety boundary, not a target. The 21,608,249-byte
  observation from 2026-11-10 is a dated feature-branch measurement, not
  a new main-branch release receipt.
- **Deployment:** #104 reconciled the deployed JavaScript, Wasm and
  `appinfo` with v0.2.5 release bytes; Cloudflare injects an analytics
  beacon into the served HTML, which explains the remaining HTML hash
  difference. Republish from the canonical release artifact is pending
  (T18).
- **Release certification:** Linux x86_64 desktop/CLI archives, DEB and
  the web bundle have published receipts for v0.2.5. Off-fleet Windows,
  macOS, AppImage and Flatpak recipes are not target-native release
  receipts. No target may be called certified without build, install,
  launch, functional smoke and publication evidence. See
  `docs/release/ARTIFACT-MATRIX.md` and T27.
- **Open integration changes:** #120 replaces deprecated `serde_yaml`
  in the packaging-manifest parser; #122 extends command discovery for
  session-scoped history rewind; #123 reconciles the live docs. Refresh
  their current review and CI state before depending on them.

The live GitHub source, current PR heads, and target receipts outrank this
snapshot. Historical audits should remain dated and clearly identified as
historical; do not silently rewrite their measurements to look current.

## 3. Agent roles and file ownership

Use one integration coordinator and specialist agents with bounded areas of ownership. Multiple agents must not concurrently edit the same shared files.

| Agent role | Ownership | Explicit boundary |
|---|---|---|
| Integration coordinator | branch/PR map, merge order, issue disposition, task ledger, final acceptance | does not concurrently rewrite specialist code |
| WASM size engineer | Trunk build, wasm-opt, Cargo web profile, Wasm profiling, size reports | owns xtask distribution policy, web entry options, generated parity source |
| Rust architecture engineer | crate boundaries, dependency graph, features, code duplication, error handling, memory/allocation hot spots | no behavior-changing format refactor without corpus/golden coverage |
| Application QA engineer | persistence, recovery, multi-document behavior, import/export, undo/redo, stress and regression scenarios | data operations require executable tests, not screenshot-only proof |
| UI/UX engineer | shell, tools, dialogs, panels, keyboard, display scaling, accessibility, responsive host | coordinate shared shell state before editing it |
| Platform/release engineer | Linux/Windows/macOS/Android feasibility, packaging, workflows, aggregation, deployment | no target marked operational without target-native receipts |
| Security/documentation engineer | Aegis, dependency policy, SBOM, provenance, signatures, docs/link/status reconciliation | no suppressing rules or claiming compliance without evidence |

### File ownership

- WASM/release-size owner: xtask/src/dist.rs, xtask/src/util.rs (build invocation helpers only), xtask/src/status.rs, web/index.html, docs/planning/generated/parity.md, decision 0090.
- Release/platform owner: .github/workflows, .gitforge.yml, infrastructure/docker, packaging, release aggregation and deployment configuration.
- Application specialists: assigned crate/module directories only; split work by crate or feature family.
- Integration coordinator: docs/TASKS.md, docs/PRODUCTION-READINESS-PLAN.md, T27 status, artifact matrix, release-plan status, and PR sequencing.
- A generated document is edited at its generator and regenerated; never patch the generated copy independently.

## 4. Work packages and execution sequence

### WP-00 — repository baseline and branch integration (P0)

**Objective:** establish one accurate view of what exists, what is pushed, what is local, and which PRs depend on other PRs.

Tasks:
- Capture current main SHA, origin/main SHA, local branches, uncommitted/staged/untracked files, and local-only commits. Preserve all local changes before synchronization; do not reset or force-push another agent's work.
- Inventory open PRs: base/head branch and SHA, ancestry, mergeability, files changed, reviews, unresolved threads, required checks, skipped jobs, and current workflow attempts.
- Build an explicit dependency graph for stacked PRs. Record merge order and identify stale or superseded branches.
- Compare issue descriptions and task rows with current code and merged commits. Classify each as open-valid, duplicate, fixed-on-branch, fixed-on-main, blocked externally, cannot reproduce, or obsolete.
- Do not close an issue because a PR description says it is fixed. Close only after the fix is on main and its acceptance evidence passes.
- Refresh the artifact matrix from actual releases, workflow artifacts, hashes, platform receipts, and deployment observations.
- Correct contradictory task statuses and stale SHA/version/toolchain references as part of the same integration change.

**Acceptance:** reviewed branch/PR dependency map, current issue-disposition list, source-SHA-stamped artifact inventory, no lost local work, and one owner for each remaining P0/P1 item.

### WP-01 — WASM bundle analysis and size reduction (P0)

**Objective:** reduce shipped bytes and improve loading behavior with evidence. The 50 MB hard ceiling is a last-resort guardrail, not the optimization goal.

#### 1. Reproducible baseline

For one exact source SHA and pinned build toolchain, record:
- raw .wasm bytes and section/function sizes;
- generated JavaScript glue, HTML/CSS, icons/fonts/assets, and total uncompressed bundle bytes;
- gzip and Brotli bytes for every served asset;
- Rust, Cargo, target, Trunk, wasm-bindgen, Binaryen, and optimizer versions;
- build command, environment, cache state, artifact hashes, and CI run;
- cold-cache fetch, compile/instantiate, first frame, interactive-ready time, and warm-load timings;
- representative browser memory and CPU during open/edit/export;
- size-profile reports from a Wasm-aware tool such as Twiggy.

Never compare measurements from different SHAs or toolchains and call the difference an optimization result.

#### 2. Optimize build stages

- Keep the web-only Cargo release profile at opt-level=z unless an A/B comparison proves a better trade-off. This override must remain scoped to the Trunk subprocess; do not make desktop binaries size-optimized by changing the workspace-wide profile without a separate benchmark.
- Keep Binaryen wasm-opt=z for size-focused production builds. Compare it to s and speed-focused modes using same-SHA output, startup, and interaction measurements.
- Test ThinLTO versus fat LTO, codegen-units, debug/name stripping, panic strategy, and other release-profile controls on separate benchmark branches. Adopt only measured wins with acceptable diagnostics and runtime impact.
- Pin Trunk, wasm-bindgen CLI, and Binaryen consistently across GitHub Actions, GitForge, developer setup, and release instructions.
- Keep the exact release-byte browser smoke mandatory. A smaller bundle that fails the bridge or real editing flow is a regression.
- Add a per-asset compressed-size report. Raw Wasm size alone does not describe actual network transfer.

#### 3. Inspect the WASM feature and dependency graph

- Capture Cargo's target-filtered dependency and feature trees for wasm32-unknown-unknown, including duplicate crate versions.
- Walk the retained graph from the web binary through app, app-cli, compose, engine, IO, codecs, serialization, scripting, UI framework, and platform adapters.
- Audit native-only eframe features, especially window-system integrations such as Wayland and X11. The current `web/Cargo.toml` explicitly enables both, while `crates/app/Cargo.toml` also enables both. Prove with the actual `cargo tree --target wasm32-unknown-unknown -e features` and compiler output whether those features contribute anything to Wasm. Remove them from the web feature set only if the eframe/WASM build remains valid and desktop features remain enabled for the native app.
- Audit the current `web/Cargo.toml` dependency on `app-cli`: it does not set `default-features = false`, while `app-cli` defaults to its `scripting` feature, documented as gating native CLI `run-script` and MCP `firecraft.runScript` surfaces. The application shell separately depends on and invokes the `scripting` crate directly for its Run Script UI. Verify the call graph and feature graph, then test whether the browser can use `app-cli` with default features disabled without removing the shell's direct scripting capability. Do not disable it based on manifest inspection alone.
- Make optional dependencies genuinely optional. Browser builds should not retain native dialogs, OS integrations, or CLI functionality unless the browser shell calls them.
- Split codecs and format support only where product requirements allow it. Preserve the supported-format contract and corpus/golden tests.
- Measure embedded fonts and icons. Reduce or subset only after checking glyph coverage for Unicode filenames, symbols, non-Latin text, UI labels, and accessibility text; preserve required licenses and attribution.
- Check duplicated versions and feature unification. Consolidate only when compatible upgrades do not create wider dependency churn.

#### 4. Profile code-level contributors

- Use profiler output to rank the largest functions, data segments, static tables, generated code paths, and dependency contributions.
- Inspect monomorphization, generic adapter layers, excessive inlining, large closure captures, serialization/deserialization copies, formatting tables, error strings, retained command catalogs, and codecs.
- Refactor repeated code only when the compiler emits less code or runtime behavior improves. A generic helper can increase Wasm size through monomorphization; shared source does not guarantee shared machine code.
- Reduce avoidable allocations, clones, intermediate buffers, and duplicated state where profiling confirms their cost.
- Prefer feature/dependency boundaries and dead-code elimination over hand-shortening identifiers.

#### 5. JavaScript minification and obfuscation

- Measure generated JS glue separately. If it is a material share of transfer bytes, evaluate a pinned minifier such as Terser or esbuild and compare raw, gzip, and Brotli output.
- Keep source maps in development. Decide explicitly whether production source maps are published; ensure errors can still be diagnosed.
- Do not shorten Rust local/variable/function names as a Wasm-size strategy: ordinary optimized Wasm does not ship Rust source identifiers as source text. Name/debug sections can be stripped only after validating stack traces and diagnostics.
- Do not apply JavaScript obfuscation by default. It is not a size optimization, may reduce compression efficiency, complicate debugging, and can break runtime behavior.
- Minification must preserve wasm-bindgen imports/exports, Trunk-generated asset loading, WebOS messaging, browser file open/drop/save, and source-map policy.
- Every minifier experiment must pass exact release-byte browser smoke and functional browser tests.

**Acceptance:** same-SHA before/after report, profiler evidence, raw and compressed sizes, startup/interaction metrics, unchanged format/corpus behavior, no skipped browser smoke, and a documented keep/revert decision for every adopted optimization.

### WP-02 — Rust architecture and maintainability cleanup (P1)

**Objective:** remove accidental complexity while preserving format behavior and public contracts.

Tasks:
- Compare real crate dependencies with the intended layer table; identify cycles, overly broad dependencies, redundant wrapper crates, and platform-specific functionality leaking into shared crates.
- Identify dead code, duplicate implementations, stale feature flags, unused abstractions, redundant wrappers, unnecessary trait objects, over-generic APIs, and avoidable code duplication.
- Audit error propagation and recovery: every parse, write, import, export, and persistence failure must be visible and contextual. Avoid panics, silent defaults, partial writes, and success-shaped failure.
- Audit allocations, copying, serialization, and large raster buffers. Use existing performance budgets and representative profiles before optimizing.
- Review document identity and dirty-state handling across multiple tabs and recovery snapshots.
- Preserve unknown nodes, fields, ordering, binary residues, and unsupported structures when the format contract requires round-tripping them.
- Keep refactor PRs small and behavior-preserving. Add a regression fixture before changing a parser/writer or operation semantics.
- Re-run formatting, Clippy, tests, layers, corpus, golden, fuzz, performance, web bridge, and generated-doc gates appropriate to the changed surface.

**Acceptance:** no layer violations, no unexplained baseline changes, existing corpus/golden behavior preserved, changed failure cases tested, and no material size/performance regression without explicit decision.

### WP-03 — product correctness and daily-driver QA (P0/P1)

**Objective:** prove the workflows that users rely on are safe.

Required test scenarios:
- Create/open/import documents; edit vector and raster content; save, close, reopen, and export.
- Undo/redo across tools, selection, paths, filters, layers, tabs, document switches, and failure paths.
- Open files with spaces and Unicode in paths, large dimensions, malformed chunks, truncated streams, unsupported variants, and unknown data.
- Autosave clean and dirty documents; crash/restart; recover; decline recovery; recover from conflicting original and sidecar; preserve original data.
- Multi-document switching, close prompts, tab reorder, save failures, per-document layout and state, and isolation between tabs.
- Clipboard, drag/drop, browser picker/download, file dialogs, recent files, and WebOS bridge.
- Scripting success/failure, malformed scripts, large scripts, exceptions, and cancellation/timeout where applicable.
- Large-document stress: many paths, layers, raster surfaces, filters, undo history, repeated open/close, and repeated export.
- Recovery from disk-full, permission denied, locked files, invalid destination, interrupted write, and process termination during save.

Test at the highest stable layer: compare document models and serialized bytes; use corpus/golden tests for fidelity; use browser automation for browser-specific behavior. Screenshots alone do not prove file operations or recovery.

**Acceptance:** no data loss or cross-document state bleed; critical scenarios pass on each claimed platform; negative cases fail safely; regressions become permanent automated tests.

### WP-04 — UI/UX, accessibility, and platform polish (P1)

- Review first-run layout, default window size, saved layout restore, empty states, status messages, error recovery, menu discoverability, tooltips, context menus, panel resizing, and keyboard focus.
- Verify HiDPI/fractional scale, resize, mixed-DPI monitors, monitor movement, and scale changes while the app is running.
- Audit menu actions and shortcuts. Active controls must work; unsupported actions must be clearly disabled or labeled, not silently inert.
- Review open/save dialogs, drag/drop, clipboard, path encoding, and download behavior.
- Keyboard-test core workflows, menu navigation, focus order, dialog focus return, and screen-reader behavior where the toolkit exposes semantics.
- Keep accessibility limitations explicit. Do not claim conformance from static markup checks alone.
- Exercise web layouts at supported desktop/tablet/mobile viewport sizes; record what is and is not supported.
- Review visual consistency across desktop and web; do not let a screenshot-only pass replace interactive tests.

**Acceptance:** screenshots plus executable interaction receipts, documented supported input/display behavior, no dead primary actions, and accessible keyboard paths for core editing workflows.

### WP-05 — release platform and artifact verification (P0)

Follow RP-00 through RP-08 in RELEASE-ENGINEERING-PLAN.md and T27. Do not create a second release contract.

- Resolve DEB runtime dependency declarations. Test installation on a minimal supported Debian system without manually installing missing GUI libraries first.
- Build/test AppImage and Flatpak on a host with their real tooling; verify install, launch, sandbox permissions, file picker, icon, desktop entry, upgrade, and uninstall.
- Build Windows MSVC artifacts and test portable ZIP/installer on a clean Windows VM; verify signing, update, uninstall, and user-data preservation.
- Build macOS DMG on macOS; verify architecture, moving the app out of the mounted disk image, Gatekeeper, codesign, notarization, staple, open/save/export, update, and uninstall.
- Keep Android behind the native feasibility decision. Require APK install and workflow tests on ARM64 hardware before calling it native. A web wrapper is not a native app.
- RPM remains deferred unless the decision is explicitly reopened.
- Build source archive from the exact tag and perform a clean locked build/test after extraction.
- Validate CycloneDX against the declared schema, including SPDX license expressions.
- Publish one canonical manifest covering all assets, target triple, version, size, SHA-256, source SHA, toolchain identity, and build receipt.
- Add provenance/attestation and signature verification to the level promised by the approved scope.
- Reconcile production Pages hashes with the exact tagged web bundle.
- Run the complete tag workflow, download assets independently, verify checksums and manifest contents, and attach all receipts to the release issue and artifact matrix.

**Acceptance:** every artifact required by the approved scope has been built from the same tag, installed/launched and functionally tested on its target OS, independently downloaded and verified, published with complete metadata, and reflected accurately in release notes.

### WP-06 — security and supply-chain hardening (P0/P1)

- Review every dependency update for changelog, license, MSRV, duplicate versions, feature impact, vulnerability status, and reproducibility.
- Keep Aegis rules armed. Baseline additions require a documented reason and must never be used to silence a valid finding.
- Pin GitHub Actions by immutable commit SHA; automate updates with reviewed PRs.
- Apply least-privilege permissions per workflow job. Disable checkout credential persistence in write-enabled publication jobs when it is not needed.
- Validate SBOM schema and license expressions; check that the component inventory matches the locked graph.
- Generate signed provenance/attestation that identifies the exact source commit, workflow identity, toolchain, dependencies, and output artifacts.
- Keep signing keys, certificates, API tokens, notarization credentials, and store secrets out of source and logs.
- Test signature and attestation verification from a clean machine, not only the generation step.

**Acceptance:** zero unresolved P0 security findings, validated SBOM, attestations and signatures verified independently, and credentials restricted to jobs that require them.

### WP-07 — documentation and issue reconciliation (P0/P1)

One integration coordinator owns the final synchronized update.

- Refresh TASKS, T27, the artifact matrix, release plan, RELEASING, TOOLCHAINS, NEXT, HANDOFF, GAPS, decisions, README, and release notes from the current implementation and evidence.
- Correct MSRV, tool versions, workflow behavior, smoke-skip conditions, build flags, generated sizes, artifact names, release dates, and branch/PR references.
- Label old audits as historical and retain their receipts; move current truth into the live artifact matrix and current decisions.
- Remove duplicate tasks and outdated issue claims. Close issues only after the fix is on main and verified.
- Verify every relative Markdown link, decision ID, issue/PR link, and generated document.
- Generate parity, scorecard, changelog, and other generated docs from their source tool; never hand-edit generated copies.
- Ensure each active issue names the next action, owner, dependency, and acceptance evidence.

**Acceptance:** no contradictory active documentation; current source SHA and statuses are explicit; all local links resolve; every open P0/P1 issue is actionable and non-duplicated.

## 5. Integration protocol

1. Inventory in-flight work before each integration batch. Never reset another agent's branch or discard uncommitted/untracked work.
2. Record PR dependencies and merge order. Land foundational toolchain/configuration changes before PRs that depend on them.
3. Keep one logical change per PR. Do not mix UI feature work, broad Rust refactors, WASM optimization, packaging, and bulk documentation updates unless a change is intrinsically inseparable.
4. Rebase or retarget stacked PRs only after their base is merged; validate the final head SHA.
5. Do not count a green backup workflow when the primary gate is pending, skipped, or failed.
6. Do not allow agents to simultaneously change dist.rs, status.rs, release.yml, the same shell module, TASKS, or the artifact matrix.
7. High-risk changes to file parsing/writing, release publication, signing, and security policy require an independent review.
8. Keep generated binaries, credentials, local caches, Adobe reference material, and unverified artifacts out of Git.
9. Every documentation status update must cite a source commit, CI run, release asset, target-host receipt, or decision record.
10. Do not close a task merely to make the board look clean. Re-scope or defer it with an explicit reason if it is not required for the approved release.

## 6. Required PR handoff

Every agent PR must state:
- Work package and issue IDs.
- Base SHA and exact head SHA.
- Files/modules changed and why.
- Before/after bundle size, compressed size, startup/runtime metrics, or other measurements when relevant.
- Commands, exit codes, CI run URLs, skipped jobs, and target host details.
- New regression tests and affected test suites.
- Risk, migration, rollback, and compatibility notes.
- Updated docs/decision/task paths.
- Artifact names, sizes, SHA-256, target triple, and installation/runtime receipts for platform work.
- Explicitly unverified claims and the next owner/action.

"CI green" without the run and job names is not an acceptable handoff. "Built" is not "installed"; "installed" is not "launched"; "launched" is not "smoke-tested"; and "published" is not "certified".

## 7. Exit criteria

FireCraft is production-ready only when:
- The maintainer-approved release scope is explicit.
- Critical document workflows pass with no data loss, corruption, or cross-document state leakage.
- The WASM optimization work has a same-SHA report and measured improvements or a documented explanation for why a proposed optimization was rejected.
- The 25 MB warning and 50 MB hard ceiling are enforced; the 50 MB cap is not treated as a size target.
- Web release smoke is mandatory and runs on the actual release bytes.
- All artifacts advertised as supported have target-native build, install, launch, functional test, publication, and checksum receipts.
- No platform is declared supported from a recipe, cross-compilation, skipped CI, or upload alone.
- The production website matches the exact release bundle hashes.
- SBOM, manifest, provenance, signatures, and verification work for the selected release scope.
- No unresolved P0 defect threatens document integrity, package installation, publication authenticity, or deployment identity.
- Documentation, issue state, and artifact matrix reflect the exact certified source SHA and remaining limitations.

The 50 MB hard ceiling is not an exit target. Continue reducing bundle size and preserving startup and runtime performance even when the bundle is well below the cap.
