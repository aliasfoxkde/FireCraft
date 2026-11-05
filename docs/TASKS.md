# FireCraft task board

> **Live ledger.** Last reconciled against `main` at `5fac7a2` on 2026-11-04. Use [`NEXT.md`](NEXT.md) for phase detail and [`tasks/`](tasks/) for task tickets. Historical plans and session notes are not current status sources.

## Active

| Priority | Task | Status | Detail |
|---|---|---|---|
| P0 | Refresh user-visible CS6 parity matrix | **Done — matrix published** | [T26](tasks/T26-parity-matrix.md) → [`CS6-PARITY.md`](CS6-PARITY.md): every CS6 menu, tool, panel, inspector field set, workflow, import/export surface, canvas interaction, keymap area, and selection state classified with an evidence citation and an owning ticket. The browser-sampling check the ticket named was performed on 2026-11-04 against a fresh optimized bundle and is recorded in T22. |
| P0 | Phase 22: CS6 shell fidelity and screenshot audit | In progress | [T22](tasks/T22-screenshot-ui.md) — the ticket's own acceptance criteria are met (captures at both viewports, the empty/vector/bitmap state set, the fresh bundle, and the replaced tracked screenshot); what remains is the work other Phase 22 items own: 22.1 panel inventory, 22.2 tool strip, 22.5 chrome. [Phase 22](NEXT.md#phase-22--cs6-shell-fidelity-p0-the-looks-and-acts-like-fireworks-phase) |
| P1 | Phase 20: daily-driver editing features | Queued | In-canvas text, Optimize, animation timeline, mask stack, tabs, history rewind, slice behavior; [details](NEXT.md#phase-20--daily-driver-features-p1) |
| P1 | Phase 21: modern differentiators | Queued | Autosave, HiDPI, palette, preferences, accessibility, scripting; [details](NEXT.md#phase-21--modern-differentiators) |
| P1 | Reconcile live web deployment | Artifact differs | The public manifest's JS/wasm/index hashes differ from local `dist/web`; identify intended deploy and verify it before publishing. See [T18](tasks/T18-release.md). |
| P2 | Next-release asset breadth and platform verification | Open | [T23](tasks/T23-release-assets.md); recipes exist, but non-Linux artifacts need real platform build/launch receipts. |
| P2 | Coverage and test quality ratchet | Open | Replace the old aspirational 99% label with measured per-crate targets and snapshots; [NEXT.md](NEXT.md). |

## Phase 22 detail

| Item | State at `5fac7a2` | Remaining evidence/work |
|---|---|---|
| 22.1 Panel inventory and docking | Partial | A right-side stack of independent windows exists. Improve fit and navigation at standard viewport sizes; finish missing Optimize, Common Library, CSS Properties, Find and Replace, and Slices surfaces. |
| 22.2 Tool strip and flyouts | Open | Compare order, grouping, flyout interaction, and keyboard behavior against the documented reference. |
| 22.3 Adaptive canvas context menu | Complete | Selection-specific rows dispatch through the existing command surface. |
| 22.4 Property inspector | Partial | Per-kind field sets landed 2026-11-04 (group, bitmap, slice). Open: the bitmap transform strip and fill/stroke, the document-level and multi-selection field sets, and enter-group. |
| 22.5 CS6 chrome | Partial | Audit tab/status/header layout and responsive behavior against the approved reference. |
| 22.6 Open-license icon pack | Complete | Attribution and measured wasm-budget effects are recorded in the phase history. |
| Screenshot capture and comparison | Complete | Captured and inspected at 1280×800 and 1920×1080 in the empty, vector, and bitmap states against a fresh optimized bundle on 2026-11-04; the tracked screenshot was replaced and labelled for its build. Receipts in [T22](tasks/T22-screenshot-ui.md). |

## Completed

| Task | Evidence |
|---|---|
| 19.1–19.5 canvas competence | Transform handles, path nodes, gradient editor, command palette, guides/grid/snap; [NEXT.md](NEXT.md). |
| 19.6 menu wiring campaign | Batches 1–17 landed; current census is 8 inert rows of 289, derived and gated by `menus::tests::inert_count_never_rises`. Remaining rows are feature work or evidence-gated, not an active burn-down. |
| 19b path nodes and wasm budget | Path node add/delete and dated wasm ceiling re-pin recorded in [Progress](PROGRESS.md). |
| v0.2.5 release tag | `v0.2.5` exists; see [release notes](releases/v0.2.5.md). The live deployment uses a different manifest from the current local distribution. |

## Blocked or evidence-gated

- The 3 partial CS6 DOM rows remain evidence-gated; see [`planning/generated/dom-parity.md`](planning/generated/dom-parity.md) and the specific decisions.
- The remaining 8 menu rows are tracked with explicit reasons by the live `inert_worklist`; do not wire behavior without a model/engine or reference basis.
- Non-Linux release artifacts remain unverified until built and launched on their target platforms.

## Ledger rules

- Update this file and the ticket when work starts, changes scope, or lands.
- Keep detailed implementation history in [`SESSIONS.md`](SESSIONS.md); keep host/tooling diagnostics out of the FireCraft session history.
- Do not copy a second active task list into the handoff or historical audit documents.
