# T27-59 — Keep the execution board and the ticket ledger truthful as rows close

> **RP:** RP-00 · **Priority:** P1 · **Status:** IMPLEMENTED
> **Board row:** [EXECUTION-BOARD.md §6](../release/EXECUTION-BOARD.md)
> **Depends on:** T27-07

**Objective:** keep the board and its tickets in one-to-one correspondence and keep
every status no stronger than the evidence behind it.

**Inputs:** `docs/release/EXECUTION-BOARD.md`; `docs/tasks/T27-*.md`; `docs/TASKS.md`.

**Authority:** the board's own rules in §10; the campaign's requirement that status
states are distinct and never inferred from a script or workflow definition.

**Acceptance criteria:** every closed row carries a completion date and a cited
evidence link; no row claims a status stronger than its evidence; the board's task-ID
set and the set of `docs/tasks/T27-*.md` files match one-to-one; no ticket file is
orphaned and no board row lacks a ticket.

**Verification:** `ls docs/tasks/T27-[0-9]*.md | wc -l` and
`grep -c '^| T27-' docs/release/EXECUTION-BOARD.md`

**Evidence:** the board's §6 table and the 61 ticket files now state the same status, evidence, blockers and completion date for every row; [`scripts/verify_release_ledger.sh`](https://github.com/aliasfoxkde/FireCraft/blob/main/scripts/verify_release_ledger.sh) reports `rows=61 tickets=61 problems=0`. The reconciliation is a tracked tool, not a one-off script: `scripts/reconcile_release_ledger.py` writes the board's cells into their tickets (the board is the only source of truth), and the verifier is the gate that keeps them in step. The verifier also refuses a closed row that cites no evidence link (NO LINK) — the board's acceptance criterion is a completion date *and* a cited evidence link, so every advanced row now carries one and the gate keeps it that way. The four rows whose verification cell carries an escaped pipe inside a code span — T27-08, T27-22, T27-41 and this one — keep the escape, so each row is ten cells and not eleven. **Done:** 2026-11-15: the board and its 61 tickets reconciled and the verifier landed with a CI step.

**Blockers:** —

**Stopping condition:** the two counts match and every non-NOT STARTED row's status is
justified by a cited file, command output, digest or job id.

---

## Outcome (2026-11-16)

The ledger was reconciled against `main` at `978215e9`.

**Counts.** 61 board rows, 61 ticket files, no orphan ticket and no row without a
ticket. The board's rows are grouped by RP, so their order differs from the tickets'
numeric order — correspondence is by ID, not position.

**Dates.** Every row in a completion state (`IMPLEMENTED`, `BUILD VERIFIED`,
`SMOKE TESTED`) carries a `Done` date. One did not: T27-54's implementation had
landed in `main` as `658f015a` (#107) on 2026-11-11 with an empty `Done` cell. It now
carries both the date and the merge SHA.

**Evidence.** Every row above `NOT STARTED` cites evidence. The rows that cited stale
or partial facts were corrected:

- **T27-03** — PR #9 and PR #8 are both **closed unmerged** (2026-11-10 and
  2026-11-11), so their heads are `a0409ed6` and `d4c1c322`. The row's old evidence
  cited the 2026-11-08 baseline SHAs and PR #8's 29-comment surface; PR #8 now carries
  **38** inline comments and 9 review bodies, because nine were raised after the
  baseline (2026-11-09 and 2026-11-10) and the PR was closed before anyone
  dispositioned them. Every one of the nine is now enumerated and classified in
  §6 of the board against `main` at `978215e9`: four are resolved, one is a **live
  application defect** (a pure click with the Vector Path tool begins a node drag
  that never ends — `begin_node_drag` on `clicked()` at `canvas_ui.rs:1268`,
  `end_node_drag` only on `drag_stopped()` at `canvas_ui.rs:1152`), and four are
  open documentation-accuracy items. The Major one is carried to T27-24; the
  documentation ones to T27-10 and T27-24.
- **T27-10** — the `docs/TASKS.md` cross-reference this row was blocked on now
  exists (row 27 and the "Production readiness and T27 work-package detail"
  section), so the blocker is cleared and the four open documentation findings are
  listed against the row.
- **T27-24** — its candidate list now names the verified-live Vector Path finding.

**Blocker.** `docs/TASKS.md` is owned by another agent in this session — cleared:
the cross-reference landed, and the board's T27-59 row no longer names it.
