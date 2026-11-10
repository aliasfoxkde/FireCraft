FireCraft — Repository re-audit and AI-agent handoff
Fresh audit required
Read-only audit first

The objective is not simply to finish PR #9 or produce a collection of platform build scripts. It is to take FireCraft from its current development state to a reproducible, verifiable, production-ready release process, with evidence for every supported platform.

I recommend treating this as a new engineering engagement with three separate deliverables:
1. Repository audit: Establish the actual state of main, PRs, issues, release assets, CI, and the release plan.
2. Execution task ledger: Create an atomic, dependency-aware task list in the repository, with owners, priorities, acceptance criteria, evidence, and status.
3. Implementation campaign: Work through the tasks systematically, committing changes, running tests, updating the ledger, and producing a final certification report.
The agent must not inherit yesterday's report as authoritative. It should independently inspect the current repository and use the existing report only as historical context. In particular, it must distinguish implemented code, successful builds, target-native verification, and published release artifacts.
1. Fresh audit findings — November 8
Release blocked
Repository state independently queried

The live GitHub repository currently reports:
Item	Verified state
Default branch	main
Main SHA	5fac7a2267ebc6b61432d5302ab6b02a1ff15c0f
Latest push to repository	November 7, 18:59 UTC
PR #9	Open, draft
PR #9 head	eb267611e525461efa1eddf3b2f4273bf097b899
PR #8	Open, ready for review
Latest release	v0.2.5, November 3
Published release assets	5
Open issues	2, excluding PRs

There has been no new repository push or release since the previous report. The current main branch is unchanged.
PR #9 — release engineering
Open PR #9 

The PR's latest commit is eb26761. It includes the release-plan documentation, source archive generation, CycloneDX SBOM generation, workflow improvements, checksum aggregation, and a proposed reduced MVP v1 scope.

The latest available GitHub backup CI run is still failing:
PR #9 CI run #128Job	Result
Rust formatting	Passed
Web formatting	Passed
Full xtask ci-no-fmt gate	Failed
Web bundle smoke	Skipped
CodeQL	Passed

The actual failing gate is Aegis. The web smoke was skipped because the preceding gate failed. The latest PR description still refers to the backup CI as queued, which is stale.

There is another issue: the proposed MVP release lane has not been reconciled with the complete artifact matrix. The plan requires maintainer approval before reducing the target scope.
PR #8 — application work and review
Open PR #8The latest commit is 941e5b6.

The earlier inline review threads have been marked resolved/outdated, but the latest CodeRabbit review still contains eight actionable comments, including a Major data-integrity issue:
- Plotted Pen points can remain in the draft after switching to Polygon Hotspot.
- The subsequent commit can interpret those existing points as a hotspot.
The review explicitly identifies the problem in crates/app/src/editor.rs, around add_draft_point() and commit_draft().

This finding needs to be independently verified and corrected. Resolving review threads is not equivalent to resolving the underlying defect.

The latest backup CI also fails its full gate, and its web smoke is skipped.
Published release verification
Published v0.2.5 releaseThe five actual assets remain:
Asset	Size	SHA-256 prefix
Linux desktop tar.gz	22,084,726 B	707ca850
Linux CLI tar.gz	8,072,377 B	199eb1b8
Linux DEB	14,819,904 B	a81a9ad4
Web ZIP	7,174,826 B	5c1c524a
MANIFEST.sha256	398 B	ce01434c

GitHub reports all five assets as uploaded. Their upload status does not establish that each asset was installed and functionally tested.

The previously identified DEB dependency discrepancy remains a P0 remediation item for the next release. The correction exists on PR #9 but is not part of v0.2.5.
Current work-package assessment
Package	Current assessment
RP-00	Partial — inventory exists; target scope needs approval
RP-01	Partial — DEB smoke passed; AppImage rerun and Flatpak testing outstanding; RPM deferred
RP-02	Partial — GNU cross-build and Wine smoke; no native Windows release certification
RP-03	Not operational — no native macOS DMG
RP-04	Feasibility complete; native Android implementation absent
RP-05	Partial — source/web/CLI work exists; deployment reconciliation outstanding
RP-06	Partial — SBOM generator exists; provenance and signing absent
RP-07	Partial — aggregation implemented; end-to-end publication unverified
RP-08	Not started — no final certification

Audit conclusion: PR #9 should be continued rather than discarded, but it should not be merged simply because the documentation and packaging scaffolding are present. PR #8 also needs its review findings and full gate resolved before its changes are treated as production-ready.

One important distinction: the proposed MVP v1 is a sensible way to get a first stable release out, but it is not the same thing as completing the entire cross-platform campaign. The agent must track those as separate milestones.

2. Recommended repository tracking structure
I would not create another independent task board. FireCraft already has docs/TASKS.md, docs/tasks/, docs/SESSIONS.md, and docs/HISTORY.md.

Instead, have the agent extend the existing system with:
Proposed additions
docs/release/EXECUTION-BOARD.md

Single authoritative release campaign board, RP-00 through RP-08.

docs/tasks/T28-...md, T29-...md, etc.

Atomic task tickets, using the existing ticket format. First inspect task numbering to avoid collisions.

docs/history/release/

Dated implementation sessions, test receipts, decisions, and daily progress.

docs/release/evidence/

Artifact inventories, build receipts, platform verification, hashes, CI links, and certification evidence.

The existing T27 should remain the parent campaign. Child tasks should reference RP identifiers and dependencies.

The important design decision is that the board tracks work, while evidence files prove completion. Neither a checked box nor an agent's completion statement is sufficient evidence.
3. Agent handoff instructions
The following is intended to be copied as the opening instruction in a completely new Claude Code/Codex/other coding-agent session. It is deliberately self-contained and includes repository context, execution phases, atomic tasks, restrictions, verification standards, and reporting requirements.
FireCraft — Cross-Platform Release Engineering Campaign
New-session agent handoff and execution mandate
Repository: https://github.com/aliasfoxkde/FireCraft
Default branch: main
Parent campaign: T27 — Cross-platform release engineering and certification
Primary release plan: docs/RELEASE-ENGINEERING-PLAN.md
Priority: P0 — Release infrastructure
Operating mode: Audit, plan, implement, verify, document, iterate.

1. Mission
Take FireCraft's existing release infrastructure and cross-platform packaging work from its current development state to a reproducible, operational, independently verifiable release process.

This is an execution assignment, not merely a documentation exercise.

You are expected to:
1. Independently audit the repository and establish its actual current state.
2. Inspect existing release plans, task boards, packaging scripts, workflows, code, PRs, issues, and release assets.
3. Build a comprehensive, atomic, dependency-aware task list.
4. Commit the task list and supporting documentation to a dedicated PR.
5. Resolve existing release blockers and review findings.
6. Implement missing functionality and correct defective packaging/workflow code.
7. Build actual artifacts wherever the necessary toolchain is available.
8. Test artifacts on their intended operating systems.
9. Produce evidence for each completed task.
10. Iterate until the approved release scope is operational and certified.
Do not stop after producing a plan. Once the task ledger is established and the initial audit is complete, begin implementation.

Do not claim completion merely because a script exists, a workflow is green, an artifact was uploaded, or a cross-compilation succeeded.
2. Repository context and starting assumptions
The last independently inspected repository state was:
- Main: 5fac7a2267ebc6b61432d5302ab6b02a1ff15c0f
- Published release: v0.2.5
- Latest release commit: 3a4832bc7cd919f10be92248439142d0a702ec1f
- PR #9: docs/cross-platform-release-plan
- PR #9 head: eb267611e525461efa1eddf3b2f4273bf097b899
- PR #8: feat/dock-inspector-parity-matrix
- PR #8 head: 941e5b6d424e1ec2bb7bd706826e66df78e74c05
These are historical starting references, NOT guaranteed current values.

At session start, fetch the latest Git state and verify all references.

Important existing files:
- docs/RELEASE-ENGINEERING-PLAN.md
- docs/tasks/T27-cross-platform-release.md
- docs/TASKS.md
- docs/RELEASING.md
- docs/TOOLCHAINS.md
- docs/release/ARTIFACT-MATRIX.md
- docs/HANDOFF.md
- docs/SESSIONS.md
- docs/HISTORY.md
- .github/workflows/ci.yml
- .github/workflows/release.yml
- xtask/src/dist.rs
- packaging/
Existing packaging mechanisms to audit and extend:
- cargo run -p xtask -- ci
- cargo run -p xtask -- dist --native
- cargo run -p xtask -- dist --web
- cargo run -p xtask -- dist --source
- cargo run -p xtask -- dist --sbom
- packaging/build-appimage.sh
- packaging/build-flatpak.sh
- packaging/flatpak/dev.firecraft.FireCraft.yml
- packaging/build-macos-dmg.sh
- packaging/build-windows.ps1
- packaging/windows/firecraft.iss
- packaging/deb/control
Preserve existing working architecture. Avoid creating parallel release mechanisms without demonstrating why the current implementation cannot be extended.
3. Mandatory operating rules
3.1 Establish a source of truth
Before making changes:
- Fetch main.
- Fetch all open PRs, reviews, comments, and unresolved threads.
- Inspect all relevant commits and compare each PR against current main.
- Inspect all open issues.
- Inspect release tags and published assets.
- Inspect workflow runs, individual jobs, skipped steps, and logs.
- Inspect the actual content of every authoritative release document.
- Inspect the implementation behind every documented capability.
Record the exact commit SHA against which each audit was performed.

Do not rely on previous reports or PR descriptions when actual GitHub evidence is available.
3.2 Task tracking is mandatory
Create a transparent execution ledger before substantial implementation.

Use the existing T27 parent task and assign unique child task identifiers. Inspect existing ticket numbering before assigning new IDs.

Every atomic task must contain:
- Unique ID.
- RP work-package identifier.
- Title.
- Priority: P0/P1/P2.
- Status.
- Objective.
- Exact files/components affected.
- Dependencies.
- Implementation steps.
- Acceptance criteria.
- Verification commands.
- Target environment.
- Evidence links.
- Commit/PR reference.
- Remaining blockers.
- Completion date.
Use these statuses:
- NOT STARTED
- READY
- IN PROGRESS
- BLOCKED
- IMPLEMENTED
- BUILD VERIFIED
- INSTALLED
- LAUNCHED
- SMOKE TESTED
- PUBLISHED
- CERTIFIED
- DEFERRED
- REJECTED
These are distinct states, not interchangeable labels.

A task may be implemented but not operationally complete.
3.3 Atomic task requirements
Break complex work into small tasks that can be completed, reviewed, tested, and committed independently.

A task should ideally cover one logical change and one clear acceptance result.

Examples:

BAD:
- Finish Windows support.
- Complete release automation.
- Implement macOS.
- Finish all Linux packaging.
GOOD:
- Audit Windows build script environment assumptions.
- Verify MSVC target compilation on Windows runner.
- Build portable ZIP from the tagged source SHA.
- Verify ZIP directory layout and executable dependencies.
- Install ZIP on a clean Windows VM.
- Launch desktop application and capture successful startup.
- Exercise open/save/export.
- Verify installer upgrade behavior.
- Verify uninstall and user-data preservation.
- Generate and verify Authenticode signatures.
If a task requires multiple independently testable outcomes, split it into child tasks.

Maintain dependencies between tasks so the board can identify work that is blocked or ready.
3.4 Evidence requirements
Never infer that a task is complete from a script or workflow definition.

For every artifact, track:
1. Recipe exists.
2. Build completed.
3. Artifact produced.
4. Artifact metadata inspected.
5. Checksum generated.
6. Installation completed.
7. Application launched.
8. Functional smoke completed.
9. Security/signature checks completed, where applicable.
10. Artifact published.
11. Downloaded published artifact independently verified.
Record:
- Artifact filename.
- Exact version.
- Source tag and commit SHA.
- Build runner and OS.
- Toolchain version.
- Build workflow URL.
- Artifact size.
- SHA-256.
- Installation environment.
- Test results.
- Limitations.
A green workflow with skipped jobs is not a green release gate.

Cross-compilation is useful build evidence, not target-native runtime certification.

Wine is supplementary compatibility evidence, not native Windows certification.

A successful package upload does not prove the package works.
3.5 Git and collaboration discipline
- Work on dedicated feature branches.
- Do not work directly on main.
- Keep changes narrowly scoped.
- Commit coherent logical units.
- Push changes and create/update PRs.
- Keep task tickets and the execution board synchronized with implementation.
- Do not rewrite unrelated history.
- Do not force-push shared branches without explicit justification.
- Do not silently close or discard existing PRs.
- Avoid parallel agents modifying the same files.
- Before delegating work, define exact file ownership and integration boundaries.
- Rebase or merge changes carefully and rerun all affected tests.
Use descriptive conventional commits.
4. Phase 0 — Repository discovery and baseline audit
Priority: P0. Must complete before major implementation.
P0.1 — Establish repository state
- P0.1.1 Fetch and record current main SHA.
- P0.1.2 Verify workspace version and Rust toolchain.
- P0.1.3 Inspect working tree and uncommitted changes.
- P0.1.4 List local and remote branches.
- P0.1.5 Identify stale and abandoned branches.
- P0.1.6 Inspect current release tags.
- P0.1.7 Inspect every published release and asset.
- P0.1.8 Record source commit associated with each tag.
P0.2 — Audit PRs and issues
- P0.2.1 Inspect PR #9 status, diff, comments, reviews, and CI.
- P0.2.2 Inspect PR #8 status, diff, comments, reviews, and CI.
- P0.2.3 Inspect all other open PRs.
- P0.2.4 Inspect unresolved inline review threads.
- P0.2.5 Verify whether resolved findings are actually fixed.
- P0.2.6 Inspect open issues and related blockers.
- P0.2.7 Identify overlapping branches and conflicting changes.
- P0.2.8 Create a PR disposition plan.
P0.3 — Audit release infrastructure
- P0.3.1 Read the complete release engineering plan.
- P0.3.2 Read T27.
- P0.3.3 Read the live task board.
- P0.3.4 Audit release instructions.
- P0.3.5 Audit toolchain documentation.
- P0.3.6 Audit artifact matrix.
- P0.3.7 Audit xtask distribution implementation.
- P0.3.8 Audit all packaging recipes.
- P0.3.9 Audit CI and release workflows.
- P0.3.10 Audit version stamping and changelog generation.
- P0.3.11 Audit production deployment configuration.
- P0.3.12 Compare documented behavior against actual implementation.
P0.4 — Establish baseline verification
Run the available local checks without changing code:
- P0.4.1 Run formatting checks.
- P0.4.2 Run the canonical xtask ci gate.
- P0.4.3 Run native distribution build.
- P0.4.4 Run web distribution build.
- P0.4.5 Run source archive build.
- P0.4.6 Run SBOM generation.
- P0.4.7 Inspect manifests and checksums.
- P0.4.8 Record exact failures and environmental limitations.
Deliverable: A dated baseline audit with evidence, a current artifact matrix, and a list of verified blockers.
5. Phase 1 — Task ledger and release-scope decisions
Priority: P0.
P1.1 — Create the execution board
- Create docs/release/EXECUTION-BOARD.md.
- Keep T27 as the parent campaign.
- Assign unique child task IDs.
- Link every child task to RP-00 through RP-08.
- Add priorities, dependencies, acceptance criteria, and verification.
- Add a status summary for each RP.
- Add artifact verification columns.
- Add blocker and decision registers.
- Add an explicit distinction between MVP and full cross-platform scope.
P1.2 — Create atomic task tickets
Create individual tickets under docs/tasks/.

At minimum, cover all tasks in the subsequent phases of this document.

Each ticket must be actionable without requiring the next agent to reconstruct the entire conversation.
P1.3 — Establish release-scope authority
The full cross-platform matrix and proposed MVP v1 must not silently conflict.

Prepare a decision record that clearly defines:

MVP candidate:
- Web/WASM.
- Linux x86_64 desktop archive.
- Linux x86_64 DEB.
- Linux x86_64 CLI.
- Source archive.
- Checksums.
- SBOM.
- Verified production web deployment.
- Release certification.
Extended platform campaign:
- Linux AppImage.
- Linux Flatpak.
- Windows.
- macOS.
- Android.
- ARM64 assessments.
- RPM only if formally reinstated.
Record the maintainer's approved scope. Do not treat a recommendation as approval.
P1.4 — Documentation reconciliation
- Update T27 to accurately reflect implementation versus verification.
- Update docs/TASKS.md.
- Update docs/RELEASING.md.
- Update docs/TOOLCHAINS.md.
- Update docs/release/ARTIFACT-MATRIX.md.
- Correct tag/main baseline discrepancies.
- Remove stale CI status claims.
- Ensure all documentation uses the same release-scope authority.
Deliverable: A dedicated documentation/task-ledger PR.
6. Phase 2 — Resolve existing P0 defects and PR blockers
Do not proceed to a new production tag while these defects remain.
P2.1 — PR #9 CI remediation
- Identify the exact Aegis finding causing the latest failure.
- Reproduce the finding locally.
- Determine whether it is a genuine defect or verified false positive.
- Fix genuine findings.
- Add narrowly scoped documented suppressions only for verified false positives.
- Preserve Aegis security coverage.
- Run formatting.
- Run complete xtask ci.
- Verify backup GitHub CI.
- Verify authoritative GitForge CI.
- Ensure web smoke is executed, not skipped.
- Record all run links.
Do not weaken the gate or broaden exclusions merely to achieve green CI.
P2.2 — Release workflow repair
- Audit the exact gh release create invocation.
- Replace invalid -a asset arguments with supported positional filenames.
- Test argument construction independently.
- Test missing asset behavior.
- Test duplicate asset behavior.
- Test filenames containing spaces if supported.
- Verify contents: write is limited to the publishing job.
- Verify draft release creation.
- Verify asset upload.
- Verify post-upload digest validation.
- Verify draft-to-published transition.
- Exercise the process using a safe test tag or disposable release repository.
Do not create or publish a production release during this phase.
P2.3 — DEB dependency correction
- Verify the actual GLIBC symbol requirements of both shipped executables.
- Verify packaging/deb/control declares an adequate minimum GLIBC version.
- Verify the objdump-based check fails when the declared floor is too low.
- Verify the check succeeds when the floor is adequate.
- Build the DEB.
- Inspect package control metadata.
- Install on a clean Debian 12 environment.
- Execute functional CLI round trip.
- Verify reinstall.
- Verify removal and purge.
- Verify no user-data loss.
- Record receipt.
The v0.2.5 DEB is historical and must not be silently replaced.
P2.4 — PR #8 review remediation
- Retrieve the complete latest review findings.
- Compare each finding with the latest code.
- Verify the plotted-draft tool ownership issue.
- Track the originating tool in draft state.
- Reset or explicitly convert drafts on tool changes.
- Add regression tests for Pen-to-Hotspot and reverse switching.
- Verify existing polygon/hotspot behavior.
- Inspect all remaining Major and Critical findings.
- Verify each finding independently.
- Fix valid findings.
- Document rejected findings with reasons.
- Run the complete application test suite.
- Run the full CI gate.
- Verify web build and browser smoke.
- Reconcile task board and documentation.
Keep PR #8 and PR #9 changes isolated unless a dependency requires an explicit integration branch.

Deliverable: Both PRs have accurate dispositions, resolved valid defects, and reproducible verification.
7. Phase 3 — RP-01: Linux distribution completion
Priority: P0 for MVP DEB; P1 for extended formats.
7.1 DEB
- Build package from exact candidate commit.
- Validate architecture metadata.
- Validate dependency metadata.
- Validate desktop entry.
- Validate icon installation.
- Validate MIME integration where implemented.
- Test clean install.
- Test upgrade.
- Test launch.
- Test open/edit/save/export.
- Test uninstall.
- Test reinstall.
- Test CLI separately.
- Record all receipts.
7.2 AppImage
- Audit linuxdeploy invocation.
- Verify desktop entry.
- Verify Icon=firecraft.
- Verify icon file exists in staging.
- Verify executable permissions.
- Verify library bundling.
- Build actual AppImage.
- Inspect architecture and embedded metadata.
- Run with FUSE where supported.
- Run using extract-and-run.
- Install/launch equivalent runtime test.
- Open/edit/save/export.
- Test on clean Linux environment.
- Record checksum and receipt.
7.3 Flatpak
- Audit manifest source URLs and immutable revisions.
- Verify runtime and SDK versions.
- Verify application ID.
- Verify build dependencies.
- Review finish-args.
- Minimize filesystem permissions.
- Implement correct flatpak-builder repository workflow.
- Install the built application into a test installation.
- Correct flatpak run usage; do not pass unsupported --repo arguments.
- Launch using flatpak run.
- Test document portal access.
- Test file open/save/export.
- Test sandbox restrictions.
- Test removal.
- Test on a clean Flatpak-capable Linux host.
- Record checksum and receipt.
7.4 RPM decision
RPM is currently deferred by decision 0070.
- Verify decision 0070 remains appropriate.
- Assess Fedora/RHEL dependency requirements.
- Estimate ongoing package maintenance burden.
- Record whether RPM remains deferred.
- Do not add RPM merely to make the artifact table look complete.
7.5 Linux platform certification
- Test on Debian/Ubuntu.
- Test on Fedora-family environment where applicable.
- Verify X11 and Wayland behavior where supported.
- Verify desktop integration.
- Verify font/resource loading.
- Verify clean install and uninstall.
- Publish a platform-specific compatibility table.
8. Phase 4 — RP-02: Windows production packaging
Priority: P1.

GNU cross-compilation and Wine are supplementary evidence only.
8.1 Build
- Audit build-windows.ps1.
- Audit Inno Setup configuration.
- Verify Windows MSVC target.
- Verify Visual Studio Build Tools.
- Verify native dependency compatibility.
- Build desktop executable.
- Build CLI executable.
- Build portable ZIP.
- Verify stable archive layout.
- Verify required runtime DLLs.
- Verify licenses and resources.
- Build Inno Setup installer.
- Record artifact sizes and SHA-256.
8.2 Native Windows verification
Use a clean Windows VM or native Windows runner.
- Install portable distribution.
- Launch desktop application.
- Open an existing project.
- Create a document.
- Perform basic editing.
- Save and reopen.
- Export PNG.
- Export supported document format.
- Test paths containing spaces.
- Test non-ASCII paths.
- Test user-data directory behavior.
- Test installer installation.
- Test upgrade over previous version.
- Test uninstall.
- Test reinstall.
- Verify user data is handled correctly.
8.3 Signing
- Identify available Authenticode signing infrastructure.
- Document certificate requirements.
- Configure secret names, not secret values.
- Sign production executables and installer.
- Apply trusted timestamping.
- Verify signatures.
- Record signer and verification output.
- Explicitly mark unsigned builds as unsigned.
Do not claim Windows production signing readiness without verified signatures.
9. Phase 5 — RP-03: macOS production packaging
Priority: P1.
9.1 Build
- Audit build-macos-dmg.sh.
- Verify Apple toolchain.
- Verify arm64 Rust target.
- Verify x86_64 Rust target.
- Audit native dependency support for both architectures.
- Build arm64 application.
- Build x86_64 application.
- Determine whether universal binary is feasible.
- Use lipo -info to verify actual binary slices.
- Create correct .app structure.
- Verify Info.plist.
- Verify bundle identifier.
- Verify version metadata.
- Verify icons/resources.
- Build actual DMG.
- Record checksum.
Do not claim universal support simply because the script invokes lipo.
9.2 Native macOS testing
- Install on Apple Silicon.
- Launch application.
- Open/create project.
- Edit/save/export.
- Test file permissions.
- Test moving app outside mounted DMG.
- Test reinstall/upgrade.
- Test uninstall.
- Test Intel macOS separately if supported.
9.3 Signing and notarization
- Identify Developer ID requirements.
- Configure secrets externally.
- Sign application.
- Enable hardened runtime.
- Verify entitlements.
- Submit notarization.
- Retrieve notarization result.
- Staple ticket.
- Run spctl assessment.
- Verify Gatekeeper launch behavior.
Ad-hoc signing is not production notarization.
10. Phase 6 — RP-04: Android feasibility and native support gate
Priority: P2 unless Android is explicitly included in the first release.
10.1 Feasibility
- Re-read Android feasibility report.
- Audit egui integration.
- Audit winit/event-loop dependencies.
- Audit accesskit conflicts.
- Validate Android NDK requirements.
- Validate Android SDK requirements.
- Assess GameActivity integration.
- Assess storage/document picker integration.
- Assess touch interaction.
- Assess lifecycle/resume behavior.
- Define native architecture.
10.2 Native implementation
Only proceed if the feasibility gate passes.
- Create native Android entrypoint.
- Integrate Android application lifecycle.
- Implement storage permissions/document picker.
- Implement touch/input mapping.
- Build arm64-v8a APK.
- Build AAB only if Play distribution is intended.
- Configure application ID and versioning.
- Sign APK/AAB.
- Test installation on physical ARM64 device.
- Test launch.
- Test open/edit/save/export.
- Test background/resume.
- Test orientation and display changes.
- Record evidence.
A WebView or browser-hosted WASM package is not a native Android application.

If infeasible or outside approved scope, document the deferment and keep Android marked experimental.
11. Phase 7 — RP-05: Web, CLI, source and production deployment
Priority: P0 for MVP.
11.1 Web release bytes
- Build web bundle from exact tag SHA.
- Verify WASM optimization.
- Verify bundle-size ceiling.
- Verify version stamping.
- Generate bundle manifest.
- Execute bridge smoke against release bytes.
- Test open/edit/export.
- Record bundle file hashes.
11.2 Production deployment reconciliation
- Identify the actual Cloudflare Pages production project.
- Identify current production deployment ID.
- Retrieve deployed asset inventory.
- Download or otherwise hash deployed assets.
- Compare production files with tagged release files.
- Identify stale or unexpected files.
- Verify appinfo.json version stamping.
- Verify deployment source commit.
- Verify deployment URL.
- Produce a machine-readable reconciliation report.
Do not claim deployment matches release assets based only on a successful deployment command.
11.3 CLI archive
- Build standalone CLI archive.
- Verify binary architecture.
- Verify version/help output.
- Verify import.
- Verify export.
- Verify supported file operations.
- Verify clean extraction.
- Verify checksum.
11.4 Source archive
- Generate archive from exact tagged commit.
- Verify archive contains required source files.
- Verify it excludes build outputs and secrets.
- Extract into clean directory.
- Run cargo metadata --locked.
- Run cargo build --locked.
- Run appropriate tests.
- Verify license files.
- Verify vendored dependencies where applicable.
- Record build prerequisites and receipt.
12. Phase 8 — RP-06: Supply-chain security and workflow hardening
Priority: P0 for MVP metadata; P1 for full provenance/signing.
12.1 Checksums and manifest
- Define canonical asset naming.
- Generate SHA-256 for every asset.
- Generate release-wide checksum list.
- Generate machine-readable release manifest.
- Include exact source commit.
- Include version.
- Include target OS and architecture.
- Include artifact filename and size.
- Include artifact digest.
- Verify manifest entries against downloaded files.
- Test missing and corrupted asset rejection.
12.2 SBOM
- Audit CycloneDX generator.
- Validate package identity.
- Validate source references.
- Validate dependency edges.
- Validate workspace root.
- Generate SBOM from locked dependency graph.
- Validate output against CycloneDX schema.
- Inspect package completeness.
- Compare SBOM against Cargo.lock.
- Publish SBOM.
- Verify downloaded SBOM.
12.3 Provenance and attestations
- Define provenance format.
- Identify supported GitHub attestation mechanism.
- Identify build identity and workflow identity.
- Bind provenance to source SHA and artifact digest.
- Configure least-privilege permissions.
- Generate attestations.
- Verify attestations independently.
- Publish provenance metadata.
12.4 Signatures
- Inventory available signing credentials.
- Document platform-specific signing requirements.
- Configure secret names and access policy.
- Never store secrets in Git.
- Sign eligible artifacts.
- Verify signatures.
- Record unsigned exceptions.
12.5 Workflow hardening
- Pin third-party actions to full commit SHAs.
- Apply least-privilege permissions.
- Separate build and publication jobs.
- Restrict signing permissions.
- Validate workflow inputs.
- Prevent stale artifact reuse.
- Bind all artifacts to one tag SHA.
- Fail on missing outputs.
- Fail on version mismatch.
- Fail on checksum mismatch.
- Fail on unexpected extra artifacts.
- Fail when required platform jobs are skipped.
- Preserve build/test logs.
- Add safe test-release mode.
13. Phase 9 — RP-07: Release aggregation and publication
Priority: P0.
13.1 Aggregator
- Define approved required artifact matrix.
- Compare matrix with actual workflow outputs.
- Verify every required artifact exists.
- Verify artifact names.
- Verify artifact versions.
- Verify source SHA.
- Verify checksums.
- Verify SBOM.
- Verify manifest.
- Verify provenance.
- Verify signatures where required.
- Verify platform test receipts.
- Reject missing or invalid artifacts.
- Reject artifacts from different commits.
- Reject unexpected version mixing.
The aggregator must be fail-closed.

A skipped required platform must fail the release gate, not silently disappear from the results.
13.2 Draft release
- Build all required outputs from one clean tag.
- Collect target-native test receipts.
- Generate release notes.
- Generate checksums.
- Generate release manifest.
- Generate SBOM.
- Generate provenance.
- Verify signatures.
- Create draft release.
- Upload assets.
- Download assets independently.
- Verify hashes.
- Verify release notes and installation instructions.
- Verify all artifact links.
- Record draft release ID.
13.3 Publication
Publishing is a separate final action.

Do not publish until RP-08 certification and maintainer authorization are complete.
14. Phase 10 — RP-08: Final certification
Priority: P0.

Produce a final release certification report containing:
- Exact tag.
- Exact source SHA.
- Exact workspace version.
- Approved support matrix.
- Every published artifact.
- Artifact filename.
- Artifact size.
- SHA-256.
- Build runner.
- Build toolchain.
- Install test evidence.
- Launch test evidence.
- Functional smoke evidence.
- Signature status.
- SBOM status.
- Provenance status.
- Deployment hash reconciliation.
- CI workflow links.
- Known limitations.
- Unsupported targets.
- Residual risks.
- Independent verification results.
The certification must explicitly distinguish:
- Built.
- Installed.
- Launched.
- Smoke-tested.
- Published.
- Independently verified.
A release is not certified if any required target lacks its required evidence.
15. Work-package completion criteria
RP-00
Approved artifact contract, accurate inventory, baseline, and task ledger.
RP-01
Required Linux packages built and verified; deferred formats explicitly documented.
RP-02
Windows native MSVC build, installer, native testing, and signing disposition.
RP-03
macOS DMG, architecture validation, native tests, and signing/notarization disposition.
RP-04
Android feasibility decision and, if approved, native ARM64 APK/AAB with device tests.
RP-05
Web, CLI, source archives, and exact production deployment reconciliation.
RP-06
Checksums, manifest, SBOM, provenance, signatures, and secure workflows.
RP-07
Fail-closed aggregate and successful draft-release publication test.
RP-08
Independent certification for the approved release scope.
16. Priority definitions
P0 — Blocking
- CI failures.
- Broken release workflow.
- DEB compatibility defects.
- Invalid artifact aggregation.
- Missing MVP artifacts.
- Deployment hash mismatch.
- Review findings affecting data integrity or release safety.
- Missing source/version consistency.
P1 — Required for expanded production support
- Windows native builds and installer.
- macOS native builds and notarization.
- AppImage.
- Flatpak.
- Full supply-chain attestations and signatures.
- Additional distro testing.
P2 — Extended scope
- Android native port.
- ARM64 targets.
- RPM reconsideration.
- Additional platform integrations.
Do not lower priority to make the board appear healthier.
17. Daily reporting and session continuity
At the end of every implementation session:
1. Update atomic task statuses.
2. Update RP summaries.
3. Record changed files.
4. Record commits.
5. Record PR links.
6. Record tests run.
7. Record exact test results.
8. Record workflow links.
9. Record artifacts generated.
10. Record remaining blockers.
11. Add a dated session report.
12. Update the main task board.
Use this report format:
Daily engineering report
- Date/time:
- Main SHA:
- Working branch:
- PR:
- Current release readiness:
- Tasks completed:
- Tasks in progress:
- Tasks blocked:
- Files changed:
- Commits:
- Tests executed:
- CI status:
- Artifacts built:
- Artifacts installed:
- Artifacts launched:
- Artifacts smoke-tested:
- Artifacts published:
- Outstanding P0:
- Outstanding P1:
- Outstanding P2:
- Next atomic tasks:
Include a report even when no work has changed.

If a toolchain or platform is unavailable, record that fact and create a concrete alternative plan. Never mark the corresponding target operationally complete.
18. Execution strategy
Work in dependency order.

Do not start by implementing every packaging script simultaneously.

Recommended execution sequence:
1. Phase 0 audit.
2. Phase 1 task ledger and approved scope.
3. Phase 2 CI, DEB, release workflow, and review blockers.
4. Phase 3 Linux MVP.
5. Phase 7 web/CLI/source and deployment reconciliation.
6. Phase 8 metadata and supply-chain security.
7. Phase 9 aggregation and draft-release test.
8. Phase 10 MVP certification.
9. Phase 3 extended Linux packaging.
10. Phase 4 Windows.
11. Phase 5 macOS.
12. Phase 6 Android.
13. Full cross-platform certification.
Adjust the sequence only when an identified dependency justifies it.
19. Required deliverables
At minimum, produce:
1. A current-state audit report.
2. An approved artifact matrix.
3. An atomic release execution board.
4. Individual child task tickets.
5. A dependency graph.
6. A blocker/decision register.
7. Dated session reports.
8. Platform-specific build and test receipts.
9. A working release aggregator.
10. A complete draft-release test.
11. A final certification report.
Create focused PRs and iterate. Do not put unrelated application changes into the release-engineering PR merely because they are convenient.
20. Restrictions and release authorization
You are authorized to inspect, plan, implement, test, commit, push branches, create/update PRs, and generate local test artifacts.

You are NOT authorized to:
- Publish a production release.
- Create or push a production version tag.
- Sign production artifacts.
- Publish production binaries.
- Deploy production Cloudflare Pages.
- Change production secrets or signing credentials.
- Merge PRs without explicit authorization.
Draft/test releases and disposable test tags are permitted only when they cannot be mistaken for production releases and do not overwrite existing tags or releases.

Keep all production publication actions gated for explicit maintainer approval.
21. First-session stopping condition
Do not spend the entire first session rewriting documentation.

Complete the fresh audit, construct the atomic task ledger, commit it to a dedicated branch, and open a focused PR.

Then begin the highest-priority implementation tasks that are unblocked.

At the end of the first session, provide:
1. Verified current repository state.
2. The committed task ledger and PR link.
3. Number of atomic tasks created.
4. P0/P1/P2 task counts.
5. Dependency graph.
6. Confirmed release blockers.
7. First implementation tasks completed.
8. Tests and evidence.
9. Remaining work.
Primary engineering principle: FireCraft's release readiness must be demonstrated by actual artifacts and reproducible target-platform evidence, not inferred from implementation progress.




4. Dependency graph
This is the recommended implementation order. It prevents the agent from trying to certify downstream artifacts before their build and verification infrastructure works.
Release execution dependency graph
1. RP-00 — Audit and artifact contract
   
   Repository baseline, task ledger, approved support matrix.
2. CI and existing blocker remediation
   
   PR #8/#9 findings, Aegis, DEB, release workflow.
3. RP-01 / RP-05 — MVP artifacts
   
   Linux, web, CLI, source, deployment reconciliation.
4. RP-06 — Release integrity
   
   Checksums, manifest, SBOM, provenance and signatures.
5. RP-07 — Aggregation and publication
   
   Fail-closed release pipeline and draft-release verification.
6. RP-08 — MVP certification
   MVP release gate

Extended platform campaign
Linux

AppImage / Flatpak


Windows

MSVC / Installer


macOS

DMG / Notarization


Android

Native APK/AAB


ARM64

Architecture assessment


Certification

Full matrix




GitHub-hosted runners support Linux, Windows, and macOS environments, so native build and test jobs can be implemented without treating cross-compilation as the only option. 

GitHub Docs
 Draft-first publication is also supported by GitHub's release workflow, allowing assets to be assembled and verified before publication. 
GitHub Docs

5. What I would emphasize to the agent
There are five particularly important requirements that I would not compromise on:
1. Do not let the agent declare the campaign finished after fixing PR #9. PR #9 is infrastructure and planning, not the completed release.
2. Do not let it confuse the MVP with the complete cross-platform roadmap. Both need separate progress reporting.
3. Require actual Windows and macOS machines or hosted native runners. A successful GNU cross-build or a generated DMG script is insufficient.
4. Make the task board a living engineering record. Every commit, regression, failed test, decision, and artifact needs to be reflected.
5. Require evidence before task closure. Ideally, a second verification pass should validate completed tasks independently of the agent that implemented them.
One additional recommendation: have the agent maintain a machine-readable artifact manifest and a separate test-receipt format. That will make it possible to automate much of RP-07 and RP-08 instead of relying on manually maintained Markdown assertions.
6. Expected first-session outcome
I would judge the new session by these deliverables:
Deliverable	Expected
Fresh repository audit	Complete
PR #8/#9 disposition	Documented
RP-00–RP-08 breakdown	Complete
Atomic task tickets	Created
Dependency graph	Created
P0/P1/P2 prioritization	Complete
Execution board	Committed
Documentation PR	Open
First P0 implementation	Started
Verification evidence	Recorded

Do not accept a first-session result consisting only of a proposed plan. The agent should leave behind a committed, usable task ledger and begin actual remediation.

My recommended end goal is a working MVP release first, followed by the full cross-platform campaign. That gets FireCraft to a defensible public release without pretending Windows, macOS, and Android are ready before they have been tested.

The handoff above is ready to use as the new session's initial instruction.