# FireCraft — Live task board

> **Live ledger.** Reconciled on the PR #8 working branch (2026-11-07; merged
> with `main` on 2026-11-10) against `main` at `5fac7a2`. Changes remain
> unmerged until PR approval. Use [`NEXT.md`](NEXT.md) for phase detail and
> [`tasks/`](tasks/) for task tickets. Historical plans and session notes are
> not current status sources.

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
| P0 | Phase 22: CS6 shell fidelity and screenshot audit | In progress | [T22](tasks/T22-screenshot-ui.md) — screenshot acceptance is complete; 22.1 panel inventory and 22.2 tool strip are complete in PR #8; 22.5 chrome remains partial, and 22.4 still has evidence-gated bitmap transform/fill-stroke work. [Phase 22](NEXT.md#phase-22--cs6-shell-fidelity-p0-the-looks-and-acts-like-fireworks-phase) |
| P1 | Phase 20: daily-driver editing features | **In progress — 20.1 landed** | In-canvas text editing landed 2026-11-06 ([T20](tasks/T20-text-editing.md)); the mask stack's combine mode landed 2026-11-11 ([T9](tasks/T9-mask-stack.md), model + persistence + panel — the compositor does not apply masks yet); remaining: Optimize, animation timeline, mask stack rendering and level management, tabs, history rewind, slice behavior; [details](NEXT.md#phase-20--daily-driver-features-p1) |
| P1 | Phase 21: modern differentiators | Queued | Autosave, HiDPI, palette, preferences, accessibility, scripting; [details](NEXT.md#phase-21--modern-differentiators) |
| P1 | Reconcile live web deployment | Artifact differs | The public manifest's JS/wasm/index hashes differ from local `dist/web`; identify intended deploy and verify it before publishing. See [T18](tasks/T18-release.md). |
| P2 | Next-release asset breadth and platform verification | Open | [T23](tasks/T23-release-assets.md); recipes exist, but non-Linux artifacts need real platform build/launch receipts. |
| P2 | Coverage and test quality ratchet | Open | Replace the old aspirational 99% label with measured per-crate targets and snapshots; [NEXT.md](NEXT.md). |
| P2 | Aegis gate: content-keyed baseline, so a doc edit no longer fails the gate | **Done — landed on `main`** | [decisions/0074-aegis-content-keyed-baseline.md](decisions/0074-aegis-content-keyed-baseline.md). The baseline keys on content (pattern, file, digest) rather than on line, and every rule class stays armed. |

### Release & tooling lanes in flight


| # | Task | Ticket | Status | Owner | Next action |
|---|---|---|---|---|---|
| 5 | 19.6 inert-menu burn-down — Modify/File families (83 of 289; batch 8 = Attach to Path landed 2026-11-03) | [tasks/T5-inert-burndown.md](tasks/T5-inert-burndown.md) | `IN PROGRESS` | UI/verb lane | continue the per-family burn-down; each batch carries its own evidence gate. This branch's newer census has batches 1–17 landed with 2 inert rows of 294, derived by `menus::tests::inert_count_never_rises` |
| 194 | Aegis gate: content-keyed baseline, so a doc edit no longer fails the gate (decision 0074) | [decisions/0074-aegis-content-keyed-baseline.md](decisions/0074-aegis-content-keyed-baseline.md) | `IMPLEMENTED` — the content-keyed gate and its decision record are in the tree; the gate has been exercised on this branch's tree | CI/tooling lane | **landed in `main` (#18)** and merged here; attach a run id before claiming the fleet-wide state |
| 27 | Cross-platform release engineering: Windows, macOS, Linux package matrix, Android feasibility, source, SBOM/provenance, signing, CI aggregation and certification | [tasks/T27-cross-platform-release.md](tasks/T27-cross-platform-release.md) | `IN PROGRESS` — implementation is ahead of verification; **no target is `CERTIFIED`** and nothing is signed | release-workflow lane | land an end-to-end tag run of the repaired workflow and attach the run id as the RP-07 receipt; consolidate the duplicated `dist --source`/`--sbom` verbs (B-3) |
| 26 | Refresh user-visible CS6 parity matrix | [tasks/T26-parity-matrix.md](tasks/T26-parity-matrix.md) | `IMPLEMENTED` — matrix published | parity lane | every CS6 menu, tool, panel, inspector field set, workflow, import/export surface, canvas interaction, keymap area, and selection state is classified with an evidence citation and an owning ticket in [`CS6-PARITY.md`](CS6-PARITY.md); the browser-sampling check the ticket named was performed on 2026-11-04 against a fresh optimized bundle and is recorded in T22 |
| 22 | Phase 22: CS6 shell fidelity and screenshot audit | [tasks/T22-screenshot-ui.md](tasks/T22-screenshot-ui.md) | `IN PROGRESS` | UI lane | screenshot acceptance is complete; 22.1 panel inventory and 22.2 tool strip are complete in PR #8; 22.5 chrome remains partial, and 22.4 still has evidence-gated bitmap transform/fill-stroke work. [Phase 22](NEXT.md#phase-22--cs6-shell-fidelity-p0-the-looks-and-acts-like-fireworks-phase) |
| 20 | 20.1 In-canvas text editing (egui TextEdit overlay; kittest focus tests) | [tasks/T20-text-editing.md](tasks/T20-text-editing.md) | `IMPLEMENTED` — landed 2026-11-06 (`shell::begin_text_edit` / `text_edit_overlay`) | editing lane | the remaining Phase 20 items (Optimize, animation timeline, mask stack, tabs, history rewind, slice behavior) stay queued below |


## Queued (Phase 20 — daily-driver features)


| # | Task | Ticket |
|---|---|---|
| 20 | 20.1 In-canvas text editing (egui TextEdit overlay; kittest focus tests) | [tasks/T20-text-editing.md](tasks/T20-text-editing.md) |
| 7 | 20.2 Optimize panel (palette/bitrate over `ExportOptions` + live preview) | [tasks/T7-optimize-panel.md](tasks/T7-optimize-panel.md) |
| 8 | 20.3 Animation timeline (frames strip, timing, onion skinning, `symbol.tween`) | [tasks/T8-animation-timeline.md](tasks/T8-animation-timeline.md) |
| — | 20.4 Mask stack UI (add/remove/mode/link/move) | [tasks/T9-mask-stack.md](tasks/T9-mask-stack.md) |
| 9 | 20.5 Multi-document tabs + Window-menu panel toggles | — |
| 10 | 20.6 History panel click-to-rewind (journal replay to seq N) | — |
| 11 | 20.7 Slice/hotspot behavior editing + HTML preview | — (landed on `feat/slice-hotspot-edit`, PR #13 — not yet on `main`) |

## Queued (Phase 21 — modern differentiators)



| # | Task | Ticket |
|---|---|---|
| 12 | 21.1 Autosave + crash recovery (journal snapshot sidecar; restore prompt) | — |
| 13 | 21.2 HiDPI `pixels_per_point` + render-stamp→dirty-counter migration | — |
| 14 | 21.3 Command palette v2 (fuzzy scoring, recents, save-steps-as-command) | — |
| 15 | 21.4 Preferences dialog + platform-storage persistence (0067 follow-up) | — |
| 16 | 21.5 accesskit desktop + web re-audit (0067 records the web blocker) | — |
| 17 | 21.6 In-app scripting (`.jsf`/`.fwsteps` from the Commands menu) | — |

## Queued (release + quality)

| # | Task | Ticket | Status | Owner | Next action |
|---|---|---|---|---|---|
| 18 | Ship v0.2.5: release assets + web deploy | [tasks/T18-release.md](tasks/T18-release.md) | `BLOCKED` — blocked on T23's asset breadth. The v0.2.5 release exists as published artifacts, but the **live web deployment serves a bundle that corresponds to no identifiable release** (B-6), and the published set was published out of band rather than by the release workflow (B-21) | release-workflow lane | reconcile the deployed bundle against a tagged release by hashing the deployed assets individually and recording the producing commit; do not re-cut v0.2.5 — B-19 means the next artifact must be rebuilt and re-measured, not re-tagged |
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

| Item | State at `5fac7a2` | Remaining evidence/work |
|---|---|---|
| 22.1 Panel inventory and docking | Complete | The right dock is a resizable, scrolling column of stacked panels in CS6's order (`crates/app/src/shell/panel_windows.rs`: 18-panel inventory, collapse/close headers, scroll keeps every panel reachable at any viewport height). All surfaces dispatch, including the previously-missing Optimize, Slices, CSS Properties, Find and Replace, and Common Library panels (landed 2026-11-06). Common Library lists the document's own symbols — CS6's shipped libraries are proprietary resources and are not copied. |
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
| 19.6 menu wiring campaign | Batches 1–17 landed; current census is 2 inert rows of 294, derived and gated by `menus::tests::inert_count_never_rises`. Remaining rows are feature work or evidence-gated, not an active burn-down. |
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

## Blocked or evidence-gated

- The 3 partial CS6 DOM rows remain evidence-gated; see [`planning/generated/dom-parity.md`](planning/generated/dom-parity.md) and the specific decisions.
- The remaining 8 menu rows are tracked with explicit reasons by the live `inert_worklist`; do not wire behavior without a model/engine or reference basis.
- Non-Linux release artifacts remain unverified until built and launched on their target platforms.


## Ledger rules


- Update this file and the ticket when work starts, changes scope, or lands.
- Keep detailed implementation history in [`SESSIONS.md`](SESSIONS.md); keep host/tooling diagnostics out of the FireCraft session history.
- Do not copy a second active task list into the handoff or historical audit documents.

d1699c053ec52ee9585cb2294242bdd779787503
