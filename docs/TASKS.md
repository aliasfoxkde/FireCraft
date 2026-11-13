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
> [decision 0075](decisions/0075-release-scope-mvp.md) — currently
> *recommended — awaiting maintainer approval*, so the approved release
> scope is **unset**. **Evidence baseline** for release work:
> [`release/PHASE-0-AUDIT.md`](release/PHASE-0-AUDIT.md) (PR #16,
> dated 2026-11-09 — the link resolves once that PR merges).
> Per-artifact evidence state:
> [`release/ARTIFACT-MATRIX.md`](release/ARTIFACT-MATRIX.md).

## In flight

| Priority | Task | Status | Detail |
|---|---|---|---|
| P0 | Refresh user-visible CS6 parity matrix | **Done — matrix published** | [T26](tasks/T26-parity-matrix.md) → [`CS6-PARITY.md`](CS6-PARITY.md): every CS6 menu, tool, panel, inspector field set, workflow, import/export surface, canvas interaction, keymap area, and selection state classified with an evidence citation and an owning ticket. The browser-sampling check the ticket named was performed on 2026-11-04 against a fresh optimized bundle and is recorded in T22. |
| P0 | Phase 22: CS6 shell fidelity and screenshot audit | In progress | [T22](tasks/T22-screenshot-ui.md) — screenshot acceptance is complete; 22.1 panel inventory and 22.2 tool strip landed on `main` with the shell-lineage train (#107); 22.5 chrome remains partial, and 22.4 still has evidence-gated bitmap transform/fill-stroke work. [Phase 22](NEXT.md#phase-22--cs6-shell-fidelity-p0-the-looks-and-acts-like-fireworks-phase) |
| P1 | Phase 20: daily-driver editing features | **In progress — 20.1–20.3, 20.5–20.7 on main; 20.4 in PR** | In-canvas text editing landed 2026-11-06 ([T20](tasks/T20-text-editing.md)); multi-document tabs (20.5), history rewind (20.6), and slice/hotspot behavior editing with HTML preview (20.7) landed 2026-11-11 in #107; the Optimize panel (20.2, [T7](tasks/T7-optimize-panel.md)) and the animation timeline (20.3, [T8](tasks/T8-animation-timeline.md)) landed 2026-11-11 in #118. Mask stack (20.4) is in open PR #113. Do not count open PR work as shipped until merged and validated. [details](NEXT.md#phase-20--daily-driver-features-p1) |
| P1 | Phase 21: modern differentiators | **Done — all items landed 2026-11-11** | Autosave/crash recovery, HiDPI, command palette v2 (fuzzy scoring, recents, Save Steps As Command), preferences, accesskit accessibility and in-app scripting (`.jsf`/`.fwsteps`) all landed with the shell-lineage train (#107, PR #15); [details](NEXT.md#phase-21--modern-differentiators) |
| P1 | Reconcile live web deployment | **Measured — the live site is not the v0.2.5 release bytes** | Measured 2026-11-11 by digest: the live bundle's trunk asset name (`2274ac8c2925b61d`) and wasm digest (`f4f0a3f0…`) match neither the published v0.2.5 release asset (`f9dbe2f14305b2d9` / `a8a4fa4f…`) nor a rebuild of the v0.2.5 tag (`103c62d3366aef17` / `5934702f…`), so the live build corresponds to no identifiable release and its source revision is unknown. The earlier reconciliation (#104) inferred a match from equal file *sizes* and an identical `appinfo.json`; the digest measurement falsifies it. The served `index.html` difference is explained (substituted asset names plus the Cloudflare analytics beacon). **What would close it:** deploy a bundle whose asset name and wasm digest are recorded here, from a named commit — a production publish, gated for explicit maintainer approval. See [T18](tasks/T18-release.md). |
| P1 | Surface gap: the 85 carried commands the shell never binds | Open | Six issues, one per family: [#136](https://github.com/aliasfoxkde/FireCraft/issues/136) text (17), [#137](https://github.com/aliasfoxkde/FireCraft/issues/137) symbol (16), [#138](https://github.com/aliasfoxkde/FireCraft/issues/138) selection + filter (13), [#139](https://github.com/aliasfoxkde/FireCraft/issues/139) page/cell/document/frame (11), [#140](https://github.com/aliasfoxkde/FireCraft/issues/140) mask/bitmap/layer/object/path/style (18), [#141](https://github.com/aliasfoxkde/FireCraft/issues/141) hotspot/slice/primitive/autoshape/clipboard (10). The engine carries every one of them; the shell's designed surface binds none. Measured by the reachability gate (`xtask/src/reachability.rs`): 168 of 253 registry commands bound (66.40 %), 85 unreachable. [detail](GAPS.md#16-the-surface-gap--85-carried-commands-the-shell-never-binds-2026-11-12) |
| P2 | Next-release asset breadth and platform verification | Open | [T23](tasks/T23-release-assets.md); recipes exist, but non-Linux artifacts need real platform build/launch receipts. |
| P2 | Coverage and test quality ratchet | Open | Replace the old aspirational 99% label with measured per-crate targets and snapshots; [NEXT.md](NEXT.md). |
| P2 | Phase 22.1: the dockable panel set — docking, tab groups, layout persistence | Complete | [issue #91](https://github.com/aliasfoxkde/FireCraft/issues/91). Docking (a panel moves between the left and right columns, by drag or by the header arrow), tab groups (a panel dropped on another panel's title shares its slot — the group is one unit that the dock draws, moves and saves), and their persistence landed 2026-11-11 in PR #128; the merge waits on that branch's receipts. |
| P2 | Aegis gate: content-keyed baseline, so a doc edit no longer fails the gate | **Done — landed on `main`** | [decisions/0074-aegis-content-keyed-baseline.md](decisions/0074-aegis-content-keyed-baseline.md). The baseline keys on content (pattern, file, digest) rather than on line, and every rule class stays armed. |

### Release & tooling lanes in flight


| # | Task | Ticket | Status | Owner | Next action |
|---|---|---|---|---|---|
| 5 | 19.6 inert-menu wiring campaign | [tasks/T5-inert-burndown.md](tasks/T5-inert-burndown.md) | `DONE — 2026-11-11` | UI/verb lane | Batches 1–17 are complete. The current pinned census is 2 inert rows of 296; eight other rows remain explicitly feature/evidence-gated and are not part of an active wiring campaign (`menus::tests::inert_count_never_rises`, `menus::tests::menu_item_total_is_pinned`). |
| 194 | Aegis gate: content-keyed baseline, so a doc edit no longer fails the gate (decision 0074) | [decisions/0074-aegis-content-keyed-baseline.md](decisions/0074-aegis-content-keyed-baseline.md) | `IMPLEMENTED` — the content-keyed gate and its decision record are in the tree; the gate has been exercised on this branch's tree | CI/tooling lane | **landed in `main` (#18)** and merged here; attach a run id before claiming the fleet-wide state |
| — | The coverage gate explains itself: the per-file breakdown, each file's `DA` record count beside its `LF`, and the profile inventory (decision 0103) | [decisions/0103-coverage-export-counts-a-file-once-per-build.md](decisions/0103-coverage-export-counts-a-file-once-per-build.md) | `IMPLEMENTED` on the branch; the CI receipt is pending | CI/tooling lane | land it on both receipts; the check that closes the decision is the next pinned run, where `LF` must equal the `DA` count again for `crates/app/src/shell/mod.rs` — under rustc 1.100.0 the export reported 8138 lines for a file that has 6309 |
| 27 | Cross-platform release engineering: Windows, macOS, Linux package matrix, Android feasibility, source, SBOM/provenance, signing, CI aggregation and certification | [tasks/T27-cross-platform-release.md](tasks/T27-cross-platform-release.md) | `IN PROGRESS` — implementation is ahead of verification; **no target is `CERTIFIED`** and nothing is signed | release-workflow lane | land an end-to-end tag run of the repaired workflow and attach the run id as the RP-07 receipt; consolidate the duplicated `dist --source`/`--sbom` verbs (B-3) |
| 26 | Refresh user-visible CS6 parity matrix | [tasks/T26-parity-matrix.md](tasks/T26-parity-matrix.md) | `IMPLEMENTED` — matrix published | parity lane | every CS6 menu, tool, panel, inspector field set, workflow, import/export surface, canvas interaction, keymap area, and selection state is classified with an evidence citation and an owning ticket in [`CS6-PARITY.md`](CS6-PARITY.md); the browser-sampling check the ticket named was performed on 2026-11-04 against a fresh optimized bundle and is recorded in T22 |
| 22 | Phase 22: CS6 shell fidelity and screenshot audit | [tasks/T22-screenshot-ui.md](tasks/T22-screenshot-ui.md) | `IN PROGRESS` | UI lane | screenshot acceptance is complete; 22.1 panel inventory and 22.2 tool strip landed on `main` with the shell-lineage train (#107); 22.5 chrome remains partial, and 22.4 still has evidence-gated bitmap transform/fill-stroke work. [Phase 22](NEXT.md#phase-22--cs6-shell-fidelity-p0-the-looks-and-acts-like-fireworks-phase) |
| 20 | 20.1 In-canvas text editing (egui TextEdit overlay; kittest focus tests) | [tasks/T20-text-editing.md](tasks/T20-text-editing.md) | `IMPLEMENTED` — landed 2026-11-06 (`shell::begin_text_edit` / `text_edit_overlay`) | editing lane | the remaining Phase 20 items (Optimize, animation timeline, mask stack, tabs, history rewind, slice behavior) stay queued below |
| 8 | 20.3 Animation timeline (frames strip, per-state timing, onion skinning, `symbol.tween`) | [tasks/T8-animation-timeline.md](tasks/T8-animation-timeline.md) | `IMPLEMENTED` — the shell half landed 2026-11-11 (decision [0094](decisions/0094-animation-timeline-shell.md)): the States panel is the strip, the delay field writes `frame.setDuration`, the onion toggle writes `page.setOnionSkinning` and the canvas calls `compose::render_onion`; playback landed 2026-11-12 (decision [0096](decisions/0096-animation-playback-clock.md)): the panel has CS6's `Play Animation` as a clock over elapsed-time data, so the view follows the playhead and the timing is tested without sleeping | editing lane | state thumbnails, a document-level loop count (CS6's `{loopCount:%Ai}`), and the screenshot acceptance are still open — see the ticket |
| 7 | 20.2 Optimize panel (palette/bitrate over `ExportOptions` + live preview) | [tasks/T7-optimize-panel.md](tasks/T7-optimize-panel.md) | `IMPLEMENTED` — landed 2026-11-11 (decision [0095](decisions/0095-optimize-panel-honors-only-what-export-honors.md)): the controls are the fields the export honors, the preview encodes the page through the export's own `io::expv` resolution, and PNG's Adam7 interlace is disclosed and clearable | editing lane | Adam7 interlacing, CS6's selective-JPEG quality table, and the screenshot acceptance are still open — see the ticket |
| — | CI throughput: a warm Cargo cache per job, so a pipeline run starts warm instead of rebuilding the world | [decisions/0098-per-job-cargo-cache.md](decisions/0098-per-job-cargo-cache.md) | `IN PROGRESS` — the per-job `cache:` blocks, the offline validator and the validator's CI step are in the tree on this branch; no run has yet measured the warm cost | CI/tooling lane | land the change, then read the first warm run's per-job durations from the authoritative store before claiming a throughput win; the cache's own growth needs a prune policy (recorded as debt in decision 0098) |


## Phase 20 — remaining work (open PRs)


| # | Task | Ticket |
|---|---|---|
| — | 20.4 Mask stack UI (add/remove/mode/link/move) — a FireCraft extension, not a CS6 parity claim | [PR #113](https://github.com/aliasfoxkde/FireCraft/pull/113); open, not on `main` |

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
| 18 | Republish the reconciled v0.2.5 web artifact | [tasks/T18-release.md](tasks/T18-release.md) | `RECONCILED — REPUBLISH PENDING` | release-workflow lane | #104 established that the live JavaScript, Wasm and `appinfo` match the v0.2.5 release bytes; the remaining `host.html`/`index.html` differences are Cloudflare's injected analytics beacon. Republish from the canonical release artifact and verify the deployed hashes/cache stamp; do not describe the deployment as an unidentified bundle. |
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
- **The release scope is not approved.** Decision 0075 records the MVP
  candidate and the extended campaign scope, and its status is
  *recommended — awaiting maintainer approval*. Until the maintainer
  approves it, the plan's full target matrix remains the standing scope
  and no cut is authorized against a narrowed one.

## Phase 22 detail

| Item | State at `658f015` (reconciled 2026-11-11) | Remaining evidence/work |
|---|---|---|
| 22.1 Panel inventory and docking | Complete | The right dock is a resizable, scrolling column of stacked panels in CS6's order (`crates/app/src/shell/panel_windows.rs`: 18-panel inventory, collapse/close headers, scroll keeps every panel reachable at any viewport height). All surfaces dispatch, including the previously-missing Optimize, Slices, CSS Properties, Find and Replace, and Common Library panels (landed 2026-11-06). Common Library lists the document's own symbols — CS6's shipped libraries are proprietary resources and are not copied. **The movement model landed 2026-11-11 in PR #128**: a panel's title is its drag handle, both columns are drop targets, the left column appears when a panel is docked there or a drag is in flight, the `«`/`»` arrow moves a panel without dragging, and only *moved* panels are persisted (unknown names dropped on restore). The default arrangement is unchanged — the panel snapshot still records every panel on the right, in CS6's order. **Tab groups landed 2026-11-11 in the same PR**: dropping a panel on another panel's title tabs the two together, the group is one slot that the dock draws, moves and saves (its members and the tab it shows round-trip through the layout file), a group of one dissolves, and a hidden panel leaves its group. The tab is both the switch and the handle — one widget senses the click and the drag, and egui decides between them by whether the pointer moved (issue #91, step 3, closed by PR #128). |
| 22.2 Tool strip and flyouts | Complete | The strip is CS6's, as data in `crates/app/src/toolstrip.rs`: six categories in CS6's order, twenty-four groups with their one-key shortcuts, the flyout memberships, and CS6's group-cycling rule. Evidence in [`research/fw-ui-evidence.md`](research/fw-ui-evidence.md); landed 2026-11-05. |
| 22.3 Adaptive canvas context menu | Complete | Selection-specific rows dispatch through the existing command surface. |
| 22.4 Property inspector | Partial | Per-kind field sets landed 2026-11-04 (group, bitmap, slice); the document-level and multi-selection field sets and enter-group-by-selection landed 2026-11-06 (test `inspector_document_and_multi_selection_sets`). Open: the bitmap transform strip and fill/stroke, both blocked by decision 0019 / absent `IMGv` fill-stroke nodes (recorded, not faked). |
| 22.5 CS6 chrome | Partial | Landed 2026-11-06: the three-mode screen cycle (Standard / Full Screen with Menus / Full Screen, fwref 1178–1185), the Alt-hold retouch swap (Blur↔Sharpen, Dodge↔Burn, fwref 3555/3580), the Window-menu arrangement trio (57650/57651/57652; census 289→290), the Text ▸ Font submenu (five families through `text.setFont`, census 290→294; CS6 ships it as a runtime-populated placeholder), and *Repeat Filter* (re-applies the session's last preset) landed 2026-11-06, as did the F1 grouping fix (the binding table carries its own section per row, gate `menus::tests::shortcut_sections_are_contiguous`). The CS6 keymap itself landed 2026-11-11 in [decision 0080](decisions/0080-cs6-keymap-bindings.md): the transcribed tree now prints CS6's accelerators and every live printed accelerator is bound to its row's action, held by the parity gate `menus::tests::cs6_keymap_is_fully_bound` — 296 rows, 105 of them printing an accelerator, 6 display-only `ReportGap` rows exempt, 99 live printed accelerators all bound. Still open: the Main toolbar's contents (rows 54, 255, 257, 164 in [CS6-PARITY](CS6-PARITY.md)). The residual CS6 keymap commands closed 2026-11-11: 16 added bindings bring the table to 167 bindings in 40 sections, and all 133 of CS6's keymap-file entries across its 128 command ids now resolve to a binding carrying the action this build answers it with, gated by `menus::tests::cs6_keymap_entries_are_bound` over the transcribed `menus::tests::CS6_KEYMAP` (measured in [CS6-PARITY](CS6-PARITY.md) §H). |
| 22.6 Open-license icon pack | Complete | Attribution and measured wasm-budget effects are recorded in the phase history. |
| Screenshot capture and comparison | Complete | Captured and inspected at 1280×800 and 1920×1080 in the empty, vector, and bitmap states against a fresh optimized bundle on 2026-11-04; the tracked screenshot was replaced and labelled for its build. Receipts in [T22](tasks/T22-screenshot-ui.md). |

## Completed (shell and parity program)

| Task | Evidence |
|---|---|
| 19.1–19.5 canvas competence | Transform handles, path nodes, gradient editor, command palette, guides/grid/snap; [NEXT.md](NEXT.md). |
| 19.6 menu wiring campaign | Batches 1–17 landed; current census is 2 inert rows of 296 (pinned total `menus::tests::menu_item_total_is_pinned`, ceiling `menus::tests::inert_count_never_rises`). Remaining rows are feature work or evidence-gated, not an active burn-down. |
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

## Blocked or evidence-gated

- The 3 partial CS6 DOM rows remain evidence-gated; see [`planning/generated/dom-parity.md`](planning/generated/dom-parity.md) and the specific decisions.
- The remaining 8 menu rows are tracked with explicit reasons by the live `inert_worklist`; do not wire behavior without a model/engine or reference basis.
- Non-Linux release artifacts remain unverified until built and launched on their target platforms.


## Ledger rules


- Update this file and the ticket when work starts, changes scope, or lands.
- Keep detailed implementation history in [`SESSIONS.md`](SESSIONS.md); keep host/tooling diagnostics out of the FireCraft session history.
- Do not copy a second active task list into the handoff or historical audit documents.
