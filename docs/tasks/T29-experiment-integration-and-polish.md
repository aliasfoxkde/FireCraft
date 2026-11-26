# T29 — Experiment integration and product polish

**Status:** In progress — separate CraftAnything repository has active uncommitted changes; FireCraft PR #187 is open and required checks fail. The first review binary is rejected by user report: it opened a terminal and exited without a GUI. Verify the repaired startup path before replacing the artifact. Do not merge or prune either line until reconciled.

## Objective

Integrate the useful, independently reviewed GUI and workflow improvements from `FireCraft-CraftAnything-Experiment` into FireCraft, then deliver a Windows review binary and a short feedback loop. Improve the visible editor without regressing the shared engine, WASM build, document integrity or clean-room boundary.

## Inputs and authority

- User's active objective in this conversation.
- FireCraft project policy in [`../../AGENTS.md`](../../AGENTS.md), task ledger [`../TASKS.md`](../TASKS.md), release process [`../HANDOFF.md`](../HANDOFF.md) and current task snapshot.
- Experiment repository: `/nas/Temp/repos/FireCraft-CraftAnything-Experiment`; at audit it had active local edits on `main` after commit `3a09973`.
- Existing FireCraft experiment integration PR [#187](https://github.com/aliasfoxkde/FireCraft/pull/187), plus consolidation PR [#186](https://github.com/aliasfoxkde/FireCraft/pull/186).
- Product-pattern reference: [`../research/bonbon-product-benchmark.md`](../research/bonbon-product-benchmark.md).
- Other Fireworks workflow references: [`../research/emberframe-product-benchmark.md`](../research/emberframe-product-benchmark.md), [`../research/fireworks-command-library-benchmark.md`](../research/fireworks-command-library-benchmark.md) and [`../research/fwlib-extension-benchmark.md`](../research/fwlib-extension-benchmark.md).
- Scripting-console reference: [`../research/fwconsole-benchmark.md`](../research/fwconsole-benchmark.md).
- Legacy scripting behavior reference: [`../research/fwjs-errata-compatibility.md`](../research/fwjs-errata-compatibility.md).

## Work sequence and acceptance

### Gate A — reconcile active changes

- Compare experiment `HEAD`, dirty diff, screenshots, tests and roadmap with FireCraft PR #187 and the current FireCraft branch.
- Produce a file-by-file transfer map: already integrated, candidate to port, superseded, conflicting, or deferred.
- Resolve the duplicate shell lineage between PR #186 and #187; preserve the experiment agent's work until its owner confirms that snapshot is complete.
- Acceptance: one authoritative merge candidate, a clean diff, current task ledger and no unexplained duplicate or generated binaries.

### Gate B — GUI behavior and parity

- Compare every menu and major option with clean-room Fireworks CS6 reference evidence; record behavior, existing FireCraft behavior, discrepancy, priority and owner.
- Reconcile Emberframe's documented limits against FireCraft before creating parity tickets: transforms, Bezier editing, masks/effects, library drag/drop, printing, layer blending, selective JPEG and editable Fireworks PNG layers. Existing FireCraft capabilities need verification, not duplicate implementation.
- Exercise one-click panel collapse/expand, grouping/float/dock/snap, tab canvas-mode focus round-trip, canvas drag/drop, clipboard copy/paste, tool shortcuts, file dialogs, import/export filters and cancel/overwrite paths.
- Review left Tools rail grouping against the provided CS6 panel screenshots; retain a compact, predictable tool hierarchy and keyboard flyouts.
- Verify whether Auto Shapes and Common Library items can be dragged to canvas, whether print remains an explicit gap, and whether JPEG selective quality/smoothing has a real UI path.
- Acceptance: interaction receipts for native and WASM where applicable; every unresolved control has a named task, and no menu/option claims complete without exercising it.

### Gate C — startup and workspace polish

- Deliver the splash as a floating native window without a dark/black parent application window, centered on the active display, large, branded, and lasting roughly 2–3 seconds without a progress bar.
- Refine file dialogs for directory navigation, file-type filters, overwrite and error states, keyboard operation and drag/drop.
- Align fonts, icons, spacing and control sizing; verify on Windows and Linux and on the web build.
- Acceptance: screenshots at documented sizes/themes and an interaction checklist; no placeholder control or misleading progress indication.

### Gate D — Windows review build

- Build from the reconciled FireCraft branch using the repository's release recipe and pinned toolchain; include version/commit provenance and a SHA-256 checksum.
- Place the review artifact under an agreed ignored output path accessible over SMB; do not commit binaries.
- Acceptance: Windows-target build succeeds, archive/executable launches on Windows 11, open/edit/save/export smoke passes, and the user receives the exact file path, commit and hash.
- A cross-compile is insufficient: verify the PE uses the GUI subsystem and exercise splash-to-editor in one native event loop; obtain a Windows 11 launch/edit receipt before calling it reviewed.

### Gate E — stability, performance and public handoff

- Measure startup, canvas frame time, memory, large-document open/save/export and browser bundle size against a documented baseline.
- Triage clipboard, focus, panel, splash and dialog bugs with reproducible steps; add regression coverage at the owning behavior seam.
- Publish the feature list, public roadmap, tutorials and issue-reporting route; keep shipped status separate from planned work.
- Specify an Automation Console over the existing script/command boundary; keep an optional system terminal as a separate security and lifecycle design.
- Acceptance: all agreed quality gates pass, limitations are recorded, and release/deployment claims have receipts.

## Work boundaries

- No destructive branch/worktree cleanup while either checkout has active work.
- No direct copy of BonBon implementation or assets; BonBon's GPL-3.0 codebase is behavioral research only.
- Do not publish or deploy a build during this task without release authorization; a private/local review build is in scope.
- Treat the working experiment repository's roadmap and dirty files as agent-owned until its work is explicitly handed off.

## Verification plan

- Run the repository CI gate and the focused app/browser tests after integration, as required by project policy.
- Run real-browser smoke for any web/shell changes.
- Build and launch the Windows review artifact; record host/OS/date, revision and digest.
- Reconcile `TASKS.md`, this ticket, public roadmap, `FEATURES.md` and `log/devlog.md` in the same landing change.
