# T26 — Refresh the user-visible CS6 parity matrix

**Status:** **matrix published** as [`../CS6-PARITY.md`](../CS6-PARITY.md)
(2026-11-04). The stopping condition is met — every in-scope CS6
workflow and UI area carries a classification and an evidence link,
and every missing/partial row has an owner ticket and a priority. The
ticket's browser-sampling verification is **still open**: it needs a
fresh optimized web bundle and rides with
[`T22-screenshot-ui.md`](T22-screenshot-ui.md)'s capture.

**What the matrix added beyond the DOM matrix.** The DOM matrix scores
*scripting* verbs (268 carried / 3 partial / 0 missing of 356). The
user-visible matrix scores *reachability*: a verb can be fully carried
and still have no UI path. The largest findings were

- the CS6 panel set is **twenty-three**, not eighteen — five CS6
  panels have no Window-menu row (`research/fw-ui-evidence.md`);
- the CS6 right dock is **five stacked tab-groups**, not the flat
  column FireCraft draws;
- the inspector has **no fields at all** for a bitmap, a group, or a
  slice selection;
- FireCraft is single-document, so *Duplicate Window*, *Cascade* and
  *Tile* all collapse to the same action.

**CS6-side evidence.** The panel command-id table came from
`Fireworks Resources.dll`'s `MENU_DESCRIPTION_MAP`; the dock
arrangement came from the install's own
`Configuration/Commands/Workspace Layouts/Win/Expanded Mode.xml`.
Both are recorded in [`../research/fw-ui-evidence.md`](../research/fw-ui-evidence.md).
Raw dumps stay uncommitted, per PLANNING §4.3.

**Objective:** create one evidence-backed inventory of Fireworks CS6 user-visible workflows and classify FireCraft's coverage, so remaining feature and UI/UX work can be completed in a defensible order.

**Scope:** menu commands, tools and flyouts, panels, inspector fields, document workflows, import/export surfaces, canvas interactions, keyboard access, and representative object-selection states. Keep modern FireCraft additions in a separate section.

**Inputs:** `docs/research/fw-ui-evidence.md`, `docs/FEATURES.md`, `docs/NEXT.md` Phase 22, `docs/planning/generated/dom-parity.md`, current source, and authorized behavioral observations. Do not use or commit Adobe code or artwork.

**Method:** mark each row `implemented`, `partial`, `missing`, `evidence-gated`, or `not applicable`; cite source, decision, research note, screenshot, or observable behavior. Link each actionable gap to a ticket with acceptance criteria. Do not infer completeness from a command registry count or a panel inventory.

**Stopping condition:** every in-scope CS6 workflow and UI area has a classification and evidence link; all missing/partial items have an owner, priority, and acceptance criterion; the matrix distinguishes parity work from modern additions and documented refusals.

**Verification:** independent review of the matrix against the reference inventory; sample each category in the running browser at normal viewport sizes; ensure every item labeled implemented has an executable UI path or an explicitly documented equivalent.
