# FireCraft — Live task board

> **Live ledger.** Reconciled against `main` at `b61f198` on 2026-11-11,
> after the shell-lineage train (#107) and the release-engineering train
> (#108, merged into the train as `1884c04`) landed, plus the post-train
> dependency work (#117, #120). Use [`NEXT.md`](NEXT.md)
> for phase detail and [`tasks/`](tasks/) for task tickets. Historical plans
> and session notes are not current status sources.
>
> Working document (not a historical record). Canonical planning:
> [`PLANNING.md`](PLANNING.md) + [`planning/firecraft/ROADMAP.md`](planning/firecraft/ROADMAP.md).
> Per-task tickets: [`tasks/`](tasks/) (one file per task, same
> objective/inputs/authority/stopping-condition/verification shape as the
> milestone tickets). Session-level history: [`SESSIONS.md`](SESSIONS.md);
> arc-level history: [`HISTORY.md`](HISTORY.md).
>
> **Refresh rule:** update this board in the same change as any task
> lands, starts, or is re-scoped. The board mirrors the agent session
> task list; if the two disagree, this file is the authority to reconcile
> from.
>
> **Release-task status vocabulary.** Every release-relevant row below
> uses the cross-platform release campaign's status words
> (campaign §3.2): `NOT STARTED` · `READY` · `IN PROGRESS` · `BLOCKED` ·
> `IMPLEMENTED` · `BUILD VERIFIED` · `INSTALLED` · `LAUNCHED` ·
> `SMOKE TESTED` · `PUBLISHED` · `CERTIFIED` · `DEFERRED` · `REJECTED`.
> These are not interchangeable labels. A row may claim `INSTALLED`,
> `LAUNCHED`, `SMOKE TESTED`, `PUBLISHED` or `CERTIFIED` only when it
> carries a receipt naming what was observed, on what host, and on what
> date. A row with no receipt is `IMPLEMENTED` or `BLOCKED`.
>
> **Scope authority** for release work:
> [decision 0075](decisions/0075-release-scope-mvp.md) — **accepted**
> (approved by Michael Kinney on 2026-11-17), so the approved release
> scope is the MVP candidate scope in that record. **Evidence baseline**
> for release work:
> [`release/PHASE-0-AUDIT.md`](release/PHASE-0-AUDIT.md) (PR #16,
> dated 2026-11-09 — the link resolves once that PR merges).
> Per-artifact evidence state:
> [`release/ARTIFACT-MATRIX.md`](release/ARTIFACT-MATRIX.md).

<!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:02fc8616d93d636ed040496ef1a26a0ee890ba0a52cbd6956e427d86c81c5473 -->

## In flight

Review follow-up: [filter controls and master transform](tasks/filter-controls-review.md) <!-- coderabbit-ai-generated:v1 -->

<!-- coderabbit-ai-generated-section:v1:84b20441c095cd11716da1ee0963a86f9429fc177faf73c2a278afd30da90aff -->

— fixed; focused tests pass; the app suite retains a confirmed baseline markup-test failure.

<!-- /coderabbit-ai-generated:v1 -->

<!-- /coderabbit-ai-generated:v1 -->

| Priority | Task | Status | Detail |
|---|---|---|---|
| P0 | Refresh user-visible CS6 parity matrix | **Done — matrix published** | [T26](tasks/T26-parity-matrix.md) → [`CS6-PARITY.md`](CS6-PARITY.md): every CS6 menu, tool, panel, inspector field set, workflow, import/export surface, canvas interaction, keymap area, and selection state classified with an evidence citation and an owning ticket. The browser-sampling check the ticket named was performed on 2026-11-04 against a fresh optimized bundle and is recorded in T22. |
| P0 | Phase 22: CS6 shell fidelity and screenshot audit | In progress | [T22](tasks/T22-screenshot-ui.md) — screenshot acceptance is complete; 22.1 panel inventory and 22.2 tool strip landed on `main` with the shell-lineage train (#107); 22.5 chrome remains partial, and 22.4 still has evidence-gated bitmap transform/fill-stroke work. [Phase 22](NEXT.md#phase-22--cs6-shell-fidelity-p0-the-looks-and-acts-like-fireworks-phase) |
| P1 | Phase 20: daily-driver editing features | **In progress — 20.1–20.3, 20.5–20.7 landed; 20.4 partial** | In-canvas text editing landed 2026-11-06 ([T20](tasks/T20-text-editing.md)); multi-document tabs (20.5), history rewind (20.6), and slice/hotspot behavior editing with HTML preview (20.7) landed 2026-11-11 in #107; the Optimize panel (20.2, [T7](tasks/T7-optimize-panel.md)) and the animation timeline (20.3, [T8](tasks/T8-animation-timeline.md)) landed 2026-11-11 in #118; the mask stack (20.4, [T9](tasks/T9-mask-stack.md)) landed in #113 as `PARTIAL` — the stack, its fold, and the level panel with per-level link/unlink plus the add/remove/move level affordances (`mask.addLevel`/`mask.removeLevel`/`mask.moveLevel`, all dispatched from the panel); what remains open is the screenshot acceptance and the compositor's mask sampling resolution ([#114](https://github.com/aliasfoxkde/FireCraft/issues/114)). [details](NEXT.md#phase-20--daily-driver-features-p1) |
| P1 | Phase 21: modern differentiators | **Done — all items landed 2026-11-11** | Autosave/crash recovery, HiDPI, command palette v2 (fuzzy scoring, recents, Save Steps As Command), preferences, accesskit accessibility and in-app scripting (`.jsf`/`.fwsteps`) all landed with the shell-lineage train (#107, PR #15); [details](NEXT.md#phase-21--modern-differentiators) |
| P2 | Alternative keymap sets and the published shortcut table | **In progress — implemented on `feat/keymap-sets`, CI receipt pending** | Issue [#92](https://github.com/aliasfoxkde/FireCraft/issues/92) (ticket 21.4): the Illustrator, Photoshop, FreeHand and Web Standard sets are selectable in Edit ▸ Preferences ▸ Keyboard and ride the persisted layout (`ShellLayout::keymap`), and the whole binding table is published as [`SHORTCUTS.md`](SHORTCUTS.md) — generated by `xtask shortcuts` from the tables the shell dispatches from and gated in CI, so it cannot drift from the code. A set is a set of overrides on the CS6 table, not a second copy of it: CS6 stays the default and byte-identical, and one keystroke binds one verb. Decision [0126](decisions/0126-the-keymap-sets-are-overrides-of-the-cs6-table.md). |
| P1 | Reconcile live web deployment | **Measured — the live site is not the v0.2.5 release bytes** | Measured 2026-11-11 by digest: the live bundle's trunk asset name (`2274ac8c2925b61d`) and wasm digest (`f4f0a3f0…`) match neither the published v0.2.5 release asset (`f9dbe2f14305b2d9` / `a8a4fa4f…`) nor a rebuild of the v0.2.5 tag (`103c62d3366aef17` / `5934702f…`), so the live build corresponds to no identifiable release and its source revision is unknown. The earlier reconciliation (#104) inferred a match from equal file *sizes* and an identical `appinfo.json`; the digest measurement falsifies it. The served `index.html` difference is explained (substituted asset names plus the Cloudflare analytics beacon). **What would close it:** deploy a bundle whose asset name and wasm digest are recorded here, from a named commit — a production publish, gated for explicit maintainer approval. See [T18](tasks/T18-release.md). |
| P1 | Surface gap: the carried commands the shell never binds | **Landed — 0 of 85 left; 257/257 (100%)** | Six issues, one per family: [#136](https://github.com/aliasfoxkde/FireCraft/issues/136) text, [#137](https://github.com/aliasfoxkde/FireCraft/issues/137) symbol, [#138](https://github.com/aliasfoxkde/FireCraft/issues/138) selection + filter, [#139](https://github.com/aliasfoxkde/FireCraft/issues/139) page/cell/document/frame, [#140](https://github.com/aliasfoxkde/FireCraft/issues/140) mask/bitmap/layer/object/path/style, [#141](https://github.com/aliasfoxkde/FireCraft/issues/141) hotspot/slice/primitive/autoshape/clipboard. Measured 2026-11-14 by the reachability gate (`xtask/src/reachability.rs`): **257 of 257 registry commands bound (100%), 0 unreachable** — the last eight (`document.deleteAllInDocument`; `object.resize`, `object.setGroupType`, `path.cropSelection`, `path.warp`; `autoshape.redraw`, `hotspot.appendPoint`, `slice.appendPoint`) closed on the surfaces decision [0117](decisions/0117-the-slices-panel-moves-the-web-object-and-the-style-panel-updates.md) named, recorded in [0118](decisions/0118-the-last-eight-verbs-reach-their-named-surfaces.md); the floor was re-blessed to 100%. #138–#141 are closed with measured evidence (2026-11-16); #136 (text paragraph layout) and #137 (symbol.setParam renderer consumer) stay open on their renderer-honours criteria — progress comments 2026-11-16. [detail](GAPS.md#16-the-surface-gap--85-carried-commands-the-shell-never-binds-2026-11-12) |
| P2 | Next-release asset breadth and platform verification | Open | [T23](tasks/T23-release-assets.md); recipes exist, but non-Linux artifacts need real platform build/launch receipts. |
| P2 | Coverage and test quality ratchet | Open | Replace the old aspirational 99% label with measured per-crate targets and snapshots; [NEXT.md](NEXT.md). |
| P2 | Phase 22.1: the dockable panel set — docking, tab groups, layout persistence | Complete — on `main` | [issue #91](https://github.com/aliasfoxkde/FireCraft/issues/91) (closed by the merge). Docking (a panel moves between the left and right columns, by drag or by the header arrow), tab groups (a panel dropped on another panel's title shares its slot — the group is one unit that the dock draws, moves and saves), and their persistence landed 2026-11-11 in PR #128, merged 2026-11-14. |
| P2 | Aegis gate: content-keyed baseline, so a doc edit no longer fails the gate | **Done — landed on `main`** | [decisions/0074-aegis-content-keyed-baseline.md](decisions/0074-aegis-content-keyed-baseline.md). The baseline keys on content (pattern, file, digest) rather than on line, and every rule class stays armed. |
| P1 | Close the measured reachability gap — bind the engine commands the shell never reached | **Landed — PR #143 merged 2026-11-14; wave-2 merged 2026-11-16 (#150)** | The reachability gate measured 85 registry commands the engine could execute and the shell's designed surface could not. The branch binds seventy-six of them, family by family, with no menu row added: the Select verbs go through the engine's selection door ([0104](decisions/0104-the-shells-selection-verbs-go-through-the-engine.md)), the Pages panel renames and deletes pages ([0105](decisions/0105-the-pages-panel-renames-and-deletes-pages.md)), the Styles panel redefines and scales styles ([0106](decisions/0106-the-styles-panel-redefines-and-scales-styles.md)), the States panel reorders the states and fans the selection onto them ([0107](decisions/0107-the-states-panel-reorders-and-fans-the-selection.md)), the Properties panel writes the text object's paragraph layout and fits its box ([0108](decisions/0108-the-properties-panel-writes-the-text-layout.md)), its rendering dials and run list ([0109](decisions/0109-the-text-properties-write-the-rendering-dials.md)), the Pages panel lists the guides while the Layers panel moves the selection ([0110](decisions/0110-the-guides-and-the-layer-move-reach-the-engine.md)), and the symbol instance's panel carries the whole symbol family — the button's flags and options, the animation span, the parameters and custom data, the definition's contents ([0111](decisions/0111-the-symbol-panel-carries-the-symbol-family.md)) — while the text panel carries the markup verbs and the button carries its text ([0112](decisions/0112-the-markup-verbs-and-the-button-text.md)), and the Filters panel bakes and renames ([0113](decisions/0113-the-filters-panel-bakes-and-renames.md)), and the Properties panel carries the element mask ([0114](decisions/0114-the-properties-panel-carries-the-mask.md)), and the Pages panel marks the master page while the Layers panel shares layers ([0115](decisions/0115-the-pages-panel-marks-the-master-and-the-layers-panel-shares.md)), and the panel edits the rectangle primitive, the bitmap's pixel mask, and the pattern fill ([0116](decisions/0116-the-panel-edits-the-rectangle-the-pixel-mask-and-the-pattern.md)), and the Slices panel moves the web object and edits its background, the Styles panel updates the named style, and the Properties panel pastes a channel ([0117](decisions/0117-the-slices-panel-moves-the-web-object-and-the-style-panel-updates.md)). The measurement moves from 168 bound commands of 253 (66.4 %) to **249 of 257 (96.89 %) on `main`** (the wave-closeout re-bless), and wave-2 (#150, merged 2026-11-16) closed the remaining eight — **257 of 257 (100 %), 0 unreachable**, floor re-blessed to 100%, surfaces recorded in [0118](decisions/0118-the-last-eight-verbs-reach-their-named-surfaces.md). Issues #138–#141 are closed with measured evidence; #136 and #137 stay open on renderer-honours criteria (progress comments 2026-11-16). |

### Release & tooling lanes in flight


| # | Task | Ticket | Status | Owner | Next action |
|---|---|---|---|---|---|
| 5 | 19.6 inert-menu wiring campaign | [tasks/T5-inert-burndown.md](tasks/T5-inert-burndown.md) | `DONE — 2026-11-11` | UI/verb lane | Batches 1–17 are complete. The current pinned census is 2 inert rows of 298; eight other rows remain explicitly feature/evidence-gated and are not part of an active wiring campaign (`menus::tests::inert_count_never_rises`, `menus::tests::menu_item_total_is_pinned`). |
| 194 | Aegis gate: content-keyed baseline, so a doc edit no longer fails the gate (decision 0074) | [decisions/0074-aegis-content-keyed-baseline.md](decisions/0074-aegis-content-keyed-baseline.md) | `IMPLEMENTED` — the content-keyed gate and its decision record are in the tree; the gate has been exercised on this branch's tree | CI/tooling lane | **landed in `main` (#18)** and merged here; attach a run id before claiming the fleet-wide state |
| — | CI false red: a job compiles against an `app` artifact built from another tree (`E0425: cannot find function 'menu_census' in module 'app::menus'`) | [HANDOFF §6.1](HANDOFF.md#61-pipeline) | `RESOLVED — the mechanism is identified and the mitigation is in the tree` — the mechanism is the cache's **scope** (decision [0122](decisions/0122-the-cargo-cache-is-scoped-per-job-not-per-commit.md)): the pipeline's per-job `cache:` mount is not scoped to the commit, so cargo can reuse another branch's `app` rlib. `scripts/ci_refresh_stale_crates.sh` now runs before any job compiles, and refreshes only the crates whose sources are newer than the cached copy; its own self-test lays the mtimes out and proves the selection. The last recurrence of this signature was the `layers` job of run `5c64a539` (2026-11-16T15:05:32Z); the mitigation landed in `702ad246` (2026-11-17T02:29Z). **Since then the store records 44 runs and 504 jobs with no recurrence.** The one later `E0425` is a *different* signature — `cannot find function 'stated_tag_names_version' in this scope` in the xtask manifest test (the `clippy-test` job of `c396b18a`, 2026-11-17T22:08:03Z), a genuine source fault, fixed by `d1029347` ("the manifest tests import the helpers they exercise"); it is not this class | CI/tooling lane — the root cause is the runner's cache scope (GitForge CI); the mitigation lives in this repository (`.gitforge.yml` + `scripts/ci_refresh_stale_crates.sh`) | closed — the mechanism is identified, the mitigation is in the tree and measured, and no recurrence followed; a future run that fails with this signature is a new defect, not this one |
| — | Release workflow hardening: every action pinned to a commit SHA, a fail-closed pin check in CI (decision 0099), a classified release mode with a fail-closed aggregate gate (decision 0100), a pinned measuring toolchain (decision 0102) | [decisions/0099-workflow-actions-are-pinned.md](decisions/0099-workflow-actions-are-pinned.md) · [decisions/0100-release-mode-classification-and-a-tested-release-gate.md](decisions/0100-release-mode-classification-and-a-tested-release-gate.md) · [decisions/0102-coverage-floor-is-measured-with-a-pinned-toolchain.md](decisions/0102-coverage-floor-is-measured-with-a-pinned-toolchain.md) | `IMPLEMENTED` and **rehearsed end to end** — the 19 action references are pinned; the aggregate gate is a tested script that refuses a missing, empty, extra, mismatched or platform-less artifact set; a rehearsal is a `TEST`-titled prerelease on a disposable tag; the validator and the three release self-tests pass locally. The CI receipt exists: `workflow_dispatch` runs **44188107435** (`v0.2.5-dryrun.6`, head `c396b18a`) completed `success`, produced draft 449279892 with nine uploaded assets, and passed the aggregate gate and the mode classifier — [receipt](release/RECEIPT-0.2.5-DRYRUN-6.md) | CI/tooling lane | publish the production release from a real tag, which needs explicit maintainer authorization (campaign §20); until then, the rehearsal drafts and their disposable tags stay on the remote and their removal is maintainer-gated |
| — | The coverage gate explains itself: the per-file breakdown, each file's `DA` record count beside its `LF`, and the profile inventory (decision 0103) | [decisions/0103-coverage-export-counts-a-file-once-per-build.md](decisions/0103-coverage-export-counts-a-file-once-per-build.md) | `IMPLEMENTED` on the branch; the CI receipt is pending | CI/tooling lane | land it on both receipts; the check that closes the decision is the next pinned run, where `LF` must equal the `DA` count again for `crates/app/src/shell/mod.rs` — under rustc 1.100.0 the export reported 8138 lines for a file that has 6309 |
| 27 | Cross-platform release engineering: Windows, macOS, Linux package matrix, Android feasibility, source, SBOM/provenance, signing, CI aggregation and certification | [tasks/T27-cross-platform-release.md](tasks/T27-cross-platform-release.md) · the atomic ledger is [release/EXECUTION-BOARD.md](release/EXECUTION-BOARD.md) (62 rows, one per `docs/tasks/T27-*.md`) | `IN PROGRESS` — implementation is ahead of verification; **no target is `CERTIFIED`** and nothing is signed; the SBOM half is build-verified against the vendored CycloneDX 1.5 schema and compared against the lockfile's own inventory (decision [0119](decisions/0119-the-sbom-is-validated-against-the-schema-it-claims.md)); the **RP-07 receipt now exists** — the end-to-end tag run of the repaired workflow is run **44188107435** (`v0.2.5-dryrun.6`), which produced draft 449279892 with nine assets, ran the web bridge smoke green, passed the DEB's eight-stage package verification in a clean Debian 12 container, and had three of its assets independently re-verified from the draft ([receipt](release/RECEIPT-0.2.5-DRYRUN-6.md)); the scope that can honestly be certified is written up in [release/CERTIFICATION.md](release/CERTIFICATION.md) | release-workflow lane | publish the production release from a real tag — maintainer-authorized only (campaign §20); consolidate the duplicated `dist --source`/`--sbom` verbs (B-3); the board ledger was reconciled on 2026-11-16 (T27-59) and again on 2026-11-17 — 62 rows against 62 tickets, no problems, with eleven rows advanced by the rehearsals (T27-19/21/49/50/51/52/53/54/55/57/60) and a twelfth (T27-25) advanced by the green CI on the head `f710c524` — the full gate, the application test suite, the web build and the browser smoke all pass there; PR #8's undispositioned comments are adjudicated (T27-24) and the one valid application defect among them — a plotted draft carrying no owning tool — is fixed on `main` by `d3e45cf9` (T27-23, PR #171), and the Aegis scan now runs on documentation-only changes in both CI systems (T27-62, PR #172 → `7bcf28fd`) |
| 26 | Refresh user-visible CS6 parity matrix | [tasks/T26-parity-matrix.md](tasks/T26-parity-matrix.md) | `IMPLEMENTED` — matrix published | parity lane | every CS6 menu, tool, panel, inspector field set, workflow, import/export surface, canvas interaction, keymap area, and selection state is classified with an evidence citation and an owning ticket in [`CS6-PARITY.md`](CS6-PARITY.md); the browser-sampling check the ticket named was performed on 2026-11-04 against a fresh optimized bundle and is recorded in T22 |
| 22 | Phase 22: CS6 shell fidelity and screenshot audit | [tasks/T22-screenshot-ui.md](tasks/T22-screenshot-ui.md) | `IN PROGRESS` | UI lane | screenshot acceptance is complete; 22.1 panel inventory and 22.2 tool strip landed on `main` with the shell-lineage train (#107); 22.5 chrome remains partial, and 22.4 still has evidence-gated bitmap transform/fill-stroke work. [Phase 22](NEXT.md#phase-22--cs6-shell-fidelity-p0-the-looks-and-acts-like-fireworks-phase) |
| 20 | 20.1 In-canvas text editing (egui TextEdit overlay; kittest focus tests) | [tasks/T20-text-editing.md](tasks/T20-text-editing.md) | `IMPLEMENTED` — landed 2026-11-06 (`shell::begin_text_edit` / `text_edit_overlay`) | editing lane | Phase 20's other items are tracked in the rows above and in the Phase 20 board below |
| 8 | 20.3 Animation timeline (frames strip, per-state timing, onion skinning, `symbol.tween`) | [tasks/T8-animation-timeline.md](tasks/T8-animation-timeline.md) | `IMPLEMENTED` — the shell half landed 2026-11-11 (decision [0094](decisions/0094-animation-timeline-shell.md)): the States panel is the strip, the delay field writes `frame.setDuration`, the onion toggle writes `page.setOnionSkinning` and the canvas calls `compose::render_onion`; playback landed 2026-11-12 (decision [0096](decisions/0096-animation-playback-clock.md)): the panel has CS6's `Play Animation` as a clock over elapsed-time data, so the view follows the playhead and the timing is tested without sleeping | editing lane | state thumbnails, a document-level loop count (CS6's `{loopCount:%Ai}`), and the screenshot acceptance are still open — see the ticket |
| 7 | 20.2 Optimize panel (palette/bitrate over `ExportOptions` + live preview) | [tasks/T7-optimize-panel.md](tasks/T7-optimize-panel.md) | `IMPLEMENTED` — landed 2026-11-11 (decision [0095](decisions/0095-optimize-panel-honors-only-what-export-honors.md)): the controls are the fields the export honors, the preview encodes the page through the export's own `io::expv` resolution, and PNG's Adam7 interlace is disclosed and clearable | editing lane | Adam7 interlacing, CS6's selective-JPEG quality table, and the screenshot acceptance are still open — see the ticket |
| — | 20.4 Mask stack UI (add/remove/mode/link/move) | [tasks/T9-mask-stack.md](tasks/T9-mask-stack.md) | `PARTIAL` — the stack, its fold, and the level panel landed in #113 (decisions [0092](decisions/0092-mask-stack-combine-mode.md) / [0093](decisions/0093-mask-stack-levels-and-folding.md)): the model, the FWMap persistence, the compositor fold, and the Properties-panel level list with per-level link/unlink | editing lane | add/remove/move landed with #113 (`mask.addLevel`/`mask.removeLevel`/`mask.moveLevel` dispatched from the level panel); the screenshot acceptance and the compositor's mask sampling resolution remain — see the ticket and [#114](https://github.com/aliasfoxkde/FireCraft/issues/114) |
| — | CI throughput: a warm Cargo cache per job, so a pipeline run starts warm instead of rebuilding the world | [decisions/0098-per-job-cargo-cache.md](decisions/0098-per-job-cargo-cache.md) | `IN PROGRESS` — the per-job `cache:` blocks, the offline validator and the validator's CI step are in the tree on this branch; no run has yet measured the warm cost | CI/tooling lane | land the change, then read the first warm run's per-job durations from the authoritative store before claiming a throughput win; the cache's own growth needs a prune policy (recorded as debt in decision 0098) |
| — | The repository pins the toolchain it measures with: a root `rust-toolchain.toml` at the compiler the coverage floors were measured with (decision 0128) | [decisions/0128-the-repository-pins-the-toolchain-it-measures-with.md](decisions/0128-the-repository-pins-the-toolchain-it-measures-with.md) | `IMPLEMENTED` — the file pins `channel = "1.99.1"`, the compiler the workflows install and the CI image carries; `rustc --version` in the tree now reads 1.99.1 instead of the box's floating default | CI/tooling lane | land it and attach the run id; the pin moves with the workflows' `toolchain:` in one change, and the coverage floor is re-blessed only when the measurement itself moves ([#146](https://github.com/aliasfoxkde/FireCraft/issues/146), decision [0102](decisions/0102-coverage-floor-is-measured-with-a-pinned-toolchain.md)) |


## Phase 20 — remaining work (open PRs)


| # | Task | Ticket |
|---|---|---|
| — | 20.4 Mask stack UI (add/remove/mode/link/move) — `PARTIAL`: the stack, its fold, and the level panel landed in #113 (decisions 0092/0093) with the level add/remove/move affordances; the screenshot acceptance and the compositor's mask sampling resolution remain ([#114](https://github.com/aliasfoxkde/FireCraft/issues/114)) — a FireCraft extension, not a CS6 parity claim | [tasks/T9-mask-stack.md](tasks/T9-mask-stack.md) |
| — | 20.7 Slice behaviors + HTML preview — **the export-level receipt landed** (`crates/app-cli/tests/web_behavior_export.rs`): a behavior attached through the engine's command surface reaches the exported HTML as the `MM_*` markup, and the file it points at is one the export wrote. Writing the regression exposed a real defect — the model stores the *name* of the slice a swap points at, while the exporter emitted that name where the runtime needs a *file* — fixed by resolving the name to the exported file at export time ([decision 0133](decisions/0133-a-behavior-target-names-a-slice-and-the-exporter-resolves-the-file.md)). The preview's own widgets are now asserted through egui's own harness (`crates/app/src/shell/html_preview.rs`, `the_preview_draws_its_widgets_through_the_harness`): the panel is drawn into the accessibility tree and its labels are queried as widgets. The image-snapshot half of that coverage needs egui_kittest's `wgpu` feature and a verified Vulkan adapter, which this host and the CI image do not carry — that half is the queued item 21 below ([#93](https://github.com/aliasfoxkde/FireCraft/issues/93) stays open on it) | — |

20.1 and 20.5–20.7 are already on `main`; do not reopen them.

## Phase 21 — delivered modern differentiators

**All Phase 21 items landed 2026-11-11** with the shell-lineage train
(#107, PR #15): autosave/crash recovery, HiDPI, command palette v2
(fuzzy scoring, recents, Save Steps As Command), preferences, accesskit
accessibility, and in-app scripting (`.jsf` and `.fwsteps` from the
Commands menu). Scope detail stays in
[NEXT.md](NEXT.md#phase-21--modern-differentiators).

## Queued (release + quality)

| # | Task | Ticket | Status | Owner | Next action |
|---|---|---|---|---|---|
| 23 | Broaden release assets to photocraft breadth (dmg, AppImage, Flatpak, win installer) | [tasks/T23-release-assets.md](tasks/T23-release-assets.md) | `IMPLEMENTED` for the recipe set, `NOT STARTED` for every off-fleet artifact — the AppImage, Flatpak, Windows and macOS recipes exist and are parse-checked (or existence-checked) by the gate, but **none has ever been produced**: `linuxdeploy`, `flatpak`/`flatpak-builder`, `pwsh`/`ISCC`, and `lipo`/`hdiutil`/Xcode CLT are all absent from the fleet (B-8, B-11, B-12) | release-workflow lane | either provision a host with each SDK and produce the artifact, or keep the recipes recipe-only and let the scope decision in 0075 exclude those targets — do not advertise them as supported |
| 19 | Quality ratchet: coverage → 99%, `deny(missing_docs)` on geom/color, WCAG AAA | — | `NOT STARTED` | quality lane | schedule after the release lane; the coverage floor is a workspace-wide change |
| 21 | egui_kittest snapshot gates per panel/toolbar | — | `NOT STARTED` | quality lane | needs a stable per-panel harness |
| 22 | Screenshot-driven frontend audit + systematic refinement | — | `IN PROGRESS` | frontend lane | audit against the CS6 shell-parity directive; record screenshots per panel |
| 25 | Responsive/adaptive layout audit + fixes | — | `NOT STARTED` | frontend lane | follows the responsive audit in the current UI work |

### Production readiness and T27 work-package detail

The cross-cutting cleanup, WASM optimization, QA, UI polish, agent ownership, and integration sequence are defined in [PRODUCTION-READINESS-PLAN.md](PRODUCTION-READINESS-PLAN.md).


The full implementation-versus-verification table for T27 — per work
package, the implementation state, the verification state, the owner and
the next action — lives in
[`tasks/T27-cross-platform-release.md`](tasks/T27-cross-platform-release.md).
The per-artifact evidence state (one row per artifact, each with a status
word, an evidence-ladder rung and a receipt) lives in
[`release/ARTIFACT-MATRIX.md`](release/ARTIFACT-MATRIX.md). The board
does not restate either; where the two disagree, the ticket and the
matrix win.

Two conditions are recorded here because they are easy to misread as
progress:

- **Nothing is `CERTIFIED`.** No artifact is signed and no provenance or
  attestation exists on any branch (B-14), so no artifact can reach
  evidence-ladder rung 9.
- **The release scope is approved, but nothing is certified.** Decision
  0075 is **accepted** (Michael Kinney, 2026-11-17): the approved
  release scope is the MVP candidate scope in that record. The plan's
  full target matrix remains the standing *campaign* matrix — Windows,
  macOS, AppImage, Flatpak, Android, ARM64 and RPM stay outside the
  approved cut until each carries its own acceptance evidence. The
  approval authorizes the cut; it does not certify any artifact, and
  every item inside the scope still needs its own receipt.

## Phase 22 detail

| Item | State at `658f015` (reconciled 2026-11-11) | Remaining evidence/work |
|---|---|---|
| 22.1 Panel inventory and docking | Complete | The right dock is a resizable, scrolling column of stacked panels in CS6's order (`crates/app/src/shell/panel_windows.rs`: 18-panel inventory, collapse/close headers, scroll keeps every panel reachable at any viewport height). All surfaces dispatch, including the previously-missing Optimize, Slices, CSS Properties, Find and Replace, and Common Library panels (landed 2026-11-06). Common Library lists the document's own symbols — CS6's shipped libraries are proprietary resources and are not copied. **The movement model landed 2026-11-11 in PR #128**: a panel's title is its drag handle, both columns are drop targets, the left column appears when a panel is docked there or a drag is in flight, the `«`/`»` arrow moves a panel without dragging, and only *moved* panels are persisted (unknown names dropped on restore). The default arrangement is unchanged — the panel snapshot still records every panel on the right, in CS6's order. **Tab groups landed 2026-11-11 in the same PR**: dropping a panel on another panel's title tabs the two together, the group is one slot that the dock draws, moves and saves (its members and the tab it shows round-trip through the layout file), a group of one dissolves, and a hidden panel leaves its group. The tab is both the switch and the handle — one widget senses the click and the drag, and egui decides between them by whether the pointer moved (issue #91, step 3, closed by PR #128). |
| 22.2 Tool strip and flyouts | Complete | The strip is CS6's, as data in `crates/app/src/toolstrip.rs`: six categories in CS6's order, twenty-four groups with their one-key shortcuts, the flyout memberships, and CS6's group-cycling rule. Evidence in [`research/fw-ui-evidence.md`](research/fw-ui-evidence.md); landed 2026-11-05. |
| 22.3 Adaptive canvas context menu | Complete | Selection-specific rows dispatch through the existing command surface. |
| 22.4 Property inspector | Partial | Per-kind field sets landed 2026-11-04 (group, bitmap, slice); the document-level and multi-selection field sets and enter-group-by-selection landed 2026-11-06 (test `inspector_document_and_multi_selection_sets`). Open: the bitmap transform strip and fill/stroke, both blocked by decision 0019 / absent `IMGv` fill-stroke nodes (recorded, not faked). |
| 22.5 CS6 chrome | Partial | Landed 2026-11-06: the three-mode screen cycle (Standard / Full Screen with Menus / Full Screen, fwref 1178–1185), the Alt-hold retouch swap (Blur↔Sharpen, Dodge↔Burn, fwref 3555/3580), the Window-menu arrangement trio (57650/57651/57652; census 289→290), the Text ▸ Font submenu (five families through `text.setFont`, census 290→294; CS6 ships it as a runtime-populated placeholder), and *Repeat Filter* (re-applies the session's last preset) landed 2026-11-06, as did the F1 grouping fix (the binding table carries its own section per row, gate `menus::tests::shortcut_sections_are_contiguous`). The CS6 keymap itself landed 2026-11-11 in [decision 0080](decisions/0080-cs6-keymap-bindings.md): the transcribed tree now prints CS6's accelerators and every live printed accelerator is bound to its row's action, held by the parity gate `menus::tests::cs6_keymap_is_fully_bound` — 296 rows, 105 of them printing an accelerator, 6 display-only `ReportGap` rows exempt, 99 live printed accelerators all bound. Still open: the Main toolbar's contents (rows 54, 255, 257, 164 in [CS6-PARITY](CS6-PARITY.md)). **The chrome itself was captured and measured 2026-11-16** (build `a34a4034`): frames at 1280×800 and at a deliberately short 1280×400 ([docs/img/chrome-1280x800.png](img/chrome-1280x800.png), [docs/img/chrome-1280x400.png](img/chrome-1280x400.png)), with the receipt in [research/fw-ui-evidence.md](research/fw-ui-evidence.md#the-firecraft-chrome-measured-2026-11-16) and the parity rows updated in [CS6-PARITY](CS6-PARITY.md) §C. The residual CS6 keymap commands closed 2026-11-11: 16 added bindings bring the table to 167 bindings in 40 sections, and all 133 of CS6's keymap-file entries across its 128 command ids now resolve to a binding carrying the action this build answers it with, gated by `menus::tests::cs6_keymap_entries_are_bound` over the transcribed `menus::tests::CS6_KEYMAP` (measured in [CS6-PARITY](CS6-PARITY.md) §H). |
| 22.6 Open-license icon pack | Complete | Attribution and measured wasm-budget effects are recorded in the phase history. |
| Screenshot capture and comparison | Complete | Captured and inspected at 1280×800 and 1920×1080 in the empty, vector, and bitmap states against a fresh optimized bundle on 2026-11-04; the tracked screenshot was replaced and labelled for its build. Receipts in [T22](tasks/T22-screenshot-ui.md). |

## Completed (shell and parity program)

| Task | Evidence |
|---|---|
| 19.1–19.5 canvas competence | Transform handles, path nodes, gradient editor, command palette, guides/grid/snap; [NEXT.md](NEXT.md). |
| 19.6 menu wiring campaign | Batches 1–17 landed; current census is 2 inert rows of 298 (pinned total `menus::tests::menu_item_total_is_pinned`, ceiling `menus::tests::inert_count_never_rises`). Remaining rows are feature work or evidence-gated, not an active burn-down. |
| 19b path nodes and wasm budget | Path node add/delete and dated wasm ceiling re-pin recorded in [Progress](PROGRESS.md). |
| v0.2.5 release tag | `v0.2.5` exists; see [release notes](releases/v0.2.5.md). The live deployment uses a different manifest from the current local distribution. |


## Closed (this board's lifetime)

| Task | Landed |
|---|---|
| Aegis baseline regeneration + `.aegisignore` self-exclusion | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 6 — View menu (10 inerts) | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 7 — Text menu (14 inerts) | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 8 — Attach In Path / Attach to Path (2 inerts); vertical-align rows evidence-gated | 2026-11-03 (`a8c86af`) |
| Gate `perf` step repair — the memory probe re-execs through `/proc/self/exe` so it survives the gate's own binary being replaced mid-run ([decision 0072](decisions/0072-gate-reexec-resolution.md)) | 2026-11-09 (`7252648`) |
| CI-image offline fix — the baked registry is a fast floor, not a wall; the fleet-wide red from a moved `Cargo.lock` was a tool defect, not a stale image ([decision 0073](decisions/0073-ci-image-fast-floor-not-wall.md)) | 2026-11-09 (`02c1c9d`) |
| Aegis baseline re-pin for the release-workflow tree — the baseline was line-keyed, so PR #14's prose edits shifted two documented false positives and the gate failed on noise. Superseded by the content-keyed gate below; the re-pin was a procedure that existed only to route around the tool defect | 2026-11-09 (`7e7f8ad`) |
| Aegis gate is content-keyed on this branch — a finding's identity is `(pattern, file, content digest)` rather than Aegis's line-keyed fingerprint, so a prose edit that moves a documented false positive no longer fails the gate. No rule class is switched off; each class's rows carry the reason they were adjudicated ([decision 0074](decisions/0074-aegis-content-keyed-baseline.md)) | 2026-11-09 |
| Host fork-bomb from a throwaway `/proc/self/exe` probe — ~5800 processes at load 2057 starved the GitForge runner's sandbox; terminated by per-PID cmdline+exe verification and the probe directory moved aside | 2026-11-09 |
| Task-board tracking scaffold — `docs/TASKS.md` plus the `docs/tasks/` per-task ticket set (one file per task, same objective/inputs/authority/stopping-condition/verification shape as the milestone tickets). The scaffold is this board and the `docs/tasks/` directory; the row it carried in the queued table is closed by this reconciliation | 2026-11-09 |
| Shell-lineage and release-engineering trains merged — #107 squash `658f015` carrying #8, #13, #15, #41, #83, #84, #100, #102, #103 and (via #108 → `1884c04`) #85, #101, #104, #105, #106; the 14 constituent PRs were closed as merged-by-train, and Phase 20.5–20.7 plus all of Phase 21 shipped in the same landing | 2026-11-11 |
| Post-train RustSec lockfile clear — #117 (`b61f198`): `zbus_xml` 5.1.0→5.2.1 (drops `quick-xml` 0.38.4, clearing RUSTSEC-2026-0194/0195) and `tokio` 1.53.2→1.54.1 (clears RUSTSEC-2026-0386); `cargo audit` exit 0. The `serde_yaml`/`unsafe-libyaml` removal (RUSTSEC-2024-0449) rides PR #120 (saphyr swap in xtask's Flatpak parse) | 2026-11-11 |
| Post-train issue triage — closed with evidence: #79, #81, #86, #94, #98, #109, #111, #112 (all fixed by #107/#108's content or #14; squash broke the auto-close links); progress-commented and left open: #75, #76, #78, #93, #97, #110; filed #121 (`xtask changelog` cannot regenerate main's committed CHANGELOG after a squash merge) | 2026-11-11 |
| Command-count and menu-census doc repair — README, GAPS, FEATURES, HANDOFF, CS6-PARITY, PROGRESS and NEXT quoted stale pre-train numbers (252 commands / 294 items); corrected to the gated truth: **253 registered commands** (measured by `xtask reachability`, 164 bound = 64.82%) and **2 inert of 296** (the pinned census) | 2026-11-11 |
| Changelog squash-row repair — the #123 gate's changelog step ran before its merge commit existed, so `git log` never saw main's nine squash rows (#118/#119/#120/#123/#124/#125/#127/#130/#135); regenerated additively on `55a0e54` (399 → 408 unreleased entries, none dropped) — the missing rows included #120's saphyr dependency swap flagged by the 2026-11-13 audit | 2026-11-13 |
| Audit remediation 2026-11-13 — S2 restored to the measured 168/253, `symbol.breakApart` undo divergence fixed at the registry, `reachability --list` added ([T28](tasks/T28-reachability-list.md)), the io lint table made complete, four phantom layer entries and the duplicate decision number removed, and the two contradictory board rows reconciled | 2026-11-13 |

## Blocked or evidence-gated

- The 3 partial CS6 DOM rows remain evidence-gated; see [`planning/generated/dom-parity.md`](planning/generated/dom-parity.md) and the specific decisions.
- The remaining 8 menu rows are tracked with explicit reasons by the live `inert_worklist`; do not wire behavior without a model/engine or reference basis.
- Non-Linux release artifacts remain unverified until built and launched on their target platforms.


## Ledger rules


- Update this file and the ticket when work starts, changes scope, or lands.
- Keep detailed implementation history in [`SESSIONS.md`](SESSIONS.md); keep host/tooling diagnostics out of the FireCraft session history.
- Do not copy a second active task list into the handoff or historical audit documents.
