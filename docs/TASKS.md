# FireCraft task board

> **Live ledger.** Last reconciled against `main` at `5fac7a2` on 2026-11-04. Use [`NEXT.md`](NEXT.md) for phase detail and [`tasks/`](tasks/) for task tickets. Historical plans and session notes are not current status sources.

## Active

| Priority | Task | Status | Detail |
|---|---|---|---|
| P0 | Refresh user-visible CS6 parity matrix | **Done — matrix published** | [T26](tasks/T26-parity-matrix.md) → [`CS6-PARITY.md`](CS6-PARITY.md): every CS6 menu, tool, panel, inspector field set, workflow, import/export surface, canvas interaction, keymap area, and selection state classified with an evidence citation and an owning ticket. The browser-sampling check the ticket named was performed on 2026-11-04 against a fresh optimized bundle and is recorded in T22. |
| P0 | Phase 22: CS6 shell fidelity and screenshot audit | In progress | [T22](tasks/T22-screenshot-ui.md) — the ticket's own acceptance criteria are met (captures at both viewports, the empty/vector/bitmap state set, the fresh bundle, and the replaced tracked screenshot); what remains is the work other Phase 22 items own: 22.1 panel inventory, 22.2 tool strip, 22.5 chrome. [Phase 22](NEXT.md#phase-22--cs6-shell-fidelity-p0-the-looks-and-acts-like-fireworks-phase) |
| P1 | Phase 20: daily-driver editing features | **In progress — 20.1, 20.5, 20.6 landed** | In-canvas text editing landed 2026-11-06 ([T20](tasks/T20-text-editing.md)); multi-document tabs landed 2026-11-07 (park on Open/New, Close promotes, dirty dots); history rewind landed; remaining: Optimize, animation timeline, mask stack, slice behavior; [details](NEXT.md#phase-20--daily-driver-features-p1) |
| P1 | Phase 21: modern differentiators | **Complete — 21.1–21.4, 21.6 landed; 21.5 desktop arm landed** | Autosave/crash recovery (web localStorage + desktop sidecar journal), HiDPI device-pixel rendering, command palette v2, Preferences dialog, Commands-menu scripting (.jsf + .fwsteps), and desktop accesskit all landed 2026-11-07. The 21.5 web arm remains the recorded upstream eframe refusal (decision 0067); [details](NEXT.md#phase-21--modern-differentiators) |
| P1 | Reconcile live web deployment | Artifact differs | The public manifest's JS/wasm/index hashes differ from local `dist/web`; identify intended deploy and verify it before publishing. See [T18](tasks/T18-release.md). |
| P2 | Next-release asset breadth and platform verification | Open | [T23](tasks/T23-release-assets.md); recipes exist, but non-Linux artifacts need real platform build/launch receipts. |
| P2 | Coverage and test quality ratchet | Open | Replace the old aspirational 99% label with measured per-crate targets and snapshots; [NEXT.md](NEXT.md). |
| P2 | egui_kittest snapshot gates per panel/toolbar | Open | Theme and WCAG regression snapshots per panel; carried over from `main`'s board — [NEXT.md](NEXT.md#phase-21--modern-differentiators). |
| P2 | Responsive/adaptive layout audit + fixes | Open | Layout behavior across viewport sizes; carried over from `main`'s board — [NEXT.md](NEXT.md). |
| P1 | Aegis gate: content-keyed baseline, so a doc edit no longer fails the gate | **Done — landed with the `main` merge** | [decision 0074](decisions/0074-aegis-content-keyed-baseline.md): the committed baseline `.aegis-baseline.tsv` is keyed on content (pattern, file, digest) rather than on line, so a prose edit above a finding no longer invalidates it. No rule class is switched off — every occurrence is adjudicated individually. |

## Phase 22 detail

| Item | State at `5fac7a2` | Remaining evidence/work |
|---|---|---|
| 22.1 Panel inventory and docking | Complete | The right dock is a resizable, scrolling column of stacked panels in CS6's order (`crates/app/src/shell/panel_windows.rs`: 18-panel inventory, collapse/close headers, scroll keeps every panel reachable at any viewport height). All surfaces dispatch, including the previously-missing Optimize, Slices, CSS Properties, Find and Replace, and Common Library panels (landed 2026-11-06). Common Library lists the document's own symbols — CS6's shipped libraries are proprietary resources and are not copied. |
| 22.2 Tool strip and flyouts | Complete | The strip is CS6's, as data in `crates/app/src/toolstrip.rs`: six categories in CS6's order, twenty-four groups with their one-key shortcuts, the flyout memberships, and CS6's group-cycling rule. Evidence in [`research/fw-ui-evidence.md`](research/fw-ui-evidence.md); landed 2026-11-05. |
| 22.3 Adaptive canvas context menu | Complete | Selection-specific rows dispatch through the existing command surface. |
| 22.4 Property inspector | Partial | Per-kind field sets landed 2026-11-04 (group, bitmap, slice); the document-level and multi-selection field sets and enter-group-by-selection landed 2026-11-06 (test `inspector_document_and_multi_selection_sets`). Open: the bitmap transform strip and fill/stroke, both blocked by decision 0019 / absent `IMGv` fill-stroke nodes (recorded, not faked). |
| 22.5 CS6 chrome | Partial | Landed 2026-11-06: the three-mode screen cycle (Standard / Full Screen with Menus / Full Screen, fwref 1178–1185), the Alt-hold retouch swap (Blur↔Sharpen, Dodge↔Burn, fwref 3555/3580), the Window-menu arrangement trio (57650/57651/57652; census 289→290), the Text ▸ Font submenu (five families through `text.setFont`, census 290→294; CS6 ships it as a runtime-populated placeholder), and *Repeat Filter* (re-applies the session's last preset) landed 2026-11-06, as did the F1 grouping fix (`SHORTCUT_GROUPS` tiles the table, gate `the_shortcut_groups_cover_the_table`). Still open: the 76 unbound keymap rows and the Main toolbar's contents (rows 54, 255, 257, 164 in [CS6-PARITY](CS6-PARITY.md)). |
| 22.6 Open-license icon pack | Complete | Attribution and measured wasm-budget effects are recorded in the phase history. |
| Screenshot capture and comparison | Complete | Captured and inspected at 1280×800 and 1920×1080 in the empty, vector, and bitmap states against a fresh optimized bundle on 2026-11-04; the tracked screenshot was replaced and labelled for its build. Receipts in [T22](tasks/T22-screenshot-ui.md). |

## Completed

| Task | Evidence |
|---|---|
| 19.1–19.5 canvas competence | Transform handles, path nodes, gradient editor, command palette, guides/grid/snap; [NEXT.md](NEXT.md). |
| 19.6 menu wiring campaign | Batches 1–17 landed; current census is 2 inert rows of 294, derived and gated by `menus::tests::inert_count_never_rises`. Remaining rows are feature work or evidence-gated, not an active burn-down. |
| 19b path nodes and wasm budget | Path node add/delete and dated wasm ceiling re-pin recorded in [Progress](PROGRESS.md). |
| v0.2.5 release tag | `v0.2.5` exists; see [release notes](releases/v0.2.5.md). The live deployment uses a different manifest from the current local distribution. |

## Blocked or evidence-gated

- The 3 partial CS6 DOM rows remain evidence-gated; see [`planning/generated/dom-parity.md`](planning/generated/dom-parity.md) and the specific decisions.
- The remaining 2 menu rows are tracked with explicit reasons by the live `inert_worklist`; do not wire behavior without a model/engine or reference basis.
- Non-Linux release artifacts remain unverified until built and launched on their target platforms.

## Closed (this board's lifetime)

Carried over from `main`'s board so no row is lost in the restructure.

| Task | Landed |
|---|---|
| Aegis baseline regeneration + `.aegisignore` self-exclusion | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 6 — View menu (10 inerts) | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 7 — Text menu (14 inerts) | 2026-11-03 (`72aa1d7`) |
| 19.6 batch 8 — Attach In Path / Attach to Path (2 inerts); vertical-align rows evidence-gated | 2026-11-03 (`a8c86af`) |
| 19.6 inert-menu burn-down (Modify/File families) | Complete — batches 1–17 landed; the census is 2 inert of 294 items |

## Ledger rules

- Update this file and the ticket when work starts, changes scope, or lands.
- Keep detailed implementation history in [`SESSIONS.md`](SESSIONS.md); keep host/tooling diagnostics out of the FireCraft session history.
- Do not copy a second active task list into the handoff or historical audit documents.
